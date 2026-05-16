//! `notification_response` payload — SCHEMA.md §4.21.
//!
//! Peer's optional support / rebut / clarify response to a
//! `notification`. The consensus-on-observations pattern: peers
//! concur with or dispute an observation without escalating to a
//! §4.11 `moderation_event` formal accusation.

use serde::{Deserialize, Serialize};

/// `subject_kind` discriminator. Wire constant; matches SCHEMA §3.2.
pub const SUBJECT_KIND: &str = "notification_response";

/// Stance enum for `notification_response`. Three positions:
/// `Support` (I concur with the observation), `Rebut` (I dispute it),
/// `Clarify` (additional information without concurring or rebutting).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationStance {
    /// Concur with the observation.
    Support,
    /// Dispute the observation.
    Rebut,
    /// Additional info, neither supporting nor rebutting.
    Clarify,
}

/// `notification_response` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResponsePayload {
    /// Back-ref to the original notification's `contribution_id`.
    pub notification_id: String,
    /// Stance.
    pub stance: NotificationStance,
    /// Free-text explanation.
    pub rationale: String,
    /// Supporting evidence.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_kind_constant_matches_schema() {
        assert_eq!(SUBJECT_KIND, "notification_response");
    }

    #[test]
    fn stance_snake_case() {
        assert_eq!(
            serde_json::to_string(&NotificationStance::Support).unwrap(),
            r#""support""#
        );
        assert_eq!(
            serde_json::to_string(&NotificationStance::Rebut).unwrap(),
            r#""rebut""#
        );
        assert_eq!(
            serde_json::to_string(&NotificationStance::Clarify).unwrap(),
            r#""clarify""#
        );
    }

    #[test]
    fn round_trip() {
        let p = NotificationResponsePayload {
            notification_id: "01HX".into(),
            stance: NotificationStance::Support,
            rationale: "Confirmed; same profile observed in our cell.".into(),
            evidence_refs: vec!["trace_01HX".into()],
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: NotificationResponsePayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.stance, NotificationStance::Support);
    }
}
