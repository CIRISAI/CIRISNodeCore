//! Crate-wide error type.
//!
//! Mission constraint: typed errors via `thiserror`. Every fallible
//! operation has a defined failure mode; no `.unwrap()` / `.expect()`
//! in non-test paths. Mirrors the discipline `ciris-persist` and
//! `ciris-lens-core` enforce at the `#[deny(missing_docs)]` boundary.

use thiserror::Error;

/// Crate-wide error type.
#[derive(Debug, Error)]
pub enum Error {
    /// Wire-format / schema validation failure.
    #[error("schema: {0}")]
    Schema(String),

    /// Signature verification failure on a Contribution / Vote / attestation.
    /// Note: edge owns the per-message verify; this variant surfaces only
    /// for envelopes node-core constructs internally (promotion attestations,
    /// reconsideration outcomes) where signing itself fails.
    #[error("signature: {0}")]
    Signature(String),

    /// Witness-set requirement violated for a high-stakes Contribution
    /// (SCHEMA.md §3.5 / WitnessSet §6).
    #[error("witness set: {0}")]
    WitnessSet(String),

    /// Ledger invariant violated (Credits or Expertise non-negative
    /// floor per `MISSION.md` §2.9 / SCHEMA.md §10).
    #[error("ledger invariant: {0}")]
    LedgerInvariant(String),

    /// Reconsideration bounds violated — recursion bound (3 triggers
    /// harassment review) or time bound (180-day default for
    /// NEW_EVIDENCE / PROCEDURAL_ERROR) per SCHEMA.md §9.
    #[error("reconsideration bounds: {0}")]
    ReconsiderationBounds(String),

    /// Substrate read/write failure (persist or edge boundary).
    #[error("substrate: {0}")]
    Substrate(String),

    /// JSON canonicalization round-trip failure. Should never fire in
    /// practice — persist owns canonicalization, AV-5 enforced.
    #[error("canonicalization: {0}")]
    Canonicalization(#[from] serde_json::Error),
}

/// Convenience result alias for crate-internal use.
pub type Result<T> = std::result::Result<T, Error>;
