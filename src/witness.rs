//! WitnessSet — diversity-bounded co-signer set for high-stakes Contributions.
//!
//! Per `MISSION.md` Primitive 10 / SCHEMA.md §3.5 / §6. Required for:
//!
//! - `moderation_event` (always)
//! - `wa_candidacy` (always)
//! - Policy proposals above magnitude threshold
//! - `expertise_attestation` whose acceptance would jump the target's
//!   Expertise standing past a threshold
//!
//! NOT required for routine Contributions (battery evaluation, vote,
//! deferral request/response, ExpertiseAttestation below jump-threshold).

use serde::{Deserialize, Serialize};

use crate::identity::ContributorId;
use crate::signature::HybridSignature;

/// Witness attestation — one signer's co-signature on the witnessed
/// Contribution. Per SCHEMA.md §6.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessAttestation {
    /// The witness's federation identity.
    pub witness_id: ContributorId,
    /// Witness's signature over the canonical Contribution bytes.
    pub signature: HybridSignature,
}

/// Witness set carried in a high-stakes Contribution envelope. Per
/// SCHEMA.md §6.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessSet {
    /// Co-signing witnesses. Diversity requirements (jurisdictional,
    /// organizational) per `MISSION.md` Primitive 10 / §3.5 are policy
    /// parameters checked at validation time, not enforced by the type.
    pub attestations: Vec<WitnessAttestation>,
}
