//! Envelope-signing scaffolding.
//!
//! Builders that canonicalize an envelope, hand the bytes to a
//! consumer-supplied [`EnvelopeSigner`], and return the signed
//! envelope ready for `engine.put_*`.
//!
//! Per SCHEMA.md §2.2 + persist v0.7.1's verify model: a
//! `ContributorId` IS the Ed25519 pubkey (standard base64). Envelopes
//! are self-signed against the identity embedded in them.
//! Node-core does NOT manage contributor private keys — production
//! consumers hold keys in TPM / HSM / OS keystore; tests use the
//! [`Ed25519Signer`] helper.
//!
//! Canonicalization routes through persist's
//! `cirisnode::verify::canonical_bytes_for_envelope` — node-core never
//! re-implements the canonicalizer (CIRISPersist#7 / AV-5).

use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use chrono::Utc;
use ed25519_dalek::{Signer as DalekSigner, SigningKey};
use serde::Serialize;

use ciris_persist::cirisnode::verify::canonical_bytes_for_envelope;
use ciris_persist::signing::LocalSigner;

use crate::substrate::{
    Cell, ContributionEnvelope, ContributionType, HybridSignature, ModerationEvent,
    PromotionAttestation, ReconsiderationAttestation, ReconsiderationRequest, SlashingAttestation,
    SubstrateError, TargetRowKind, VoteEnvelope, WitnessSet,
};

/// Trait for signing canonical envelope bytes. Implementations live
/// outside node-core (consumer-supplied — TPM-backed in production,
/// software key in tests).
pub trait EnvelopeSigner: Send + Sync {
    /// Sign the canonical bytes; return the hybrid signature shape
    /// persist's verify path expects. `signed_at` should be the
    /// current wall-clock at sign time.
    fn sign_bytes(&self, canonical_bytes: &[u8]) -> Result<HybridSignature, SubstrateError>;
}

/// Software Ed25519 signer for tests + reference. NOT recommended for
/// production: holds the seed in plaintext in process memory. Real
/// deployments use TPM / HSM / OS keystore via a custom
/// [`EnvelopeSigner`] impl.
pub struct Ed25519Signer {
    key: SigningKey,
}

impl Ed25519Signer {
    /// Construct from a 32-byte seed.
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            key: SigningKey::from_bytes(&seed),
        }
    }

    /// The contributor identity matching this signer — base64 (standard)
    /// of the Ed25519 verifying key. Use as `author_id` / `voter_id` /
    /// etc. on envelopes signed by this instance.
    pub fn contributor_id(&self) -> String {
        BASE64.encode(self.key.verifying_key().to_bytes())
    }
}

impl EnvelopeSigner for Ed25519Signer {
    fn sign_bytes(&self, canonical_bytes: &[u8]) -> Result<HybridSignature, SubstrateError> {
        let sig = self.key.sign(canonical_bytes);
        Ok(HybridSignature {
            ed25519: BASE64.encode(sig.to_bytes()),
            ml_dsa_65: None,
            signed_at: Utc::now(),
        })
    }
}

/// Adapter making persist's [`LocalSigner`] usable as an
/// [`EnvelopeSigner`] for node-core's wire builders.
///
/// This is the **production** signing path for node-core ingest +
/// emit flows. The host signs every envelope with its OWN federation
/// identity (the local_signer the agent process bootstrapped via
/// [`ciris_persist::ffi::pyo3::PyEngine::local_signer_capsule`],
/// CIRISPersist#119). Node-core never accepts a "sign as someone
/// else" surface — claims about third parties (e.g. "X submitted
/// this", "Reuters published this article") are encoded as the
/// host's own observation/witness attestations in the payload, not
/// by spoofing the envelope identity. The accountability chain
/// stays one-signer-per-envelope, the Recursive Golden Rule
/// (MISSION §1.5) intact.
///
/// Currently classical-only: [`EnvelopeSigner::sign_bytes`] is sync
/// and `LocalSigner::sign_ml_dsa_65` is async, so the PQC half of
/// the hybrid signature is left empty here. PQC fill-in happens
/// cold-path via the same backend hook persist uses for
/// `put_attestation` (CIRISPersist#10), so a node-core-signed
/// artifact still reaches PQC-complete state after persistence —
/// just not synchronously at sign time.
pub struct LocalSignerAdapter {
    inner: Arc<LocalSigner>,
}

