# Persist v4.0 (Data Access Surface) absorption checklist

**Status:** staged 2026-06-05 (pre-tag). Persist v4.0 ships tonight; lens is
pausing for it. NodeCore is pinned at persist **3.6.9** — ~8 minor versions
behind the live matrix (**3.14.3**) before the 4.0 cut.

This checklist was built by inspecting the **actual v4.0 API** in the local
`~/CIRISPersist` checkout (`v4.0-das` branch, 3.14.3, commits E "ReadEngine v2
— scope on every read" + F "write-path cohort_scope admission AV-45"). Every
break below is confirmed against shipping signatures, not predicted.

When the conformance matrix updates for the 4.0 triple, work top to bottom.

---

## TL;DR — the break surface is small + mechanical

The v4.0 DAS reorganizes persist's **`ReadEngine`** lens/dashboard read surface
(`src/read/` → `src/ceg/`, `CallerScope` on every read, `NotImplemented`
dropped, `ScopeRefused` added). NodeCore consumes **`cirisnode::NodeCoreService`
+ `federation::{FederationDirectory, BlobStorage}` + `audit::AuditService`** —
NOT `ciris_persist::read::*`. So the module reorg does **not** hit NodeCore's
imports. The real breaks are:

1. Two duck-typed PyEngine read calls changed arity/name (python.rs).
2. New required trait methods the test mocks must stub (the recurring pattern).
3. `put_contribution` + the cohabitation capsules are **unchanged** ✓.

**Dividend:** the 3.6.9 → 3.14.x catch-up lands the **V059 identity/family +
V060 community substrate** (`put_family` / `put_community` /
`put_identity_occurrence` + lookups now exist on `FederationDirectory`) —
unblocking the deferred async admission tails of NodeCore#29 / #30 / #31.

---

## A. Pin bumps (`Cargo.toml`)

- [ ] `ciris-persist` `tag = "v3.6.9"` → the 4.0 tag (per CIRISConformance matrix)
- [ ] `ciris-edge` → the matched 4.0-compatible edge (edge must also bump —
      edge v1.1.3 was built against persist 3.6.x; it cannot compile against
      4.0 alone. Wait for the matrix's full triple.)
- [ ] `ciris-keyring` (dev-dep) `tag = "v4.4.3"` → matched verify
- [ ] Update the substrate-triple comment block at the bottom of Cargo.toml
- [ ] `cargo update -p ciris-persist -p ciris-edge -p ciris-keyring`

## B. PyO3 duck-typed read-call fixes (`src/python.rs`)

The compose pyfunctions call persist's `PyEngine` via `call_method` (dynamic
dispatch — compiles regardless, breaks at **runtime** against the real engine).
These are NOT covered by NodeCore's unit tests (which exercise the pure
`compose.rs` logic against mocks, not a live PyEngine), so the break is latent
until a real engine is wired — verify each by hand.

- [ ] **`list_attestations`** — 4 sites (≈ lines 195, 222, 236, 263).
      v4.0 sig: `list_attestations(filter_json, cursor_json: Option, limit: i64,
      caller_occurrence_key_id: Option)`. NodeCore calls `(&filter_json,)` (1 arg)
      → update to `(filter_json, None, <limit>, None)`. `None` caller = Unauthenticated.
- [ ] **`list_contributions`** — 3 sites (≈ lines 316, 333, 346). **Renamed**:
      the v4.0 PyEngine method is **`cirisnode_list_contributions(filter, cursor,
      limit)`** (`src/ffi/pyo3.rs:10231`), not `list_contributions`. Update both
      the method name AND arity: `(filter,)` → `("cirisnode_list_contributions",
      (filter, None, <limit>))`.
- [x] **`list_attestations_for`** — 2 sites (lines 178, 298). v4.0 sig is still
      `list_attestations_for(attested_key_id)` (`src/ffi/pyo3.rs:3203`) — **NO
      CHANGE** ✓.

## C. Trait-mock catch-up (the recurring pattern)

Every persist minor that adds a **required** (no-default-impl) trait method
breaks NodeCore's test mocks until stubbed. Mocks: `MockEngine`
(`tests/support/mod.rs`), `SpikeMock` (`tests/substrate_contract.rs`),
`MockBlobStorage` (`tests/ingest.rs`), `MemBlobs` (`src/serving.rs` test mod),
the `FederationDirectory` stub (`tests/support/mod.rs`).

