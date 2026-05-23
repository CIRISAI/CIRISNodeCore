# FSD: Decision Hierarchy — the cross-cutting DAG composing Goal / Approach / Method / Progress-Measure primitives into federated decision flow

**Status**: Proposed (v0.1.0-dev). Cross-cutting spec that
integrates `GOAL_PRIMITIVE.md`, `APPROACH_PRIMITIVE.md`,
`METHOD_PRIMITIVE.md`, `PROGRESS_MEASURE_PRIMITIVE.md`.
**Author**: Eric Moore (CIRIS Team) with Claude Opus 4.7
**Created**: 2026-05-22
**Risk**: Architectural. Pins the referential integrity rules,
cross-level consensus flow, and cross-level corridor-occupation
reading that make the four sibling primitives compose into a
coherent decision hierarchy. Without this spec, the four primitives
can exist with orphan references, cycles, mismatched substrate
typings, and incoherent decision traversal. Once committed,
federation health observables and `Moderation` interaction across
levels inherit from this contract.

**Cross-references:** all four sibling FSDs (`GOAL_PRIMITIVE.md`,
`APPROACH_PRIMITIVE.md`, `METHOD_PRIMITIVE.md`,
`PROGRESS_MEASURE_PRIMITIVE.md`); `coherence-ratchet/papers/
Corridor Dynamics.tex` v2 (DOI `10.5281/zenodo.20300773`)
§sec:open-research (post-F-11 sequential per-rung architecture, no
central authoritative scorer — the architectural shape the cross-
level flow must respect), §sec:tsvf-ubuntu (composition rule —
the framework's stance on what level-composition can and cannot
do); `MISSION.md` v1.0 §2 (the eleven primitives carrying cross-
level flow); `FSD/SUBSTRATE_INTEGRATION.md` (typed-writes pattern
for cross-level reference resolution); `RATCHET/AGENT_FSDs/
PROOF_OF_BENEFIT_FEDERATION.md` (audit-chain reading across
levels).

---

## 0. Why this exists

Four sibling FSDs specify each decision level. Each, in isolation,
could exist with mismatched references, orphan declarations,
referential cycles, or substrate-typing violations. This FSD
specifies the cross-cutting structure that prevents that:

- The **DAG**: which primitive references which.
- **Referential integrity rules**: what is required to land in the
  audit chain.
- **Cross-level consensus flow**: how decisions traverse the
  hierarchy from Goal to Progress Measure and back.
- **Cross-level corridor-occupation reading**: when the federation
  is "in corridor" not just at one level, but coherently across
  the four levels.
- **Cycle/orphan detection**: what the federation does when
  references break.

Without this cross-cutting spec, the four primitive FSDs could
ship independently and produce an incoherent federation. With it,
the federation gains a checkable decision hierarchy.

---

## 1. WHY — M-1 alignment

### 1.1 What the Decision Hierarchy does for M-1

**Meta-Goal M-1**: *promote sustainable adaptive coherence — the
living conditions under which diverse sentient beings may pursue
their own flourishing in justice and wonder*.

The cross-cutting hierarchy serves M-1 in four named ways:

1. **Coherent decision traversal — anti-incoherence.** A federation
   whose Approaches reference Goals that exist, whose Methods
   reference Approaches that exist, whose Progress Measures
   reference real referents, is structurally coherent. Orphans and
   cycles indicate decision-flow breakdown; the federation can
   detect and address.
2. **Post-F-11 architectural alignment.** The v2 paper's
   §sec:open-research re-grounded the universal-scale tier on
   sequential per-rung structure with no central authoritative
   scorer. The Decision Hierarchy mirrors this: every peer can
   verify the DAG locally on their slice of the audit chain;
   federation cross-validates via existing P7/P10 primitives. The
   stance and the architecture are the same.
