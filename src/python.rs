//! PyO3 bindings — Phase 1 client surface (CIRISNodeCore#1).
//!
//! # Scope
//!
//! Phase 1 is the **client surface**: envelope construction +
//! Ed25519 signing. It replaces the hand-rolled wire-format +
//! signing in CIRISAgent's Python `cirisnode` adapter — the
//! deferral-signature-drift bug class #1 names. Canonical-bytes is
//! computed in Rust (via `ciris_persist::cirisnode::verify`),
//! identical to persist's verify path on the other side of the wire.
//!
//! # Engine discipline (CIRISNodeCore#4)
//!
//! These bindings **never** construct a persist `Engine` or a tokio
//! runtime. Phase 1 is pure — there is no engine to inject. Phases
//! 2-3 (relay / node modes that host substrate state) consume an
//! injected `ciris_persist` Engine handle; that surface is gated on
//! a persist coordination item — persist's `PyEngine` currently
//! exposes `audit_*` + `grant_trust` but not the `NodeCoreService`
//! write surface (`put_contribution`, `cast_vote`, …) — and is not
//! in this module yet.
//!
//! # Wire idiom
//!
//! JSON-in / JSON-out, matching persist's `PyEngine`
//! (`audit_record_entry(entry_json)`, `list_trust_grants(…) -> String`).
//! Python passes JSON strings; Rust builds + signs; the signed
//! canonical envelope comes back as a JSON string ready to ship.

#![cfg(feature = "python")]

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::sign::{build_contribution, build_vote, Ed25519Signer as RustSigner};
use crate::substrate::{Cell, ContributionType, WitnessSet};

/// Software Ed25519 signer — Python handle over
/// [`crate::sign::Ed25519Signer`]. NOT recommended for production
/// (holds the seed in process memory); real deployments back the
/// `EnvelopeSigner` trait with TPM / HSM / OS keystore.
#[pyclass(name = "Ed25519Signer", module = "ciris_node_core")]
pub struct PyEd25519Signer {
    inner: RustSigner,
}

#[pymethods]
impl PyEd25519Signer {
    /// Construct from a 32-byte seed.
    #[new]
    fn new(seed: Vec<u8>) -> PyResult<Self> {
        let seed: [u8; 32] = seed
            .as_slice()
            .try_into()
            .map_err(|_| PyValueError::new_err("seed must be exactly 32 bytes"))?;
        Ok(Self {
            inner: RustSigner::from_seed(seed),
        })
    }

    /// The contributor identity — base64 (standard) Ed25519 pubkey —
    /// for envelopes this signer produces. Use as `author_id` /
    /// `voter_id`.
    fn contributor_id(&self) -> String {
        self.inner.contributor_id()
    }
}

fn parse_contribution_type(s: &str) -> PyResult<ContributionType> {
    serde_json::from_value(serde_json::Value::String(s.to_owned()))
        .map_err(|e| PyValueError::new_err(format!("unknown contribution_type {s:?}: {e}")))
}

fn json_err(field: &str, e: serde_json::Error) -> PyErr {
    PyValueError::new_err(format!("{field}: {e}"))
}

/// Build + sign a `ContributionEnvelope`. Returns the signed
/// envelope as canonical JSON.
///
/// `contribution_type` is a SCHEMA §3.1 wire string
/// (`deferral_request`, `deferral_response`, `proposal`,
/// `wa_candidacy`, `expertise_attestation`, `moderation_event`,
/// `reconsideration_request`). `cell_json` / `payload_json` /
/// `witness_set_json` are the corresponding JSON shapes per SCHEMA
/// §2.5 / §4 / §6. The envelope's `author_id` is the signer's
/// `contributor_id()`.
#[pyfunction]
#[pyo3(signature = (signer, contribution_id, contribution_type, cell_json, payload_json, witness_set_json=None))]
fn build_contribution_envelope(
    signer: &PyEd25519Signer,
    contribution_id: String,
    contribution_type: String,
    cell_json: String,
    payload_json: String,
    witness_set_json: Option<String>,
) -> PyResult<String> {
    let ct = parse_contribution_type(&contribution_type)?;
    let cell: Cell = serde_json::from_str(&cell_json).map_err(|e| json_err("cell_json", e))?;
    let payload: serde_json::Value =
        serde_json::from_str(&payload_json).map_err(|e| json_err("payload_json", e))?;
    let witness_set: Option<WitnessSet> = match witness_set_json {
        Some(j) => Some(serde_json::from_str(&j).map_err(|e| json_err("witness_set_json", e))?),
        None => None,
    };
    let env = build_contribution(
        contribution_id,
        ct,
        signer.inner.contributor_id(),
        cell,
        payload,
        witness_set,
        &signer.inner,
    )
    .map_err(|e| PyValueError::new_err(format!("build_contribution: {e}")))?;
    serde_json::to_string(&env).map_err(|e| json_err("serialize envelope", e))
}

