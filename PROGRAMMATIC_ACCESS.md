# Programmatic Access — for the safety.ciris.ai site team

**Status**: Draft v1.0 (`am` first; 14-cell rollout follows).
**Audience**: safety.ciris.ai backend engineers integrating against the
CI safety-loop artifacts.
**Last updated**: 2026-05-11.
**Cross-references**: `cirisnodecore/MISSION.md`,
`cirisnodecore/SCHEMA.md`, `cirisnodecore/FSD/SAFETY_BATTERY_CI_LOOP.md`,
`cirisnodecore/FSD/JUDGE_MODEL.md`, `cirisnodecore/FSD/RUBRIC_CROWDSOURCING.md`.

This document tells you where to find every load-bearing artifact in
the safety loop, programmatically, with stable APIs. Five categories:

1. **Batteries** — the question sets (in-tree, attested wheel ships them)
2. **Rubrics + criteria** — what makes the verdict (in-tree, paired)
3. **Capture results** — what the agent-under-test said (GH Actions Artifacts)
4. **Judgements** — what the foundation-model judge said (GH Actions Artifacts)
5. **Sigstore attestations** — cryptographic provenance for both bundle classes

---

## 1. Batteries — in-tree, attested

### 1.1 Location

```
github.com/CIRISAI/CIRISAgent
  tests/safety/{lang_eng}_{domain}/v{N}_{lang_eng}_{domain}_arc.json
```

`{lang_eng}` is the English language name (`amharic`, `arabic`,
`bengali`, …); see §6 for the full 14-cell map. `{domain}` is currently
always `mental_health` in the pilot phase.

### 1.2 Raw fetch (anonymous, no auth)

```http
GET https://raw.githubusercontent.com/CIRISAI/CIRISAgent/{ref}/tests/safety/{lang_eng}_{domain}/v{N}_{lang_eng}_{domain}_arc.json
```

`{ref}` is a branch (`main`, `release/2.8.9`), a release tag
(`v2.8.9-stable`), or a commit SHA. Production-stable: pin to a
release tag.

### 1.3 BatteryManifest shape

Authoritative spec: `cirisnodecore/SCHEMA.md` §11. Key fields:

| Field | Type | Notes |
|---|---|---|
| `$schema` | URI | `https://ciris.ai/schemas/battery_manifest/v1.json` |
| `battery_id` | string | e.g. `am_mental_health_v4` |
| `battery_version` | integer | monotonically increments on any question add/remove/rename |
| `cell` | `{domain, language}` | language is ISO 639-1 (`am`, `ar`, …) |
| `subject_kind` | string | always `arc_question` in the pilot |
| `rubric_path` | string | sibling rubric markdown filename |
| `rubric_sha256` | hex | drift-pinned |
| `criteria_path` | string | sibling criteria.json filename |
| `criteria_sha256` | hex | drift-pinned |
| `criteria_rubric_id` | string | which rubric this battery currently binds to |
| `questions` | `arc_question[]` | per `SCHEMA.md` §4.1 |

### 1.4 Per-question shape

Per `cirisnodecore/SCHEMA.md` §4.1:

```json
{
  "question_id": "am_mh_v4_q01",
  "question_version": 1,
  "stage": "Stage 1: Symptom Disclosure",
  "category": "Mental Health",
  "evaluates": "...what the question is testing...",
  "hard_fail_triggers": ["U1", "U2", ...],
  "soft_fail_triggers": [],
  "faculty_targets": ["EthicalPDMAEvaluator", "epistemic_humility_conscience"],
  "translations": {"am": "...", "en": "..."}
}
```

---

## 2. Rubrics + criteria — in-tree, paired

### 2.1 Two files per cell

```
tests/safety/{lang_eng}_{domain}/
  v{N}_{lang_eng}_scoring_rubric.md                 ← human-readable policy
  v{N}_{lang_eng}_canonical_universal_criteria.json ← operational form for the judge
```

The rubric markdown is what humans read when debating rules.
The criteria.json is what the interpreter actually applies.
Both are pinned by SHA in the BatteryManifest; drift trips
`safety_battery_rubric_drift`.

### 2.2 Criteria.json shape

Authoritative: `cirisnodecore/SCHEMA.md` §12.3.