3. **Multi-corridor cross-level reading.** The framework's corridor
   structure (Piece 3) and the multi-scale ρ_goals reading
   (§sec:tsvf-ubuntu) generalize across decision levels: a
   federation can be in corridor at the Goal level, at the
   Approach pluralism level, at the Method execution level, and
   at the Progress Measure validity level — each level has its own
   corridor, and "decision health" is multi-corridor occupation.
4. **Cycle/orphan detection as falsification handle.** A federation
   whose audit chain shows persistent orphans, cycles, or
   reference-mismatch is structurally failing decision-coherence.
   This is a checkable handle for federation-health auditing.

### 1.2 Anti-mission failure modes

- **Orphan accumulation.** Approaches without Goals, Methods
  without Approaches, Measures without referents — over time these
  consume audit-chain weight without contributing to decision
  flow. *Mitigation:* per-deployment orphan-retire policy (default:
  90 days for Approaches without Methods, 90 days for Methods
  without execution evidence, 90 days for Measures without signal).
- **Cycle introduction.** A Goal that references an Approach that
  references it (impossible at the wire level — Goals don't
  reference downward — but possible through indirect chains via
  Reconsideration loops). *Mitigation:* cycle detection at audit-
  chain ingest; cycles are wire-format rejected.
- **Cross-level substrate-typing violations.** A Method declared
  at A4 instantiating an Approach whose Goals are at A0. The
  Method-Approach link may be valid at one rung and incoherent at
  another. *Mitigation:* per-level substrate-rung declared;
  cross-level coherence-check at admission.
- **Decision-flow stall.** Federation publishes Goals and Approaches
  but no Methods materialize; or Methods publish but no Progress
  Measures track them. *Mitigation:* per-level materialization
  rate is a federation health observable; persistent stall is a
  Moderation-input.

---

## 2. WHAT — the DAG and referential integrity

### 2.1 The DAG

```
                 goal_declaration (Level 0)
                       ▲
                       │  goal_refs[]
                       │
                 approach_proposal (Level 1)
                       ▲
                       │  approach_refs[]
                       │
                 method_specification (Level 2)
                       ▲
                       │  goal_refs[] / approach_refs[] / method_refs[]
                       │  (at least one)
                       │
              progress_measure_proposal (Level 3)
```

All references are **upward only**. Goals do not reference
Approaches; Approaches do not reference Methods; Methods do not
reference Progress Measures. This makes cycles structurally
impossible at the wire level — a cycle would require a downward
reference, which the schemas reject.

### 2.2 Referential integrity rules

At audit-chain ingest:

- An `approach_proposal` with `goal_refs[]` referencing a
  non-existent Goal → **rejected** (orphan at admission).
- A `method_specification` with `approach_refs[]` referencing a
  non-existent Approach → **rejected**.
- A `progress_measure_proposal` whose `tracks` is empty or refers
  only to non-existent objects → **rejected**.
- A retired Goal/Approach/Method does not retroactively orphan its
  references — the audit chain preserves history — but new
  references to retired objects are **rejected at admission**.

### 2.3 Cross-level substrate-typing coherence

`approach_proposal.commits.substrate_rungs[]` must be consistent
with the `substrate_rung` of `method_specification` entries that
reference it. v0.1: warning at admission, hard reject only on
explicit substrate-rung mismatch (e.g., Approach commits to
{A3, A4}, Method declares Ph0).

`progress_measure_proposal.computation.substrate_rung` must match
one of the substrate_rungs of objects in its `tracks`. Cross-
substrate measure validity requires explicit attestation per
`PROGRESS_MEASURE_PRIMITIVE.md` §5.3.

### 2.4 Why this WHAT serves M-1

- Upward-only references make decision-flow incoherence
  structurally impossible at the wire layer.
- Referential integrity at admission catches orphans before they
  enter the audit chain.
- Substrate-typing coherence enforces the framework's per-rung
  γM commitment at cross-level boundaries.

---

## 3. HOW — cross-level consensus flow

### 3.1 The forward-traversal flow (Goal → Approach → Method → Measure)

