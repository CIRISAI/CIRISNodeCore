//! `moderation_event` payload — SCHEMA.md §4.11 / §8.
//!
//! Per `MISSION.md` §4.7 / §5.6. An accusation of rogue action against
//! a Contribution, Vote, or attestation. Witness set always required
//! at the envelope level per §3.5 — the `NodeCoreEngine::put_moderation_event`
//! impl enforces this. Outcome materializes as a separate
//! `SlashingAttestation` row (see `slashing_attestation.rs`).

use serde::{Deserialize, Serialize};

use crate::identity::ContributorId;

/// What kind of row the moderation event targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    /// A Contribution envelope.
    Contribution,
    /// A Vote.
    Vote,
    /// An ExpertiseAttestation.
    ExpertiseAttestation,
}

/// Allegation category per SCHEMA.md §8.
///
/// `MISSION.md` §4.7 narrows the rogue-action surface to actions that
/// violate protocol — bad-faith voting, coordinated voting, attestations
/// outside the attester's distribution, evidence of external inducement,
/// or expertise-claim fraud. Miscalibration is **not** slashable per
/// `MISSION.md` Primitive 9 — that lives outside this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Allegation {
    /// Target voted in bad faith (e.g. with knowledge that the vote
    /// did not reflect their actual judgment).
    RogueVote,
    /// Target participated in coordinated voting per `MISSION.md`
    /// §6.4 anti-Sybil tuning.
    CoordinatedVoting,
    /// Target's ExpertiseAttestation falls outside their attester
    /// distribution per `MISSION.md` §3.7 hard-case signals.
    OutOfDistributionAttestation,
    /// Evidence the target was externally induced — bribery,
    /// pressure, coercion. Per `MISSION.md` §6.4.
    ExternalInducementEvidence,
    /// Target claims expertise they do not possess.
    ExpertiseFraud,
}

/// `moderation_event` payload per SCHEMA.md §4.11 / §8.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationEvent {
    /// Federation identity of the accuser.
    pub accuser_id: ContributorId,
    /// What kind of row is being accused.
    pub target_kind: TargetKind,
    /// ULID of the target row.
    pub target_id: String,
    /// Allegation category.
    pub allegation: Allegation,
    /// Canonical-encoded evidence payload. Application-specific shape
    /// inside this opaque blob; the audit chain stores it verbatim for
    /// forensic completeness.
    pub evidence: String,
    /// Accuser's at-risk stake in Commons Credits. Decimal string (not
    /// `f64`) to avoid drift on the audit chain. Disposition determined
    /// by the SlashingAttestation outcome per `MISSION.md` Primitive 9.
    pub accuser_stake: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allegation_serde_snake_case() {
        let a = Allegation::RogueVote;
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(json, r#""rogue_vote""#);
    }

    #[test]
    fn moderation_event_round_trip() {
        let m = ModerationEvent {
            accuser_id: ContributorId::new("accuserpub"),
            target_kind: TargetKind::Vote,
            target_id: "vote_01HX".into(),
            allegation: Allegation::CoordinatedVoting,
            evidence: "{\"timing\":[...]}".into(),
            accuser_stake: "12.5".into(),
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: ModerationEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.allegation, m.allegation);
        assert_eq!(back.accuser_stake, "12.5");
    }
}
