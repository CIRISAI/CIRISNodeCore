//! Trust recursion depth — admission-decision helpers for the
//! [`FEDERATION_SCALING_MODEL`](../../FSD/FEDERATION_SCALING_MODEL.md)
//! §1.4 trust-recursion-depth knob.
//!
//! # The discipline
//!
//! Per [FSD §1.4](../../FSD/FEDERATION_SCALING_MODEL.md), trust
//! recursion depth is **operator-side local config** — no CEG wire
//! enhancement. The federation's existing `delegates_to` attestations
//! already carry the full trust graph; each server independently
//! chooses how deep to walk it when admitting inbound content.
//!
//! Tier-tied defaults: client=0, proxy/L0=0, server/L1=1. Operators
//! tune via local config.
//!
//! # What this module exposes
//!
//! * [`effective_trust_set`] — for a `root_key_id` and `depth`,
//!   compute the set of keys whose content the root admits (the
//!   depth-N transitive closure over active `delegates_to` edges).
//! * [`admits_at_depth`] — yes/no membership check for
//!   `(root, source, depth)`. The actual admission gate at the
//!   intake site (persist's CIRISPersist#123 implementation) is the
//!   authoritative check; this module is the *decision oracle* that
//!   the CIRISConformance harness (CIRISNodeCore#21) asserts
//!   against.
//!
//! # Substrate consumed
//!
//! [`ciris_persist::federation::topology::build_delegation_graph`]
//! does the BFS — cycle-safe, withdrawals-aware (`withdraws` /
//! `recants` annotate retracted edges), and depth-bounded
//! (`MAX_DELEGATION_DEPTH = 16`). This module wraps that helper for
//! the admission-decision use case.
//!
//! # Active-only semantics
//!
//! An edge with a non-None `withdrawn_by` annotation has been
//! retracted by the granter and **does not contribute** to the
//! effective trust set. The retraction propagation is wire-format-
//! defined (the `withdraws` / `recants` structural primitives);
//! persist's BFS surfaces it; we honor it here.

use std::collections::HashSet;

use ciris_persist::federation::topology::{build_delegation_graph, DelegationGraph};
use ciris_persist::federation::FederationDirectory;

use crate::substrate::SubstrateError;
use crate::trust::fed_err;

/// Pure-function tail: collapse a [`DelegationGraph`] (cycle-safe,
/// retraction-annotated BFS output from persist) into the set of
/// admitted keys.
///
/// Filters out edges whose `withdrawn_by` is `Some(_)` — a retracted
/// delegation must NOT contribute to the active trust set. The root
/// is always included.
///
/// Separated from [`effective_trust_set`] so the decision logic can
/// be unit-tested against fixture `DelegationGraph` values without
/// mocking persist's `FederationDirectory` trait (which churns more
/// often than this admission-decision shape).
pub fn admitted_set_from_graph(graph: &DelegationGraph) -> HashSet<String> {
    let mut set: HashSet<String> = HashSet::new();
    set.insert(graph.root_key.clone());
    for edge in &graph.edges {
        if edge.withdrawn_by.is_some() {
            continue;
        }
        set.insert(edge.to_key.clone());
    }
    set
}

/// Compute the effective trust set of `root_key_id` within `depth`
/// hops over active `delegates_to` edges.
///
/// The root itself is included in the returned set. At `depth = 0`
/// the set is `{root_key_id}` (no edges traversed). At `depth = N`
/// it includes every key reachable from root via a chain of ≤ N
/// active `delegates_to` edges.
///
/// Withdrawn edges (`withdraws` / `recants` against a prior
/// `delegates_to`) are excluded — only currently-active delegations
/// contribute.
///
/// Depth is `usize`-typed; persist clamps at `MAX_DELEGATION_DEPTH`
/// (currently 16) for runaway-BFS protection.
///
/// # Errors
///
/// Propagates any [`ciris_persist::federation::Error`] from the
/// underlying BFS (typically backend / FK / canonicalization errors).
/// `root_key_id` must be non-empty.
pub async fn effective_trust_set(
    directory: &dyn FederationDirectory,
    root_key_id: &str,
    depth: usize,
) -> Result<HashSet<String>, SubstrateError> {
    if depth == 0 {
        let mut set = HashSet::new();
        set.insert(root_key_id.to_string());
        return Ok(set);
    }
    let graph: DelegationGraph = build_delegation_graph(directory, root_key_id, depth)
        .await
        .map_err(fed_err)?;
    Ok(admitted_set_from_graph(&graph))
}

