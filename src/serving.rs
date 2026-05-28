//! Node-mode `agent_files` serving capability per CIRISNodeCore#11.
//!
//! Registers an Edge `ContentFetch` handler that responds with bytes
//! from persist's `BlobStorage`. Only node-mode deployments install
//! this handler (per FSD-002 v1.4 §3.6.7 / NodeCore MISSION §3.4 c/r/n
//! taxonomy — client and relay modes do not serve).
//!
//! # Wire flow
//!
//! 1. Peer A (any mode) sees an attestation citing `evidence_refs:sha256:X`
//! 2. A consults `PeerResolver::resolve_holders(X)` → set of holders
//! 3. A sends [`ContentFetch { sha256: X }`](ciris_edge::messages::ContentFetch)
//!    to peer B (node-mode)
//! 4. B's [`ContentFetchServingHandler`] queries
//!    [`BlobStorage::get_blob`](ciris_persist::federation::BlobStorage::get_blob)
//! 5. B sends back [`ContentBody { sha256: X, bytes, attestation_ref }`](ciris_edge::messages::ContentBody)
//!    or [`ContentMiss { sha256: X, reason }`](ciris_edge::messages::ContentMiss)
//! 6. A verifies `sha256(bytes) == X` on receipt and trusts the
//!    originating attestation as the trust anchor (per Edge#21 spec)
//!
//! # Engine discipline (CIRISNodeCore#4)
//!
//! Takes an injected `Arc<S: BlobStorage>` + `Arc<Edge>`; never
//! constructs an engine or runtime. The Python cohabitation bootstrap
//! obtains `Arc<dyn BlobStorage>` from persist's Engine and hands it
//! to [`install_node_mode_serving`].

use std::sync::Arc;

use async_trait::async_trait;
use ciris_edge::handler::{Handler, HandlerContext, HandlerError};
use ciris_edge::messages::{ContentBody, ContentFetch, ContentMiss, MissReason};
use ciris_edge::{Edge, EdgeError};
use ciris_persist::federation::{BlobBody, BlobStorage};

/// The pure-logic core of a ContentFetch response decision — what
/// the responder should emit given the storage lookup result.
/// Separated from the handler proper for unit-testing without an
/// `Edge` instance.
#[derive(Debug, Clone)]
pub enum ServingResponse {
    /// Storage returned inline bytes; emit a [`ContentBody`].
    Body(ContentBody),
    /// Storage returned `None`, an external pointer NodeCore can't
    /// resolve to bytes locally, or an error — emit a [`ContentMiss`]
    /// with the appropriate reason.
    Miss(ContentMiss),
}

/// Compute the serving response for a [`ContentFetch`] by querying
/// the injected [`BlobStorage`] and applying the response-hint policy.
///
/// **Hint discipline**: if `fetch.response_hint.max_body_bytes` is
/// set and the held inline bytes exceed it, the responder emits
/// [`MissReason::PolicyDenied`] rather than truncating. The fetcher
/// can advertise a lower cap and the holder can defer to it.
///
/// **External-ref discipline**: v0.1 holders only serve inline blobs.
/// A [`BlobBody::External`] (S3 URI / external URL) result returns
/// [`MissReason::NotHeld`] from this holder — the bytes are in
/// external storage, not directly resolvable via the ContentFetch
/// wire. Phase 2 (`MessageType::ContentChunk`) and S3-pointer-aware
/// flows are tracked separately (CIRISEdge#21-phase2).
pub async fn compute_serving_response<S: BlobStorage + ?Sized>(
    storage: &S,
    fetch: ContentFetch,
) -> ServingResponse {
    match storage.get_blob(&fetch.sha256).await {
        Ok(Some(BlobBody::Inline(bytes))) => {
            if let Some(cap) = fetch.response_hint.as_ref().and_then(|h| h.max_body_bytes) {
                if (bytes.len() as u64) > cap {
                    return ServingResponse::Miss(ContentMiss {
                        sha256: fetch.sha256,
                        reason: MissReason::PolicyDenied,
                    });
                }
            }
            ServingResponse::Body(ContentBody {
                sha256: fetch.sha256,
                bytes,
                attestation_ref: None,
            })
        }
        Ok(Some(BlobBody::External(_))) => ServingResponse::Miss(ContentMiss {
            sha256: fetch.sha256,
            reason: MissReason::NotHeld,
        }),
        Ok(None) => ServingResponse::Miss(ContentMiss {
            sha256: fetch.sha256,
            reason: MissReason::NotHeld,
        }),
        Err(_) => ServingResponse::Miss(ContentMiss {
            sha256: fetch.sha256,
            reason: MissReason::PolicyDenied,
        }),
    }
}

