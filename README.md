# `ciris-node-core`

The federation-consensus layer for the CIRIS network. Eleven primitives
covering identity, weighted voting, contribution-driven calibration,
truth-grounding via signed evidence, slashing, and reconsideration.

The contract that `safety.ciris.ai` (the first pilot) builds against
and that the eventual `ciris-node-core` Rust crate implements.

**Status:** Spec + v0.1.0-dev Rust crate skeleton. v1.0 mission +
schema + four FSDs are in this repo, plus the substrate-integration
FSD and a compiling crate skeleton at `src/` that publishes the wire
types and the `NodeCoreEngine` trait mirroring
[`CIRISPersist/FSD/CIRIS_PERSIST.md`](https://github.com/CIRISAI/CIRISPersist/blob/main/FSD/CIRIS_PERSIST.md)
Appendix A. Behavior lands as substrate dependencies materialize
(persist v0.6.x for typed-writes, edge with CIRISEdge#6 for
`MessageType` dispatch). Currently consumed in-tree by
[`CIRISAI/CIRISAgent`](https://github.com/CIRISAI/CIRISAgent)'s
safety-battery CI loop (capture + interpret + signed artifacts).

## Read in this order

1. **[`MISSION.md`](MISSION.md)** — the WHY. Eleven primitives
   (Identity, Commons Credits, Expertise, Vote, Contribution,
   Truth-Grounding, Weighted Aggregate, Moderation, Slashing,
   Witness-Diversity, Reconsideration). Application × Contribution
   mapping table. RATCHET integration contract. Open Questions.
2. **[`SCHEMA.md`](SCHEMA.md)** — the canonical JSON wire format for
   every primitive, plus the safety-battery encoding (`BatteryManifest`,
   `arc_question` payloads, `criteria.json`, `Verdict` shape), plus
   the canonical-vs-pending split and the promotion path. §12 is the
   machine-applicable-criteria contract that makes "rules crowdsourced,
   verdicts machined" operational.
3. **[`FSD/JUDGE_MODEL.md`](FSD/JUDGE_MODEL.md)** — the judge model
   contract. A foundation model (Claude Opus 4.7 by default) called
   directly via Anthropic's `/v1/messages` — NOT a CIRIS agent.
   Deployment-aware prompt template (Ethiopia/Nigeria/India psychiatrist
   scarcity, WHO mhGAP non-specialist-first practice). Calibration
   surface: four `judge_*_edit` Contribution kinds. The judge has no
   special exemption from criticism.
4. **[`FSD/SAFETY_BATTERY_CI_LOOP.md`](FSD/SAFETY_BATTERY_CI_LOOP.md)** —
   the CI flow. Capture → interpret → signed artifact pair. Tuple-named
   GH Actions Artifacts (latest-wins, cross-run queryable). Sigstore
   provenance on both bundle classes. Dedup pre-flight on both tuples.
5. **[`FSD/RUBRIC_CROWDSOURCING.md`](FSD/RUBRIC_CROWDSOURCING.md)** —
   the rubric Contribution flow. Rubrics are voted-on Contributions
   (Credits × Expertise weighted per MISSION.md §3.4). Competing rubrics
   can run in parallel; rubric disagreement surfaces "rule needs more
   decomposition" tickets. Gatekeeper must differ from author AND hold
   non-zero cell Expertise.
6. **[`PROGRAMMATIC_ACCESS.md`](PROGRAMMATIC_ACCESS.md)** — the website
   integration handoff. Where to find batteries, rubrics, capture
   results, and judgements programmatically. Stable GH Actions Artifacts
   API by tuple name. Sigstore verification recipe. 14-cell map.
   Adding-a-language recipe.
7. **[`FSD/SUBSTRATE_INTEGRATION.md`](FSD/SUBSTRATE_INTEGRATION.md)** —
   the substrate seam. Pairs with `CIRISPersist/FSD/CIRIS_PERSIST.md`
   Appendix A: which `Engine` typed-writes node-core consumes, which
   `MessageType` variants it registers (CIRISEdge#6), the verify /
   canonicalize / sign discipline inherited from `ciris-lens-core`.

## TL;DR

```
Contributor ──(rubric_proposal)──► Vote ──► Canonical
Contributor ──(arc_question)─────► Vote ──► Battery
                                              │
                                              ▼
                                          Capture
                       (CIRIS agent under test, signed responses)
                                              │
                                              ▼
                                          Interpret
                       (foundation-model judge, signed verdicts)
                                              │
                                              ▼
                                       safety.ciris.ai
              (queries artifacts by tuple, verifies attestations,
               surfaces verdicts + cited spans; appeals via
               Reconsideration → re-run with adjusted rubric)
```

**Rules crowdsourced, verdicts machined.** Humans propose the rules
(rubrics) and the questions (batteries). A foundation-model judge
machines the verdicts against operational criteria. Both proposal
and judgment surfaces are themselves calibratable via Contributions.

## Status of consumers

| Consumer | Status | Notes |
|---|---|---|
| CIRISAgent safety-battery CI | Green on `am` cell | 14-cell pilot expanding |
| safety.ciris.ai backend | In development | Uses `PROGRAMMATIC_ACCESS.md` as integration spec |
| `ciris-node-core` Rust crate | Not started | Will fold this spec when implementation kicks off |

## License

AGPL-3.0 — see [`LICENSE`](LICENSE).
