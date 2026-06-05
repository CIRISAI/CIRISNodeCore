//! Locality-aware WA quorum sizing per FSD-002 v1.4 §6.1.5
//! (CIRISNodeCore#10 — closes G3 from §13.11).
//!
//! When a decision carries an explicit `locality:decision:{scale}`
//! attestation (FSD-002 §3.6.5), the WA quorum size scales with the
//! decision's consequential reach rather than using the federation-wide
//! `N=3` default. This module exposes the policy function + extraction +
//! validation primitives so admission gates and quorum-selection code can
//! be locality-aware.
//!
//! **Pure logic, no I/O.** This module takes attestation JSON as input
//! and returns typed results. The PyO3 surface in [`crate::python`] can
//! thin-wrap these for the UI; the cohabitation Rust path calls directly.
//!
//! **Composition with P10 witness diversity**: orthogonal. Locality-scaled
//! quorum sizes the quorum *count*; P10 still applies its diversity bar
//! (jurisdictional / organizational / software-stack) on the quorum
//! *shape*.
//!
//! **Default fallback**: when no `locality:decision:*` attestation
//! exists, callers should fall through to the current `N=3` default
//! (backward-compatible with v1.3 callers that don't yet emit locality).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::compose::AttestationRow;

/// The four scales `locality:decision:{scale}` enumerates per
/// FSD-002 v1.4 §3.6.5. Ordered by consequential reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LocalityScale {
    /// Decision affects a single locality (city / town / community).
    /// Default quorum 2; min cell pool 4.
    Local,
    /// Decision affects a region (state / province / sub-national area).
    /// Default quorum 3; min cell pool 6.
    Regional,
    /// Decision affects a nation (single sovereign jurisdiction).
    /// Default quorum 4; min cell pool 8.
    National,
    /// Decision affects the federation as a whole. Default quorum 6;
    /// min cell pool 12. This is the broadest consequential reach a
    /// cell can claim.
    Federation,
}

impl LocalityScale {
    /// Parse from the dimension suffix (the `{scale}` part of
    /// `locality:decision:{scale}`).
    pub fn from_suffix(s: &str) -> Option<Self> {
        match s {
            "local" => Some(Self::Local),
            "regional" => Some(Self::Regional),
            "national" => Some(Self::National),
            "federation" => Some(Self::Federation),
            _ => None,
        }
    }

    /// String form for serialization / logging.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Regional => "regional",
            Self::National => "national",
            Self::Federation => "federation",
        }
    }
}

/// FSD-002 v1.4 §6.1.5 default reference function — `quorum_size(scale)`.
/// Policy-tunable per deployment; this is the federation default.
///
/// ```text
/// local      → 2
/// regional   → 3
/// national   → 4
/// federation → 6
/// ```
pub fn default_quorum_size(scale: LocalityScale) -> usize {
    match scale {
        LocalityScale::Local => 2,
        LocalityScale::Regional => 3,
        LocalityScale::National => 4,
        LocalityScale::Federation => 6,
    }
}

/// FSD-002 v1.4 §6.1.5: `min_pool(scale) = quorum_size(scale) × 2`.
///
/// Minimum cell-pool size for fresh-quorum recusal under P11 Reconsideration
/// to be structurally feasible at the named locality scale.
pub fn default_min_pool(scale: LocalityScale) -> usize {
    default_quorum_size(scale).saturating_mul(2)
}

/// Locality-mismatch error per FSD-002 §6.1.5 — a decision claiming a
/// locality scale its cell cannot substantively review under recusal.
/// "Named" failure mode: not silently downgraded to ad-hoc fallback.
#[derive(Debug, Clone, Serialize)]
pub struct LocalityMismatch {
    /// The scale the decision attempted to claim.
    pub claimed_scale: LocalityScale,
    /// The cell's current WA pool size at admission time.
    pub cell_pool_size: usize,
    /// The minimum cell pool required to structurally support
    /// `claimed_scale` under fresh-quorum recusal.
    pub required_min_pool: usize,
}

