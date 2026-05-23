# FSD: Progress Measure Primitive — federation-tier carrier for metrics that count as evidence of progress toward Goals

**Status**: Proposed (v0.1.0-dev). Sibling to `GOAL_PRIMITIVE.md`,
`APPROACH_PRIMITIVE.md`, `METHOD_PRIMITIVE.md`. Cross-cut spec at
`DECISION_HIERARCHY.md`.
**Author**: Eric Moore (CIRIS Team) with Claude Opus 4.7
**Created**: 2026-05-22
**Risk**: Architectural. Pins how the federation decides **what
counts as evidence of progress**. This is where Goodhart-resistance
discipline lives. Once committed, measure-retirement flows,
measure-evolution under Reconsideration, per-substrate validity
attestations, and the meta-discipline (a measure of progress can
itself decouple from what it tracks) inherit from this contract.

**Cross-references:** `GOAL_PRIMITIVE.md` (Level 0; Progress Measures
ground Goals via 𝒞_CIRIS factors); `APPROACH_PRIMITIVE.md` (Level 1;
Progress Measures evaluate Approaches over time);
`METHOD_PRIMITIVE.md` (Level 2; Progress Measures observe Method
execution); `DECISION_HIERARCHY.md`; `coherence-ratchet/papers/
Corridor Dynamics.tex` v2 (DOI `10.5281/zenodo.20300773`)
§sec:tsvf-ubuntu (the committed multiplicative composition rule and
the unity-of-virtues structural reading — anti-Goodhart at the Goal
layer); `CIRISLens/FSD/ciris_scoring_specification.md` (the per-
factor SQL queries against `covenant_traces` — the framework's
existing Progress Measure operational layer);
`RATCHET/AGENT_FSDs/PROOF_OF_BENEFIT_FEDERATION.md` (N_eff anti-
Sybil reading; the federation-level diversity discipline measures
contribute to); `MISSION.md` v1.0 §2 (P11 Reconsideration is the
central retirement path); `SCHEMA.md` v1.0; `SUBSTRATE_INTEGRATION.md`.

---

## 0. The gap this FSD fills

`GOAL_PRIMITIVE.md` captures outcomes, `APPROACH_PRIMITIVE.md`
captures strategies, `METHOD_PRIMITIVE.md` captures operational
practice. None of those evaluate themselves. The Progress Measure
Primitive carries **what counts as evidence** that the federation
is moving toward Goals under chosen Approaches via specific Methods.

This is also where the federation's **Goodhart-resistance** discipline
lives. The multiplicative composition rule at Goal level
(`𝒞_CIRIS = C · I_int · R · I_inc · S`) makes single-metric gaming
structurally hard at the goal layer, but Progress Measures
themselves can drift toward proxies that decouple from what they
were supposed to track. The federation needs a meta-level: *is this
Progress Measure tracking what matters, or has it become a target
that gets optimized for its own sake?*

---

## 1. WHY — M-1 alignment

### 1.1 What the Progress Measure Primitive does for M-1

**Meta-Goal M-1**: *promote sustainable adaptive coherence — the
living conditions under which diverse sentient beings may pursue
their own flourishing in justice and wonder*.

The Progress Measure Primitive serves M-1 in five named ways:

1. **Goodhart-resistance discipline at the measure level.** M-1's
   "transparency requirement" requires that measures actually track
   what matters. Progress Measures carry an explicit Goodhart-
   resistance attestation (track record of correlation with the
   referent over a long window); measures that decouple can be
   retired via P11 Reconsideration.
2. **Measure-evolution.** When a measure stops correlating with
   the goal/approach/method it tracks, the federation can vote to
   retire it. This is M-1's "sustainable" at the metric layer:
   measures are not permanent, they earn continued use by
   continued validity.
3. **Per-substrate measure validity.** A measure validated at one
   substrate rung is not automatically valid at another (per Piece 2's
   substrate-typing). Cross-substrate validity requires explicit
   attestation.
4. **Multiplicative composition at Goal level prevents single-
   measure dominance.** Because 𝒞_CIRIS is multiplicative across
   five factors, no single Progress Measure can dominate the Goal-
   level score; gaming one factor's measure does not produce a
   high composite. The framework structurally protects against
   single-measure capture.
5. **Per-factor formulas in CIRISLens spec are framework-provided
   Progress Measures.** The five-factor decomposition
   (C, I_int, R, I_inc, S) and the per-factor SQL queries against
   `covenant_traces` constitute the framework's contribution at
   this layer. Federation-defined Progress Measures extend or
   specialize these without replacing them.