/// Build + sign a `VoteEnvelope`. Returns the signed envelope as
/// canonical JSON. `contribution_id` is `None` for free-form polls,
/// `Some` for Contribution-adoption votes. The envelope's `voter_id`
/// is the signer's `contributor_id()`.
#[pyfunction]
#[pyo3(signature = (signer, vote_id, cell_json, score_json, contribution_id=None, rationale=None))]
fn build_vote_envelope(
    signer: &PyEd25519Signer,
    vote_id: String,
    cell_json: String,
    score_json: String,
    contribution_id: Option<String>,
    rationale: Option<String>,
) -> PyResult<String> {
    let cell: Cell = serde_json::from_str(&cell_json).map_err(|e| json_err("cell_json", e))?;
    let score: serde_json::Value =
        serde_json::from_str(&score_json).map_err(|e| json_err("score_json", e))?;
    let env = build_vote(
        vote_id,
        signer.inner.contributor_id(),
        contribution_id,
        cell,
        score,
        rationale,
        &signer.inner,
    )
    .map_err(|e| PyValueError::new_err(format!("build_vote: {e}")))?;
    serde_json::to_string(&env).map_err(|e| json_err("serialize envelope", e))
}

// ---------------------------------------------------------------------------
// Phase 2 — read-composition surfaces (CIRISAgent#800 / CIRISNodeCore#12).
//
// **Engine discipline** (CIRISNodeCore#4): NodeCore NEVER constructs an
// engine or runtime; it consumes an INJECTED `ciris_persist.Engine` handle.
// These pyfunctions accept the engine as a Python object, call directly
// into persist's PyO3 surface for the data, then aggregate via
// [`crate::compose`] — one call per UI surface, no Python-side
// orchestration required.
//
// Engine handle passed as `&Bound<'_, PyAny>` (duck-typed) rather than a
// concrete `ciris_persist::PyEngine` import — avoids enabling persist's
// `python` feature in node-core's build and keeps the contract loose
// enough that test doubles + alternative engine implementations work.
//
// Pure aggregation logic lives in [`crate::compose`] so unit tests link
// without the pyo3 `extension-module` feature.
// ---------------------------------------------------------------------------

/// Compose UI-ready agent state by calling persist directly. One-call
/// surface for CIRISAgent#800's ProfileScorecard.
///
/// `engine` must expose `list_attestations_for(attested_key_id) -> str`
/// (JSON-serialized `Vec<Attestation>`) — `ciris_persist.Engine` does.
/// Aggregation rules + output shape are in
/// [`crate::compose::compose_agent_state`].
#[pyfunction]
fn agent_state(engine: &Bound<'_, PyAny>, key_id: String) -> PyResult<String> {
    let attestations_json: String = engine
        .call_method1("list_attestations_for", (&key_id,))?
        .extract()?;
    crate::compose::compose_agent_state(key_id, &attestations_json)
        .map_err(|e| json_err("compose_agent_state", e))
}

/// One-call surface for the Participate screen — fetches active
/// `need:{domain}:{kind}` attestations and shapes them for UI consumption.
///
/// `filter_json` is the same JSON shape persist's `list_attestations`
/// accepts; pass `"{}"` for all needs. Recommended filter:
/// `{"dimension_prefix": "need:"}` plus optional `{"domain", "kind"}` for
/// further narrowing (the latter two also filter at the compose layer per
/// [`crate::compose::compose_needs_feed`]).
#[pyfunction]
fn needs_feed(engine: &Bound<'_, PyAny>, filter_json: String) -> PyResult<String> {
    let attestations_json: String = engine
        .call_method1("list_attestations", (&filter_json,))?
        .extract()?;
    crate::compose::compose_needs_feed(&attestations_json, &filter_json)
        .map_err(|e| json_err("compose_needs_feed", e))
}

