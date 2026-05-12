//! `NodeCoreEngine` — trait surface mirroring CIRISPersist Appendix A.
//!
//! Persist publishes the federation-consensus typed-write + read methods
//! in `CIRISPersist/FSD/CIRIS_PERSIST.md` Appendix A (closes CIRISPersist#30).
//! v0.6.0 publishes the contract; the actual methods materialize at v0.6.x
//! or v0.7.0 per Appendix A.5.
//!
//! Until then, node-core depends on this **trait** rather than persist's
//! concrete `Engine`. When persist v0.6.x ships:
//!
//! 1. Replace this module's body with a re-export of persist's typed-write
//!    methods (or implement this trait *for* `ciris_persist::Engine`).
//! 2. Bump the `ciris-persist` Cargo pin in `Cargo.toml`.
//! 3. Existing callers continue working — the trait was the seam.
//!
//! Same shape lens-core's FSD §6 ("Day-1 integration: edge + persist
//! consumed implicitly") established.

use async_trait::async_trait;

use crate::contribution::ContributionEnvelope;
use crate::error::Result;
use crate::ledger::{CommonsCreditsLedger, ExpertiseLedger, VoteWeight};
use crate::vote::Vote;

/// Federation-consensus substrate trait. Mirrors Appendix A.2 (writes)
/// + A.3 (reads) of `CIRISPersist/FSD/CIRIS_PERSIST.md`.
///
/// Implementors:
///
/// - `ciris-persist v0.6.x` (or v0.7.0) — production substrate.
/// - In-memory mock for tests — `tests/support/mock_engine.rs` (forthcoming).
#[async_trait]
pub trait NodeCoreEngine: Send + Sync {
    // ── Writes (Appendix A.2) ────────────────────────────────────────────

    /// Append a signed Contribution to the federation audit chain.
    /// Routes to the `contributions` table; discriminated by the
    /// envelope's `contribution_type`. Returns the row's persist-issued
    /// canonical id (may equal `envelope.contribution_id` or wrap it).
    async fn put_contribution(&self, envelope: ContributionEnvelope) -> Result<String>;

    /// Record a signed Vote on a Contribution. Routes to the `votes`
    /// table. Persist enforces unique-per-`(voter_id, contribution_id)`;
    /// duplicate casts return [`Error::Schema`] from this trait's POV.
    ///
    /// [`Error::Schema`]: crate::error::Error::Schema
    async fn cast_vote(&self, vote: Vote) -> Result<String>;

    /// Derived-state write: bump a contributor's Credits at a cell.
    /// Triggered by the truth-grounding loop (`MISSION.md` §3.4).
    /// `delta` may be negative (decay) but the non-negative floor is
    /// enforced by persist at the boundary; a write that would push
    /// below zero returns [`Error::LedgerInvariant`].
    ///
    /// [`Error::LedgerInvariant`]: crate::error::Error::LedgerInvariant
    async fn update_credits_ledger(
        &self,
        contributor_id: &crate::identity::ContributorId,
        cell: &crate::cell::Cell,
        delta: f64,
    ) -> Result<()>;

    /// Derived-state write: bump a contributor's Expertise standing
    /// at a cell. Per `MISSION.md` §3.7 (Expertise attestation flow +
    /// hard-case track-record signals).
    async fn update_expertise_ledger(
        &self,
        contributor_id: &crate::identity::ContributorId,
        cell: &crate::cell::Cell,
        delta: f64,
    ) -> Result<()>;

    // ── Reads (Appendix A.3) ─────────────────────────────────────────────

    /// Composite read for §5.2 vote weighting. Returns
    /// `Credits × expertise_multiplier × active_tier_multiplier`.
    async fn read_vote_weight(
        &self,
        contributor_id: &crate::identity::ContributorId,
        cell: &crate::cell::Cell,
    ) -> Result<VoteWeight>;

    /// Point-lookup ledger read — full Credits view for a contributor.
    async fn get_credits_ledger(
        &self,
        contributor_id: &crate::identity::ContributorId,
    ) -> Result<CommonsCreditsLedger>;

    /// Point-lookup ledger read — full Expertise view for a contributor.
    async fn get_expertise_ledger(
        &self,
        contributor_id: &crate::identity::ContributorId,
    ) -> Result<ExpertiseLedger>;

    /// Routing-eligibility query: contributors with non-zero Expertise
    /// in `(domain, language)`, filtered to Active tier per `MISSION.md`
    /// §3.8. Used by deferral routing per §3.3 step 1-2.
    async fn routable_contributors(
        &self,
        domain: &str,
        language: &str,
    ) -> Result<Vec<crate::identity::ContributorId>>;

    // ── Inherited from persist's existing v0.4.2+ surface ────────────────

    /// Sign canonical bytes with the host's steward identity. Seed
    /// never crosses the FFI boundary into node-core. Mirrors persist's
    /// existing `StewardSigner` interface.
    async fn steward_sign(&self, canonical_bytes: &[u8]) -> Result<crate::signature::HybridSignature>;

    /// Canonicalize an envelope for signing. Routes through persist's
    /// `Engine.canonicalize_envelope_for_signing` — node-core never
    /// re-implements canonicalization (CIRISPersist#7 closure / AV-5).
    fn canonicalize<T: serde::Serialize + Send>(&self, value: &T) -> Result<Vec<u8>>;
}
