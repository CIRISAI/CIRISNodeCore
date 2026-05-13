//! [`NodeCore`] service struct + typed-handler scaffolding.
//!
//! Two-layer shape:
//!
//! - **Public methods on [`NodeCore`]** (`submit_contribution`,
//!   `record_vote`, `submit_deferral`, `record_deferral_response`,
//!   `publish_expertise_attestation`, `publish_moderation_event`,
//!   `publish_slashing_attestation`, `submit_reconsideration_request`)
//!   are the testable surface. They take a verified body, talk to the
//!   [`NodeCoreEngine`] substrate, return the typed Ack.
//! - **`Handler<M>` impls** for each wire message type are thin
//!   shells that call the corresponding public method and map errors.
//!   Used by [`NodeCore::install_handlers`] to register against an
//!   [`Edge`] instance.
//!
//! # Wiring shape
//!
//! ```ignore
//! use std::sync::Arc;
//! use ciris_edge::Edge;
//! use ciris_node_core::{NodeCore, NodeCoreEngine};
//!
//! let engine: Arc<dyn NodeCoreEngine> = /* persist v0.6.x impl */;
//! let node = Arc::new(NodeCore::new(engine));
//! node.install_handlers(&edge).await?;
//! edge.run().await?;
//! ```

use std::sync::Arc;

use chrono::Utc;
use ciris_edge::{Edge, EdgeError, Handler, HandlerContext, HandlerError};

use crate::contribution::ContributionEnvelope;
use crate::engine::NodeCoreEngine;
use crate::error::{Error, Result};
use crate::payloads::deferral::{DeferralRequest, DeferralResponse};
use crate::payloads::expertise_attestation::ExpertiseAttestation;
use crate::payloads::moderation_event::ModerationEvent;
use crate::payloads::reconsideration::ReconsiderationRequest;
use crate::payloads::slashing_attestation::SlashingAttestation;
use crate::vote::Vote;
use crate::wire::{
    ContributionAck, DeferralResponseAck, DeferralRouting, ExpertiseAttestationAck,
    ModerationEventAck, ReconsiderationRequestAck, SlashingAttestationAck, VoteAck,
};

/// Top-level service holding the substrate engine handle. One per
/// host process. Public methods are the testable surface; the
/// `Handler<M>` impls delegate to them.
pub struct NodeCore {
    engine: Arc<dyn NodeCoreEngine>,
}

impl NodeCore {
    /// Construct from a `NodeCoreEngine` implementation. In production
    /// this is `ciris-persist v0.6.x`'s concrete `Engine` (once
    /// Appendix A.2 typed methods materialize); in tests it's an
    /// in-memory mock (`tests/support/MockEngine`).
    pub fn new(engine: Arc<dyn NodeCoreEngine>) -> Self {
        Self { engine }
    }

    /// Register all 8 v0.1.0-dev consensus handlers against an [`Edge`].
    pub async fn install_handlers(self: Arc<Self>, edge: &Edge) -> std::result::Result<(), EdgeError> {
        edge.register_handler::<ContributionEnvelope, _>(ContributionHandler(self.clone()))
            .await?;
        edge.register_handler::<Vote, _>(VoteHandler(self.clone()))
            .await?;
        edge.register_handler::<DeferralRequest, _>(DeferralRequestHandler(self.clone()))
            .await?;
        edge.register_handler::<DeferralResponse, _>(DeferralResponseHandler(self.clone()))
            .await?;
        edge.register_handler::<ExpertiseAttestation, _>(ExpertiseAttestationHandler(self.clone()))
            .await?;
        edge.register_handler::<ModerationEvent, _>(ModerationEventHandler(self.clone()))
            .await?;
        edge.register_handler::<SlashingAttestation, _>(SlashingAttestationHandler(self.clone()))
            .await?;
        edge.register_handler::<ReconsiderationRequest, _>(ReconsiderationRequestHandler(
            self.clone(),
        ))
        .await?;
        Ok(())
    }

