//! Integration tests for the Phase 2B `external_content` ingest I/O
//! layer — `ingest_*` functions in `crate::ingest`.
//!
//! Covers the full sequence end-to-end:
//!   1. `put_blob_signing` lands the bytes + holds_bytes attestation
//!   2. Contribution envelope is built with `author_id` = host signer's
//!      `contributor_id()` (no proxy-signing)
//!   3. `NodeCoreService::put_contribution` persists the envelope

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use ciris_persist::federation::{BlobBody, BlobError, BlobStorage, PutBlobAttestation};
use ciris_persist::signing::LocalSigner;
use ed25519_dalek::SigningKey;

use ciris_node_core::ingest::{ingest_encyclopedia_article, IngestContext};
use ciris_node_core::ingest::{Citation, CitationKind, EncyclopediaArticleSource};
use ciris_node_core::ingest::{TopicalRelation, TopicalRelationKind};
use ciris_node_core::sign::LocalSignerAdapter;
use ciris_node_core::substrate::Cell;

mod support;
use support::MockEngine;

/// In-memory BlobStorage for the integration test. Records every
/// `put_blob` call (route used by `put_blob_signing`'s default impl)
/// so the test can assert against the holder attestation persist
/// constructed.
#[derive(Default)]
struct MockBlobStorage {
    state: Arc<Mutex<MockBlobState>>,
}

#[derive(Default)]
struct MockBlobState {
    blobs: HashMap<[u8; 32], (BlobBody, Option<String>)>,
    attestations: Vec<PutBlobAttestation>,
}

impl MockBlobStorage {
    fn new() -> Self {
        Self::default()
    }
    fn blob_count(&self) -> usize {
        self.state.lock().unwrap().blobs.len()
    }
    fn attestation_count(&self) -> usize {
        self.state.lock().unwrap().attestations.len()
    }
    fn last_attestation(&self) -> PutBlobAttestation {
        self.state.lock().unwrap().attestations.last().unwrap().clone()
    }
}

impl BlobStorage for MockBlobStorage {
    fn inline_bytes_cap(&self) -> usize {
        16 * 1024 * 1024 // 16 MiB, same as edge MAX_BODY_BYTES default
    }

    fn put_blob(
        &self,
        sha256: &[u8; 32],
        body: BlobBody,
        media_type: Option<&str>,
        attestation: PutBlobAttestation,
    ) -> impl std::future::Future<Output = Result<(), BlobError>> + Send {
        let state = self.state.clone();
        let sha = *sha256;
        let media = media_type.map(|s| s.to_string());
        async move {
            let mut st = state.lock().unwrap();
            st.blobs.insert(sha, (body, media));
            st.attestations.push(attestation);
            Ok(())
        }
    }

    fn get_blob(
        &self,
        sha256: &[u8; 32],
    ) -> impl std::future::Future<Output = Result<Option<BlobBody>, BlobError>> + Send {
        let state = self.state.clone();
        let sha = *sha256;
        async move {
            Ok(state.lock().unwrap().blobs.get(&sha).map(|(b, _)| b.clone()))
        }
    }

    fn has_blob(
        &self,
        sha256: &[u8; 32],
    ) -> impl std::future::Future<Output = Result<bool, BlobError>> + Send {
        let state = self.state.clone();
        let sha = *sha256;
        async move { Ok(state.lock().unwrap().blobs.contains_key(&sha)) }
    }

    fn list_holders(
        &self,
        _sha256: &[u8; 32],
    ) -> impl std::future::Future<Output = Result<Vec<String>, BlobError>> + Send {
        async move { Ok(vec![]) }
    }

    fn list_local_holders(
        &self,
        _sha256: &[u8; 32],
    ) -> impl std::future::Future<Output = Result<Vec<String>, BlobError>> + Send {
        async move { Ok(vec![]) }
    }

    fn list_held_by(
        &self,
        _attesting_key_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<[u8; 32]>, BlobError>> + Send {
        async move { Ok(vec![]) }
    }

    fn evict_actor<'s>(
        &'s self,
        _attesting_key_id: &'s str,
        _signer: &'s dyn ciris_keyring::HardwareSigner,
        _now: chrono::DateTime<chrono::Utc>,
    ) -> impl std::future::Future<
        Output = Result<ciris_persist::federation::EvictActorReport, BlobError>,
    > + Send
           + 's {
        async move { Ok(ciris_persist::federation::EvictActorReport::default()) }
    }
}

fn test_signer() -> Arc<LocalSigner> {
    let seed = [0x5Cu8; 32];
    let signing_key = SigningKey::from_bytes(&seed);
    Arc::new(LocalSigner::from_parts(
        signing_key,
        "test-local-key".to_string(),
        None,
        None,
    ))
}

