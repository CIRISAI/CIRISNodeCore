//! `service_deprecation` payload — SCHEMA.md §4.24.
//!
//! Retracts a prior `service_announcement`. Author-only revocation
//! per `FSD/MESSAGE_TAXONOMY.md` §4 (mirrors §4.13 `registry_vouch`
//! / §4.14 `trust_grant` precedent).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `subject_kind` discriminator. Wire constant; matches SCHEMA §3.2.
pub const SUBJECT_KIND: &str = "service_deprecation";

/// `service_deprecation` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDeprecationPayload {
    /// Back-ref to the `service_announcement` being retracted. MUST
    /// be authored by the same key issuing this deprecation.
    pub service_announcement_id: String,
    /// When the deprecation takes effect. `now()` for immediate;
    /// future for graceful retirement.
    pub effective_at: DateTime<Utc>,
    /// Free-text rationale recorded on the audit chain.
    pub reason: String,
}

impl ServiceDeprecationPayload {
    /// True if the deprecation is in force at `now`.
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        self.effective_at <= now
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn subject_kind_constant_matches_schema() {
        assert_eq!(SUBJECT_KIND, "service_deprecation");
    }

    #[test]
    fn round_trip() {
        let p = ServiceDeprecationPayload {
            service_announcement_id: "01HX".into(),
            effective_at: Utc::now(),
            reason: "Model claude-opus-4-7 deprecated.".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: ServiceDeprecationPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.service_announcement_id, p.service_announcement_id);
    }

    #[test]
    fn future_effective_at_not_yet_active() {
        let p = ServiceDeprecationPayload {
            service_announcement_id: "01HX".into(),
            effective_at: Utc::now() + Duration::days(7),
            reason: String::new(),
        };
        assert!(!p.is_active_at(Utc::now()));
    }

    #[test]
    fn past_effective_at_active() {
        let p = ServiceDeprecationPayload {
            service_announcement_id: "01HX".into(),
            effective_at: Utc::now() - Duration::seconds(1),
            reason: String::new(),
        };
        assert!(p.is_active_at(Utc::now()));
    }
}
