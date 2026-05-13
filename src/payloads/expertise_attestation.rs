//! `expertise_attestation` payload — SCHEMA.md §4.10 / §7.
//!
//! Per `MISSION.md` §3.7. An existing expertise-bearer attests that
//! another contributor has expertise in a cell. Witness set required
//! at the envelope level when the attestation would jump the target's
//! Expertise standing past the cell's jump-threshold policy parameter
//! (MISSION.md §9 question 10) — gate enforced by `NodeCoreEngine` at
//! the `put_contribution` boundary.
//!
//! Body-shaped (signature, witness_set, attested_at live on the
//! `ContributionEnvelope` wrapper, not the payload). The §7 example
//! shows the full row; this struct is only what goes into the
//! envelope's `payload` field.

use serde::{Deserialize, Serialize};

use crate::cell::Cell;
use crate::identity::ContributorId;

/// `expertise_attestation` payload per SCHEMA.md §4.10 / §7.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertiseAttestation {
    /// Federation identity of the attester. MUST have non-zero
    /// Expertise standing in `cell` per `MISSION.md` §3.7; the
    /// `NodeCoreEngine::put_contribution` impl enforces this.
    pub attester_id: ContributorId,
    /// Federation identity of the target — the contributor whose
    /// Expertise standing the attestation increases.
    pub target_id: ContributorId,
    /// Expertise-granularity cell — `subject` field MUST be `None`.
    pub cell: Cell,
    /// Free-text justification. E.g. *"Target has shipped 12 well-received
    /// guide edits in this cell over 8 months."*
    pub rationale: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_omits_subject() {
        let att = ExpertiseAttestation {
            attester_id: ContributorId::new("attesterpub"),
            target_id: ContributorId::new("targetpub"),
            cell: Cell::expertise("mental_health", "am"),
            rationale: "Target has shipped 12 well-received guide edits".into(),
        };
        let json = serde_json::to_string(&att).unwrap();
        assert!(json.contains(r#""cell":{"domain":"mental_health","language":"am"}"#));
        let back: ExpertiseAttestation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.attester_id, att.attester_id);
    }
}
