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

// `ExpertiseAttestationPublish` / `ModerationEventPublish` /
// `SlashingAttestationPublish` / `ReconsiderationRequest` impls land
// alongside their typed payloads (`payloads/{expertise_attestation,
// moderation_event, slashing_attestation, reconsideration}.rs`),
// pending in `payloads/mod.rs`.
