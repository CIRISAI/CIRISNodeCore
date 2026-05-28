//! Phase 2 read-composition logic for the Epistemic Commons Framework UI
//! (CIRISAgent#800 / CIRISNodeCore#12).
//!
//! **Pure aggregation** — these functions take raw attestation JSON and
//! return UI-shaped JSON. They do not hold engine handles or perform I/O.
//!
//! The [`crate::python`] PyO3 wrappers accept an injected persist Engine
//! handle, call directly into persist's PyO3 surface for the data, then
//! aggregate here. Engine discipline (CIRISNodeCore#4): NodeCore never
//! *constructs* an engine; injected ones are the cohabitation pattern.
//!
//! Aggregation logic lives in this module (not [`crate::python`]) so unit
//! tests link without the pyo3 `extension-module` feature.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Minimal projection of `persist::federation::types::Attestation` —
/// only the fields read-composition consumes. Persist's full struct
/// carries scrub signatures + canonical hashes that NodeCore does not
/// need for aggregation (signatures verified at persist's admission
/// gate).
#[derive(Deserialize)]
pub(crate) struct AttestationRow {
    pub attestation_type: String,
    pub asserted_at: DateTime<Utc>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    pub attestation_envelope: serde_json::Value,
}

impl AttestationRow {
    fn dimension(&self) -> Option<&str> {
        self.attestation_envelope.get("dimension")?.as_str()
    }

    fn score(&self) -> Option<f64> {
        self.attestation_envelope.get("score")?.as_f64()
    }

    fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        self.expires_at.map_or(true, |exp| now <= exp)
    }
}

/// UI-shaped agent state per CIRISAgent#800 ProfileScorecard.
#[derive(Serialize, Default)]
pub(crate) struct AgentStateOutput {
    pub key_id: String,
    /// Credits totals keyed by `"{domain}/{language}/{subject}"`.
    /// Sum of positive scores on active `credits:*` attestations targeting
    /// the key.
    pub credits: HashMap<String, f64>,
    /// Expertise standings keyed by `"{domain}/{language}"`.
    /// Latest non-expired score on `expertise:*` attestations targeting
    /// the key.
    pub expertise: HashMap<String, f64>,
    /// Latest non-expired `activity_tier:*` reading mapped to a human
    /// label: `score > 0.5` → `"active"`; otherwise `"below_active"`.
    /// `None` if unknown.
    pub activity_tier: Option<String>,
    /// When the composition ran. Lets the UI staleness-check.
    pub computed_at: DateTime<Utc>,
}

/// Compose UI-ready agent state from raw persist attestation rows.
///
/// **Input**: JSON-serialized output of persist's
/// `list_attestations_for(key_id)` — a list of attestation rows targeting
/// `key_id`.
///
/// **Output**: JSON object matching [`AgentStateOutput`].
///
/// **Semantics** (Phase 2 v0.1 — simple aggregations; sophisticated
/// weighting per FSD-002 v1.4 §6 composition policies is future work):
/// - `credits:{domain}:{language}:{subject}` — sum of positive scores
///   from active attestations
/// - `expertise:{domain}:{language}` — latest score by `asserted_at` from
///   active attestations
/// - `activity_tier:{period}` — latest active score, mapped to label
///
/// Attestations of other types (`delegates_to` / `supersedes` /
/// `withdraws` / `recants`) and other dimension prefixes are ignored —
/// they do not contribute to agent state. Callers concerned with
/// lifecycle (e.g., `withdraws`-aware aggregation) can pre-filter at
/// persist or compose with the dedicated lifecycle surface (Phase 2
/// follow-up).
pub fn compose_agent_state(
    key_id: String,
    attestations_json: &str,
) -> Result<String, serde_json::Error> {
    compose_agent_state_at(key_id, attestations_json, Utc::now())
}

