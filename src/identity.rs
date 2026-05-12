//! Federation identity newtype.
//!
//! Per `MISSION.md` Primitive 1: Identity is substrate inheritance,
//! not a CIRISNodeCore primitive. The substrate (CIRISRegistry +
//! CIRISVerify + CIRISPersist's federation_keys directory) produces
//! identity; node-core consumes it and indexes all consensus state by it.

use serde::{Deserialize, Serialize};

/// Federation identity. Base64url-encoded Ed25519 public key, no padding,
/// per SCHEMA.md §2.2.
///
/// Carried in every Contribution `author_id`, Vote `voter_id`, etc.
/// MUST resolve via `engine.lookup_public_key(...)` (persist's federation
/// directory) before any envelope is accepted.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContributorId(pub String);

impl ContributorId {
    /// Construct from a base64url string. No validation here — wire-side
    /// validity is edge's job (signature verification will fail before
    /// node-core sees a malformed key).
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Inner string view.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ContributorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
