//! [`NodeCore`] service struct + typed-handler scaffolding.
//!
//! Generic over `E: NodeCoreService` because persist's trait uses
//! RPITIT (`impl Future + Send`) which is not dyn-compatible — no
//! `Arc<dyn NodeCoreService>`. Each `NodeCore<E>` instance is bound
//! to a concrete substrate (persist's `Engine` in production; an
//! in-memory mock in tests).
//!
//! # Wiring shape
//!
//! ```ignore
//! use std::sync::Arc;
//! use ciris_edge::Edge;
//! use ciris_node_core::NodeCore;
//!
//! let engine: Arc<MyEngine> = /* impl NodeCoreService */;
//! let node = Arc::new(NodeCore::new(engine));
//! node.install_handlers(&edge).await?;
//! edge.run().await?;
//! ```
//!
//! # Two-layer surface
//!
//! - **Public methods on [`NodeCore`]** — testable surface. Take a
//!   verified body (persist envelope), call the matching
//!   `NodeCoreService` method, return the typed [`crate::wire`] Ack.
//! - **`Handler<M>` impls** for each wire message type — thin shells
//!   that unwrap the newtype, delegate to the matching public method,
//!   map errors.

use std::sync::Arc;

use chrono::Utc;
use ciris_edge::{Edge, EdgeError, Handler, HandlerContext, HandlerError};

use crate::substrate::{
    ContributionEnvelope, ModerationEvent, NodeCoreService, ReconsiderationRequest,
    SlashingAttestation, SubstrateError, VoteEnvelope,
};
use crate::wire::{
    self, ContributionAck, DeferralResponseAck, DeferralRouting, ExpertiseAttestationAck,
    ModerationEventAck, ReconsiderationRequestAck, SlashingAttestationAck, VoteAck,
};

/// Top-level service. Holds the substrate engine handle; exposes the
/// 8 wire-typed public methods (testable) + `install_handlers` for
/// edge dispatch.
pub struct NodeCore<E: NodeCoreService> {
    engine: Arc<E>,
}

impl<E: NodeCoreService> NodeCore<E> {
    /// Construct from any `NodeCoreService` impl.
    pub fn new(engine: Arc<E>) -> Self {
        Self { engine }
    }

    /// Borrow the substrate engine handle. Useful for direct
    /// `list_contributions` / ledger reads outside the wire path.
    pub fn engine(&self) -> &Arc<E> {
        &self.engine
    }

    // ── Public service methods (testable) ────────────────────────────────

    /// Submit a verified [`ContributionEnvelope`] to the federation
    /// audit chain. Discriminated downstream by `contribution_type`.
    pub async fn submit_contribution(
        &self,
        envelope: ContributionEnvelope,
    ) -> Result<ContributionAck, SubstrateError> {
        let id = envelope.contribution_id.clone();
        self.engine.put_contribution(envelope).await?;
        Ok(ContributionAck {
            contribution_id: id,
            accepted_at: Utc::now(),
        })
    }

    /// Record a verified [`VoteEnvelope`]. Returns the cast-time
    /// weight so the sender can display it without a second read.
    pub async fn record_vote(&self, envelope: VoteEnvelope) -> Result<VoteAck, SubstrateError> {
        let vote_id = envelope.vote_id.clone();
        let voter_id = envelope.voter_id.clone();
        let domain = envelope.cell.domain.clone();
        let language = envelope.cell.language.clone();
        let subject = envelope.cell.subject.clone().unwrap_or_default();
        self.engine.cast_vote(envelope).await?;
        let weight = self
            .engine
            .read_vote_weight(&voter_id, &domain, &language, &subject)
            .await?
            .unwrap_or_else(|| crate::substrate::VoteWeight {
                contributor_id: voter_id.clone(),
                domain,
                language,
                subject,
                credits: 0.0,
                expertise_multiplier: 0.0,
                active_tier_multiplier: 0.0,
                weight: 0.0,
            });
        Ok(VoteAck {
            vote_id,
            weight,
            recorded_at: Utc::now(),
        })
    }

