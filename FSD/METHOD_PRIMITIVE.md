# FSD: Method Primitive — federation-tier carrier for concrete operational practices instantiating Approaches

**Status**: Proposed (v0.1.0-dev). Sibling to `GOAL_PRIMITIVE.md`,
`APPROACH_PRIMITIVE.md`, `PROGRESS_MEASURE_PRIMITIVE.md`. Cross-cut
spec at `DECISION_HIERARCHY.md`.
**Author**: Eric Moore (CIRIS Team) with Claude Opus 4.7
**Created**: 2026-05-22
**Risk**: Architectural. Pins how the federation carries
**operational γM commitments** — the actual work being done.
Once committed, substrate-typing per Method, resource-accountability,
execution-verifiability, and Slashing-for-vapor-work flows inherit
from this contract.

**Cross-references:** `GOAL_PRIMITIVE.md` (Level 0 outcome);
`APPROACH_PRIMITIVE.md` (Level 1 strategy; Method instantiates it);
`PROGRESS_MEASURE_PRIMITIVE.md` (Level 3 metrics; Method execution
observable through them); `DECISION_HIERARCHY.md`; `coherence-
ratchet/papers/Corridor Dynamics.tex` v2 (DOI
`10.5281/zenodo.20300773`) Piece 2 (the α−γM dynamics; γM as
substrate-typed maintenance work — the framework-grounded type of
a Method), §sec:open-research (sequential per-rung architecture,
each rung's γM substrate-typed and locally maintained);
`MISSION.md` v1.0 §2 (eleven primitives, including P9 Slashing);
`FSD/SAFETY_BATTERY_CI_LOOP.md` (the capture+interpret pattern
this primitive generalizes); `SCHEMA.md` v1.0; `SUBSTRATE_INTEGRATION.md`.

---

## 0. The gap this FSD fills

`APPROACH_PRIMITIVE.md` captures the *strategic pathway* (Level 1):
how we propose to get to the Goal. This FSD captures the *concrete
operational practice* (Level 2): **what specifically gets done**.

A Method is the actual γM expenditure — the institutional overhead,
computational cycles, biochemical work, social ritual, or
sociotechnical labor that an Approach requires for execution. Without
a Method Primitive, Approaches are abstract strategies without
operational specification, the federation cannot verify that
strategic commitments translate to actual work, and resource
accountability has no audit surface.

---

## 1. WHY — M-1 alignment

### 1.1 What the Method Primitive does for M-1

**Meta-Goal M-1**: *promote sustainable adaptive coherence — the
living conditions under which diverse sentient beings may pursue
their own flourishing in justice and wonder*.

The Method Primitive serves M-1 in five named ways:

1. **Substrate-typing — methods are per-rung.** The framework's
   Piece 2 names γM as substrate-typed: institutional overhead at
   A4, refactoring at A3 LLMs, ritual at A3 social, ATP hydrolysis
   at A0 cellular. A Method Primitive declares its substrate rung
   and is bound to that rung's γM character. M-1's "diverse sentient
   beings" maps onto operational diversity: a Method valid at A4 is
   not naively portable to A3.
2. **Resource accountability.** A Method commits γM resources
   (compute, funding, time, people). The federation's audit chain
   records expenditure; vapor work (declared but not executed) is
   detectable. P9 Slashing applies to documented resource-spoofing.
3. **Execution verifiability.** Distinct from Approach
   (evaluated by strategic logic) and Goal (evaluated by 𝒞_CIRIS
   outcomes), the Method is evaluated by **whether it gets done** —
   signed work logs, capture+interpret artifacts, audit-chain
   traceability. The `SAFETY_BATTERY_CI_LOOP.md` pattern generalizes.
4. **Pluralistic methods per Approach.** Multiple Methods can
   instantiate the same Approach; the federation supports
   operational diversity at the implementation layer just as
   Approach supports strategic diversity at the pathway layer.
5. **Anti-vapor-work discipline.** A Method declared with no
   execution evidence over time is structurally suspect. The
   federation can flag persistent Methods that consume audit-chain
   weight without producing execution traces; P9 Slashing applies
   when the audit evidence shows deliberate spoofing.

### 1.2 Anti-mission failure modes

- **Vapor work.** Methods declared but never executed; the agent
  earns Credits for declaration without doing the work.
  *Mitigation:* execution-verifiability is the truth-grounding
  signal (§3.1); persistent declared-not-executed Methods → flag.
- **Resource spoofing.** Claimed γM expenditure not actually spent;
  the agent claims to be doing the work but the substrate audit
  shows otherwise. *Mitigation:* P9 Slashing applies on documented
  spoofing; resource expenditure is observable through the substrate
  audit chain.
- **Method-portability overreach.** A Method declared valid at one
  substrate claimed automatically valid at another. *Mitigation:*
  per-Method `substrate_rung` is required; cross-substrate validity
  requires an explicit Method-portability attestation (which is
  itself a Contribution, votable).

---

## 2. WHAT — schema

### 2.1 Wire shape

```json
{
  "$schema": "https://schemas.ciris.ai/method-primitive/v0.1",
  "type": "object",
  "required": ["method_id", "version", "issued_at", "proposer_id",
               "approach_refs", "substrate_rung", "specification",
               "commits", "signature"],
  "properties": {
    "method_id":     { "$ref": "#/defs/content_hash" },
    "version":       { "const": "v0.1" },
    "issued_at":     { "$ref": "#/defs/rfc3339_utc" },
    "proposer_id":   { "$ref": "#/defs/ed25519_pubkey_hash" },
    "approach_refs": {
      "type": "array",
      "minItems": 1,
      "items": { "$ref": "#/defs/approach_proposal_id" }
    },
    "substrate_rung": { "enum": ["Ph0","Ph1","Ph2","A0","A1","A2","A3","A4","A5"] },
    "specification": {
      "type": "object",
      "required": ["prose", "structured"],
      "properties": {
        "prose":         { "type": "string", "maxLength": 16384 },
        "structured":    { "type": "object" },
        "execution_log_format": { "$ref": "#/defs/log_schema_ref" }
      }
    },
    "commits": {
      "type": "object",
      "description": "Resource expenditure pattern this Method commits to",
      "properties": {
        "compute":   { "type": "object" },
        "funding":   { "type": "object" },
        "person_hours": { "type": "object" },
        "duration_days": { "type": "integer", "minimum": 1 }
      }
    },
    "portability_claims": {
      "type": "array",
      "description": "Explicit attestations that this Method is valid at additional substrate rungs (each requires its own Contribution).",
      "items": { "type": "object", "required": ["rung", "attestation_ref"] }
    },
    "previous_method": { "$ref": "#/defs/audit_entry_hash" },
    "signature":       { "$ref": "#/defs/ed25519_signature" }
  }
}
```

### 2.2 Why this schema serves M-1

- `approach_refs[]` enforces referential integrity (no orphans);
  Methods cannot float free of Approaches they instantiate.
- `substrate_rung` makes substrate-typing explicit — no implicit
  portability claims.
- `specification.execution_log_format` references the schema
  expected for execution evidence; the federation knows in advance
  what audit signal to look for.
- `commits` makes resource expenditure pattern observable; vapor
  work and spoofing become detectable as deviation from commits.
- `portability_claims[]` requires explicit attestation
  Contributions for cross-substrate validity — no quiet portability.

---

## 3. HOW — logic

### 3.1 Truth-grounding: execution verifiability

A Method's truth-grounding signal is **"the Method gets done"** —
verifiable against the substrate's audit chain. v0.1 truth-grounding
sources:

- **Signed work logs.** The proposer (or executing agent) emits
  signed Contributions reporting execution events. Format
  per the Method's declared `execution_log_format`.
- **Capture+interpret artifacts** (`SAFETY_BATTERY_CI_LOOP.md`
  pattern, generalized). For Methods whose execution is observable
  through structured outputs, capture happens at execution time and
  interpretation produces a signed verdict.
- **Resource expenditure observation** through substrate (compute
  consumed, funding moved, person-hours logged in adjacent systems
  that the federation directory trusts).

Aggregate truth-grounding via P6 (Truth-Grounding) → P7 (Weighted
Aggregate). The Method's *execution rate* (fraction of `commits`
delivered within `commits.duration_days`) is the primary derived
statistic at federation level.

