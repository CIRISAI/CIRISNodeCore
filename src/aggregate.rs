//! Weighted-aggregate computation per `MISSION.md` Primitive 7 / §5.3.
//!
//! Pure compute over a Vote stream:
//!
//! - For each Vote on a Contribution, read the voter's cast-time
//!   `VoteWeight` via [`NodeCoreService::read_vote_weight`].
//! - Sum the effective weights (`credits × expertise_multiplier ×
//!   active_tier_multiplier`) bucketed by verdict (approve / reject /
//!   abstain).
//! - Apply a fail-secure quorum gate: if vote count is below the
//!   policy-tunable minimum, return [`Aggregate::BelowQuorum`] rather
//!   than a misleading numeric outcome.
//!
//! Threshold-crossing for canonical promotion is the caller's policy
//! (not enforced here) — when the caller decides the [`Aggregate`]
//! warrants promotion, they sign a `PromotionAttestation` via
//! [`crate::sign::build_promotion_attestation`] and call
//! `engine.put_promotion_attestation`.
//!
//! Currently scoped to **`proposal_adoption`-shaped** score payloads
//! per `SCHEMA.md` §5.1 — `{ "verdict": "approve"|"reject"|"abstain",
//! "magnitude": f64 }`. Other score shapes (`battery_response` with
//! hard_fail/soft_fail/pass) live in safety-battery aggregation, not
//! this function.

use crate::substrate::{NodeCoreService, SubstrateError, VotesFilter};

/// Result of aggregating votes on a Contribution.
#[derive(Debug, Clone, PartialEq)]
pub enum Aggregate {
    /// Quorum met; weighted aggregate available.
    Resolved {
        /// The contribution being aggregated.
        contribution_id: String,
        /// How many votes contributed to this aggregate.
        votes_counted: usize,
        /// Sum of effective weights for `verdict = "approve"`.
        approve_weight: f64,
        /// Sum of effective weights for `verdict = "reject"`.
        reject_weight: f64,
        /// Sum of effective weights for `verdict = "abstain"`.
        abstain_weight: f64,
    },
    /// Vote count below the cell's minimum quorum. Fail-secure — caller
    /// must NOT treat as a zero-weight outcome.
    BelowQuorum {
        /// The contribution being aggregated.
        contribution_id: String,
        /// How many votes have been cast so far.
        votes_counted: usize,
        /// The minimum required for the aggregate to resolve.
        minimum_required: usize,
    },
}

impl Aggregate {
    /// Total effective weight across all verdicts (only meaningful for
    /// [`Aggregate::Resolved`]; returns `None` for `BelowQuorum`).
    pub fn total_weight(&self) -> Option<f64> {
        match self {
            Aggregate::Resolved {
                approve_weight,
                reject_weight,
                abstain_weight,
                ..
            } => Some(approve_weight + reject_weight + abstain_weight),
            Aggregate::BelowQuorum { .. } => None,
        }
    }

    /// Approval ratio (`approve / (approve + reject)`) — abstain
    /// excluded from the denominator. `None` for `BelowQuorum` or
    /// when no approve+reject votes (avoids 0/0).
    pub fn approval_ratio(&self) -> Option<f64> {
        match self {
            Aggregate::Resolved {
                approve_weight,
                reject_weight,
                ..
            } => {
                let denom = approve_weight + reject_weight;
                if denom > 0.0 {
                    Some(approve_weight / denom)
                } else {
                    None
                }
            }
            Aggregate::BelowQuorum { .. } => None,
        }
    }
}

