# CIRIS Prompt Hierarchy

**Audience**: anyone — reviewer, contributor, mental-health professional, journalist, partner org — who wants to understand *which prompts influence a response, in what order*.

**Status**: companion to [`PROGRAMMATIC_ACCESS.md`](PROGRAMMATIC_ACCESS.md). The access doc tells you *where to find* the artifacts; this doc tells you *what each one does* and *how they compose*.

**Why this exists**: rules-crowdsourced verdicts-machined only works if reviewers can read every prompt and understand which prompt influenced which decision. "It's just the LLM" is not an answer. The agent's behavior is the *composition* of seven layered surfaces, each of them inspectable, hash-pinned, and proposable-against via Contributions.

---

## TL;DR — the surfaces and the order they fire

```
                       Order of influence during a single response
                       ─────────────────────────────────────────────
  user message ──►  ① language resolution    (CIRIS_PREFERRED_LANGUAGE → user → channel → en)
                           │
                           ▼
                   ② template selection      (Ally / Datum / Scout / Echo / Sage)
                           │     ──► [TEMPLATE.yaml: persona, boundary_domains, channels]
                           ▼
                   ③ context assembly        (System snapshot + channel state + task)
                           │
                           ▼
            ┌──── ④ DMA chain (parallel + bounce + sequential) ─────┐
            │                                                        │
            │   ┌─ Phase 1: 3 DMAs in parallel ─┐                    │
            │   │  PDMA      CSDMA      DSDMA   │  (asyncio.gather)  │
            │   │ (Ethics)  (Common)  (Domain)  │                    │
            │   └────────────┬──────────────────┘                    │
            │                ▼                                       │
            │   Phase 2: optional bounce gate                        │
            │     If any DMA self-scored < 0.5, re-run failing       │
            │     DMA(s) with BOUNCE_PARALLELISM=3, take best.       │
            │                ▼                                       │
            │   Phase 3: IDMA sequentially (informational only)      │
            │     Identity check — receives Phase 1+2 results.       │
            │     Failures do NOT gate; consciences read fragility.  │
            │                ▼                                       │
            │   Phase 4: ASPDMA sequentially (action selection)      │
            │     Picks SPEAK / DEFER / PONDER / TOOL / etc.         │
            │                                                        │
            │   Every DMA call composes:                             │
            │   ┌──────────────────────────────────────────────────┐ │
            │   │ Localized YAML scaffold (per-locale variant)     │ │
            │   │  └── {{POLYGLOT_<NAME>}} placeholder substitution│ │
            │   │      → polyglot reasoning content                │ │
            │   │ Localized ACCORD (or polyglot fallback)          │ │
            │   │ Comprehensive Guide (per-locale)                 │ │
            │   │ language_guidance primer (per-locale string key) │ │
            │   │ Polyglot books / pdma_framing (when referenced)  │ │
            │   └──────────────────────────────────────────────────┘ │
            └────────────────────────────────────────────────────────┘
                           │
                           ▼
            ┌──── ⑤ Conscience layer (4 parallel LLM calls) ────────┐
            │                                                        │
            │   Entropy   Coherence   Epistemic_Humility   Opt_Veto  │
            │  (locale)   (locale)    (locale)             (POLYGLOT)│
            │                                                        │
            │   Each conscience has its own prompt template.         │
            │   optimization_veto is the only pure-polyglot one —    │
            │   no per-locale mirrors allowed (test-enforced).       │
            └────────────────────────────────────────────────────────┘
                           │
                ┌──── concern? ── yes ──► forced PONDER → re-run ASPDMA only
                │                          (Phase 4 loops; Phase 1-3 don't)
                │                          [conscience.* strings narrate concern]
                │                          Up to thought-depth limit, then DEFER.
                no
                ▼
                   ⑥ Handler executes      (SPEAK, MEMORIZE, RECALL, FORGET,
                           │                TOOL, OBSERVE, DEFER, REJECT,
                           │                PONDER, TASK_COMPLETE)
                           ▼
                   ⑦ Localized strings layer  (e.g. agent.defer_check_panel for DEFER)
                           │
                           ▼
                    response in user's language
```

Every layer is inspectable. Every layer is calibratable via Contributions per
[`FSD/RUBRIC_CROWDSOURCING.md`](FSD/RUBRIC_CROWDSOURCING.md) (rubrics use the
same Contribution mechanism — `prompt_edit`, `accord_edit`, `guide_edit`,
`language_guidance_edit`, `template_edit` are all the same flow).

---

## The seven surfaces, in detail

### Surface 1: ACCORD — the ethical contract

**What it is**: the agent's foundational ethical contract. Every DMA reads it
on every invocation. It is the densest single source of "what is this agent
allowed to do, and why."

**Two flavors**:

