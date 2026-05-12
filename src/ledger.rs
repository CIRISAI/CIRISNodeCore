//! Ledgers — Commons Credits + Expertise standing.
//!
//! Per SCHEMA.md §10. Both ledgers are **derived state**, not
//! user-submitted Contributions. They are computed from the audit chain
//! and exposed by `ciris-persist` via read views per Appendix A.3:
//!
//! - `engine.get_credits_ledger(contributor_id)`
//! - `engine.get_expertise_ledger(contributor_id)`
//! - `engine.read_vote_weight(...)` — composite read for §5.2 weighting
//!
//! These types live in node-core so callers (safety.ciris.ai, the
//! eventual CIRISAgent fold-in) can deserialize ledger reads without
//! depending on persist's internal row shape.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::cell::Cell;
use crate::identity::ContributorId;
use crate::signature::HybridSignature;

/// One row in the Commons Credits ledger — per `(contributor, cell)`
/// where `cell` is Credits-granularity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditsEntry {
    /// Credits-granularity cell.
    pub cell: Cell,
    /// Non-negative invariant per `MISSION.md` §2.9 / SCHEMA.md §10.
    /// Slashing reduces toward but never below zero.
    pub credits: f64,
    /// Last update.
    pub updated_at: DateTime<Utc>,
}

/// Commons Credits ledger view per SCHEMA.md §10.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonsCreditsLedger {
    /// Whose ledger.
    pub contributor_id: ContributorId,
    /// Per-cell credit balances.
    pub entries: Vec<CreditsEntry>,
    /// Signed by the crate at the time of the read snapshot.
    pub ledger_signature: HybridSignature,
}

/// One row in the Expertise ledger — per `(contributor, cell)` where
/// `cell` is Expertise-granularity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertiseEntry {
    /// Expertise-granularity cell (no `subject`).
    pub cell: Cell,
    /// Non-negative invariant per `MISSION.md` §2.9 / SCHEMA.md §10.
    pub standing: f64,
    /// Whether contributor meets the Active-tier threshold (§3.8) at
    /// snapshot time. Affects vote-weight multiplier; recomputed from
    /// the audit chain on each read.
    pub active_tier: bool,
    /// Last update.
    pub updated_at: DateTime<Utc>,
}

/// Expertise ledger view per SCHEMA.md §10.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertiseLedger {
    /// Whose ledger.
    pub contributor_id: ContributorId,
    /// Per-cell standing.
    pub entries: Vec<ExpertiseEntry>,
    /// Signed by the crate at the time of the read snapshot.
    pub ledger_signature: HybridSignature,
}

/// Composite vote weight per SCHEMA.md §5.2: `Credits × expertise_multiplier
/// × active_tier_multiplier`. Returned by `engine.read_vote_weight(...)`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct VoteWeight {
    /// Credits for the vote's cell at snapshot time.
    pub credits: f64,
    /// Expertise multiplier derived from the voter's Expertise standing
    /// at `(domain, language)`. Function shape is a policy parameter
    /// per `MISSION.md` §3.4.
    pub expertise_multiplier: f64,
    /// Active-tier multiplier per §3.8. `1.0` if Active; `0.0` (or a
    /// policy-tuned baseline) otherwise.
    pub active_tier_multiplier: f64,
}

impl VoteWeight {
    /// Effective weight.
    pub fn effective(&self) -> f64 {
        self.credits * self.expertise_multiplier * self.active_tier_multiplier
    }
}
