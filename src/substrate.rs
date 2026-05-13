//! Re-exports of CIRISPersist v0.7.0-α3's federation-consensus contract.
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
//! # One known gap (filed as CIRISPersist#32)
//!
//! No write method to flip `is_canonical: FALSE → TRUE`. Persist's
//! V011 schema has the column, the read-side filter handles the
//! split, but the trait has no `mark_canonical` / `put_promotion_attestation`
//! to flip. Blocks the SCHEMA.md §13.3 canonical-promotion path —
//! filed for a v0.7.x patch.
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