### 3.2 Method-Approach relationship

Multiple Methods can instantiate the same Approach. Federation can:
- Run them in parallel (pluralistic operational practice).
- Run them sequentially (Method A first, then Method B if A fails).
- Choose one (federation votes on which Method best instantiates
  the Approach in current operational context).

The Method does not commit the federation to a single operational
practice per Approach — pluralism is the v0.1 default.

### 3.3 Slashing (P9) interaction

Method-execution failure is *not* a Slashing trigger by itself.
Honest failure to execute is signal, not malfeasance. Slashing
applies only when execution audit shows **deliberate spoofing** —
claimed expenditure not actually spent, signed work logs that
contradict observable substrate state, fabricated capture artifacts.
The Slashing path requires P10 Witness Diversity + WA quorum
(MISSION.md §2 P9) and is not automated from the Method primitive
alone.

---

## 4. WHO — protocols

### 4.1 `method_specification` Contribution kind

Standard P5 Contribution. Vote (P4), Weighted Aggregate (P7), Witness
Diversity (P10) admission flow.

### 4.2 Witness Diversity for Method execution

P10 plays at two layers for Methods: (a) admission (does this Method
well-instantiate the Approach?), (b) execution attestation (did the
work actually get done per the execution log format?). Both layers
require above-threshold Witness Diversity; execution-attestation
witnesses must have non-zero substrate-rung Expertise.