impl std::fmt::Display for LocalityMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "locality mismatch: locality:decision:{} claimed but cell_pool={} < min_pool({})={}",
            self.claimed_scale.as_str(),
            self.cell_pool_size,
            self.claimed_scale.as_str(),
            self.required_min_pool
        )
    }
}

impl std::error::Error for LocalityMismatch {}

/// Validate that a cell pool can structurally support a decision at the
/// claimed locality scale under fresh-quorum recusal (per §6.1.5).
///
/// Caller passes the current cell pool size (count of WAs holding
/// non-zero Expertise in the cell); this function checks against
/// `default_min_pool(scale)` and returns `LocalityMismatch` if the cell
/// is overclaiming reach.
pub fn validate_cell_pool(
    cell_pool_size: usize,
    scale: LocalityScale,
) -> Result<(), LocalityMismatch> {
    let required = default_min_pool(scale);
    if cell_pool_size >= required {
        Ok(())
    } else {
        Err(LocalityMismatch {
            claimed_scale: scale,
            cell_pool_size,
            required_min_pool: required,
        })
    }
}

/// Extract the active `locality:decision:{scale}` claim from a list of
/// attestations targeting (or referencing) a specific decision.
///
/// **Input**: JSON-serialized `Vec<Attestation>` — typically the result
/// of `engine.list_attestations(filter)` filtered to attestations whose
/// `attested_key_id` or `context.decision_id` matches the decision under
/// adjudication.
///
/// **Returns**: `Some(LocalityScale)` if at least one active `scores`
/// attestation on `locality:decision:{scale}` is present (latest by
/// `asserted_at` wins on tie); `None` if no locality claim exists, in
/// which case the caller should fall through to `N=3` default per the
/// FSD-002 §6.1.5 backward-compatibility commitment.
///
/// Multiple-scale conflicts (e.g., `locality:decision:national` AND
/// `locality:decision:federation` on the same decision) are resolved by
/// **most-recent wins**. Callers concerned about the cross-cutting case
/// (decision affects multiple scales) should decompose into multiple
/// `locality:decision:{scale}` Contributions, each adjudicated at its own
/// scale per §6.1.5 residual-cases discipline.
pub fn extract_locality_from_attestations(
    attestations_json: &str,
) -> Result<Option<LocalityScale>, serde_json::Error> {
    extract_locality_from_attestations_at(attestations_json, Utc::now())
}

pub(crate) fn extract_locality_from_attestations_at(
    attestations_json: &str,
    now: DateTime<Utc>,
) -> Result<Option<LocalityScale>, serde_json::Error> {
    let rows: Vec<AttestationRow> = serde_json::from_str(attestations_json)?;
    let mut latest: Option<(DateTime<Utc>, LocalityScale)> = None;

    for row in rows {
        if row.attestation_type != "scores" || !row.is_active_at(now) {
            continue;
        }
        let Some(dim) = row.dimension() else { continue };
        let Some(scale_suffix) = dim.strip_prefix("locality:decision:") else {
            continue;
        };
        let Some(scale) = LocalityScale::from_suffix(scale_suffix) else {
            continue;
        };
        // Polarity rule: positive = decision IS at this scale; negative =
        // overreach claim. Only positive contributes to the locality claim.
        let Some(score) = row.score() else { continue };
        if score <= 0.0 {
            continue;
        }
        match latest {
            Some((prior_ts, _)) if prior_ts >= row.asserted_at => {}
            _ => latest = Some((row.asserted_at, scale)),
        }
    }

    Ok(latest.map(|(_, s)| s))
}

