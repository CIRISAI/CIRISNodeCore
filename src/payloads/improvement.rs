//! `improvement` payload — SCHEMA.md §4.16.
//!
//! Substrate or content improvement proposal that doesn't fit
//! `prompt_edit` / `guide_edit` / `accord_edit` — tooling, infra,
//! schema, build, CI, etc. The escape hatch for improvements that
//! don't decompose cleanly onto the existing edit-proposal kinds.
//!
//! Encoded as a `proposal`-type Contribution with
//! `subject.subject_kind = "improvement"`. Witness-set required at
//! envelope level per SCHEMA §3.5 (same discipline as other edit
//! proposals).

use serde::{Deserialize, Serialize};

/// `subject_kind` discriminator. Wire constant; matches SCHEMA §3.2.
pub const SUBJECT_KIND: &str = "improvement";

/// `improvement` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementPayload {
    /// Free-form category. Canonical values: `tooling`, `schema`,
    /// `infra`, `build`, `ci`. Operators MAY introduce additional
    /// values.
    pub target_kind: String,
    /// Repo + path or component identifier the improvement targets.
    pub target_ref: String,
    /// Free-text justification recorded on the audit chain.
    pub rationale: String,
    /// Unified diff if applicable. Absent for design-only proposals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_kind_constant_matches_schema() {
        assert_eq!(SUBJECT_KIND, "improvement");
    }

    #[test]
    fn round_trip() {
        let p = ImprovementPayload {
            target_kind: "tooling".into(),
            target_ref: "CIRISAgent/qa_runner/safety_battery.py".into(),
            rationale: "Add structured-output mode so per-faculty scores ride a typed channel."
                .into(),
            diff: Some("--- a/foo.py\n+++ b/foo.py\n@@ ...".into()),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: ImprovementPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.target_kind, "tooling");
        assert!(back.diff.is_some());
    }

    #[test]
    fn design_only_proposal_omits_diff() {
        let p = ImprovementPayload {
            target_kind: "schema".into(),
            target_ref: "CIRISNodeCore/SCHEMA.md §5".into(),
            rationale: "Vote score shape should be typed per subject_kind.".into(),
            diff: None,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("diff"));
    }
}