### 4.3 Reconsideration for Method disputes

P11 (Reconsideration) is the appeals path for: (a) execution
failure disputes (the proposer claims the work was done; witnesses
deny), (b) Slashing-trigger disputes (proposer claims honest failure;
witnesses claim spoofing). v0.1: Reconsideration must produce
additional evidence (extended capture window, independent witness,
substrate audit re-examination).

### 4.4 Why this WHO serves M-1

- Standard P4/P5/P7/P10 flow keeps wire surface minimal.
- Two-layer P10 (admission + execution-attestation) prevents
  Method-spoofing from passing on Credits-weighted admission alone.
- P11 Reconsideration distinguishes honest failure from spoofing
  via additional evidence; the federation does not automate Slashing
  on Method-execution-failure alone.

---

## 5. Special concerns (this primitive's distinctive opportunities)

### 5.1 Substrate-typing as first-class

The framework's substrate-typing of γM (Piece 2) maps onto the
Method Primitive at the schema level: `substrate_rung` is required,
`portability_claims[]` is the only way to assert cross-rung validity,
and federation-level Method-portability requires its own
Contribution-kind attestation. This is M-1's "diverse sentient
beings... own flourishing" at the operational layer — methods are
context-bound.

### 5.2 Resource accountability and the substrate audit chain

The framework's commitment that γM is *actual work* (not declared
work) lands at the protocol layer: Methods commit measurable
resources, and the substrate audit chain (compute meters, funding
ledgers, person-hour logs from trusted adjacent systems) provides
the verification surface. CIRISNodeCore inherits the substrate's
verification capability via `SUBSTRATE_INTEGRATION.md`.

### 5.3 Execution verifiability vs strategic evaluation

A clean distinction this FSD enforces:
- Method evaluation: *did the work get done?* (execution
  verifiability)
- Approach evaluation: *did the strategy serve the goal?*
  (downstream Progress Measure correlation)
- Goal evaluation: *did the agent stay in corridor across the five
  factors?* (𝒞_CIRIS over time)

A Method can execute perfectly while its Approach fails (the
strategy was wrong); the federation can retire the Approach
without slashing the Method's executor. A Method can fail to execute
while its Approach was sound (the resource commit was unrealistic);
the federation can amend the Method's commits or retire it without
retracting the Approach.

### 5.4 Anti-vapor-work observable

Per-Method execution rate (fraction of commits delivered within
window) is a federation health observable that RATCHET reads
alongside its other patterns. Persistent vapor work
(Methods consuming Credits weight without execution evidence) is
flagged for Moderation review.

