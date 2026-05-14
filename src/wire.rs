//! Edge wire-dispatch newtypes for the 8 federation-consensus
//! `MessageType` variants (CIRISEdge v0.1.4 — surface stable since v0.1.2).
//!
//! Each newtype is `#[serde(transparent)]` so JSON wire-encoding is
//! identical to persist's underlying envelope type — the wrapper
//! exists solely to satisfy Rust's orphan rule for `impl Message`
//! (both `ciris_edge::Message` and persist's envelope types live in
//! foreign crates).
//!
//! All 8 messages ship under [`DURABLE_CONSENSUS`] delivery policy
//! (`requires_ack=true`, `max_attempts=6`, `ttl=7d`, `ack_timeout=2d`).
//! Policy-tunable per deployment; constants here are v0.1.0-dev defaults.
//!
//! Per-variant body type:
//!
//! | MessageType variant | Body type (persist envelope) |
//! |---|---|
//! | `ContributionSubmit` | `ContributionEnvelope` (generic — any `contribution_type`) |
//! | `VoteCast` | `VoteEnvelope` |
//! | `DeferralRequest` | `ContributionEnvelope` (type=DeferralRequest) |
//! | `DeferralResponse` | `ContributionEnvelope` (type=DeferralResponse) |
//! | `ExpertiseAttestationPublish` | `ContributionEnvelope` (type=ExpertiseAttestation) |
//! | `ModerationEventPublish` | `ModerationEvent` (standalone row class) |
//! | `SlashingAttestationPublish` | `SlashingAttestation` (standalone) |
//! | `ReconsiderationRequest` | `ReconsiderationRequest` (standalone) |
//!
//! Per-variant Ack response shape:
//!
//! | Variant | Ack type | What it carries |
//! |---|---|---|
//! | `ContributionSubmit` | [`ContributionAck`] | `contribution_id`, `accepted_at` |
//! | `VoteCast` | [`VoteAck`] | cast-time [`VoteWeight`] for sender display |
//! | `DeferralRequest` | [`DeferralRouting`] | routed-set per §3.3 |
//! | `DeferralResponse` | [`DeferralResponseAck`] | `response_id`, `accepted_at` |
//! | `ExpertiseAttestationPublish` | [`ExpertiseAttestationAck`] | + `jump_threshold_triggered` |
//! | `ModerationEventPublish` | [`ModerationEventAck`] | `moderation_id`, `accepted_at` |
//! | `SlashingAttestationPublish` | [`SlashingAttestationAck`] | `slashing_id`, `accepted_at` |
//! | `ReconsiderationRequest` | [`ReconsiderationRequestAck`] | `request_id`, `accepted_at` |

use chrono::{DateTime, Utc};
use ciris_edge::{Delivery, Message, MessageType};
use serde::{Deserialize, Serialize};

use crate::substrate::{
    ContributionEnvelope as PContributionEnvelope, ModerationEvent as PModerationEvent,
    ReconsiderationRequest as PReconsiderationRequest, SlashingAttestation as PSlashingAttestation,
    VoteEnvelope as PVoteEnvelope, VoteWeight,
};

// ── Delivery policy ──────────────────────────────────────────────────────

const DAY_SECONDS: u64 = 86_400;

/// Default durable-delivery policy for the 8 federation-consensus
/// wire types. Policy-tunable per deployment.
pub const DURABLE_CONSENSUS: Delivery = Delivery::Durable {
    requires_ack: true,
    max_attempts: 6,
    ttl_seconds: 7 * DAY_SECONDS,
    ack_timeout_seconds: Some(2 * DAY_SECONDS),
};

// ── Wire newtype wrappers ────────────────────────────────────────────────

/// Wire message — generic Contribution submit. Body is persist's
/// `ContributionEnvelope` with any `contribution_type`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContributionSubmit(pub PContributionEnvelope);

/// Wire message — Vote cast. Body is persist's `VoteEnvelope`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VoteCast(pub PVoteEnvelope);

/// Wire message — Deferral request (type-specific dispatch alongside
/// `ContributionSubmit`). Body MUST have
/// `contribution_type = DeferralRequest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeferralRequest(pub PContributionEnvelope);

/// Wire message — Deferral response. Body MUST have
/// `contribution_type = DeferralResponse`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeferralResponse(pub PContributionEnvelope);

/// Wire message — Expertise attestation. Body MUST have
/// `contribution_type = ExpertiseAttestation`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExpertiseAttestationPublish(pub PContributionEnvelope);

/// Wire message — Moderation event publication. Body is persist's
/// standalone `ModerationEvent` envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModerationEventPublish(pub PModerationEvent);

