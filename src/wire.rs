//! Edge wire contract — `impl Message` for node-core's body types.
//!
//! Closes CIRISEdge#6 from the consumer side. Edge v0.1.2 introduced
//! 8 federation-consensus `MessageType` variants; node-core owns the
//! body structs and points each back to its variant via the
//! `ciris_edge::Message` trait.
//!
//! All federation-consensus messages ship durable+requires_ack with
//! the [`DURABLE_CONSENSUS`] policy below. The values are policy-tunable
//! per deployment; the constants here are the v0.1.0-dev defaults.

use chrono::{DateTime, Utc};
use ciris_edge::{Delivery, Message, MessageType};
use serde::{Deserialize, Serialize};

use crate::contribution::ContributionEnvelope;
use crate::identity::ContributorId;
use crate::ledger::VoteWeight;
use crate::payloads::deferral::{DeferralRequest, DeferralResponse};
use crate::payloads::expertise_attestation::ExpertiseAttestation;
use crate::payloads::moderation_event::ModerationEvent;
use crate::payloads::reconsideration::ReconsiderationRequest;
use crate::payloads::slashing_attestation::SlashingAttestation;
use crate::vote::Vote;

// ── Delivery policy ──────────────────────────────────────────────────────

const DAY_SECONDS: u64 = 86_400;

/// Default durable-delivery policy for federation-consensus messages.
/// All 8 CIRISEdge#6 variants ship under this profile.
///
/// - `requires_ack: true` — receiver signs + returns an Ack envelope
///   before the row transitions to delivered. Aligns with the
///   audit-chain story: consensus messages always have an ACK trail.
/// - `max_attempts: 6` — bounded retry; abandons with
///   `abandoned_reason='max_attempts'` after the sixth try.
/// - `ttl_seconds: 7 days` — federation timescale, not hot-path.
/// - `ack_timeout_seconds: 2 days` — generous so deferral responders
///   on slow links (Reticulum / LoRa Phase 3) have time to respond.
pub const DURABLE_CONSENSUS: Delivery = Delivery::Durable {
    requires_ack: true,
    max_attempts: 6,
    ttl_seconds: 7 * DAY_SECONDS,
    ack_timeout_seconds: Some(2 * DAY_SECONDS),
};

// ── Response (ACK) types ─────────────────────────────────────────────────

/// Receiver's ACK to a `ContributionSubmit`. Returned by the typed
/// handler post-`engine.put_contribution`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionAck {
    /// Persisted contribution id (may equal the envelope's id, or wrap it).
    pub contribution_id: String,
    /// When persist accepted the write.
    pub accepted_at: DateTime<Utc>,
}

/// Receiver's ACK to a `VoteCast`. Carries the cast-time vote weight
/// so the sender can display "your vote counted as W" without a
/// second round-trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteAck {
    /// Persisted vote id.
    pub vote_id: String,
    /// `Credits × expertise_multiplier × active_tier_multiplier` at
    /// cast time. Snapshot value; the aggregate recomputes on each
    /// downstream read per `MISSION.md` §3.4.
    pub weight: VoteWeight,
    /// When persist accepted the cast.
    pub recorded_at: DateTime<Utc>,
}

/// Receiver's ACK to a `DeferralRequest`. Returns the routed-set so
/// the consumer (CIRIS agent) knows who's being asked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferralRouting {
    /// Echo of the request's `deferral_id`.
    pub deferral_id: String,
    /// Federation identities selected per `MISSION.md` §3.3 (non-zero
    /// Expertise × Active tier × diversity policy × bounded count).
    pub routed_responders: Vec<ContributorId>,
    /// When persist accepted the request.
    pub accepted_at: DateTime<Utc>,
}

/// Receiver's ACK to a `DeferralResponse`. Confirms the response was
/// accepted into the per-deferral aggregate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferralResponseAck {
    /// Persisted response id.
    pub response_id: String,
    /// When persist accepted the response.
    pub accepted_at: DateTime<Utc>,
}

/// Receiver's ACK to an `ExpertiseAttestationPublish`. Echoes the
/// persisted contribution id; the standing-jump effect on the target's
/// Expertise ledger is observable via `engine.get_expertise_ledger`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertiseAttestationAck {
    /// Persisted contribution id (the attestation is recorded as a
    /// Contribution row per SCHEMA.md §3.1).
    pub contribution_id: String,
    /// When persist accepted the attestation.
    pub accepted_at: DateTime<Utc>,
    /// Whether this attestation triggered the cell's jump-threshold
    /// witness-set gate per `MISSION.md` §3.7. If `true`, the envelope's
    /// `witness_set` field was validated; if `false`, the attestation
    /// was below threshold and witness-free.
    pub jump_threshold_triggered: bool,
}

/// Receiver's ACK to a `ModerationEventPublish`. Carries the
/// moderation-event id and a placeholder for the eventual
/// SlashingAttestation cross-reference (filled when the quorum
/// adjudication completes downstream).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationEventAck {
    /// Persisted contribution id (the moderation event is recorded as
    /// a Contribution row per SCHEMA.md §3.1).
    pub contribution_id: String,
    /// When persist accepted the filing.
    pub accepted_at: DateTime<Utc>,
}

/// Receiver's ACK to a `SlashingAttestationPublish`. SlashingAttestation
/// is a standalone row class (not a Contribution); the persisted id is
/// distinct from the originating moderation event's id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingAttestationAck {
    /// Persisted attestation id (the row's identifier in the
    /// `slashing_attestations` table per CIRISPersist Appendix A.2 row 6).
    pub attestation_id: String,
    /// When persist accepted the attestation.
    pub accepted_at: DateTime<Utc>,
}

/// Receiver's ACK to a `ReconsiderationRequest`. Confirms the request
/// passed the recursion + time bounds and was accepted onto the audit
/// chain; the fresh-quorum adjudication outcome arrives later as a
/// separate `ReconsiderationAttestation` row (v0.1.0 cut+ work).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconsiderationRequestAck {
    /// Persisted contribution id.
    pub contribution_id: String,
    /// When persist accepted the request.
    pub accepted_at: DateTime<Utc>,
}

// ── Message impls (the wire contract) ────────────────────────────────────

impl Message for ContributionEnvelope {
    const TYPE: MessageType = MessageType::ContributionSubmit;
    const DELIVERY: Delivery = DURABLE_CONSENSUS;
    type Response = ContributionAck;
}

impl Message for Vote {
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

impl Message for ExpertiseAttestation {
    const TYPE: MessageType = MessageType::ExpertiseAttestationPublish;
    const DELIVERY: Delivery = DURABLE_CONSENSUS;
    type Response = ExpertiseAttestationAck;
}

impl Message for ModerationEvent {
    const TYPE: MessageType = MessageType::ModerationEventPublish;
    const DELIVERY: Delivery = DURABLE_CONSENSUS;
    type Response = ModerationEventAck;
}

impl Message for SlashingAttestation {
    const TYPE: MessageType = MessageType::SlashingAttestationPublish;
    const DELIVERY: Delivery = DURABLE_CONSENSUS;
    type Response = SlashingAttestationAck;
}

impl Message for ReconsiderationRequest {
    const TYPE: MessageType = MessageType::ReconsiderationRequest;
    const DELIVERY: Delivery = DURABLE_CONSENSUS;
    type Response = ReconsiderationRequestAck;
}
