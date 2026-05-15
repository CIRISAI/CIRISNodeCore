//! Typed policy payloads for the persist envelope shapes.
//!
//! Persist's wire envelopes (`ContributionEnvelope`, `ModerationEvent`,
//! `SlashingAttestation`, `ReconsiderationRequest`) carry the
//! per-row-class identity + signature + audit timestamps + an opaque
//! `payload: serde_json::Value` field. The types in this module are
//! the **policy schemas** for that Value field — typed enums and
//! discriminators that node-core owns as the consensus-layer policy
//! authority.
//!
//! Persist is the substrate, not the policy: it doesn't care what's
//! in `payload` as long as the envelope verifies. Node-core defines
//! what `payload` means per `contribution_type`.
//!
//! v0.1.0-dev coverage:
//!
//! - §4.7  [`deferral::DeferralRequestPayload`]
//! - §4.8  [`deferral::DeferralResponsePayload`]
//! - §4.10 [`expertise_attestation::ExpertiseAttestationPayload`]
//! - §4.11 [`moderation_event::ModerationEventPayload`]
//! - §8    [`slashing_attestation::SlashingAttestationPayload`]
//! - §4.12 [`reconsideration::ReconsiderationRequestPayload`]
//!
//! Pending typed payloads (Value is the only option today):
//! `arc_question` §4.1, `proposed_battery` §4.2, `prompt_edit` §4.3,
//! `guide_edit` §4.4, `accord_edit` §4.5, `failure_pattern` §4.6,
//! `wa_candidacy` §4.9, `ReconsiderationAttestation` §9.

pub mod deferral;
pub mod expertise_attestation;
pub mod moderation_event;
pub mod reconsideration;
pub mod registry_vouch;
pub mod slashing_attestation;