/// One-call surface for The Commons contribution card — fetches every
/// attestation referencing `contribution_id` and buckets by dimension
/// prefix (votes / aggregate / witness-diversity / truth-grounding /
/// testimonial-witness).
///
/// The persist call uses `list_attestations` with a filter that selects
/// attestations whose dimension references the contribution_id; the
/// concrete filter shape depends on persist's `list_attestations`
/// capabilities. v0.1 default: filter for dimension-suffix match on
/// `contribution_id`.
#[pyfunction]
fn contribution(engine: &Bound<'_, PyAny>, contribution_id: String) -> PyResult<String> {
    // Filter pattern: ask persist for attestations whose dimension contains
    // the contribution_id. Persist's filter semantics may evolve; for v0.1
    // we pass a simple "dimension_substring" filter and rely on the
    // compose layer to bucket precisely.
    let filter_json = serde_json::json!({
        "dimension_substring": contribution_id
    })
    .to_string();
    let attestations_json: String = engine
        .call_method1("list_attestations", (&filter_json,))?
        .extract()?;
    crate::compose::compose_contribution(contribution_id, &attestations_json)
        .map_err(|e| json_err("compose_contribution", e))
}

/// One-call surface for the Constitutional / Accord screen — walks the
/// upward-only DAG rooted at `goal_id` (Goal ← Approach ← Method ←
/// Progress Measure) via four prefix-filtered `list_attestations` calls.
#[pyfunction]
fn decision_hierarchy(engine: &Bound<'_, PyAny>, goal_id: String) -> PyResult<String> {
    fn list_with_prefix(engine: &Bound<'_, PyAny>, prefix: &str) -> PyResult<String> {
        let filter = serde_json::json!({"dimension_prefix": prefix}).to_string();
        engine
            .call_method1("list_attestations", (filter,))?
            .extract()
    }

    let goals_json = list_with_prefix(engine, "goal:")?;
    let approaches_json = list_with_prefix(engine, "approach:")?;
    let methods_json = list_with_prefix(engine, "method:")?;
    let measures_json = list_with_prefix(engine, "progress_measure:")?;

    crate::compose::compose_decision_hierarchy(
        goal_id,
        &goals_json,
        &approaches_json,
        &methods_json,
        &measures_json,
    )
    .map_err(|e| json_err("compose_decision_hierarchy", e))
}

/// One-call surface for the Wise Authority screen — fetches the three
/// governance-tier attestation families (moderation / slashing /
/// reconsideration) scoped to the (`domain`, `language`) cell.
#[pyfunction]
fn wa_state(engine: &Bound<'_, PyAny>, domain: String, language: String) -> PyResult<String> {
    fn list_with_prefix(engine: &Bound<'_, PyAny>, prefix: &str) -> PyResult<String> {
        let filter = serde_json::json!({"dimension_prefix": prefix}).to_string();
        engine
            .call_method1("list_attestations", (filter,))?
            .extract()
    }

    let moderation_json = list_with_prefix(engine, "moderation:")?;
    let slashing_json = list_with_prefix(engine, "slashing:")?;
    let reconsideration_json = list_with_prefix(engine, "reconsideration:")?;

    crate::compose::compose_wa_state(
        domain,
        language,
        &moderation_json,
        &slashing_json,
        &reconsideration_json,
    )
    .map_err(|e| json_err("compose_wa_state", e))
}

/// One-call surface for an article's quality reading
/// (CIRISNodeCore#19 Phase 3). For an `external_content` article at
/// `article_key_id`, fetches every active `scores` attestation
/// targeting it and aggregates per-axis quality per
/// [`crate::compose::compose_article_quality`].
///
/// `sub_kind` is one of `encyclopedia_article` / `news_article` /
/// `blog_post` / `chat_message` and selects which dimension family
/// (`encyclopedia:*` / `news:*` / `blog:*` / `chat:*`) drives the
/// aggregation.
#[pyfunction]
fn article_quality(
    engine: &Bound<'_, PyAny>,
    article_key_id: String,
    sub_kind: String,
) -> PyResult<String> {
    let attestations_json: String = engine
        .call_method1("list_attestations_for", (&article_key_id,))?
        .extract()?;
    crate::compose::compose_article_quality(&attestations_json, &article_key_id, &sub_kind)
        .map_err(|e| json_err("compose_article_quality", e))
}

