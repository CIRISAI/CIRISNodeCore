//! `subscription_request` payload — SCHEMA.md §4.27.
//!
//! Subscribe to an ongoing notification stream matching a filter.
//! Per `FSD/MESSAGE_TAXONOMY.md` §7 (FIPA `subscribe` /
//! `request-whenever` gap). Trust-gated — the publisher checks the
//! subscriber's trust grants before honoring the subscription.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `subject_kind` discriminator. Wire constant; matches SCHEMA §3.2.
pub const SUBJECT_KIND: &str = "subscription_request";

/// Subscription filter — what events the subscriber wants.
/// `subject_kind` is the primary discriminator; subject-kind-specific
/// fields narrow within. Opaque JSON for forward compatibility — new
/// filter fields don't require schema changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionFilter {
    /// Required — one of SCHEMA §3.2 `subject_kind` values, OR a §3.1
    /// `contribution_type` value (allowed for top-level
    /// `deferral_request` / etc. subscriptions).
    pub subject_kind: String,
    /// Subject-kind-specific narrowing fields (e.g. `category` for
    /// `notification`, `service_kind` for `service_announcement`).
    /// Schema is consumer-policy; left as opaque JSON.
    #[serde(default, flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// `subscription_request` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRequestPayload {
    /// Federation identity of the agent being subscribed to.
    pub publisher_key: String,
    /// Subscription filter.
    pub filter: SubscriptionFilter,
    /// When the subscription auto-expires. `None` = open-ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// Edge transport hint. `None` = publisher chooses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_endpoint: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_kind_constant_matches_schema() {
        assert_eq!(SUBJECT_KIND, "subscription_request");
    }

    #[test]
    fn round_trip() {
        let mut extra = serde_json::Map::new();
        extra.insert("category".into(), serde_json::json!("anomaly"));
        let p = SubscriptionRequestPayload {
            publisher_key: "pub_b64".into(),
            filter: SubscriptionFilter {
                subject_kind: "notification".into(),
                extra,
            },
            expires_at: None,
            delivery_endpoint: None,
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: SubscriptionRequestPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.filter.subject_kind, "notification");
        assert_eq!(
            back.filter.extra.get("category").and_then(|v| v.as_str()),
            Some("anomaly")
        );
    }
}
