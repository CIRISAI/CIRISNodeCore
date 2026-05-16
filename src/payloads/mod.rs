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
//! what `payload` means per `contribution_type` + `subject_kind`.
//!
//! Each module exposes a `SUBJECT_KIND` constant matching the
//! corresponding SCHEMA.md §3.2 / §3.1 wire value, plus the typed
//! struct + round-trip serde tests. Constants must match SCHEMA
//! verbatim — that invariant is tested in each module.
//!
//! Taxonomy + placement: see [`FSD/MESSAGE_TAXONOMY.md`].
//!
//! [`FSD/MESSAGE_TAXONOMY.md`]: https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/MESSAGE_TAXONOMY.md

pub mod deferral;
pub mod expertise_attestation;
pub mod moderation_event;
pub mod reconsideration;
pub mod registry_vouch;
pub mod slashing_attestation;

// ─── §3.2 subject_kind additions (FSD/MESSAGE_TAXONOMY round) ────────────

pub mod assistance_request;
pub mod assistance_response;
pub mod cancellation;
pub mod commitment;
pub mod gratitude_signal;
pub mod improvement;
pub mod notification;
pub mod notification_response;
pub mod service_announcement;
pub mod service_deprecation;
pub mod service_usage_summary;
pub mod subscription_request;
pub mod test_result;
pub mod trust_grant;
pub mod unsolicited_guidance;
