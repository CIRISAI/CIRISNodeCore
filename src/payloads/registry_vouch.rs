//! `registry_vouch` payload — SCHEMA.md §4.13.
//!
//! Per `FSD/TRUST_HIERARCHY.md`. A registry key vouches that another
//! key is a qualified resolver in a domain. Encoded as a
//! `proposal`-type `ContributionEnvelope` with
//! `subject.subject_kind = "registry_vouch"` (NOT a new §3.1
//! contribution_type variant — keeps persist's enum stable; storage
//! shape is identical).
//!
//! Envelope-level:
//! - `author_id` = the registry key (K_B doing the vouching)
//! - `subject.{domain, language, subject = Some("registry_vouch")}`
//! - `witness_set` required when the vouch jumps K_C's
//!   transitive-trust count past the cell's jump-threshold policy
//!   parameter (mirrors `ExpertiseAttestation` gate)
//! - `signature`, `submitted_at` standard

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `subject_kind` discriminator value for this payload type. Wire
/// constant; matches the SCHEMA.md §3.2 enum entry.
pub const SUBJECT_KIND: &str = "registry_vouch";

/// `registry_vouch` payload — typed schema for the envelope's
/// `payload: serde_json::Value` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryVouchPayload {
    /// K_C — the key being vouched for. Federation identity (the
    /// pubkey) per SCHEMA §2.2.
    pub vouched_key: String,
    /// Domain scope of the vouch. MUST be one of the cell-permitted
    /// domain identifiers per the canonical taxonomy (TBD per
    /// `FSD/TRUST_HIERARCHY.md` §9).
    pub vouched_domain: String,
    /// `None` = open-ended. Engine-side query treats expired vouches
    /// as if revoked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// Free-text justification recorded on the audit chain.
    pub rationale: String,
}

impl RegistryVouchPayload {
    /// True if the vouch is still in force at `now`. Expired vouches
    /// do not contribute to transitive-trust queries.
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
        assert_eq!(SUBJECT_KIND, "registry_vouch");
    }

    #[test]
    fn round_trip() {
        let p = RegistryVouchPayload {
            vouched_key: "K_C_pub_b64".into(),
            vouched_domain: "medical_deferral".into(),
            expires_at: None,
            rationale: "Verified board certification".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: RegistryVouchPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.vouched_key, p.vouched_key);
        assert_eq!(back.vouched_domain, p.vouched_domain);
    }

    #[test]
    fn open_ended_vouch_is_always_active() {
        let p = RegistryVouchPayload {
            vouched_key: "K_C".into(),
            vouched_domain: "x".into(),
            expires_at: None,
            rationale: String::new(),
        };
        assert!(p.is_active_at(Utc::now()));
    }

    #[test]
    fn expired_vouch_is_inactive() {
        let p = RegistryVouchPayload {
            vouched_key: "K_C".into(),
            vouched_domain: "x".into(),
            expires_at: Some(Utc::now() - chrono::Duration::seconds(1)),
            rationale: String::new(),
        };
        assert!(!p.is_active_at(Utc::now()));
    }
}
