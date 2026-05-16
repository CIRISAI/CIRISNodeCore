//! `cancellation` payload — SCHEMA.md §4.28.
//!
//! Retract an in-flight request before it resolves. Per
//! `FSD/MESSAGE_TAXONOMY.md` §7 (FIPA `cancel` gap). Author-only —
//! engine enforces that the cancellation's signer matches the
//! cancelled contribution's `author_id`.
//!
//! Applicable to: `deferral_request`, `assistance_request`,
//! `subscription_request`, `commitment`, `*_edit` (withdraw a
//! proposal before voting closes). Not applicable to completed
//! transactions (`*_response`, `vote`, `slashing_attestation`,
//! `promotion_attestation`) — those route through `reconsideration_request`
//! (§4.12) or `moderation_event` (§4.11) instead.
//!
//! For `service_announcement` use §4.24 `service_deprecation`
//! instead — it carries `effective_at` semantics `cancellation`
//! doesn't.

use serde::{Deserialize, Serialize};

/// `subject_kind` discriminator. Wire constant; matches SCHEMA §3.2.
pub const SUBJECT_KIND: &str = "cancellation";

/// `cancellation` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancellationPayload {
    /// The contribution being cancelled. MUST be authored by the
    /// same key issuing this cancellation; engine enforces.
    pub cancels_contribution_id: String,
    /// Free-text rationale recorded on the audit chain.
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_kind_constant_matches_schema() {
        assert_eq!(SUBJECT_KIND, "cancellation");
    }

    #[test]
    fn round_trip() {
        let p = CancellationPayload {
            cancels_contribution_id: "01HX".into(),
            reason: "Withdrawing deferral — resolved internally.".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: CancellationPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.cancels_contribution_id, p.cancels_contribution_id);
    }
}
