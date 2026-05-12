//! Deferral payloads — SCHEMA.md §4.7 / §4.8.
//!
//! Generalizes CIRISNode's existing WBD submit/response surface per
//! `MISSION.md` §1.2 item 1 / §1.6 / §3.3 / §5.1. Routing target
//! selection happens via the Expertise ledger (non-zero standing in
//! `(domain, language)`); aggregation is per Primitive 7. The truth-
//! grounding signal is "sustained substantive contribution by routed
//! responders" (`MISSION.md` §1.6, medium fidelity).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::cell::Cell;
use crate::identity::ContributorId;

/// Expected response shape for a deferral — pins the `verdict` discriminator
/// on `DeferralResponse` per SCHEMA.md §4.7's `response_format` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    /// Approve / reject. Verdict carries `{"decision": "approve"|"reject", ...}`.
    Binary,
    /// One of N categorical options enumerated by the consumer. Consumer-
    /// provided options vector lives in `routing_preferences` for now;
    /// promoted to a top-level field if the pattern is used widely.
    Categorical,
    /// Free-form text + optional numeric score.
    Freeform,
}

/// Diversity preference for routing per SCHEMA.md §4.7 `routing_preferences`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiversityPolicy {
    /// Bias routing across distinct jurisdictions (per contributor
    /// metadata in the federation directory).
    Jurisdictional,
    /// Bias routing across distinct organizations.
    Organizational,
    /// No diversity preference; route purely by Expertise + Active tier.
    None,
}

/// Routing preferences — consumer hints into `MISSION.md` §3.3 steps 3–4.
/// Crate policy MAY override.
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

/// `deferral_request` payload per SCHEMA.md §4.7.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferralRequest {
    /// ULID identifier per §2.2.
    pub deferral_id: String,
    /// Expertise-granularity cell — `subject` field MUST be `None`.
    /// Redundant with envelope `subject.{domain,language}`; MUST match.
    pub cell: Cell,
    /// Requesting agent's federation identity.
    pub consumer_id: ContributorId,
    /// Optional back-reference to the consumer's internal task ID.
    /// Preserves CIRISNode WBD's `agent_task_id` audit anchor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_task_id: Option<String>,
    /// Short human label for routing UIs and aggregation grouping.
    pub title: String,
    /// The actual deferral content — what routed responders are asked to
    /// weigh in on.
    pub context: String,
    /// Constrains the `verdict` shape of routed responses.
    pub response_format: ResponseFormat,
    /// Soft hint; the §3.3 aggregate MAY exclude responders that have
    /// not responded by this time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<DateTime<Utc>>,
    /// Consumer hints into §3.3 steps 3–4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing_preferences: Option<RoutingPreferences>,
}

/// `deferral_response` payload per SCHEMA.md §4.8.
///
/// Routed responses are aggregated per Primitive 7 directly (no separate
/// `Vote`-on-response layer); each response carries its own weight per
/// §5.2: `Credits(domain, language, subject='deferral_response') ×
/// expertise_multiplier × active_tier_multiplier`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferralResponse {
    /// ULID identifier per §2.2.
    pub response_id: String,
    /// The originating `deferral_request` being answered.
    pub deferral_id: String,
    /// MUST match the originating `deferral_request.cell`.
    pub cell: Cell,
    /// Responder's federation identity. MUST appear in the routed set the
    /// crate selected per §3.3; out-of-set responses are rejected at append.
    pub responder_id: ContributorId,
    /// Shape constrained by the originating request's `response_format`.
    pub verdict: serde_json::Value,
    /// Free-text justification. Recorded in the audit chain per §5.1 step 8.
    pub rationale: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deferral_request_round_trip() {
        let req = DeferralRequest {
            deferral_id: "def_01HX5".into(),
            cell: Cell::expertise("mental_health", "am"),
            consumer_id: ContributorId::new("authorpubkey"),
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
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""response_format":"binary""#));
        assert!(json.contains(r#""diversity":"jurisdictional""#));
        let back: DeferralRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.deferral_id, req.deferral_id);
    }
}