/// True iff `source_key_id` is admitted by `root_key_id` at the
/// given trust recursion depth.
///
/// Equivalent to
/// `effective_trust_set(directory, root, depth).await?.contains(source)`.
///
/// The Conformance harness (CIRISNodeCore#21) asserts:
/// * `admits_at_depth(_, root, source, 0)` — true only for direct
///   trust; a friend-of-friend source is refused
/// * `admits_at_depth(_, root, source, 1)` — admits friend-of-
///   friends; a 2-hop source is refused
/// * `admits_at_depth(_, root, source, N)` — admits exactly the
///   transitive closure within N hops
pub async fn admits_at_depth(
    directory: &dyn FederationDirectory,
    root_key_id: &str,
    source_key_id: &str,
    depth: usize,
) -> Result<bool, SubstrateError> {
    let set = effective_trust_set(directory, root_key_id, depth).await?;
    Ok(set.contains(source_key_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use ciris_persist::federation::topology::{DelegationEdge, WithdrawalEntry};

    fn edge(from: &str, to: &str, depth: usize, withdrawn: bool) -> DelegationEdge {
        DelegationEdge {
            from_key: from.to_string(),
            to_key: to.to_string(),
            scope: String::new(),
            granted_at: Utc::now(),
            evidence_refs: vec![],
            withdrawn_by: if withdrawn {
                Some(WithdrawalEntry {
                    key_id: from.to_string(),
                    withdrawn_at: Utc::now(),
                    kind: "withdraws".to_string(),
                })
            } else {
                None
            },
            depth,
        }
    }

    fn graph(root: &str, max_depth: usize, edges: Vec<DelegationEdge>) -> DelegationGraph {
        DelegationGraph {
            root_key: root.to_string(),
            max_depth,
            edges,
        }
    }

    #[test]
    fn depth_zero_admits_only_root() {
        // An empty graph (no edges traversed) still includes the root.
        let g = graph("alice", 0, vec![]);
        let set = admitted_set_from_graph(&g);
        assert_eq!(set.len(), 1);
        assert!(set.contains("alice"));
    }

    #[test]
    fn depth_one_admits_direct_trust_but_not_friend_of_friend() {
        // BFS at depth 1: only direct delegations are in the graph.
        // persist's build_delegation_graph would NOT include bob→carol
        // when called with max_depth=1, so the test fixture reflects that.
        let g = graph("alice", 1, vec![edge("alice", "bob", 1, false)]);
        let set = admitted_set_from_graph(&g);
        assert!(set.contains("alice"));
        assert!(set.contains("bob"));
        assert!(
            !set.contains("carol"),
            "carol is 2 hops, not in depth-1 graph"
        );
    }

    #[test]
    fn depth_two_admits_friend_of_friend_but_not_three_hop() {
        // BFS at depth 2: alice→bob (depth 1) + bob→carol (depth 2).
        let g = graph(
            "alice",
            2,
            vec![
                edge("alice", "bob", 1, false),
                edge("bob", "carol", 2, false),
            ],
        );
        let set = admitted_set_from_graph(&g);
        assert!(set.contains("alice"));
        assert!(set.contains("bob"));
        assert!(set.contains("carol"));
        assert!(
            !set.contains("dave"),
            "dave is 3 hops, not in depth-2 graph"
        );
    }

    #[test]
    fn withdrawn_edge_does_not_contribute() {
        // Alice delegated to mallory, then withdrew.
        let g = graph(
            "alice",
            1,
            vec![
                edge("alice", "bob", 1, false),
                edge("alice", "mallory", 1, true),
            ],
        );
        let set = admitted_set_from_graph(&g);
        assert!(set.contains("alice"));
        assert!(set.contains("bob"));
        assert!(
            !set.contains("mallory"),
            "withdrawn delegation must not contribute to the active trust set"
        );
    }

    #[test]
    fn mixed_active_and_withdrawn_at_multiple_depths() {
        // Realistic case: deep graph with selective withdrawals.
        let g = graph(
            "alice",
            3,
            vec![
                edge("alice", "bob", 1, false),
                edge("alice", "frank", 1, true), // withdrawn
                edge("bob", "carol", 2, false),
                edge("bob", "evan", 2, true), // withdrawn by bob
                edge("carol", "dave", 3, false),
            ],
        );
        let set = admitted_set_from_graph(&g);
        let expected: HashSet<String> = ["alice", "bob", "carol", "dave"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(set, expected);
    }
}
