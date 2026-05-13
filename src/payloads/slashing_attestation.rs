//! `SlashingAttestation` — SCHEMA.md §8.
//!
//! Per `MISSION.md` Primitive 9 / §5.6. The outcome row that a WA
//! quorum publishes in response to a `moderation_event`. Carries the
//! multi-sig collective approval inline (`signatures`), plus the
//! disposition for the accuser's stake.
//!
//! **NOT a Contribution subtype.** Distinct row class on the federation
//! audit chain (`slashing_attestations` table per CIRISPersist
//! Appendix A.2 row 6). Written via `NodeCoreEngine::put_slashing_attestation`.
//! Edge dispatches this via `MessageType::SlashingAttestationPublish`
//! (CIRISEdge#6); the publisher (one of the quorum members or a steward
//! proxy) signs the wire envelope separately.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::identity::ContributorId;
use crate::signature::HybridSignature;

/// Outcome of a slashing adjudication per SCHEMA.md §8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlashingOutcome {
    /// Quorum found the target acted against protocol terms (rogue,
    /// not miscalibrated). Triggers ledger reductions.
    ProvenRogue,
    /// Quorum did not find proven rogue action. Stake disposition
    /// returns to the accuser; ledgers unchanged.
    NotProven,
}

/// Disposition of the accuser's at-risk stake per `MISSION.md`
/// Primitive 9. Encoded as a tagged union with the `kind` discriminator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AccuserStakeDisposition {
    /// Stake returned to the accuser intact (e.g. proven rogue outcome).
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
    /// Stake forfeited (e.g. not_proven outcome where the filing was
    /// found to be bad-faith).
    Forfeited {
        /// Decimal string of forfeited amount.
        forfeited: String,
    },
}

/// One signer's contribution to the multi-sig quorum approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumSignature {
    /// Federation identity of the quorum member.
    pub signer_id: ContributorId,
    /// Quorum member's signature over the canonical attestation bytes.
    pub signature: HybridSignature,
}

/// `SlashingAttestation` per SCHEMA.md §8.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingAttestation {
    /// ULID identifier per §2.2.
    pub attestation_id: String,
    /// The `moderation_event` Contribution this attestation responds to.
    pub moderation_event_id: String,
    /// Federation identities of the quorum members who adjudicated.
    /// `signatures` MUST have one entry per `quorum_id`.
    pub quorum_ids: Vec<ContributorId>,
    /// Adjudication outcome.
    pub outcome: SlashingOutcome,
    /// Credits reduction applied to the target (decimal string to
    /// avoid float drift). Non-negative; the `NodeCoreEngine` impl
    /// enforces the non-negative ledger floor per `MISSION.md` §2.9.
    pub credits_reduced: String,
    /// Expertise reduction applied to the target (decimal string).
    /// Non-negative; floor enforced at write.
    pub expertise_reduced: String,
    /// Accuser's stake disposition.
    pub accuser_stake_disposition: AccuserStakeDisposition,
    /// Multi-sig: one signature per quorum member.
    pub signatures: Vec<QuorumSignature>,
    /// When the quorum attested.
    pub attested_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disposition_tag_discriminator() {
        let d = AccuserStakeDisposition::ReturnWithBounty {
            returned: "12.5".into(),
            bounty: "2.5".into(),
        };
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains(r#""kind":"return_with_bounty""#));
        let back: AccuserStakeDisposition = serde_json::from_str(&json).unwrap();
        match back {
            AccuserStakeDisposition::ReturnWithBounty { returned, bounty } => {
                assert_eq!(returned, "12.5");
                assert_eq!(bounty, "2.5");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn slashing_outcome_serde_snake_case() {
        let o = SlashingOutcome::ProvenRogue;
        assert_eq!(serde_json::to_string(&o).unwrap(), r#""proven_rogue""#);
    }
}
