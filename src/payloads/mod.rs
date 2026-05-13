//! Typed payloads per SCHEMA.md §4 and §8-§9.
//!
//! The Contribution envelope (`crate::contribution::ContributionEnvelope`)
//! carries `payload: serde_json::Value`. These typed structs are what
//! that JSON deserializes into, discriminated by the envelope's
//! `contribution_type` + `subject.subject_kind`.
//!
//! Standalone (non-Contribution) attestation row classes — currently
//! [`slashing_attestation::SlashingAttestation`] — live alongside the
//! payloads since they share the typed-wire dispatch shape but are
//! written to their own persist tables via dedicated `NodeCoreEngine`
//! methods (Appendix A.2 rows 6 / 8).
//!
//! v0.1.0-dev coverage:
//!
//! - §4.7 [`deferral::DeferralRequest`]
//! - §4.8 [`deferral::DeferralResponse`]
//! - §4.10 / §7 [`expertise_attestation::ExpertiseAttestation`]
//! - §4.11 / §8 [`moderation_event::ModerationEvent`]
//! - §8 [`slashing_attestation::SlashingAttestation`] (standalone)
//! - §4.12 / §9 [`reconsideration::ReconsiderationRequest`]
//!
//! Remaining (covered by `serde_json::Value` in the envelope payload
//! field for v0.1.0-dev; typed structs land in v0.1.0 cut):
//!
//! - §4.1 `arc_question`
//! - §4.2 `proposed_battery`
//! - §4.3 `prompt_edit`
//! - §4.4 `guide_edit`
//! - §4.5 `accord_edit`
//! - §4.6 `failure_pattern` (ticket)
//! - §4.9 `wa_candidacy`
//! - §9 `ReconsiderationAttestation` (the quorum-issued outcome row)

pub mod deferral;
pub mod expertise_attestation;
pub mod moderation_event;
pub mod reconsideration;
pub mod slashing_attestation;
