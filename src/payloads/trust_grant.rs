//! `trust_grant` payload — SCHEMA.md §4.14.
//!
//! Per `CIRISPersist/FSD/FEDERATION_TRUST_INTERFACE.md` §3.2 +
//! `FSD/MESSAGE_TAXONOMY.md` §4. Purpose-scoped trust grant from the
//! envelope's `author_id` to a `grantee_key`. Materializes a row in
//! `federation_trust_grants` when the Contribution lands on the audit
//! chain (persist v1.5.0 ingest hook).
//!
//! Encoded as a `proposal`-type Contribution with
//! `subject.subject_kind = "trust_grant"`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::trust::TrustPurpose;

/// `subject_kind` discriminator. Wire constant; matches SCHEMA §3.2.
pub const SUBJECT_KIND: &str = "trust_grant";

/// `trust_grant` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustGrantPayload {
    /// K_C — the key being trusted. Base64 hybrid pubkey per SCHEMA §2.2.
    pub grantee_key: String,
    /// Purpose axis. Scope shape depends on this.
    pub purpose: TrustPurpose,
    /// Purpose-specific opaque string. See `TrustPurpose` docs +
    /// `CIRISPersist/FSD/FEDERATION_TRUST_INTERFACE.md` §3.3 for the
    /// scope grammar per purpose.
    pub scope: String,
    /// `None` = open-ended. Engine projects expired grants as if revoked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// Free-text justification recorded on the audit chain.
    pub rationale: String,
}

impl TrustGrantPayload {
    /// True if the grant is still in force at `now`.
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        match self.expires_at {
            Some(t) => t > now,
            None => true,
        }
    }

    /// True if the scope is a wildcard. Wildcard grants are a strict
    /// trust elevation per persist FSD §3.3 — consumers should
    /// require witness-set per SCHEMA §3.5.
    pub fn is_wildcard(&self) -> bool {
        self.scope == "*"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_kind_constant_matches_schema() {
        assert_eq!(SUBJECT_KIND, "trust_grant");
    }

    #[test]
    fn round_trip() {
        let p = TrustGrantPayload {
            grantee_key: "K_C_pub_b64".into(),
            purpose: TrustPurpose::Service,
            scope: "service:llm:claude-opus-4-7".into(),
            expires_at: None,
            rationale: "Provider has shipped consistent quality on 30+ calls.".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains(r#""purpose":"service""#));
        let back: TrustGrantPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.scope, p.scope);
        assert!(matches!(back.purpose, TrustPurpose::Service));
    }

    #[test]
    fn wildcard_detection() {
        let p = TrustGrantPayload {
            grantee_key: "K".into(),
            purpose: TrustPurpose::Contribution,
            scope: "*".into(),
            expires_at: None,
            rationale: String::new(),
        };
        assert!(p.is_wildcard());

        let q = TrustGrantPayload {
            scope: "proposal:registry_vouch".into(),
            ..p
        };
        assert!(!q.is_wildcard());
    }

    #[test]
    fn purpose_snake_case() {
        assert_eq!(
            serde_json::to_string(&TrustPurpose::Technical).unwrap(),
            r#""technical""#
        );
        assert_eq!(
            serde_json::to_string(&TrustPurpose::Deferral).unwrap(),
            r#""deferral""#
        );
        assert_eq!(
            serde_json::to_string(&TrustPurpose::Contribution).unwrap(),
            r#""contribution""#
        );
        assert_eq!(
            serde_json::to_string(&TrustPurpose::Service).unwrap(),
            r#""service""#
        );
    }
}
