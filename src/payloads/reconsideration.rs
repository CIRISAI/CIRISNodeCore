//! `reconsideration_request` payload — SCHEMA.md §4.12 / §9.
//!
//! Per `MISSION.md` Primitive 11 / §3.9 / §5.7. A signed request to
//! reverse a prior `SlashingAttestation` (or fold previously-discounted
//! evidence into the standing). Witness set always required at the
//! envelope level per §3.5.
//!
//! Time bound (per `MISSION.md` §3.9): 180-day default from the
//! target SlashingAttestation's `attested_at` for `NewEvidence` and
//! `ProceduralError`; unlimited for `QuorumCompromise`.
//!
//! Recursion bound (per `MISSION.md` §3.9): one Reconsideration per
//! ground per SlashingAttestation; three filings on a single
//! SlashingAttestation trips harassment review.
//!
//! Both bounds are enforced by `NodeCoreEngine::put_reconsideration_request`
//! at the write boundary; violations surface as
//! [`crate::Error::ReconsiderationBounds`].

use serde::{Deserialize, Serialize};

use crate::identity::ContributorId;

/// Grounds for filing a reconsideration per SCHEMA.md §9.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Grounds {
    /// New evidence emerged that the original quorum did not see.
    /// Time-bounded (180-day default).
    NewEvidence,
    /// The original adjudication suffered a procedural error
    /// (witness diversity failure, signature gap, etc.).
    /// Time-bounded (180-day default).
    ProceduralError,
    /// The original quorum itself was compromised. Unlimited time bound
    /// per `MISSION.md` §3.9.
    QuorumCompromise,
}

/// `reconsideration_request` payload per SCHEMA.md §4.12 / §9.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconsiderationRequest {
    /// Federation identity of the requester. Per `MISSION.md` §3.9 any
    /// contributor with standing in the affected cell may file —
    /// not just the slashing target.
    pub requester_id: ContributorId,
    /// The `SlashingAttestation` being reconsidered.
    pub target_slashing_id: String,
    /// Grounds.
    pub grounds: Grounds,
    /// Canonical-encoded evidence payload. Application-specific shape;
    /// audit chain stores verbatim.
    pub evidence: String,
    /// Requester's at-risk stake in Commons Credits. Decimal string to
    /// avoid float drift. Disposition determined by the
    /// `ReconsiderationAttestation` outcome.
    pub requester_stake: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grounds_serde_snake_case() {
        assert_eq!(
            serde_json::to_string(&Grounds::NewEvidence).unwrap(),
            r#""new_evidence""#
        );
        assert_eq!(
            serde_json::to_string(&Grounds::QuorumCompromise).unwrap(),
            r#""quorum_compromise""#
        );
    }

    #[test]
    fn reconsideration_round_trip() {
        let r = ReconsiderationRequest {
            requester_id: ContributorId::new("requesterpub"),
            target_slashing_id: "slash_01HX".into(),
            grounds: Grounds::ProceduralError,
            evidence: "{\"missing_witness\":\"jur=US\"}".into(),
            requester_stake: "8.0".into(),
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: ReconsiderationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.grounds, r.grounds);
        assert_eq!(back.requester_stake, "8.0");
    }
}
