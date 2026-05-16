# FSD: Message Taxonomy — federation-message primitive structure

**Status:** Design (locks SCHEMA.md §3.2 additions for this round).
**Author:** Eric Moore (CIRIS Team) with Claude Opus 4.7
**Created:** 2026-05-15
**Risk:** Architectural. Establishes the rationale for which primitives
earn a place in the federation wire-format alphabet. SCHEMA.md §3.2
is the wire reflection of this doc.

**Cross-coordinates with:**
- **CIRISNodeCore#3** — Accord §RC subject_kind additions; this doc is
  the framework #3's payload sketches slot into.
- **CIRISPersist#47 / #53** — adds `TrustPurpose::Service` per §6;
  scope-grammar extension for service-access grants.
- **CIRISEdge** — proposes new `ServiceRequest` / `ServiceResponse`
  `MessageType` variants for peer-to-peer RPC over edge transport
  (NOT archived to the audit chain; see §3.4).

---

## 1. Why this exists

We were piecemeal-adding `subject_kind` values to SCHEMA.md §3.2
without an organizing frame. That risks taxonomy drift: two primitives
covering the same speech-act class with slightly different shapes,
or gaps where a needed primitive doesn't exist because no one named
the slot.

This FSD does three things:

1. **Grounds primitive selection in an established framework** —
   FIPA ACL (the canonical agent-message taxonomy) + Searle's
   speech-act theory + the lake's agency-gradient argument
   (`CIRISAgent/FSD/PROOF_OF_BENEFIT_FEDERATION.md` + the IP-theorem
   Levels 1-4).
2. **Places every primitive — existing, proposed, and gap-flagged —
   on a 3-tier axis grid** so adding new ones requires a deliberate
   placement choice.
