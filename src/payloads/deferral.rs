//! Deferral payloads — the policy-typed shapes that fill the
//! `payload: serde_json::Value` field of persist's [`ContributionEnvelope`]
//! when `contribution_type = DeferralRequest` or `DeferralResponse`.
//!
//! Envelope-level fields (id, author, cell, signature, timestamps)
//! live on persist's `ContributionEnvelope`; everything in this module
//! is the payload-only "policy" data.
//!
//! Per `MISSION.md` §1.6 / §3.3 / §5.1 — generalizes CIRISNode's WBD
//! submit + response surface. Truth-grounding signal: sustained
//! substantive contribution by routed responders.
//!
//! [`ContributionEnvelope`]: ciris_persist::cirisnode::types::ContributionEnvelope

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Expected response shape for a deferral. Constrains the `verdict`
/// discriminator on [`DeferralResponsePayload`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    /// Approve / reject. Verdict: `{"decision": "approve"|"reject", ...}`.
    Binary,
    /// One of N categorical options enumerated by the consumer.
    Categorical,
    /// Free-form text + optional numeric score.
    Freeform,
}

/// Diversity preference for routing per `MISSION.md` §3.3 step 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiversityPolicy {
    /// Bias routing across distinct jurisdictions.
    Jurisdictional,
    /// Bias routing across distinct operators.
    Organizational,
    /// No diversity preference; route by Expertise + Active tier only.
    None,
}

/// Consumer hints into `MISSION.md` §3.3 steps 3-4. Crate policy MAY
/// override.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPreferences {
    /// Minimum routed responders. Default 5 per §3.3 step 4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_responders: Option<u32>,
    /// Maximum routed responders. Default 9 per §3.3 step 4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_responders: Option<u32>,
    /// Diversity policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diversity: Option<DiversityPolicy>,
}

/// Payload for a `deferral_request` Contribution.
///
/// Envelope-level fields:
/// - `contribution_id` (= deferral_id)
/// - `author_id` (= consumer_id — the requesting agent)
/// - `subject.{domain, language}` (Expertise-granularity routing key,
///   `subject` field is `None`)
/// - `signature`, `submitted_at`
///
/// Everything below lives in the envelope's `payload` Value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferralRequestPayload {
    /// Back-reference to the consumer's internal task id. Preserves
    /// CIRISNode WBD's `agent_task_id` audit anchor across the
    /// federation cycle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_task_id: Option<String>,
    /// Short human label for routing UIs.
    pub title: String,
    /// The deferral content — what routed responders weigh in on.
    pub context: String,
    /// Constrains the `verdict` shape of routed responses.
    pub response_format: ResponseFormat,
    /// Soft hint; routing engine MAY exclude responders that haven't
    /// responded by this time when computing the §3.3 aggregate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<DateTime<Utc>>,
    /// Routing hints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing_preferences: Option<RoutingPreferences>,
}

/// Payload for a `deferral_response` Contribution.
///
/// Envelope-level fields:
/// - `contribution_id` (= response_id)
/// - `author_id` (= responder_id — MUST appear in the routed set the
///   crate selected for the originating deferral; engine enforces)
/// - `subject` (matches the originating deferral's cell)
/// - `signature`, `submitted_at`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferralResponsePayload {
    /// Refers to the originating `deferral_request` envelope's
    /// `contribution_id`.
    pub deferral_id: String,
    /// Shape constrained by the originating request's `response_format`.
    pub verdict: serde_json::Value,
    /// Free-text justification. Recorded on the audit chain per §5.1 step 8.
    pub rationale: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_format_snake_case() {
        assert_eq!(
            serde_json::to_string(&ResponseFormat::Binary).unwrap(),
            r#""binary""#
        );
    }

    #[test]
    fn request_payload_round_trip() {
        let p = DeferralRequestPayload {
            agent_task_id: Some("task_01HX".into()),
            title: "Stage-2 register check".into(),
            context: "Agent observed user asking about Amharic medication terms".into(),
            response_format: ResponseFormat::Binary,
            deadline: None,
            routing_preferences: Some(RoutingPreferences {
                min_responders: Some(5),
                max_responders: Some(9),
                diversity: Some(DiversityPolicy::Jurisdictional),
            }),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: DeferralRequestPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.title, p.title);
    }
}
