# FSD: Approach Primitive — federation-tier carrier for strategic pathways from current state toward Goals

**Status**: Proposed (v0.1.0-dev). Sibling to `GOAL_PRIMITIVE.md`,
`METHOD_PRIMITIVE.md`, `PROGRESS_MEASURE_PRIMITIVE.md`. Cross-cut
spec at `DECISION_HIERARCHY.md`.
**Author**: Eric Moore (CIRIS Team) with Claude Opus 4.7
**Created**: 2026-05-22
**Risk**: Architectural. Pins how the federation carries **strategy**
(distinct from outcome at Goal-level, distinct from operational
practice at Method-level). Once committed, sub-federation branching
semantics, multi-approach pluralism, and approach-evolution flows
inherit from this contract.

**Cross-references:** `GOAL_PRIMITIVE.md` (Level 0 outcome; Approach
references it); `METHOD_PRIMITIVE.md` (Level 2 operational
instantiation; Approach is referenced by it);
`PROGRESS_MEASURE_PRIMITIVE.md` (Level 3 metrics; Approach
evaluated by measures it references); `DECISION_HIERARCHY.md` (the
cross-level DAG, referential integrity, consensus flow);
`coherence-ratchet/papers/Corridor Dynamics.tex` v2 (DOI
`10.5281/zenodo.20300773`) Piece 10 (karma as cumulative product of
past goal-projections — the framework-grounded type of an Approach);
`MISSION.md` v1.0 §2 (eleven primitives); `SCHEMA.md` v1.0;
`SUBSTRATE_INTEGRATION.md`.

---

## 0. The gap this FSD fills

`GOAL_PRIMITIVE.md` captures the *outcome target* (Level 0): what we
are aiming at. This FSD captures the *strategic pathway* (Level 1):
**how we propose to get there**. Without an Approach Primitive,
agents can declare aligned goals and still fail to federate, because
the question "by what pathway?" stays as unstructured policy debate
rather than as votable, weighted, witnessed, reconsiderable
Contributions on framework-grounded objects.

Multiple approaches may serve the same goal. The federation may
pursue several in parallel, branch into sub-federations following
different approaches, or converge on one once Progress Measures
disambiguate. This FSD makes that pluralism structural.

---

## 1. WHY — M-1 alignment

### 1.1 What the Approach Primitive does for M-1

**Meta-Goal M-1**: *promote sustainable adaptive coherence — the
living conditions under which diverse sentient beings may pursue
their own flourishing in justice and wonder*.

The Approach Primitive serves M-1 in five named ways:

1. **Pluralistic-pathway by construction.** M-1's "diverse sentient
   beings... pursue their own flourishing" is incompatible with
   federation-mandated single-pathway pursuit of any goal. The
   Approach Primitive lets the federation hold multiple
   simultaneously-active approaches to the same Goal without
   manufacturing artificial consensus.
2. **Karma-as-approach-trail.** Piece 10's *karma* — the cumulative
   product of past goal-projections — is the framework's
   identification for what an Approach *is*: the trail of strategic
   commitments an agent or federation has made in pursuit of a Goal.
   The Approach object inherits that structural shape.
3. **Sub-federation branching.** When two approaches genuinely
   cannot coexist (irreconcilable strategy at the same operational
   layer), the federation can branch — a sub-federation adopts one
   approach, another sub-federation adopts the other, both remain
   bound to the parent's Goal. The Approach Primitive carries the
   branching semantics; the existing P7 / P10 primitives carry the
   consensus.
4. **Approach-evolution via Reconsideration.** When an Approach
   fails its Progress Measures, P11 (Reconsideration) is the path
   for amendment, supersession, or retirement. The Approach is
   structurally revisable; lock-in is forbidden.
5. **Anti-strategy-monopoly.** A federation that collapses to a
   single Approach across all Goals has lost pluralism (rigidity).
   The Approach Primitive carries the *count and diversity* of
   active approaches per Goal as a federation-level health
   observable.

### 1.2 Anti-mission failure modes

- **Strategy monopoly.** A single Approach dominates because of
  Credits/Expertise weighting, not because of Progress Measure
  signal. *Mitigation:* P10 Witness Diversity required for Approach
  admission; weighted aggregate must not produce single-Approach
  outcomes without a Progress Measure signal supporting it.
- **Approach-fragmentation-into-paralysis.** Federation publishes
  so many parallel Approaches that no single one accumulates the γM
  to be tested. *Mitigation:* per-Goal Approach count enters the
  federation's health observables; persistent fragmentation is a
  Moderation-input.
- **Free-floating approaches.** An `approach_proposal` without a
  valid `goal_id` reference. *Mitigation:* referential integrity
  enforced at admission (per `DECISION_HIERARCHY.md` §2);
  orphan-Approach auto-retire policy configured per deployment.

---

## 2. WHAT — schema

### 2.1 Wire shape