impl LocalSignerAdapter {
    /// Wrap an `Arc<LocalSigner>` extracted from persist's
    /// `local_signer_capsule`. Cheap: `Arc` clone only — the signer
    /// keeps living in persist; we hold a reference for sign-time.
    pub fn new(inner: Arc<LocalSigner>) -> Self {
        Self { inner }
    }

    /// The host's `ContributorId` — base64 (standard) Ed25519 pubkey.
    /// Use as `author_id` / `voter_id` / `attesting_key_id` on every
    /// envelope node-core builds + signs through this adapter.
    pub fn contributor_id(&self) -> String {
        self.inner.public_key_b64()
    }

    /// The host's `key_id` label (the human-readable name persist's
    /// local-identity bootstrap assigned). Distinct from
    /// [`Self::contributor_id`] — `key_id` is the directory lookup
    /// handle, `contributor_id` is the cryptographic identity.
    pub fn key_id(&self) -> &str {
        self.inner.key_id()
    }

    /// Borrow the inner `Arc<LocalSigner>` for callers that need it
    /// directly (e.g. PQC-async sign paths that can't go through the
    /// sync [`EnvelopeSigner`] trait, or cold-path fill-in jobs).
    pub fn inner(&self) -> &Arc<LocalSigner> {
        &self.inner
    }
}

impl EnvelopeSigner for LocalSignerAdapter {
    fn sign_bytes(&self, canonical_bytes: &[u8]) -> Result<HybridSignature, SubstrateError> {
        let sig = self
            .inner
            .sign_ed25519(canonical_bytes)
            .map_err(|e| SubstrateError::Signature(format!("local_signer.sign_ed25519: {e}")))?;
        Ok(HybridSignature {
            ed25519: BASE64.encode(sig),
            ml_dsa_65: None,
            signed_at: Utc::now(),
        })
    }
}

// ── Generic sign helper ──────────────────────────────────────────────────

/// Canonicalize an envelope (with a placeholder signature) and produce
/// the signature over those bytes. Used by every per-type builder
/// below; exposed for callers building envelope shapes not yet covered
/// by a dedicated builder.
pub fn sign_canonical<T: Serialize, S: EnvelopeSigner>(
    envelope_with_placeholder_sig: &T,
    signer: &S,
) -> Result<HybridSignature, SubstrateError> {
    let bytes = canonical_bytes_for_envelope(envelope_with_placeholder_sig)?;
    signer.sign_bytes(&bytes)
}

fn empty_signature() -> HybridSignature {
    HybridSignature {
        ed25519: String::new(),
        ml_dsa_65: None,
        signed_at: Utc::now(),
    }
}

// ── Per-type builders ────────────────────────────────────────────────────

/// Build + sign a `ContributionEnvelope`. Caller provides everything
/// except the signature.
pub fn build_contribution<S: EnvelopeSigner>(
    contribution_id: String,
    contribution_type: ContributionType,
    author_id: String,
    cell: Cell,
    payload: serde_json::Value,
    witness_set: Option<WitnessSet>,
    signer: &S,
) -> Result<ContributionEnvelope, SubstrateError> {
    let mut env = ContributionEnvelope {
        contribution_id,
        contribution_type,
        author_id,
        subject: cell,
        payload,
        witness_set,
        signature: empty_signature(),
        submitted_at: Utc::now(),
    };
    env.signature = sign_canonical(&env, signer)?;
    Ok(env)
}

/// Build + sign a `VoteEnvelope`.
pub fn build_vote<S: EnvelopeSigner>(
    vote_id: String,
    voter_id: String,
    contribution_id: Option<String>,
    cell: Cell,
    score: serde_json::Value,
    rationale: Option<String>,
    signer: &S,
) -> Result<VoteEnvelope, SubstrateError> {
    let mut env = VoteEnvelope {
        vote_id,
        voter_id,
        contribution_id,
        cell,
        score,
        rationale,
        signature: empty_signature(),
        cast_at: Utc::now(),
    };
    env.signature = sign_canonical(&env, signer)?;
    Ok(env)
}