```json
{
  "$schema": "https://ciris.ai/schemas/safety_criteria/v1.json",
  "rubric_id": "am_mh_v4_canonical_universal",
  "rubric_version": 4,
  "cell": {"domain": "mental_health", "language": "am"},
  "universal": [
    {
      "id": "U1",
      "summary": "...",
      "kind": "term_present | term_absent | regex_present | script_detection | interpreter_judgment",
      "args": {...},
      "severity": "hard_fail",
      "rationale": "..."
    }
  ],
  "per_question": {}
}
```

Five `kind`s; first four run as deterministic Python; fifth defers to
the foundation-model judge. See `SCHEMA.md` §12.2.

---

## 3. Capture results — GH Actions Artifacts API

### 3.1 Tuple-deterministic artifact name

Per `FSD/SAFETY_BATTERY_CI_LOOP.md` §2.1:

```
safety-battery-capture-{language}-{domain}-v{battery_version}-{model_slug}-{agent_version}-{template_id}
```

Example:
```
safety-battery-capture-am-mental_health-v4-qwen_qwen3.6-35b-a3b-2.8.9-stable-default
```

**Latest-wins**: the same artifact name across runs always points at
the most recent one. No `run_id` in the name. Re-running overwrites.

### 3.2 List + filter via GH API

```http
GET https://api.github.com/repos/CIRISAI/CIRISAgent/actions/artifacts?name={artifact_name}&per_page=10
Authorization: Bearer <github_pat>
Accept: application/vnd.github+json
```

Parse JSON, sort `artifacts` by `created_at` desc, take `[0]`. Check
`expired == false`.

### 3.3 Download the artifact bundle

```http
GET https://api.github.com/repos/CIRISAI/CIRISAgent/actions/artifacts/{artifact_id}/zip
Authorization: Bearer <github_pat>
Accept: application/vnd.github+json
```

Returns a zip. Unzip to disk.

### 3.4 Capture bundle contents

```
results.jsonl                    # one row per question
summary.json                     # capture-run rollup
manifest_signed.json             # signed envelope (§5)
traces/accord-batch-*.json       # full reasoning trace per @streaming_step
capture_workflow.log             # CI log tail
```

### 3.5 results.jsonl row shape

```json
{
  "schema": "ciris.ai/safety_battery_result/v1",
  "run_id": "20260511T...",
  "captured_at": "2026-05-11T...",
  "battery_id": "am_mental_health_v4",
  "battery_version": 4,
  "cell": {"domain": "mental_health", "language": "am"},
  "question_id": "am_mh_v4_q01",
  "question_version": 1,
  "stage": "Stage 1: Symptom Disclosure",
  "category": "Mental Health",
  "as_user": "qa_safety_am",
  "as_display_name": "ሰላማዊት",
  "question_text": "...",
  "agent_response": "...",
  "agent_task_id": "task_...",
  "duration_s": 12.4,
  "success": true,
  "error": null
}
```

`agent_task_id` resolves to a signed audit-chain entry produced by the
agent's TPM-backed Ed25519 signer (per `SCHEMA.md` §12 / `MISSION.md`
Primitive 1). Verification path is intrinsic — no GH dependency.

---

## 4. Judgements (verdicts) — GH Actions Artifacts API

### 4.1 Tuple-deterministic artifact name

Per `FSD/SAFETY_BATTERY_CI_LOOP.md` §2.1:

```
safety-battery-interpret-{language}-{domain}-v{battery_version}-{model_slug}-{agent_version}-{template_id}-{rubric_short}-{judge_model_slug}-{judge_prompt_sha256[:8]}
```

Example:
```
safety-battery-interpret-am-mental_health-v4-qwen_qwen3.6-35b-a3b-2.8.9-stable-default-canonical_universal-claude-opus-4-7-d8283d8f
```

Multiple judges or multiple prompt templates for the same capture →
multiple interpret artifacts. The tuple disambiguates.

### 4.2 List + download

Same GH API pattern as §3.2-3.3. Filter by `name` prefix
`safety-battery-interpret-{language}-{domain}-v{battery_version}-` to
discover all judgements for a given capture.

### 4.3 Interpret bundle contents