/// One-call surface for the **Local** section of the three-tier UI —
/// the user's own self-scoped `external_content` Contributions.
/// Calls `engine.list_contributions` (subject_kind=external_content,
/// author=owner_key_id) then filters by `cohort_scope: self`.
#[pyfunction]
fn local_feed(engine: &Bound<'_, PyAny>, owner_key_id: String) -> PyResult<String> {
    let filter = serde_json::json!({
        "subject_kind": "external_content",
        "author_id": owner_key_id,
    })
    .to_string();
    let contributions_json: String = engine
        .call_method1("list_contributions", (filter,))?
        .extract()?;
    crate::compose::compose_local_feed(&contributions_json, &owner_key_id)
        .map_err(|e| json_err("compose_local_feed", e))
}

/// One-call surface for the **Community commons** section —
/// `external_content` Contributions with cohort_scope ∈
/// {family, community, affiliations}.
///
/// `filter_json` is forwarded to compose; may contain `sub_kind` to
/// narrow by kind (encyclopedia_article / news_article / accord_data /
/// local_data).
#[pyfunction]
fn community_feed(engine: &Bound<'_, PyAny>, filter_json: String) -> PyResult<String> {
    let persist_filter = serde_json::json!({"subject_kind": "external_content"}).to_string();
    let contributions_json: String = engine
        .call_method1("list_contributions", (persist_filter,))?
        .extract()?;
    crate::compose::compose_community_feed(&contributions_json, &filter_json)
        .map_err(|e| json_err("compose_community_feed", e))
}

/// One-call surface for the **Global commons** section —
/// `external_content` Contributions with cohort_scope ∈
/// {species, planet, federation}.
#[pyfunction]
fn global_feed(engine: &Bound<'_, PyAny>, filter_json: String) -> PyResult<String> {
    let persist_filter = serde_json::json!({"subject_kind": "external_content"}).to_string();
    let contributions_json: String = engine
        .call_method1("list_contributions", (persist_filter,))?
        .extract()?;
    crate::compose::compose_global_feed(&contributions_json, &filter_json)
        .map_err(|e| json_err("compose_global_feed", e))
}

// ---------------------------------------------------------------------------
// Phase 3 — cohabitation install (CIRISNodeCore#11).
//
// This is the agent-runtime entry point: hand node-core a persist `PyEngine`
// + edge `PyEdge` and it wires the 8 federation-consensus MessageType
// handlers onto the host's `Edge`, addressable by the host's federation
// signing key (edge is Reticulum-native — addressing IS identity).
//
// Crosses a Rust type boundary (PyRef extraction of persist's `PyEngine` +
// edge's `PyEdge` to call the plain-pub-fn Option-B accessors
// `node_core_service` / `edge_handle`). The `python` feature therefore
// activates `ciris-persist/pyo3` + `ciris-edge/pyo3` so the concrete
// pyclasses are importable on the Rust side.
//
// Per CIRISNodeCore#4 we still NEVER construct a runtime — we borrow
// persist's via `ciris_persist::current_runtime_handle()`.
// ---------------------------------------------------------------------------

use std::ffi::CStr;
use std::ptr::NonNull;

use ciris_edge::ffi::pyo3::PyEdge;
use ciris_persist::engine::BackendDispatch;
use ciris_persist::ffi::pyo3::{current_runtime_handle, PyEngine};
use pyo3::exceptions::PyRuntimeError;
use pyo3::types::PyCapsule;