### 1.2 Anti-mission failure modes

- **Measure decoupling (Goodhart).** A measure passes consistently
  while the goal/approach/method it tracks fails. *Mitigation:* the
  Goodhart-resistance attestation requires longitudinal evidence
  of measure-referent correlation; persistent decoupling triggers
  Reconsideration → retirement.
- **Measure proliferation.** Federation publishes so many measures
  that none accumulates the signal weight to be meaningful.
  *Mitigation:* per-referent measure count enters federation
  health observables; persistent proliferation flagged.
- **Measure monopoly.** A single measure dominates evaluation of
  many referents; gaming it captures the federation. *Mitigation:*
  multiplicative composition at Goal level (already in framework);
  P10 Witness Diversity required for measure admission across
  cells.
- **Meta-Goodhart.** Measures of measure-validity themselves drift.
  *Mitigation:* Goodhart-resistance attestation is itself a
  Contribution, votable and retire-able via Reconsideration; no
  permanent measures, no permanent measures-of-measures.

---

## 2. WHAT — schema

### 2.1 Wire shape

```json
{
  "$schema": "https://schemas.ciris.ai/progress-measure-primitive/v0.1",
  "type": "object",
  "required": ["measure_id", "version", "issued_at", "proposer_id",
               "tracks", "computation", "validity_window",
               "goodhart_resistance", "signature"],
  "properties": {
    "measure_id":  { "$ref": "#/defs/content_hash" },
    "version":     { "const": "v0.1" },
    "issued_at":   { "$ref": "#/defs/rfc3339_utc" },
    "proposer_id": { "$ref": "#/defs/ed25519_pubkey_hash" },
    "tracks": {
      "type": "object",
      "description": "What this measure tracks. At least one of goal_refs / approach_refs / method_refs must be non-empty.",
      "properties": {
        "goal_refs":     { "type": "array", "items": { "$ref": "#/defs/goal_declaration_id" } },
        "approach_refs": { "type": "array", "items": { "$ref": "#/defs/approach_proposal_id" } },
        "method_refs":   { "type": "array", "items": { "$ref": "#/defs/method_specification_id" } }
      }
    },
    "computation": {
      "type": "object",
      "required": ["kind", "specification"],
      "properties": {
        "kind":          { "enum": ["sql", "function", "observation_protocol", "human_judgment"] },
        "specification": { "type": "string" },
        "substrate_rung": { "enum": ["Ph0","Ph1","Ph2","A0","A1","A2","A3","A4","A5"] }
      }
    },
    "validity_window": {
      "type": "object",
      "required": ["initial_days", "renewal_policy"],
      "properties": {
        "initial_days":   { "type": "integer", "minimum": 7 },
        "renewal_policy": { "enum": ["expire", "reconsider", "automatic_extend"] }
      }
    },
    "goodhart_resistance": {
      "type": "object",
      "description": "Evidence the measure correlates with what it tracks.",
      "required": ["attestation_kind"],
      "properties": {
        "attestation_kind":   { "enum": ["longitudinal_correlation", "framework_provided", "convergent_validation", "new_measure_pending"] },
        "attestation_refs":   { "type": "array", "items": { "$ref": "#/defs/audit_entry_hash" } },
        "correlation_window_days": { "type": "integer" }
      }
    },
    "privacy_tier":   { "enum": ["public", "federation_only", "encrypted"] },
    "previous_measure": { "$ref": "#/defs/audit_entry_hash" },
    "signature":      { "$ref": "#/defs/ed25519_signature" }
  }
}
```

### 2.2 Why this schema serves M-1

- `tracks` requires at least one referent; orphan measures are
  schema-rejected (no free-floating metrics).
- `computation.specification` makes the measure computable and
  recomputable; M-1's transparency requirement is enforced at the
  wire layer.
- `validity_window` makes measure-expiration explicit; permanence
  is opt-in (and requires a renewal policy).
- `goodhart_resistance` is required — every measure must declare
  how it earns its validity claim. `new_measure_pending` is the
  honest "no track record yet" attestation; it does not get full
  weight in P7 Weighted Aggregate until track record exists.
- `previous_measure` chains the measure-evolution audit trail; a
  retired measure leaves its successor traceable.

---

## 3. HOW — logic

### 3.1 Truth-grounding for measures themselves

Measures evaluate other primitives, but they also need to be
evaluated themselves. v0.1 truth-grounding for Progress Measures:

- **Longitudinal correlation.** Over a window
  (`goodhart_resistance.correlation_window_days`), does the measure
  signal correlate with framework-distinctive outcomes at the
  referent (𝒞_CIRIS movement at Goal level; Method execution rate
  at Method level)? Correlation persistence is the primary truth-
  grounding signal.
