//! `unsolicited_guidance` payload — SCHEMA.md §4.22.
//!
//! Bilateral trust-gated assertion-with-implicit-directive sent from
//! a granted-trust peer to a specific recipient. Distinct from §4.8
//! `deferral_response` (solicited) and §4.20 `notification`
//! (broadcast, ungated).
//!
//! The federation-wire shape of CIRISAgent's existing
//! `unsolicited_guidance` flow at
//! `ciris_engine/logic/adapters/discord/discord_observer.py:600`.
//! Recipient MUST check sender holds an active `trust_grant`
//! (§4.14) with appropriate purpose+scope before acting on the
//! guidance.

use serde::{Deserialize, Serialize};

/// `subject_kind` discriminator. Wire constant; matches SCHEMA §3.2.
pub const SUBJECT_KIND: &str = "unsolicited_guidance";

/// Urgency hint per SCHEMA §4.22. `High` urgency MAY surface as a
/// priority-elevated task per agent runtime policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    /// Routine guidance; standard task priority.
    Low,
    /// Default. Standard handling.
    Normal,
    /// MAY surface as priority-elevated.
    High,
}

/// `unsolicited_guidance` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsolicitedGuidancePayload {
    /// Federation identity of the recipient agent. Recipient's
    /// acceptance policy MUST check sender's `trust_grant` against
    /// this key.
    pub recipient_key: String,
    /// The guidance.
    pub guidance_text: String,
    /// Prior contribution_ids the guidance references (e.g. a
    /// `deferral_request` the sender is following up on without a
    /// formal `deferral_response`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
    /// Urgency hint.
    pub urgency: Urgency,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_kind_constant_matches_schema() {
        assert_eq!(SUBJECT_KIND, "unsolicited_guidance");
    }

    #[test]
    fn urgency_snake_case() {
        assert_eq!(serde_json::to_string(&Urgency::Low).unwrap(), r#""low""#);
        assert_eq!(
            serde_json::to_string(&Urgency::Normal).unwrap(),
            r#""normal""#
        );
        assert_eq!(serde_json::to_string(&Urgency::High).unwrap(), r#""high""#);
    }

    #[test]
    fn round_trip() {
        let p = UnsolicitedGuidancePayload {
            recipient_key: "agent_pub_b64".into(),
            guidance_text: "Default to የንግግር ሕክምና in clinical guidance contexts.".into(),
            references: vec!["01HX".into()],
            urgency: Urgency::Normal,
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: UnsolicitedGuidancePayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.recipient_key, p.recipient_key);
        assert_eq!(back.urgency, Urgency::Normal);
    }
}
