//! `service_usage_summary` payload — SCHEMA.md §4.25.
//!
//! Aggregated per-window usage report. Aggregated to the chain (not
//! per-call) to keep audit-chain volume sane while preserving
//! accountability + commons-credit attribution.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `subject_kind` discriminator. Wire constant; matches SCHEMA §3.2.
pub const SUBJECT_KIND: &str = "service_usage_summary";

/// `service_usage_summary` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceUsageSummaryPayload {
    /// The service being reported on.
    pub service_announcement_id: String,
    /// Reporting window start.
    pub window_start: DateTime<Utc>,
    /// Reporting window end.
    pub window_end: DateTime<Utc>,
    /// Total calls in window.
    pub invocation_count: u64,
    /// Calls that completed without error.
    pub successful_count: u64,
    /// Calls that errored.
    pub failed_count: u64,
    /// Service-kind-specific metrics. Schema varies per service kind.
    /// Left as opaque JSON for flexibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregate_metrics: Option<serde_json::Value>,
    /// Per-caller call counts (pubkey → count). Privacy-policy-gated;
    /// operators MAY redact callers below a noise floor.
    /// `BTreeMap` for deterministic JSON serialization.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub caller_distribution: BTreeMap<String, u64>,
}

impl ServiceUsageSummaryPayload {
    /// Validate that `successful + failed == invocation_count`.
    /// Returns `Ok(())` on consistency, `Err(...)` with the mismatch
    /// detail otherwise.
    pub fn validate_counts(&self) -> Result<(), String> {
        let sum = self.successful_count.saturating_add(self.failed_count);
        if sum != self.invocation_count {
            Err(format!(
                "invocation_count {} != successful {} + failed {} = {}",
                self.invocation_count, self.successful_count, self.failed_count, sum
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_kind_constant_matches_schema() {
        assert_eq!(SUBJECT_KIND, "service_usage_summary");
    }

    #[test]
    fn round_trip() {
        let mut callers = BTreeMap::new();
        callers.insert("caller_a_pub".into(), 412);
        callers.insert("caller_b_pub".into(), 188);
        let p = ServiceUsageSummaryPayload {
            service_announcement_id: "01HX".into(),
            window_start: Utc::now(),
            window_end: Utc::now(),
            invocation_count: 1247,
            successful_count: 1219,
            failed_count: 28,
            aggregate_metrics: Some(serde_json::json!({"p50_ms": 320})),
            caller_distribution: callers,
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: ServiceUsageSummaryPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.invocation_count, 1247);
        assert!(back.validate_counts().is_ok());
    }

    #[test]
    fn inconsistent_counts_detected() {
        let p = ServiceUsageSummaryPayload {
            service_announcement_id: "01HX".into(),
            window_start: Utc::now(),
            window_end: Utc::now(),
            invocation_count: 100,
            successful_count: 50,
            failed_count: 10, // 50+10 = 60, not 100
            aggregate_metrics: None,
            caller_distribution: BTreeMap::new(),
        };
        assert!(p.validate_counts().is_err());
    }
}