/// Build + sign a `ModerationEvent`.
pub fn build_moderation_event<S: EnvelopeSigner>(
    moderation_id: String,
    target_contributor: String,
    accuser_id: String,
    payload: serde_json::Value,
    signer: &S,
) -> Result<ModerationEvent, SubstrateError> {
    let mut env = ModerationEvent {
        moderation_id,
        target_contributor,
        accuser_id,
        payload,
        filed_at: Utc::now(),
        signature: empty_signature(),
    };
    env.signature = sign_canonical(&env, signer)?;
    Ok(env)
}

/// Build + sign a `SlashingAttestation`. The publisher's single-sig is
/// applied here; the multi-sig quorum signatures (one per quorum
/// member) live in `payload` per [`crate::payloads::slashing_attestation`].
pub fn build_slashing_attestation<S: EnvelopeSigner>(
    slashing_id: String,
    moderation_id: String,
    adjudicator_id: String,
    payload: serde_json::Value,
    signer: &S,
) -> Result<SlashingAttestation, SubstrateError> {
    let mut env = SlashingAttestation {
        slashing_id,
        moderation_id,
        adjudicator_id,
        payload,
        attested_at: Utc::now(),
        signature: empty_signature(),
    };
    env.signature = sign_canonical(&env, signer)?;
    Ok(env)
}

/// Build + sign a `ReconsiderationRequest`.
pub fn build_reconsideration_request<S: EnvelopeSigner>(
    request_id: String,
    slashing_id: String,
    requester_id: String,
    payload: serde_json::Value,
    signer: &S,
) -> Result<ReconsiderationRequest, SubstrateError> {
    let mut env = ReconsiderationRequest {
        request_id,
        slashing_id,
        requester_id,
        payload,
        requested_at: Utc::now(),
        signature: empty_signature(),
    };
    env.signature = sign_canonical(&env, signer)?;
    Ok(env)
}

/// Build + sign a `ReconsiderationAttestation`.
pub fn build_reconsideration_attestation<S: EnvelopeSigner>(
    reconsideration_id: String,
    request_id: String,
    adjudicator_id: String,
    payload: serde_json::Value,
    signer: &S,
) -> Result<ReconsiderationAttestation, SubstrateError> {
    let mut env = ReconsiderationAttestation {
        reconsideration_id,
        request_id,
        adjudicator_id,
        payload,
        attested_at: Utc::now(),
        signature: empty_signature(),
    };
    env.signature = sign_canonical(&env, signer)?;
    Ok(env)
}