/// Edge [`Handler<ContentFetch>`] implementation that backs node-mode
/// serving. Constructed by [`install_node_mode_serving`]; not
/// typically instantiated directly.
pub struct ContentFetchServingHandler<S: BlobStorage + ?Sized> {
    storage: Arc<S>,
    edge: Arc<Edge>,
}

#[async_trait]
impl<S: BlobStorage + ?Sized + 'static> Handler<ContentFetch> for ContentFetchServingHandler<S> {
    async fn handle(
        &self,
        msg: ContentFetch,
        ctx: HandlerContext,
    ) -> Result<(), HandlerError> {
        match compute_serving_response(&*self.storage, msg).await {
            ServingResponse::Body(body) => {
                // Fire-and-forget reply via Ephemeral delivery. We
                // intentionally discard EdgeError on send failure —
                // the fetcher's retry + PeerResolver fallback covers
                // transient delivery loss, and a serving peer
                // shouldn't fail-loudly on per-recipient send errors.
                let _ = self.edge.send(&ctx.signing_key_id, body).await;
            }
            ServingResponse::Miss(miss) => {
                let _ = self.edge.send(&ctx.signing_key_id, miss).await;
            }
        }
        Ok(())
    }
}

/// Wire a node-mode peer into the federation `agent_files:*` serving
/// surface. Registers a [`Handler<ContentFetch>`] on `edge` that
/// queries the injected `storage` and replies with [`ContentBody`] or
/// [`ContentMiss`] over Edge's Ephemeral request/response class.
///
/// Per FSD-002 v1.4 §3.6.7: **only node-mode peers call this**.
/// Client-mode and relay-mode peers fetch bytes via PeerResolver +
/// ContentFetch but do not serve (relay's transit cache is private
/// and ephemeral; never advertised via `holds_bytes:*`).
///
/// # Discovery
///
/// Persist's [`BlobStorage::put_blob`](ciris_persist::federation::BlobStorage::put_blob)
/// auto-emits `holds_bytes:sha256:{prefix}` attestations into the
/// federation directory (per CIRISPersist#103). The serving handler
/// installed here is what makes those `holds_bytes:*` advertisements
/// actually serviceable: fetchers reading the directory see the
/// peer-as-holder, send `ContentFetch`, and this handler responds.
///
/// # Engine discipline
///
/// `storage` and `edge` are injected; NodeCore constructs neither.
pub async fn install_node_mode_serving<S>(storage: Arc<S>, edge: Arc<Edge>) -> Result<(), EdgeError>
where
    S: BlobStorage + ?Sized + 'static,
{
    let handler = ContentFetchServingHandler {
        storage,
        edge: edge.clone(),
    };
    edge.register_handler::<ContentFetch, _>(handler).await
}

