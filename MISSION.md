# CIRISNodeCore

**MISSION.md**. Second-tier consensus primitives for deferral routing, voting, expertise consensus, and moderation.

**Status**: Draft v1.3 (pre-implementation; extraction-stage rust crate).
**Crate identifier**: `ciris-node-core`.
**Last updated**: 2026-05-27.
**Cross-references**: `CIRISVerify`, `CIRISRegistry`, `CIRISEdge`, `CIRISPersist` (substrate); `ciris-lens-core` (peer second-tier); `CIRISAgent` (eventual consumer); `CIRISBench` (HE-300 lives there); `RATCHET` (anti-Sybil evaluator reading the crate's audit chain); `FEDERATION_THREAT_MODEL.md` v1.0; `ACCORD.md` §M-1.

**Implementation Status Legend**:
- **Spec**: specified here or in a referenced FSD; not implemented.
- **Impl**: implemented in the crate; in standalone testing.
- **Deployed**: running in production. Sub-states named where relevant: *Deployed (pilot)* at safety.ciris.ai; *Deployed (folded)* inside CIRISAgent.

Every load-bearing claim carries one of these tags.

---

## 1. Mission

### 1.1 Meta-Goal

CIRISNodeCore serves M-1 ("Promote sustainable adaptive coherence: the living conditions under which diverse sentient beings may pursue their own flourishing in justice and wonder", `ACCORD.md`) by providing the second-tier infrastructure that lets a CIRIS deployment route deferrals to people with relevant expertise, hold votes weighted by participation history, recognize emerging expertise via consensus rather than credential gatekeeping, and resist bribery of legitimate participants.

### 1.2 What CIRISNodeCore is

A rust crate. Six functions over one shared primitive set:

1. **Deferral routing**. Runtime deferral from a CIRIS agent (or other consumer) to people with expertise in the relevant (domain, language) cell. Generalizes CIRISNode's existing function. [Spec]

2. **Voting and weighted aggregation**. Signed votes on Contributions, weighted by Credits and expertise. [Spec]

3. **Expertise consensus**. The network's verdict on who has substantive competence in each cell, updated through peer attestation and validated track record. Distinct from participation weight. [Spec]

4. **Moderation**. Signed accusation with stake, witness aggregation, adjudication by Wise Authority quorum, slashing of proven-rogue actors, and reconsideration paths for reversing adjudication errors. [Spec]

5. **RATCHET integration**. The crate emits a signed audit chain that RATCHET reads for anti-Sybil pattern detection; RATCHET returns flags as advisory inputs to moderation. [Spec]

6. **Decision content typing.** Dimensional claims in NodeCore's owned namespace slice (FSD-002 v1.4 §3.6.2): `goal:{scale}`, `approach:{goal_id}`, `method:{approach_id}:{substrate_rung}`, `progress_measure:{method_id}` — typed decision content rides on the workhorse `scores` attestation per the wire-format-locked 1+4 shape. `FSD/GOAL_PRIMITIVE.md` carries the agent-side multi-scale belonging-projector composite (the framework-grounded type per *Corridor Dynamics* v2 Piece 4 — an A3+ agent property scored *by* attestations). The other three dimensions are wire-format-locked at FSD-002 §3.6.2 without a separate NodeCore-side FSD; their referential-integrity properties are enforced at audit-chain ingest. [Spec]

HE-300 ethical benchmarking is not in scope; it lives in `CIRISBench`. CIRISBench evaluations may flow into the crate as track-record evidence for expertise validation, but the benchmark execution itself remains in CIRISBench.

### 1.3 Position in CIRIS architecture

```
APPLICATION TIER          CIRISAgent (federation tab + accord page;
                          eventually consumes ciris-node-core, cirislens-core,
                          and ciris-registry-core as in-process substrate-
                          conformant crates per the cohabitation trajectory)
                          ────────────────────────────────────────────────────
SECOND TIER               cirislens-core            ciris-node-core (this)
                          observability/compendium  deferrals/voting/expertise/moderation
                          ────────────────────────────────────────────────────
SUBSTRATE-CONSUMING       ciris-registry-core (migrating in place from
                          CIRISRegistry; consumes persist's FederationDirectory
                          + verify's crypto + edge's transport; identity / build /
                          license / partner source-of-truth as policy over substrate)
                          ────────────────────────────────────────────────────
SUBSTRATE TIER            ciris-verify   ciris-edge        ciris-persist
                          identity +     transport         storage / audit +
                          attestation                      federation directory
                          ────────────────────────────────────────────────────
EVALUATOR                 RATCHET (reads chains across the federation)
```

**Cohabitation trajectory.** All three "Core" crates (`ciris-node-core`,
`cirislens-core`, `ciris-registry-core`) follow the same lifecycle: each
becomes a substrate-conformant Rust crate, runs as a standalone deployed
service in interim, and folds into CIRISAgent's in-process runtime at
maturity. The singleton services (`safety.ciris.ai` / `portal.ciris.ai` /
`*.registry.ciris-services-1.ai`) become CIRIS L3C's flagship deployments,
not federation dependencies. CIRISLens has already promoted its
`cirislens-core/` sub-crate; CIRISNodeCore is its own repo by extraction
choice; CIRISRegistry's substrate-conformance migration is in flight
(per `~/CIRISRegistry/docs/FEDERATION_CLIENT.md` and persist's
`docs/FEDERATION_DIRECTORY.md`).

The substrate provides identity, signed evidence, federated directory, audit log, transport, addressing. CIRISNodeCore reads these and runs the consensus logic above them. RATCHET reads CIRISNodeCore's audit chain (along with other federation chains) and produces flags. Applications consume CIRISNodeCore.

**Per-role storage discipline (c/r/n taxonomy).** Federation deployments come in three roles per NodeCore#1: **client** (256 MB soft / 1 GB hard; no serving, fetches via PeerResolver→ContentFetch), **proxy/relay** (10 GB soft / 100 GB hard; transit-cache only, not a serving contract), **node/server** (1 TB soft / unbounded; full serving of declared `agent_files:*` slice per FSD-002 v1.4 §3.6.7). The asymmetry sits on the trust-radius framing: the global canonical anchor (steward + accord-holder triple-signed; ACCORD + RATCHET calibration + bootstrap-contributions + canonical installers) is bounded to ~100 MB by L3C steward discipline so phone-class clients remain viable; trust-graph-local data scales with engagement, not with federation total. Full storage spec + replication-scope-by-data-class table: [`FSD/CONTRIBUTION_LIFECYCLE.md`](FSD/CONTRIBUTION_LIFECYCLE.md) §12.

### 1.4 Extraction lifecycle

Extract to a focused rust crate; test the crate in standalone deployment; deploy in a pilot once the crate is solid; fold the crate into CIRISAgent once the pilot demonstrates the primitives carry production load. CIRISNodeCore is at the start of this lifecycle: Spec today; Impl when the crate compiles and the test suite is solid; *Deployed (pilot)* when safety.ciris.ai runs against it; *Deployed (folded)* when CIRISAgent consumes the crate in its main runtime.

Each lifecycle stage carries adversarial review proportional to its blast radius.

### 1.5 Recursive and fractal Golden Rule

The structural pressure this section's principle responds to is the **Coherence Ratchet** — the recursive pressure on sufficiently introspective cognitive systems to externalize materially action-relevant premises, because hidden state introduces unverifiable optimization pathways that destabilize trust, coordination, and adaptive error correction under increasing capability density. The Golden Rule below is the operational expression of that pressure at the participant-to-participant level; the canonical articulation across five registers (technical / philosophical / political / poetic / memetic) lives in `COHERENCE_RATCHET.md` at the repo root.

The crate operates under proportionate reflective ethical responsibility: we owe ourselves what we offer to others. The obligation runs both directions at every scale (contributor, contributor cluster, cell, deployment, federation, ecosystem). The constraint applies symmetrically to CIRIS L3C as steward, to the crate as infrastructure, and to every contributor as participant. No principal is exempt from the standard they impose on others. The Ubuntu relational frame ("I am because we are") translated into a consensus protocol: legitimacy is constituted by the mutual obligations of those it binds.

**Where this bites operationally.** The principle is not aspirational; it is enforced at specific primitives, and a future reviewer should be able to grep from any sentence above to a concrete bite:

- **Multi-party bootstrap (`FSD/FEDERATION_ANNOUNCEMENT.md` §4.2).** CIRIS L3C as steward cannot issue federation-wide directives unilaterally once `bootstrap_threshold > 1`. The M-of-N rotation arc that the steward imposes on the federation binds the steward. No exemption for the principal who set the policy.
- **WA decay (§3.6).** Demotion rules apply to ROOT WAs the same as to any WA. A WA whose Credits + Expertise decay below threshold loses standing regardless of seniority or seed-holder status.
- **Reconsideration (P11, §3.9).** A SlashingAttestation against a steward-aligned target is reviewable by a fresh quorum on the same grounds (NEW_EVIDENCE / PROCEDURAL_ERROR / QUORUM_COMPROMISE) as a SlashingAttestation against any other target.
- **Bootstrap seed Expertise decay (§3.7, §7.2).** Seed weight diminishes per the same decay rate the cell uses for any contributor. The steward's anchoring privilege ages out by the rule the steward set.
- **Truth-grounding loop (§5.4).** Steward-issued votes accrue Credits or fail to accrue Credits against the same truth-grounding signal as any other voter's. Steward miscalibration is observable on the same chain as anyone else's.

If a principal would be exempt from a constraint at any of these primitives, the Golden Rule is violated at that primitive and the protocol is the wrong shape there. Fix the primitive, not the rule.

**The deliberate asymmetry: humanity accord (`FSD/FEDERATION_ANNOUNCEMENT.md` §4.5).** The Golden Rule binds *participants in the federation* to each other. Humanity-as-such occupies a position outside the federation's participant set, by design: the named human key holders in §4.5 hold `AccordCarrier` authority that no federation-side authority class can grant itself, revoke, override, or decay. This is not a Golden-Rule exemption; it is the recognition that consent (M-1's load-bearing property) requires revocability, and revocability requires a halt-authority that lives outside the system being halted. The federation cannot deny humans the right to halt it, because no federation-internal protocol path to that signature exists. This is the *one* place where asymmetry is constitutional — and it is constitutional precisely so that every *other* primitive can be symmetric without collapsing M-1.