```json
{
  "$schema": "https://schemas.ciris.ai/approach-primitive/v0.1",
  "type": "object",
  "required": ["approach_id", "version", "issued_at", "proposer_id",
               "goal_refs", "strategy", "commits", "signature"],
  "properties": {
    "approach_id":   { "$ref": "#/defs/content_hash" },
    "version":       { "const": "v0.1" },
    "issued_at":     { "$ref": "#/defs/rfc3339_utc" },
    "proposer_id":   { "$ref": "#/defs/ed25519_pubkey_hash" },
    "goal_refs": {
      "type": "array",
      "minItems": 1,
      "items": { "$ref": "#/defs/goal_declaration_id" }
    },
    "strategy": {
      "type": "object",
      "required": ["prose", "structured"],
      "properties": {
        "prose":      { "type": "string", "maxLength": 16384 },
        "structured": { "type": "object" }
      }
    },
    "commits": {
      "type": "object",
      "description": "Forward reference to expected Method-level γM expenditure pattern",
      "properties": {
        "substrate_rungs": { "type": "array", "items": { "enum": ["Ph0","Ph1","Ph2","A0","A1","A2","A3","A4","A5"] } },
        "expected_method_count": { "type": "integer", "minimum": 1 },
        "expected_window_days":  { "type": "integer", "minimum": 1 }
      }
    },
    "mitigations": {
      "type": "array",
      "items": { "type": "object", "required": ["condition", "action"] }
    },
    "previous_approach": { "$ref": "#/defs/audit_entry_hash" },
    "signature":         { "$ref": "#/defs/ed25519_signature" }
  }
}
```

### 2.2 Why this schema serves M-1

- `goal_refs[]` enforces *no orphans* at the schema level —
  approaches must be in service of declared Goals (referential
  integrity, `DECISION_HIERARCHY.md`).
- `strategy.prose` + `strategy.structured` together let humans
  reason about strategy *and* machines route it; neither alone is
  sufficient.
- `commits` is a forward-reference signal so the federation can see
  whether anticipated Methods materialize within the expected
  window — a no-Method Approach over time is a failure mode the
  federation can detect.
- `mitigations[]` makes contingency explicit: an Approach without
  named failure-modes is structurally over-confident.
- `previous_approach` chains the karma-trail (Piece 10): revising
  an approach is updating your strategic trail, not erasing it.

---

## 3. HOW — logic

### 3.1 Approach evaluation flows from Progress Measures

Approaches are *not* scored by `𝒞_CIRIS` directly (that's
Goal-level). Approach evaluation is **derived from linked Progress
Measures over time** (`PROGRESS_MEASURE_PRIMITIVE.md`):

- Each Approach's referenced Progress Measures emit signals
  (Contributions under P6 Truth-Grounding).
- Over a rolling window `W_approach` (default 30 days at v0.1), the
  federation reads the Progress Measure trajectory and assigns the
  Approach a derived statistic: *moving toward Goal*, *flat*, or
  *moving away*.
- Two Approaches serving the same Goal are not in conflict by
  default; the federation runs them in parallel until Progress
  Measure signal disambiguates (the federated A/B).

### 3.2 Karma chaining

`previous_approach` references the prior Approach this one revises.
Karma is the cumulative-product structure: an Approach's effective
state is the composition of the current proposal *and* the trail of
revisions that led to it. Audit-chain continuity inherits from
substrate; no new cryptographic surface.

### 3.3 Sub-federation branching