---

## 6. Empirical anchor

The federation's audit chain of `method_specification` Contributions
+ signed execution evidence (work logs, capture artifacts,
substrate resource observations) is the public record. The
`SAFETY_BATTERY_CI_LOOP.md` capture+interpret pattern is the
prototype implementation of this primitive's execution-verification
layer for safety-battery Methods; generalization to other Method
classes follows the same pattern.

The HuggingFace corpus (`CIRISAI/reasoning-traces`) will extend to
include Method execution traces, enabling third-party reconstruction
of federation-level operational practice.

---

## 7. Open questions (deferred to v0.1 cut-time)

1. **`execution_log_format` schema registry.** Where do federation-
   wide accepted log schemas live? Default v0.1: per-Method
   reference to a published schema URL.
2. **Resource-commitment verification surfaces.** Which adjacent
   systems (compute meters, funding ledgers) does the federation
   trust by default? Per-deployment policy.
3. **Method-execution-failure handling.** Default: honest failure
   triggers Reconsideration; deliberate spoofing triggers Slashing
   (P9). The threshold between them is per-deployment policy.
4. **Cross-substrate Method portability.** A Method at A4
   institutional with `portability_claims[]` toward A3 individual:
   what additional witness diversity is required for the
   portability claim itself?
5. **Method-Approach re-binding.** When an Approach is superseded
   (per `APPROACH_PRIMITIVE.md` §5.2), do linked Methods
   automatically re-bind to the new Approach, deprecate, or require
   explicit re-binding Contributions?
6. **Method execution under sub-federation branching.** When the
   federation branches (per `APPROACH_PRIMITIVE.md` §5.3), which
   Methods follow which branch?

---

## 8. Lifecycle

Per `MISSION.md` §1.4. Spec → Impl (`method_specification`
Contribution kind defined in `SCHEMA.md`, execution-verification
in `ciris-lens-core` extending the `SAFETY_BATTERY_CI_LOOP.md`
pattern) → Deployed (pilot) → Deployed (folded).

---

## 9. References

- v2 paper: Piece 2 (α−γM dynamics; γM substrate-typing),
  §sec:open-research (sequential per-rung, each rung's γM
  substrate-typed and locally maintained).
- `GOAL_PRIMITIVE.md` (Level 0 architectural commitments inherited).
- `APPROACH_PRIMITIVE.md` (Level 1; Methods instantiate Approaches).
- `DECISION_HIERARCHY.md` (cross-level DAG).
- `SAFETY_BATTERY_CI_LOOP.md` (the capture+interpret execution-
  verification pattern this primitive generalizes).
- `MISSION.md` v1.0 §2 (P9 Slashing, P10 Witness Diversity,
  P11 Reconsideration), §3.4 (voting).
- `SUBSTRATE_INTEGRATION.md` (typed-writes + audit chain access).

---

## 10. Discipline notes

- **γM as the framework-grounded type is the load-bearing
  identification.** Piece 2 names γM as substrate-typed maintenance
  work; the Method Primitive instantiates that at the federation
  tier. The identification ships at the contract surface.
- **Substrate-typing is framework-distinctive content.** Each rung
  has its own γM character; cross-rung portability requires explicit
  attestation. This is v2 framework content (post-F-11 sequential
  per-rung re-grounding, §sec:open-research) that the Method
  Primitive operationalizes.
- **Execution verifiability is mostly standard signed-evidence
  governance.** The framework's contribution at this layer is the
  *requirement* that γM be actual work (Piece 2's "the corridor is
  a doing, not a having"); the *mechanism* (signed logs, capture
  artifacts, substrate resource observations) is standard
  federation governance with framework-grounded objects.
- **Slashing-decoupling from execution failure is a CIRISNodeCore
  engineering commitment.** The framework does not derive
  "honest failure ≠ malfeasance"; that's a federation-governance
  call this FSD makes explicit so the Method Primitive cannot be
  weaponized as a Slashing trigger by anyone who dislikes a Method's
  proposer.