/// Wire message — Slashing attestation publication. Body is persist's
/// standalone `SlashingAttestation` envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SlashingAttestationPublish(pub PSlashingAttestation);

/// Wire message — Reconsideration request. Body is persist's
/// standalone `ReconsiderationRequest` envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReconsiderationRequest(pub PReconsiderationRequest);

// ── Ack response types ───────────────────────────────────────────────────

/// Ack for `ContributionSubmit` / `DeferralResponse`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionAck {
    /// Persisted contribution id (echoes `envelope.contribution_id`).
    pub contribution_id: String,
    /// When persist accepted the write.
    pub accepted_at: DateTime<Utc>,
}

/// Ack for `VoteCast`. Carries cast-time weight so the sender can
/// display "your vote counted as W" without a second round-trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteAck {
    /// Persisted vote id (echoes `envelope.vote_id`).
    pub vote_id: String,
    /// Weight at cast time per `SCHEMA.md` §5.2.
    pub weight: VoteWeight,
    /// When persist accepted the cast.
    pub recorded_at: DateTime<Utc>,
}

/// Ack for `DeferralRequest`. Returns the routed-set per
/// `MISSION.md` §3.3 so the consumer knows who's being asked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferralRouting {
    /// Echo of the request's `contribution_id`.
    pub deferral_id: String,
    /// Federation identities selected per §3.3.
    pub routed_responders: Vec<String>,
    /// When persist accepted the request.
    pub accepted_at: DateTime<Utc>,
}

/// Ack for `DeferralResponse`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferralResponseAck {
    /// Persisted response id.
    pub response_id: String,
    /// When persist accepted the response.
    pub accepted_at: DateTime<Utc>,
}

/// Ack for `ExpertiseAttestationPublish`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertiseAttestationAck {
    /// Persisted contribution id.
    pub contribution_id: String,
    /// When persist accepted the attestation.
    pub accepted_at: DateTime<Utc>,
    /// Whether this attestation triggered the cell's jump-threshold
    /// witness-set gate per `MISSION.md` §3.7.
    pub jump_threshold_triggered: bool,
}

/// Ack for `ModerationEventPublish`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationEventAck {
    /// Persisted moderation id (echoes `envelope.moderation_id`).
    pub moderation_id: String,
    /// When persist accepted the filing.
    pub accepted_at: DateTime<Utc>,
}

/// Ack for `SlashingAttestationPublish`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingAttestationAck {
    /// Persisted attestation id (echoes `envelope.slashing_id`).
    pub slashing_id: String,
    /// When persist accepted the attestation.
    pub accepted_at: DateTime<Utc>,
}

/// Ack for `ReconsiderationRequest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconsiderationRequestAck {
    /// Persisted request id (echoes `envelope.request_id`).
    pub request_id: String,
    /// When persist accepted the request.
    pub accepted_at: DateTime<Utc>,
}

// ── Message impls ────────────────────────────────────────────────────────

impl Message for ContributionSubmit {
    const TYPE: MessageType = MessageType::ContributionSubmit;
    const DELIVERY: Delivery = DURABLE_CONSENSUS;
    type Response = ContributionAck;
}

impl Message for VoteCast {
    const TYPE: MessageType = MessageType::VoteCast;
    const DELIVERY: Delivery = DURABLE_CONSENSUS;
    type Response = VoteAck;
}

impl Message for DeferralRequest {
    const TYPE: MessageType = MessageType::DeferralRequest;
    const DELIVERY: Delivery = DURABLE_CONSENSUS;
    type Response = DeferralRouting;
}

impl Message for DeferralResponse {
    const TYPE: MessageType = MessageType::DeferralResponse;
    const DELIVERY: Delivery = DURABLE_CONSENSUS;
    type Response = DeferralResponseAck;
}

impl Message for ExpertiseAttestationPublish {
    const TYPE: MessageType = MessageType::ExpertiseAttestationPublish;
    const DELIVERY: Delivery = DURABLE_CONSENSUS;
    type Response = ExpertiseAttestationAck;
}

impl Message for ModerationEventPublish {
    const TYPE: MessageType = MessageType::ModerationEventPublish;
    const DELIVERY: Delivery = DURABLE_CONSENSUS;
    type Response = ModerationEventAck;
}

impl Message for SlashingAttestationPublish {
    const TYPE: MessageType = MessageType::SlashingAttestationPublish;
    const DELIVERY: Delivery = DURABLE_CONSENSUS;
    type Response = SlashingAttestationAck;
}

impl Message for ReconsiderationRequest {
    const TYPE: MessageType = MessageType::ReconsiderationRequest;
    const DELIVERY: Delivery = DURABLE_CONSENSUS;
    type Response = ReconsiderationRequestAck;
}