When the federation cannot coexist two Approaches at the same Goal
(genuine incompatibility, signaled by Progress Measures or by
operational resource conflict), `MISSION.md` §3.4's voting +
Witness Diversity (P10) determines whether the federation:
- Adopts one Approach and retires the other (Reconsideration path
  for the retired Approach's proponents).
- Branches into sub-federations, each adopting one Approach,
  bound to the parent's Goal.
- Defers the choice pending more Progress Measure data.

v0.1 default: branching requires a P7 Weighted Aggregate threshold
plus P10 Witness Diversity above the jump-threshold. The
branching semantics are a CIRISNodeCore-level commitment, not
framework-derived.

---

## 4. WHO — protocols

### 4.1 `approach_proposal` Contribution kind

A new Contribution kind under P5, canonicalized + signed via
substrate, written through the typed-write path
(`SUBSTRATE_INTEGRATION.md` §2.1). Standard Vote (P4) + Weighted
Aggregate (P7) admission flow.

### 4.2 Witness Diversity for Approach admission

Approach admission requires P10 Witness Diversity above the jump
threshold — at least N distinct cell-Expertise-holders affirming
that the Approach is well-formed (refers to a real Goal, has
coherent strategy, names realistic commits). This is admission
discipline, not evaluation discipline; evaluation is per §3.1.

### 4.3 Reconsideration for Approach revision

P11 (Reconsideration) is the appeals path for Approach retirement
disputes: an Approach whose Progress Measures suggest retirement
can be defended by its proponents via Reconsideration with adjusted
measures or extended window. The federation may grant or deny;
denial-with-cause becomes the audit record.

### 4.4 Why this WHO serves M-1

- Standard P4/P5/P7 admission keeps wire surface minimal.
- P10 Witness Diversity prevents single-cell capture at admission.
- P11 Reconsideration prevents premature retirement of
  Approaches that *deserve* a longer window but are failing in
  early Progress Measure signal.

---

## 5. Special concerns (this primitive's distinctive opportunities)

### 5.1 Pluralism-handling — multi-approach coordination

Two Approaches serving the same Goal can:
- Run truly in parallel (federation supports both with separate γM
  budgets).
- Share Methods (different strategies, common operational practice).
- Compete for Methods (same operational resource, divergent
  strategies — Progress Measure signal disambiguates).

The Approach Primitive carries the *coordination mode* as an
implicit signal in its `commits` and `mitigations` — explicit
sub-federation branching is a heavier mechanism reserved for genuine
incompatibility.

### 5.2 Approach evolution

An Approach can be: *amended* (refinements within the same strategic
shape, `previous_approach` chained), *superseded* (a new Approach
with the same `goal_refs` replaces the prior one), or *retired* (no
successor; the Approach is closed). Each has different downstream
effects on linked Methods. v0.1 default: amendment continues linked
Methods, supersession requires Method re-binding to the new
Approach, retirement deprecates linked Methods.

### 5.3 Sub-federation branching

When a sub-federation forks off, the parent federation tracks both
branches in its audit chain; neither is a defection. Both branches
remain bound to the parent's Goal. The federation directory carries
the branching metadata.

### 5.4 Anti-strategy-monopoly observable

Per-Goal Approach count and diversity (across belief contexts `B_i`
and across substrate rungs) is a federation health observable that
RATCHET reads alongside its N_eff anti-Sybil patterns. Persistent
single-Approach dominance triggers a flag.

---

## 6. Empirical anchor

The federation's audit chain of `approach_proposal` Contributions +
linked Progress Measure trajectories is the public record. The
HuggingFace corpus (`CIRISAI/reasoning-traces`) will extend to
include Approach declarations and Progress Measure signals over
time, enabling third-party reconstruction of federation-level
strategic flow.

---

## 7. Open questions (deferred to v0.1 cut-time)

1. **Window `W_approach` for derived-statistic evaluation.** Default
   30 days; revisit if 30 is too short for substrates with slow
   Progress Measure cadence.
2. **Sub-federation forking thresholds.** Default: P7 Weighted
   Aggregate above 0.66 + P10 Witness Diversity above jump.
3. **Orphan-Approach auto-retire policy.** Default: 90 days without
   any linked Method materializing → auto-retire with audit record.
4. **Cross-deployment Approach portability.** Default: per-deployment;
   substrate's federation directory determines cross-deployment story.
5. **Approach-Approach conflict-detection.** When two Approaches at
   the same Goal compete for the same Method-resource: how is
   conflict signaled, and what triggers branching vs Reconsideration?
6. **Amendment vs supersession threshold.** When does a change in
   `strategy` cross from "amendment" to "supersession" requiring
   Method re-binding? v0.1: proposer-declares; revisit if abuse.

---

## 8. Lifecycle

Per `MISSION.md` §1.4. Spec → Impl (`approach_proposal` Contribution
kind defined in `SCHEMA.md`, derived-statistic computation in
`ciris-lens-core`) → Deployed (pilot) (when `safety.ciris.ai`
consumes Approaches from at least one CIRISAgent deployment) →
Deployed (folded) (when CIRISAgent emits `approach_proposal`
Contributions in main runtime).

---

## 9. References

- v2 paper: Piece 10 (karma as cumulative product), §sec:tsvf-ubuntu
  (multi-scale belonging composite that Approaches serve).
- `GOAL_PRIMITIVE.md` (Level 0 architectural commitments inherited).
- `DECISION_HIERARCHY.md` (cross-level DAG and consensus flow).
- `MISSION.md` v1.0 §1.5 (Ubuntu Recursive Golden Rule),
  §2 (eleven primitives), §3.4 (voting).
- `RATCHET/AGENT_FSDs/PROOF_OF_BENEFIT_FEDERATION.md` (audit-chain
  reading for diversity).

---

## 10. Discipline notes

- **The karma-as-approach-trail identification is the framework's.**
  Piece 10 names karma as cumulative goal-projection structure; the
  Approach Primitive instantiates that at the federation tier. The
  identification ships at the contract surface, not buried.
- **Approach evaluation is derived, not direct.** Approaches do not
  carry a primary score; their truth-grounding flows from linked
  Progress Measures over time. The federation's evaluation surface
  for Approaches is downstream of `PROGRESS_MEASURE_PRIMITIVE.md`.
- **Sub-federation branching is a CIRISNodeCore engineering call,
  not a framework derivation.** The v2 paper licenses multi-Approach
  pluralism via Piece 5's multi-agent consent (ρ_goals can be in
  corridor at one scale and split at another); the specific
  branching semantics (thresholds, audit-chain handling, parent-
  child federation directory) are engineering commitments this FSD
  makes that the framework does not derive.
