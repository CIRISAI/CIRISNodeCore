//! Moderation-event payload — SCHEMA.md §4.11 / §8.
//!
//! Per `MISSION.md` §4.7 / §5.6. An accusation of rogue action.
//! Persist's `ModerationEvent` envelope carries `moderation_id`,
//! `target_contributor` (the accused identity), `accuser_id`,
//! `filed_at`, `signature` at the envelope level; the typed policy
//! shape below fills `envelope.payload`.

use serde::{Deserialize, Serialize};

/// Which row class the accusation targets — disambiguates the action
/// being called rogue, separate from `envelope.target_contributor`
/// which identifies the actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    /// A Contribution envelope.
    Contribution,
    /// A Vote.
    Vote,
    /// An ExpertiseAttestation Contribution.
    ExpertiseAttestation,
}

/// Allegation category per SCHEMA.md §8.
///
/// `MISSION.md` Primitive 9 — miscalibration is NOT slashable; this
/// enum covers protocol violations only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Allegation {
    /// Target voted in bad faith.
    RogueVote,
    /// Coordinated voting per `MISSION.md` §6.4 anti-Sybil tuning.
    CoordinatedVoting,
    /// ExpertiseAttestation outside the attester's distribution.
    OutOfDistributionAttestation,
    /// Evidence of external inducement (bribery, coercion).
    ExternalInducementEvidence,
    /// Target claims expertise they do not possess.
    ExpertiseFraud,
}

/// `moderation_event` payload — the typed schema for
/// `ModerationEvent.payload: serde_json::Value`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationEventPayload {
    /// Which row class the specific allegation targets.
    pub target_kind: TargetKind,
    /// ULID of the specific target row (distinct from
    /// `envelope.target_contributor`, which is the actor identity).
    pub target_row_id: String,
    /// Allegation category.
    pub allegation: Allegation,
    /// Canonical-encoded evidence payload.
    pub evidence: String,
    /// Accuser's at-risk stake in Commons Credits. Decimal string to
    /// avoid float drift on the audit chain.
    pub accuser_stake: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allegation_snake_case() {
        assert_eq!(
            serde_json::to_string(&Allegation::RogueVote).unwrap(),
            r#""rogue_vote""#
        );
    }

    #[test]
    fn round_trip() {
        let p = ModerationEventPayload {
            target_kind: TargetKind::Vote,
            target_row_id: "vote_01HX".into(),
            allegation: Allegation::CoordinatedVoting,
            evidence: "{\"timing\":[...]}".into(),
            accuser_stake: "12.5".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: ModerationEventPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back.allegation, p.allegation);
        assert_eq!(back.accuser_stake, "12.5");
    }
}
