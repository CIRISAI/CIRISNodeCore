//! Expertise-attestation payload — SCHEMA.md §4.10 / §7.
//!
//! Per `MISSION.md` §3.7. An existing expertise-bearer attests that
//! another contributor has expertise in a cell. Rides a
//! `ContributionEnvelope` with `contribution_type = ExpertiseAttestation`.
//!
//! Envelope-level fields:
//! - `contribution_id`
//! - `author_id` (= attester — MUST have non-zero Expertise in `subject`)
//! - `subject.{domain, language}` (Expertise-granularity, subject = None)
//! - `witness_set` (required when attestation jumps target standing
//!   past the cell's jump-threshold policy parameter)
//! - `signature`, `submitted_at`

use serde::{Deserialize, Serialize};

/// `expertise_attestation` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertiseAttestationPayload {
    /// Federation identity (base64 Ed25519) of the target contributor.
    pub target_id: String,
    /// Free-text justification. E.g. *"Target has shipped 12
    /// well-received guide edits in this cell over 8 months."*
    pub rationale: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let p = ExpertiseAttestationPayload {
            target_id: "target_pub_b64".into(),
            rationale: "12 well-received guide edits over 8 months".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: ExpertiseAttestationPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.target_id, p.target_id);
    }
}
