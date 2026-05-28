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
    pub(crate) fn dimension(&self) -> Option<&str> {
        self.attestation_envelope.get("dimension")?.as_str()
    }

    pub(crate) fn score(&self) -> Option<f64> {
        self.attestation_envelope.get("score")?.as_f64()
    }

    pub(crate) fn is_active_at(&self, now: DateTime<Utc>) -> bool {
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

// ---------------------------------------------------------------------------
// Surface 2 — needs_feed (Participate screen, CIRISAgent#800)
// ---------------------------------------------------------------------------

/// One entry in the federation needs feed — a `need:{domain}:{kind}`
/// attestation reshaped for the Participate UI.
#[derive(Serialize)]
pub(crate) struct NeedEntry {
    pub need_id: String,
    /// The entity that has the stated need (`attested_key_id`).
    pub needer_key_id: String,
    /// The entity claiming the need exists (`attesting_key_id` — often
    /// the same as `needer_key_id` for self-declared needs).
    pub attesting_key_id: String,
    /// `{domain}` from the dimension (e.g., `mental_health:en`).
    pub domain: String,
    /// `{kind}` from the dimension (e.g., `witness`, `method_contributor`).
    pub kind: String,
    /// Urgency = score magnitude (positive = active call).
    pub urgency: f64,
    pub asserted_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
    /// Free-form description from `context.description` if present.
    pub description: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct NeedsFeedOutput {
    pub needs: Vec<NeedEntry>,
    pub computed_at: DateTime<Utc>,
}

/// Compose the Participate feed from `need:{domain}:{kind}` attestations.
///
/// **Input**: JSON-serialized `Vec<Attestation>` — typically the output of
/// `engine.list_attestations(filter)` with a dimension-prefix filter on
/// `need:*`. Caller may pre-filter by domain / kind via the persist filter.
///
/// **Filter** (optional JSON object):
/// - `domain`: only include needs whose `{domain}` matches
/// - `kind`: only include needs whose `{kind}` matches
/// - `active_only`: default `true`; if false, includes expired needs too
///
/// Lifecycle attestations (`withdraws` / `recants` / `supersedes`) are not
/// composed here — caller should query for active scores attestations only.
/// (Persist's federation directory already excludes withdrawn rows from
/// the default scoped list.)
pub fn compose_needs_feed(
    attestations_json: &str,
    filter_json: &str,
) -> Result<String, serde_json::Error> {
    compose_needs_feed_at(attestations_json, filter_json, Utc::now())
}

pub(crate) fn compose_needs_feed_at(
    attestations_json: &str,
    filter_json: &str,
    now: DateTime<Utc>,
) -> Result<String, serde_json::Error> {
    let rows: Vec<AttestationRow> = serde_json::from_str(attestations_json)?;
    let filter: serde_json::Value = if filter_json.trim().is_empty() {
        serde_json::Value::Object(Default::default())
    } else {
        serde_json::from_str(filter_json)?
    };

    let domain_filter = filter.get("domain").and_then(|v| v.as_str());
    let kind_filter = filter.get("kind").and_then(|v| v.as_str());
    let active_only = filter
        .get("active_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let mut needs = Vec::new();
    for row in rows {
        if row.attestation_type != "scores" {
            continue;
        }
        if active_only && !row.is_active_at(now) {
            continue;
        }
        let Some(dim) = row.dimension() else { continue };
        let Some(rest) = dim.strip_prefix("need:") else {
            continue;
        };
        // rest = "{domain}:{kind}" — `{domain}` may itself contain colons
        // (e.g., "mental_health:en"); `{kind}` is a single enumerated
        // segment without colons. Split off the trailing kind.
        let Some((domain, kind)) = rest.rsplit_once(':') else {
            continue;
        };
        if domain_filter.is_some_and(|d| d != domain) {
            continue;
        }
        if kind_filter.is_some_and(|k| k != kind) {
            continue;
        }
        let Some(score) = row.score() else { continue };
        if score <= 0.0 {
            continue;
        }

        let env = &row.attestation_envelope;
        needs.push(NeedEntry {
            need_id: env
                .get("attestation_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            needer_key_id: env
                .get("attested_key_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            attesting_key_id: env
                .get("attesting_key_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            domain: domain.to_owned(),
            kind: kind.to_owned(),
            urgency: score,
            asserted_at: row.asserted_at,
            deadline: row.expires_at,
            description: env
                .get("context")
                .and_then(|v| v.as_str())
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .and_then(|c| {
                    c.get("description")
                        .and_then(|d| d.as_str())
                        .map(str::to_owned)
                }),
        });
    }

    // Sort by urgency desc, then by recency desc
    needs.sort_by(|a, b| {
        b.urgency
            .partial_cmp(&a.urgency)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.asserted_at.cmp(&a.asserted_at))
    });

    serde_json::to_string(&NeedsFeedOutput {
        needs,
        computed_at: now,
    })
}

// ---------------------------------------------------------------------------
// Surface 3 — contribution detail (The Commons card, CIRISAgent#800)
// ---------------------------------------------------------------------------

#[derive(Serialize, Default)]
pub(crate) struct ContributionDetailOutput {
    pub contribution_id: String,
    /// Sum of vote scores from non-expired `vote:{contribution_id}` attestations.
    pub vote_tally: f64,
    /// Number of distinct attesters who voted.
    pub vote_count: u32,
    /// Latest `weighted_aggregate:{contribution_id}` score, if any.
    pub weighted_aggregate: Option<f64>,
    /// Latest `witness_diversity:{contribution_id}` score, if any.
    pub witness_diversity: Option<f64>,
    /// Latest `truth_grounding:{subject}` value for the contribution's
    /// subject — if the consumer included these attestations in the input.
    pub truth_grounding: Option<f64>,
    /// `testimonial_witness:{kind}` narratives preserved for this
    /// contribution. Per FSD-002 v1.4 §3.6.3: preservation-only, never
    /// aggregated.
    pub testimonial_witnesses: Vec<TestimonialWitnessEntry>,
    pub computed_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub(crate) struct TestimonialWitnessEntry {
    pub witness_key_id: String,
    /// `{kind}` — `harmed_party` / `whistleblower` / `displaced_worker` / etc.
    pub kind: String,
    pub asserted_at: DateTime<Utc>,
}

/// Compose The Commons card detail for one Contribution.
///
/// **Input**: JSON-serialized `Vec<Attestation>` — typically the output of
/// `engine.list_attestations(filter)` with a contribution-context filter
/// that pulls every attestation referencing `contribution_id`.
///
/// The compose function buckets by dimension prefix:
/// - `vote:{contribution_id}` — sums into `vote_tally`, counts attesters
/// - `weighted_aggregate:{contribution_id}` — latest score
/// - `witness_diversity:{contribution_id}` — latest score
/// - `truth_grounding:*` — latest score (caller must provide the right ones)
/// - `testimonial_witness:{kind}` — preserves narratives (no aggregation)
pub fn compose_contribution(
    contribution_id: String,
    attestations_json: &str,
) -> Result<String, serde_json::Error> {
    compose_contribution_at(contribution_id, attestations_json, Utc::now())
}

pub(crate) fn compose_contribution_at(
    contribution_id: String,
    attestations_json: &str,
    now: DateTime<Utc>,
) -> Result<String, serde_json::Error> {
    let rows: Vec<AttestationRow> = serde_json::from_str(attestations_json)?;

    let mut out = ContributionDetailOutput {
        contribution_id: contribution_id.clone(),
        computed_at: now,
        ..Default::default()
    };
    let mut vote_attesters = std::collections::HashSet::<String>::new();
    let mut aggregate_latest: Option<(DateTime<Utc>, f64)> = None;
    let mut diversity_latest: Option<(DateTime<Utc>, f64)> = None;
    let mut grounding_latest: Option<(DateTime<Utc>, f64)> = None;
    let vote_prefix = format!("vote:{contribution_id}");
    let aggregate_prefix = format!("weighted_aggregate:{contribution_id}");
    let diversity_prefix = format!("witness_diversity:{contribution_id}");

    for row in rows {
        if row.attestation_type != "scores" || !row.is_active_at(now) {
            continue;
        }
        let Some(dim) = row.dimension() else { continue };
        let Some(score) = row.score() else { continue };

        if dim == vote_prefix {
            out.vote_tally += score;
            if let Some(k) = row
                .attestation_envelope
                .get("attesting_key_id")
                .and_then(|v| v.as_str())
            {
                vote_attesters.insert(k.to_owned());
            }
        } else if dim == aggregate_prefix {
            match aggregate_latest {
                Some((t, _)) if t >= row.asserted_at => {}
                _ => aggregate_latest = Some((row.asserted_at, score)),
            }
        } else if dim == diversity_prefix {
            match diversity_latest {
                Some((t, _)) if t >= row.asserted_at => {}
                _ => diversity_latest = Some((row.asserted_at, score)),
            }
        } else if dim.starts_with("truth_grounding:") {
            match grounding_latest {
                Some((t, _)) if t >= row.asserted_at => {}
                _ => grounding_latest = Some((row.asserted_at, score)),
            }
        } else if let Some(kind) = dim.strip_prefix("testimonial_witness:") {
            if let Some(witness_key) = row
                .attestation_envelope
                .get("attesting_key_id")
                .and_then(|v| v.as_str())
            {
                out.testimonial_witnesses.push(TestimonialWitnessEntry {
                    witness_key_id: witness_key.to_owned(),
                    kind: kind.to_owned(),
                    asserted_at: row.asserted_at,
                });
            }
        }
    }

    out.vote_count = vote_attesters.len() as u32;
    out.weighted_aggregate = aggregate_latest.map(|(_, s)| s);
    out.witness_diversity = diversity_latest.map(|(_, s)| s);
    out.truth_grounding = grounding_latest.map(|(_, s)| s);
    out.testimonial_witnesses
        .sort_by(|a, b| b.asserted_at.cmp(&a.asserted_at));

    serde_json::to_string(&out)
}

// ---------------------------------------------------------------------------
// Surface 4 — decision_hierarchy (Constitutional / Accord screen)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub(crate) struct DecisionLevelEntry {
    pub attestation_id: String,
    pub attested_key_id: String,
    /// The dimensional value at this level — e.g., `species` for `goal:species`,
    /// the parent goal_id for `approach:{goal_id}`, etc.
    pub key: String,
    /// Latest score on the dimension; the Goal level reports the 𝒞_CIRIS
    /// composite if computed externally.
    pub score: f64,
    pub asserted_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub(crate) struct DecisionHierarchyOutput {
    pub goal_id: String,
    pub goals: Vec<DecisionLevelEntry>,
    pub approaches: Vec<DecisionLevelEntry>,
    pub methods: Vec<DecisionLevelEntry>,
    pub progress_measures: Vec<DecisionLevelEntry>,
    pub computed_at: DateTime<Utc>,
}

/// Compose the upward-only DAG (Goal ← Approach ← Method ← Progress Measure)
/// rooted at a specific `goal_id` for the Constitutional UI.
///
/// **Input** (caller queries persist for each level separately, then passes
/// all four lists):
/// - `goal_attestations_json`: attestations with dimension prefix `goal:`
///   (filtered to the goal_id if possible, otherwise compose filters)
/// - `approach_attestations_json`: attestations with dimension prefix `approach:`
/// - `method_attestations_json`: attestations with dimension prefix `method:`
/// - `measure_attestations_json`: attestations with dimension prefix `progress_measure:`
///
/// Cross-level linking is by the `{parent_id}` suffix in each dimension:
/// - `approach:{goal_id}` links to its goal via the dimension suffix
/// - `method:{approach_id}:{substrate_rung}` links to its approach
/// - `progress_measure:{method_id}` links to its method
pub fn compose_decision_hierarchy(
    goal_id: String,
    goal_attestations_json: &str,
    approach_attestations_json: &str,
    method_attestations_json: &str,
    measure_attestations_json: &str,
) -> Result<String, serde_json::Error> {
    compose_decision_hierarchy_at(
        goal_id,
        goal_attestations_json,
        approach_attestations_json,
        method_attestations_json,
        measure_attestations_json,
        Utc::now(),
    )
}

pub(crate) fn compose_decision_hierarchy_at(
    goal_id: String,
    goal_attestations_json: &str,
    approach_attestations_json: &str,
    method_attestations_json: &str,
    measure_attestations_json: &str,
    now: DateTime<Utc>,
) -> Result<String, serde_json::Error> {
    fn parse(s: &str) -> Result<Vec<AttestationRow>, serde_json::Error> {
        serde_json::from_str(s)
    }

    let goal_rows = parse(goal_attestations_json)?;
    let approach_rows = parse(approach_attestations_json)?;
    let method_rows = parse(method_attestations_json)?;
    let measure_rows = parse(measure_attestations_json)?;

    let entry_from = |row: &AttestationRow, key: String| -> Option<DecisionLevelEntry> {
        let score = row.score()?;
        let attestation_id = row
            .attestation_envelope
            .get("attestation_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        let attested_key_id = row
            .attestation_envelope
            .get("attested_key_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        Some(DecisionLevelEntry {
            attestation_id,
            attested_key_id,
            key,
            score,
            asserted_at: row.asserted_at,
        })
    };

    let bucket_by_suffix = |rows: Vec<AttestationRow>, prefix: &str| -> Vec<DecisionLevelEntry> {
        let mut out = Vec::new();
        for row in rows {
            if row.attestation_type != "scores" || !row.is_active_at(now) {
                continue;
            }
            let Some(dim) = row.dimension() else { continue };
            let Some(suffix) = dim.strip_prefix(prefix) else {
                continue;
            };
            if let Some(entry) = entry_from(&row, suffix.to_owned()) {
                out.push(entry);
            }
        }
        out
    };

    let goals = bucket_by_suffix(goal_rows, "goal:")
        .into_iter()
        // Goals level can include sibling scales for the same goal_id — caller
        // is responsible for restricting if they want just one.
        .collect();
    let mut approaches = bucket_by_suffix(approach_rows, "approach:");
    let mut methods = bucket_by_suffix(method_rows, "method:");
    let mut progress_measures = bucket_by_suffix(measure_rows, "progress_measure:");

    // Restrict approaches to those linking to this goal (suffix == goal_id).
    approaches.retain(|e| e.key == goal_id);
    let approach_ids: std::collections::HashSet<String> =
        approaches.iter().map(|e| e.attestation_id.clone()).collect();
    // method:{approach_id}:{substrate_rung} — first segment is approach_id
    methods.retain(|e| {
        e.key
            .split_once(':')
            .map(|(approach_id, _)| approach_ids.contains(approach_id))
            .unwrap_or(false)
    });
    let method_ids: std::collections::HashSet<String> =
        methods.iter().map(|e| e.attestation_id.clone()).collect();
    progress_measures.retain(|e| method_ids.contains(&e.key));

    serde_json::to_string(&DecisionHierarchyOutput {
        goal_id,
        goals,
        approaches,
        methods,
        progress_measures,
        computed_at: now,
    })
}

// ---------------------------------------------------------------------------
// Surface 5 — wa_state (Wise Authority screen)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub(crate) struct WaEvent {
    pub attestation_id: String,
    /// The target of the moderation/slashing/reconsideration (`attested_key_id`).
    pub target_key_id: String,
    pub attesting_key_id: String,
    /// The category from the dimension: e.g., `rogue_vote` for
    /// `moderation:rogue_vote`, `PROVEN_ROGUE` for `slashing:PROVEN_ROGUE`,
    /// `new_evidence` for `reconsideration:new_evidence`.
    pub category: String,
    pub score: f64,
    pub asserted_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub(crate) struct WaStateOutput {
    pub domain: String,
    pub language: String,
    pub moderation_queue: Vec<WaEvent>,
    pub slashing_history: Vec<WaEvent>,
    pub reconsideration_appeals: Vec<WaEvent>,
    pub computed_at: DateTime<Utc>,
}

/// Compose the Wise Authority screen state for one (domain, language) cell.
///
/// **Input** (caller queries persist for each prefix separately):
/// - `moderation_attestations_json`: `moderation:{allegation_type}` attestations
/// - `slashing_attestations_json`: `slashing:{outcome}` attestations
/// - `reconsideration_attestations_json`: `reconsideration:{grounds}` attestations
///
/// Cell scoping is applied via `context.cell` (if present in the envelope's
/// `context` JSON string): caller may pre-filter at persist for efficiency,
/// or compose filters here.
pub fn compose_wa_state(
    domain: String,
    language: String,
    moderation_attestations_json: &str,
    slashing_attestations_json: &str,
    reconsideration_attestations_json: &str,
) -> Result<String, serde_json::Error> {
    compose_wa_state_at(
        domain,
        language,
        moderation_attestations_json,
        slashing_attestations_json,
        reconsideration_attestations_json,
        Utc::now(),
    )
}

pub(crate) fn compose_wa_state_at(
    domain: String,
    language: String,
    moderation_attestations_json: &str,
    slashing_attestations_json: &str,
    reconsideration_attestations_json: &str,
    now: DateTime<Utc>,
) -> Result<String, serde_json::Error> {
    fn bucket(
        attestations_json: &str,
        prefix: &str,
        now: DateTime<Utc>,
        domain: &str,
        language: &str,
    ) -> Result<Vec<WaEvent>, serde_json::Error> {
        let rows: Vec<AttestationRow> = serde_json::from_str(attestations_json)?;
        let mut out = Vec::new();
        for row in rows {
            if row.attestation_type != "scores" || !row.is_active_at(now) {
                continue;
            }
            let Some(dim) = row.dimension() else { continue };
            let Some(category) = dim.strip_prefix(prefix) else {
                continue;
            };
            let Some(score) = row.score() else { continue };

            // Cell-scope filter: read context.cell.{domain,language} if present.
            let in_cell = row
                .attestation_envelope
                .get("context")
                .and_then(|v| v.as_str())
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .and_then(|c| c.get("cell").cloned())
                .map(|cell| {
                    cell.get("domain").and_then(|d| d.as_str()) == Some(domain)
                        && cell.get("language").and_then(|l| l.as_str()) == Some(language)
                })
                .unwrap_or(false);
            if !in_cell {
                continue;
            }

            out.push(WaEvent {
                attestation_id: row
                    .attestation_envelope
                    .get("attestation_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned(),
                target_key_id: row
                    .attestation_envelope
                    .get("attested_key_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned(),
                attesting_key_id: row
                    .attestation_envelope
                    .get("attesting_key_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned(),
                category: category.to_owned(),
                score,
                asserted_at: row.asserted_at,
            });
        }
        out.sort_by(|a, b| b.asserted_at.cmp(&a.asserted_at));
        Ok(out)
    }

    let moderation_queue = bucket(moderation_attestations_json, "moderation:", now, &domain, &language)?;
    let slashing_history = bucket(slashing_attestations_json, "slashing:", now, &domain, &language)?;
    let reconsideration_appeals = bucket(
        reconsideration_attestations_json,
        "reconsideration:",
        now,
        &domain,
        &language,
    )?;

    serde_json::to_string(&WaStateOutput {
        domain,
        language,
        moderation_queue,
        slashing_history,
        reconsideration_appeals,
        computed_at: now,
    })
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

    // --- needs_feed -------------------------------------------------------

    #[test]
    fn needs_feed_filters_by_domain_and_kind() {
        let rows = att_json(serde_json::json!([
            { "attestation_type": "scores", "asserted_at": "2026-05-20T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "need:mental_health:en:witness", "score": 0.9,
                  "attestation_id": "n-1", "attested_key_id": "k-a", "attesting_key_id": "k-a",
                  "context": "{\"description\":\"need a WA in am cell\"}"
              } },
            { "attestation_type": "scores", "asserted_at": "2026-05-20T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "need:legal:en:method_contributor", "score": 0.4,
                  "attestation_id": "n-2", "attested_key_id": "k-b", "attesting_key_id": "k-b"
              } },
            { "attestation_type": "scores", "asserted_at": "2026-05-20T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "need:mental_health:en:witness", "score": -0.5,
                  "attestation_id": "n-3", "attested_key_id": "k-c", "attesting_key_id": "k-c"
              } }
        ]));

        let filter = serde_json::json!({"domain": "mental_health:en", "kind": "witness"}).to_string();
        let out = compose_needs_feed_at(&rows, &filter, fixed_now()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();

        let needs = parsed["needs"].as_array().unwrap();
        // n-3 (negative score) filtered; n-2 (wrong domain) filtered; only n-1 remains.
        assert_eq!(needs.len(), 1);
        assert_eq!(needs[0]["need_id"], "n-1");
        assert_eq!(needs[0]["urgency"].as_f64().unwrap(), 0.9);
        assert_eq!(needs[0]["description"], "need a WA in am cell");
    }

    // --- contribution detail ---------------------------------------------

    #[test]
    fn contribution_sums_votes_and_collects_testimonials() {
        let rows = att_json(serde_json::json!([
            { "attestation_type": "scores", "asserted_at": "2026-05-15T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "vote:c-1", "score": 0.8, "attesting_key_id": "v-1" } },
            { "attestation_type": "scores", "asserted_at": "2026-05-15T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "vote:c-1", "score": 0.5, "attesting_key_id": "v-2" } },
            { "attestation_type": "scores", "asserted_at": "2026-05-16T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "weighted_aggregate:c-1", "score": 0.71 } },
            { "attestation_type": "scores", "asserted_at": "2026-05-15T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "testimonial_witness:harmed_party", "score": 1.0,
                  "attesting_key_id": "w-1" } },
            { "attestation_type": "scores", "asserted_at": "2026-05-15T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "vote:other-contribution", "score": 9.0, "attesting_key_id": "v-3" } }
        ]));

        let out = compose_contribution_at("c-1".into(), &rows, fixed_now()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();

        assert_eq!(parsed["vote_tally"].as_f64().unwrap(), 1.3);
        assert_eq!(parsed["vote_count"].as_u64().unwrap(), 2);
        assert_eq!(parsed["weighted_aggregate"].as_f64().unwrap(), 0.71);
        assert_eq!(parsed["testimonial_witnesses"].as_array().unwrap().len(), 1);
    }

    // --- decision_hierarchy ----------------------------------------------

    #[test]
    fn decision_hierarchy_walks_dag_from_goal() {
        let goals = att_json(serde_json::json!([
            { "attestation_type": "scores", "asserted_at": "2026-05-01T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "goal:species", "score": 0.8,
                  "attestation_id": "g-1", "attested_key_id": "k-agent" } }
        ]));
        let approaches = att_json(serde_json::json!([
            { "attestation_type": "scores", "asserted_at": "2026-05-02T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "approach:species", "score": 0.7,
                  "attestation_id": "a-1", "attested_key_id": "k-agent" } },
            { "attestation_type": "scores", "asserted_at": "2026-05-02T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "approach:family", "score": 0.5,
                  "attestation_id": "a-2", "attested_key_id": "k-agent" } }
        ]));
        let methods = att_json(serde_json::json!([
            { "attestation_type": "scores", "asserted_at": "2026-05-03T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "method:a-1:A3", "score": 0.6,
                  "attestation_id": "m-1", "attested_key_id": "k-agent" } },
            { "attestation_type": "scores", "asserted_at": "2026-05-03T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "method:a-2:A4", "score": 0.5,
                  "attestation_id": "m-2", "attested_key_id": "k-agent" } }
        ]));
        let measures = att_json(serde_json::json!([
            { "attestation_type": "scores", "asserted_at": "2026-05-04T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "progress_measure:m-1", "score": 0.7,
                  "attestation_id": "pm-1", "attested_key_id": "k-agent" } },
            { "attestation_type": "scores", "asserted_at": "2026-05-04T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "progress_measure:m-2", "score": 0.4,
                  "attestation_id": "pm-2", "attested_key_id": "k-agent" } }
        ]));

        let out = compose_decision_hierarchy_at(
            "species".into(),
            &goals,
            &approaches,
            &methods,
            &measures,
            fixed_now(),
        )
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();

        // Only the species-rooted branch survives (approach:species → method:a-1:A3 → pm-1)
        assert_eq!(parsed["approaches"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["approaches"][0]["attestation_id"], "a-1");
        assert_eq!(parsed["methods"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["methods"][0]["attestation_id"], "m-1");
        assert_eq!(parsed["progress_measures"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["progress_measures"][0]["attestation_id"], "pm-1");
    }

    // --- wa_state ---------------------------------------------------------

    #[test]
    fn wa_state_buckets_by_dimension_and_scopes_to_cell() {
        let cell_json = "{\"cell\":{\"domain\":\"mental_health\",\"language\":\"en\"}}";
        let other_cell = "{\"cell\":{\"domain\":\"legal\",\"language\":\"en\"}}";
        let moderation = att_json(serde_json::json!([
            { "attestation_type": "scores", "asserted_at": "2026-05-20T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "moderation:rogue_vote", "score": -0.9,
                  "attestation_id": "mo-1", "attested_key_id": "k-x",
                  "attesting_key_id": "k-wa", "context": cell_json } },
            { "attestation_type": "scores", "asserted_at": "2026-05-20T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "moderation:rogue_vote", "score": -0.9,
                  "attestation_id": "mo-2", "attested_key_id": "k-y",
                  "attesting_key_id": "k-wa", "context": other_cell } }
        ]));
        let slashing = att_json(serde_json::json!([
            { "attestation_type": "scores", "asserted_at": "2026-05-21T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "slashing:PROVEN_ROGUE", "score": 1.0,
                  "attestation_id": "sl-1", "attested_key_id": "k-x",
                  "attesting_key_id": "k-wa", "context": cell_json } }
        ]));
        let reconsideration = att_json(serde_json::json!([
            { "attestation_type": "scores", "asserted_at": "2026-05-22T00:00:00Z",
              "attestation_envelope": {
                  "dimension": "reconsideration:new_evidence", "score": 0.8,
                  "attestation_id": "rc-1", "attested_key_id": "k-x",
                  "attesting_key_id": "k-wa", "context": cell_json } }
        ]));

        let out = compose_wa_state_at(
            "mental_health".into(),
            "en".into(),
            &moderation,
            &slashing,
            &reconsideration,
            fixed_now(),
        )
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();

        // mo-2 in other_cell filtered out
        assert_eq!(parsed["moderation_queue"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["moderation_queue"][0]["category"], "rogue_vote");
        assert_eq!(parsed["slashing_history"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["slashing_history"][0]["category"], "PROVEN_ROGUE");
        assert_eq!(parsed["reconsideration_appeals"].as_array().unwrap().len(), 1);
        assert_eq!(
            parsed["reconsideration_appeals"][0]["category"],
            "new_evidence"
        );
    }
}
