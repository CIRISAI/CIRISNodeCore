//! Re-exports of CIRISPersist's federation-consensus contract
//! (currently pinned at v0.9.1; cirisnode track API-stable since v0.7.1).
//!
//! Persist's `cirisnode` module hosts the canonical wire types and the
//! [`NodeCoreService`] trait per `CIRISPersist/FSD/CIRIS_PERSIST.md`
//! Appendix A.2 + A.3. This module exposes them under stable names so
//! node-core code can pin against the substrate's source-of-truth
//! types without reaching into persist's namespace at every site.
//!
//! # Status (v0.7.1 validated)
//!
//! `tests/substrate_contract.rs` is the validation spike — implements
//! `NodeCoreService` for an in-memory mock using RPITIT (no
//! `async_trait`), constructs every wire envelope from scratch, and
//! round-trips all 14 trait methods. 7/7 tests pass against v0.7.1.
//! Contract fits node-core's needs.
//!
//! # CIRISPersist#32 — closed in v0.7.2
//!
//! v0.7.2 added the `put_promotion_attestation` method + the
//! `PromotionAttestation` + `TargetRowKind` types — exactly the
//! Option B shape recommended in the issue. Trait now has 15
//! methods (was 14); transactional flip with affected-row-count
//! assertion keeps the canonical-promotion path safe under partial
//! failure. No open substrate asks.
//!
//! # OQ-7 collapse status
//!
//! Node-core still publishes parallel wire types at
//! [`crate::contribution`], [`crate::vote`], etc. plus its own
//! [`crate::engine::NodeCoreEngine`] async_trait. The collapse
//! (re-export persist's types as canonical; rename payloads to
//! `*Payload`; make `NodeCore` generic over `E: NodeCoreService`)
//! is deferred to a focused subsequent commit. See
//! [`FSD/SUBSTRATE_INTEGRATION.md`](../../FSD/SUBSTRATE_INTEGRATION.md)
//! §6 OQ-7 for the 5-step plan.

pub use ciris_persist::cirisnode::types::{
    Cell, ContributionEnvelope, ContributionListPage, ContributionType, ContributionsFilter,
    CreditsLedgerEntry, CreditsUpdate, DiversityProof, ExpertiseLedgerEntry, ExpertiseUpdate,
    HybridSignature, ListCursor, ModerationEvent, PromotionAttestation, ReconsiderationAttestation,
    ReconsiderationRequest, RoutableContributor, SlashingAttestation, TargetRowKind, VoteEnvelope,
    VoteListPage, VoteWeight, VotesFilter, Witness, WitnessSet,
};

pub use ciris_persist::cirisnode::{Error as SubstrateError, NodeCoreService};

#[cfg(test)]
mod tests {
    use super::*;

    /// Sanity check: the substrate types are actually accessible from
    /// node-core's namespace. If persist's α-cut rearranges the
    /// surface this test will fail at compile time and we'll need to
    /// adjust the re-exports.
    #[test]
    fn substrate_contract_accessible() {
        let _cell = Cell {
            domain: "mental_health".into(),
            language: "am".into(),
            subject: Some("arc_question".into()),
        };
        // Expertise-granularity — subject omitted per SCHEMA §7 / §10.
        let _expertise_cell = Cell {
            domain: "mental_health".into(),
            language: "am".into(),
            subject: None,
        };
        let _err: SubstrateError = SubstrateError::Conflict("test".into());
        assert_eq!(_err.kind(), "cirisnode_conflict");
    }

    /// The `NodeCoreService` trait uses RPITIT (`impl Future + Send`)
    /// per persist v0.7.0-α3, NOT `async_trait`. That means consumers
    /// hold concrete `E: NodeCoreService` or generic-bound types,
    /// not `dyn NodeCoreService`. Document the constraint via a
    /// compile-time check.
    fn _assert_node_core_service_is_a_trait<T: NodeCoreService>() {}
}
