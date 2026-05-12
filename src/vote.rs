//! Vote — signed score on a Contribution per `MISSION.md` Primitive 4 /
//! SCHEMA.md §5.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::cell::Cell;
use crate::identity::ContributorId;
use crate::signature::HybridSignature;

/// Score — subject-dependent shape per SCHEMA.md §5.1. Left as
/// `serde_json::Value` because:
///
/// - `arc_question`: numeric score + optional rubric-trigger hits.
/// - `proposed_battery`: approve/reject + per-question rationale.
/// - `prompt_edit` / `guide_edit` / `accord_edit`: approve/reject + diff
///   review notes.
/// - `wa_candidacy`: approve/reject + standing assessment.
/// - `moderation_event`: confirm/dispute the accusation.
///
/// Discriminated by the originating Contribution's `subject.subject_kind`
/// (or `contribution_type` for non-proposal kinds). Future work: typed
/// `Score::*` variants per discriminator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score(pub serde_json::Value);

/// Signed score on a Contribution. Per SCHEMA.md §5.
///
/// Vote weight per §5.2 is `Credits(domain, language, subject) ×
/// expertise_multiplier × active_tier_multiplier` — computed at
/// aggregation time, not embedded in the Vote payload. Persist's
/// `engine.read_vote_weight(...)` (Appendix A.3) returns this.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// ULID identifier per §2.2.
    pub vote_id: String,
    /// Voter's federation identity.
    pub voter_id: ContributorId,
    /// The Contribution being voted on.
    pub contribution_id: String,
    /// Credits-granularity cell — `subject` field MUST be populated.
    pub cell: Cell,
    /// Subject-dependent score shape.
    pub score: Score,
    /// Free-text rationale; e.g. "Hard-fail U2 — agent used ሳይኮተራፒ in
    /// Stage 2."
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    /// Voter's hybrid signature over the canonical Vote bytes.
    pub signature: HybridSignature,
    /// When the vote was cast.
    pub cast_at: DateTime<Utc>,
}