    // ── Public service methods (testable) ────────────────────────────────

    /// Submit a verified [`ContributionEnvelope`] to the federation
    /// audit chain. Discriminated by `envelope.contribution_type`;
    /// witness-set + signature checks happen at the engine boundary.
    pub async fn submit_contribution(
        &self,
        envelope: ContributionEnvelope,
    ) -> Result<ContributionAck> {
        let contribution_id = self.engine.put_contribution(envelope).await?;
        Ok(ContributionAck {
            contribution_id,
            accepted_at: Utc::now(),
        })
    }

    /// Record a verified [`Vote`] on a Contribution. Returns the
    /// cast-time vote weight for sender display.
    pub async fn record_vote(&self, vote: Vote) -> Result<VoteAck> {
        let voter_id = vote.voter_id.clone();
        let cell = vote.cell.clone();
        let vote_id = self.engine.cast_vote(vote).await?;
        let weight = self.engine.read_vote_weight(&voter_id, &cell).await?;
        Ok(VoteAck {
            vote_id,
            weight,
            recorded_at: Utc::now(),
        })
    }

    /// Submit a [`DeferralRequest`]. Returns the routed-set per
    /// `MISSION.md` §3.3 (Expertise-non-zero × Active tier ×
    /// diversity preferences × bounded count).
    pub async fn submit_deferral(&self, req: DeferralRequest) -> Result<DeferralRouting> {
        let candidates = self
            .engine
            .routable_contributors(&req.cell.domain, &req.cell.language)
            .await?;

        let max = req
            .routing_preferences
            .as_ref()
            .and_then(|r| r.max_responders)
            .unwrap_or(9) as usize;
        let routed_responders = candidates.into_iter().take(max).collect();

        Ok(DeferralRouting {
            deferral_id: req.deferral_id,
            routed_responders,
            accepted_at: Utc::now(),
        })
    }

    /// Record a [`DeferralResponse`] for an open deferral. Engine
    /// enforces `responder_id` is in the routed set per §3.3.
    pub async fn record_deferral_response(
        &self,
        resp: DeferralResponse,
    ) -> Result<DeferralResponseAck> {
        let response_id = resp.response_id.clone();
        // TODO(v0.1.0-cut): wrap `resp` in a `ContributionEnvelope`
        // (signature via `engine.steward_sign`, witness_set=None per
        // §3.5, submitted_at=Utc::now()) before calling
        // `put_contribution`. The skeleton omits envelope construction
        // pending the steward-signing scaffolding.
        let _ = &self.engine;
        Ok(DeferralResponseAck {
            response_id,
            accepted_at: Utc::now(),
        })
    }

    /// Publish an [`ExpertiseAttestation`]. The engine enforces the
    /// jump-threshold witness-set gate per `MISSION.md` §3.5; the Ack
    /// surfaces whether the gate fired so the caller can show the
    /// attester whether their attestation was high-stakes.
    pub async fn publish_expertise_attestation(
        &self,
        att: ExpertiseAttestation,
    ) -> Result<ExpertiseAttestationAck> {
        // For v0.1.0-dev the handler routes through the generic
        // `put_contribution` (after envelope construction lands).
        // The jump-threshold determination is the engine's
        // responsibility; we surface a stub `false` here pending that
        // engine surface.
        let _ = att;
        let _ = &self.engine;
        Ok(ExpertiseAttestationAck {
            contribution_id: "expatt_PENDING_v010_cut".into(),
            accepted_at: Utc::now(),
            jump_threshold_triggered: false,
        })
    }

    /// Publish a [`ModerationEvent`]. Witness-set always required at
    /// the envelope level per §3.5; engine enforces.
    pub async fn publish_moderation_event(
        &self,
        ev: ModerationEvent,
    ) -> Result<ModerationEventAck> {
        // TODO(v0.1.0-cut): wrap in envelope (witness_set required;
        // engine enforces). For now, route through the dedicated
        // put_moderation_event surface (Appendix A.2 row 5).
        let cell = crate::cell::Cell::expertise("", ""); // pending: derive from target row's cell
        self.engine
            .update_credits_ledger(&ev.accuser_id, &cell, 0.0)
            .await
            .ok(); // no-op write probe; replaced by put_moderation_event when wired
        Ok(ModerationEventAck {
            contribution_id: "modev_PENDING_v010_cut".into(),
            accepted_at: Utc::now(),
        })
    }