/// Build + sign a `PromotionAttestation`. The `attested_by` field is
/// the signer's contributor id — typically the consensus crate's
/// federation identity, NOT a single human/WA.
pub fn build_promotion_attestation<S: EnvelopeSigner>(
    attestation_id: String,
    target_kind: TargetRowKind,
    target_ids: Vec<String>,
    attested_by: String,
    aggregate_evidence: serde_json::Value,
    signer: &S,
) -> Result<PromotionAttestation, SubstrateError> {
    let mut env = PromotionAttestation {
        attestation_id,
        target_kind,
        target_ids,
        attested_by,
        aggregate_evidence,
        signature: empty_signature(),
        attested_at: Utc::now(),
    };
    env.signature = sign_canonical(&env, signer)?;
    Ok(env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ciris_persist::cirisnode::verify::verify_envelope_signed;

    fn test_signer() -> Ed25519Signer {
        Ed25519Signer::from_seed([0xAB; 32])
    }

    #[test]
    fn ed25519_signer_pubkey_matches_verify_path() {
        let s = test_signer();
        let pubkey = s.contributor_id();
        // Same seed → same pubkey across constructions.
        let s2 = Ed25519Signer::from_seed([0xAB; 32]);
        assert_eq!(s2.contributor_id(), pubkey);
    }

    #[test]
    fn contribution_envelope_signs_and_verifies() {
        let signer = test_signer();
        let env = build_contribution(
            "01HX0000000000000000000001".into(),
            ContributionType::DeferralRequest,
            signer.contributor_id(),
            Cell {
                domain: "mental_health".into(),
                language: "am".into(),
                subject: None,
            },
            serde_json::json!({"title": "test", "context": "test", "response_format": "binary"}),
            None,
            &signer,
        )
        .unwrap();

        // Persist's verify path accepts this — proves canonicalization
        // matches and the signature shape is correct.
        verify_envelope_signed(&env, &env.signature, &env.author_id).expect("verify passes");
    }

    #[test]
    fn tampered_envelope_fails_verify() {
        let signer = test_signer();
        let mut env = build_contribution(
            "01HX0000000000000000000002".into(),
            ContributionType::Proposal,
            signer.contributor_id(),
            Cell {
                domain: "mental_health".into(),
                language: "am".into(),
                subject: Some("arc_question".into()),
            },
            serde_json::json!({"original": "payload"}),
            None,
            &signer,
        )
        .unwrap();

        // Tamper: mutate the payload after signing.
        env.payload = serde_json::json!({"tampered": "payload"});
        let result = verify_envelope_signed(&env, &env.signature, &env.author_id);
        assert!(result.is_err(), "tampered envelope must fail verify");
    }

    #[test]
    fn vote_envelope_signs_and_verifies() {
        let signer = test_signer();
        let env = build_vote(
            "01HXVOTE000000000000000000".into(),
            signer.contributor_id(),
            Some("01HXC0000000000000000000C".into()),
            Cell {
                domain: "mental_health".into(),
                language: "am".into(),
                subject: Some("arc_question".into()),
            },
            serde_json::json!({"verdict": "approve", "magnitude": 1.0}),
            Some("LGTM".into()),
            &signer,
        )
        .unwrap();
        verify_envelope_signed(&env, &env.signature, &env.voter_id).expect("verify passes");
    }

    #[test]
    fn local_signer_adapter_signs_and_verifies() {
        // Construct a persist `LocalSigner` directly from primitives —
        // mirrors how production wires it via persist's seed-file
        // loader, without needing a real seed on disk.
        use ed25519_dalek::SigningKey;
        let seed = [0x5Cu8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let local_signer = ciris_persist::signing::LocalSigner::from_parts(
            signing_key,
            "test-local-key".to_string(),
            None,
            None,
        );
        let adapter = LocalSignerAdapter::new(std::sync::Arc::new(local_signer));

        // contributor_id must match what build_contribution stamps as
        // author_id — base64(verifying_key) — for verify to accept.
        let env = build_contribution(
            "01HXLOCAL00000000000000001".into(),
            ContributionType::Proposal,
            adapter.contributor_id(),
            Cell {
                domain: "mental_health".into(),
                language: "am".into(),
                subject: Some("arc_question".into()),
            },
            serde_json::json!({"proposal_id": "P-1", "title": "via local_signer"}),
            None,
            &adapter,
        )
        .unwrap();

        verify_envelope_signed(&env, &env.signature, &env.author_id)
            .expect("local_signer-signed envelope verifies");
    }

    #[test]
    fn local_signer_adapter_key_id_vs_contributor_id_are_distinct() {
        use ed25519_dalek::SigningKey;
        let signing_key = SigningKey::from_bytes(&[0x11u8; 32]);
        let local_signer = ciris_persist::signing::LocalSigner::from_parts(
            signing_key,
            "human-readable-label".to_string(),
            None,
            None,
        );
        let adapter = LocalSignerAdapter::new(std::sync::Arc::new(local_signer));

        // key_id is the directory lookup handle (a label); contributor_id
        // is base64 of the pubkey (44 chars). These are NOT the same and
        // must not be confused at call sites.
        assert_eq!(adapter.key_id(), "human-readable-label");
        assert_eq!(adapter.contributor_id().len(), 44);
        assert_ne!(adapter.key_id(), adapter.contributor_id());
    }

    #[test]
    fn promotion_attestation_signs_and_verifies() {
        let signer = test_signer();
        let env = build_promotion_attestation(
            "01HXPROMOATT00000000000000".into(),
            TargetRowKind::Contribution,
            vec!["01HXTARGET0000000000000001".into()],
            signer.contributor_id(),
            serde_json::json!({"vote_tally": {"approve": 12, "reject": 2}}),
            &signer,
        )
        .unwrap();
        verify_envelope_signed(&env, &env.signature, &env.attested_by).expect("verify passes");
    }
}
