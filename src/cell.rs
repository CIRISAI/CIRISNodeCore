//! Cell — the granularity at which federation-consensus state is indexed.
//!
//! Per SCHEMA.md §2.5: two cell granularities apply.
//!
//! - `(domain, language, subject)` — Credits-granularity. Per
//!   `MISSION.md` Primitive 2; the cell at which Commons Credits accrue.
//! - `(domain, language)` — Expertise-granularity. Per `MISSION.md`
//!   Primitive 3; the cell at which Expertise standing is held.
//!
//! The `subject` field is omitted in Expertise-granularity contexts.

use serde::{Deserialize, Serialize};

/// Cell. Wire-format identical to SCHEMA.md §2.5; `subject` is `None`
/// for Expertise-granularity uses (deferral routing, WA candidacy,
/// expertise attestation) and `Some` for Credits-granularity uses
/// (Contribution payload classification, Vote weighting).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cell {
    /// Domain — drawn from the categories in
    /// `ciris_engine/logic/buses/prohibitions.py` (`MEDICAL`, `LEGAL`,
    /// `SPIRITUAL_DIRECTION`, etc.) plus `mental_health` (capability-allowed
    /// but high-stakes, not prohibited).
    pub domain: String,
    /// Language — ISO 639-1 code drawn from
    /// `ciris_engine/data/localized/manifest.json` (29 locales).
    pub language: String,
    /// Subject — Credits-granularity discriminator. `None` for
    /// Expertise-granularity uses; `Some(_)` for Credits-granularity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
}

impl Cell {
    /// Expertise-granularity cell.
    pub fn expertise(domain: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            language: language.into(),
            subject: None,
        }
    }

    /// Credits-granularity cell.
    pub fn credits(
        domain: impl Into<String>,
        language: impl Into<String>,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            domain: domain.into(),
            language: language.into(),
            subject: Some(subject.into()),
        }
    }

    /// True if this cell carries a `subject` (Credits-granularity).
    pub fn is_credits_granularity(&self) -> bool {
        self.subject.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expertise_cell_omits_subject_on_wire() {
        let c = Cell::expertise("mental_health", "am");
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, r#"{"domain":"mental_health","language":"am"}"#);
    }

    #[test]
    fn credits_cell_carries_subject() {
        let c = Cell::credits("mental_health", "am", "arc_question");
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains(r#""subject":"arc_question""#));
    }
}