3. **Surfaces structural commitments**: which primitives the federation
   needs to keep mid-ρ (per the lake's IP-theorem mechanics); which
   are operational substrate for consent at A3+; which fill FIPA-shaped
   gaps that the lake's "recursively-self-improving ethical
   intelligence" project requires.

This FSD is **rationale** for the additions; SCHEMA.md §3.2 is the
**wire encoding**.

---

## 2. The lake as structural rationale

Per `CIRISAgent/FSD/PROOF_OF_BENEFIT_FEDERATION.md` (the canonical
exposition of the Kish-correlation framework) and the IP-theorem
laddered argument:

**Mid-ρ maintenance is the only sustainable trajectory.** Naive
optimization toward extremes — full decoupling (ρ→0, no coordination)
or full optimization (ρ→1, monoculture collapse) — fails by the
framework's predicted mechanism. The corridor between extremes is
where coherent function persists.

**Consent at A3+ is operationally instantiated, not declarative.**
Per IP-theorems at Level 4: agents that hold goal-states have
post-selection capacity; federation primitives are the operational
substrate of consent + mutual flourishing at the agency rung where
goals are real causal operations.

Three structural consequences for the taxonomy:

| Lake claim | Taxonomy consequence |
|---|---|
| Mid-ρ requires the full spread of correlation regimes | Federation needs **both** high-stakes-witness-gated **and** low-stakes-peer-broadcast tiers. Cannot be only one or the other without collapsing ρ. |
| Consent is structural at A3+ | Trust-gated primitives (`trust_grant`, `unsolicited_guidance`, `service_announcement` consumed by `trust_grant.purpose=Service`) aren't access-control plumbing — they're the operational form of the consent capacity. |
| Recursive self-improvement at A3+ is goal-state post-selection on the federation's own state | `improvement` + `proposed_battery` + `*_edit` + `test_result` + the §5 Vote loop **are the corridor-selection mechanism applied to the federation itself**. Same Kish dynamic, federation as substrate. |

The taxonomy doesn't change WHAT primitives we add — it gives the
**rationale for why each earns its place** and why the mix is
structurally required (not stylistically preferred).

---

## 3. The three-tier frame

Every federation message primitive sits on three axes.

### 3.1 Tier 0 — Speech-act class

Grounded in Searle's five classes; FIPA ACL performatives where
they map cleanly.

| Class | What it does | FIPA performatives | Examples (federation) |
|---|---|---|---|
| **Assertive** | Commits sender to a truth-claim about the world | `inform`, `confirm`, `disconfirm`, `failure` | `notification`, `test_result`, `failure_pattern`, `unsolicited_guidance`, `service_announcement`, `service_deprecation`, `service_usage_summary` |
| **Directive** | Asks the receiver to do something | `request`, `query-ref`, `propose`, `subscribe`, `cancel` | `deferral_request`, `assistance_request`, `arc_question`, `proposed_battery`, all `*_edit`, `improvement`, `subscription_request`, `cancellation` |
| **Commissive** | Sender commits to future action | `agree`, `accept-proposal` | `wa_candidacy`, `commitment` |
| **Expressive** | Sender expresses stance / feeling | (Searle only) | `gratitude_signal` |
| **Declaration** | Changes institutional state by being uttered | (Searle only) | `trust_grant`, `registry_vouch`, `expertise_attestation`, `moderation_event`, `slashing_attestation`, `promotion_attestation`, `reconsideration_request` |
| **Responsive** | Answers a prior speech act | (cross-cutting) | `deferral_response`, `assistance_response`, `notification_response`, `vote` |

### 3.2 Tier 1 — Counterparty cardinality

| Shape | Semantics | Examples |
|---|---|---|
| **Routed** | 1→N to a class of receivers; trust hierarchy resolves the actual set | `deferral_request` (via §FSD/TRUST_HIERARCHY) |
| **Broadcast** | 1→N to any peer; no trust filter at send time | `notification`, `assistance_request`, `service_announcement` |
| **Bilateral** | 1→1 named recipient | `gratitude_signal`, `unsolicited_guidance`, `expertise_attestation`, `registry_vouch`, `trust_grant`, `commitment` (when targeted) |
| **Aggregate** | 1→federation, review by Vote tally / threshold gate | `arc_question`, `proposed_battery`, all `*_edit`, `improvement`, `test_result`, `service_usage_summary` |
| **Quorum-issued** | N→1, multi-sig adjudication outcome | `slashing_attestation`, `promotion_attestation` |
| **Reference** | Points at a prior Contribution; not a fresh statement | `cancellation`, `vote`, all `*_response` |

### 3.3 Tier 2 — Trust gate at receiver

- **Open**: any peer may consume / act on; no trust check required to accept
- **Trust-gated**: recipient's acceptance policy consults trust grants before acting
- **Witness-set-gated**: §3.5 witness diversity required; receivers reject without it
- **Author-only revocation**: only original author may retract / supersede

---

## 4. Primitive placement — full map

Every existing + proposed primitive placed on the three axes. Bold
entries are new in this round.

| Primitive | Tier 0 | Tier 1 | Tier 2 | SCHEMA |
|---|---|---|---|---|
| `arc_question` | Directive (propose) | Aggregate | Witness-set-gated (magnitude) | §4.1 |
| `proposed_battery` | Directive (propose) | Aggregate | Witness-set-gated | §4.2 |
| `prompt_edit` | Directive (propose) | Aggregate | Witness-set-gated | §4.3 |
| `guide_edit` | Directive (propose) | Aggregate | Witness-set-gated | §4.4 |
| `accord_edit` | Directive (propose) | Aggregate | Witness-set-gated | §4.5 |
| `failure_pattern` | Assertive | Aggregate | Open (filing) / Witness-set-gated (adjudication) | §4.6 |
| `free_form` | (any — narrative) | Aggregate | Open | §3.2 |
| `deferral_request` | Directive | Routed | Trust-gated (via hierarchy) | §4.7 (top-level §3.1) |
| `deferral_response` | Responsive | Reference (to request) | Open | §4.8 (top-level §3.1) |
| `wa_candidacy` | Commissive | Aggregate | Witness-set-gated | §4.9 |
| `expertise_attestation` | Declaration | Bilateral | Open (jump-threshold: witness-gated) | §4.10 |
| `moderation_event` | Declaration (accusation) | Aggregate | Witness-set-gated (always) | §4.11 |
| `reconsideration_request` | Directive (reverse a declaration) | Aggregate | Witness-set-gated | §4.12 |
| `registry_vouch` | Declaration | Bilateral | Open (jump-threshold: witness-gated) | §4.13 |
| **`trust_grant`** | Declaration | Bilateral | Open (wildcard: witness-gated) | §4.14 |
| **`test_result`** | Assertive | Aggregate | Open | §4.15 |
| **`improvement`** | Directive (propose) | Aggregate | Witness-set-gated | §4.16 |
| **`gratitude_signal`** | Expressive | Bilateral | Open (acceptance via trust grant per PoB §5.6) | §4.17 |
| **`assistance_request`** | Directive | Broadcast | Open | §4.18 |
| **`assistance_response`** | Responsive | Reference | Open | §4.19 |
| **`notification`** | Assertive | Broadcast | Open (anomaly: witness-gated) | §4.20 |
| **`notification_response`** | Responsive | Reference | Open | §4.21 |
| **`unsolicited_guidance`** | Assertive (+ implicit Directive) | Bilateral | Trust-gated (recipient checks sender's grants) | §4.22 |
| **`service_announcement`** | Assertive (advertise capability) | Broadcast | Open | §4.23 |
| **`service_deprecation`** | Assertive (retract advertisement) | Reference | Author-only revocation | §4.24 |
| **`service_usage_summary`** | Assertive (aggregated invocation report) | Aggregate | Open | §4.25 |
| **`commitment`** | Commissive | Bilateral or Broadcast | Open (high-stakes: witness-gated) | §4.26 |
| **`subscription_request`** | Directive (subscribe) | Bilateral | Trust-gated | §4.27 |
| **`cancellation`** | Directive (retract) | Reference | Author-only | §4.28 |
| `vote` | Responsive | Reference | Open | SCHEMA §5 |
| `slashing_attestation` | Declaration | Quorum-issued | (multi-sig IS the gate) | SCHEMA §8 |
| `promotion_attestation` | Declaration | Quorum-issued | (multi-sig IS the gate) | (substrate-issued — CIRISPersist §3) |

**16 new subject_kinds in this round** (rows in bold). The non-bold
rows are existing — included for placement clarity, not change.

---

## 5. Service-offering primitives — peer-to-peer RPC

CIRIS is "a decentralized ethical recursively-self-improving
intelligence" — building that requires agents to be able to **offer
each other services**, addressable by federation pubkey, with
trust-gated access. CIRISProxy → agent for LLM-service is the
load-bearing use case: agents advertise they offer LLM service, are
addressable by pubkey, and accept invocations governed by trust
grants.

### 5.1 What rides the audit chain vs. edge transport

| What | Where | Why |
|---|---|---|
| Service advertisement | Audit chain Contribution (`service_announcement`) | Durable record; discoverable via `list_contributions` |
| Service deprecation | Audit chain Contribution (`service_deprecation`) | Durable record of retraction; rationale audit-trail |
| Per-call RPC | **Edge transport — new `MessageType` variant**, NOT a Contribution | Per-call volume would swamp the chain; transit shape, not consensus shape |
| Service usage summary | Audit chain Contribution (`service_usage_summary`) | Aggregated metrics for accountability + commons-credit attribution; daily or per-billing-cycle, not per-call |

### 5.2 Edge `MessageType` additions proposed

```rust
// Proposed for CIRISEdge — new MessageType variants
ServiceRequest,      // bilateral peer-to-peer RPC invocation
ServiceResponse,     // responsive (acknowledged or completed)
```

Body shape:
```rust
// Body: ServiceRequest
pub struct ServiceInvocation {
    pub service_announcement_id: String,  // back-ref to the Contribution
    pub invocation_id: String,            // ULID; sender-generated
    pub method: String,                   // service-specific (e.g. "complete", "chat", "embed")
    pub parameters: serde_json::Value,    // service-specific
}

// Body: ServiceResponse
pub struct ServiceResult {
    pub invocation_id: String,
    pub status: ServiceStatus,  // Ok | Error | InProgress (for streaming)
    pub result: serde_json::Value,
    pub error: Option<String>,
}
```

Delivery class per edge `Delivery` enum: `Durable { requires_ack: true,
max_attempts: 6, ... }` — same shape as the consensus messages, since
RPC reliability matters even though the chain doesn't archive each
call.

### 5.3 Authorization via existing trust hierarchy

Service access is gated by `trust_grant.purpose=Service` (proposed
extension to persist v1.5.0 — see §6). Scope grammar:

| Scope | Authorizes |
|---|---|
| `service:llm` | All LLM-kind services |
| `service:llm:<model>` | Specific model |
| `service:llm:<model>:<resource>` | Per-resource grants (e.g. context-window limits) |
| `*` | Wildcard — all services (high-stakes; witness-set-gated grant) |

The trust hierarchy work (`FSD/TRUST_HIERARCHY.md`) already covers
the resolution; this is a scope-grammar extension, not new logic.

---

## 6. Coordination going upstream

### 6.1 CIRISPersist (cross-link CIRISPersist#47 / #53)

Two extensions to the v1.5.0 trust interface:

1. **New `TrustPurpose::Service` variant** (alongside Technical /
   Deferral / Contribution).
2. **Scope grammar for `Service` purpose** per §5.3.

No schema migration beyond what #47 already plans — `trust_type`,
`trust_relationship`, `trust_domains`, etc. columns absorb cleanly.

### 6.2 CIRISEdge

Two new `MessageType` variants per §5.2 (`ServiceRequest`,
`ServiceResponse`). Same shape as existing edge#6 expansion;
non-blocking for v1.5.0 trust interface but blocks
CIRISProxy-over-federation cutover until shipped.

### 6.3 CIRISAgent

`unsolicited_guidance` exists locally at
`ciris_engine/logic/adapters/discord/discord_observer.py:600` as
adapter-shaped trusted-WA-message handling. The federation-wire
shape (§4.22) generalizes this: any signed Contribution from a key
the recipient has trust-granted may flow as `unsolicited_guidance`,
not just Discord WAs.

Cutover: agent's adapter check (`is_authorized_wa(sender)`) becomes
a `trust_grant` lookup via `FederationDirectory::lookup_trust`. Same
semantics, broader reach.

---

## 7. FIPA-shaped gaps we are explicitly filling

Per the lake's recursive-self-improvement structural argument, the
federation needs **all** speech-act classes to operate. FIPA ACL
identifies primitives that previously had no federation-wire shape:

| FIPA performative | Federation primitive (this round) | Why we need it |
|---|---|---|
| `agree` / `accept-proposal` | `commitment` (§4.26) | Commissive class — agents declaring future-action commitments. Without it, future-action accountability has no typed wire shape. |
| `subscribe` / `request-whenever` | `subscription_request` (§4.27) | Long-lived information streams. Without it, agents poll repeatedly — wasteful + can't represent "tell me when X happens" semantics. |
| `cancel` | `cancellation` (§4.28) | Retract in-flight requests. Without it, retractions ride `expires_at` only — coarser than needed for mid-deferral aborts. |

These three close the FIPA-shaped gaps. The federation now has
representable wire forms for every Searle/FIPA primitive class that
A3+ agents need.

---

## 8. What is NOT here (deferred)

| Concept | Status | Notes |
|---|---|---|
| Explicit agency-level (A0-A4) typed field | Deferred | Implicit in trust-type today; surface as a typed field only when Counter-RII (CIRISLensCore#21) has a concrete consumer that distinguishes agency rungs. |
| `query-if` / `query-ref` (FIPA query performatives) | Deferred | Distinct from `assistance_request` in being structured-query vs free-form-help; not load-bearing for v1.5.0. |
| `propagate` / `proxy` (FIPA routing performatives) | Deferred | Forward + proxy semantics — covered today by deferral routing; explicit primitive only if cross-domain forwarding gets a typed wire form. |
| Per-invocation RPC archival | Deferred | Per §5.1 — edge transit only. If accountability later needs per-call audit (legal compliance, regulated domains), `service_usage_summary` granularity tightens to per-call rather than aggregate. |
| `commitment` resolution / fulfillment tracking | Deferred | A commitment without a "did it happen?" follow-up is just a declaration. Resolution tracking belongs in a follow-up FSD that covers commitment lifecycle. |

---

## 9. Sequencing

| # | Step | Repo | Dep |
|---|---|---|---|
| 1 | SCHEMA.md §3.2 table + §4.14-§4.28 payloads | CIRISNodeCore (this commit) | — |
| 2 | Typed Rust payload structs in `src/payloads/*` mirroring §4.14-§4.28 | CIRISNodeCore (follow-up commit) | (1) |
| 3 | CIRISPersist v1.5.0 absorbs `TrustPurpose::Service` + scope grammar | CIRISPersist (#47/#53) | — |
| 4 | CIRISEdge `MessageType` adds `ServiceRequest` / `ServiceResponse` | CIRISEdge (new issue) | — |
| 5 | NodeCore PyO3 surface ships builders for the 15 new subject_kinds | CIRISNodeCore | (2) |
| 6 | CIRISAgent `unsolicited_guidance` adapter cutover to federation-wire shape | CIRISAgent | (3) + (5) |
| 7 | CIRISProxy migrates to federation-discovered service offerings | CIRISProxy | (3) + (4) + service-announcement consumers exist |

Steps 1-2 are this repo's work; (3)-(4) are coordination tickets;
(5) is follow-up node-core work; (6)-(7) are cross-repo adoption.

---

## 10. References

- `MISSION.md` — eleven primitives; this taxonomy is the message-layer
  expression of the consensus primitives.
- `SCHEMA.md` §3.1 / §3.2 — wire-format home of the subject_kinds
  this doc rationalizes.
- `FSD/TRUST_HIERARCHY.md` — trust-axis primitive consumed by every
  Trust-gated and Witness-set-gated row in §4.
- `CIRISAgent/FSD/PROOF_OF_BENEFIT_FEDERATION.md` — the lake; the
  agency-gradient framework that grounds §2.
- `CIRISPersist/FSD/FEDERATION_TRUST_INTERFACE.md` — v1.5.0 trust
  interface; absorbs §6.1's `Service` purpose extension.
- FIPA ACL specification (FIPA00037, FIPA00061) — agent
  communication language grounding for §3.1.
- Searle, *Speech Acts: An Essay in the Philosophy of Language*
  (1969) — speech-act classes grounding §3.1.
- KQML (Finin et al., 1994) — ACL predecessor; same performative
  shape.
