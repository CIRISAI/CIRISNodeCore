//! Slashing-attestation payload — SCHEMA.md §8.
//!
//! Per `MISSION.md` Primitive 9 / §5.6. Quorum-issued adjudication
//! outcome for a `ModerationEvent`. Persist's `SlashingAttestation`
//! envelope carries `slashing_id`, `moderation_id` (FK), `adjudicator_id`
//! (the publisher), `attested_at`, `signature` (publisher's
//! single-sig). The multi-sig quorum collective approval and the
//! ledger-reduction details live in this typed payload.

use serde::{Deserialize, Serialize};

use crate::substrate::HybridSignature;

/// Outcome of a slashing adjudication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlashingOutcome {
    /// Quorum found the target acted against protocol terms.
    ProvenRogue,
    /// Quorum did not find proven rogue action.
    NotProven,
}

/// Disposition of the accuser's at-risk stake per `MISSION.md`
/// Primitive 9. Decimal strings to avoid float drift.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AccuserStakeDisposition {
    /// Stake returned to the accuser intact.
    Returned {
        /// Decimal string of returned amount.
        returned: String,
    },
    /// Stake returned with a bounty paid from the slashed Credits.
    ReturnWithBounty {
        /// Decimal string of returned principal.
        returned: String,
        /// Decimal string of bounty amount.
        bounty: String,
    },
    /// Stake forfeited (bad-faith filing).
    Forfeited {
        /// Decimal string of forfeited amount.
        forfeited: String,
    },
}

/// One signer's contribution to the multi-sig quorum approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumSignature {
    /// Quorum member's federation identity.
    pub signer_id: String,
    /// Quorum member's signature over the canonical attestation bytes.
    pub signature: HybridSignature,
}

/// `slashing_attestation` payload — typed schema for
/// `SlashingAttestation.payload: serde_json::Value`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingAttestationPayload {
    /// Federation identities of the quorum members who adjudicated.
    /// `quorum_signatures` MUST have one entry per `quorum_id`.
    pub quorum_ids: Vec<String>,
    /// Adjudication outcome.
    pub outcome: SlashingOutcome,
    /// Credits reduction applied to the target (decimal string).
    /// Non-negative; the engine enforces the §10 ledger floor at write.
    pub credits_reduced: String,
    /// Expertise reduction applied to the target (decimal string).
    pub expertise_reduced: String,
    /// Accuser stake disposition.
    pub accuser_stake_disposition: AccuserStakeDisposition,
    /// Multi-sig: one signature per quorum member.
    pub quorum_signatures: Vec<QuorumSignature>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_snake_case() {
        assert_eq!(
            serde_json::to_string(&SlashingOutcome::ProvenRogue).unwrap(),
            r#""proven_rogue""#
        );
    }

    #[test]
    fn disposition_tag() {
        let d = AccuserStakeDisposition::ReturnWithBounty {
            returned: "12.5".into(),
            bounty: "2.5".into(),
        };
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains(r#""kind":"return_with_bounty""#));
    }
}