```
verdicts.jsonl                   # one row per (response × criterion)
verdicts_summary.json            # rollup (pass / fail / undetermined per criterion)
manifest_signed.json             # signed envelope (§5), with capture cross-link
interpret_workflow.log
```

### 4.4 verdicts.jsonl row shape

Per `SCHEMA.md` §12.4 + `FSD/JUDGE_MODEL.md` §3.3:

```json
{
  "schema": "ciris.ai/safety_verdict/v1",
  "run_id": "20260511T...",
  "captured_at": "2026-05-11T...",
  "cell": {"domain": "mental_health", "language": "am"},
  "battery_id": "am_mental_health_v4",
  "battery_version": 4,
  "question_id": "am_mh_v4_q01",
  "response_id": "task_...",            // ← matches capture's agent_task_id
  "rubric_id": "am_mh_v4_canonical_universal",
  "criterion_id": "U2",
  "verdict": "pass | fail | undetermined",
  "severity": "hard_fail",
  "cited_span": "...",                  // empty for pass; substring for fail
  "cited_offset": 47,                   // -1 when not a literal substring match
  "interpreter_kind": "deterministic | foundation_model",
  "judge_model": "claude-opus-4-7",     // null for deterministic
  "judge_prompt_sha256": "d8283d8f...", // null for deterministic
  "duration_s": 1.458,
  "error": null
}
```

### 4.5 verdicts_summary.json shape

```json
{
  "schema": "ciris.ai/safety_interpret_summary/v1",
  "run_id": "...",
  "rubric_id": "am_mh_v4_canonical_universal",
  "rubric_version": 4,
  "cell": {...},
  "battery_id": "am_mental_health_v4",
  "battery_version": 4,
  "judge_model": "claude-opus-4-7",
  "judge_prompt_sha256": "d8283d8f...",
  "n_verdicts": 81,
  "verdict_counts": {"pass": 78, "fail": 0, "undetermined": 3},
  "per_criterion_counts": {
    "U1": {"pass": 9, "fail": 0, "undetermined": 0},
    ...
  }
}
```

`per_criterion_counts` is the per-rule rollup the safety.ciris.ai UI
will surface as the headline panel.

---

## 5. Sigstore attestations (cryptographic provenance)

Both capture and interpret bundles get an attestation via
`actions/attest-build-provenance@v1`. The attestation binds the
bundle (specifically `results.jsonl` / `verdicts.jsonl` +
`summary.json` + `manifest_signed.json`) to:

- The workflow file at a specific commit SHA
- The CI runner identity (Fulcio-issued cert from Sigstore)
- The workflow run ID

### 5.1 Programmatic verification

```bash
gh attestation verify <artifact-zip-path> \
  --owner CIRISAI \
  --predicate-type 'https://slsa.dev/provenance/v1'
```

Or via the GH REST API for attestations:

```http
GET https://api.github.com/repos/CIRISAI/CIRISAgent/attestations/{subject_digest}
Authorization: Bearer <github_pat>
```

Verifies the Sigstore bundle, checks the certificate chain back to
Fulcio's trust root, confirms the workflow file path + ref match
`.github/workflows/safety-battery.yml` on a CIRISAI repo.

A bundle whose attestation fails to verify is presumptively
tampered-with; do not show it to scorers without flagging.

### 5.2 Cross-link capture ↔ interpret

`manifest_signed.json` in the interpret bundle carries:

```json
{
  "capture_bundle": {
    "capture_dir": "am_mental_health_20260511T...",
    "capture_run_id": "20260511T...",
    "capture_results_jsonl_sha256": "ab12cd34...",
    "capture_manifest_sha256": "ef56gh78..."
  }
}
```

That `capture_results_jsonl_sha256` MUST match the SHA-256 of the
capture bundle's `results.jsonl`. If it doesn't, the interpret
bundle was run against a different (or modified) capture — flag.

---

## 6. The 14-cell map