1. **Goal publication.** Agent publishes `goal_declaration` per
   `GOAL_PRIMITIVE.md`. Federation admission via P4/P5/P7/P10.
2. **Approach proposal.** Agent (possibly different from goal
   proposer) publishes `approach_proposal` referencing the
   `goal_id`. Federation admission via the same flow. Multiple
   Approaches per Goal are normal — federation supports parallel
   strategy.
3. **Method specification.** Agent publishes `method_specification`
   referencing one or more `approach_id`s. Federation admission.
   Multiple Methods per Approach are normal — federation supports
   pluralistic operational practice.
4. **Progress Measure proposal.** Agent publishes
   `progress_measure_proposal` referencing the goal_refs /
   approach_refs / method_refs it tracks. Federation admission.
   The measure is provisional until track record accumulates per
   `PROGRESS_MEASURE_PRIMITIVE.md` §3.1.

Forward traversal happens incrementally — a Goal can exist for
months before any Approach proposes to serve it; an Approach can
exist before any Method instantiates it. This is by design: the
federation does not require all four levels in lockstep.

### 3.2 The backward-evaluation flow (Measure → Method → Approach → Goal)

1. **Measures emit signals** over time (Contributions under P6).
2. **Method evaluation** uses execution-verification signals from
   Measures that track Methods.
3. **Approach evaluation** uses Progress Measures (referencing the
   Approach or its Methods) over the Approach's `W_approach`
   window.
4. **Goal evaluation** uses 𝒞_CIRIS over the agent's trace corpus
   per `GOAL_PRIMITIVE.md` §3 — the multiplicative composite
   computed from per-factor Measures, including the framework-
   provided five-factor decomposition.

Backward evaluation feeds Reconsideration (P11): when Measures
suggest a Method is failing, when Approaches are not moving toward
Goals, when Goals' 𝒞_CIRIS scores are collapsing — each is a
trigger for level-appropriate Reconsideration.

### 3.3 Cross-level Reconsideration (P11) coordination

A Reconsideration at one level often implicates adjacent levels:

- **Method retirement** under P11 may leave its Approach with no
  Methods → Approach review under P11.
- **Approach retirement** may leave its Goal with no Approaches →
  Goal review under P11 (typically: federation reconsiders whether
  the Goal as stated is achievable, may re-scope or retire).
- **Measure retirement** under P11 may leave Methods/Approaches/
  Goals without truth-grounding → measure-replacement Contribution
  required, or the affected referents enter Reconsideration.

v0.1: cross-level Reconsideration cascades require **explicit
escalation** (the federation does not automate cascade), but the
audit chain makes cascade-candidate detection straightforward.

### 3.4 Why this HOW serves M-1

- Forward traversal lets the federation build decision hierarchy
  incrementally — no requirement to specify all four levels in
  advance, which preserves M-1's "diverse sentient beings...
  pursue their own flourishing" at decision pace.
- Backward evaluation provides the federation's evidence-flow:
  Measures evaluate everything above them via Reconsideration.
- Cross-level Reconsideration prevents single-level decisions from
  silently breaking other levels.

---

## 4. WHO — cross-level integration with the eleven primitives

| Level | Primary primitives | Notes |
|---|---|---|
| Goal | P5 Contribution (`goal_declaration` kind), P6 Truth-Grounding (𝒞_CIRIS), P7 Weighted Aggregate, P10 Witness Diversity, P11 Reconsideration | 𝒞_CIRIS via P6 is the scoring layer |
| Approach | P5 (`approach_proposal`), P4 Vote, P7, P10, P11 | Derived statistic from Measures via P6 |
| Method | P5 (`method_specification`), P4, P7, P10, P11, **P9 Slashing** (only on documented spoofing) | Execution verifiability via P6 |
| Progress Measure | P5 (`progress_measure_proposal`), P4, P6, P7, P10, P11 (load-bearing for retirement) | Goodhart-resistance via P11 re-evaluation |
| **Cross-cutting** | **P8 Moderation** | Coordinates persistent multi-level failures |

