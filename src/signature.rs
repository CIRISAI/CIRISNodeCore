//! HybridSignature — Ed25519 + ML-DSA-65 PQC.
//!
//! Per SCHEMA.md §2.4. Same shape `ciris-edge` already uses on the wire
//! (`CIRISEdge/src/messages/mod.rs` `EdgeEnvelope.signature` +
//! `signature_pqc`). The ML-DSA-65 half is `None` when the sender's
//! `federation_keys` row is hybrid-pending; consumer policy (per
//! `ciris_edge::HybridPolicy`) selects acceptance.
//!
//! Node-core never re-verifies inbound signatures (edge owns that).
//! This type exists for envelopes node-core *issues* — promotion
//! attestations, reconsideration outcomes, etc. — where signing
//! routes through `engine.steward_sign`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Hybrid Ed25519 + ML-DSA-65 signature envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HybridSignature {
    /// Base64url-encoded Ed25519 signature, no padding.
    pub ed25519: String,
    /// Base64url-encoded ML-DSA-65 PQC signature, no padding. `None` for
    /// hybrid-pending federation_keys rows; required when the signer's
    /// row has `pubkey_ml_dsa_65_base64` populated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ml_dsa_65: Option<String>,
    /// When the signature was produced. Used for replay-window arithmetic
    /// at edge; preserved here for audit-chain forensics.
    pub signed_at: DateTime<Utc>,
}
