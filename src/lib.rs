//! `ciris-node-core` — second-tier consensus runtime for the CIRIS federation.
//!
//! Implements the eleven primitives from [`MISSION.md`] §2 by composing
//! CIRISPersist's `NodeCoreService` substrate (v0.7.4) with CIRISEdge's
//! typed-handler dispatch (v0.1.2). The node-core crate hosts:
//!
//! - **Wire newtype wrappers** ([`wire`]) over persist's envelope
//!   shapes — exists for the orphan rule on `impl Message`. Serde-
//!   transparent so wire encoding matches persist's types byte-for-byte.
//! - **Typed policy payloads** ([`payloads`]) — schemas for the
//!   `payload: serde_json::Value` field on persist's envelopes.
//!   Encodes node-core's consensus-layer policy (allegation enums,
//!   reconsideration grounds, deferral routing preferences, etc.).
//! - **Service struct** ([`NodeCore`]) — generic over the substrate
//!   engine implementing [`NodeCoreService`]. Exposes 8 wire-typed
//!   public methods + `install_handlers` to register all 8 against
//!   an [`ciris_edge::Edge`].
//!
//! # Substrate boundaries
//!
//! - **Verify is implicit.** CIRISEdge owns wire-side hybrid signature
//!   verification before any byte reaches node-core. Node-core consumes
//!   `VerifiedEnvelope`-typed inputs and trusts the type attestation.
//! - **Storage is implicit.** Persist's `NodeCoreService` is the typed
//!   write + read surface; node-core never opens DB connections.
//! - **Signing is implicit.** Persist owns
//!   `cirisnode::verify::verify_envelope_signed` and the canonicalizer.
//!
//! # Per-Contribution lifecycle
//!
//! ```text
//! Edge (verified bytes) ──► wire::ContributionSubmit handler
//!                                  │
//!                                  ├── unwrap to persist::ContributionEnvelope
//!                                  ├── NodeCore::submit_contribution
//!                                  └── engine.put_contribution (Appendix A.2)
//!                                          │
//!                                          ▼
//!                                    cirisnode.contributions
//!                                    is_canonical=FALSE (pending)
//!                                          │
//!                  [ Vote, Vote, Vote — aggregated per Primitive 7 ]
//!                                          │
//!                                          ▼  threshold crossed
//!                          NodeCore signs PromotionAttestation
//!                          engine.put_promotion_attestation
//!                          (transactional: insert + flip
//!                           is_canonical=TRUE on target rows)
//! ```
//!
//! [`MISSION.md`]: https://github.com/CIRISAI/CIRISNodeCore/blob/main/MISSION.md
//! [`SCHEMA.md`]: https://github.com/CIRISAI/CIRISNodeCore/blob/main/SCHEMA.md

#![deny(unsafe_code)]
#![deny(missing_docs)]

pub mod aggregate;
pub mod payloads;
pub mod routing;
pub mod service;
pub mod sign;
pub mod substrate;
pub mod wire;

pub use service::NodeCore;
pub use substrate::{
    Cell, ContributionEnvelope, ContributionListPage, ContributionType, ContributionsFilter,
    CreditsLedgerEntry, CreditsUpdate, DiversityProof, ExpertiseLedgerEntry, ExpertiseUpdate,
    HybridSignature, ListCursor, ModerationEvent, NodeCoreService, PromotionAttestation,
    ReconsiderationAttestation, ReconsiderationRequest, RoutableContributor, SlashingAttestation,
    SubstrateError, TargetRowKind, VoteEnvelope, VoteListPage, VoteWeight, VotesFilter, Witness,
    WitnessSet,
};

/// Crate-level error alias — `ciris_node_core::Error` is persist's
/// [`SubstrateError`]. All trait methods return this type per
/// `NodeCoreService` v0.7.4.
pub type Error = SubstrateError;

/// Crate-level result alias.
pub type Result<T> = std::result::Result<T, Error>;
