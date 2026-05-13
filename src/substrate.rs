//! Re-exports of CIRISPersist v0.7.0-α3's federation-consensus contract.
//!
//! Persist's `cirisnode` module hosts the canonical wire types and the
//! [`NodeCoreService`] trait per `CIRISPersist/FSD/CIRIS_PERSIST.md`
//! Appendix A.2 + A.3. This module exposes them under stable names so
//! node-core code can pin against the substrate's source-of-truth
//! types without reaching into persist's namespace at every site.
//!
//! # Divergence today (v0.1.0-dev)
//!
//! Node-core ALSO publishes its own wire types at [`crate::contribution`],
//! [`crate::vote`], [`crate::ledger`] etc., and its own
//! [`crate::engine::NodeCoreEngine`] trait. The two type families are
//! intentionally parallel during the v0.1.0-dev → v0.1.0 cut window:
//!
//! - **Persist's types** are envelope-shaped (id + signature + opaque
//!   `payload: serde_json::Value`) and follow `impl Future + Send`
//!   GAT trait shape. Source of truth at the storage boundary.
//! - **Node-core's types** carry the policy enums (`Allegation`,
//!   `Grounds`, `SlashingOutcome`, `AccuserStakeDisposition`, etc.)
//!   that fill the envelope's payload field. Use `async_trait` and
//!   `Arc<dyn>` for the test seam.
//!
//! When CIRISPersist v0.7.0 cuts final (α4-α6 to land), node-core's
//! v0.1.0 release collapses the two: re-export persist's wire types
//! as the canonical wire shapes; relegate node-core's parallel types
//! to thin payload-only structs (`*Payload` suffix) that fill the
//! `.payload` Value field. [`NodeCore`](crate::NodeCore) becomes
//! generic over `E: NodeCoreService` to consume persist's RPITIT
//! trait directly. Tracked in
//! [`FSD/SUBSTRATE_INTEGRATION.md`](../../FSD/SUBSTRATE_INTEGRATION.md)
//! §5 sequencing.

pub use ciris_persist::cirisnode::types::{
    Cell, ContributionEnvelope, ContributionListPage, ContributionType, ContributionsFilter,
    CreditsLedgerEntry, CreditsUpdate, DiversityProof, ExpertiseLedgerEntry, ExpertiseUpdate,
    HybridSignature, ListCursor, ModerationEvent, ReconsiderationAttestation,
    ReconsiderationRequest, RoutableContributor, SlashingAttestation, VoteEnvelope, VoteListPage,
    VoteWeight, VotesFilter, Witness, WitnessSet,
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
            subject: "arc_question".into(),
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