    /// Submit a Deferral request (a `ContributionEnvelope` with
    /// `contribution_type = DeferralRequest`). Returns the routed-set
    /// per `MISSION.md` §3.3. Persists the request as a Contribution
    /// row alongside.
    pub async fn submit_deferral(
        &self,
        envelope: ContributionEnvelope,
    ) -> Result<DeferralRouting, SubstrateError> {
        let deferral_id = envelope.contribution_id.clone();
        let domain = envelope.subject.domain.clone();
        let language = envelope.subject.language.clone();

        // Decode the policy payload to honor routing_preferences.
        let payload: Option<crate::payloads::deferral::DeferralRequestPayload> =
            serde_json::from_value(envelope.payload.clone()).ok();
        let max = payload
            .as_ref()
            .and_then(|p| p.routing_preferences.as_ref())
            .and_then(|r| r.max_responders)
            .unwrap_or(9) as usize;

        self.engine.put_contribution(envelope).await?;
        let candidates = self.engine.routable_contributors(&domain, &language).await?;
        let routed_responders = candidates
            .into_iter()
            .take(max)
            .map(|c| c.contributor_id)
            .collect();

        Ok(DeferralRouting {
            deferral_id,
            routed_responders,
            accepted_at: Utc::now(),
        })
    }

    /// Record a Deferral response (a `ContributionEnvelope` with
    /// `contribution_type = DeferralResponse`).
    pub async fn record_deferral_response(
        &self,
        envelope: ContributionEnvelope,
    ) -> Result<DeferralResponseAck, SubstrateError> {
        let response_id = envelope.contribution_id.clone();
        self.engine.put_contribution(envelope).await?;
        Ok(DeferralResponseAck {
            response_id,
            accepted_at: Utc::now(),
        })
    }

    /// Publish an Expertise attestation (a `ContributionEnvelope`
    /// with `contribution_type = ExpertiseAttestation`). The engine
    /// enforces the jump-threshold witness-set gate per §3.5 / §3.7;
    /// we surface `jump_threshold_triggered = true` when the envelope
    /// carries a witness_set.
    pub async fn publish_expertise_attestation(
        &self,
        envelope: ContributionEnvelope,
    ) -> Result<ExpertiseAttestationAck, SubstrateError> {
        let contribution_id = envelope.contribution_id.clone();
        let jump_threshold_triggered = envelope.witness_set.is_some();
        self.engine.put_contribution(envelope).await?;
        Ok(ExpertiseAttestationAck {
            contribution_id,
            accepted_at: Utc::now(),
            jump_threshold_triggered,
        })
    }

    /// Publish a `ModerationEvent` (standalone row class, not a
    /// Contribution). Witness-set always required at the envelope
    /// level per §3.5; engine enforces.
    pub async fn publish_moderation_event(
        &self,
        envelope: ModerationEvent,
    ) -> Result<ModerationEventAck, SubstrateError> {
        let moderation_id = envelope.moderation_id.clone();
        self.engine.put_moderation_event(envelope).await?;
        Ok(ModerationEventAck {
            moderation_id,
            accepted_at: Utc::now(),
        })
    }

    /// Publish a `SlashingAttestation`. Engine validates the multi-sig
    /// quorum (recorded in payload) and applies the non-negative
    /// ledger floor per §10.
    pub async fn publish_slashing_attestation(
        &self,
        envelope: SlashingAttestation,
    ) -> Result<SlashingAttestationAck, SubstrateError> {
        let slashing_id = envelope.slashing_id.clone();
        self.engine.put_slashing_attestation(envelope).await?;
        Ok(SlashingAttestationAck {
            slashing_id,
            accepted_at: Utc::now(),
        })
    }

    /// Submit a `ReconsiderationRequest`. Engine enforces the
    /// recursion + time bounds per `MISSION.md` §3.9; bound
    /// violations surface as `SubstrateError::Conflict` or
    /// `NotAuthorized`.
    pub async fn submit_reconsideration_request(
        &self,
        envelope: ReconsiderationRequest,
    ) -> Result<ReconsiderationRequestAck, SubstrateError> {
        let request_id = envelope.request_id.clone();
        self.engine.put_reconsideration_request(envelope).await?;
        Ok(ReconsiderationRequestAck {
            request_id,
            accepted_at: Utc::now(),
        })
    }
}

