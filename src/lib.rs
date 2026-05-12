//! `ciris-node-core` — second-tier consensus runtime for the CIRIS federation.
//!
//! Implements the eleven primitives from [`MISSION.md`] §2: Identity,
//! Commons Credits, Expertise, Vote, Contribution, Truth-Grounding,
//! Weighted Aggregate, Moderation, Slashing, Witness-Diversity,
//! Reconsideration. Folds into the agent post-PoB §3.1 (same trajectory
//! `ciris-lens-core` follows: spec → impl → pilot → folded).
//!
//! # Per-Contribution lifecycle
//!
//! ```text
//! Edge (verified ContributionEnvelope) ──► node-core::submit
//!         │                                       │
//!         │                                       ├── validate envelope + payload shape
//!         │                                       ├── check witness-set requirements (§3.5)
//!         │                                       ├── persist.put_contribution (Appendix A.2)
//!         │                                       └── append to pending audit chain (§13.2)
//!         ▼
//!     Vote, Vote, Vote (signed scores, weighted by Credits × Expertise × Active tier)
//!         │
//!         ▼
//!     Weighted Aggregate (P7) ──► threshold check (§9 policy)
//!         │
//!         ▼  (when threshold crossed)
//!     Promotion attestation signed by crate ──► PR opened against canonical artifact (§13.3)
//! ```
//!
//! # Substrate boundaries
//!
//! - **Verify is implicit.** CIRISEdge owns wire-side Ed25519 + ML-DSA-65
//!   verification before any byte reaches node-core. Node-core consumes
//!   `VerifiedEnvelope`-typed inputs and trusts the type attestation;
//!   it does NOT re-verify. Same discipline `ciris-lens-core` follows
//!   (`CIRISLensCore/FSD/CIRIS_LENS_CORE.md` §3.3).
//! - **Storage is implicit.** CIRISPersist owns the federation-consensus
//!   tables (Appendix A of `CIRIS_PERSIST.md`). Node-core holds an
//!   `Engine` handle, calls the typed-write methods named in Appendix
//!   A.2, never opens its own DB connection.
//! - **Signing is implicit.** Steward signing seeds never cross the FFI
//!   boundary into node-core. All node-core-issued attestations (promotion,
//!   moderation, slashing, reconsideration outcomes) sign via
//!   `engine.steward_sign(canonical_bytes)` per persist's existing
//!   v0.4.2 surface.
//! - **Canonicalization is implicit.** `engine.canonicalize_envelope_for_signing`
//!   only — node-core never re-implements canonicalization. CIRISPersist#7
//!   closure / AV-5 enforcement.
//!
//! # Mission alignment
//!
//! See `MISSION.md` at the repo root for the full eleven-primitive
//! spec and the Application × Contribution × Truth-Grounding mapping
//! table (§1.6).
//!
//! # Threat model
//!
//! Anti-Sybil resistance is a continuously-tuned policy posture, not
//! an emergent property of the primitives (`MISSION.md` §6.4). Hard
//! invariants the crate enforces:
//!
//! - **Witness-set gates on high-stakes Contributions** (§3.5) —
//!   moderation events, WA candidacy, policy proposals above magnitude
//!   threshold, expertise attestations that would jump standing.
//! - **Non-negative ledger invariants** (§10) — slashing reduces toward
//!   but never below zero; Credits and Expertise floors enforced at the
//!   typed-write boundary.
//! - **Gatekeeper-must-differ-from-author** for rubric promotion
//!   (`FSD/RUBRIC_CROWDSOURCING.md`).
//! - **Reconsideration bounds** — three triggers harassment review;
//!   180-day default time bound for NEW_EVIDENCE / PROCEDURAL_ERROR.
//!
//! # Status
//!
//! **v0.1.0-dev — skeleton.** This crate currently exposes the wire
//! types from [`SCHEMA.md`] and the trait surface that Appendix A of
//! CIRIS_PERSIST.md will implement. Behavior (validation, aggregation,
//! promotion-PR emission) lands as substrate dependencies materialize:
//!
//! 1. CIRISPersist v0.6.x or v0.7.0 — typed `put_contribution`,
//!    `cast_vote`, ledger writes per Appendix A.5.
//! 2. CIRISEdge with `MessageType` expansion (CIRISEdge#6) — typed
//!    handler registration for the 8 new federation-consensus wire types.
//! 3. Node-core v0.1.0 cut — full behavior wired against the above.
//!
//! [`MISSION.md`]: https://github.com/CIRISAI/CIRISNodeCore/blob/main/MISSION.md
//! [`SCHEMA.md`]: https://github.com/CIRISAI/CIRISNodeCore/blob/main/SCHEMA.md

#![deny(unsafe_code)]
#![deny(missing_docs)]

pub mod cell;
pub mod contribution;
pub mod engine;
pub mod error;
pub mod identity;
pub mod ledger;
pub mod payloads;
pub mod signature;
pub mod vote;
pub mod witness;

pub use cell::Cell;
pub use contribution::{ContributionEnvelope, ContributionType, SubjectKind};
pub use engine::NodeCoreEngine;
pub use error::{Error, Result};
pub use identity::ContributorId;
pub use signature::HybridSignature;
pub use vote::{Score, Vote};
pub use witness::WitnessSet;
