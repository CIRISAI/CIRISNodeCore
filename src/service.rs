//! [`NodeCore`] service struct + typed-handler scaffolding for the
//! 4 federation-consensus message types whose payloads are pinned in
//! v0.1.0-dev (`ContributionSubmit`, `VoteCast`, `DeferralRequest`,
//! `DeferralResponse`).
//!
//! The 4 remaining MessageType variants from CIRISEdge#6
//! (`ExpertiseAttestationPublish`, `ModerationEventPublish`,
//! `SlashingAttestationPublish`, `ReconsiderationRequest`) wire up
//! alongside their typed payloads.
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
use crate::error::Error;
use crate::payloads::deferral::{DeferralRequest, DeferralResponse};
use crate::vote::Vote;
use crate::wire::{ContributionAck, DeferralResponseAck, DeferralRouting, VoteAck};

/// Top-level service holding the substrate engine handle. One per
/// host process. Register against an `Edge` instance via
/// [`NodeCore::install_handlers`].
pub struct NodeCore {
    engine: Arc<dyn NodeCoreEngine>,
}

impl NodeCore {
    /// Construct from a `NodeCoreEngine` implementation. In production
    /// this is `ciris-persist v0.6.x`'s concrete `Engine` (once the
    /// Appendix A typed methods materialize); in tests it's an
    /// in-memory mock.
    pub fn new(engine: Arc<dyn NodeCoreEngine>) -> Self {
        Self { engine }
    }

    /// Register all v0.1.0-dev consensus handlers against an [`Edge`].
    ///
    /// Returns once all 4 registrations succeed. Edge accepts no
    /// duplicate handlers per `MessageType` — calling this twice on
    /// the same `Edge` returns `EdgeError::DuplicateHandler` from the
    /// second call.
    pub async fn install_handlers(self: Arc<Self>, edge: &Edge) -> Result<(), EdgeError> {
        edge.register_handler::<ContributionEnvelope, _>(ContributionHandler(self.clone()))
            .await?;
        edge.register_handler::<Vote, _>(VoteHandler(self.clone()))
            .await?;
        edge.register_handler::<DeferralRequest, _>(DeferralRequestHandler(self.clone()))
            .await?;
        edge.register_handler::<DeferralResponse, _>(DeferralResponseHandler(self.clone()))
            .await?;
        Ok(())
    }
}

// ── Handler impls ────────────────────────────────────────────────────────
//
// Each handler:
// 1. Receives a verified body (edge owns verify).
// 2. Calls the matching `NodeCoreEngine` method (Appendix A.2 typed write).
// 3. Returns the typed Ack response (`wire::*Ack`).
//
// Witness-set + cell consistency validation lives in `NodeCoreEngine`
// implementations, not here — keeps the trait surface honest and lets
// in-memory mocks share the validation path.

struct ContributionHandler(Arc<NodeCore>);

#[async_trait::async_trait]
impl Handler<ContributionEnvelope> for ContributionHandler {
    async fn handle(
        &self,
        msg: ContributionEnvelope,
        _ctx: HandlerContext,
    ) -> Result<ContributionAck, HandlerError> {
        let contribution_id = self
            .0
            .engine
            .put_contribution(msg)
            .await
            .map_err(map_engine_err)?;
        Ok(ContributionAck {
            contribution_id,
            accepted_at: Utc::now(),
        })
    }
}

struct VoteHandler(Arc<NodeCore>);

#[async_trait::async_trait]
impl Handler<Vote> for VoteHandler {
    async fn handle(&self, msg: Vote, _ctx: HandlerContext) -> Result<VoteAck, HandlerError> {
        let voter_id = msg.voter_id.clone();
        let cell = msg.cell.clone();
        let vote_id = self.0.engine.cast_vote(msg).await.map_err(map_engine_err)?;
        let weight = self
            .0
            .engine
            .read_vote_weight(&voter_id, &cell)
            .await
            .map_err(map_engine_err)?;
        Ok(VoteAck {
            vote_id,
            weight,
            recorded_at: Utc::now(),
        })
    }
}

struct DeferralRequestHandler(Arc<NodeCore>);

#[async_trait::async_trait]
impl Handler<DeferralRequest> for DeferralRequestHandler {
    async fn handle(
        &self,
        msg: DeferralRequest,
        _ctx: HandlerContext,
    ) -> Result<DeferralRouting, HandlerError> {
        // Route per MISSION.md §3.3 step 1-2: Expertise-non-zero × Active tier.
        // Diversity preferences + bounded count (steps 3-4) apply when
        // `routable_contributors` returns more than the request's max.
        let candidates = self
            .0
            .engine
            .routable_contributors(&msg.cell.domain, &msg.cell.language)
            .await
            .map_err(map_engine_err)?;

        let max = msg
            .routing_preferences
            .as_ref()
            .and_then(|r| r.max_responders)
            .unwrap_or(9) as usize;
        let routed_responders = candidates.into_iter().take(max).collect();

        // Persist the request as a Contribution per §13.2 — the
        // envelope-shape conversion happens at the caller boundary;
        // for now we return the routing without persisting the
        // request itself (a v0.1.0 cut-time concern: whether the
        // Edge dispatch already persists the envelope, or whether
        // node-core has a separate `put_contribution` call here).
        // OQ-7 (forthcoming) — flag for FSD/SUBSTRATE_INTEGRATION.md.

        Ok(DeferralRouting {
            deferral_id: msg.deferral_id,
            routed_responders,
            accepted_at: Utc::now(),
        })
    }
}

// `Arc<NodeCore>` field is unused until the v0.1.0-cut TODO below
// wraps the response in a `ContributionEnvelope` and calls
// `engine.put_contribution`; suppress until then.
#[allow(dead_code)]
struct DeferralResponseHandler(Arc<NodeCore>);

#[async_trait::async_trait]
impl Handler<DeferralResponse> for DeferralResponseHandler {
    async fn handle(
        &self,
        msg: DeferralResponse,
        _ctx: HandlerContext,
    ) -> Result<DeferralResponseAck, HandlerError> {
        // Per SCHEMA.md §4.8 / MISSION.md §3.3: validate that
        // `responder_id` was in the routed set for `deferral_id`.
        // That validation lives in the engine impl (read the original
        // deferral routing record from the audit chain, check
        // membership). Skipped at the trait surface to keep this
        // handler simple.
        let response_id = msg.response_id.clone();
        // No dedicated `put_deferral_response`; routes through the
        // generic `put_contribution` path per Appendix A.2 row 1
        // (deferral_response is a Contribution subtype).
        //
        // TODO(v0.1.0-cut): wrap `msg` in a `ContributionEnvelope`
        // before calling `put_contribution`. The envelope construction
        // (signature, witness_set=None per §3.5, submitted_at) happens
        // at the call boundary, not here.
        Ok(DeferralResponseAck {
            response_id,
            accepted_at: Utc::now(),
        })
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

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
