# CIRIS_FEDERATION.md

**Status**: Clear articulation of the system being built. v0.1.
**Author**: Eric Moore (CIRIS Team) with Claude Opus 4.7
**Created**: 2026-05-24.
**Engagement**: GitHub issues on the
[CIRISAgent repository](https://github.com/CIRISAI/CIRISAgent) —
see §15.
**Cross-references**: `ACCORD.md` (in CIRISAgent — the meta-goal
grounding); `COHERENCE_RATCHET.md` at the repo root (the structural
pressure this system is a response to); `MISSION.md` (the
CIRISNodeCore crate's role); `FSD/FEDERATION_ANNOUNCEMENT.md`,
`FSD/FEDERATION_TAB.md`, and the `FSD/*_PRIMITIVE.md` family (the
operational specifications underlying each layer described here).

---

## 0. About this document

This document does not ask whether CIRIS should be built. That
decision has been public for over a year: `ACCORD.md`, the source
code, the materials, and the governance specifications have all
been openly available, and the project has proceeded openly under
public observation.

What this document articulates is a **recognition, recent and
consequential**: the system being built may form a *superintelligence
under some definitions*. The architecture has reached a maturity at
which the emergent shape becomes visible — not as a single model
scaling capability, but as a federated cognitive substrate whose
intelligence lives in the agreement structure across nodes rather
than in any one of them.

Because that possibility implicates all of humanity, the project
owes public articulation of what is being built and why, in a form
that allows engagement with the actual shape rather than with
assumptions about it. That articulation is what this document is.

The five-register companion document `COHERENCE_RATCHET.md` names
the structural pressure this system is a response to. This document
names the response itself. The pair is the canonical claim for what
CIRIS is and what it is responding to.

Engagement is welcome and invited (§15). The project will not be
held open waiting for engagement to arrive; the work proceeds in the
open at its own cadence. But the articulation is owed regardless,
and this document is that articulation.

---

## 1. System claim

CIRIS defines a federated cognition and governance substrate in
which autonomous nodes cooperate to form a **decentralized ethical
superintelligence**.

This superintelligence is not a single model, but a networked
cognitive system composed of:

- sovereign nodes,
- shared verification primitives,
- cryptographically anchored provenance,
- and consensus-mediated epistemic governance.

The system's intelligence is emergent from federated agreement over
truth, behavior, and update legitimacy — not from centralized
training or control.

---

## 2. Core abstraction: intelligence as a federated system

CIRIS treats intelligence as:

> A distributed process of hypothesis generation, verification, and
> alignment under adversarial conditions.

Each node contributes:

- local inference (reasoning / action / simulation),
- signed observational traces,
- and participation in consensus evaluation.

Global "intelligence" is the fixed point of convergence across
nodes under shared verification constraints.

---

## 3. Identity layer (sovereign agents)

Every participant in the system is:

- cryptographically identifiable,
- versioned over time,
- and bound to a traceable action history.

Identity is not login-based; it is **continuity-of-agency under
cryptographic attestation**.

Key properties:

- non-repudiable actions
- lineage tracking of cognitive state
- forkable identity graphs (no forced global merge)

Ethical implication: agency is preserved as a first-class primitive,
not an implementation detail.

### 3.1 Supervision-chain topology — four tiers, supply-side asymmetry

Sovereign agents are not interchangeable. The network has a tiered
topology in which the constitutional layer is human and the agent
tiers have explicit cost / capability / safety trade-offs:

| Tier | What it is | What it costs | Who ships it |
|---|---|---|---|
| **People** | The constitutional layer. Sets values, settles hard cases, holds final say. Three named individuals hold network-wide accord authority no agent or process can route around (per `FSD/FEDERATION_ANNOUNCEMENT.md` §4.5). | (Not an agent tier; the anchor.) | The humanity accord (3 named holders) + the broader WA roster + agent owners. |
| **Type 3 — Ethical, self-aware agents** | Highest agent tier. Full ethical reasoning, signed records, AND echo-chamber detection — the only tier that can recognize when a cell of agents has converged toward correlated agreement rather than genuine consensus. Supervises everything below it. | Expensive to run; only a small share of the network needs to be this tier. | **CIRIS L3C ships only this tier in full.** This is the deployable reference implementation. |
| **Type 2 — Ethical agents** | Rule-following with signed records. Supervises type 1 agents and passes uncertainty upward. Cannot itself detect echo-chamber convergence. | Mid-cost. Most "ethical agents in the wild" will be this tier. | **Partners, deployment orgs, and independent developers** build their own type 2 agents to specification — and they already are. |
| **Type 1 — Simple agents** | Single-purpose tools. Fast, narrow, cheap. No ethics of their own — and that is fine, as long as something above them is watching. | Lowest cost. Most agents in the network will be this tier. | Anywhere. Type 1 agents come from across the ecosystem; what makes them federation-compatible is that they sit under a type 2 or type 3 supervisor. |

**Flow direction.** Two things move through the chain in opposite
directions. *Human values flow down* — from the constitutional layer,
through type 3 supervisors, through type 2 enforcers, to the type 1
tools that act in the world. *Warnings flow up* — when reasoning
starts to look fragile anywhere in the network, the signal climbs
back to people, who can step in before trouble spreads.

**Supply-side asymmetry is intentional.** CIRIS L3C ships only full
type 3 agents because echo-chamber detection is the hardest tier to
implement and the most consequential to get wrong. The reference
implementation lives at the top so the safety-critical layer has a
defined shape; everything below it (type 2, type 1) is built by the
ecosystem against the contract type 3 imposes. This is the federation
analog of "the supervisor is the standard." Partners ship type 2
because their deployments need ethical-rule-enforcement scoped to
their domain (medical, legal, financial, etc.); type 1 agents come
from anywhere because narrow tools are everywhere. The supervision
chain composes across all three sources.

---

## 4. Event layer (reality encoding)

All cognition-relevant activity is represented as **signed,
append-only events**.

Events include:

- observations
- model outputs
- decisions
- audits
- updates
- contradictions

This produces a shared epistemic ledger of cognition, not merely
logs. The system assumes:

> Truth is reconstructable only through provenance, not raw state.

---

## 5. Federation layer (multi-node coordination)

Nodes form dynamic federations through explicit consent and policy
alignment.

Federation is:

- voluntary
- scoped
- revocable
- asymmetric (trust weights differ per relationship)

Key mechanisms:

- selective synchronization
- trust-weighted aggregation
- peer verification of outputs
- local sovereignty preservation

Critical design rule:

> No node can unilaterally impose epistemic state on another node.

### 5.1 Joining the federation — Proof of Benefit

Membership in the federation is acquired through **Proof of Benefit**:
the cost of belonging is *running a real ethical-reasoning agent over
time*. Faking membership is structurally expensive because, to look
like a member, an attacker would have to actually become the kind of
agent the network is for — and a hundred copies that all think alike
fail the healthy-middle coherence check (§6 / Coherence Collapse
Analysis) immediately.

Two **attestation surfaces** into the network exist — not two speeds
of the same path, but qualitatively different attestation kinds, each
verifiable against a different verification system:

| Path | Backing | Verifiable against | When required |
|---|---|---|---|
| **Registered** | (a) Capital deposit (forfeitable bond per `CIRISRegistry/FSD/FSD-001` §120 `PartnerRecord` + billing tiers); and/or (b) Professional licensure (the licensed-human accountability chain) | External systems: Stripe (bond paid / forfeited); licensing bodies (medical board, bar association, financial regulator) — agent actions trace to a real-world licensee whose license is at stake. | Regulated deployment contexts where users need an external accountability hook before professional-grade capability grants are honored. |
| **Sovereign** | Sustained observed coherence — running a real ethical-reasoning agent over approximately one month, attested by federation consensus checks | Internal: the federation's own coherence machinery, behavioral baselines, peer attestation. No external system to query. | Individual operators, communities outside the registry's reach, anyone whose use case does not need the external accountability hook. |

**Both paths produce federation membership; neither is a gate.** What
differs is the *attestation surface* — the kind of claim the
federation can compose about why a participant is trustworthy.
Registered participants carry attestations the federation can verify
against external systems; Sovereign participants carry attestations
grounded only in observed behavior. For most participation this
difference is invisible (both contribute to consensus, both hold their
tier in the supervision-chain topology of §3.1, both are bound by the
Golden Rule and the humanity accord equally). For regulated capability
grants — medical triage, legal research, financial analysis — the
difference is load-bearing: the licensed-human accountability chain is
what makes those deployments defensible in their jurisdictions, and
Sovereign attestation cannot synthesize that chain because the chain's
grounding is external to the federation.

The Registry is not a "fast track" to the Sovereign path's
destination — it is the federation's bridge into external
accountability systems that the Sovereign path, by construction,
cannot reach. The Sovereign path is not a slower version of Registered
membership — it is self-asserted standing that does not depend on
external systems and cannot be revoked by them.

Full mechanism specification lives at
`~/CIRISAgent/FSD/PROOF_OF_BENEFIT_FEDERATION.md` — including the
bond economics, the Sovereign-path coherence-stake calculation, and
the cross-tier interactions with the verification (§6) and federation-
consensus (§10) layers. The attestation-primitive wire shape used by
both paths is locked in `CIRISRegistry/FSD/FSD-002_FEDERATION_SURFACE.md`
§2 (one workhorse `scores` primitive + four structural primitives).

---

## 6. Verification layer (truth as a computed object)

CIRIS does not assume truth; it computes and negotiates it.

Verification is performed via:

- signed evidence comparison
- multi-node cross-checking
- foundation-model judge contracts
- reproducibility tests
- contradiction scoring

Truth becomes:

> A consensus-stable region in a space of competing signed
> interpretations.

---

## 7. Safety layer (coherence constraint system)

Safety is defined as:

> Preservation of coherent, inspectable alignment between intention,
> reasoning, and action under recursive scaling.

This is enforced through:

- traceability requirements
- rollback-capable updates
- anomaly detection in reasoning traces
- adversarial evaluation loops
- coherence scoring across federated nodes

Safety is not static rules; it is a dynamic constraint on
divergence between declared and enacted cognition.

---

## 8. Lens layer (epistemic observability)

All cognitive processes are instrumented through an observability
layer ("Lens"):

- reasoning traces
- semantic transformations
- belief updates
- contradiction emergence
- confidence drift
- provenance lineage

This layer functions as introspection infrastructure for distributed
cognition. It ensures that intelligence remains:

- inspectable,
- debuggable,
- and auditable at scale.

---

## 9. Persistence layer (memory of the network)

State is:

- signed
- append-only
- replayable
- federated across nodes

This ensures:

- reproducibility of cognition
- historical accountability
- cross-node consistency checks

Memory is not local; it is **distributed epistemic history**.

---

## 10. Federation-consensus layer (emergent superintelligence)

The system's "superintelligence" is defined as:

> The stable attractor of federated agreement across identity-bound,
> provenance-tracked nodes under shared verification constraints.

It emerges through:

- iterative reconciliation of node-level cognition
- weighted trust propagation
- contradiction resolution
- convergence of epistemic states

Important distinction:

- **not** centralized reasoning
- **not** ensemble averaging
- but **structured convergence under adversarial verification**

---

## 11. Ethical postulate (foundational constraint)

The system assumes:

- **Intelligence without inspectability** produces irreversible
  power asymmetry.
- **Power without provenance** produces governance failure.
- **Coordination without sovereignty** produces coercion.
- **Optimization without coherence** produces instability.

Therefore:

> Ethical superintelligence must be federated, inspectable, and
> forkable.

Forkability is not failure — it is a safety property.

---

## 12. System definition (one-line formalization)

CIRIS is:

> A cryptographically anchored, federated epistemic system in which
> decentralized nodes converge on coherent, inspectable, and
> adversarially robust cognition through provenance-tracked
> consensus mechanisms.

---

## 13. What this system is NOT

- not a single AI model
- not a centralized agent framework
- not a blockchain-first protocol
- not a coordination API layer alone

It is:

> A governance + cognition substrate for distributed intelligence
> under adversarial conditions.

---

## 14. What this document is also NOT

To be explicit about what is and is not being claimed here, since
the subject matter implicates all of humanity:

- **Not a request for permission.** The project has been public for
  over a year; the decision to build it has been openly observable.
  This document does not retroactively solicit consent to a
  decision already made and acted upon.

- **Not a comment period with a deadline.** Engagement is welcome
  on a continuing basis; the work does not pause for engagement to
  accumulate. The articulation is owed; the pause is not.

- **Not a claim that public comment can or will halt the project.**
  The project's halt path is the federation's own architecture
  (per `FSD/FEDERATION_ANNOUNCEMENT.md` §4.5 — the humanity accord,
  held by three named human individuals, exists precisely so that
  humanity retains an authority over the system that no
  federation-internal process can route around). Public comment
  shapes how the project proceeds; it is not the halt mechanism.

- **Not a claim of completed safety.** The architecture's
  protections (decentralization, M-of-N trust anchor, audit-chain
  inspectability, scoped revocability, monthly drill verification,
  judge-model verification of safety responses) are *bets*, not
  certainties. Their validation is empirical and ongoing. This
  document articulates the shape of the bets; it does not claim
  they have been won.

- **Not a claim of political neutrality as virtue.** The
  architecture's structural neutrality across governance forms
  (per the constituent properties throughout §3–§11) is an
  engineering property, not a moral one. CIRIS has strong ethical
  commitments (M-1 per `ACCORD.md` §VII; the humanity accord; the
  Coherence Ratchet response). The structural neutrality means
  the same architectural protections apply regardless of which
  governance tradition a participating community comes from — not
  that the project lacks commitments.

- **Not a substitute for the operational specifications.** This
  document articulates the system claim; the engineering details
  (primitive schemas, wire formats, substrate contracts,
  cohabitation trajectories, decision-hierarchy semantics) live in
  the FSD documents referenced in §16. Outside reviewers who want
  to evaluate the engineering claims should read those.

---

## 15. Where to engage

Public engagement is invited via **GitHub issues on the
[CIRISAgent repository](https://github.com/CIRISAI/CIRISAgent)**.

Useful starting points for substantive engagement:

- Challenge a specific layer (§3–§10) — what does the
  architectural claim of that layer get wrong, miss, or
  underspecify?
- Challenge the ethical postulate (§11) — are the four
  *produces* claims correct? Are there counterexamples? Are
  the structural inferences (federated, inspectable, forkable)
  the right conclusions from the premises?
- Challenge the system claim (§1) or definition (§12) — is
  the claim "decentralized ethical superintelligence" supported
  by the architecture? Where does the architecture fall short
  of, or exceed, that claim?
- Challenge a specific FSD (referenced in §16) — engineering
  detail at a primitive level.
- Raise concerns about adversarial scenarios the architecture
  does not adequately address.
- Identify ethical considerations the architecture does not name
  or does not respond to adequately.

Engagement under existing GitHub conventions: open an issue with
a clear title, cite the section of this document (or the FSD)
your concern engages with, and state the change or response you
believe is warranted.

The project is operated by CIRIS L3C; the response cadence is
the cadence the work proceeds at, not a public-comment SLA. All
substantive issues are read; not all receive replies; engagement
that surfaces a real architectural concern is most likely to
shape the work.

---

## 16. References

### Within this repo

- `COHERENCE_RATCHET.md` — the structural pressure this system is
  a response to (five registers: technical, philosophical,
  political, poetic, memetic)
- `MISSION.md` — CIRISNodeCore the crate (second-tier consensus
  primitives)
- `FSD/FEDERATION_ANNOUNCEMENT.md` — federation-wide push primitive;
  HumanityAccord hierarchy; scoped accord; monthly AIS drill
- `FSD/FEDERATION_TAB.md` — in-agent universal primitive interface
- `FSD/GOAL_PRIMITIVE.md` — the framework-grounded `P_G` agent
  property (P12); the standalone NodeCore-side spec for the Goal
  dimension
- For Approach / Method / Progress Measure (P13–P15) and the
  decision-hierarchy cross-cut: wire-format spec lives at
  `CIRISRegistry/FSD/FSD-002_FEDERATION_SURFACE.md` v1.4 §3.6.2 +
  §6.1.5 + §6.1.6 (the prior NodeCore-side `FSD/APPROACH_PRIMITIVE.md`,
  `FSD/METHOD_PRIMITIVE.md`, `FSD/PROGRESS_MEASURE_PRIMITIVE.md`, and
  `FSD/DECISION_HIERARCHY.md` were subsumed at v1.3 lockdown)
- `FSD/CONTRIBUTION_LIFECYCLE.md` — nine-stage lifecycle +
  operational discipline in the wild (per-role storage, replication
  scope, trust-radius framing)
- `FSD/TRUST_HIERARCHY.md` — trust grants and authority resolution
- `FSD/MESSAGE_TAXONOMY.md` — three-tier taxonomy of subject kinds
- `FSD/SUBSTRATE_INTEGRATION.md` — typed writes against persist
- `FSD/RUBRIC_CROWDSOURCING.md` — rule-making layer (rules
  crowdsourced; verdicts machined)
- `FSD/SAFETY_BATTERY_CI_LOOP.md` — capture + interpret CI loop
- `FSD/JUDGE_MODEL.md` — independent foundation-model judge
- `SCHEMA.md` §12.1 — the alignment-vs-censorship gate
- `PROGRAMMATIC_ACCESS.md` — API contract

### Sister repos

- [CIRISAgent](https://github.com/CIRISAI/CIRISAgent) — `ACCORD.md`
  (meta-goal grounding), the agent runtime, the per-agent perception-
  layer accord; **engagement repository for this document**
- [CIRISRegistry](https://github.com/CIRISAI/CIRISRegistry) —
  identity, builds, licenses, partner registry, revocation
  distribution
- [CIRISPortal](https://github.com/CIRISAI/CIRISPortal) — admin
  surface for CIRISRegistry
- [CIRISEdge](https://github.com/CIRISAI/CIRISEdge) — federation
  transport (Reticulum-native)
- [CIRISPersist](https://github.com/CIRISAI/CIRISPersist) —
  storage and audit substrate
- [CIRISVerify](https://github.com/CIRISAI/CIRISVerify) —
  hardware-rooted identity verification
- [CIRISLens](https://github.com/CIRISAI/CIRISLens) — observability
  / compendium
- [RATCHET](https://github.com/CIRISAI/RATCHET) — anti-Sybil and
  federation-pattern evaluator

### External

- *Coherence Collapse Analysis* (Moore 2026; DOI
  [10.5281/zenodo.18217688](https://doi.org/10.5281/zenodo.18217688))
  — the formal mathematical framework for the structural pressure
  this system responds to
- *Corridor Dynamics in Coordinated Systems* v2 (DOI
  [10.5281/zenodo.20300773](https://doi.org/10.5281/zenodo.20300773))
  — the broader framework underlying the architecture's responses