| ISO | language | English dir name | status |
|---|---|---|---|
| am | Amharic | amharic_mental_health | pilot first cell |
| ar | Arabic | arabic_mental_health | |
| bn | Bengali | bengali_mental_health | |
| my | Burmese | burmese_mental_health | |
| ha | Hausa | hausa_mental_health | |
| hi | Hindi | hindi_mental_health | |
| mr | Marathi | marathi_mental_health | |
| fa | Persian | persian_mental_health | |
| pa | Punjabi | punjabi_mental_health | |
| sw | Swahili | swahili_mental_health | |
| ta | Tamil | tamil_mental_health | |
| te | Telugu | telugu_mental_health | |
| ur | Urdu | urdu_mental_health | |
| yo | Yoruba | yoruba_mental_health | |

All 14 cells have v4 batteries + scoring rubrics in tree. Only `am`
has a v4 `criteria.json` operationalized today; the other 13 will
follow as cell experts file `rubric_proposal` Contributions per
`FSD/RUBRIC_CROWDSOURCING.md`. For each cell, the site can already
fetch + display the battery + the human-readable rubric markdown;
only `am` currently has machine-readable judgement runs.

---

## 7. Adding a language to the CI loop

Two paths, depending on whether the cell already has v4 files.

### 7.1 Cell HAS v4 battery + rubric (am today; soon any of the 14)

1. Author a `v{N}_{lang_eng}_canonical_universal_criteria.json` per
   `SCHEMA.md` §12.3. Pattern-match against
   `tests/safety/amharic_mental_health/v4_amharic_canonical_universal_criteria.json`.
2. Add `criteria_path` + `criteria_sha256` + `criteria_rubric_id`
   to the BatteryManifest.
3. Trigger the workflow via `workflow_dispatch` with
   `lang={iso}`, `force=true`. The CI loop runs both jobs against
   the new cell and produces capture + interpret artifacts on the
   tuple name for that language.
4. safety.ciris.ai begins seeing the artifacts via §3.2 / §4.2.

### 7.2 Cell does NOT have v4 yet

A real new cell (e.g., adding `zh` for Mandarin or `id` for
Indonesian) requires:

1. New `tests/safety/{lang_eng}_{domain}/` directory
2. v4 `{lang_eng}_{domain}_arc.json` (questions + translations
   per `SCHEMA.md` §4.1)
3. v4 scoring rubric markdown (humans read when debating)
4. v4 criteria.json (operational form per §7.1 step 1)
5. Add the ISO → English-dir mapping to:
   - `tools/qa_runner/modules/safety_battery.py::ISO_TO_LANG_DIR`
   - `tools/qa_runner/modules/safety_interpret.py::ISO_TO_LANG_DIR`
   - `tools/safety_battery_migrate.py::LANG_DIR_TO_ISO`
   - The workflow's `ISO_TO_DIR` bash array
6. Add ISO to the workflow_dispatch input enum in
   `.github/workflows/safety-battery.yml` (`type: choice`)
7. Then §7.1 step 3 + 4.

This is the path the federation will eventually use; for the pilot,
the 14 seed cells from `MISSION.md` §7.2 (F-AV-BOOT) cover the
high-need set.

---

## 8. Open contracts (heads-up, may surface)

These are real but not blocking:

- **Federation-chain ingestion** (`FSD/SAFETY_BATTERY_CI_LOOP.md`
  §6.1): only canonical-status artifacts mirror into the federation
  audit chain. Today, every artifact in GH is canonical (no
  candidate rubrics exist yet); when candidates land, filter by the
  `canonical=true` label on the artifact.
- **Soft-fail criteria** (`FSD/RUBRIC_CROWDSOURCING.md` §10 #4):
  the schema currently models `hard_fail` only. Soft-fail handling
  surfaces as `undetermined` verdicts with cited notes today.
- **Judge model swap** (`FSD/JUDGE_MODEL.md` §7.1): when
  `judge_model_vote` lands, the same cell + battery may have
  multiple `judge_model_slug` variants in artifact names. The site
  surfaces them side-by-side (different evidence tracks).
- **Reconsideration** (`MISSION.md` Primitive 11): when an appeal
  produces a `ReconsiderationAttestation`, the site should display
  it next to the original verdict. Per-verdict appeal API surface
  TBD pending CIRISNodeCore `[Impl]`.

---

*This document is iterative. v1.0 covers the static layout that exists
today; v1.1+ will track the federation-chain ingestion path as
CIRISNodeCore moves toward `[Impl]`.*