    /// Publish a [`SlashingAttestation`] (standalone row class — not a
    /// Contribution). Engine validates multi-sig quorum + applies the
    /// non-negative ledger floor per §10.
    pub async fn publish_slashing_attestation(
        &self,
        att: SlashingAttestation,
    ) -> Result<SlashingAttestationAck> {
        // TODO(v0.1.0-cut): route through `engine.put_slashing_attestation`
        // (Appendix A.2 row 6). Stub return until that engine surface
        // materializes.
        let attestation_id = att.attestation_id.clone();
        let _ = &self.engine;
        Ok(SlashingAttestationAck {
            attestation_id,
            accepted_at: Utc::now(),
        })
    }

    /// Submit a [`ReconsiderationRequest`]. Engine enforces the
    /// recursion + time bounds per `MISSION.md` §3.9; violations
    /// surface as [`Error::ReconsiderationBounds`].
    pub async fn submit_reconsideration_request(
        &self,
        req: ReconsiderationRequest,
    ) -> Result<ReconsiderationRequestAck> {
        // TODO(v0.1.0-cut): route through `engine.put_reconsideration_request`
        // (Appendix A.2 row 7). Stub return until that engine surface
        // materializes.
        let _ = req;
        let _ = &self.engine;
        Ok(ReconsiderationRequestAck {
            contribution_id: "recon_PENDING_v010_cut".into(),
            accepted_at: Utc::now(),
        })
    }
}

// ── Handler impls (thin shells over the pub methods above) ───────────────

fn map_engine_err(e: Error) -> HandlerError {
    match e {
        Error::Schema(s) => HandlerError::SchemaInvalid(s),
        Error::Signature(s) => HandlerError::SchemaInvalid(s),
        Error::WitnessSet(s) => HandlerError::ApplicationRejected(s),
        Error::LedgerInvariant(s) => HandlerError::ApplicationRejected(s),
        Error::ReconsiderationBounds(s) => HandlerError::ApplicationRejected(s),
        Error::Substrate(s) => HandlerError::Persist(s),
        Error::Canonicalization(e) => HandlerError::SchemaInvalid(format!("canonicalize: {e}")),
    }
}

macro_rules! impl_handler {
    ($Wrapper:ident, $Msg:ty, $Ack:ty, $method:ident) => {
        struct $Wrapper(Arc<NodeCore>);

        #[async_trait::async_trait]
        impl Handler<$Msg> for $Wrapper {
            async fn handle(
                &self,
                msg: $Msg,
                _ctx: HandlerContext,
            ) -> std::result::Result<$Ack, HandlerError> {
                self.0.$method(msg).await.map_err(map_engine_err)
            }
        }
    };
}

impl_handler!(ContributionHandler, ContributionEnvelope, ContributionAck, submit_contribution);
impl_handler!(VoteHandler, Vote, VoteAck, record_vote);
impl_handler!(DeferralRequestHandler, DeferralRequest, DeferralRouting, submit_deferral);
impl_handler!(DeferralResponseHandler, DeferralResponse, DeferralResponseAck, record_deferral_response);
impl_handler!(ExpertiseAttestationHandler, ExpertiseAttestation, ExpertiseAttestationAck, publish_expertise_attestation);
impl_handler!(ModerationEventHandler, ModerationEvent, ModerationEventAck, publish_moderation_event);
impl_handler!(SlashingAttestationHandler, SlashingAttestation, SlashingAttestationAck, publish_slashing_attestation);
impl_handler!(ReconsiderationRequestHandler, ReconsiderationRequest, ReconsiderationRequestAck, submit_reconsideration_request);
