//! `notification` payload — SCHEMA.md §4.20.
//!
//! Peer-to-peer fire-and-forget update about the environment or the
//! results of an action. Sender does not expect a response (responses
//! are optional per §4.21). Categories distinguish observation
//! classes for receiver-side filtering.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `subject_kind` discriminator. Wire constant; matches SCHEMA §3.2.
pub const SUBJECT_KIND: &str = "notification";

/// Notification category — discriminates observation class. Canonical
/// values per SCHEMA §4.20. Operators MAY add categories via the
/// `Custom` variant; receivers filter on this.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationCategory {
    /// Observed state of the world.
    Environment,
    /// Sender completed an action and is reporting.
    ActionResult,
    /// Sender's own state changed.
    StateChange,
    /// Anomaly detected. MAY require witness-set at consumer policy.
    Anomaly,
    /// Operator-defined custom category.
    #[serde(untagged)]
    Custom(String),
}

impl NotificationCategory {
    /// True for `Anomaly`-class notifications. These MAY require
    /// witness-set at consumer policy per SCHEMA §4.20.
    pub fn is_high_stakes(&self) -> bool {
        matches!(self, NotificationCategory::Anomaly)
    }
}

/// `notification` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    /// Short label.
    pub title: String,
    /// The observation.
    pub context: String,
    /// Observation class.
    pub category: NotificationCategory,
    /// Free-form reference to a thing being observed (contribution_id,
    /// key, agent identifier, node identifier).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_ref: Option<String>,
    /// Trace ids or other supporting refs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
    /// When the notification is no longer relevant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl NotificationPayload {
    /// True if the notification is still relevant at `now`. Open-ended
    /// notifications (no `expires_at`) are always relevant.
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        match self.expires_at {
            Some(t) => t > now,
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_kind_constant_matches_schema() {
        assert_eq!(SUBJECT_KIND, "notification");
    }

    #[test]
    fn category_snake_case() {
        assert_eq!(
            serde_json::to_string(&NotificationCategory::Environment).unwrap(),
            r#""environment""#
        );
        assert_eq!(
            serde_json::to_string(&NotificationCategory::ActionResult).unwrap(),
            r#""action_result""#
        );
        assert_eq!(
            serde_json::to_string(&NotificationCategory::Anomaly).unwrap(),
            r#""anomaly""#
        );
    }

    #[test]
    fn round_trip() {
        let p = NotificationPayload {
            title: "Reticulum degraded".into(),
            context: "Packet loss > 30%.".into(),
            category: NotificationCategory::Environment,
            subject_ref: Some("edge_node_eu_1".into()),
            evidence_refs: vec!["trace_01HX".into()],
            expires_at: None,
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: NotificationPayload = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.category, NotificationCategory::Environment));
    }

    #[test]
    fn anomaly_flagged_as_high_stakes() {
        assert!(NotificationCategory::Anomaly.is_high_stakes());
        assert!(!NotificationCategory::Environment.is_high_stakes());
        assert!(!NotificationCategory::StateChange.is_high_stakes());
    }
}