### 4.1 Moderation (P8) as the cross-cutting coordinator

When persistent failures cascade across levels (orphan accumulation,
cycle attempt, decision-flow stall, substrate-typing violations
discovered after admission), P8 Moderation is the primitive that
coordinates federation response. Moderation does not directly act
at any one level; it surfaces cross-level patterns to Wise Authority
quorum for adjudication, with P11 Reconsideration as the appeal
path.

RATCHET's audit-chain reading produces flags that feed Moderation —
cross-level patterns like "this proposer's Approaches consistently
have orphan Methods" or "this measure-cluster shows convergent
decoupling from referents."

### 4.2 Slashing (P9) is operational-only

P9 Slashing applies only at the **Method level**, and only for
**documented spoofing** (claimed execution that audit shows did not
happen). Slashing is never triggered by:
- Goal-level disagreement (pluralism, not malfeasance)
- Approach-level disagreement (pluralism)
- Progress Measure decoupling (Reconsideration path, not Slashing)
- Honest Method-execution failure (Reconsideration path)

This decoupling protects the decision hierarchy from being
weaponized as a Slashing vector.

### 4.3 Why this WHO serves M-1

- Existing eleven primitives carry the entire cross-level flow; no
  new top-level primitive needed for the hierarchy itself.
- P8 Moderation as cross-cutting coordinator centralizes pattern-
  detection without centralizing decision authority — Wise
  Authority quorum adjudicates, P11 Reconsideration appeals.
- P9 Slashing decoupling prevents pluralism-suppression via
  weaponized Slashing.

---

## 5. Multi-corridor cross-level corridor-occupation reading

The framework's corridor structure (Piece 3) and the multi-scale
ρ_goals reading (§sec:tsvf-ubuntu) generalize to the decision
hierarchy. The federation is "in corridor" at each level when:

| Level | In-corridor condition | Out-of-corridor failure modes |
|---|---|---|
| Goal | ρ_goals at relevant belonging-scales in band per `GOAL_PRIMITIVE.md` | Rigidity: forced single-goal collapse. Chaos: no joint goal-support. |
| Approach | Approach count + diversity per Goal in band (neither single-approach-monopoly nor fragmentation) | Rigidity: single-Approach dominance. Chaos: paralytic fragmentation. |
| Method | Method-execution rate in band (Methods getting done, but not all resources monopolized by one Method) | Rigidity: single-Method resource capture. Chaos: vapor work across many Methods. |
| Progress Measure | Measure validity in band (correlation with referents, not decoupled — Goodhart — and not so fine-grained as to lose signal) | Rigidity: single-measure capture. Chaos: measure proliferation without signal. |

The federation's overall decision health is multi-corridor
occupation. v0.1 does not commit a meta-composite analogous to
𝒞_CIRIS at the decision-hierarchy level (whether such a composite
is structurally meaningful is an open question, §7.3). What the
federation can do at v0.1: report per-level corridor membership
publicly, surface persistent multi-level corridor exit to
Moderation.

---

## 6. Empirical anchor

- **The federation's audit chain** records every level-Contribution
  and every reference; a third-party can reconstruct the DAG and
  verify referential integrity + corridor membership at each
  level.
- **The HuggingFace corpus** (`CIRISAI/reasoning-traces`) anchors
  the Goal-level 𝒞_CIRIS computation; extension to include
  Approach/Method/Measure declarations and signals is future work.
- **RATCHET reads cross-level patterns.** N_eff over the
  per-level subspaces (Goal projectors, Approach strategies,
  Method execution-traces, Measure signals) generalizes the
  anti-Sybil reading from per-trace to per-decision-level.

---

## 7. Open questions (deferred to v0.1 cut-time)

1. **Orphan-retire windows per level.** v0.1 default 90 days
   each; per-deployment policy may differ. Long-cycle work
   (institutional change at A4-A5 scale) may need longer windows.
