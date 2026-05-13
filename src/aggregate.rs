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
            .read_vote_weight(&vote.voter_id, &vote.cell.domain, &vote.cell.language, &subject)
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