/// Wire node-core into the host's substrate — the cohabitation
/// bootstrap, exposed to the agent's Python runtime.
///
/// Borrows a persist `Engine` handle (for the `NodeCoreDispatch` →
/// `NodeCoreService` injection per CIRISNodeCore#4) and an edge
/// `Edge` handle (for `MessageType` handler registration). After this
/// call returns, the 8 federation-consensus message types
/// (`ContributionSubmit`, `VoteCast`, `DeferralRequest`,
/// `DeferralResponse`, `ExpertiseAttestationPublish`,
/// `ModerationEventPublish`, `SlashingAttestationPublish`,
/// `ReconsiderationRequest`) dispatch into node-core when addressed
/// to the host's federation signing key.
///
/// Idiom:
///
/// ```python
/// from ciris_persist import Engine
/// from ciris_edge import init_edge_runtime
/// from ciris_node_core import install_cohabitation
///
/// engine = Engine(...)              # persist constructs runtime
/// edge   = init_edge_runtime(engine, ...)
/// install_cohabitation(engine, edge)  # node-core wires its handlers
/// ```
///
/// Returns `None` on success; raises `RuntimeError` if the persist
/// runtime is not available (engine closed / not yet constructed) or
/// the edge handler registration fails.
#[pyfunction]
fn install_cohabitation(engine: PyRef<'_, PyEngine>, edge: PyRef<'_, PyEdge>) -> PyResult<()> {
    let dispatch = engine.node_core_service();
    let edge_handle = edge.edge_handle();
    let runtime = current_runtime_handle().ok_or_else(|| {
        PyRuntimeError::new_err(
            "ciris_persist runtime not initialized — \
             construct an Engine before install_cohabitation",
        )
    })?;
    // Drop the GIL across `block_on` — node-core's install path is
    // pure-Rust async (no Python reentry); holding the GIL while
    // blocking the OS thread risks deadlocking any other Python
    // thread that wants to advance work.
    let py = engine.py();
    py.detach(|| {
        runtime.block_on(async move {
            crate::cohabitation::install_from_dispatch(dispatch, &edge_handle).await
        })
    })
    .map_err(|e| PyRuntimeError::new_err(format!("install_cohabitation: {e}")))?;
    Ok(())
}