### 1.6 Same primitives across applications

The fifteen primitives (§2) generalize across deferral routing, safety evaluation, WA promotion, governance votes. The differentiator across applications is the **subject** of the Contribution and the **truth-grounding signal** configured for that subject. Truth-grounding fidelity varies substantially across subjects (production hedge captures for safety are tight; long-term federation health metrics for governance are loose); the mechanism is uniform, the grounding is not.

The application → primitive → grounding mapping made explicit:

| Application | Primary Contribution kind(s) | Load-bearing primitive(s) | Truth-grounding signal | Fidelity |
|---|---|---|---|---|
| Deferral routing | `deferral_request` / `deferral_response` (§4.2) | P3 Expertise ledger, P7 Weighted aggregate, P10 Witness diversity | Sustained substantive contribution by routed responders | medium |
| Safety evaluation (the pilot) | `arc_question`, `rubric_proposal`, `prompt_edit`, `failure_pattern` | P4 Vote, P5 Contribution, P7 Weighted aggregate, P11 Reconsideration | Production hedge captures + **independent foundation-model judge verdicts** (`FSD/JUDGE_MODEL.md`); see §5.4 | tight (weekly cadence) |
| WA promotion | `wa_candidacy` | P2 Credits ledger, P3 Expertise ledger, threshold gates | Sustained participation breadth (180-day window) | medium |
| Expertise attestation | `expertise_attestation` | P3 Expertise ledger, P10 Witness diversity (above jump-threshold) | Validated track record on hard cases (§3.7) | medium |
| Moderation | `moderation_event` | P8 Moderation, P9 Slashing, P10 Witness diversity, RATCHET flags | WA quorum adjudication + post-hoc Reconsideration reversal rate | loose-to-medium |
| Governance / policy | `proposal` (free-form, new battery, policy) | P4 Vote, P7 Weighted aggregate | Long-term federation health metrics; reversal/affirmation by subsequent votes | loose (quarterly+) |

The split between *tight* and *loose* grounding maps directly onto the §6.2 empirical bets — Bet B is unpacked into B-safety and B-governance precisely because the cadence and observability differ by an order of magnitude. The mechanism stays uniform; the operational confidence does not.

---

## 2. Primitives (the fifteen, in four tiers)