### `BlobStorage` (9 required in v4.0) — NEW: `store_blob_local`
- [ ] Add `store_blob_local` stub to `MemBlobs` (serving.rs) + `MockBlobStorage`
      (ingest.rs). (Others — get_blob / has_blob / inline_bytes_cap / put_blob /
      list_holders / list_local_holders / list_held_by / evict_actor — already
      stubbed.)

### `FederationDirectory` (23 required in v4.0) — NEW: the CEG 0.7/0.8 substrate
The mock stub returns `NotImplemented` for unused methods; add stubs for the
new required methods:
- [ ] `put_identity_occurrence`, `lookup_identity_for_occurrence`,
      `list_identity_occurrences_for` (CEG 0.7 §5.6.8.8 — V059)
- [ ] `put_family`, `lookup_family`, `list_families_for_member` (CEG 0.7 — V059)
- [ ] `put_community`, `lookup_community`, `list_communities_for_member`
      (CEG 0.8 — V060)
- [ ] `lookup_keys_for_identity`, `list_keys_by_identity_type` (if new vs 3.6.9)
- [ ] Verify the PQC fill-in methods (`attach_*_pqc_signature`,
      `list_hybrid_pending_*`) match the mock's existing stubs.

### `NodeCoreService` (22 required in v4.0) — likely STABLE
- [ ] Confirm all 22 impl'd (cast_vote, put_contribution, list_contributions,
      list_votes, read_vote_weight, routable_contributors, the ledger updates,
      moderation/slashing/reconsideration/promotion puts, the multimedia
      `list_takedowns_for` / `list_key_grants_for` / `list_key_grants_for_content`
      / `retire_key_grants`, the delivery-attestation trio). These were all
      present at 3.6.9; expect no delta.

### `AuditService`
- [ ] Confirm `record_entry` / `list_entries` / `verify_chain` unchanged.

## D. Confirmed UNCHANGED (no action) ✓

- `put_contribution(env)` — AV-45 write-path cohort_scope admission is enforced
  inside the engine, NOT via the trait signature. Mocks + call sites safe.
- Cohabitation capsules — `blob_storage_capsule`, `federation_directory_capsule`
  exist in v4.0 (`#[pyo3(name=...)]` wrappers). NodeCore's capsule accessors
  (python.rs lines 476, 539, 593) are stable.
- NodeCore's own `SubstrateError::NotImplemented` (service.rs:270) is NodeCore's
  error type, not persist's removed `Error::NotImplemented` — unaffected.

## E. Verify (not yet confirmed — check at absorption time)

- [ ] **`executor_capsule` (#157)** — v4.0 adds an ABI-stable `AsyncExecutor`
      vtable capsule (closes the cross-cdylib tokio aliasing class, CIRISEdge#58).
      NodeCore's `cohabitation.rs` install does NOT currently consume it. Confirm
      the 4.0 cohabitation contract doesn't REQUIRE NodeCore to thread the
      executor capsule (it may be needed for the runtime to be shared correctly
      across cdylibs). If required, wire it in `install_from_dispatch`.
- [ ] **edge ↔ persist-4.0 compatibility** — cannot compile-validate NodeCore
      against persist-4.0 alone (edge v1.1.3 built against 3.6.x). The matrix's
      4.0 triple resolves this; do not attempt a persist-only path-patch.

## F. Verification gates (the v0.1.0 CI contract)

- [ ] `cargo test --no-default-features` — 215 tests
- [ ] `cargo test --features python` — 215 tests
- [ ] `RUSTFLAGS="-D warnings" cargo clippy --all-targets --no-default-features`
- [ ] `RUSTFLAGS="-D warnings" cargo clippy --all-targets --features python`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo deny check` (new persist deps may need allowlist additions)

## G. After absorption — the unblocked work

The catch-up lands the V059/V060 substrate, so these deferred tails become
actionable in the same or a follow-on PR:

- NodeCore#29/#30/#31 **async ingest + admission paths** — `put_family` /
  `put_community` / `put_identity_occurrence` now exist; the membership-change
  admission can call `evaluate_consensus_protocol` against the live directory.
- `evaluate_subkind_admission` (#31 Ask 5) — `lookup`/`list` methods exist for
  the location_proof containment lookup.

---

*Built from `~/CIRISPersist@v4.0-das` (3.14.3) signature inspection. Re-verify
the exact line numbers + any post-staging API drift when the tag lands.*