- **Framework-provided attestation.** The per-factor formulas in
  `CIRISLens/FSD/ciris_scoring_specification.md` (C, I_int, R,
  I_inc, S decompositions and their SQL queries) are framework-
  provided measures; they inherit the framework's empirical record
  (the HuggingFace corpus reproducibility).
- **Convergent validation.** When multiple measures track the same
  referent and their signals agree, mutual validation strengthens
  each.
- **Pending attestation.** A new measure with `attestation_kind:
  new_measure_pending` is provisional — it can be used but does
  not carry P7 weight until correlation evidence accumulates.

### 3.2 The federation's evaluation flow

A measure publishes signals (Contributions). The federation:

1. Reads signals (per-peer; the post-F-11 architectural
   commitment).
2. Aggregates via P7 Weighted Aggregate.
3. Cross-validates via P10 Witness Diversity (do agents with
   substrate-rung Expertise concur on the measure signal?).
4. Periodically (per `validity_window.initial_days`) re-evaluates
   the measure's Goodhart-resistance via P11 Reconsideration: has
   the measure's signal continued to correlate with its referent's
   outcome?

A measure that fails its re-evaluation enters retirement flow.

### 3.3 Measure retirement

Three retirement paths:

- **Expiration.** `validity_window.renewal_policy: expire` — the
  measure auto-expires at window end unless explicitly renewed.
- **Reconsideration-driven retirement.** Federation votes to retire
  via P11 because the measure has decoupled from its referent.
- **Supersession.** A new measure references the retired one via
  `previous_measure`; the retired measure's audit trail is
  preserved.

Retirement does not delete the audit history; it stops the measure
from contributing to current evaluation flow.

---

## 4. WHO — protocols

### 4.1 `progress_measure_proposal` Contribution kind

Standard P5 Contribution. Vote (P4), Weighted Aggregate (P7),
Witness Diversity (P10) admission flow.

### 4.2 Two-layer Witness Diversity

P10 plays at two layers for measures: (a) admission (is this
measure well-formed and likely to track its referent?), (b) periodic
re-validation (does the measure continue to correlate over time?).
Both layers require above-threshold Witness Diversity from
substrate-rung-Expertise holders.

### 4.3 Reconsideration as the central retirement primitive

P11 (Reconsideration) is the load-bearing path for measure
retirement disputes. Goodhart-detection is operationalized as
Reconsideration evidence: "the measure has decoupled; here is the
correlation data over the window." The federation's measure-base
is dynamic, not permanent.

### 4.4 RATCHET reads measure history

RATCHET (PoB §2.4) reads the federation's audit chain — including
measure signal history — for N_eff anti-Sybil patterns. Measures
that show drift toward "single-source-agreement" or "high-frequency
oscillation" become RATCHET-flagged inputs to Moderation review.

### 4.5 Why this WHO serves M-1

- P5 + P4 + P7 + P10 + P11 carry the measure lifecycle entirely;
  no new top-level primitive needed.
- Two-layer P10 prevents both proposer-capture at admission and
  retirement-suppression by interested parties.
- RATCHET reading provides federation-external auditing of measure
  health; the federation does not exclusively self-audit.

---

## 5. Special concerns (this primitive's distinctive opportunities)

### 5.1 Goodhart-resistance as first-class concern

This is the primitive's distinctive load-bearing concern.
`goodhart_resistance` is required at the schema layer, the
federation's evaluation flow includes periodic re-validation, and
P11 Reconsideration is structured to handle measure-retirement
disputes. The framework's multiplicative composition at Goal level
(structurally anti-Goodhart) is upstream protection; this
primitive carries the discipline downstream.

### 5.2 Measure-evolution with audit-chain continuity

A federation that cannot retire measures becomes ossified — every
measure ever proposed continues to weigh on evaluation. A federation
that retires too easily loses signal continuity. The
`previous_measure` chain + the three retirement paths (expiration /
Reconsideration / supersession) balance these.

### 5.3 Per-substrate validity

Per Piece 2's substrate-typing of γM: a Progress Measure validated
at A4 institutional substrate (e.g., institutional-overhead-per-
governance-decision) does not transfer to A3 individual substrate
(e.g., LLM attention-head ρ). Cross-substrate validity requires
explicit `computation.substrate_rung` + Goodhart-resistance
attestation specific to each rung.

### 5.4 Composition with 𝒞_CIRIS