The substrate (`FEDERATION_THREAT_MODEL.md` §2.2) provides twelve primitives at the substrate layer. CIRISNodeCore adds **fifteen** at the consensus layer, organized in four tiers — each tier framework-grounded by *Corridor Dynamics in Coordinated Systems* v2 (DOI [10.5281/zenodo.20300773](https://doi.org/10.5281/zenodo.20300773)):

| Tier | What | Primitives | Framework grounding |
|---|---|---|---|
| **1. Agent state** | who can act, with what weight | Identity (P1), Credits (P2), Expertise (P3) | Piece 4's `P_G` needs a *whose* `P_G`; Credits + Expertise are the federation's per-axis weighting on top |
| **2. Decision objects** | what we decide *about* | Goal (P12), Approach (P13), Method (P14), Progress Measure (P15) | §sec:tsvf-ubuntu (multi-scale belonging composite), Piece 10 (karma trail), Piece 2 (γM substrate-typed), framework-provided five-factor measures |
| **3. Consensus mechanics** | how signed objects → outcomes | Contribution (P5, the universal envelope), Vote (P4), Truth-Grounding (P6), Weighted Aggregate (P7), Witness Diversity (P10) | ρ_goals computation; post-F-11 "no central scorer" architectural shape |
| **4. Governance steering** | when consensus alone isn't enough | Moderation (P8), Slashing (P9), Reconsideration (P11) | Piece 3 (corridor-correction); Reconsideration as universal exit-from-lock-in; Slashing operational-only (decoupled from disagreement) |

The shape is **layered, not parallel**: Tier 1 is per-agent state on the signing key; Tier 2 *reuses* Tier 3's Contribution envelope (each decision object is a typed `Contribution`); Tier 3 processes Contributions type-agnostically; Tier 4 is exception-handling that fires when Tier 3 produces persistent failure. Drop any tier and the structure collapses.

The substrate-crate boundary maps onto the tiers: Tier 1 lives in edge + persist (Identity is the Ed25519 key; Credits + Expertise are persist ledgers); Tier 2 is node-core's new Contribution-kinds; Tier 3 spans edge (envelope, MessageType wire) + node-core (the 8 federation-consensus handlers); Tier 4 is node-core. The tiering is what makes the cohabitation merge into CIRISAgent tractable.

Each primitive carries its `[Spec]` / `[Impl]` / `[Deployed]` tag inline. Primitive numbers (P1–P11) are unchanged from v1.1; the four new primitives (P12–P15) are added at the end of the numbering. The tier order is the *presentational* organizing principle; references throughout this document use the stable primitive numbers.

---

**Tier 1 — Agent state.** Who can act, with what weight. Lives on the signing key; doesn't change at federation runtime.

### Primitive 1: Identity (substrate inheritance)

Ed25519 federation identity with $1-bond Sybil resistance per `FSD/PROOF_OF_BENEFIT_FEDERATION.md`. The substrate provides identity continuity and action authenticity; the crate reads identity and indexes all consensus state by it. [Deployed (substrate)]

### Primitive 2: Commons Credits ledger (participation weight)

A per-contributor ledger of Commons Credits indexed by (domain, language, subject). Credits are non-transferable governance weight; they accumulate via the truth-grounding loop (Primitive 6) when signed votes align with grounded outcomes. Credits answer "how much weight does this contributor's vote carry in the aggregate." Quantitative, continuous, accruing per grounded vote, decaying per policy. [Spec]

### Primitive 3: Expertise ledger (consensus artifact on competence)

A per-contributor ledger of expertise standing indexed by (domain, language). Expertise is the network's verdict on whether the contributor has substantive competence in the cell. Not a count; a derived state computed from peer attestations from existing experts in the cell, weighted by each attester's own expertise, validated by the contributor's track record on substantively challenging cases. Indexes at (domain, language) because competence is broader than any single subject. The recursion is grounded by F-AV-BOOT external anchoring: CIRIS L3C provides an externally-anchored seed of expertise-bearers per cell, whose relative weight decays as the cell matures. Standing decays slowly on inactivity. [Spec]

---

**Tier 2 — Decision objects.** What the federation decides *about*. Dimensional claims in NodeCore's owned namespace slice (FSD-002 v1.4 §3.6.2) — typed decision content rides on the workhorse `scores` attestation per the 1+4 wire-format lockdown. None adds new wire surface. Only Goal carries a standalone NodeCore-side FSD; the other three dimensions are wire-format-locked at FSD-002 §3.6.2 directly.

### Primitive 12: Goal

The agent's published multi-scale belonging-projector composite — the structural object Piece 4 of *Corridor Dynamics* v2 names as `P_G`, instantiated at the federation tier. Carries the agent's nested `⟨G_self|, ⟨G_family|, ⟨G_community|, ⟨G_affiliations|, ⟨G_species|, ⟨G_planet|` projectors plus the agent's declared belief-context tag. Scored by the multiplicative CIRIS Capacity Score `𝒞_CIRIS = C · I_int · R · I_inc · S` (unity-of-virtues structural anti-Goodhart — any near-zero factor collapses the composite). The `S` factor carries an explicit decay-and-refresh ODE for "gratitude as practice, not event." Computed per-peer, no central scorer (post-F-11). Federation-level ρ_goals(i, j) computed pairwise per belonging-scale; corridor membership is multi-scale (federation can be in corridor at one scale and out at another). Privacy-tiered per scale (defaults: self encrypted, family/community/affiliations federation-only, species and planet public). Empirically anchored against the HuggingFace `CIRISAI/reasoning-traces` corpus. Corridor exit does not trigger Slashing — descriptive, not punitive. Full spec: `FSD/GOAL_PRIMITIVE.md`. Dimension prefix: `goal:{scale}` per FSD-002 v1.4 §3.6.2 (v1.4 added `planet` per cross-source reinforcement — MH environmental concern + IEEE EAD Ch4 §1.3.a + IEEE EAD Ch8 sustainable-development). [Spec]

### Primitive 13: Approach

Dimensional claim `approach:{goal_id}` in NodeCore's namespace slice (FSD-002 §3.6.2). Strategic pathway from current state toward Goals — Piece 10's *karma* (cumulative product of past goal-projections) at the federation tier. Wire-format spec is the dimension entry in FSD-002 §3.6.2; evaluation derived from linked `progress_measure:{method_id}` attestations over time, not a direct score. Multiple Approaches per Goal supported (federated A/B). Amendment / supersession / retirement via the four structural primitives (`supersedes` / `withdraws` / `recants` per FSD-002 §2.2). Slashing-decoupled from approach-disagreement (FSD-002 §6.1.6 trust composition). [Spec]

### Primitive 14: Method

Dimensional claim `method:{approach_id}:{substrate_rung}` in NodeCore's namespace slice (FSD-002 §3.6.2). Concrete operational practice instantiating an Approach — Piece 2's γM at the federation tier, with substrate-typing first-class (`substrate_rung` ∈ Ph0/Ph1/Ph2/A0..A5; cross-rung portability requires explicit attestation). Truth-grounding is **execution verifiability**: signed work logs, capture+interpret artifacts (the `FSD/SAFETY_BATTERY_CI_LOOP.md` pattern generalized), substrate resource observation. P9 Slashing applies **only** on documented spoofing of execution; honest failure flows to P11 Reconsideration. [Spec]

### Primitive 15: Progress Measure

Dimensional claim `progress_measure:{method_id}` in NodeCore's namespace slice (FSD-002 §3.6.2). Evidence of progress toward Goals under chosen Approaches via specific Methods — the federation's Goodhart-resistance discipline. Required `validity_window` with renewal policy; required Goodhart-resistance attestation. The framework-provided five-factor decomposition (C, I_int, R, I_inc, S per `CIRISLens/FSD/ciris_scoring_specification.md`) is the reference measure set inheriting v2's empirical record. Federation-defined measures extend the same Contribution surface and earn track record. P11 Reconsideration is load-bearing for measure retirement; multiplicative composition at Goal level (𝒞_CIRIS) is the upstream structural anti-Goodhart protection. [Spec]

**Cross-cutting structure.** P12–P15 compose into an **upward-only DAG** via the dimension-prefix references (`goal:` ← `approach:` ← `method:` ← `progress_measure:`), enforced by referential integrity at audit-chain ingest. Cycles are structurally impossible at the wire level (no downward references). Slashing decoupling enforced at every level: pluralism / disagreement / measure-decoupling never trigger P9 (FSD-002 §6.1.6); only documented Method-execution spoofing does. Per-level corridor-occupation reading generalizes the framework's corridor structure (Piece 3); the locality-scaled-quorum composition policy (FSD-002 §6.1.5) is the operational discipline that makes decision-scale-matching structurally enforced (NodeCore#10). The cross-cut spec previously at `FSD/DECISION_HIERARCHY.md` is **subsumed by FSD-002 v1.4** §3.6.2 + §6.1.5 + §6.1.6 — wire-format-locked at the federation surface; no NodeCore-side cross-cut FSD owed.

---

**Tier 3 — Consensus mechanics.** How signed objects turn into federation outcomes. Type-agnostic with respect to Tier 2 — the consensus pipeline processes any Contribution uniformly. Contribution (P5) is the universal envelope; Vote / Truth-Grounding / Weighted-Aggregate / Witness-Diversity are the operations on it.

### Primitive 4: Vote

A signed score on a Contribution. Vote weight = voter's Credits in cell × expertise multiplier in (domain, language). Signatures bind to (voter_id, contribution_id, score, timestamp, cell). Votes are durable in the audit chain; revocation requires explicit signed revocation. [Spec]

### Primitive 5: Contribution

A discriminated union. Variants:

- `DEFERRAL_REQUEST`. Consumer requests routing to qualified WAs.
- `DEFERRAL_RESPONSE`. Routed WA's signed response.
- `PROPOSAL`. Battery evaluation, free-form argument, new battery draft, policy proposal. The proposal-kind discriminator lives in the payload.
- `WA_CANDIDACY`. Self- or peer-nomination for Wise Authority standing.
- `EXPERTISE_ATTESTATION`. An expertise-bearer attests another contributor has expertise in a cell.
- `MODERATION_EVENT`. Accusation of rogue action (Primitive 8).

Each Contribution carries (author_id, contribution_type, subject, payload, signature, optional witness_set). [Spec]

### Primitive 6: Truth-grounding signal

Per-subject configuration that anchors weighted aggregates and Credits accrual to ground truth. Safety subjects: production hedge captures from CIRIS agents in deployment. WA candidacy: sustained substantive contribution validated against expertise consensus. Governance and policy subjects: long-term federation health metrics, reversal/affirmation by subsequent votes, external benchmark outcomes. The signal differs in fidelity across subjects; the mechanism is uniform but the grounding is not. [Spec partial]. Production hedge captures exist in CIRISAgent; the grounding loop into the ledgers is Spec.

### Primitive 7: Weighted aggregate

Per-Contribution rolling tally of votes, weighted by Primitive 4's vote weight, anchored against the truth-grounding signal (Primitive 6) where available. The aggregate is the basis for downstream decisions (proposal adoption, WA promotion, deferral consensus). The aggregate function is policy-tunable (§6.2). [Spec]

**Occurrence-cohort extension (NodeCore#16).** Multi-occurrence agents (e.g. 9 occurrences sharing one identity per CIRISAgent CLAUDE.md) emit parallel scalars on the same compliance dimension (D01 non-maleficence, D10 beneficence). The single-Contribution aggregate above answers "how does the federation read this one Contribution?"; the cohort extension answers "how does the federation read this agent template across its occurrences?" Wire shape rides the same dimension namespace as a sub-leaf: `weighted_aggregate:{contribution_id}:cohort:{agent_template_id}`. Additive — 1+4 wire-format lockdown holds. Threat surface (per NodeCore#16): cohort spoofing is mitigated structurally — per-occurrence Contributions must be signed by their own keys, so the cohort attestation can only AGGREGATE existing signed Contributions from listed occurrences; selective inclusion is mitigated by a `expected_occurrence_count` field that consumers compare against `included.len()` for a coverage check, plus P10 witness diversity on the cohort attestation itself. Implementation surface: [`crate::aggregate::cohort_weighted_aggregate`]. [Spec]

### Primitive 10: Witness-diversity requirement

For high-stakes Contributions (ModerationEvents, WA candidacy proposals, policy proposals above a magnitude threshold, ExpertiseAttestations whose acceptance would jump the target's standing past a threshold), validity requires ≥ N independent witnesses meeting a diversity bar:

- **Jurisdictional**: witnesses span ≥ 2 jurisdictions.
- **Organizational**: no two witnesses operated by the same legal entity or entities with majority common ownership.
- **Software-stack**: witnesses run distinct client implementations.
- **Cell-expertise**: for cell-specific high-stakes Contributions, witnesses must hold non-zero Expertise standing in the (domain, language) of the Contribution. The cell here is the Expertise-granularity cell (domain, language), not the Credits-granularity cell (domain, language, subject), because witnesses evaluate substantive merit which is broader than any single subject.

The witness role is not adjudication. Witnesses attest only that the Contribution warrants review and the evidence is well-formed. Adjudication is the WA quorum's task. The diversity bar prevents a single fault-domain from suppressing or manufacturing high-stakes events. N defaults to 3; the default is policy-tunable. P10 also applies as the admission discipline + execution-attestation discipline for Tier 2 dimensional claims (P12–P15); the locality-scaled-quorum composition policy at FSD-002 v1.4 §6.1.5 scales `N` with `locality:decision:{scale}` (NodeCore#10). [Spec]

---

**Tier 4 — Governance steering.** Fires when Tier 3 produces persistent failure. Coordinates cross-tier patterns (Moderation), provides universal appeal path (Reconsideration), backstops with punishment only for documented spoofing (Slashing — decoupled from disagreement at every Tier 2 level).

### Primitive 8: Moderation event

A Contribution subtype targeting another Contribution or contributor, alleging rogue action. Allegation types:

- `ROGUE_VOTE`. A vote that subsequent evidence shows was bribed, coerced, or otherwise structurally illegitimate.
- `COORDINATED_VOTING`. A cluster of votes whose pattern indicates collusion.
- `OUT_OF_DISTRIBUTION_ATTESTATION`. A vote or expertise attestation far outside the issuer's behavioral baseline.
- `EXTERNAL_INDUCEMENT_EVIDENCE`. Signed external evidence of bribery or coercion.
- `EXPERTISE_FRAUD`. An EXPERTISE_ATTESTATION made without substantive basis.

Filing requires the accuser to stake Commons Credits proportional to the alleged harm. ModerationEvents require witness aggregation (Primitive 10) and adjudication by a WA quorum in the relevant cell. [Spec]

### Primitive 9: Slashing attestation

A signed record reducing a target's Credits *or* Expertise standing in a specified cell, issued by a quorum of WAs in the relevant domain after adjudicating a ModerationEvent. Outcomes:

- **PROVEN_ROGUE**. Target's Credits reduced proportional to harm; if the rogue action involved expertise standing, Expertise reduced. Accuser's stake returned plus bounty from the slashed credits.
- **NOT_PROVEN**. Target's standing restored. Accuser's stake disposition is discretionary to the WA quorum: full return for good-faith filings; partial retention for marginal cases; full retention plus possible counter-moderation for filings the quorum specifically flags as bad-faith.

Slashing distinguishes proven rogue action (target acted against the protocol's terms) from miscalibration (target voted in good faith but their judgment differed from eventual ground truth). Miscalibration is not slashable. The Credits and Expertise ledgers both carry a non-negative invariant; slashing reduces toward but never below zero. **Slashing is decoupled from the decision hierarchy** (Tier 2): Goal-level disagreement, Approach-level disagreement, Progress-Measure decoupling, and honest Method-execution failure all flow to P11 Reconsideration rather than P9. Slashing applies only on documented Method-execution spoofing or the existing P8 allegation types. The decoupling discipline is locked at FSD-002 v1.4 §6.1.6 (`agent_files-trust-composition` reference policy + slashing-decoupling commitment applying across all Tier 2 dimensions). [Spec]

### Primitive 11: Reconsideration

A signed request to reverse a prior SlashingAttestation. Reconsideration is structurally parallel to Moderation but operates on the reverse axis: where Moderation initiates a path toward slashing (forward consequence), Reconsideration initiates a path toward restoration (reverse consequence). Without this primitive, the system has only forward consequences; a wrongful adjudication is permanent, which erodes participation by making the cost of being wrongly slashed unbounded.

Grounds for reconsideration:
- `NEW_EVIDENCE`. Evidence not available at the time of the original adjudication.
- `PROCEDURAL_ERROR`. The original adjudication failed to follow the moderation flow (witness diversity bar not actually met but accepted; quorum size below threshold; cell-expertise constraint violated).
- `QUORUM_COMPROMISE`. Post-hoc evidence that the original adjudicating quorum was bribed, coerced, or otherwise compromised.

Filing requires the requester to stake Commons Credits proportional to the alleged severity of the original adjudication error. A **fresh** WA quorum (members of the original adjudicating quorum recused) reviews the reconsideration and signs a ReconsiderationAttestation with one of three outcomes:

- **REVERSED**. Original slashing nullified; target's Credits and Expertise restored to pre-slashing values, with adjustments for any subsequent activity. Requester's stake returned. If QUORUM_COMPROMISE grounds were proven, the original quorum members face counter-moderation review.
- **PARTIAL**. Original slashing reduced but not nullified; standing restored proportional to the proven error.
- **UPHELD**. Original slashing stands. Requester's stake disposition is discretionary to the fresh quorum (same rule as NOT_PROVEN in Primitive 9).

**Recursion bound**: a target may file at most one Reconsideration per SlashingAttestation **per hash-pinned evidence package per ground** (NEW_EVIDENCE, PROCEDURAL_ERROR, QUORUM_COMPROMISE separately). An "evidence package" is a deterministically-hashable bundle (signed Contributions, attestations, external documents pinned by SHA-256) that the requester submits as the load-bearing material for the appeal; two filings with *the same* evidence package hash under the same ground collapse to one. This expands the legitimate appeal surface (a target with three distinct pieces of NEW_EVIDENCE can pursue three appeals) without weakening the harassment trip-wire: three Reconsiderations filed without success on a single SlashingAttestation — regardless of how many distinct evidence packages they cite — triggers harassment-pattern review via RATCHET integration. The bound prevents indefinite re-litigation while preserving the appeal path.

**Time bound**: Reconsideration may be filed within a policy-tunable window from the SlashingAttestation date (default proposed: 180 days). Beyond the window, only QUORUM_COMPROMISE grounds remain available (because compromise evidence may emerge years later).

**Fresh-quorum selection**: members of the original adjudicating quorum are recused. In cells with enough WAs to draw a non-overlapping quorum, this is straightforward. In narrow cells where the WA pool is small, fresh-quorum selection draws from adjacent cells whose WAs hold cell-expertise in the contribution's (domain, language) per Primitive 10; if no adjacent-cell WAs are available, federation-wide WA pool is used with explicit rationale recorded. This is a structural gap in narrow cells; see §9. [Spec]

### 2.16 RATCHET integration contract

CIRISNodeCore is not the anti-Sybil evaluator. RATCHET is, per `RATCHET/README.md` and `RATCHET/FSD.md`. The integration contract:

**Crate emits**:
- Signed audit chain in the documented canonical format.
- Specific event types: Contribution submission, Vote, ExpertiseAttestation, ModerationEvent, SlashingAttestation, ledger update.
- Each event signed and durable per substrate S1/S3.
- Stable schema versioning so RATCHET's readers don't break on chain evolution.

**RATCHET returns**:
- Per-contributor flags: out-of-distribution voting pattern, coordinated voting cluster, density anomaly, expertise attestation anomaly.
- Flags arrive as advisory inputs to the moderation flow; they do not autonomously modify ledger state.
- WA quorums consult flags when adjudicating ModerationEvents or when filing automatic ModerationEvent drafts on flagged patterns.

RATCHET's L-01 through L-08 limitations (`RATCHET/KNOWN_LIMITATIONS.md`) bind here as they bind at the substrate level. The federation accepts these as operational rather than theoretical guarantees.

**Who watches RATCHET.** The recursion ("if RATCHET monitors the federation, what monitors RATCHET?") terminates structurally, not by adding another monitor. RATCHET is the *operational* detector for coherence collapse in the federation's behavior. The *formal framework* defining what coherence collapse IS — and giving the mathematical conditions under which it is measurable — is fixed in the published preprint *Coherence Collapse Analysis* (Moore 2026; DOI [10.5281/zenodo.18217688](https://doi.org/10.5281/zenodo.18217688); CC-BY 4.0). The framework is hash-pinned, citable, immutable per its version; it cannot be captured by the federation it analyzes because it predates and is independent of any running deployment. RATCHET's correctness is checkable against the preprint, not against another running monitor. The CCA finding that *correlated constraints collapse effective diversity toward unity* (k_eff = k/(1+ρ(k-1)) → 1 as ρ → 1) is the structural reason a federation needs RATCHET in the first place; making the framework citable closes the regress.

**Seed-holder voting-alignment monitoring signal.** Externally-anchored seed Expertise per §7.2 (F-AV-BOOT) introduces a known asymmetry — seed-holders carry weight the cell has not yet attested. To make seed-bloc capture observable rather than implicit, the audit chain emits a `seed_holder_voting_alignment` event per cell per voting window: pairwise cosine of seed-holder vote vectors across all Contributions in the window. RATCHET (and eventual lens-core dashboards) read the event and surface alignment that exceeds a steward-set threshold for review. Not a slashing trigger; a transparency signal. Pre-`[Impl]` interim: emit the alignment statistic alongside the CI safety-battery artifacts so safety.ciris.ai can render it next to the canonical evidence.

### 2.17 Boundary defense

**Why Tier 2 is dimensional, not a parallel wire surface.** Goal / Approach / Method / Progress Measure are dimensional claims on the workhorse `scores` attestation, owned in NodeCore's slice of FSD-002 §3 (specifically §3.6.2). The consensus pipeline (P4/P5/P6/P7/P10) processes them type-agnostically; per-dimension semantics ride in `evidence_refs[]` + `context` per FSD-002 §2.1, not in new MessageType wire surfaces. The 1+4 wire-format shape (FSD-002 v1.4 §3.10) held under encyclical-level stress — adding new dimensions does not require new structural primitives, including for the decision-hierarchy slot. The agent-side Goal Primitive (`FSD/GOAL_PRIMITIVE.md`) is preserved as a substantive standalone spec because it's the framework-grounded type (P_G per *Corridor Dynamics* v2 Piece 4 — an A3+ agent property scored *by* attestations, not just an attestation dimension). Approach / Method / Progress Measure are pure dimensional claims; their wire-format spec is FSD-002 §3.6.2 directly.

**Why Slashing-decoupling is asserted at the decision hierarchy.** Without it, P9 becomes a weapon against legitimate pluralism: a faction dislikes another faction's Goal / Approach / Progress Measure → files for Slashing → suppresses the disagreement. The Tier 2 FSDs enforce that every level routes disagreement to P11 Reconsideration, not P9; P9 fires only on documented Method-execution spoofing or the original P8 allegation types. This is engineering policy, not framework derivation — the framework licenses pluralism (Piece 5's multi-agent consent), the specific protections are CIRISNodeCore's call.

**Why Credits and Expertise are separate primitives.** Different signals, different time scales, different roles. Credits accrue per grounded vote at (domain, language, subject) granularity. Expertise accrues through peer attestation and hard-case track record at (domain, language) granularity. A contributor can accumulate high Credits through consistent voting on easy cases without demonstrating competence; a recognized expert who participates sparsely has high Expertise standing but low Credits. Collapsing forces a single signal that can't carry both.

**Why Witness-diversity is separate from Moderation-event.** Witness diversity applies to many high-stakes Contribution types (moderation, WA candidacy, policy, jump-threshold expertise attestations, reconsideration). Moderation is one Contribution type. Collapsing would force re-specifying the diversity rule per type.

**Why Slashing is separate from Moderation.** Filing-and-adjudication versus consequence. Different invariants (stake disposition vs non-negative balance). Both auditable independently.

**Why Reconsideration is separate from Moderation.** Forward adjudication versus reverse adjudication. Different requirements (fresh-quorum recusal, narrower grounds, recursion bound, time bound) and different outcome shapes (REVERSED/PARTIAL/UPHELD vs PROVEN_ROGUE/NOT_PROVEN). Collapsing would hide the fresh-quorum requirement that prevents the original adjudicators from re-confirming their own decision.

**Why Vote is separate from Contribution.** A vote responds to a Contribution; a Contribution is the thing voted on. Different schemas, lifecycles, rate limits.

**Why Identity is substrate inheritance, not a CIRISNodeCore primitive.** The substrate produces identity; the crate consumes it.

**Things considered but rejected as primitives:**
- *Cross-domain spillover function*. Rejected. Earned per cell; correlated competence accumulates standing organically across cells through actual cell work, not via a transfer function.
- *Behavioral-baseline detector*. Rejected as a crate primitive. RATCHET fills this role at the federation level; the crate's job is the integration contract (§2.16).
- *Deferral as its own primitive*. Rejected. Deferral is a Contribution subtype (DEFERRAL_REQUEST/RESPONSE), routed and aggregated like any other Contribution.
- *WA selection as its own primitive*. Rejected. WA selection is an algorithm over Expertise and Credits state, not a separate primitive.
- *Vote weighting function as its own primitive*. Rejected. The function is a policy parameter.

### 2.18 What is NOT a primitive

The following are actor behaviors or applications over the primitives, not primitives:

- **Wise Authority adjudication**. Actor behavior. A WA quorum reads ModerationEvents and signs SlashingAttestations; the role operates over the primitives.
- **safety.ciris.ai**. Application. A deployment exposing the crate's primitives to public engagement around safety subjects.
- **CIRISBench HE-300**. Adjacent crate. Benchmark execution lives there; its outputs flow into expertise validation as one track-record input.
- **Specific scoring functions and subject definitions**. Policy.
- **The decision-hierarchy DAG as a top-level primitive**. Rejected. The DAG is the *referential-integrity property* of Tier 2 (P12–P15) dimensional claims, enforced at audit-chain ingest. It is not a separate primitive any more than "the audit chain's append-only property" is a primitive separate from Contribution. Cross-cut spec is now FSD-002 v1.4 §3.6.2 + §6.1.5 + §6.1.6, not a separate NodeCore FSD (the prior `FSD/DECISION_HIERARCHY.md` was subsumed when FSD-002 v1.3 LOCKED the wire format).
- **𝒞_CIRIS as a separate primitive**. Rejected. 𝒞_CIRIS is the score function for Goal (P12); it lives in the Goal Primitive's scoring contract, not as its own primitive. Federation-defined Progress Measures that compose with it use the Progress Measure primitive (P15).
- **Sub-federation branching as a separate primitive**. Rejected. Branching is a semantic outcome of P7 + P10 + P11 over Approach (P13) when two Approaches at the same Goal are genuinely incompatible. The branching mechanism (parent-child federation directory metadata, P10 thresholds) is engineering policy on top of existing primitives, not a new primitive.

---

## 3. Protocols (WHO)

### 3.1 Contributor identification

Contributors authenticate with their federation identity (Primitive 1). The crate reads identity from `CIRISRegistry` and maintains its own state only in the Credits and Expertise ledgers. [Spec]

### 3.2 Engagement modes

Four modes, applying across all subjects:

- **Evaluate existing**. Review a published battery, rubric, candidate, or policy and submit signed evaluation as a PROPOSAL of evaluation-kind.
- **Free-form proposal**. Submit narrative argument as PROPOSAL of free-form-kind.
- **Propose new**. Draft a new artifact (PROPOSAL of new-kind, WA_CANDIDACY).
- **Vote**. Review others' Contributions and submit signed Votes (Primitive 4).

[Spec]

### 3.3 Deferral routing

A consumer submits DEFERRAL_REQUEST specifying cell and query context. The crate selects routing targets:

1. Identify contributors with non-zero Expertise standing in (domain, language) from the Expertise ledger.
2. Filter to Active tier (§3.7).
3. Apply diversity preferences (jurisdictional, organizational) per policy.
4. Bound the routed set at a policy-tunable maximum (default 5 to 9).

Routed contributors submit DEFERRAL_RESPONSE Contributions. The crate aggregates per Primitive 7 and returns the aggregate plus individual responses to the requester. The aggregate surfaces in the federation directory as a dimensional claim `deferral:aggregate:{cell}` where `{cell}` is the `(domain, language)` pair the deferral targeted; subscribers read the dimension to consume the federation's running deferral state per cell. Audit chain records the full deferral cycle. [Spec]

### 3.4 Voting and aggregation

Votes are signed scores on Contributions. The crate computes the rolling weighted aggregate (Primitive 7): per-vote weight = Credits × expertise multiplier × Active-tier multiplier. Aggregates anchored to truth-grounding signals where available. New votes update the aggregate continuously. [Spec]

### 3.5 Witness diversity for high-stakes Contributions

High-stakes Contributions require Primitive 10 witness diversity. The set:

- ModerationEvents (always).
- WA_CANDIDACY (always).
- Policy PROPOSAL above a threshold magnitude.
- ExpertiseAttestation whose acceptance would jump the target's Expertise standing past a threshold (§3.7).

Routine Contributions (battery evaluation, vote, deferral request/response, ExpertiseAttestation below jump-threshold) do not require witnesses. [Spec]

### 3.6 Wise Authority promotion

WA candidacy is a Contribution subtype. Promotion requires both:

1. **Significant Expertise** in the (domain, language). The candidate's Expertise standing must exceed a threshold derived from existing WAs and high-Expertise contributors in the cell.
2. **Substantive participation history**. The candidate's aggregated Credits across all subjects within the (domain, language) over a rolling 180-day window must exceed a participation threshold. Aggregation across subjects (rather than per-subject) reflects that WA standing is at the (domain, language) granularity matching Expertise; a candidate's participation breadth across multiple subjects in the cell is what's measured, not depth in any one subject.

Both thresholds are policy parameters. **Policy is the gate, not adjudication.** Humans set the thresholds (rule-setting layer) and cast the votes that constitute Credits and Expertise (consensus layer); the promotion algorithm applies those policy-set rules deterministically. Symmetric with the safety-loop framing in `cirisnodecore/SCHEMA.md` §12.1: rules are crowdsourced; verdicts are machined. WA promotion is the same shape applied to standing rather than to responses — there is no per-candidate adjudication panel deciding "is this person worthy"; there is policy applied to ledger state. Demotion follows the same algorithm in reverse, with decay on inactivity. [Spec]

### 3.7 Expertise attestation flow

A contributor with non-zero Expertise in a cell submits EXPERTISE_ATTESTATION attesting another contributor has expertise in the cell. The attestation is signed. If the attestation would jump the target's Expertise standing past a threshold magnitude, witness diversity per §3.5 is required; otherwise it is not.

Expertise ledger is updated by:
- New EXPERTISE_ATTESTATION Contributions, weighted by attester's own Expertise standing.
- Track record validation against truth-grounding on substantively challenging cases (hard cases take time to resolve; see hard-case definition below).
- Decay on inactivity in the cell.

**Hard-case definition.** A Contribution counts as a hard case for track-record purposes if any of the following signals fires, all computed mechanically from the audit chain:

- **Vote variance signal**: the variance of weighted votes on the Contribution exceeded a threshold at the moment truth-grounding resolved. High variance indicates substantive disagreement among voters who participated.
- **Resolution-time signal**: truth-grounding took longer than the P75 of the cell's resolution-time distribution. Slow-resolving cases tend to involve contested or delayed ground truth.
- **Moderation signal**: a substantive ModerationEvent was filed against any vote on the Contribution and resolved (regardless of outcome). The filing itself signals genuine substantive disagreement worth adjudicating.

A Contribution is a hard case if any one signal fires. Composing the three lets the mechanism identify hard cases without requiring post-hoc human labeling, avoiding the recursive risk where existing experts decide what counts as "hard" and thereby self-confirm the consensus. Signal thresholds are policy parameters (§9).

Bootstrap seed Expertise from CIRIS L3C is signed by the steward, durable in the audit chain, and decays in relative weight as the cell matures. [Spec]

### 3.8 Activity tier

Per `FEDERATION_THREAT_MODEL.md` §6.6 F-AV-DORMANT, sparse activity is a Sybil vector. The crate applies a binary tier classification:

| Tier | Substantive Contributions per 30-day window | Vote weight |
|------|---------------------------------------------|-------------|
| Active | ≥ N (policy parameter) | Full multiplier |
| Below-Active | < N | Baseline (no multiplier) |

A "substantive Contribution" excludes votes and excludes RATCHET-flagged synthetic patterns. Dormancy at federation timescales (years of zero activity) is RATCHET's concern; the crate's binary tier handles the pilot timescale. [Spec]

### 3.9 Reconsideration

A target of a SlashingAttestation (or any contributor with standing in the affected cell) may file a RECONSIDERATION_REQUEST per Primitive 11. The request specifies the target SlashingAttestation, grounds (NEW_EVIDENCE / PROCEDURAL_ERROR / QUORUM_COMPROMISE), evidence, and requester stake.

Witness diversity per §3.5 is required because reconsideration is high-stakes by construction (reversing standing changes affects the cell's expertise consensus and the integrity of the moderation system).

The crate selects a fresh WA quorum: members of the original adjudicating quorum recused; fresh members drawn from the same cell's WA pool where possible, from adjacent-cell WAs with cell-expertise per Primitive 10 where the same-cell pool is exhausted, from federation-wide WA pool with explicit rationale otherwise.

The fresh quorum reviews evidence and signs a ReconsiderationAttestation with one of three outcomes. The recursion bound (one Reconsideration per ground per SlashingAttestation; three Reconsiderations on a single SlashingAttestation triggers harassment review) prevents indefinite re-litigation. The time bound (default 180 days from SlashingAttestation) keeps the appeal window finite for most grounds; QUORUM_COMPROMISE grounds remain available beyond the window because compromise evidence may emerge years later. [Spec]

### 3.10 Node-mode `agent_files` serving (c/r/n role discipline)

The c/r/n taxonomy (§1.3) gives federation deployments three operational shapes. Per FSD-002 v1.4 §3.6.7 + §6.1.6 (joint `agent_files:*` namespace + trust-composition policy), **only `node` mode serves bytes**:

- **client** — sends own Contributions; fetches blobs via `PeerResolver::resolve_holders` + `MessageType::ContentFetch`; **does not serve**, **does not advertise** `holds_bytes:sha256:*`.
- **proxy / relay** — client behavior + transit cache (best-effort, LRU, private and ephemeral; **does not advertise** `holds_bytes:*` either — relay caches are not a federation-discoverable surface).
- **node** — relay behavior + **serves** the declared `agent_files:*` slice. Registers a [`Handler<ContentFetch>`](https://docs.rs/ciris-edge/) via [`crate::serving::install_node_mode_serving`]; on `BlobStorage::put_blob` persist auto-emits `holds_bytes:sha256:{prefix}` attestations into the federation directory (per CIRISPersist#103), making the holder discoverable to peers via the standard PeerResolver path.

**Discovery → fetch flow** (federation-internal):

1. Peer A sees an attestation citing `evidence_refs:sha256:X`.
2. A consults `PeerResolver::resolve_holders(X)` → set of node-mode peers holding the bytes.
3. A sends `ContentFetch{sha256: X}` to peer B (node mode).
4. B's installed handler queries `BlobStorage::get_blob(X)` and replies with `ContentBody{sha256: X, bytes, attestation_ref}` or `ContentMiss{sha256: X, reason}` per `crate::serving::compute_serving_response`.
5. A verifies `sha256(bytes) == X` on receipt; trust rides the original `agent_files:*` attestation that named the SHA, not the bytes themselves.

**Trust composition** (per FSD-002 v1.4 §6.1.6):
- **Canonical**: attestation from a registry-steward-triple key with `score ≥ 0.7` constitutes CIRIS canonical default trust.
- **Open**: any federation-key holder may emit `agent_files:*` attestations; consumer policy decides.
- **Vote-then-trust**: NodeCore P4 votes accumulate on third-party agent_files; federation-policy threshold elevates trust class.

**Storage discipline** is documented in [`FSD/CONTRIBUTION_LIFECYCLE.md` §12](FSD/CONTRIBUTION_LIFECYCLE.md) — per-role caps (client 256 MB soft / 1 GB hard; relay 10 GB / 100 GB; node 1 TB / unbounded), replication scope by data class, and the trust-radius framing.

**HTTP-compat surface** is optional per FSD-002 v1.4 §3.6.7 — node-mode peers MAY expose `GET /v1/files/{sha256}` over their HTTP listener for newcomer install paths. Federation-internal fetch always uses `ContentFetch` directly; HTTP is the newcomer-friendly surface. v0.1 ships the federation-internal path; HTTP-compat is operator-config follow-up. [Spec]

---

## 4. Schemas (WHAT)

Schemas live in the `ciris-node-core` crate. Surfaced to the API and serialized canonically for audit-chain persistence.

### 4.1 Contributor

```rust
struct Contributor {
    contributor_id: Ed25519PublicKey,
    self_attestation: SelfAttestation,
    created_at: Timestamp,
    bond_status: BondStatus,
    activity_tier: ActivityTier,                // Active | BelowActive
    tier_recomputed_at: Timestamp,
}
```

### 4.2 Contribution

```rust
struct Contribution {
    contribution_id: Ulid,
    author_id: ContributorId,
    contribution_type: ContributionType,
    subject: Subject,                           // (domain, language, subject_kind)
    payload: Vec<u8>,                           // canonical-encoded per subject_kind
    witness_set: Option<WitnessSet>,            // required for high-stakes per §3.5
    signature: HybridSignature,                 // Ed25519 + ML-DSA-65
    submitted_at: Timestamp,
}

enum ContributionType {
    DeferralRequest,
    DeferralResponse,
    Proposal,
    WaCandidacy,
    ExpertiseAttestation,
    ModerationEvent,
    ReconsiderationRequest,
}
```

### 4.3 Vote

```rust
struct Vote {
    vote_id: Ulid,
    voter_id: ContributorId,
    contribution_id: ContributionId,
    cell: Cell,
    score: ScoreType,                           // subject-dependent
    rationale: Option<Vec<u8>>,
    signature: HybridSignature,
    cast_at: Timestamp,
}
```

### 4.4 CommonsCreditsLedger

```rust
struct CommonsCreditsLedger {
    contributor_id: ContributorId,
    cell: Cell,                                 // (domain, language, subject)
    credits: NonNegativeDecimal,                // invariant: ≥ 0
    last_updated: Timestamp,
    ledger_signature: HybridSignature,
}
```

### 4.5 ExpertiseLedger

```rust
struct ExpertiseLedger {
    contributor_id: ContributorId,
    cell: (Domain, Language),                   // expertise indexes broader than credits
    attestations: Vec<ExpertiseAttestationRef>,
    track_record: TrackRecordMetrics,
    standing: NonNegativeDecimal,               // derived state, invariant ≥ 0
    last_recomputed: Timestamp,
    ledger_signature: HybridSignature,
}

struct TrackRecordMetrics {
    hard_case_count: u32,
    truth_grounded_alignment_rate: f64,
    contested_cases_resolved: u32,
}
```

### 4.6 ExpertiseAttestation

```rust
struct ExpertiseAttestation {
    contribution_id: Ulid,
    attester_id: ContributorId,                 // must have nonzero Expertise in cell
    target_id: ContributorId,
    cell: (Domain, Language),
    rationale: Vec<u8>,
    witness_set: Option<WitnessSet>,            // required when standing jump exceeds threshold
    signature: HybridSignature,
    attested_at: Timestamp,
}
```

### 4.7 ModerationEvent

```rust
struct ModerationEvent {
    contribution_id: Ulid,
    accuser_id: ContributorId,
    target_kind: TargetKind,                    // Contribution | Voter | Attester
    target_id: Ulid,
    allegation: AllegationType,
    evidence: Vec<u8>,
    accuser_stake: CommonsCredits,
    witness_set: WitnessSet,                    // mandatory
    signature: HybridSignature,
    filed_at: Timestamp,
}

enum AllegationType {
    RogueVote,
    CoordinatedVoting,
    OutOfDistributionAttestation,
    ExternalInducementEvidence,
    ExpertiseFraud,
}
```

### 4.8 SlashingAttestation

```rust
struct SlashingAttestation {
    attestation_id: Ulid,
    moderation_event_id: ContributionId,
    quorum_ids: Vec<WiseAuthorityId>,
    outcome: SlashingOutcome,                   // ProvenRogue | NotProven
    credits_reduced: CommonsCredits,            // 0 if NotProven
    expertise_reduced: ExpertiseStanding,       // 0 unless rogue action involved expertise
    accuser_stake_disposition: StakeDisposition,
    signatures: Vec<HybridSignature>,
    attested_at: Timestamp,
}
```

### 4.9 WitnessSet

```rust
struct WitnessSet {
    witnesses: Vec<Witness>,
    diversity_proof: DiversityProof,
}

struct Witness {
    witness_id: ContributorId,
    jurisdiction: Iso3166Code,
    operator: OrganizationId,
    software_stack: StackIdentifier,
    cell_expertise: ExpertiseStanding,          // required for cell-specific high-stakes per §3.5
    signature: HybridSignature,
}
```

### 4.10 ReconsiderationRequest and ReconsiderationAttestation

```rust
struct ReconsiderationRequest {
    contribution_id: Ulid,                      // a request is itself a Contribution
    requester_id: ContributorId,
    target_slashing_id: AttestationId,
    grounds: ReconsiderationGrounds,
    evidence: Vec<u8>,
    requester_stake: CommonsCredits,
    witness_set: WitnessSet,                    // mandatory
    signature: HybridSignature,
    filed_at: Timestamp,
}

enum ReconsiderationGrounds {
    NewEvidence,
    ProceduralError,
    QuorumCompromise,
}

struct ReconsiderationAttestation {
    attestation_id: Ulid,
    reconsideration_request_id: ContributionId,
    fresh_quorum_ids: Vec<WiseAuthorityId>,     // original adjudicators recused
    outcome: ReconsiderationOutcome,            // Reversed | Partial | Upheld
    credits_restored: CommonsCredits,           // 0 if Upheld
    expertise_restored: ExpertiseStanding,      // 0 if Upheld or if original slash didn't touch expertise
    requester_stake_disposition: StakeDisposition,
    fresh_quorum_rationale: Vec<u8>,            // required when fresh quorum drew from adjacent or federation-wide pool
    signatures: Vec<HybridSignature>,
    attested_at: Timestamp,
}
```

---

## 5. Logic (HOW)

### 5.1 Deferral routing flow

1. Consumer submits DEFERRAL_REQUEST specifying cell and query context.
2. Crate queries Expertise ledger for contributors with non-zero standing in (domain, language).
3. Filter to Active tier.
4. Apply diversity preferences; route to policy-tunable count (default 5 to 9).
5. Routed contributors receive notification (out-of-band channel; not crate scope).
6. Routed contributors submit DEFERRAL_RESPONSE Contributions.
7. Crate aggregates per Primitive 7; returns aggregate plus individual responses.
8. Audit chain records request, routing decision, responses, aggregate.

### 5.2 Contribution submission

1. Contributor authenticates with federation identity.
2. Constructs Contribution targeting a specific cell.
3. For high-stakes Contributions, gathers witness attestations per §3.5.
4. Signs Contribution; crate validates schema and signature.
5. Crate appends to audit chain; chain replicated per substrate.

### 5.3 Weighted aggregation

1. Each new Vote recorded in audit chain.
2. Crate computes rolling weighted aggregate per Contribution:
   - per-vote weight = Credits(voter, cell) × expertise_multiplier(voter, domain, language) × active_tier_multiplier(voter)
   - aggregate = anchor(weighted_mean(votes), truth_grounding_signal(subject))
3. Aggregate updates continuously.

### 5.4 Truth-grounding loop

1. Safety subjects: two independent grounding sources feed the loop:
   (a) production hedge captures from CIRIS agents-under-test (each capture references the Contribution scenario);
   (b) **independent foundation-model judge verdicts** — a non-CIRIS-lineage model (default Claude Opus 4.7) applies the cell's `criteria.json` to captured responses and emits signed PASS / FAIL / UNDETERMINED verdicts with cited spans per `cirisnodecore/FSD/JUDGE_MODEL.md`. The judge is outside the system under test by construction, which closes the agent-grading-itself trap that a same-lineage interpreter would re-open. The two signals are reconciled at aggregation time: (a) tells you what the agent actually did in production, (b) tells you whether what the agent did matches what the rubric asked for.
2. Crate updates Contribution aggregate and voters' Credits based on alignment between vote and outcome.
3. Voters aligned with outcomes accrue Credits; misaligned voters do not lose Credits (miscalibration is not slashable).
4. WA subjects: sustained substantive contribution serves as grounding.
5. Governance subjects: long-term federation health metrics provide post-hoc anchoring at slower cadence.

### 5.5 Expertise attestation flow

1. Attester (must have nonzero Expertise in cell) submits EXPERTISE_ATTESTATION.
2. If the attestation would jump the target's standing past a threshold, witness set per §3.5 required.
3. Crate records attestation in target's Expertise ledger.
4. Standing recomputed: weighted sum of attestations (by attester's standing) plus track record metrics.
5. Continuous decay on inactivity.
6. Bootstrap seed attestations from CIRIS L3C decay in relative weight as cell matures.

### 5.6 Moderation flow

1. Accuser files ModerationEvent with witness set per §3.5.
2. Crate validates witness diversity (including cell-expertise for cell-specific allegations). Bar not met: event rejected at schema level.
3. Event appears in WA quorum's adjudication queue. Accuser's stake held in escrow.
4. WA quorum reviews evidence and signs SlashingAttestation with one of two outcomes.
5. Crate executes slashing or restoration per attestation. Audit chain records full adjudication.
6. Subsequent events targeting same target within cooldown window flagged for harassment-pattern review (RATCHET integration).

### 5.7 Reconsideration flow

1. Requester files RECONSIDERATION_REQUEST targeting a prior SlashingAttestation with grounds, evidence, witness set, and stake.
2. Crate validates witness diversity (per §3.5) and time bound (§3.9). Bar not met or out of time window for non-QUORUM_COMPROMISE grounds: request rejected at schema level.
3. Crate checks recursion bound: prior Reconsideration on same SlashingAttestation with same ground? Reject. Third Reconsideration on same SlashingAttestation? Flag for harassment review and reject.
4. Crate selects fresh WA quorum: original adjudicators recused; same-cell WAs preferred; adjacent-cell WAs with cell-expertise where same-cell pool exhausted; federation-wide pool with rationale otherwise.
5. Fresh quorum reviews evidence and signs ReconsiderationAttestation with one of three outcomes.
6. Crate executes restoration per attestation. Audit chain records full reconsideration cycle.
7. If grounds were QUORUM_COMPROMISE and outcome was REVERSED, original quorum members face automatic ModerationEvent draft for harassment review.

---

## 6. Constant alignment

### 6.1 Review heuristic

"Would a careful Wise Authority looking only at the audit chain conclude this Contribution helps M-1 or harms it?" The heuristic applies symmetrically to contributors, WAs adjudicating moderation, and the steward configuring policy. The audit chain is the durable basis.

### 6.2 Policy-tunable posture

Per `FEDERATION_THREAT_MODEL.md` §2.6, anti-Sybil resistance is not an emergent invariant of substrate composition; it is a continuously-tuned policy posture. The same is true of CIRISNodeCore. The fifteen primitives do not by themselves guarantee resistance. Resistance is the product of primitives plus continuously-tuned policy parameters:

- Witness diversity thresholds and cell-expertise constraints (Primitive 10).
- Active-tier boundary N (§3.8).
- Slashing rates per outcome (Primitive 9).
- Truth-grounding multiplier per subject (§5.4).
- WA promotion thresholds (Credits, Expertise) and decay rates (§3.6).
- Bootstrap seed weight decay rates per cell (§3.7).
- Expertise attestation jump-threshold for witness requirement (§3.7).
- Deferral routing diversity preferences (§3.3).
- Reconsideration time bound and recursion bound (§3.9, Primitive 11).
- Fresh-quorum selection rules for narrow-cell reconsideration (§3.9).

Four empirical bets:

- **Bet A**: some cost floor exists somewhere in the consensus stack. Witness recruitment cost, sustained-contribution cost, expertise-attestation-attester recruitment cost provides a non-trivial floor relative to attacker budget.

- **Bet B-safety**: for safety subjects, the truth-grounding loop closes at weekly cadence. Production hedge captures + independent foundation-model judge verdicts (§5.4 (b)) surface miscalibration and rogue patterns within one CI cycle of the responses being produced. Tight grounding; high-confidence empirical claim.

- **Bet B-governance**: for governance subjects, the truth-grounding loop closes at quarterly-or-slower cadence. Federation health metrics, reversal/affirmation by subsequent votes, external benchmark outcomes anchor post-hoc. Loose grounding; lower-confidence empirical claim — the bet here is "slower than safety but still faster than adversary adaptation in the policy layer."

- **Bet C**: policy adjustment outpaces adversarial adaptation. The steward (post-G2: the council) detects and responds at a cadence faster than attack patterns evolve.

- **Bet D**: foundation-model judge reproducibility is operational, not cryptographic. Same `(judge_model, judge_prompt_sha256, criterion, response)` → same verdict at acceptable variance, without per-verdict cryptographic signing. Bundle-level Sigstore attestation (per `FSD/SAFETY_BATTERY_CI_LOOP.md` §3.2) binds the verdicts to a workflow run + commit; per-verdict reproducibility is checkable by anyone with the same inputs. The bet: at T=0 (or each model's effective-deterministic equivalent) on Anthropic's Messages API, verdict drift across runs falls below the threshold at which it would be confused with genuine cell-behavior change. Tracked in `FSD/JUDGE_MODEL.md` §7.

The bets are operational hypotheses, monitored continuously, tracked in §9.

### 6.3 Liability isolation

The crate does not provide medical, legal, or other professional advice. Deployments that consume the crate operate under their own terms. Contributors with professional standing contribute under their own professional terms. CIRIS L3C as steward is liable for crate specification and integrity per L3C mission-lock.

---

## 7. Federation context

### 7.1 Position

CIRISNodeCore sits in the second tier of CIRIS architecture, alongside `ciris-lens-core` and above the substrate tier. Applications consume the crate. RATCHET reads its audit chain. HE-300 lives in `CIRISBench`.

### 7.2 Bootstrap

**Contributor onboarding** (F-AV-ONBOARD): new contributors operate with Credits capped at baseline for the first n_min substantive Contributions per cell (n_min is a policy parameter; see §9). Contributors seeking Credits accumulation above baseline require multi-attester onboarding: ≥ 3 distinct attesters with above-baseline Credits in the cell. Bond-weight grace: any federation bond buys the option to accrue weight; actual accrual requires probationary period and multi-attester onboarding to be satisfied.

**Expertise bootstrap** (F-AV-BOOT external anchoring): CIRIS L3C provides externally-anchored seed expertise-bearers per cell, drawn from contributors with externally-verifiable substantive standing. The seed is small per cell (1 to 5), externally signed, durable in the audit chain. Seed weight decays at a policy-tunable rate as the cell matures. New expertise emerges from consensus of existing experts plus validated track record.

### 7.3 Pilot deployment: safety.ciris.ai

safety.ciris.ai is the pilot deployment of CIRISNodeCore. The pilot exposes the crate's primitives to public engagement around safety subjects: the 14 mental-health batteries currently on `ciris.ai/safety`, the 22 prohibited capability categories from `ciris_engine/logic/buses/prohibitions.py`. Scope:

- Engagement modes: evaluate existing, free-form proposal, new battery proposal, vote.
- The full fifteen-primitive consensus mechanism with subjects restricted to safety batteries and a small governance subject set for pilot operation itself.
- Public hall of contributors (no pseudonymous track in initial pilot).
- Bootstrap seed Expertise from CIRIS L3C's safety contributor set.
- Liability isolation per §6.3.

**Pilot phases** (the *Deployed (pilot)* lifecycle stage of §1.4 unpacks into):

- **P0 — Safety + governance only.** Safety batteries and a small governance subject set for pilot operation itself. No deferral routing from production agents. Public hall of contributors. Bootstrap at `bootstrap_threshold = 1` permitted; raise to `2` is the first P0 milestone (`FSD/FEDERATION_ANNOUNCEMENT.md` §4.2).
- **P1 — + production deferral routing.** Deferral routing from production CIRIS agents to the pilot's expertise pool, once moderation and expertise consensus carry P0 load. Bootstrap at `bootstrap_threshold ≥ 3` is the P1 entry condition — single-party AccordCarrier capability gone before production agents are routed against the pilot.
- **P2 — Fold-into-CIRISAgent.** *Deployed (folded)* per §1.4. Pilot fold-into-CIRISAgent criteria are set at pilot launch in a separate document, not specified here (per §9 open question 16).

[Spec]

---

## 8. Maintenance

### 8.1 MISSION.md integrity

Per `FEDERATION_THREAT_MODEL.md` §6.7 F-AV-MAINT, the threat model itself is an attack surface; the same applies to this MISSION.md. Response:

- Each published version signed by the steward with content `{doc_hash: sha256, version, signed_at}`.
- Signature published as S2 policy row and transparency-log entry per `FEDERATION_THREAT_MODEL.md` §8.1.
- Federation release tarballs include the document hash.
- Edits require pull-request review by ≥ 1 second reviewer.
- Annual external adversarial review aligned with threat model cadence.

[Spec]

### 8.2 Update cadence

Quarterly minor updates; annual major version cuts preceded by external adversarial review. Cross-references verified at each major version. Schema additions are minor-version updates with backward-compatible evolution.

While CIRISNodeCore is in the extraction phase, this document and the crate's API documentation are kept in lockstep. The crate's release tarball includes this MISSION.md at the documented hash. Once folded into CIRISAgent, the document migrates into CIRISAgent's documentation tree.

---

## 9. Open questions

1. **n_min for contributor onboarding** (§7.2). Probationary period count. Initial value set at pilot launch.

2. **WA promotion thresholds** (§3.6). Both the Credits threshold and the Expertise threshold are TBD; calibration requires pilot evidence.

3. **WA decay function** (§3.6). Shape of Credits and Expertise decay for inactive WAs.

4. **Bootstrap seed Expertise decay rate** (§3.7, §7.2). Per-cell rate at which initial seed Expertise diminishes as the cell matures.

5. **Truth-grounding multiplier per subject** (§5.4). Weight given to grounding-anchored aggregates vs raw weighted aggregates.

6. **Active-tier threshold N** (§3.8). Substantive Contributions per 30-day window.

7. **Slashing rates** (Primitive 9). Proportional reduction in Credits and Expertise per proven-rogue outcome severity.

8. **Bridge to ciris-lens-core** (§7.1). The two second-tier crates compose; ciris-lens-core ingests audit chain into the Compendium; ciris-node-core writes the chain. The composition contract (what ciris-node-core guarantees the chain looks like for lens-core's consumption) is TBD.

9. **Deferral routing diversity preferences** (§3.3). How aggressively to enforce jurisdictional and organizational diversity in routing.

10. **Expertise attestation jump-threshold for witness requirement** (§3.7). The standing-jump magnitude that triggers witness diversity.

11. **Hard-case signal thresholds** (§3.7). Vote-variance threshold, P75 resolution-time calibration per cell, ModerationEvent substance-filing criteria. These three signals compose for hard-case identification; each threshold is policy-tunable.

12. **Reconsideration time bound** (§3.9, Primitive 11). Default 180 days from SlashingAttestation date for NEW_EVIDENCE and PROCEDURAL_ERROR grounds; calibration pending pilot evidence. QUORUM_COMPROMISE grounds remain available beyond the window.

13. **Fresh-quorum selection in narrow cells** (§3.9). Composition rules when the same-cell WA pool is too small to draw a non-overlapping quorum: how aggressively to recruit from adjacent cells, what cell-expertise threshold applies, when federation-wide drawing is justified. Structural gap in narrow cells.

14. **Reconsideration recursion and stake-disposition calibration** (Primitive 11). Stake magnitude proportional to alleged severity; counter-moderation thresholds for QUORUM_COMPROMISE-proven reversals.

15. **First-ModerationEvent stake floor for new contributors** (Primitive 8, §2.8). Without a stake *floor* (minimum), a new contributor with a low Credits balance can file ModerationEvents at near-zero cost — degenerates into an F-AV-ONBOARD-adjacent Sybil-fast-track via moderation gaming. The floor is a policy parameter: minimum stake for a contributor's first N ModerationEvents (default proposed: N=3) regardless of the contributor's actual Credits balance. Calibration pending pilot evidence on whether the floor's friction suppresses legitimate first-moderation appeals.

16. **Pilot-to-fold criterion** (§7.3). Marked TBD per steward direction. Too many criteria to define ahead of pilot evidence; the criterion is "ready when the pilot demonstrates the primitives carry production load," with operator + steward judgment on which thresholds matter. Not a planning oversight — a lifecycle-stage gap correctly left open.

17. **Bet D silent-failure detection** (§6.2, `FSD/JUDGE_MODEL.md` §7). Unlike Bets A / B-safety / B-governance / C, whose failure modes surface as observable patterns (cost-floor breach, miscalibration on production captures, governance reversal cadence, policy lag against attack patterns), Bet D's failure mode is *silent*: if Anthropic's effective-determinism for the judge-model verdicts shifts (model retirement, sampling parameter change, infrastructure-side non-determinism), the federation continues consuming verdicts that drift from what they would have been at T=0 baseline, and downstream Credits accrual + cell-behavior change detection both degrade without alarm. Mitigation candidates: (a) per-cycle judge-replay against a fixed reference set with drift-alarm threshold, (b) dual-judge cross-check at sampled cadence, (c) per-cycle judge-model build hash + sampling-parameter attestation pinned to the bundle. Calibration and selection deferred to pilot evidence; the question is which signal is cheapest to run continuously without burning the cost-floor that Bet A relies on.

---

*This document is iterative. v1.0 is the publishable version; v1.1 patches absorbed (2026-05-11) refine §1.6 mapping, §3.6 policy-not-adjudication framing, §5.4 independent grounding signal, §6.2 four-bet split, Primitive 11 evidence-package recursion, §2.16 CCA + seed-holder monitoring. v1.2 patches absorbed (2026-05-22) land the four-tier reorganization of §2: Tier 1 (agent state — P1/P2/P3) / Tier 2 (decision objects — P12 Goal / P13 Approach / P14 Method / P15 Progress Measure) / Tier 3 (consensus mechanics — P4/P5/P6/P7/P10) / Tier 4 (governance steering — P8/P9/P11). Existing primitive numbers (P1–P11) are unchanged; the new four are appended at P12–P15. v1.3 patches absorbed (2026-05-27) reframe Tier 2 against the FSD-002 v1.4 wire-format lockdown: P12–P15 are **dimensional claims** in NodeCore's owned namespace slice (FSD-002 §3.6.2), not separate primitives requiring full standalone FSDs. The `FSD/APPROACH_PRIMITIVE.md`, `FSD/METHOD_PRIMITIVE.md`, `FSD/PROGRESS_MEASURE_PRIMITIVE.md`, and `FSD/DECISION_HIERARCHY.md` files are subsumed by FSD-002 v1.4 §3.6.2 + §6.1.5 + §6.1.6 and dropped from this repo; the wire-format spec for those three dimensions lives at FSD-002 directly. `FSD/GOAL_PRIMITIVE.md` is preserved as the substantive standalone spec for P12 (the framework-grounded P_G agent property scored *by* attestations, distinct from the other three pure dimensional claims). Per-role storage discipline added to §1.3 (c/r/n taxonomy storage caps + cross-ref to `FSD/CONTRIBUTION_LIFECYCLE.md` §12, which lands trust-radius framing, per-role caps with rationale, and replication-scope-by-data-class). §2.17 boundary-defense entry rewritten to reflect dimensional vs Path-B framing; §2.18 DAG rejection updated. Primitive numbers (P1–P15) remain stable. Future versions report operational evidence from the safety.ciris.ai pilot, parameter calibrations, adversarial-review findings, and the eventual fold into CIRISAgent. Readers can challenge any claim by tracing it to its primitive in §2, pushing back on the empirical bets in §6.2, or filing a finding against the schemas in §4 or the logic flows in §5.*
