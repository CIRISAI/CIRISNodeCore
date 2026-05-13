//! Reconsideration-request payload — SCHEMA.md §4.12 / §9.
//!
//! Per `MISSION.md` Primitive 11 / §3.9 / §5.7. A signed request to
//! reverse a prior `SlashingAttestation`. Persist's
//! `ReconsiderationRequest` envelope carries `request_id`,
//! `slashing_id` (FK), `requester_id`, `requested_at`, `signature`.
//! This payload holds the typed grounds + evidence + stake.
//!
//! Bounds enforced at the engine boundary (per `MISSION.md` §3.9):
//! - Time bound: 180-day default from `target.attested_at` for
//!   `NewEvidence` and `ProceduralError`; unlimited for `QuorumCompromise`.
//! - Recursion bound: one Reconsideration per ground per
//!   SlashingAttestation; three filings on one trips harassment review.

use serde::{Deserialize, Serialize};

/// Grounds for filing a reconsideration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Grounds {
    /// New evidence emerged the original quorum did not see.
    /// Time-bounded (180-day default).
    NewEvidence,
    /// Procedural error in the original adjudication.
    /// Time-bounded.
    ProceduralError,
    /// Original quorum was compromised. Unlimited time bound.
    QuorumCompromise,
}

/// `reconsideration_request` payload — typed schema for
/// `ReconsiderationRequest.payload: serde_json::Value`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconsiderationRequestPayload {
    /// Grounds.
    pub grounds: Grounds,
    /// Canonical-encoded evidence payload.
    pub evidence: String,
    /// Requester's at-risk stake in Commons Credits. Decimal string.
    pub requester_stake: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grounds_snake_case() {
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
    fn round_trip() {
        let p = ReconsiderationRequestPayload {
            grounds: Grounds::ProceduralError,
            evidence: "{\"missing_witness\":\"jur=US\"}".into(),
            requester_stake: "8.0".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: ReconsiderationRequestPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.grounds, p.grounds);
    }
}