/// Per-locality-scale health snapshot for one cell — the §6.1.5 federation
/// health observable. Surfaced to RATCHET + CIRISLens for downstream
/// downweighting of cells that overclaim their consequential reach.
#[derive(Debug, Clone, Serialize)]
pub struct CellLocalityHealth {
    /// Number of WAs in the cell holding non-zero Expertise.
    pub cell_pool_size: usize,
    /// The highest scale this cell can structurally support under
    /// fresh-quorum recusal — `None` if the pool is too small even for
    /// local-scale (`pool < 4`).
    pub max_supportable_scale: Option<LocalityScale>,
    /// Count of decisions currently being adjudicated in the cell at
    /// each locality scale (ordered Local / Regional / National /
    /// Federation). Lets observers detect cells overclaiming their
    /// consequential reach (e.g., many federation-scale decisions
    /// in a narrow specialty cell).
    pub current_decisions_at_each_scale: [(LocalityScale, usize); 4],
}

/// Compute the highest locality scale a cell pool can structurally
/// support under fresh-quorum recusal.
///
/// Walks scales from `Federation` down; returns the first one whose
/// `min_pool` ≤ `cell_pool_size`. Returns `None` if the pool is below
/// `min_pool(Local)` (i.e., < 4) — such cells can't host any
/// locality-claimed adjudication, only N=3-default decisions, and
/// even then only if `cell_pool ≥ 6` for P11 recusal feasibility.
pub fn max_supportable_scale(cell_pool_size: usize) -> Option<LocalityScale> {
    [
        LocalityScale::Federation,
        LocalityScale::National,
        LocalityScale::Regional,
        LocalityScale::Local,
    ]
    .into_iter()
    .find(|&scale| cell_pool_size >= default_min_pool(scale))
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

    // --- default policy functions (per FSD-002 v1.4 §6.1.5) --------------

    #[test]
    fn default_quorum_sizes_match_fsd_002_v1_4_table() {
        assert_eq!(default_quorum_size(LocalityScale::Local), 2);
        assert_eq!(default_quorum_size(LocalityScale::Regional), 3);
        assert_eq!(default_quorum_size(LocalityScale::National), 4);
        assert_eq!(default_quorum_size(LocalityScale::Federation), 6);
    }

    #[test]
    fn min_pool_is_quorum_size_times_two() {
        for scale in [
            LocalityScale::Local,
            LocalityScale::Regional,
            LocalityScale::National,
            LocalityScale::Federation,
        ] {
            assert_eq!(default_min_pool(scale), default_quorum_size(scale) * 2);
        }
    }

    // --- validate_cell_pool ----------------------------------------------

    #[test]
    fn validate_passes_when_pool_meets_min() {
        // Federation needs pool 12; pool=12 passes
        validate_cell_pool(12, LocalityScale::Federation).unwrap();
        // Local needs pool 4; pool=4 passes
        validate_cell_pool(4, LocalityScale::Local).unwrap();
    }

    #[test]
    fn validate_fails_with_named_mismatch_when_pool_too_small() {
        let err = validate_cell_pool(5, LocalityScale::Federation).unwrap_err();
        assert_eq!(err.claimed_scale, LocalityScale::Federation);
        assert_eq!(err.cell_pool_size, 5);
        assert_eq!(err.required_min_pool, 12);
        // Display formatting names the mismatch per §6.1.5
        let s = format!("{err}");
        assert!(s.contains("locality mismatch"));
        assert!(s.contains("federation"));
        assert!(s.contains("cell_pool=5"));
    }

    // --- max_supportable_scale -------------------------------------------

    #[test]
    fn max_supportable_walks_down_from_federation() {
        assert_eq!(max_supportable_scale(20), Some(LocalityScale::Federation));
        assert_eq!(max_supportable_scale(12), Some(LocalityScale::Federation));
        assert_eq!(max_supportable_scale(11), Some(LocalityScale::National));
        assert_eq!(max_supportable_scale(8), Some(LocalityScale::National));
        assert_eq!(max_supportable_scale(7), Some(LocalityScale::Regional));
        assert_eq!(max_supportable_scale(6), Some(LocalityScale::Regional));
        assert_eq!(max_supportable_scale(5), Some(LocalityScale::Local));
        assert_eq!(max_supportable_scale(4), Some(LocalityScale::Local));
        assert_eq!(max_supportable_scale(3), None);
        assert_eq!(max_supportable_scale(0), None);
    }

    // --- extract_locality_from_attestations -------------------------------

    #[test]
    fn extract_picks_latest_active_locality() {
        let rows = att_json(serde_json::json!([
            { "attestation_type": "scores", "asserted_at": "2026-05-01T00:00:00Z",
              "attestation_envelope": {"dimension": "locality:decision:local", "score": 1.0} },
            { "attestation_type": "scores", "asserted_at": "2026-05-15T00:00:00Z",
              "attestation_envelope": {"dimension": "locality:decision:national", "score": 1.0} }
        ]));
        let scale = extract_locality_from_attestations_at(&rows, fixed_now()).unwrap();
        assert_eq!(scale, Some(LocalityScale::National));
    }

    #[test]
    fn extract_ignores_expired_negative_and_non_scores() {
        let rows = att_json(serde_json::json!([
            // Expired
            { "attestation_type": "scores", "asserted_at": "2020-01-01T00:00:00Z",
              "expires_at": "2020-12-31T00:00:00Z",
              "attestation_envelope": {"dimension": "locality:decision:federation", "score": 1.0} },
            // Negative score (overreach claim, not a locality declaration)
            { "attestation_type": "scores", "asserted_at": "2026-05-01T00:00:00Z",
              "attestation_envelope": {"dimension": "locality:decision:federation", "score": -1.0} },
            // Non-scores type
            { "attestation_type": "withdraws", "asserted_at": "2026-05-01T00:00:00Z",
              "attestation_envelope": {"dimension": "locality:decision:federation", "score": 1.0} },
            // Active positive — should be picked
            { "attestation_type": "scores", "asserted_at": "2026-05-10T00:00:00Z",
              "attestation_envelope": {"dimension": "locality:decision:regional", "score": 1.0} }
        ]));
        let scale = extract_locality_from_attestations_at(&rows, fixed_now()).unwrap();
        assert_eq!(scale, Some(LocalityScale::Regional));
    }

    #[test]
    fn extract_returns_none_when_no_locality_claim() {
        let rows = att_json(serde_json::json!([
            { "attestation_type": "scores", "asserted_at": "2026-05-01T00:00:00Z",
              "attestation_envelope": {"dimension": "credits:foo:en:s", "score": 1.0} }
        ]));
        let scale = extract_locality_from_attestations_at(&rows, fixed_now()).unwrap();
        assert_eq!(scale, None);
    }

    #[test]
    fn extract_rejects_unknown_scale_suffixes() {
        let rows = att_json(serde_json::json!([
            { "attestation_type": "scores", "asserted_at": "2026-05-01T00:00:00Z",
              "attestation_envelope": {"dimension": "locality:decision:planetary", "score": 1.0} }
        ]));
        let scale = extract_locality_from_attestations_at(&rows, fixed_now()).unwrap();
        assert_eq!(scale, None);
    }

    // --- Integration: end-to-end ------------------------------------------

    #[test]
    fn end_to_end_a_narrow_cell_overclaim_is_named() {
        // Decision claims federation-scale locality
        let rows = att_json(serde_json::json!([
            { "attestation_type": "scores", "asserted_at": "2026-05-15T00:00:00Z",
              "attestation_envelope": {"dimension": "locality:decision:federation", "score": 1.0} }
        ]));
        let scale = extract_locality_from_attestations_at(&rows, fixed_now()).unwrap();
        assert_eq!(scale, Some(LocalityScale::Federation));

        // Cell pool of 5 is below min_pool(federation)=12 → named mismatch
        let mismatch = validate_cell_pool(5, scale.unwrap()).unwrap_err();
        assert_eq!(mismatch.required_min_pool, 12);
        // Cell COULD support Local-scale decisions (min_pool=4, pool=5)
        assert_eq!(max_supportable_scale(5), Some(LocalityScale::Local));
    }
}
