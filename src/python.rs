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

/// The `ciris_node_core` Python module — Phase 1 client surface +
/// Phase 2 read-composition (CIRISNodeCore#12).
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
    Ok(())
}