impl<E: NodeCoreService + 'static> NodeCore<E> {
    /// Register all 8 v0.1.0-dev consensus handlers against an [`Edge`].
    pub async fn install_handlers(self: Arc<Self>, edge: &Edge) -> Result<(), EdgeError> {
        edge.register_handler::<wire::ContributionSubmit, _>(ContributionHandler(self.clone()))
            .await?;
        edge.register_handler::<wire::VoteCast, _>(VoteHandler(self.clone()))
            .await?;
        edge.register_handler::<wire::DeferralRequest, _>(DeferralRequestHandler(self.clone()))
            .await?;
        edge.register_handler::<wire::DeferralResponse, _>(DeferralResponseHandler(self.clone()))
            .await?;
        edge.register_handler::<wire::ExpertiseAttestationPublish, _>(
            ExpertiseAttestationHandler(self.clone()),
        )
        .await?;
        edge.register_handler::<wire::ModerationEventPublish, _>(ModerationEventHandler(
            self.clone(),
        ))
        .await?;
        edge.register_handler::<wire::SlashingAttestationPublish, _>(SlashingAttestationHandler(
            self.clone(),
        ))
        .await?;
        edge.register_handler::<wire::ReconsiderationRequest, _>(ReconsiderationRequestHandler(
            self.clone(),
        ))
        .await?;
        Ok(())
    }
}

// ── Handler shells ───────────────────────────────────────────────────────

fn map_substrate_err(e: SubstrateError) -> HandlerError {
    match e {
        SubstrateError::InvalidArgument(s) => HandlerError::SchemaInvalid(s),
        SubstrateError::NotAuthorized(s) => HandlerError::ApplicationRejected(s),
        SubstrateError::Signature(s) => HandlerError::SchemaInvalid(s),
        SubstrateError::Conflict(s) => HandlerError::ApplicationRejected(s),
        SubstrateError::NotFound(s) => HandlerError::ApplicationRejected(s),
        SubstrateError::Backend(s) => HandlerError::Persist(s),
        SubstrateError::NotImplemented(s) => HandlerError::Persist(format!("not implemented: {s}")),
        SubstrateError::Internal(s) => HandlerError::Persist(s),
        // Per persist v2.2.0+ (CIRISPersist#101): constitutional-asymmetry
        // write-admission enforced at the substrate (only HumanityAccord
        // may sign AccordCarrier-priority FederationAnnouncements). Bubble
        // up as ApplicationRejected — wire format admits the attempt;
        // policy rejects.
        SubstrateError::FederationAnnouncementAuthorityMismatch(s) => {
            HandlerError::ApplicationRejected(s)
        }
    }
}

macro_rules! impl_handler {
    ($Wrapper:ident, $Msg:ty, $Ack:ty, $method:ident) => {
        struct $Wrapper<E: NodeCoreService + 'static>(Arc<NodeCore<E>>);

        #[async_trait::async_trait]
        impl<E: NodeCoreService + 'static> Handler<$Msg> for $Wrapper<E> {
            async fn handle(
                &self,
                msg: $Msg,
                _ctx: HandlerContext,
            ) -> Result<$Ack, HandlerError> {
                self.0.$method(msg.0).await.map_err(map_substrate_err)
            }
        }
    };
}

impl_handler!(ContributionHandler, wire::ContributionSubmit, ContributionAck, submit_contribution);
impl_handler!(VoteHandler, wire::VoteCast, VoteAck, record_vote);
impl_handler!(DeferralRequestHandler, wire::DeferralRequest, DeferralRouting, submit_deferral);
impl_handler!(DeferralResponseHandler, wire::DeferralResponse, DeferralResponseAck, record_deferral_response);
impl_handler!(ExpertiseAttestationHandler, wire::ExpertiseAttestationPublish, ExpertiseAttestationAck, publish_expertise_attestation);
impl_handler!(ModerationEventHandler, wire::ModerationEventPublish, ModerationEventAck, publish_moderation_event);
impl_handler!(SlashingAttestationHandler, wire::SlashingAttestationPublish, SlashingAttestationAck, publish_slashing_attestation);
impl_handler!(ReconsiderationRequestHandler, wire::ReconsiderationRequest, ReconsiderationRequestAck, submit_reconsideration_request);