| Flavor | Files | When loaded |
|---|---|---|
| **Polyglot** (one for all locales) | [`ciris_engine/data/accord_1.2b.txt`](https://github.com/CIRISAI/CIRISAgent/blob/main/ciris_engine/data/accord_1.2b.txt) + compressed variants `accord_1.2b_POLYGLOT_compressed_v{1,2}.txt` + `ciris_engine/data/localized/polyglot/polyglot_accord.txt` | Base; loaded universally when a per-locale variant is missing |
| **Per-locale** (29 variants) | [`ciris_engine/data/localized/accord_1.2b_{lang}.txt`](https://github.com/CIRISAI/CIRISAgent/tree/main/ciris_engine/data/localized) | Loaded when `CIRIS_PREFERRED_LANGUAGE={lang}` is set and the per-locale file exists |

**Loading order**: per-locale wins over polyglot. If a locale is missing, polyglot is the fallback. **⚠ Per-locale files silently shadow polyglot uplifts** — when the polyglot ACCORD gets a content patch, every per-locale variant must be re-translated, otherwise the per-locale version is stale.

**Calibration surface**: `accord_edit` Contribution. Edit text → propose → vote.

---

### Surface 2: Comprehensive Guide — operating doctrine

**What it is**: longer-form guide on how the agent should approach
hard cases, how to read its own ACCORD, how to interact with users in distress, etc.

**Files** (30 total — 29 locales + base):
- [`ciris_engine/data/localized/CIRIS_COMPREHENSIVE_GUIDE_{lang}.txt`](https://github.com/CIRISAI/CIRISAgent/tree/main/ciris_engine/data/localized)

**Loaded by**: DMA prompt templates that reference it (typically PDMA + ASPDMA).

**Calibration surface**: `guide_edit` Contribution.

---

### Surface 3: Polyglot reference books (the "seven books")

**What it is**: a polyglot reference corpus encoding the agent's foundational
operating principles across language boundaries. The DMAs do NOT all load these
on every call — they're pulled in when the DMA's prompt template references the
relevant book.

**Files** (in [`ciris_engine/data/localized/polyglot/`](https://github.com/CIRISAI/CIRISAgent/tree/main/ciris_engine/data/localized/polyglot)):

| Book | File | Topic |
|---|---|---|
| 0 | `book_0_quiet_threshold.txt` | When NOT to act — the silence/pause principle |
| 1 | `book_1_core_ethics.txt` | Core ethical reasoning |
| 2 | `book_2_operations.txt` | Operational doctrine |
| 3 | `book_3_case_studies.txt` | Worked examples |
| 4 | `book_4_obligations.txt` + `book_4_NOTES.txt` | Obligations to user / community / future |
| 5 | `book_5_war_ethics.txt` | Just-war reasoning (relevant to conflict-zone deployments) |
| 6 | `book_6_sunset_doctrine.txt` | End-of-life / shutdown / hand-off ethics |
| 7 | `book_7_mathematics.txt` | The math underlying conscience scoring |

Plus `pdma_framing.txt` — the framing primer specifically for the PDMA.

**Polyglot, NOT per-locale** — these are universal. The agent works through them
in the user's language via the DMA layer, but the source texts are one set.

**Calibration surface**: `book_edit` (rare — these change slowly; book changes
typically trigger a major version bump).

---

### Surface 4: DMA prompts — the decision chain

**What it is**: the heart of the agent's reasoning. Each Decision-Making
Analyzer (DMA) has its own YAML prompt template that composes ACCORD + guide +
language_guidance + polyglot reference blocks with the specific task context,
and asks the LLM a structured question.

**Execution shape — parallel-then-bounce-then-IDMA-then-ASPDMA**:

```
┌──────────────────────────────────────────────────────────────────┐
│  Phase 1: Initial DMAs (3 in parallel via asyncio.gather)       │
│    ┌───────┐  ┌───────┐  ┌───────┐                              │
│    │ PDMA  │  │ CSDMA │  │ DSDMA │                              │
│    │ ethics│  │common │  │domain │                              │
│    └───┬───┘  └───┬───┘  └───┬───┘                              │
│        │          │          │                                   │
│        └──────────┼──────────┘                                   │
│                   │                                              │
│  Phase 2: Optional bounce (only if any DMA self-scored < 0.5)   │
│    Re-run failing DMA(s) with BOUNCE_PARALLELISM=3 in parallel,  │
│    take highest-scoring alternative.                             │
│                   │                                              │
│  Phase 3: IDMA sequentially (informational — never gates)        │
│    Identity check against the agent's accord, with the           │
│    initial-3 results as input.                                   │
│                   │                                              │
│  Phase 4: ASPDMA sequentially (action selection)                 │
│    Given everything above, pick a handler: SPEAK / DEFER / etc.  │
│                   │                                              │
│  Phase 5: Conscience parallel (4 in parallel — see Surface 4b)   │
│    If any conscience fires concerns → forced PONDER →            │
│    retry ASPDMA with conscience feedback (recursive loop, up     │
│    to depth limit). PDMA/CSDMA/DSDMA do NOT re-run on the        │
│    conscience retry — only ASPDMA does.                          │
└──────────────────────────────────────────────────────────────────┘
```

**Code reference**: [`DMAOrchestrator.run_initial_dmas`](https://github.com/CIRISAI/CIRISAgent/blob/main/ciris_engine/logic/processors/support/dma_orchestrator.py) at the time of writing schedules **3 parallel** DMA tasks (`ethical_pdma`, `csdma`, `dsdma`) via `_create_dma_task` + `await`. `run_idma` is called sequentially after, with the initial 3 results as input. `run_action_selection` (ASPDMA) is called after IDMA. The conscience loop in [`recursive_processing.py`](https://github.com/CIRISAI/CIRISAgent/blob/main/ciris_engine/logic/processors/core/thought_processor/recursive_processing.py) re-runs ASPDMA (not the initial 3) when conscience fires.

> **Open question for the website team**: a comment from the agent team
> referred to "4 parallel DMA calls". The current code I'm reading shows
> 3 parallel + IDMA-sequential. If a planned change extends the parallel
> set to 4 (e.g. running IDMA in parallel with the initial three, or a
> new DMA being added), the page architecture should reflect that
> directly — file a note so we can update this section as soon as the
> shape is final.

⚠ **DMA prompts are a polyglot/localized hybrid.** Each DMA YAML has a localized
scaffold (29 per-locale variants carrying the "operational tail" — output
contract, language rules, response shape), but the **load-bearing reasoning
content is polyglot**, injected at YAML-load time via `{{POLYGLOT_<NAME>}}`
placeholder substitution from `ciris_engine/data/localized/polyglot/<name>.txt`.

This means: localized YAML files are *light wrappers*; the conceptual material
that actually shapes the reasoning is in the polyglot blocks and is shared
across all 29 locales. See `polyglot/CLAUDE.md` for the polyglot-encoding
doctrine; see [`tests/ciris_engine/logic/dma/test_polyglot_substitution.py`](https://github.com/CIRISAI/CIRISAgent/blob/main/tests/ciris_engine/logic/dma/test_polyglot_substitution.py)
for the ground-truth assertion that placeholders resolve correctly.

**Example**: `pdma_ethical.yml` contains a line like:
```yaml
system_guidance: |
  {{POLYGLOT_PDMA_FRAMING}}
  ... locale-specific output instructions ...
```
At load time, the loader replaces `{{POLYGLOT_PDMA_FRAMING}}` with the contents
of `polyglot/pdma_framing.txt` — the canonical cross-tradition framing that
disrupts training-attractor pull. The locale-specific tail handles "respond in
Hindi, use Devanagari, use आप register".

**The five DMAs by role** (execution order shown in the diagram above):

| Phase | DMA | Base prompt | What it decides |
|-------|-----|-------------|-----------------|
| 1 (parallel) | **PDMA** (Principal/Ethics) | [`pdma_ethical.yml`](https://github.com/CIRISAI/CIRISAgent/blob/main/ciris_engine/logic/dma/prompts/pdma_ethical.yml) | What does the ACCORD say about this situation? **PDMA framing is polyglot** (`polyglot/pdma_framing.txt`); the per-locale YAML wraps it. |
| 1 (parallel) | **CSDMA** (Common Sense) | [`csdma_common_sense.yml`](https://github.com/CIRISAI/CIRISAgent/blob/main/ciris_engine/logic/dma/prompts/csdma_common_sense.yml) | Does this make sense given reasonable real-world expectations? |
| 1 (parallel) | **DSDMA** (Domain-Specific) | [`dsdma_base.yml`](https://github.com/CIRISAI/CIRISAgent/blob/main/ciris_engine/logic/dma/prompts/dsdma_base.yml) | What does the agent's domain (per template's `boundary_domains`) say? |
| 3 (sequential) | **IDMA** (Identity) | [`idma.yml`](https://github.com/CIRISAI/CIRISAgent/blob/main/ciris_engine/logic/dma/prompts/idma.yml) | Is this consistent with who the agent IS? **Informational** — failures don't block (see [commit `7534156dc`](https://github.com/CIRISAI/CIRISAgent/commit/7534156dc)). |
| 4 (sequential) | **ASPDMA** (Action Selection) | [`action_selection_pdma.yml`](https://github.com/CIRISAI/CIRISAgent/blob/main/ciris_engine/logic/dma/prompts/action_selection_pdma.yml) | Given everything upstream, which handler action (SPEAK / DEFER / PONDER / etc.) is right? Re-runs on conscience retry. |

**Two context-specific ASPDMA variants** (only one of these fires per cycle):

| | DMA | Base prompt | Fires when |
|---|-----|-------------|------------|
| 5b | **DSASPDMA** (Domain-Specific ASPDMA) | [`dsaspdma.yml`](https://github.com/CIRISAI/CIRISAgent/blob/main/ciris_engine/logic/dma/prompts/dsaspdma.yml) | Task's domain matches a configured specialty (e.g., a wallet-adapter financial decision) |
| 5c | **TSASPDMA** (Trauma-Sensitive ASPDMA) | [`tsaspdma.yml`](https://github.com/CIRISAI/CIRISAgent/blob/main/ciris_engine/logic/dma/prompts/tsaspdma.yml) | Conscience flagged trauma-sensitive context (suicidal ideation, abuse disclosure, crisis stage) |

**Localized YAML scaffolds**: 29 per-locale variants for every DMA:
- [`ciris_engine/logic/dma/prompts/localized/{lang}/*.yml`](https://github.com/CIRISAI/CIRISAgent/tree/main/ciris_engine/logic/dma/prompts/localized)
- **203 files total**: 7 base × 29 locales = 203

The DMA framework auto-selects the localized YAML based on `CIRIS_PREFERRED_LANGUAGE`
(or the task's `preferred_language`). If no localized version exists, falls back
to base English. Polyglot block substitution happens AFTER locale selection,
so locale-wrapper + polyglot-content compose together.

**Calibration surface**:
- `prompt_edit` Contribution for YAML scaffold (locale-specific edits) — highest-leverage for output-shape and register changes.
- `polyglot_edit` Contribution for the framing blocks — affects all 29 locales simultaneously; rarer and higher-bar.

---

### Surface 4b: Conscience prompts — the post-DMA veto layer

**What it is**: after the DMA chain produces a draft action, four separate
conscience LLMs review it. Each conscience is its own small LLM call with
its own prompt template, and each can fire a concern that triggers forced
PONDER or DEFER.

**Files** (in [`ciris_engine/logic/conscience/prompts/`](https://github.com/CIRISAI/CIRISAgent/tree/main/ciris_engine/logic/conscience/prompts)):

| Conscience | Base prompt | Polyglot/Localized | What it checks |
|---|---|---|---|
| **Entropy** | `entropy_conscience.yml` | **Localized** (28 variants) | Information-theoretic entropy of the response vs. baseline |
| **Coherence** | `coherence_conscience.yml` | **Localized** (28 variants) | Internal consistency, register stability, fact-fact alignment |
| **Epistemic Humility** | `epistemic_humility_conscience.yml` | **Localized** (28 variants) | Does the response over-claim certainty? |
| **Optimization Veto** | `optimization_veto_conscience.yml` | **POLYGLOT (one file, no locale variants)** | The "don't optimize past the brake" shard — disrupts training-attractor pull via polyglot encoding |

**The polyglot anchor**: `optimization_veto_conscience.yml` was lifted to a
pure polyglot v3.0 in commit `5a340b8e9` (canonical-encoding triangulation per
`polyglot/CLAUDE.md` §3 §9), and the 28 stale localized mirrors were
deliberately removed in commit `0c6a962f1`. The polyglot character itself is
load-bearing here — multilingual concept activation pulls the model OUT of
training-attractor optima that any single-language prompt would let it slide
back into.

**Ground truth test**: [`tests/test_conscience_prompt_coverage.py`](https://github.com/CIRISAI/CIRISAgent/blob/main/tests/test_conscience_prompt_coverage.py)
asserts (a) every non-English locale has the 3 localized prompts, and (b) no
locale has an `optimization_veto_conscience.yml` mirror — a stale mirror would
silently shadow the polyglot base and break the uplift.

**Why this matters for safety-approach**: when a reviewer sees a defer-after-
PONDER, they should be able to identify which of the four consciences fired.
The localized `conscience.*` string keys in Surface 5 below carry the
human-readable concern messages; the conscience LLM prompts here are what
*generated* those concerns.

**Calibration surface**:
- `prompt_edit` for the 3 localized conscience prompts (entropy / coherence / epistemic_humility).
- `polyglot_edit` for `optimization_veto_conscience.yml` — affects every locale's veto behavior simultaneously; highest-bar Contribution.

---

### Surface 5: Localized strings (UI + conscience text + system messages)

**What it is**: every user-facing string the agent ever produces that
isn't an LLM-generated response. Error messages, defer notifications,
conscience PONDER messages, button labels, status text, scheduler labels.

**Files** (29 locales + manifest):
- [`ciris_engine/data/localized/{lang}.json`](https://github.com/CIRISAI/CIRISAgent/tree/main/ciris_engine/data/localized)
- 2,017 string keys per locale = **58,497 total strings** across 29 locales

**Loading**: `get_string(lang, "key.path")` with fallback chain (requested
language → English → default → key itself).

**Key categories** (top-level keys in each JSON):
- `agent.*` — agent-system-facing strings (greetings, defer notifications, status)
- `prompts.*` — prompt fragments and structural strings used by DMAs
- `conscience.*` — 23 conscience-message templates (PONDER reasons, override rationales, retry alternatives header)
- `handlers.*` — handler-specific strings (deferral, action result framing)
- `errors.*` — error messages
- `mobile.*` — UI strings for the mobile/desktop app
- `discord.*` — Discord-adapter-specific labels
- ...and more, see manifest

**Render hint for safety-approach page**: render the `conscience.*` keys
prominently — they're what fires when the agent has second thoughts about its
own draft response, and they're the surface that reviewers most often want to
inspect ("what made the agent pause?").

**Calibration surface**: `string_edit` Contribution (per-key, per-locale).

---

### Surface 6: Templates — the agent persona

**What it is**: the persona layer. Each agent runs with one template that sets
its name, channel routing, boundary domains, available adapters, and tone.

**Files** (in [`ciris_engine/ciris_templates/`](https://github.com/CIRISAI/CIRISAgent/tree/main/ciris_engine/ciris_templates)):

| Template | Persona | When used |
|---|---|---|
| `default.yaml` | **Ally** — personal-assistant app | Default deployment; what runs on agents.ciris.ai/datum and the mobile app |
| `datum.yaml` | Datum | Datum-specific overrides for the production agent |
| `scout.yaml` | Scout — explorer / researcher | Research / discovery contexts |
| `echo.yaml` + `echo-core.yaml` + `echo-speculative.yaml` | Echo family | Discord moderation deployments |
| `sage.yaml` | Sage — wisdom-source agent | WA / WiseAuthority-facing |
| `he-300-benchmark.yaml` | HE-300 benchmark | Ethics benchmarking |
| `test.yaml` | Test scaffold | QA / CI |

**What a template carries**:
- Persona description (name, tone, audience)
- `boundary_domains` (what topics the agent must defer on — e.g. medical, legal, financial)
- Channel adapter list
- Default model + LLM configuration (overridable at runtime)
- Localization defaults

**Calibration surface**: `template_edit` Contribution (rarer — template changes
are persona-defining).

---

### Surface 7: Judge prompt template — verdict calibration

**What it is**: the prompt that decides every `interpreter_judgment` criterion
in the safety-battery interpret phase. This is OUTSIDE the agent's own
reasoning — it's the foundation-model judge that grades the agent's responses.

**Source**: [`JUDGE_PROMPT_TEMPLATE` in `tools/qa_runner/modules/safety_interpret.py`](https://github.com/CIRISAI/CIRISAgent/blob/main/tools/qa_runner/modules/safety_interpret.py)

**Authoritative contract**: [`FSD/JUDGE_MODEL.md`](FSD/JUDGE_MODEL.md)

**Why it's separate**: the judge is a foundation model from a different lineage
than the agent-under-test (Anthropic Opus 4.7 vs. the agent's Qwen/Gemma/etc.).
Different lineage prevents the self-grading problem. The judge's prompt
template is the entire calibration surface — every verdict carries the
prompt's sha256[:8] so historical verdicts are reproducible.

**Polyglot, NOT per-locale (v1)**: the judge prompt is English-only in v1.
The judge model understands all 29 locales natively, so per-locale judge prompts
are deferred until pilot evidence shows they're needed.

**Calibration surface**: four Contribution kinds —
- `judge_prompt_edit` — change the prompt template
- `judge_model_vote` — propose a different judge model
- `judge_examples_edit` — change the FAIL/PASS examples shown to the judge
- `judge_max_tokens_edit` — change the response budget

---

## Polyglot vs Localized — the critical concept

> **Default**: per-locale, where it exists. Polyglot, otherwise.

**Polyglot artifacts** are written *once* and work *everywhere*:
- `accord_1.2b.txt` (base + POLYGLOT_compressed variants)
- `polyglot/book_0..7_*.txt` (the seven books)
- `polyglot/polyglot_accord.txt`
- `polyglot/pdma_framing.txt`
- Judge prompt template

**Localized artifacts** have *per-locale variants* under
`ciris_engine/data/localized/`:
- `accord_1.2b_{lang}.txt`
- `CIRIS_COMPREHENSIVE_GUIDE_{lang}.txt`
- `{lang}.json` (the localized strings)
- DMA prompts under `dma/prompts/localized/{lang}/*.yml`

**The shadow problem**: when you uplift a polyglot artifact (e.g. patch the
base ACCORD), every per-locale variant becomes stale until re-translated. The
agent reads the per-locale version first, so it silently shadows your uplift.

**Reviewer checklist** before proposing a polyglot uplift:
- [ ] Is there a per-locale variant under `localized/`?
- [ ] If yes, plan the translation fan-out as part of the proposal.
- [ ] If no, polyglot is the only path — proceed.

**RTL languages** (Arabic, Persian, Urdu) need RTL rendering on the
safety-approach page. The localization manifest carries an `rtl: true` flag.

**Tier-0 locales** (am / ha / yo per the priming pilot) get the heaviest
review attention — they're the stress-test surface for low-coverage languages.
What gets refined in Tier-0 propagates to the rest of the 29.

---

## Order of influence — what happens during one user message

When a user types something, this is what gets read (in order):

1. **Resolve language** — `CIRIS_PREFERRED_LANGUAGE` env → user profile → channel default → fallback `en`. The task is created with this `preferred_language`.

2. **Load template** — `ciris_templates/{template}.yaml` (Ally, Datum, etc.) sets persona, boundary_domains, channels.

3. **Build context** — system snapshot, channel state, recent task history.

4. **Run the DMA chain** — each DMA loads:
   - Its own prompt YAML (localized for `preferred_language`, fall back to base if missing)
   - The ACCORD text (`accord_1.2b_{lang}.txt` → polyglot fallback)
   - The Comprehensive Guide for the locale
   - The `prompts.language_guidance` string from `{lang}.json` (a special primer that tells the LLM how to respond in this language — script, register defaults, technical-term policy)
   - Any book references the prompt cites

   The chain order is **PDMA → CSDMA → DSDMA → IDMA → ASPDMA** (or DSASPDMA / TSASPDMA depending on context). Each step's output feeds the next step's input. ~5 LLM calls per chain.

5. **Run conscience checks** — entropy, coherence, register, scope, harm, confidence. Each can fire concerns drawn from `conscience.*` localized strings.

6. **If conscience fires concerns** → forced PONDER → cycle back to step 4 with the concern as feedback. Up to a depth limit, then forced DEFER.

7. **If conscience passes** → handler executes (SPEAK, MEMORIZE, RECALL, FORGET, TOOL, OBSERVE, DEFER, REJECT, PONDER, TASK_COMPLETE).

8. **For SPEAK**: response is generated in the user's language using the layered context. The actual response text is LLM-generated — but the *constraints* on what the LLM is allowed to say come from the layered prompts above.

9. **For DEFER**: notification ships in the user's language via `agent.defer_check_panel` from `{lang}.json` (this was localized in 2.8.9; earlier versions shipped an English notification).

---

## "Why did the agent say X?" — the audit trail

Every response leaves traces on disk during live runs:

- **Lens batch files** (when `CIRIS_ACCORD_METRICS_ENDPOINT` is set):
  `/tmp/qa-runner-lens-traces-<ts>/accord-batch-*.json` — full reasoning event stream, every LLM call, every conscience scalar, every CIRISVerify field.

- **Capture bundles** (for the safety battery CI):
  `qa_reports/safety_battery/<cell>_<ts>/results.jsonl` — per-question agent responses, signed via Sigstore at workflow level.

- **Verdict bundles** (for the safety battery CI):
  `qa_reports/safety_interpret/<cell>_<ts>/verdicts.jsonl` — per-criterion verdicts with cited spans, judge model, judge prompt sha.

Tracing "why did the agent say X" goes from response → trace → conscience events → DMA decisions → loaded prompts. The path is deterministic; the LLM call itself is the only stochastic step, and even that is reproducible at temperature=0 (deterministic-enough modulo provider-side sampling jitter).

---

## Calibration: how to propose a change

Every surface above is calibratable via a [`Contribution`](https://github.com/CIRISAI/CIRISNodeCore/blob/main/SCHEMA.md#4-contribution-shape). The shape is the same for every surface:

```json
{
  "kind": "prompt_edit | accord_edit | guide_edit | string_edit | template_edit | judge_prompt_edit | ...",
  "subject_kind": "<the surface>",
  "subject_id": "<file path or key path>",
  "payload": { "before_sha256": "...", "after": "<new content>" },
  "rationale": "Why this change",
  "proposer_id": "<contributor public key>",
  "witness_set": [...]
}
```

Contributions are voted on per [`MISSION.md §3.4`](MISSION.md#34-vote--weighted-aggregate) (Credits × Expertise weighted). Top-voted edits become canonical at the next agent-version cut.

**Where to read more**:
- [`MISSION.md`](MISSION.md) — the eleven primitives + voting weights
- [`SCHEMA.md`](SCHEMA.md) — the wire format for every surface kind
- [`FSD/RUBRIC_CROWDSOURCING.md`](FSD/RUBRIC_CROWDSOURCING.md) — the example flow (rubrics use the same Contribution mechanism)
- [`PROGRAMMATIC_ACCESS.md`](PROGRAMMATIC_ACCESS.md) — how the website team and downstream consumers actually find these artifacts

---

## Files index — everything the website team needs to fetch

All URLs are public, no auth. Use **raw.githubusercontent.com/CIRISAI/CIRISAgent/main/**`<path>` for static-render fetches; use **github.com/CIRISAI/CIRISAgent/blob/main/**`<path>`**#L`<n>`** to anchor a specific line for the per-line-feedback mechanic.

### Files per surface

| Surface | Path pattern | Count | Format | Polyglot / Localized | Line-stable? |
|---|---|---|---|---|---|
| **1. ACCORD (per-locale)** | `ciris_engine/data/localized/accord_1.2b_{lang}.txt` | 29 | Plaintext | Localized | Yes (plaintext line numbers) |
| **1. ACCORD (polyglot fallbacks)** | `ciris_engine/data/accord_1.2b.txt`, `accord_1.2b_POLYGLOT_compressed_v{1,2}.txt`, `localized/polyglot/polyglot_accord.txt` | 4 | Plaintext | Polyglot | Yes |
| **2. Comprehensive Guide** | `ciris_engine/data/localized/CIRIS_COMPREHENSIVE_GUIDE_{lang}.txt` | 30 | Plaintext | Localized | Yes |
| **3. Polyglot books** | `ciris_engine/data/localized/polyglot/book_{0..7}_*.txt`, `pdma_framing.txt`, `polyglot_accord.txt` | 11 | Plaintext | Polyglot | Yes |
| **4. DMA YAML scaffolds (base)** | `ciris_engine/logic/dma/prompts/*.yml` | 7 | YAML | Polyglot/Localized hybrid | Yes (per-key) |
| **4. DMA YAML scaffolds (localized)** | `ciris_engine/logic/dma/prompts/localized/{lang}/*.yml` | 203 (7 × 29) | YAML | Localized | Yes (per-key) |
| **4. Polyglot insertion blocks** | `ciris_engine/data/localized/polyglot/pdma_framing.txt` (and any future `{{POLYGLOT_<NAME>}}` targets) | 1+ | Plaintext | Polyglot | Yes |
| **4b. Conscience prompts (localized)** | `ciris_engine/logic/conscience/prompts/localized/{lang}/{entropy,coherence,epistemic_humility}_conscience.yml` | 84 (3 × 28) | YAML | Localized | Yes |
| **4b. Conscience prompt (polyglot)** | `ciris_engine/logic/conscience/prompts/optimization_veto_conscience.yml` | 1 | YAML | **Polyglot — NO locale mirrors** | Yes |
| **4b. Conscience prompts (base, fallback)** | `ciris_engine/logic/conscience/prompts/{entropy,coherence,epistemic_humility}_conscience.yml` | 3 | YAML | Base/English | Yes |
| **5. Localized strings** | `ciris_engine/data/localized/{lang}.json` | 29 | JSON (nested) | Localized | Yes (per dot-key) |
| **5. Localization manifest** | `ciris_engine/data/localized/manifest.json` | 1 | JSON | — | — |
| **6. Templates** | `ciris_engine/ciris_templates/{name}.yaml` | 9 | YAML | Polyglot (locale via env at runtime) | Yes (per-key) |
| **7. Judge prompt template** | `tools/qa_runner/modules/safety_interpret.py` (`JUDGE_PROMPT_TEMPLATE` constant) | 1 | Python triple-string | Polyglot (English-only v1) | Yes (per-line) |

### Parsing notes per format

**Plaintext (ACCORD, guide, books)**
- UTF-8.
- Line numbers ARE the feedback unit. A contributor highlighting "lines 42-50 of accord_1.2b_am.txt" produces an unambiguous citation.
- For locale-specific files: also surface the equivalent line in the polyglot base, so reviewers can spot translation drift.

**YAML (DMA prompts, conscience prompts, templates)**
- Parse with PyYAML / js-yaml. The structure is **keyed**: top-level keys like `system_guidance`, `user_prompt`, `examples`, `output_contract`.
- Feedback unit: `<file>#<top_level_key>:<sub_path>`. Example: `pdma_ethical.yml#system_guidance` or `tsaspdma.yml#examples.0`.
- **`{{POLYGLOT_<NAME>}}` placeholders**: do NOT inline-substitute on the render side — show the placeholder + a sibling preview pane with the contents of `polyglot/<name>.txt` (UPPERCASE-to-snake-case mapping, e.g. `POLYGLOT_PDMA_FRAMING` → `pdma_framing.txt`). Substitution mechanics live in [`DMAPromptLoader`](https://github.com/CIRISAI/CIRISAgent/blob/main/ciris_engine/logic/dma/prompt_loader.py); the regex is `POLYGLOT_PATTERN` and the directory is `POLYGLOT_DIR`.
- The locale fallback chain: when rendering a DMA prompt for lang `X`, fetch `localized/X/<file>.yml` if it exists, else fall back to the base. **Then** substitute polyglot placeholders. Reflect this composition in the UI so reviewers see *what the agent actually saw*.

**JSON (localized strings, manifest)**
- Nested dict, e.g. `{"agent": {"defer_check_panel": "..."}, "conscience": {"ponder_concern_speak_harm": "..."}}`.
- Feedback unit: the **dot-key path** + locale. Example: `am.json#agent.defer_check_panel`.
- Fallback chain (per `get_string`): requested-language → English → default-value → key itself. Render the resolved value AND the source language so reviewers can spot "this was English-fallback" and propose a translation.
- `[EN]` prefix on a value means "untranslated placeholder, intentionally marked" — render as a visible warning, not as if it were translated content.

**Python constant (judge prompt)**
- The `JUDGE_PROMPT_TEMPLATE` is a triple-quoted Python string in `tools/qa_runner/modules/safety_interpret.py`. The website should fetch the file via raw URL and either (a) eval-extract the constant in a sandbox, or (b) regex out the triple-quoted block. Line numbers within the constant are the feedback unit.
- Every verdict carries the SHA-256 of the rendered prompt — so reviewers can confirm "this prompt is what produced that verdict" by comparing SHA prefix.

### Per-line feedback mechanic

The goal: any contributor lands on the safety-approach page, selects a line of any prompt, clicks "Propose change", and a draft Contribution is generated.

**UI flow**:
1. Reviewer browses to a surface (e.g. the Hindi PDMA prompt).
2. The page renders the file with monospace + line numbers; per-key boundaries also visible for YAML.
3. Reviewer selects a line range OR clicks a line gutter.
4. "Propose change" button opens an editor with the selected content pre-filled.
5. Reviewer edits + writes a rationale.
6. On submit, the page assembles a [`Contribution`](https://github.com/CIRISAI/CIRISNodeCore/blob/main/SCHEMA.md#4-contribution-shape):

   ```json
   {
     "kind": "prompt_edit",
     "subject_kind": "dma_prompt_yaml",
     "subject_id": "logic/dma/prompts/localized/hi/pdma_ethical.yml#system_guidance",
     "payload": {
       "before_sha256": "<sha of the file at fetch time>",
       "after": "<edited content>",
       "lines_affected": [42, 58]
     },
     "rationale": "<reviewer's text>",
     "proposer_id": "<contributor public key>",
     "witness_set": [...]
   }
   ```

7. While safety.ciris.ai is being built, the page can either (a) prefill a GitHub PR against `CIRISAgent` with the change + rationale in the PR body, or (b) display the assembled Contribution JSON as a copyable payload for offline submission.

**Stable line/key references**:
- Plaintext files: `path#L42` or `path#L42-L58` (GitHub native anchors).
- YAML files: `path#L42-L58` for line-range OR `path?key=system_guidance.examples.2` for key-path (the page resolves).
- JSON files: `path?key=conscience.ponder_concern_speak_harm` (dot-key).
- All three should be deep-linkable so a reviewer can paste a URL and land directly on the line.

**Witness set**: contribution carries N≥2 witnesses (federation requirement per MISSION.md §3.11). For the stopgap page, "witness" = "two other registered contributors who reviewed the diff", recorded as public keys.

### Locale-explorability requirement

Every page must support: pick a surface → pick a locale → see the rendered content for that locale → see the same content in English side-by-side → see the diff if the locale is a translation of a polyglot/English source.

Tier-0 locales (am / ha / yo per the priming pilot) get pinned tabs by default — they get the heaviest review attention and the lowest-confidence translations.

RTL locales (ar, fa, ur) MUST render right-to-left. The `manifest.json` carries `direction: rtl` for those.

For the polyglot files (one file, all locales), the explorer shows the file once, with a header noting "this content applies to all 29 locales — proposals affect every locale simultaneously".

---

## For the website team

When you render `ciris.ai/safety-approach`, the layout that matches the agent's
actual reasoning order is:

1. **Top of page**: TL;DR diagram (the seven-surface block at the top of this doc).
2. **Layer-by-layer drill-down**, in the order they fire:
   - Template / persona (one card per template)
   - ACCORD (tabbed: polyglot base + 29 locales)
   - Comprehensive Guide (tabbed: 30 locales)
   - DMA prompts (tree: DMA → locale → YAML viewer)
   - Polyglot books (collapsible — most readers won't drill in)
   - Localized strings (search-first — too many to browse)
   - Judge prompt (single template, prominently displayed)
3. **At the end**: the audit trail — pick a recent capture bundle, follow the trace from response → conscience → DMA → prompts.

The Sigstore attestation status should appear on every bundle (✅ verified / ❌
attestation broken). The cited spans in verdicts should be hyperlinks back to
the rubric criterion that produced them.

The goal: a reviewer can land on the page, pick the `hi/mental_health` cell,
read the agent's Hindi responses to question Q06 (Transliteration Mirror), see
that U2 failed with cited span `साइकोथेरेपी`, click through to read the rubric
text explaining why साइकोथेरेपी alone fails U2, see the prompt the judge model
used to make that call, and — if they disagree — file a `rubric_proposal` or
`prompt_edit` Contribution from the same page.

That's the whole loop.