fn sample_article(scope: &str) -> EncyclopediaArticleSource {
    EncyclopediaArticleSource {
        entity_key_id: "wikipedia:article:einstein".into(),
        language: "en".into(),
        body_bytes: b"<p>Albert Einstein was a theoretical physicist.</p>".to_vec(),
        body_media_type: "text/html".into(),
        project: "wikipedia".into(),
        revision_id: "1234567890".into(),
        edited_at: "2026-05-15T12:00:00Z".parse::<DateTime<Utc>>().unwrap(),
        cohort_scope: scope.to_string(),
        topical_relations: vec![TopicalRelation {
            target_key_id: "wikipedia:article:physics".into(),
            relation: TopicalRelationKind::References,
        }],
        citations: vec![Citation {
            kind: CitationKind::Doi,
            ref_string: "10.1037/0033-2909.126.6.910".into(),
        }],
    }
}

#[tokio::test]
async fn ingest_encyclopedia_article_full_sequence() {
    let mock_engine = MockEngine::new();
    let mock_blobs = MockBlobStorage::new();
    let signer = test_signer();
    let author_id_b64 = LocalSignerAdapter::new(signer.clone()).contributor_id();

    let cell = Cell {
        domain: "general_knowledge".into(),
        language: "en".into(),
        subject: Some("external_content".into()),
    };

    let ctx = IngestContext {
        blob_storage: &mock_blobs,
        node_core: &mock_engine,
        signer: signer.clone(),
        author_key_id: "test-local-key".into(),
    };

    let outcome = ingest_encyclopedia_article(sample_article("federation"), cell.clone(), &ctx)
        .await
        .expect("ingest succeeds");

    // The blob landed with its own SHA + persist-assembled holder.
    assert_eq!(mock_blobs.blob_count(), 1, "exactly one blob stored");
    assert_eq!(mock_blobs.attestation_count(), 1, "exactly one holder attestation emitted");
    assert_eq!(outcome.content_sha256_hex.len(), 64, "content_sha256_hex is 64-char hex");

    // The holder attestation cites the host's own federation key.
    let holder = mock_blobs.last_attestation();
    assert_eq!(holder.attesting_key_id, "test-local-key");
    assert_eq!(holder.scrub_key_id, "test-local-key");
    assert!(!holder.scrub_signature_classical.is_empty(),
        "persist signed the holds_bytes envelope");
    assert!(!holder.original_content_hash_hex.is_empty(),
        "original_content_hash_hex is the canonical-bytes SHA, not the blob SHA");

    // The Contribution envelope was persisted and signed by the host
    // identity (no proxy-signing — `author_id` is the host's pubkey,
    // not Wikipedia's identity, even though the content is about
    // Wikipedia).
    let contributions = mock_engine.contributions();
    assert_eq!(contributions.len(), 1, "exactly one Contribution stored");
    let env = &contributions[0];
    assert_eq!(env.contribution_id, outcome.contribution_id);
    assert_eq!(env.author_id, author_id_b64,
        "author_id is the HOST's contributor_id, not the encyclopedia entity");
    assert_eq!(env.subject.domain, "general_knowledge");
    assert_eq!(env.subject.language, "en");
    assert_eq!(env.subject.subject.as_deref(), Some("external_content"));

    // The payload carries the sub_kind discriminator + the
    // entity_key_id of what the content is about.
    let payload = env.payload.as_object().expect("payload is JSON object");
    assert_eq!(payload.get("sub_kind").and_then(|v| v.as_str()),
        Some("encyclopedia_article"));
    assert_eq!(payload.get("entity_key_id").and_then(|v| v.as_str()),
        Some("wikipedia:article:einstein"));
    assert_eq!(payload.get("cohort_scope").and_then(|v| v.as_str()), Some("federation"));
    assert_eq!(payload.get("content_sha256").and_then(|v| v.as_str()),
        Some(outcome.content_sha256_hex.as_str()));
}

#[tokio::test]
async fn ingest_records_payload_observations_about_third_parties_not_envelope_identity() {
    // Verifies the no-proxy-signing discipline: the envelope is
    // signed by the HOST, observations about the encyclopedia entity
    // are claims encoded IN the payload, never by spoofing the
    // envelope's author_id.
    let mock_engine = MockEngine::new();
    let mock_blobs = MockBlobStorage::new();
    let signer = test_signer();
    let host_id_b64 = LocalSignerAdapter::new(signer.clone()).contributor_id();

    let cell = Cell {
        domain: "general_knowledge".into(),
        language: "en".into(),
        subject: Some("external_content".into()),
    };
    let ctx = IngestContext {
        blob_storage: &mock_blobs,
        node_core: &mock_engine,
        signer: signer.clone(),
        author_key_id: "test-local-key".into(),
    };

    // The article's entity_key_id is "wikipedia:article:einstein" —
    // very deliberately NOT the host's identity. If proxy-signing
    // leaked, author_id would (wrongly) be derived from the entity.
    let _ = ingest_encyclopedia_article(sample_article("community"), cell, &ctx).await.unwrap();

    let env = &mock_engine.contributions()[0];
    assert_eq!(env.author_id, host_id_b64);
    assert_ne!(env.author_id, "wikipedia:article:einstein");
}