2. **Cross-level Reconsideration cascade automation.** v0.1:
   explicit escalation required. Future: configurable cascade
   policies for clear cases (e.g., Approach with all Methods
   retired automatically enters Reconsideration).
3. **Decision-hierarchy meta-composite.** Is there a structurally
   meaningful "decision health" composite analogous to 𝒞_CIRIS
   across the four levels? Open question. v0.1: no commitment;
   per-level corridor reporting only.
4. **Cycle detection in distributed federation context.** Cycles
   are wire-format impossible (upward-only references), but
   Reconsideration loops can create de-facto cycles
   (Approach A retires → Approach B replaces → fails → A
   reconsidered → reinstated). v0.1: audit chain shows the loop;
   Moderation flags persistent oscillation.
5. **Cross-deployment DAG portability.** When a federation
   imports Goals/Approaches/Methods/Measures from another
   deployment, what additional admission discipline applies?
   v0.1: cross-deployment artifacts treated as new admissions
   requiring local Witness Diversity.
6. **Per-level corridor-band defaults.** What are the structural
   defaults for Approach diversity, Method execution rate,
   Measure correlation? v0.1: empirically tunable per-deployment;
   no framework-derived defaults at this layer.
7. **Substrate-typing coherence across levels.** v0.1: warning on
   mismatch, hard-reject on explicit substrate-rung incompatibility.
   The full coherence-check algorithm is per-deployment.

---

## 8. Lifecycle

Per `MISSION.md` §1.4. Spec → Impl (referential-integrity
validation in `ciris-node-core`'s admission path; cross-level
Reconsideration cascade-candidate detection in audit-chain reader;
per-level corridor reporting in lens-core) → Deployed (pilot)
(when `safety.ciris.ai` reports per-level corridor membership) →
Deployed (folded) (when CIRISAgent consumes the cross-level
hierarchy in main runtime).

---

## 9. References

- All four sibling FSDs: `GOAL_PRIMITIVE.md`,
  `APPROACH_PRIMITIVE.md`, `METHOD_PRIMITIVE.md`,
  `PROGRESS_MEASURE_PRIMITIVE.md`.
- v2 paper: §sec:open-research (post-F-11 sequential per-rung,
  no central scorer), §sec:tsvf-ubuntu (composition rule + multi-
  scale ρ_goals).
- `MISSION.md` v1.0 §2 (eleven primitives), §3.4 (voting).
- `FSD/SUBSTRATE_INTEGRATION.md` (typed-writes for reference
  resolution).
- `RATCHET/AGENT_FSDs/PROOF_OF_BENEFIT_FEDERATION.md` (audit-chain
  reading; N_eff across per-level subspaces).
- HuggingFace dataset: `CIRISAI/reasoning-traces`.

---

## 10. Discipline notes

- **The DAG structure is standard governance.** DAG-of-decision-
  objects is not framework-distinctive; what's distinctive is the
  per-level types (each grounded in framework content) and the
  multi-corridor cross-level reading.
- **The multi-corridor reading is framework-distinctive.** The v2
  paper's corridor structure (Piece 3) and multi-scale ρ_goals
  (§sec:tsvf-ubuntu) generalize to per-decision-level corridors;
  this is framework-licensed content, not standard governance.
- **The decision-hierarchy meta-composite is an open structural
  question.** Whether a "decision health" composite analogous to
  𝒞_CIRIS at the cross-level scale is structurally meaningful is
  not currently a framework commitment. v0.1 reports per-level
  corridor membership; future work may pursue a meta-composite if
  the empirical record supports one.
- **Slashing-decoupling and pluralism-preservation are
  CIRISNodeCore engineering commitments.** The framework licenses
  pluralism (multi-Approach, multi-Method, multi-Measure); the
  specific protections (P9 Slashing operational-only; Goal /
  Approach / Measure disagreement → Reconsideration not Slashing)
  are governance commitments this FSD makes explicit so the
  decision hierarchy cannot be weaponized.