/// Substrate adapter — wire node-mode serving in from persist's
/// [`BackendDispatch`] enum (per CIRISPersist#106, shipped v2.6.0).
///
/// `Engine::federation_directory()` returns `BackendDispatch` rather
/// than `Arc<dyn FederationDirectory>` because the trait isn't
/// object-safe (RPITIT-style methods don't dispatch dynamically).
/// Each variant carries a concrete backend implementing
/// [`BlobStorage`]; both route into [`install_node_mode_serving`]
/// identically — the backend choice is the host's, transparent to
/// node-core. Parallel to
/// [`crate::cohabitation::install_from_dispatch`] for the write side.
pub async fn install_from_dispatch(
    dispatch: ciris_persist::engine::BackendDispatch,
    edge: Arc<Edge>,
) -> Result<(), EdgeError> {
    use ciris_persist::engine::BackendDispatch;
    match dispatch {
        BackendDispatch::Postgres(backend) => install_node_mode_serving(backend, edge).await,
        #[cfg(feature = "sqlite")]
        BackendDispatch::Sqlite(backend) => install_node_mode_serving(backend, edge).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    use ciris_edge::messages::HintShape;
    use ciris_persist::federation::{BlobError, BlobStorage, ExternalRef, PutBlobAttestation};

    /// Minimal in-memory `BlobStorage` for the pure-logic
    /// `compute_serving_response` tests. Only get_blob is exercised;
    /// the other methods are no-op / unimplemented stubs.
    #[derive(Default)]
    struct MemBlobs {
        inline: Mutex<HashMap<[u8; 32], Vec<u8>>>,
        external: Mutex<HashMap<[u8; 32], String>>,
        fail_on: Mutex<Option<[u8; 32]>>,
    }

    impl BlobStorage for MemBlobs {
        fn inline_bytes_cap(&self) -> usize {
            64 * 1024
        }

        fn put_blob(
            &self,
            _sha256: &[u8; 32],
            _body: BlobBody,
            _media_type: Option<&str>,
            _attestation: PutBlobAttestation,
        ) -> impl std::future::Future<Output = Result<(), BlobError>> + Send {
            async { unimplemented!("not exercised in compute_serving_response tests") }
        }

        fn get_blob(
            &self,
            sha256: &[u8; 32],
        ) -> impl std::future::Future<Output = Result<Option<BlobBody>, BlobError>> + Send {
            let fail_on = *self.fail_on.lock().unwrap();
            let inline = self.inline.lock().unwrap().get(sha256).cloned();
            let external = self.external.lock().unwrap().get(sha256).cloned();
            let key = *sha256;
            async move {
                if fail_on == Some(key) {
                    return Err(BlobError::HashMismatch {
                        expected_hex: "test-induced".into(),
                        got_hex: "test-induced".into(),
                    });
                }
                if let Some(bytes) = inline {
                    return Ok(Some(BlobBody::Inline(bytes)));
                }
                if let Some(uri) = external {
                    return Ok(Some(BlobBody::External(ExternalRef {
                        uri,
                        size_bytes: 0,
                        media_type: None,
                    })));
                }
                Ok(None)
            }
        }

        fn has_blob(
            &self,
            sha256: &[u8; 32],
        ) -> impl std::future::Future<Output = Result<bool, BlobError>> + Send {
            let held = self.inline.lock().unwrap().contains_key(sha256)
                || self.external.lock().unwrap().contains_key(sha256);
            async move { Ok(held) }
        }

        fn list_holders(
            &self,
            _sha256: &[u8; 32],
        ) -> impl std::future::Future<Output = Result<Vec<String>, BlobError>> + Send {
            async { Ok(Vec::new()) }
        }
    }

    fn sha(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn fetch(sha256: [u8; 32], max_body_bytes: Option<u64>) -> ContentFetch {
        ContentFetch {
            sha256,
            response_hint: max_body_bytes.map(|cap| HintShape {
                max_body_bytes: Some(cap),
                prefer_chunked: false,
            }),
        }
    }

    #[tokio::test]
    async fn inline_blob_returns_content_body_with_bytes_and_sha() {
        let store = MemBlobs::default();
        store.inline.lock().unwrap().insert(sha(1), vec![0xAA, 0xBB, 0xCC]);

        let resp = compute_serving_response(&store, fetch(sha(1), None)).await;
        match resp {
            ServingResponse::Body(body) => {
                assert_eq!(body.sha256, sha(1));
                assert_eq!(body.bytes, vec![0xAA, 0xBB, 0xCC]);
                assert!(body.attestation_ref.is_none());
            }
            other => panic!("expected Body, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn missing_blob_returns_content_miss_with_not_held() {
        let store = MemBlobs::default();
        let resp = compute_serving_response(&store, fetch(sha(42), None)).await;
        match resp {
            ServingResponse::Miss(miss) => {
                assert_eq!(miss.sha256, sha(42));
                assert_eq!(miss.reason, MissReason::NotHeld);
            }
            other => panic!("expected Miss, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn external_blob_returns_not_held_in_v0_1() {
        let store = MemBlobs::default();
        store
            .external
            .lock()
            .unwrap()
            .insert(sha(7), "s3://test-bucket/blob".into());

        let resp = compute_serving_response(&store, fetch(sha(7), None)).await;
        match resp {
            ServingResponse::Miss(miss) => {
                assert_eq!(miss.reason, MissReason::NotHeld);
            }
            other => panic!("expected Miss(NotHeld) for external blob, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn hint_max_body_bytes_too_small_returns_policy_denied() {
        let store = MemBlobs::default();
        // 100 bytes held
        store.inline.lock().unwrap().insert(sha(3), vec![0; 100]);

        // Fetcher caps at 50 bytes — policy denied
        let resp = compute_serving_response(&store, fetch(sha(3), Some(50))).await;
        match resp {
            ServingResponse::Miss(miss) => {
                assert_eq!(miss.reason, MissReason::PolicyDenied);
            }
            other => panic!("expected Miss(PolicyDenied), got {other:?}"),
        }

        // Fetcher caps at 200 bytes — body returned
        let resp = compute_serving_response(&store, fetch(sha(3), Some(200))).await;
        assert!(matches!(resp, ServingResponse::Body(_)));
    }

    #[tokio::test]
    async fn storage_error_returns_policy_denied() {
        let store = MemBlobs::default();
        store.inline.lock().unwrap().insert(sha(5), vec![1, 2, 3]);
        *store.fail_on.lock().unwrap() = Some(sha(5));

        let resp = compute_serving_response(&store, fetch(sha(5), None)).await;
        match resp {
            ServingResponse::Miss(miss) => {
                // Storage error → PolicyDenied (conservative; the
                // serving peer is refusing rather than claiming
                // not-held)
                assert_eq!(miss.reason, MissReason::PolicyDenied);
            }
            other => panic!("expected Miss(PolicyDenied) on storage error, got {other:?}"),
        }
    }
}
