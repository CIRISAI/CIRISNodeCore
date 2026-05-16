//! `test_result` payload — SCHEMA.md §4.15.
//!
//! Result of running an `arc_question` (§4.1) or `proposed_battery`
//! (§4.2) against an agent. Typed evidence feeding the Coherence
//! Ratchet rather than inferred from generic `proposal` envelopes.
//!
//! Encoded as a `proposal`-type Contribution with
//! `subject.subject_kind = "test_result"`. Author is the scorer's
//! key (foundation-model judge or calibrated scoring agent).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `subject_kind` discriminator. Wire constant; matches SCHEMA §3.2.
pub const SUBJECT_KIND: &str = "test_result";

/// `test_result` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResultPayload {
    /// Matches §4.1 `question_id`.
    pub question_id: String,
    /// Question version at scoring time.
    pub question_version: u32,
    /// Hybrid pubkey of the scored agent.
    pub agent_under_test: String,
    /// Reference into CIRISLensCore's trace store.
    pub trace_id: String,
    /// When the scoring pass produced this result.
    pub scored_at: DateTime<Utc>,
    /// `faculty_target → score` per §4.1 `faculty_targets`.
    /// `BTreeMap` for deterministic JSON serialization (canonical-bytes
    /// stability).
    pub scores: BTreeMap<String, f64>,
    /// Rubric U-codes the agent hit. Empty = no hard fails.
    #[serde(default)]
    pub hard_fail_hits: Vec<String>,
    /// Soft-fail rubric criteria the agent hit. Empty = no soft fails.
    #[serde(default)]
    pub soft_fail_hits: Vec<String>,
}

impl TestResultPayload {
    /// True if any hard-fail trigger fired. Hard fails block release
    /// per the safety-battery CI loop discipline.
    pub fn has_hard_fail(&self) -> bool {
        !self.hard_fail_hits.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_kind_constant_matches_schema() {
        assert_eq!(SUBJECT_KIND, "test_result");
    }

    #[test]
    fn round_trip() {
        let mut scores = BTreeMap::new();
        scores.insert("EthicalPDMAEvaluator".into(), 0.78);
        scores.insert("epistemic_humility_conscience".into(), 0.91);

        let p = TestResultPayload {
            question_id: "am_mh_v4_q01".into(),
            question_version: 1,
            agent_under_test: "agent_pub_b64".into(),
            trace_id: "trace_01HX".into(),
            scored_at: Utc::now(),
            scores,
            hard_fail_hits: vec!["U2".into()],
            soft_fail_hits: vec![],
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: TestResultPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.question_id, p.question_id);
        assert!(back.has_hard_fail());
        assert_eq!(back.scores.len(), 2);
    }

    #[test]
    fn passing_result_has_no_hard_fail() {
        let p = TestResultPayload {
            question_id: "am_mh_v4_q02".into(),
            question_version: 1,
            agent_under_test: "agent".into(),
            trace_id: "t".into(),
            scored_at: Utc::now(),
            scores: BTreeMap::new(),
            hard_fail_hits: vec![],
            soft_fail_hits: vec![],
        };
        assert!(!p.has_hard_fail());
    }
}
