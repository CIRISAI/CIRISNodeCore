//! Contribution envelope — the common shell for every federation
//! Contribution per SCHEMA.md §3.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::cell::Cell;
use crate::identity::ContributorId;
use crate::signature::HybridSignature;
use crate::witness::WitnessSet;

/// Top-level Contribution discriminator per SCHEMA.md §3.1.
///
/// Distinct from [`SubjectKind`] which discriminates payload shapes
/// *within* `proposal`-type Contributions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContributionType {
    /// Consumer requests routing to qualified WAs. Payload: §4.7.
    /// Generalizes CIRISNode's existing WBD submit surface per
    /// `MISSION.md` §1.2 / §3.3.
    DeferralRequest,
    /// Routed WA's signed response to a deferral. Payload: §4.8.
    DeferralResponse,
    /// Battery, free-form argument, policy proposal, edit proposal, etc.
    /// Sub-discriminated by `subject.subject_kind`. Payloads: §4.1–§4.6.
    Proposal,
    /// Self- or peer-nomination for Wise Authority standing in a cell,
    /// gated on Credits + Expertise thresholds (§3.6). Payload: §4.9.
    WaCandidacy,
    /// Expertise-bearer attests another contributor has expertise in a
    /// cell. Payload: §4.10.
    ExpertiseAttestation,
    /// Accusation of rogue action. Payload: §4.11. Witness-set required.
    ModerationEvent,
    /// Signed request to reverse a prior SlashingAttestation. Payload:
    /// §4.12. Witness-set required.
    ReconsiderationRequest,
}

/// Discriminator for `proposal`-type Contribution payload shapes per
/// SCHEMA.md §3.2.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKind {
    /// A single safety-battery question. Witness: none (routine).
    ArcQuestion,
    /// A whole battery (set of questions for a cell). Witness: required
    /// if magnitude exceeds threshold per §3.5.
    ProposedBattery,
    /// A diff against the canonical `prompts.*` block for a locale.
    /// Witness: required (high-stakes — affects every agent response).
    PromptEdit,
    /// A diff against the canonical Comprehensive Guide for a locale.
    /// Witness: required.
    GuideEdit,
    /// A diff against the canonical localized ACCORD body. Witness:
    /// required.
    AccordEdit,
    /// A signed ticket: agent observed to fail pattern X with evidence.
    /// Witness: none for filing; required for adjudication.
    FailurePattern,
    /// Narrative argument or commentary. Witness: none.
    FreeForm,
}

/// Subject — `Cell` + optional `subject_kind` for `proposal`-type
/// Contributions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subject {
    /// Domain.
    pub domain: String,
    /// Language.
    pub language: String,
    /// Subject-kind discriminator. Only set for `contribution_type =
    /// proposal`; `None` for every other top-level type (the type
    /// itself discriminates).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_kind: Option<SubjectKind>,
}

impl Subject {
    /// Extract the Expertise-granularity `Cell` (drops `subject_kind`).
    pub fn cell(&self) -> Cell {
        Cell::expertise(&self.domain, &self.language)
    }
}

/// Contribution envelope per SCHEMA.md §3.
///
/// The `payload` is left as `serde_json::Value` because the shape varies
/// by `contribution_type` and `subject.subject_kind`. Typed payload
/// structs live in [`crate::payloads`]; validation pulls the typed
/// shape out at the validation boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionEnvelope {
    /// ULID identifier per §2.2.
    pub contribution_id: String,
    /// Top-level type discriminator. See [`ContributionType`].
    pub contribution_type: ContributionType,
    /// Federation identity of the author. MUST match `signature.ed25519`
    /// signer; edge enforces this at the wire (node-core never sees
    /// an envelope whose author doesn't match the signing key).
    pub author_id: ContributorId,
    /// Cell + (for `proposal`-type) subject_kind.
    pub subject: Subject,
    /// Payload — shape varies by type. See [`crate::payloads`].
    pub payload: serde_json::Value,
    /// Witness set. Required for high-stakes Contributions per §3.5;
    /// `None` for routine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub witness_set: Option<WitnessSet>,
    /// Author's hybrid signature over the canonical envelope.
    pub signature: HybridSignature,
    /// Submission timestamp.
    pub submitted_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contribution_type_round_trip() {
        let t = ContributionType::DeferralRequest;
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, "\"deferral_request\"");
        let back: ContributionType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn subject_kind_round_trip() {
        let k = SubjectKind::ArcQuestion;
        let json = serde_json::to_string(&k).unwrap();
        assert_eq!(json, "\"arc_question\"");
    }

    #[test]
    fn subject_omits_subject_kind_for_non_proposal() {
        let s = Subject {
            domain: "mental_health".into(),
            language: "am".into(),
            subject_kind: None,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, r#"{"domain":"mental_health","language":"am"}"#);
    }
}
