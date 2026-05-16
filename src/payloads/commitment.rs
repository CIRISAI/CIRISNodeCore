//! `commitment` payload — SCHEMA.md §4.26.
//!
//! Commissive primitive — sender commits to a future action. Per
//! `FSD/MESSAGE_TAXONOMY.md` §7 (FIPA `agree` / `accept-proposal`
//! gap). Bilateral when `recipient_key` is set; broadcast otherwise.
//!
//! Resolution (did the commitment hold?) is deferred to a follow-up
//! FSD — this payload is the declaration, not the lifecycle.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `subject_kind` discriminator. Wire constant; matches SCHEMA §3.2.
pub const SUBJECT_KIND: &str = "commitment";

/// `commitment` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentPayload {
    /// The commitment.
    pub commitment_text: String,
    /// If set, bilateral — the named peer is the addressee. If `None`,
    /// broadcast — all peers are witnesses to the commitment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient_key: Option<String>,
    /// Free-form category. Canonical: `release` / `migration` /
    /// `audit` / `resolution`. Operators MAY add categories.
    pub action_kind: String,
    /// When the commitment falls due.
    pub due_at: DateTime<Utc>,
    /// Prior contribution_ids the commitment is in response to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
}

impl CommitmentPayload {
    /// True if the commitment names a specific recipient (bilateral
    /// counterparty cardinality per `FSD/MESSAGE_TAXONOMY.md` §3.2).
    pub fn is_bilateral(&self) -> bool {
        self.recipient_key.is_some()
    }

    /// True if the commitment's due-date has passed at `now`. A
    /// commitment past its `due_at` without a resolution Contribution
    /// is structurally overdue — consumers may surface this as a
    /// signal.
    pub fn is_overdue(&self, now: DateTime<Utc>) -> bool {
        self.due_at <= now
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn subject_kind_constant_matches_schema() {
        assert_eq!(SUBJECT_KIND, "commitment");
    }

    #[test]
    fn round_trip_bilateral() {
        let p = CommitmentPayload {
            commitment_text: "Ship v0.1.0-cut by 2026-06-01.".into(),
            recipient_key: Some("peer_pub_b64".into()),
            action_kind: "release".into(),
            due_at: Utc::now() + Duration::days(14),
            references: vec!["01HX".into()],
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: CommitmentPayload = serde_json::from_str(&json).unwrap();
        assert!(back.is_bilateral());
        assert!(!back.is_overdue(Utc::now()));
    }

    #[test]
    fn broadcast_commitment_has_no_recipient() {
        let p = CommitmentPayload {
            commitment_text: "I will publish v0.1.0-cut by 2026-06-01.".into(),
            recipient_key: None,
            action_kind: "release".into(),
            due_at: Utc::now() + Duration::days(14),
            references: vec![],
        };
        assert!(!p.is_bilateral());
        let json = serde_json::to_string(&p).unwrap();
        assert!(!json.contains("recipient_key"));
    }

    #[test]
    fn past_due_commitment_is_overdue() {
        let p = CommitmentPayload {
            commitment_text: String::new(),
            recipient_key: None,
            action_kind: "audit".into(),
            due_at: Utc::now() - Duration::days(1),
            references: vec![],
        };
        assert!(p.is_overdue(Utc::now()));
    }
}