When Progress Measures decompose into the five 𝒞_CIRIS factors
(C, I_int, R, I_inc, S), the multiplicative composition at Goal
level provides structural protection: gaming one factor's measure
does not produce a high composite (any near-zero factor collapses
the score). This is the unity-of-virtues discipline the v2 paper
formalizes, operationalized at the federation tier.

### 5.5 The meta-Goodhart problem

A measure of measure-validity (Goodhart-resistance attestation)
can itself be gamed. v0.1 mitigation: the Goodhart-resistance
attestation is itself a Contribution under P5, votable and
retire-able under P11. No permanent measures, no permanent
measures-of-measures, no permanent measures-of-measures-of-
measures — turtles, but all of them have retirement paths.

---

## 6. Empirical anchor

- **The HuggingFace corpus is the public testbed for framework-
  provided measures.** `CIRISAI/reasoning-traces` enables third-
  party reproduction of every per-factor formula in CIRISLens spec.
- **Federation audit chain extends to measure signal history.** A
  federation's chain of `progress_measure_proposal` Contributions +
  signal Contributions over time enables third-party reconstruction
  of measure validity / decoupling patterns.
- **RATCHET N_eff over the measure subspace.** Per RATCHET PoB
  §2.4, the federation's measures themselves form a vector space
  on which N_eff can be computed; persistent N_eff < 9 (the
  deceptive-basin threshold) is a federation-level measure-monopoly
  signal.

---

## 7. Open questions (deferred to v0.1 cut-time)

1. **Goodhart-resistance attestation format.** v0.1: free-form
   `attestation_refs[]` pointing to audit-chain evidence; future
   versions may standardize the format.
2. **Validity window defaults.** v0.1: 90 days initial, renewal via
   Reconsideration; revisit per-deployment.
3. **Measure-cluster handling.** When multiple measures track the
   same referent and their signals agree, what's the weight
   adjustment under convergent validation?
4. **Per-substrate measure-portability claims.** Cross-substrate
   attestation format and witness requirements.
5. **Goodhart-detection automation.** Can RATCHET emit automatic
   measure-Goodhart flags, or does the meta-discipline require
   human Reconsideration even for clear decoupling cases?
6. **Pending-attestation weight policy.** How much P7 weight does
   a `new_measure_pending` measure get during its initial window?
   v0.1: 0.25× full weight; revisit if abuse.
7. **Cross-deployment measure portability.** Default per-
   deployment; cross-deployment requires explicit re-attestation.

---

## 8. Lifecycle

Per `MISSION.md` §1.4. Spec → Impl
(`progress_measure_proposal` Contribution kind defined in
`SCHEMA.md`; the framework-provided measures inherit
implementation from `CIRISLens/FSD/ciris_scoring_specification.md`;
federation-defined measures use the Contribution surface) →
Deployed (pilot) → Deployed (folded).

---

## 9. References

- v2 paper: §sec:tsvf-ubuntu (multiplicative composition rule and
  unity-of-virtues structural anti-Goodhart at Goal level), Piece 2
  (substrate-typing).
- `GOAL_PRIMITIVE.md`, `APPROACH_PRIMITIVE.md`,
  `METHOD_PRIMITIVE.md`.
- `DECISION_HIERARCHY.md` (cross-level DAG).
- `CIRISLens/FSD/ciris_scoring_specification.md` (the framework-
  provided measure operational layer — five factors, per-factor
  SQL queries).
- `RATCHET/AGENT_FSDs/PROOF_OF_BENEFIT_FEDERATION.md` (N_eff on the
  measure subspace).
- `MISSION.md` v1.0 §2 (P11 Reconsideration is load-bearing here),
  §3.4 (voting).
- HuggingFace dataset: `CIRISAI/reasoning-traces`.

---

## 10. Discipline notes

- **Framework-provided measures (CIRISLens five-factor spec) are
  framework-distinctive content.** Federation-defined measures use
  the same Contribution surface but are not framework-grounded
  beyond the requirement to declare their substrate rung and
  Goodhart-resistance attestation.
- **The meta-Goodhart problem is real and named, not solved.**
  v0.1 mitigates via Contribution-and-Reconsideration recursion,
  not by a structural theorem. The framework's multiplicative
  composition at Goal level is the only structural anti-Goodhart
  protection; everything downstream is governance discipline that
  could itself drift.
- **Per-substrate measure validity is framework-distinctive.**
  Piece 2's substrate-typing of γM licenses the requirement that
  cross-substrate measure-portability requires explicit
  attestation. The mechanism (additional Contributions, witness
  diversity) is standard governance with framework-grounded
  objects.
