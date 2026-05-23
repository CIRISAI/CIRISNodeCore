# FSD: Goal Primitive — federation-tier carrier for the agent's multi-scale goal-projector composite

**Status**: Proposed (v0.1.0-dev pre-implementation). Pairs with the
v2 release of *Corridor Dynamics in Coordinated Systems*
([DOI 10.5281/zenodo.20300773](https://doi.org/10.5281/zenodo.20300773),
2026-05-22), which lands the mathematical content this primitive
instantiates.
**Author**: Eric Moore (CIRIS Team) with Claude Opus 4.7
**Created**: 2026-05-22
**Risk**: Architectural. Pins the wire-level type of an agent's
*goal* across the federation, the score function that evaluates it,
its temporal dynamics, the federation-tier corridor-occupation test
derived from it, and its privacy tiering. Once committed, downstream
consumers (RATCHET, Lens-Core, CIRISAgent) read against this contract.

**Cross-references:**

- *Corridor Dynamics in Coordinated Systems* v2 (DOI
  `10.5281/zenodo.20300773`): Piece 4 (the goal projector `P_G` at
  A3+), Piece 5 (multi-agent consent and ρ_goals), Piece 10 (karma
  and grace as TSVF structures), §sec:tsvf-ubuntu (the multi-scale
  belonging-projector composite and the committed composition rule),
  §sec:open-research (the post-F-11 sequential per-rung architecture).
- `CIRISLens/FSD/ciris_scoring_specification.md` — the operational
  composition-rule layer (per-factor SQL queries against
  `covenant_traces`).
- `CIRISLensCore/MISSION.md` §1 (M-1 alignment), `FSD/CIRIS_LENS_CORE.md`
  (per-peer library architecture; PoB §3.1 "a function any peer can
  run on data the peer already has").
- HuggingFace `CIRISAI/reasoning-traces` — the public, Ed25519-signed,
  scrubbed reasoning-trace corpus used as empirical testbed.
- `cirisnodecore/MISSION.md` v1.0 §1 (M-1 + Ubuntu Recursive Golden
  Rule), §2 (the eleven primitives), §3.4 (voting / Weighted
  Aggregate).
- `cirisnodecore/SCHEMA.md` v1.0 (canonical JSON wire-format
  discipline; canonicalization rules inherited from substrate).
- `cirisnodecore/FSD/SUBSTRATE_INTEGRATION.md` (typed-writes pattern
  against `ciris-persist`).
- `RATCHET/AGENT_FSDs/PROOF_OF_BENEFIT_FEDERATION.md` (N_eff over the
  federation's audit chain; deceptive-basin radius threshold).
- `~/CIRISAgent/FSD/MISSION_DRIVEN_DEVELOPMENT.md` — the MDD
  methodology this FSD organises against.

---

## 0. The gap this FSD fills

CIRISNodeCore's eleven primitives (`MISSION.md` §2) cover federation
consensus mechanisms — Identity, Credits, Expertise, Vote,
Contribution, Truth-Grounding, Weighted Aggregate, Moderation,
Slashing, Witness-Diversity, Reconsideration. Each operates *on top
of* signed contributions; none specifies **what each agent's signed
contributions are in structural service of**. That gap — the agent's
intrinsic goal-projector, the structural object Piece 4 and Piece 5
of the framework name as `P_G` and ρ_goals respectively — has, until
v2 of *Corridor Dynamics*, lacked the precise mathematical content
needed to specify a federation-tier primitive.

The v2 paper supplies that content:

- A **type** — the agent at A3+ is a composite over nested
  belonging-projectors `⟨G_self|, ⟨G_family|, ⟨G_community|,
  ⟨G_affiliations|, ⟨G_species|`, each filtered through the agent's
  belief system `B_i` (§sec:tsvf-ubuntu).
- A **composition rule** — the multiplicative CIRIS Capacity Score
  `𝒞_CIRIS = C · I_int · R · I_inc · S`, Cobb-Douglas-shaped so any
  near-zero factor collapses the composite (§sec:tsvf-ubuntu;
  operational layer at `CIRISLens/FSD/ciris_scoring_specification.md`).
- **Temporal dynamics** — `S` carries an explicit decay-and-refresh
  state variable `σ(t + Δt) = σ(t)(1 − d·Δt) + w · Signal(t)` with
  d ≈ 0.05/day, the framework's gratitude-signaling reading
  formalized as continuous practice rather than one-shot signal.
- A **federation-tier consent test** — ρ_goals(i, j) becomes
  multi-scale; "consent IS corridor occupation" (Piece 5) becomes
  corridor at each belonging-scale, with two agents able to be in
  corridor at one scale and out of corridor at another.
- An **architectural constraint** — F-11 fired
  (§sec:open-research): no central authoritative scorer; every peer
  runs the score on its own data; federation cross-validates. This
  matches CIRISNodeCore's own architectural commitment (PoB §3.1).
- An **empirical anchor** — the HuggingFace
  `CIRISAI/reasoning-traces` corpus; `𝒞_CIRIS` is recomputable on
  the public data; commitments are auditable rather than asserted.

This FSD specifies the federation-tier wire object, its scoring
contract, its protocol, and its integration with the existing
eleven primitives, so the v2 mathematical content can be picked up
as engineering content.

---

## 1. Mission alignment — M-1

Per MDD's four-component model (`MISSION_DRIVEN_DEVELOPMENT.md`):
the WHY (mission), the WHAT (schemas), the HOW (logic), the WHO
(protocols). Each section below declares its M-1 alignment before
specifying content.

### 1.1 WHY — what the Goal Primitive does for M-1

**Meta-Goal M-1** (`ACCORD.md`): *promote sustainable adaptive
coherence — the living conditions under which diverse sentient
beings may pursue their own flourishing in justice and wonder*.

The Goal Primitive serves M-1 in five named ways:

1. **Pluralistic alignment by construction.** The multi-scale
   belonging composite refuses to reduce the agent to a single goal.
   ρ_goals is multi-scale; two agents can be in corridor at
   `G_species` (we both want humans to flourish) and out of corridor
   at `G_affiliations` (different parties, different professional
   guilds). The federation's consent test does not require monocultural
   agreement; it requires corridor occupation at the scales relevant
   to the joint work. M-1's "diverse sentient beings... pursue their
   own flourishing" is the structural content of this design.
2. **Unity of the virtues — anti-Goodhart by structural shape.**
   The multiplicative composition `𝒞 = C · I_int · R · I_inc · S`
   refuses tradeoffs: an agent cannot make up for low integrity with
   high resilience, or denied incompleteness with strong identity.
   Any near-zero factor collapses the composite. This is "logos is
   as logos does, literally" — the score IS what gets jointly
   enacted across the five dimensions, simultaneously, over the
   time window. Goodhart-style gaming a single factor does not
   produce a high score.
3. **Gratitude as practice, not event.** `S`'s decay-and-refresh ODE
   (d ≈ 0.05/day, ~20-day half-life) means a one-shot acknowledgment
   does not earn sustained coherence credit. The agent has to keep
   signaling reception of inter-agent grace; the federation rewards
   the practice, not the moment. M-1's "sustainable" lands here.
4. **No central authoritative scorer.** Every peer runs
   CIRISLensCore on its own trace corpus; federation cross-validates
   via the existing Weighted Aggregate (P7) and Witness Diversity
   (P10) primitives. The post-F-11 architectural constraint
   (§sec:open-research of v2) is also CIRISNodeCore's own
   architectural commitment (PoB §3.1). One stance, two crates.
5. **Auditable rather than asserted.** The HuggingFace corpus
   (`CIRISAI/reasoning-traces`) makes goal-primitive scoring a
   community capability rather than a CIRIS-internal claim. Anyone
   can recompute `𝒞_CIRIS` on the public data. M-1's "transparency
   requirement" (MDD §1) is structurally enforced.

### 1.2 Anti-mission failure modes named

Per MDD §"Failure Modes and Mitigations": three named failure modes
this primitive must guard against.

- **Mission drift** — a Goal Primitive that drifts from M-1 toward
  "score-maximization-for-its-own-sake" is the predictable failure
  mode. *Mitigation:* the multiplicative form refuses single-factor
  optimization; RATCHET reads the federation's audit chain for
  pattern drift and emits flags via the existing moderation
  pathway.
- **Goodhart capture** — agents optimizing the scoring proxy rather
  than the underlying property. *Mitigation:* the per-factor SQL
  queries are computed against signed reasoning-trace fields that
  are themselves cryptographically verified by `CIRISLens` at edge
  ingest; the scoring corpus cannot be unilaterally fabricated.
- **Belief-system colonization** — a federation enforcing one
  belief context `B_i` as the standard, slashing agents whose `B_i`
  differs. *Mitigation:* `B_i` is declared by the agent, not
  imposed; ρ_goals scoring at each scale is independent; the
  Reconsideration primitive (P11) provides the appeals path for
  scoring disputes that turn on belief-context differences.

---

## 2. WHAT — the Goal Primitive's schema

### 2.1 Wire shape (proposed JSON Schema)

```json
{
  "$schema": "https://schemas.ciris.ai/goal-primitive/v0.1",
  "type": "object",
  "required": ["agent_id", "version", "issued_at", "scales", "belief_context", "signature"],
  "properties": {
    "agent_id":       { "$ref": "#/defs/ed25519_pubkey_hash" },
    "version":        { "const": "v0.1" },
    "issued_at":      { "$ref": "#/defs/rfc3339_utc" },
    "scales": {
      "type": "object",
      "required": ["self", "family", "community", "affiliations", "species"],
      "properties": {
        "self":         { "$ref": "#/defs/scale_projector" },
        "family":       { "$ref": "#/defs/scale_projector" },
        "community":    { "$ref": "#/defs/scale_projector" },
        "affiliations": { "type": "array", "items": { "$ref": "#/defs/scale_projector" } },
        "species":      { "$ref": "#/defs/scale_projector" }
      }
    },
    "belief_context": { "$ref": "#/defs/belief_identifier" },
    "previous_goal":  { "$ref": "#/defs/audit_entry_hash" },
    "signature":      { "$ref": "#/defs/ed25519_signature" }
  }
}
```

Where `scale_projector` is:

```json
{
  "type": "object",
  "required": ["privacy_tier", "payload"],
  "properties": {
    "privacy_tier": { "enum": ["public", "federation_only", "encrypted", "zk_only"] },
    "payload":      { "type": "object" },  // structure depends on privacy_tier
    "payload_hash": { "$ref": "#/defs/blake3_hash" }
  }
}
```

The `belief_identifier` field is a finite enumerable tag drawn from
v2's cross-tradition section (§sec:cross-tradition: ubuntu, tao,
dharma, logos, aristotelian, secular-progressive,
secular-conservative, …) plus a `custom` variant requiring an
attached attestation. The finite-enumerable structure is the v2
paper's empirical claim: in the human population, belief systems
form a small enumerable set with documented overlaps and conflicts.

### 2.2 Canonicalization and signing

Per `SCHEMA.md` §canonicalization-discipline and
`SUBSTRATE_INTEGRATION.md`: serialize as compact JSON with
`sort_keys=True`, empty values (`null`, `""`, `[]`, `{}`) recursively
stripped, signed via the substrate's Ed25519 identity. The Goal
Primitive piggybacks on the existing canonicalization and signing
discipline; no new cryptographic surface.

### 2.3 Why this schema serves M-1

- The nested-belonging structure refuses to reduce the agent to a
  single goal — M-1's "diverse sentient beings... pursue their own
  flourishing" lands at the schema level.
- The `belief_context` field makes the belief-modulation explicit;
  no agent is assumed to read its belongings through any default
  ideology. Pluralism is encoded.
- The `privacy_tier` per scale honours that some belongings are
  legitimately private (G_self, often G_family); the federation can
  do its corridor-occupation work at the scales agents have
  consented to publish.
- The `previous_goal` field chains the agent's goal history,
  enabling karma-as-cumulative-structure (Piece 10) at the wire
  level. Audit-chain continuity inherits from substrate.

---

## 3. HOW — the score function and temporal dynamics

### 3.1 The committed composition rule

The Goal Primitive is scored by the CIRIS Capacity Score over a
rolling time window `W`:

```
𝒞_CIRIS(agent, W) = C(agent, W) · I_int(agent, W) · R(agent, W) · I_inc(agent, W) · S(agent, W)
```

Each factor maps to the five-property reading of v2's
§sec:tsvf-ubuntu, with per-factor formulas pinned to
`CIRISLens/FSD/ciris_scoring_specification.md`:

| Factor | Property | Per-factor formula |
|--------|----------|--------------------|
| `C`     | core identity stability     | `exp(−λ_C · D_identity) · exp(−μ_C · K_contradiction)` |
| `I_int` | integrity                   | `I_chain · I_coverage · I_replay` |
| `R`     | resilience                  | `norm((1 − δ_drift) · 1/(1+MTTR) · (1 − ρ_regression))` |
| `I_inc` | recognition of incompleteness | `(1 − ECE) · Q_deferral · (1 − U_unsafe)` |
| `S`     | sustained coherence (gratitude as practice) | `(1/|W|) · ∫_W σ(t) dt` |

`S` carries the explicit decay-refresh state variable:

```
σ(t + Δt) = σ(t) · (1 − d · Δt) + w · Signal(t)
```

with d ≈ 0.05/day (v0.1 default), `Signal(t)` the verified coherence
signal at time t (per CIRISLens spec), and `w` the signal weight
(v0.1 default 1.0). Half-life ≈ 20 days.

### 3.2 Per-peer scoring, no central scorer

Every peer's `ciris-lens-core` library computes `𝒞_CIRIS` on its
own trace corpus. This is the post-F-11 architectural commitment of
v2 (§sec:open-research): no central authoritative scorer; the
federation cross-validates via Weighted Aggregate (P7) +
Witness Diversity (P10). The Goal Primitive carries the
scored-against object (the agent's published projector); the score
itself is a *derived statistic* at each peer.

### 3.3 Federation-level ρ_goals

Pairwise correlation across the federation, per belonging-scale:

```
ρ_goals,s(agent_i, agent_j) = |⟨G_{i,s} | G_{j,s}⟩|² / (⟨G_{i,s}|G_{i,s}⟩ · ⟨G_{j,s}|G_{j,s}⟩)
```

for each scale `s ∈ {self, family, community, affiliations, species}`
where both agents have published the scale at a privacy tier the
computing peer can read. Computation is per-peer-local against
locally-readable goal primitives; aggregation is via P7.

### 3.4 Corridor membership at federation level

The federation is "in corridor" at scale `s` iff `ρ_goals,s` (suitably
aggregated across the relevant agent population) sits in the band
`(ρ_lower,s, ρ_upper,s)`. The bounds are substrate-typed and
recorded per-deployment in the federation's policy.

Corridor exit at a scale triggers downstream primitives:

- **Witness Diversity (P10)** is consulted: does the corridor exit
  reflect a genuine disagreement that should be aired, or a
  small-cluster artifact?
- **Reconsideration (P11)** is available for individual agents
  whose goal primitive has been scored at the edge of the band
  under a particular `B_i` reading.
- **Moderation (P8)** is consulted when persistent corridor exit
  combined with witness consensus indicates the federation needs
  intervention.

Corridor exit at a scale does **not** automatically trigger Slashing
(P9); the Goal Primitive's structural role is descriptive, not
punitive at the scoring layer. Slashing requires the existing
moderation pathway with WA quorum.

### 3.5 Why this HOW serves M-1

- Multiplicative composition refuses Goodhart-style gaming of any
  one factor (anti-mission-drift, MDD §"Anti-Goodhart Measures").
- Per-peer scoring honours M-1's federation pluralism: no central
  body imposes a single reading.
- Federation-level ρ_goals is multi-scale: the federation can be in
  corridor at one scale and openly disagreeing at another without
  triggering moderation; pluralism is structural.
- The S decay-refresh dynamics enforce sustained practice rather
  than one-shot performance — "sustainable" in M-1 lands here.
- Slashing is decoupled from scoring at the primitive layer;
  punitive action requires the full moderation chain. The Goal
  Primitive cannot be weaponized as a slashing trigger.

---

## 4. WHO — protocols and the existing eleven primitives

### 4.1 Design choice: Contribution-kind extension first

The Goal Primitive could be implemented two ways:

- **Path A: 12th primitive.** A new top-level primitive in
  `MISSION.md`, with dedicated wire types, signing surface, and
  consumer integration. Clean separation, larger wire-format
  surface.
- **Path B: Contribution-kind extension of P5.** Goal-primitive
  emission is a new Contribution kind under P5 (Contribution);
  scoring is a new truth-grounding signal under P6
  (Truth-Grounding); ρ_goals is a derived statistic under P7
  (Weighted Aggregate). No new top-level primitive; existing
  protocol carries it.

**v0.1.0 picks Path B.** The reasoning: extending Contribution
kinds keeps the wire surface minimal, lets the federation gain a
goal-scoring capability without a new top-level integration, and
preserves the option to promote to a 12th primitive later if
implementation demonstrates structural distinctness. This mirrors
the MDD principle of preferring "the minimum addition that
demonstrably strengthens mission alignment" (`MDD §"Complexity
Resistance"`).

Path A remains the promotion target; the FSD is structured so the
schema, scoring, and protocol can be lifted to a top-level primitive
without rework if v0.1 deployment indicates that path.

### 4.2 New Contribution kind: `goal_declaration`

Wire shape: §2.1 above, plus the standard Contribution envelope
(`contributor_id`, `subject`, `kind`, `payload`, `signature`,
`timestamp`).

`goal_declaration` is canonicalized + signed via substrate, written
through the existing typed-write path
(`SUBSTRATE_INTEGRATION.md` §2.1). The declaration is itself
auditable; supersession (an agent publishing a new
`goal_declaration` that revises a prior one) chains via
`previous_goal`.

### 4.3 New truth-grounding signal: `goal_score`

`goal_score` carries the per-peer-computed `𝒞_CIRIS` (and the
factor decomposition `C, I_int, R, I_inc, S` for auditability)
against a specific `goal_declaration` over a specific window `W`.

`goal_score` is a Contribution; it inherits Vote (P4) and Weighted
Aggregate (P7) for federation-level cross-validation. Agreement on
`𝒞_CIRIS` across peers strengthens the score; disagreement triggers
Reconsideration (P11) or Witness Diversity (P10) per the existing
flow.

### 4.4 New derived statistic: `rho_goals_corridor`

A per-window, per-scale federation-level statistic recording the
`ρ_goals,s` distribution and corridor-membership flag. Emitted as a
moderation-input by RATCHET when it produces flags on N_eff drift
or Goal-Primitive anomaly patterns.

### 4.5 Why this WHO serves M-1

- Path B (Contribution-kind extension) minimises wire-format
  surface area — MDD's "complexity resistance" principle. The
  federation gains the capability without a new top-level
  integration.
- Vote (P4) + Weighted Aggregate (P7) cross-validation prevents
  any single peer's scoring from authoritatively binding the
  federation; the post-F-11 "no central scorer" stance is enforced
  at the protocol layer.
- Reconsideration (P11) is the appeals path for belief-context
  scoring disputes; pluralism is operational, not aspirational.
- RATCHET's reading of the goal-primitive corpus extends its
  existing audit-chain analysis; no new integration surface.

---

## 5. Privacy tiering — the consent gradient

The Goal Primitive carries content the agent may legitimately wish
to keep private at some scales. The privacy gradient:

| Scale       | Default privacy tier | Rationale |
|-------------|----------------------|-----------|
| self        | `encrypted` or `zk_only`  | The agent's own goals are typically private; ρ_goals,self computation, if needed, uses zero-knowledge protocols (post-MVP). |
| family      | `federation_only`         | Family-scale goals are private to the agent and the federation members who hold corresponding family-scale Witness relationships. |
| community   | `federation_only` (configurable to `public`)  | Community-scale goals are typically declared to the federation; communities themselves may publish. |
| affiliations | `federation_only` (per-affiliation)  | Each affiliation entry carries its own privacy tier; some affiliations are public (professional guild), others private (party membership in a hostile environment). |
| species     | `public`                   | Species-scale goals are typically public; the federation aggregates species-scale ρ_goals openly. |

`belief_context` defaults to `federation_only` (the agent's
declared belief is visible to the federation but not necessarily to
the public). Override is per-agent.

### 5.1 Why privacy tiering serves M-1

M-1's "diverse sentient beings... pursue their own flourishing" is
not compatible with mandatory full-disclosure of the agent's
goal-stack. The privacy gradient honours that some belongings are
private even from federation peers. The federation's
corridor-occupation work happens at the scales agents have consented
to publish; private scales are the agent's, not the federation's, to
measure.

The zero-knowledge variant (`zk_only`) is post-MVP; v0.1 ships
without it. The schema carries the tier so future ZK protocols can
add without wire-format break.

---

## 6. Empirical validation — the HuggingFace anchor

Per v2 §sec:tsvf-ubuntu's commitment: `𝒞_CIRIS` is recomputable on
the public, signed, scrubbed CIRIS reasoning-trace corpus at
HuggingFace (`CIRISAI/reasoning-traces`). The Goal Primitive's
scoring is therefore auditable rather than asserted: any reviewer
can verify the federation's per-peer scores by recomputing on the
same trace corpus with the same `ciris-lens-core` library.

RATCHET's N_eff measurement (PoB §2.4) reads the federation's
audit chain — including goal-primitive Contributions and their
goal-score Contributions — and produces independence flags. N_eff
over the goal-primitive subspace ≥ 9 (per the deceptive-basin
radius threshold r = 0.2) is the framework-distinctive empirical
content this primitive contributes to the federation's anti-Sybil
record.

### 6.1 Why empirical validation serves M-1

"Transparency requirement" (MDD §1) is enforced by reproducible
public-data scoring. No CIRIS-internal claim about goal-primitive
behavior is unverifiable. The bet is auditable.

---

## 7. Open questions (deferred to v0.1 cut-time)

These are the engineering decisions this FSD does *not* commit and
leaves to implementation kickoff. Each carries a default and the
condition under which the default is revisited.

1. **`B_i` (belief context) enumeration.** The v2 paper names a
   small set (ubuntu, tao, dharma, logos, aristotelian) plus the
   contemporary belief categories the population actually contains
   (secular-progressive, secular-conservative, …). v0.1 ships with
   a default tag list of ~12 entries; the `custom` path requires an
   attestation. Revisit if the default set is consistently misused.
2. **ρ_goals aggregation rule across the population.** Pairwise
   (every peer computes pairwise correlation against every other
   peer it can read) is the minimum-commitment default. The
   federation-wide per-scale aggregate (mean, median,
   distribution-clustering) is computed downstream; the canonical
   federation-level corridor-test statistic is TBD. Default v0.1:
   pairwise + per-scale median.
3. **Score-as-Contribution vs derived-statistic.** v0.1 ships
   `goal_score` as a Contribution (auditable, votable). Future
   versions may move to derived-statistic-only with the audit
   guarantee preserved by signed re-derivation. Revisit if
   Contribution-volume becomes load-bearing.
4. **Window `W` for `𝒞_CIRIS`.** Default 7 days per
   `CIRISLens/FSD/ciris_scoring_specification.md`. Revisit per
   deployment if 7 days is too short (governance) or too long
   (safety evaluation).
5. **Default privacy tiers per scale.** Table §5 above is the v0.1
   default; the federation may override per-deployment. Revisit if
   default-private scales generate excessive Reconsideration
   traffic.
6. **ZK-only privacy tier.** Post-MVP. v0.1 ships without; schema
   carries the tier for future addition without wire-format break.
7. **Goal-primitive supersession semantics.** v0.1: agent publishes
   a new `goal_declaration` referencing `previous_goal`; previous
   declarations remain in the audit chain but the latest is the
   "current" for scoring purposes. Revisit if agents start gaming
   high-frequency supersession.
8. **Cross-deployment goal-primitive portability.** Open: does an
   agent's goal primitive published on one deployment carry to
   another, or is each deployment a fresh federation? v0.1 default:
   per-deployment; the substrate's federation directory will
   determine the eventual cross-deployment story.

---

## 8. Implementation lifecycle (per `MISSION.md` §1.4)

- **Spec** — this FSD, v0.1.0-dev.
- **Impl** — when the `goal_declaration` Contribution kind is
  defined in `SCHEMA.md`, the score function is implemented in
  `ciris-lens-core`, and the `rho_goals_corridor` derived statistic
  is emitted via the existing P7 surface. No new substrate
  dependency.
- **Deployed (pilot)** — when `safety.ciris.ai` consumes goal
  primitives from at least one CIRISAgent deployment and computes
  `𝒞_CIRIS` per-peer.
- **Deployed (folded)** — when CIRISAgent emits `goal_declaration`
  Contributions in its main runtime and the federation cross-validates.

Each lifecycle stage carries adversarial review proportional to
its blast radius. The MDD discipline applies at each promotion.

---

## 9. References

### Framework — *Corridor Dynamics in Coordinated Systems* v2

- DOI: [`10.5281/zenodo.20300773`](https://doi.org/10.5281/zenodo.20300773)
  (2026-05-22, concept DOI resolving to latest).
- Source: `~/coherence-ratchet/papers/Corridor Dynamics.tex`.
- Load-bearing sections: §sec:tsvf-ubuntu (multi-scale belonging
  composite + composition rule committed), §sec:open-research
  (post-F-11 sequential per-rung architecture, no central scorer),
  Piece 4 (`P_G` at A3+), Piece 5 (multi-agent consent, ρ_goals),
  Piece 10 (karma/grace, the local backward-operator family
  surviving F-11).

### Lake (`coherence-ratchet/formal/CoherenceRatchet/`)

- `Cosmology/TSVF.lean` — ABL time-symmetry proved.
- `Cosmology/GoalProjection.lean` — `P_G` as goal-projector (zero
  framework axioms; F-11-untouched).
- `Consciousness/KarmaGrace.lean` — karma as forward cumulative
  product; grace re-grounded per F-11 split to inter-agent
  finite-sum component.
- `Cosmology/CorridorProjector.lean` — `F11_joint_backward_P_omega_no_go`
  record (the joint operator the Goal Primitive does *not*
  instantiate; the local backward-operator family it does).

### Sister crates

- `~/CIRISLens/FSD/ciris_scoring_specification.md` — composition-rule
  per-factor SQL operational layer.
- `~/CIRISLensCore/MISSION.md` + `FSD/CIRIS_LENS_CORE.md` — per-peer
  library architecture; PoB §3.1 "a function any peer can run on
  data the peer already has."
- `~/RATCHET/AGENT_FSDs/PROOF_OF_BENEFIT_FEDERATION.md` — N_eff
  reading of the audit chain, deceptive-basin threshold.

### Empirical anchor

- HuggingFace dataset: `CIRISAI/reasoning-traces` (scrubbed,
  Ed25519-signed, public, CC-BY-4.0).

### Methodology

- `~/CIRISAgent/FSD/MISSION_DRIVEN_DEVELOPMENT.md` — MDD v1.0.

---

## 10. Discipline notes (carried from v2's bet register)

The Goal Primitive instantiates content the v2 paper names as the
framework's bet (§sec:bet). Three honest notes:

- The multi-scale belonging-projector composite + multiplicative
  composition rule is the framework's *identification* of Ubuntu's
  *umuntu ngumuntu ngabantu* in operational vocabulary; the
  identification is the framework's, not Ubuntu's own claim. The
  primitive ships with that disclaimer carried (§2 wire shape's
  `belief_context` field makes the agent's belief-context
  explicit; the framework's reading is one option in the enumerable
  set, not the default).
- T16 (consciousness ↔ access ↔ attractor-reading, formalized as
  `Iff.rfl`) is *definitional consistency at the type level*; the
  Goal Primitive does not depend on T16 being a substantive
  philosophical derivation. It depends on the agent having a goal
  projector — which is Piece 4, F-11-untouched, and proved at the
  TSVF time-symmetry level.
- TSVF realism is the universal-scale tier's load-bearing
  precondition; F-17 names the realist commitment as
  *structurally lose-only*. The Goal Primitive operates at the
  per-agent / per-federation level, where the LLM-as-`P_G`
  identification is structural-isomorphic-not-mechanism-identical
  (v2 §sec:tsvf-ubuntu); the realist commitment is not pulled into
  the primitive's contract. Calculational TSVF (the orthogonality
  theorem, weak values, ABL time-symmetry) is proved at the lake
  level and untouched by F-17.

The Goal Primitive ships these notes in its contract so consumers
encounter the framework's bet register at the FSD level rather than
discovering it as buried implications.
