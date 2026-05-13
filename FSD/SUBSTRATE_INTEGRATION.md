# FSD: Substrate integration — CIRISNodeCore consumption of edge + persist

**Status:** Proposed (v0.1.0-dev) — pairs with the skeleton at
`Cargo.toml` + `src/`, committed alongside this document.
**Author:** Eric Moore (CIRIS Team) with Claude Opus 4.7
**Created:** 2026-05-12
**Risk:** Architectural. Pins the API seam between `ciris-node-core` and
the federation substrate (`ciris-persist`, `ciris-edge`). The substrate
crates publish their side of the contract:

- **CIRISPersist** — Appendix A of `CIRISPersist/FSD/CIRIS_PERSIST.md`
  (closes CIRISPersist#30). Names the typed-write + read surface
  node-core consumes. Schema lands in v0.6.x or v0.7.0 per A.5.
- **CIRISEdge** — `MessageType` enum expansion (CIRISEdge#6, open).
  Adds 8 federation-consensus wire-type variants; node-core registers
  typed handlers against them.

This FSD pins what node-core consumes, in what order, with what
discipline. It is the symmetric document to lens-core's
`FSD/CIRIS_LENS_CORE.md` §6 — same shape, different domain.

---

## 1. Why this exists

Two crates depend on the same substrate. Lens-core established the
patterns (`engine.steward_sign`, `engine.put_detection_event`, holds
the `Engine` handle, never opens its own DB connection, never
re-verifies, never re-canonicalizes). Node-core follows the same
discipline.

Without this FSD as a co-document with Appendix A, the contract drifts:
persist evolves typed methods that node-core doesn't track; edge ships
variants that node-core doesn't register; verify-via-persist gets
re-litigated module-by-module. The cost lens-core paid avoiding that
trap (`CIRISLens/cirislens-core` lessons) shouldn't be re-paid by
node-core.

---

## 2. Persist consumption

### 2.1 Typed writes (Appendix A.2)

Node-core writes through the `NodeCoreEngine` trait at
`src/engine.rs`. The trait surface is line-for-line Appendix A.2 + A.3.
When persist v0.6.x ships the concrete methods, one of:

- **Option A — Re-export.** Replace the trait module's body with a
  re-export of persist's concrete `Engine` methods.
- **Option B — Impl-for.** Implement `NodeCoreEngine` for
  `ciris_persist::Engine` in a single `engine_persist.rs` module.

(B) preserves the test seam (in-memory mocks can stay implementing
the trait); (A) drops weight. Decision deferred to v0.1.0 cut-time
once persist v0.6.x is on hand.

Method-by-method consumption table:

| `NodeCoreEngine` method | Persist Appendix A row | Hot path? | Witness gate? |
|---|---|---|---|
| `put_contribution(envelope)` | A.2 row 1 — `contributions` (discriminated by `contribution_type`) | warm (per-Contribution) | enforced at validation before write |
| `cast_vote(vote)` | A.2 row 2 — `votes` | warm | none (routine) |
| `update_credits_ledger(...)` | A.2 row 3 — `credits_ledger` | cold (truth-grounding batch) | non-negative invariant at persist boundary |
| `update_expertise_ledger(...)` | A.2 row 4 — `expertise_ledger` | cold | non-negative invariant + jump-threshold witness gate at write side |
| `put_moderation_event(...)` | A.2 row 5 — `moderation_events` | warm | always required |
| `put_slashing_attestation(...)` | A.2 row 6 — `slashing_attestations` | warm | always required |
| `put_reconsideration_request(...)` | A.2 row 7 — `reconsideration_requests` | warm | always required |
| `put_reconsideration_attestation(...)` | A.2 row 8 — `reconsideration_attestations` | warm | always required |

(The reconsideration attestation write isn't surfaced in the v0.1.0-dev
trait yet — adds in the next skeleton iteration alongside the
moderation/slashing typed payloads.)

### 2.2 Reads (Appendix A.3)

| `NodeCoreEngine` method | Persist Appendix A row | Use site |
|---|---|---|
| `read_vote_weight(contributor, cell)` | A.3 row 1 — composite `Credits × expertise_multiplier × active_tier_multiplier` | `crate::aggregate` (forthcoming) per `MISSION.md` §5.3 |
| `get_credits_ledger(contributor)` | A.3 row 2 — point read | safety.ciris.ai pilot UI; eventually CIRISAgent fold-in |
| `get_expertise_ledger(contributor)` | A.3 row 3 — point read | same |
| `routable_contributors(domain, language)` | A.3 row 4 — Expertise-non-zero × Active-tier filter | `crate::routing` (forthcoming) per `MISSION.md` §3.3 step 1-2 |

Pending vs canonical query split per SCHEMA.md §13.2 lands as a
follow-on read trait in v0.2.x (not blocking v0.1.0). Node-core's
v0.1.0 always queries the pending chain; promotion to canonical
remains a PR-bot path against CIRISAgent per SCHEMA §13.3.

### 2.3 Inherited from persist v0.4.2+

Two methods predate Appendix A and are used as-is from persist's
existing surface:

- `engine.steward_sign(canonical_bytes)` — every node-core-issued
  attestation (promotion, moderation outcome, reconsideration outcome)
  signs through here. Seed never crosses the FFI boundary into
  node-core. Same shape lens-core uses for detection events.
- `engine.canonicalize_envelope_for_signing(value)` — node-core
  **never** implements canonicalization. CIRISPersist#7 closure /
  AV-5 enforcement carries here verbatim.

### 2.4 Schema namespace

Per Appendix A.4: federation-consensus tables live under the
`cirisnode` PostgreSQL schema, sibling to `cirislens` +
`cirislens_derived`. Migration `V011` (or whichever the next persist
migration number is at v0.6.x cut-time) introduces the full table set.

Persist's `[features] cirisnode = [...]` gating means deployments
that don't run node-core skip the migration entirely. Lens-only
deployments (current pilot) remain unaffected.

---

## 3. Edge consumption

### 3.1 Wire types (CIRISEdge#6)

Node-core registers typed handlers for eight new `MessageType`
variants, all federation-consensus durable+requires_ack:

| MessageType (edge#6) | Body type (node-core) | Handler responsibility |
|---|---|---|
| `ContributionSubmit` | `ContributionEnvelope` per `src/contribution.rs` | Validate witness-set gate per §3.5; call `put_contribution`; emit `ContributionSubmitResponse` ack with persisted-id. |
| `VoteCast` | `Vote` per `src/vote.rs` | Validate Credits-cell match per §5; call `cast_vote`; ack with persisted vote id. |
| `ExpertiseAttestationPublish` | (payload pending — §4.10 typed shape) | Validate jump-threshold witness gate per §3.5; call `put_contribution` with `contribution_type = expertise_attestation`. |
| `ModerationEventPublish` | (payload pending — §4.11) | Validate always-required witness set; call `put_moderation_event`. |
| `SlashingAttestationPublish` | (payload pending — §8) | Validate always-required witness set; call `put_slashing_attestation`. |
| `ReconsiderationRequest` | (payload pending — §4.12) | Validate recursion + time bounds per §9; call `put_reconsideration_request`. |
| `DeferralRequest` | `payloads::deferral::DeferralRequest` (already typed in v0.1.0-dev) | Validate Expertise-granularity cell; select routed responders via `routable_contributors`; ack with the routed set. |
| `DeferralResponse` | `payloads::deferral::DeferralResponse` | Validate `responder_id` is in the routed set for `deferral_id`; call `put_contribution` (deferral_response is routed-aggregate, not vote-on-response). |

### 3.2 Handler registration shape

Mirrors `CIRISLensCore/FSD/CIRIS_LENS_CORE.md` §3.2:

```rust
use ciris_edge::Edge;
use ciris_node_core::{NodeCoreEngine, ContributionEnvelope, Vote};
use ciris_node_core::payloads::deferral::{DeferralRequest, DeferralResponse};

let edge = Edge::builder()
    .persist(persist_engine.clone())
    .transport(/* ... */)
    .build()?;

// One register_handler call per MessageType variant.
edge.register_handler::<ContributionEnvelope, _>(|envelope, ctx| {
    let core = node_core.clone();
    async move {
        core.submit_contribution(envelope).await
    }
})?;

edge.register_handler::<Vote, _>(|vote, ctx| {
    let core = node_core.clone();
    async move { core.record_vote(vote).await }
})?;

edge.register_handler::<DeferralRequest, _>(|req, ctx| { /* ... */ })?;
edge.register_handler::<DeferralResponse, _>(|resp, ctx| { /* ... */ })?;
// ... etc for the remaining 4 variants.

edge.run().await?;
```

Node-core's public API surface (forthcoming `NodeCore::builder().build()`)
holds the `NodeCoreEngine` handle and exposes `submit_contribution`,
`record_vote`, `submit_deferral`, `respond_deferral`, etc. Edge calls
those; the typed handler dispatch is the only place envelopes cross
into node-core code.

### 3.3 Verify-via-persist (inherited)

Node-core never re-verifies inbound bytes. Edge owns the path:

```
Transport → parse EdgeEnvelope → lookup_public_key(signing_key_id)
         → verify Ed25519 + ML-DSA-65 → schema validate
         → typed dispatch → node-core handler
```

Node-core's `submit_contribution` (etc.) assumes the input is already
edge-verified. Verifying a second time is anti-pattern AV-5
("CIRISPersist#7 closure"). Same rule lens-core follows.

---

## 4. Substrate discipline (inherited from lens-core)

Five anti-patterns to call out in PR review (lift from
`CIRISLensCore/FSD/CIRIS_LENS_CORE.md` §8 anti-patterns 1-3; same
substrate, same rules):

1. **Re-implementing edge or persist primitives.** No verifier in
   node-core; edge owns it. No canonicalizer; persist owns it. No
   transport; edge owns it. If a PR adds one — wrong layer.
2. **Opening DB connections.** Node-core holds the `NodeCoreEngine`
   handle. Direct connection use bypasses the typed-write boundary
   and breaks the audit story.
3. **Untyped vote-weight or ledger reads.** `read_vote_weight` returns
   the `VoteWeight` struct with three fields; consumers must compute
   `effective()` rather than treating individual fields as the answer.
   Bare `f64`s drop the invariant.
4. **Caller-trusted witness set.** Witness diversity (jurisdictional +
   organizational) is policy-checked at validation time. A PR that
   skips the check because "the envelope already has a `witness_set`
   field" is the §3.5 evasion path.
5. **Silent passthrough on substrate failure.** Persist write fails →
   typed `Error::Substrate` returned to caller, NOT silent retry +
   re-emit. Same lens-core rule: SLO breach → fail-secure variant,
   not pass-through.

---

## 5. Sequencing

Aligned with Appendix A.5 (persist's own sequencing):

| Stage | Persist | Edge | Node-core |
|---|---|---|---|
| **Now (v0.1.0-dev)** | **v0.7.1 shipped** (tag + PyPI): wire types + `NodeCoreService` (14 methods, RPITIT) + PostgresBackend impl + real Ed25519 envelope verify. Cell.subject is `Option<String>` (matches node-core's original design). One gap filed: no canonical-promotion write surface — CIRISPersist#32. | v0.1.2 ships all 8 federation-consensus `MessageType` variants (CIRISEdge#6 closed). | Pinned against persist `tag = "v0.7.1"`. `crate::substrate` re-exports validated. `tests/substrate_contract.rs` (7 tests) proves the contract fits — implements `NodeCoreService` for an in-memory mock using RPITIT directly, round-trips every method. Local parallel wire types still present pending OQ-7 collapse commit. 27 tests green (13 unit + 7 handlers + 7 substrate contract). |
| **v0.1.0 cut** | v0.6.x or v0.7.0 ships `put_contribution`, `cast_vote`, ledger writes, V011 migration. | Edge ship with the 8 new MessageType variants + handler dispatch. | Bump Cargo pins; implement `NodeCoreEngine` for the concrete persist Engine; wire handlers; full test suite (round-trip + property + integration). |
| **v0.1.0 pilot (safety.ciris.ai)** | v0.6.x stable; consensus tables in production for the pilot deployment. | Edge stable with consensus variants. | Pilot deployment of node-core consuming the crowdsourcing-alignment page's Contribution submissions. |
| **v0.2.x** | Pending/canonical query split (§13.2) read surface. | (no required change) | `crate::aggregate` for §7 weighted aggregation; reconsideration + moderation + slashing typed payloads. |
| **Fold (post-PoB §3.1)** | Same persist binary across lens/agent/node. | Same edge across lens/agent/node. | Folds into CIRISAgent runtime alongside lens-core. In-agent scheduler ensures node-core async batches don't preempt lens-core's per-trace SLO (`MISSION.md` §1.4 lifecycle stage "Deployed (folded)"). |

---

## 6. Open questions

| OQ | Decision needed by | Lean |
|---|---|---|
| **OQ-1: trait-or-direct.** When persist v0.6.x ships typed writes, do we keep `NodeCoreEngine` as the seam (Option A.2 §2.1-B above) or drop the trait and call persist directly (§2.1-A)? | v0.1.0 cut-time | Keep the trait. Test seam matters; lens-core didn't have one and the mock cost shows. |
| **OQ-2: typed `Score` variants.** SCHEMA §5.1 enumerates per-subject-kind score shapes; v0.1.0-dev keeps `Score = serde_json::Value`. When do we type them? | v0.1.0 cut-time | Type at v0.1.0 — the discriminator already exists in `SubjectKind`. |
| **OQ-3: in-memory mock engine.** Where does the `MockNodeCoreEngine` live for tests — node-core's `tests/support/` or a separate `ciris-node-core-test` crate? | v0.1.0 cut-time | **CLOSED 2026-05-12** — landed at `tests/support/mod.rs` (`MockEngine`). Fixture setters (`set_routable`, `set_credits`, `set_expertise`, `set_active`) + inspectors (`contributions()`, `votes()`, `write_count()`) + full `NodeCoreEngine` impl. Promotes to a separate crate when safety.ciris.ai backend tests want it. |
| **OQ-4: aggregate result type.** Per §4 anti-pattern 1 in lens-core's FSD, untyped score returns are an evasion path. Node-core's §7 weighted-aggregate result should be a similar typed enum with fail-secure variants (`Resolved`, `BelowQuorum`, `WitnessSetIncomplete`). Shape? | v0.2.x cut | Mirror `ManifoldConformity` — three-variant enum, type system enforces no-silent-fallthrough. |
| **OQ-5: cell discovery.** Where does node-core get the domain allowlist + the language allowlist? SCHEMA §2.5 says "from `ciris_engine/logic/buses/prohibitions.py` + `manifest.json`." Cross-crate at build time? At runtime via persist's federation directory? | v0.1.0 cut-time | Runtime via persist. The agent owns the canonical lists; persist replicates them; node-core reads. |
| **OQ-6: pilot deferral SLA.** CIRISNode WBD has 24h auto-escalation. Should node-core preserve that — and where (crate policy or consumer policy)? | v0.1.0 pilot | Consumer policy. Keep the crate timeout-agnostic; deployment can run a sweeper that re-routes or escalates per local rules. |
| **OQ-7: persist v0.7.0 collapse.** Node-core currently maintains parallel wire types alongside persist α3's `cirisnode::types::*`. When do we collapse, and how? | persist v0.7.0 release | Collapse at v0.7.0 final. Steps: (1) replace `crate::{cell, signature, witness, contribution, vote, ledger}` with re-exports from `crate::substrate`; (2) rename `crate::payloads::*` structs with `Payload` suffix (they are payload-only "policy" types that fill the envelope's `payload: serde_json::Value` field); (3) make `NodeCore` generic over `E: NodeCoreService` (drop `dyn` — persist's RPITIT trait is not dyn-compatible); (4) rewrite `MockEngine` to impl `NodeCoreService` (drop `async_trait` dep); (5) update `wire.rs` newtype wrappers around persist's envelope types (orphan rule on `impl Message` requires node-core to own the type). Risk: ~300-500 lines of churn. Done in a focused commit, not staged. |

---

## 7. References

- `MISSION.md` — eleven primitives, application × contribution table (§1.6), deferral routing (§3.3).
- `SCHEMA.md` — wire format (§3 envelope, §4 payloads, §5 Vote, §6 WitnessSet, §7-9 attestations, §10 Ledgers, §13.2 pending audit chain, §13.3 promotion path).
- `CIRISPersist/FSD/CIRIS_PERSIST.md` Appendix A — substrate side of this contract.
- `CIRISLensCore/FSD/CIRIS_LENS_CORE.md` §3.2 + §6 + §8 — template this FSD follows.
- `CIRISEdge/FSD/CIRIS_EDGE.md` §3.2 + §3.3 — handler registration + verify-via-persist.
- CIRISPersist#30, CIRISPersist#31, CIRISEdge#6 — coordination issues that this contract resolves on node-core's side.
