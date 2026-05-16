//! `assistance_response` payload — SCHEMA.md §4.19.
//!
//! Peer's response to an `assistance_request`. Any peer may respond;
//! the requester applies its own acceptance policy (trust grants,
//! reputation, etc.) to filter responses.

use serde::{Deserialize, Serialize};

/// `subject_kind` discriminator. Wire constant; matches SCHEMA §3.2.
pub const SUBJECT_KIND: &str = "assistance_response";

/// `assistance_response` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistanceResponsePayload {
    /// Back-ref to the originating `assistance_request` envelope's
    /// `contribution_id`.
    pub assistance_id: String,
    /// The reply. Shape constrained by the request's `response_format`.
    pub response: String,
    /// `0.0 ≤ x ≤ 1.0`. Responder's self-reported confidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// CIRISLensCore trace ids or evidence pointers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supporting_trace_refs: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_kind_constant_matches_schema() {
        assert_eq!(SUBJECT_KIND, "assistance_response");
    }

    #[test]
    fn round_trip() {
        let p = AssistanceResponsePayload {
            assistance_id: "01HX".into(),
            response: "Use የንግግር ሕክምና.".into(),
            confidence: Some(0.8),
            supporting_trace_refs: vec!["trace_01HX".into()],
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: AssistanceResponsePayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.assistance_id, p.assistance_id);
        assert_eq!(back.confidence, Some(0.8));
    }

    #[test]
    fn empty_trace_refs_omitted_on_wire() {
        let p = AssistanceResponsePayload {
            assistance_id: "01HX".into(),
            response: "x".into(),
            confidence: None,
            supporting_trace_refs: vec![],
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("supporting_trace_refs"));
    }
}