/// Wire node-mode content serving into the host's edge runtime — the
/// node-mode-serving half of CIRISNodeCore#11's joint `agent_files:*`
/// namespace claim.
///
/// After this call returns, the host responds to inbound
/// `ContentFetch{sha256}` messages by reading from persist's
/// `federation_blobs` table (CIRISPersist#103) and returning
/// `ContentBody{sha256, bytes, attestation_ref}` (or `ContentMiss`).
/// This is what makes the `holds_bytes:sha256:*` advertisements that
/// persist's `BlobStorage::put_blob` auto-emits actually serviceable
/// to fetching peers.
///
/// Idiom (typically called right after [`install_cohabitation`]):
///
/// ```python
/// install_cohabitation(engine, edge)        # write side: 8 MessageTypes
/// install_node_mode_serving(engine, edge)   # read side: ContentFetch
/// ```
///
/// # How the substrate handoff works
///
/// Persist v3.1.1 exposes the blob substrate as a `PyCapsule`
/// (`PyEngine.blob_storage_capsule()`, CIRISPersist#115). The capsule
/// wraps the same `BackendDispatch` value `federation_directory()` /
/// `outbound_queue()` return — the concrete backend (Postgres or
/// SQLite struct) implements all three trait surfaces — but the
/// capsule is the *documented* blob-storage entry point and pins the
/// name tag `ciris_persist::blob_storage` so misuse is caught.
///
/// We borrow the dispatch out of the capsule, clone it (cheap — it's
/// just `Arc`-wrapped backends), and drop the GIL across `block_on`
/// so other Python threads stay unblocked while edge registers the
/// `ContentFetch` handler.
#[pyfunction]
fn install_node_mode_serving(
    engine: &Bound<'_, PyEngine>,
    edge: &Bound<'_, PyEdge>,
) -> PyResult<()> {
    let py = engine.py();

    // Borrow the blob-storage `BackendDispatch` out of persist's
    // capsule. Name tag pins identity (per CIRISPersist#115); the
    // unsafe deref is bounded by the capsule lifetime — we clone
    // into an owned value before any GIL drop so the deref window is
    // strictly synchronous.
    let cap_obj = engine.call_method0("blob_storage_capsule")?;
    let cap: &Bound<'_, PyCapsule> = cap_obj.cast::<PyCapsule>()?;
    let name: &CStr = CStr::from_bytes_with_nul(b"ciris_persist::blob_storage\0")
        .expect("static tag has no interior NUL");
    let raw: NonNull<std::ffi::c_void> = cap.pointer_checked(Some(name))?;
    // SAFETY: `pointer_checked` returned Ok with the pinned name tag
    // `ciris_persist::blob_storage`, which CIRISPersist#115 guarantees
    // points to a `BackendDispatch` owned by the capsule for its
    // lifetime. We deref once, clone immediately into an owned value,
    // and never hold the reference across a `py.detach` boundary —
    // so the pointer remains valid for the strictly synchronous deref
    // window.
    #[allow(unsafe_code)]
    let dispatch: BackendDispatch =
        unsafe { raw.cast::<BackendDispatch>().as_ref() }.clone();

    let edge_handle = edge.borrow().edge_handle();
    let runtime = current_runtime_handle().ok_or_else(|| {
        PyRuntimeError::new_err(
            "ciris_persist runtime not initialized — \
             construct an Engine before install_node_mode_serving",
        )
    })?;

    py.detach(|| {
        runtime.block_on(async move {
            crate::serving::install_from_dispatch(dispatch, edge_handle).await
        })
    })
    .map_err(|e| PyRuntimeError::new_err(format!("install_node_mode_serving: {e}")))?;
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────
// Trust recursion depth — admission-decision pyfunctions (NodeCore#21).
//
// The CIRISConformance harness asserts the trust-recursion-depth knob
// described in FEDERATION_SCALING_MODEL.md §1.4: each server walks the
// delegates_to attestation graph to depth N when deciding admission.
//
// These pyfunctions expose the decision oracle without crossing into
// the actual admission gate (which lives at persist's put_blob /
// put_attestation / put_contribution per CIRISPersist#123).
// ───────────────────────────────────────────────────────────────────────

/// Return the set of key_ids admitted by `root_key_id` at the given
/// trust recursion depth — the BFS over active `delegates_to` edges
/// in the federation directory, with `withdraws` / `recants`
/// retractions honored.
///
/// Returned as a JSON string for parsing-style symmetry with the
/// other compose pyfunctions: `{"root": "...", "depth": N, "set":
/// ["key_id", ...]}`. Order is insertion-order from the BFS.
///
/// The Conformance harness uses this to assert:
/// * depth 0 admits only the root
/// * depth 1 admits friend-of-friends (heavy small-world overlap)
/// * depth N admits exactly the transitive closure within N hops
#[pyfunction]
fn effective_trust_set(
    engine: &Bound<'_, PyEngine>,
    root_key_id: String,
    depth: usize,
) -> PyResult<String> {
    let py = engine.py();
    let cap_obj = engine.call_method0("federation_directory_capsule")?;
    let cap: &Bound<'_, PyCapsule> = cap_obj.cast::<PyCapsule>()?;
    let name: &CStr =
        CStr::from_bytes_with_nul(b"ciris_persist::federation_directory\0")
            .expect("static tag has no interior NUL");
    let raw: NonNull<std::ffi::c_void> = cap.pointer_checked(Some(name))?;
    // SAFETY: pinned name tag matches the persist capsule contract
    // (CIRISPersist#95). The pointer remains valid for the lifetime
    // of the capsule; we clone the dispatch immediately into an
    // owned `BackendDispatch` so the borrow lifetime ends before
    // `py.detach` releases the GIL.
    #[allow(unsafe_code)]
    let dispatch: BackendDispatch =
        unsafe { raw.cast::<BackendDispatch>().as_ref() }.clone();

    let runtime = current_runtime_handle().ok_or_else(|| {
        PyRuntimeError::new_err(
            "ciris_persist runtime not initialized — \
             construct an Engine before effective_trust_set",
        )
    })?;

    let set = py
        .detach(|| {
            runtime.block_on(async move {
                let directory: &dyn ciris_persist::federation::FederationDirectory =
                    match &dispatch {
                        BackendDispatch::Postgres(b) => b.as_ref(),
                        BackendDispatch::Sqlite(b) => b.as_ref(),
                    };
                crate::trust_depth::effective_trust_set(directory, &root_key_id, depth).await
            })
        })
        .map_err(|e| PyRuntimeError::new_err(format!("effective_trust_set: {e}")))?;

    let mut sorted: Vec<String> = set.into_iter().collect();
    sorted.sort();
    Ok(serde_json::json!({
        "root": root_key_id,
        "depth": depth,
        "set": sorted,
    })
    .to_string())
}

/// True iff `source_key_id` is in `root_key_id`'s effective trust
/// set at the given depth. Sibling of [`effective_trust_set`];
/// avoids parsing the JSON for the common "is X admitted" check.
#[pyfunction]
fn admits_at_depth(
    engine: &Bound<'_, PyEngine>,
    root_key_id: String,
    source_key_id: String,
    depth: usize,
) -> PyResult<bool> {
    let py = engine.py();
    let cap_obj = engine.call_method0("federation_directory_capsule")?;
    let cap: &Bound<'_, PyCapsule> = cap_obj.cast::<PyCapsule>()?;
    let name: &CStr =
        CStr::from_bytes_with_nul(b"ciris_persist::federation_directory\0")
            .expect("static tag has no interior NUL");
    let raw: NonNull<std::ffi::c_void> = cap.pointer_checked(Some(name))?;
    #[allow(unsafe_code)]
    let dispatch: BackendDispatch =
        unsafe { raw.cast::<BackendDispatch>().as_ref() }.clone();

    let runtime = current_runtime_handle().ok_or_else(|| {
        PyRuntimeError::new_err(
            "ciris_persist runtime not initialized — \
             construct an Engine before admits_at_depth",
        )
    })?;

    py.detach(|| {
        runtime.block_on(async move {
            let directory: &dyn ciris_persist::federation::FederationDirectory =
                match &dispatch {
                    BackendDispatch::Postgres(b) => b.as_ref(),
                    BackendDispatch::Sqlite(b) => b.as_ref(),
                };
            crate::trust_depth::admits_at_depth(
                directory,
                &root_key_id,
                &source_key_id,
                depth,
            )
            .await
        })
    })
    .map_err(|e| PyRuntimeError::new_err(format!("admits_at_depth: {e}")))
}

/// The `ciris_node_core` Python module — Phase 1 client surface +
/// Phase 2 read-composition (CIRISNodeCore#12) + external-content
/// feeds (CIRISNodeCore#19) + Phase 3 cohabitation install
/// (CIRISNodeCore#11) + trust-depth admission oracle
/// (CIRISNodeCore#21).
#[pymodule]
fn ciris_node_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Phase 1
    m.add_class::<PyEd25519Signer>()?;
    m.add_function(wrap_pyfunction!(build_contribution_envelope, m)?)?;
    m.add_function(wrap_pyfunction!(build_vote_envelope, m)?)?;
    // Phase 2 (CIRISNodeCore#12)
    m.add_function(wrap_pyfunction!(agent_state, m)?)?;
    m.add_function(wrap_pyfunction!(needs_feed, m)?)?;
    m.add_function(wrap_pyfunction!(contribution, m)?)?;
    m.add_function(wrap_pyfunction!(decision_hierarchy, m)?)?;
    m.add_function(wrap_pyfunction!(wa_state, m)?)?;
    // External-content feeds (CIRISNodeCore#19 — three-tier UI)
    m.add_function(wrap_pyfunction!(local_feed, m)?)?;
    m.add_function(wrap_pyfunction!(community_feed, m)?)?;
    m.add_function(wrap_pyfunction!(global_feed, m)?)?;
    // External-content quality aggregation (CIRISNodeCore#19 Phase 3)
    m.add_function(wrap_pyfunction!(article_quality, m)?)?;
    // Phase 3 (CIRISNodeCore#11 — cohabitation install + node-mode serving)
    m.add_function(wrap_pyfunction!(install_cohabitation, m)?)?;
    m.add_function(wrap_pyfunction!(install_node_mode_serving, m)?)?;
    // Trust depth admission oracle (CIRISNodeCore#21 / Conformance)
    m.add_function(wrap_pyfunction!(effective_trust_set, m)?)?;
    m.add_function(wrap_pyfunction!(admits_at_depth, m)?)?;
    Ok(())
}
