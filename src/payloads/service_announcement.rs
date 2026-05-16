//! `service_announcement` payload — SCHEMA.md §4.23.
//!
//! Service-offering advertisement per `FSD/MESSAGE_TAXONOMY.md` §5.
//! Durable Contribution stating "I offer this capability; here's how
//! to invoke me." Discoverable via
//! `list_contributions(subject_kind=service_announcement)`.
//!
//! Per-invocation RPC does NOT ride the audit chain — invocations go
//! over edge `MessageType::ServiceRequest` (proposed edge expansion).
//! Aggregated usage flows back to the chain via §4.25
//! `service_usage_summary`.

use serde::{Deserialize, Serialize};

/// `subject_kind` discriminator. Wire constant; matches SCHEMA §3.2.
pub const SUBJECT_KIND: &str = "service_announcement";

/// Service kind taxonomy. Canonical values cover the load-bearing
/// service classes; `Custom` covers operator-defined kinds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    /// LLM generation service.
    Llm,
    /// Vector embedding service.
    Embedding,
    /// Speech-to-text transcription service.
    Transcribe,
    /// Classifier service.
    Classifier,
    /// Tool / function-calling service.
    Tool,
    /// Operator-defined custom service kind. Encoded as
    /// `custom:<kind>` in scope grammar per trust-grant §4.14.
    #[serde(untagged)]
    Custom(String),
}

/// Endpoint descriptor. Multiple transports per service announcement
/// supported.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Transport identifier. Canonical: `reticulum` / `http` / `http_fallback`.
    pub transport: String,
    /// Transport-specific address (Reticulum destination hash,
    /// HTTPS URL, etc.).
    pub address: String,
}

/// `service_announcement` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAnnouncementPayload {
    /// Service kind discriminator.
    pub service_kind: ServiceKind,
    /// Per-offer human label. Distinct from `service_kind`.
    pub service_name: String,
    /// Service version. Bumps when capability surface changes.
    pub version: String,
    /// Service-specific capability descriptor. Schema varies per
    /// `service_kind`. Left as opaque JSON because the consumer
    /// (CIRISProxy, agent, etc.) decodes per-kind.
    pub capabilities: serde_json::Value,
    /// Endpoints. At least one required.
    pub endpoints: Vec<ServiceEndpoint>,
    /// Free-text terms-of-service / authorization-prerequisites note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terms: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_kind_constant_matches_schema() {
        assert_eq!(SUBJECT_KIND, "service_announcement");
    }

    #[test]
    fn service_kind_snake_case() {
        assert_eq!(serde_json::to_string(&ServiceKind::Llm).unwrap(), r#""llm""#);
        assert_eq!(serde_json::to_string(&ServiceKind::Embedding).unwrap(), r#""embedding""#);
        assert_eq!(serde_json::to_string(&ServiceKind::Tool).unwrap(), r#""tool""#);
    }

    #[test]
    fn custom_service_kind_round_trip() {
        let k = ServiceKind::Custom("audio_synthesis".into());
        let json = serde_json::to_string(&k).unwrap();
        assert_eq!(json, r#""audio_synthesis""#);
    }

    #[test]
    fn round_trip() {
        let p = ServiceAnnouncementPayload {
            service_kind: ServiceKind::Llm,
            service_name: "amharic_clinical_companion".into(),
            version: "1.0".into(),
            capabilities: serde_json::json!({
                "models": ["claude-opus-4-7"],
                "max_context_tokens": 200000
            }),
            endpoints: vec![ServiceEndpoint {
                transport: "reticulum".into(),
                address: "<reticulum-destination>".into(),
            }],
            terms: Some("Trust grant required.".into()),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: ServiceAnnouncementPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.service_name, p.service_name);
        assert_eq!(back.endpoints.len(), 1);
    }
}
