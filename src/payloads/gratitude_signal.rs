//! `gratitude_signal` payload — SCHEMA.md §4.17.
//!
//! Bilateral peer-to-peer quality signal per CIRISAgent's PoB §5.6.
//! Canonical payload shape per
//! `CIRISAgent/ciris_engine/schemas/services/agent_credits.py:75`.
//! Closes the bilateral verification loop as a cryptographic event.
//!
//! Encoded as a `proposal`-type Contribution with
//! `subject.subject_kind = "gratitude_signal"`. Envelope-level
//! signature replaces PoB's separate `DualSignature` field —
//! NodeCore envelopes already carry a hybrid signature per SCHEMA §2.4.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `subject_kind` discriminator. Wire constant; matches SCHEMA §3.2.
pub const SUBJECT_KIND: &str = "gratitude_signal";

/// `gratitude_signal` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GratitudeSignalPayload {
    /// Ed25519 pubkey hash of the signaling agent. Equals envelope `author_id`.
    pub from_agent_id: String,
    /// Ed25519 pubkey hash of the receiving agent.
    pub to_agent_id: String,
    /// Deterministic id binding both parties' trace ids — duplicate
    /// prevention per PoB §1.4.
    pub interaction_id: String,
    /// `0.0 ≤ x ≤ 1.0`. Quality rating of the interaction.
    pub quality_score: f64,
    /// Optional gratitude message. ≤ 280 characters per PoB schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// When the signal was created.
    pub timestamp: DateTime<Utc>,
}

/// Maximum length for the `message` field per PoB schema
/// (`agent_credits.py:96` Field(max_length=280)).
pub const MAX_MESSAGE_LEN: usize = 280;

impl GratitudeSignalPayload {
    /// True if the quality score is within the [0.0, 1.0] range
    /// PoB requires.
    pub fn is_valid_score(&self) -> bool {
        (0.0..=1.0).contains(&self.quality_score)
    }

    /// True if `message`, when present, respects the PoB 280-char limit.
    pub fn is_valid_message_length(&self) -> bool {
        match &self.message {
            Some(m) => m.chars().count() <= MAX_MESSAGE_LEN,
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_kind_constant_matches_schema() {
        assert_eq!(SUBJECT_KIND, "gratitude_signal");
    }

    #[test]
    fn round_trip() {
        let p = GratitudeSignalPayload {
            from_agent_id: "from_hash".into(),
            to_agent_id: "to_hash".into(),
            interaction_id: "interaction_01HX".into(),
            quality_score: 0.87,
            message: Some("Thank you for the clarification.".into()),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: GratitudeSignalPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.quality_score, 0.87);
        assert!(back.is_valid_score());
        assert!(back.is_valid_message_length());
    }

    #[test]
    fn invalid_score_outside_unit_interval() {
        let mut p = GratitudeSignalPayload {
            from_agent_id: "f".into(),
            to_agent_id: "t".into(),
            interaction_id: "i".into(),
            quality_score: 1.5,
            message: None,
            timestamp: Utc::now(),
        };
        assert!(!p.is_valid_score());
        p.quality_score = -0.1;
        assert!(!p.is_valid_score());
    }

    #[test]
    fn message_over_280_chars_rejected_by_validator() {
        let p = GratitudeSignalPayload {
            from_agent_id: "f".into(),
            to_agent_id: "t".into(),
            interaction_id: "i".into(),
            quality_score: 0.5,
            message: Some("x".repeat(281)),
            timestamp: Utc::now(),
        };
        assert!(!p.is_valid_message_length());
    }
}