/// Decode a `proposal_adoption`-shaped score into a verdict variant.
/// Returns `None` for malformed or non-proposal_adoption shapes.
fn decode_verdict(score: &serde_json::Value) -> Option<Verdict> {
    let s = score.get("verdict")?.as_str()?;
    match s {
        "approve" => Some(Verdict::Approve),
        "reject" => Some(Verdict::Reject),
        "abstain" => Some(Verdict::Abstain),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
enum Verdict {
    Approve,
    Reject,
    Abstain,
}

/// Compute the §5.3 weighted aggregate for a Contribution.
///
/// Reads all Votes from persist filtered by `contribution_id`, then
/// per-vote calls `read_vote_weight` to get the cast-time weight, sums
/// by verdict.
///
/// Votes with malformed score shapes (non-`proposal_adoption`) are
/// silently skipped — they don't contribute to the aggregate AND
/// don't count toward quorum. Caller-supplied `is_canonical` filter
/// is `None` (both pending + canonical) — typically callers want all
/// votes when computing an aggregate for promotion-decision purposes.
pub async fn weighted_aggregate<E: NodeCoreService>(
    engine: &E,
    contribution_id: &str,
    minimum_quorum: usize,
) -> Result<Aggregate, SubstrateError> {
    let filter = VotesFilter {
        contribution_id: Some(contribution_id.to_owned()),
        voter_id: None,
        domain: None,
        language: None,
        is_canonical: None,
    };
    // Pull all votes — list_votes paginates, but for an aggregate we
    // walk all pages. v0.1.0-dev: single-page assumption (limit=10_000);
    // a v0.1.0 hardening pass adds cursor iteration when production
    // contributions accumulate enough votes to spill.
    let page = engine.list_votes(filter, None, 10_000).await?;

    let mut votes_counted: usize = 0;
    let mut approve = 0.0_f64;
    let mut reject = 0.0_f64;
    let mut abstain = 0.0_f64;

    for vote in page.items {
        let verdict = match decode_verdict(&vote.score) {
            Some(v) => v,
            None => continue, // skip non-proposal_adoption scores
        };
        let subject = vote.cell.subject.clone().unwrap_or_default();
        let weight = engine
            .read_vote_weight(
                &vote.voter_id,
                &vote.cell.domain,
                &vote.cell.language,
                &subject,
            )
            .await?
            .map(|w| w.weight)
            .unwrap_or(0.0);
        votes_counted += 1;
        match verdict {
            Verdict::Approve => approve += weight,
            Verdict::Reject => reject += weight,
            Verdict::Abstain => abstain += weight,
        }
    }

    if votes_counted < minimum_quorum {
        Ok(Aggregate::BelowQuorum {
            contribution_id: contribution_id.to_owned(),
            votes_counted,
            minimum_required: minimum_quorum,
        })
    } else {
        Ok(Aggregate::Resolved {
            contribution_id: contribution_id.to_owned(),
            votes_counted,
            approve_weight: approve,
            reject_weight: reject,
            abstain_weight: abstain,
        })
    }
}

// ─── Occurrence-cohort aggregation (NodeCore#16) ──────────────────────
//
// Multi-occurrence agents (e.g. 9 occurrences sharing one identity per
// CIRISAgent CLAUDE.md) emit parallel scalars on the same dimension
// (D01 non-maleficence, D10 beneficence, ...). The single-contribution
// `weighted_aggregate` above aggregates the votes ON ONE Contribution;
// `cohort_weighted_aggregate` here aggregates the resolved aggregates
// ACROSS multiple Contributions sharing one `agent_template_id`.
//
// Wire shape (dimension namespace extension per CEG):
//   `weighted_aggregate:{contribution_id}:cohort:{agent_template_id}`
//
// Composition: a CohortAggregate attestation summarises a set of
// per-occurrence Contributions; each Contribution's own
// `weighted_aggregate:{contribution_id}` is unchanged. The cohort
// attestation is additive — 1+4 wire-format lockdown holds.
//
// Threat surface (called out per NodeCore#16):
//
//   - **Cohort spoofing**: an attacker constructs `included_occurrences`
//     listing keys they don't control. Mitigation: each
//     per-occurrence Contribution must be signed by ITS OWN key; the
//     cohort attestation cannot manufacture a Contribution it doesn't
//     have. The cohort can only AGGREGATE existing signed Contributions
//     from the listed occurrences. P10 witness-set diversity applies
//     to the cohort attestation itself.
//
//   - **Selective inclusion**: cherry-picking a favorable subset.
//     Mitigation: the `expected_occurrence_count` field declares the
//     fleet's known size; consumers compute `coverage = included /
//     expected` and reject cohort attestations below a policy
//     threshold. Witness aggregation (P10) at the cohort level
//     confirms the inclusion set is complete-or-justified.

/// An occurrence-cohort spec describing which agent occurrences should
/// be aggregated together. Per NodeCore#16, this is the surface that
/// extends P7 with fleet-level aggregation. Composes with the
/// CIRISPersist `occurrence_id` field (CIRISPersist#110) once that
/// lands; in the meantime, the caller supplies the spec directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OccurrenceCohort {
    /// The shared agent template identifier (e.g. CIRIS Agent template).
    /// All `included_occurrences` MUST be occurrences of this template;
    /// the cohort attestation does not span templates.
    pub agent_template_id: String,
    /// Federation key_ids of occurrences whose Contributions are being
    /// aggregated. Each must have actually emitted a Contribution that
    /// shows up in `list_contributions` with `author_id == this key`.
    pub included_occurrences: Vec<String>,
    /// The fleet's declared total occurrence count — used by consumers
    /// to compute coverage (included / expected) and reject
    /// selective-inclusion attestations below their policy threshold.
    /// Set to `None` when the fleet size is unknown (single-occurrence
    /// cohorts that exist for forward-compatibility).
    pub expected_occurrence_count: Option<usize>,
}

/// The fleet-level aggregate across an `OccurrenceCohort`. Per
/// NodeCore#16 — D01 non-maleficence / D10 beneficence cross-occurrence
/// readings.
#[derive(Debug, Clone, PartialEq)]
pub struct CohortAggregate {
    /// The shared agent template identifier.
    pub agent_template_id: String,
    /// Occurrences that contributed a resolved per-Contribution
    /// aggregate (parallel to `per_occurrence_approval_ratio` and
    /// `per_occurrence_total_weight` below).
    pub included_occurrences: Vec<String>,
    /// Coverage ratio: `included.len() / expected_occurrence_count`,
    /// or `None` when `expected_occurrence_count` was `None`.
    /// Consumer policy: reject cohort aggregates below a threshold.
    pub coverage: Option<f64>,
    /// Per-occurrence approval ratios. `None` entries indicate the
    /// occurrence's Contribution was `BelowQuorum` or its
    /// approve+reject weights were zero (abstain-only).
    pub per_occurrence_approval_ratio: Vec<Option<f64>>,
    /// Per-occurrence total weights — useful for variance / robustness
    /// analysis (RATCHET-style).
    pub per_occurrence_total_weight: Vec<f64>,
    /// Fleet mean of resolved approval ratios (skipping `None` entries).
    pub mean_approval_ratio: Option<f64>,
    /// Fleet standard deviation of resolved approval ratios.
    /// Population stddev (N denominator, not N-1) since the cohort is
    /// the population.
    pub stddev_approval_ratio: Option<f64>,
    /// Fleet minimum approval ratio.
    pub min_approval_ratio: Option<f64>,
    /// Fleet maximum approval ratio.
    pub max_approval_ratio: Option<f64>,
    /// Fleet total weight summed across all occurrences.
    pub total_fleet_weight: f64,
}

impl CohortAggregate {
    /// True iff coverage is `Some(r)` where `r >= threshold`. Use this
    /// at consumer policy time to reject selective-inclusion
    /// attestations. When coverage is `None` (single-occurrence
    /// forward-compat), returns `false` to fail-safe.
    pub fn meets_coverage_threshold(&self, threshold: f64) -> bool {
        matches!(self.coverage, Some(c) if c >= threshold)
    }
}

/// Compute the cohort weighted aggregate per NodeCore#16.
///
/// For each occurrence in `cohort.included_occurrences`:
///
///   1. Find every Contribution emitted by that occurrence (via
///      `list_contributions` with `author_id` filter) that shares the
///      dimension being aggregated (in this implementation, the caller
///      supplies the contribution-id stream via the per-occurrence
///      Contribution map below — see the `cohort_contributions`
///      argument).
///   2. Compute the per-Contribution `weighted_aggregate`.
///   3. Roll up: per-occurrence approval ratio + per-occurrence total
///      weight.
///   4. Fleet statistics: mean / stddev / min / max + total weight.
///   5. Coverage check: `included / expected_occurrence_count`.
///
/// The caller passes a `cohort_contributions: Vec<(occurrence_key, contribution_id)>`
/// mapping rather than the substrate guessing which Contribution per
/// occurrence — this keeps the aggregation function pure and avoids
/// the cohort attestation inheriting the substrate's ambient ranking.
/// In a deployed setting the caller derives this from the federation
/// directory's occurrence_id index (CIRISPersist#110, deferred).
pub async fn cohort_weighted_aggregate<E: NodeCoreService>(
    engine: &E,
    cohort: &OccurrenceCohort,
    cohort_contributions: &[(String, String)],
    minimum_quorum_per_occurrence: usize,
) -> Result<CohortAggregate, SubstrateError> {
    let mut per_occ_ratio: Vec<Option<f64>> = Vec::with_capacity(cohort.included_occurrences.len());
    let mut per_occ_weight: Vec<f64> = Vec::with_capacity(cohort.included_occurrences.len());
    let mut included: Vec<String> = Vec::with_capacity(cohort.included_occurrences.len());

    for occ in &cohort.included_occurrences {
        let contrib_id = cohort_contributions
            .iter()
            .find(|(o, _)| o == occ)
            .map(|(_, c)| c.clone());
        let Some(cid) = contrib_id else {
            // Occurrence declared but no Contribution mapping — skip;
            // it doesn't contribute to the aggregate (mirrors how
            // `weighted_aggregate` skips malformed-score votes).
            continue;
        };
        let agg = weighted_aggregate(engine, &cid, minimum_quorum_per_occurrence).await?;
        let (ratio, weight) = match agg {
            Aggregate::Resolved {
                approve_weight,
                reject_weight,
                abstain_weight,
                ..
            } => {
                let denom = approve_weight + reject_weight;
                let ratio = if denom > 0.0 {
                    Some(approve_weight / denom)
                } else {
                    None
                };
                let weight = approve_weight + reject_weight + abstain_weight;
                (ratio, weight)
            }
            Aggregate::BelowQuorum { .. } => (None, 0.0),
        };
        included.push(occ.clone());
        per_occ_ratio.push(ratio);
        per_occ_weight.push(weight);
    }

    let resolved: Vec<f64> = per_occ_ratio.iter().filter_map(|x| *x).collect();
    let (mean, stddev, min, max) = if resolved.is_empty() {
        (None, None, None, None)
    } else {
        let n = resolved.len() as f64;
        let mean = resolved.iter().sum::<f64>() / n;
        let variance = resolved.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
        let stddev = variance.sqrt();
        let min = resolved.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = resolved.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        (Some(mean), Some(stddev), Some(min), Some(max))
    };

    let coverage = cohort.expected_occurrence_count.map(|expected| {
        if expected == 0 {
            0.0
        } else {
            included.len() as f64 / expected as f64
        }
    });

    let total_fleet_weight: f64 = per_occ_weight.iter().sum();

    Ok(CohortAggregate {
        agent_template_id: cohort.agent_template_id.clone(),
        included_occurrences: included,
        coverage,
        per_occurrence_approval_ratio: per_occ_ratio,
        per_occurrence_total_weight: per_occ_weight,
        mean_approval_ratio: mean,
        stddev_approval_ratio: stddev,
        min_approval_ratio: min,
        max_approval_ratio: max,
        total_fleet_weight,
    })
}