/// Test-friendly variant accepting an explicit `now`. Production callers
/// use [`compose_agent_state`].
pub(crate) fn compose_agent_state_at(
    key_id: String,
    attestations_json: &str,
    now: DateTime<Utc>,
) -> Result<String, serde_json::Error> {
    let rows: Vec<AttestationRow> = serde_json::from_str(attestations_json)?;

    let mut out = AgentStateOutput {
        key_id,
        computed_at: now,
        ..Default::default()
    };

    let mut expertise_latest: HashMap<String, (DateTime<Utc>, f64)> = HashMap::new();
    let mut activity_latest: Option<(DateTime<Utc>, f64)> = None;

    for row in rows {
        if row.attestation_type != "scores" || !row.is_active_at(now) {
            continue;
        }
        let Some(dim) = row.dimension() else { continue };
        let Some(score) = row.score() else { continue };

        if let Some(rest) = dim.strip_prefix("credits:") {
            if score > 0.0 {
                *out.credits.entry(rest.replace(':', "/")).or_insert(0.0) += score;
            }
        } else if let Some(rest) = dim.strip_prefix("expertise:") {
            let cell = rest.replace(':', "/");
            match expertise_latest.get(&cell) {
                Some((prior_ts, _)) if *prior_ts >= row.asserted_at => {}
                _ => {
                    expertise_latest.insert(cell, (row.asserted_at, score));
                }
            }
        } else if dim.starts_with("activity_tier:") {
            match activity_latest {
                Some((prior_ts, _)) if prior_ts >= row.asserted_at => {}
                _ => activity_latest = Some((row.asserted_at, score)),
            }
        }
    }

    out.expertise = expertise_latest
        .into_iter()
        .map(|(cell, (_, s))| (cell, s))
        .collect();
    out.activity_tier = activity_latest
        .map(|(_, s)| if s > 0.5 { "active" } else { "below_active" }.to_owned());

    serde_json::to_string(&out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn att_json(rows: serde_json::Value) -> String {
        rows.to_string()
    }

    fn fixed_now() -> DateTime<Utc> {
        "2026-05-27T00:00:00Z".parse().unwrap()
    }

    #[test]
    fn sums_credits_and_picks_latest_expertise() {
        let rows = att_json(serde_json::json!([
            { "attestation_type": "scores", "asserted_at": "2026-05-01T00:00:00Z",
              "attestation_envelope": {"dimension": "credits:mental_health:en:arc_question", "score": 5.0} },
            { "attestation_type": "scores", "asserted_at": "2026-05-02T00:00:00Z",
              "attestation_envelope": {"dimension": "credits:mental_health:en:arc_question", "score": 3.0} },
            { "attestation_type": "scores", "asserted_at": "2026-05-01T00:00:00Z",
              "attestation_envelope": {"dimension": "expertise:mental_health:en", "score": 0.4} },
            { "attestation_type": "scores", "asserted_at": "2026-05-15T00:00:00Z",
              "attestation_envelope": {"dimension": "expertise:mental_health:en", "score": 0.8} },
            { "attestation_type": "scores", "asserted_at": "2026-05-20T00:00:00Z",
              "attestation_envelope": {"dimension": "activity_tier:30d", "score": 0.9} }
        ]));

        let out = compose_agent_state_at("key-foo".into(), &rows, fixed_now()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();

        assert_eq!(parsed["key_id"], "key-foo");
        assert_eq!(parsed["credits"]["mental_health/en/arc_question"].as_f64().unwrap(), 8.0);
        assert_eq!(parsed["expertise"]["mental_health/en"].as_f64().unwrap(), 0.8);
        assert_eq!(parsed["activity_tier"], "active");
    }

    #[test]
    fn skips_expired_and_negative_credits() {
        let rows = att_json(serde_json::json!([
            { "attestation_type": "scores", "asserted_at": "2020-01-01T00:00:00Z",
              "expires_at": "2020-12-31T00:00:00Z",
              "attestation_envelope": {"dimension": "credits:test:en:s", "score": 100.0} },
            { "attestation_type": "scores", "asserted_at": "2026-05-01T00:00:00Z",
              "attestation_envelope": {"dimension": "credits:test:en:s", "score": -2.0} },
            { "attestation_type": "scores", "asserted_at": "2026-05-01T00:00:00Z",
              "attestation_envelope": {"dimension": "activity_tier:30d", "score": 0.2} }
        ]));

        let out = compose_agent_state_at("key-bar".into(), &rows, fixed_now()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();

        assert!(parsed["credits"].as_object().unwrap().is_empty());
        assert_eq!(parsed["activity_tier"], "below_active");
    }

    #[test]
    fn ignores_non_scores_and_other_dimensions() {
        let rows = att_json(serde_json::json!([
            { "attestation_type": "delegates_to", "asserted_at": "2026-05-01T00:00:00Z",
              "attestation_envelope": {"dimension": "credits:foo:en:s", "score": 9.0} },
            { "attestation_type": "scores", "asserted_at": "2026-05-01T00:00:00Z",
              "attestation_envelope": {"dimension": "need:foo:witness", "score": 0.9} }
        ]));

        let out = compose_agent_state_at("key-empty".into(), &rows, fixed_now()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();

        assert!(parsed["credits"].as_object().unwrap().is_empty());
        assert!(parsed["expertise"].as_object().unwrap().is_empty());
        assert!(parsed["activity_tier"].is_null());
    }
}
