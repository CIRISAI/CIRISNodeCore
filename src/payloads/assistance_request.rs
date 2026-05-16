//! `assistance_request` payload — SCHEMA.md §4.18.
//!
//! Peer-to-peer broadcast request for help. Distinct from §4.7
//! `deferral_request` (peer → trusted entity through the trust
//! hierarchy / WA routing): assistance is broadcast to all peers,
//! any peer may respond, no domain classification, no witness
//! diversity, no registry lookup. The lightweight pre-trust path.
//!
//! Encoded as a `proposal`-type Contribution with
//! `subject.subject_kind = "assistance_request"`. The envelope's
//! `contribution_id` is the `assistance_id` referenced by
//! [`crate::payloads::assistance_response::AssistanceResponsePayload`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::payloads::deferral::ResponseFormat;

/// `subject_kind` discriminator. Wire constant; matches SCHEMA §3.2.
pub const SUBJECT_KIND: &str = "assistance_request";

/// `assistance_request` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistanceRequestPayload {
    /// Short label for receiver-side filtering.
    pub title: String,
    /// The request body.
    pub context: String,
    /// Constrains the verdict shape of responses. Reuses the §4.7
    /// `ResponseFormat` enum.
    pub response_format: ResponseFormat,
    /// Soft hint; receivers MAY ignore late responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<DateTime<Utc>>,
    /// Free-form descriptor; non-enforced hint (e.g.
    /// `"amharic-mental-health"`). Filtering is receiver-side policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_audience: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_kind_constant_matches_schema() {
        assert_eq!(SUBJECT_KIND, "assistance_request");
    }

    #[test]
    fn round_trip() {
        let p = AssistanceRequestPayload {
            title: "Amharic medication terminology".into(),
            context: "Use ሳይኮተራፒ or የንግግር ሕክምና in Stage 2?".into(),
            response_format: ResponseFormat::Freeform,
            deadline: None,
            preferred_audience: Some("amharic-mental-health".into()),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains(r#""response_format":"freeform""#));
        let back: AssistanceRequestPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.title, p.title);
    }
}
