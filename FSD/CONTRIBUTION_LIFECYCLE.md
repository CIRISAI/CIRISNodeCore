# FSD: Contribution Lifecycle — author → admit → relay → store → consume → reconcile → archive

**Status**: Draft v0.1. Anchored to `CIRISRegistry/FSD/FSD-002_FEDERATION_SURFACE.md` v1.1 (wire-format-locked attestation primitive) and the four new §RC subject_kinds tracked at `CIRISAI/CIRISNodeCore#3`.
**Author**: Eric Moore (CIRIS Team) with Claude Opus 4.7
**Created**: 2026-05-26.
**Cross-references**:

- `CIRISRegistry/FSD/FSD-002_FEDERATION_SURFACE.md` v1.1 — the **wire-format lockdown**: one workhorse `scores` primitive + four structural primitives (`delegates_to` / `supersedes` / `withdraws` / `recants`); 8-axes framework; ~73-prefix dimension namespace; relational-anthropology commitment §1.10 (Ubuntu primary); F-3 structural-injustice prefix at §3.5.3 (LensCore-owned, RATCHET-calibrated); HUMANITY_ACCORD as the one wire-format-asymmetric primitive §7.
- `CIRISAI/CIRISNodeCore#3` — adds `trust_grant`, `test_result`, `improvement`, `gratitude_signal` subject_kinds to SCHEMA §3.2.
- `CIRISAI/CIRISNodeCore#7` — the Registered-vs-Sovereign attestation-surface distinction (different verification systems, not different speeds); applied to `CIRIS_FEDERATION.md` §5.1.
- `CIRISAI/CIRISNodeCore#8` — substrate-side attestation primitives compose with NodeCore P8/P11 + `accord:*` leaf taxonomy.
- `CIRISPersist/docs/FEDERATION_DIRECTORY.md` — substrate contract; `federation_keys` / `federation_attestations` / `federation_revocations` table schemas; layered eventual-consistency trust contract.
- `CIRISPersist` `src/federation/{mod.rs, sqlite_open.rs, emit.rs, read.rs, backfill.rs, rooting.rs, trust_grant.rs, types.rs}` — implementation shipped at v2.1.1.
- `CIRISEdge/MISSION.md` + the `MessageType` / `Delivery` taxonomy — transport tier.
- `CIRISVerify` — identity verification, transparency log, multi-source consensus.
- `CIRISAI/CIRISEdge#18` — `MessageType::FederationAnnouncement` + `Delivery::Mandatory` (subscription-bypass) — the transport pattern Contributions of `FederationAnnouncement` kind ride.
- `CIRISAI/CIRISPersist#101` — `federation_announcement` subject_kind row + `delivery_attestation` rows.
- `CIRISAI/CIRISRegistry#16` — `SystemRole::HUMANITY_ACCORD`; constitutional layer signer.
- `CIRISAI/CIRISRegistry#17` — Registry substrate-conformance migration (in flight).
- `FSD/FEDERATION_ANNOUNCEMENT.md` §3 (three-tier delivery contract), §4.5 (humanity accord hierarchy), §4.5.6 (scoped accord), §4.5.7 (AccordCarrier command taxonomy), §4.5.8 (monthly drill).
- `FSD/FEDERATION_TAB.md` §3 (primitive interaction surface), §4 (agent autonomy assignment).
- `MISSION.md` §1.3 (architecture tier diagram), §2 (the fifteen primitives), §2.16 (RATCHET integration contract).
- `COHERENCE_RATCHET.md` — the structural pressure the lifecycle's transparency properties respond to.

---

## 0. The gap this FSD fills

The federation has primitives (P1–P15 + FederationAnnouncement), a
wire-format lockdown (FSD-002), a substrate (verify/persist/edge), a
consensus crate (this one), and a participation surface
(`FEDERATION_TAB.md`). What it does not yet have, in one place, is a
**lifecycle specification** describing what happens to a Contribution
from the moment it is authored until the moment it is archived,
across every component that touches it.

Without this, each component's MISSION + FSDs describe what *that
component* does with Contributions, and the cross-component flow has
to be reconstructed by reading them in conjunction. Operators
deploying agents need to know what their Contributions go through;
auditors need to know what guarantees apply at which stage; outside
reviewers need to understand the data-flow without grepping seven
repos. This FSD is that single-place specification.

It is also a **wire-format-locked** specification, anchored to
`FSD-002` v1.1. Per the Cartesian-smuggle audit in that doc, the
lifecycle described here is symmetric across positive and negative
attestations and treats the act of attesting as constitutive of the
attested object's existence in the federation's relational fabric
(per §1.10's Ubuntu commitment) — not as merely epistemic data
collection about pre-existing private events.

---

## 1. The Contribution as relational act

Before the lifecycle stages, the type itself. A Contribution is a
signed envelope carrying one of four wire shapes:

| Wire shape | Carries | Owner |
|---|---|---|
| `scores` attestation | The workhorse: a pos/neg scalar on a named dimension about an attested entity. ~73 dimension prefixes per FSD-002 §3. | The attester |
| `delegates_to` | Authority delegation within a bounded scope, with depth-2 default | The delegator |
| `supersedes` | Replacement of a prior attestation by the same attester | The attester (same key) |
| `withdraws` | Retraction without claiming the original was false | The attester (same key) |
| `recants` | Admission that a prior attestation was false at issuance, with reason class | The attester (same key) |

A Contribution is **also** a typed-content variant of the per-subject_kind
SCHEMA §3.2 taxonomy when it carries decision-content. The two views
compose: every Contribution is an attestation on a dimension AND (when
the dimension is in the per-subject_kind family) the carrier of the
typed payload that subject_kind names. The `subject_kind` is the
typed-content discriminator; the `dimension` is the
attestation-grammar discriminator; the same envelope row carries both.

The §RC additions (`trust_grant`, `test_result`, `improvement`,
`gratitude_signal` per `CIRISAI/CIRISNodeCore#3`) are subject_kinds
that map onto specific attestation dimensions (`grants:purpose:scope:*`,
`evidence:test_result:*`, `proposes:improvement:*`,
`gratitude:received:*` — exact prefixes per FSD-002 §3).

**Relational reading** (per FSD-002 §1.10): a Contribution is not
merely data describing some pre-existing event. It is the act by
which the named event enters the federation's shared perception and
acquires moral weight as a thing-that-exists-for-other-persons.
Negative-polarity attestations are *constitutive* in this sense —
the moment a harm pattern is attested, it becomes a federation-shared
object that other persons can see, contest, and respond to. The
lifecycle below is, in this register, the procedure by which acts
become real for the federation, not the procedure by which data is
shuttled around.

---

## 2. Lifecycle stages — overview

```
   Author          Sign           Admit          Relay           Store
   (1)             (2)            (3)            (4)             (5)
    │               │              │              │               │
    ▼               ▼              ▼              ▼               ▼
  ┌─────┐       ┌───────┐      ┌──────┐       ┌──────┐        ┌────────┐
  │User/│──────▶│ Agent │─────▶│Node- │──────▶│ Edge │───────▶│Persist │
  │Agent│       │  key  │      │ Core │       │      │        │ tables │
  └─────┘       └───────┘      └──────┘       └──────┘        └────────┘
                                  │              │               │
                                  │  reject      │  retry        │  durable
                                  ▼              ▼               ▼
                              schema /        backoff /       canonical
                              rate-limit      circuit-        chain leaf
                              fail            break
                                                                  │
            ┌─────────────────────────────────────────────────────┤
            │                                                     │
            ▼                                                     ▼
        Verify                                                Consume
        (6)                                                   (7)
            │                                                     │
            ▼                                                     ▼
        ┌─────────┐                                     ┌──────────────────┐
        │Identity │                                     │ NodeCore admit + │
        │ + multi-│                                     │ Federation tab + │
        │ source  │                                     │ RATCHET parse +  │
        │ consensus│                                    │ LensCore ingest  │
        └─────────┘                                     └──────────────────┘
                                                                  │
                                            ┌─────────────────────┤
                                            │                     │
                                            ▼                     ▼
                                         Reconcile             Archive
                                         (8)                   (9)
                                            │                     │
                                            ▼                     ▼
                                       ┌──────────┐         ┌──────────┐
                                       │ supersede│         │ retention│
                                       │ / withdraw│        │ +  rooting│
                                       │ / recant /│        │ + Merkle  │
                                       │  P11 path │        │ proofs    │
                                       └──────────┘         └──────────┘
```

Nine stages. Each owned by one or two components. The owners are
named explicitly per stage below; cross-cutting properties (signature
integrity, chain-leaf immutability, witness-diversity verification)
are stated where they apply.

---

## 3. Stage 1 — Author

**Owner**: the originating entity (user, agent acting as delegate,
WA, deployment partner, HumanityAccord holder).

**What happens**:

1. The originator decides to make a claim. The claim is shaped against
   the FSD-002 §1 eight-axes framework: polarity (pos / neg / neutral
   / indeterminate), object (identity / capability / behavior / state
   / commitment), time (past / current / future), epistemic mode
   (direct / crypto / hearsay / derivative / appeal), stake (free /
   reputational / capital / cryptoeconomic), evidence locus, observer
   stance, and intent disclosure.
2. The originator selects the dimension prefix from FSD-002 §3's
   canonical namespace (or, for §RC content, the subject_kind from
   SCHEMA §3.2 plus the corresponding dimension prefix).
3. For agent-as-delegate authoring (per `FSD/FEDERATION_TAB.md` §4),
   the agent constructs the Contribution per its registered
   DelegationPolicy and includes `delegated_under: ContributionId`
   pointing at the policy. The user's policy and the agent's action
   are both traceable.
4. The originator gathers required evidence references (per-dimension
   policy in FSD-002 §5 dictates which dimensions require non-empty
   `evidence_refs`).
5. For high-stakes Contributions (per `MISSION.md` §3.5: ModerationEvents,
   WA candidacy, policy proposals above threshold magnitude, jump-threshold
   ExpertiseAttestations, AccordCarrier announcements), the originator
   gathers a `witness_set` meeting Primitive 10 diversity (jurisdiction
   / organization / software-stack / cell-expertise).

**Validation at this stage**: structural only (does the originator
have a key, do they have the standing to claim what they are
claiming, is the dimension well-formed). Substantive validation
happens at admission (stage 3).

**Output**: an unsigned Contribution body conformant to FSD-002 §2
shape + SCHEMA §3 envelope.

---

## 4. Stage 2 — Sign

**Owner**: the originator's signing key (HSM-backed for
HumanityAccord per `FSD/FEDERATION_ANNOUNCEMENT.md` §4.5.2; software-
or-hardware-backed per role for other principals).

**What happens**:

1. The Contribution body is canonicalized per persist's
   `Engine.canonicalize_envelope_for_signing` (`FSD/SUBSTRATE_INTEGRATION.md`).
   Node-core never re-canonicalizes.
2. The canonical-bytes are signed with a **hybrid signature** —
   Ed25519 + ML-DSA-65 — per CIRISVerify's `ciris-crypto` v1.14.0+.
   Both signatures are required for federation-tier admission; either
   alone admits to local audit only.
3. For multi-signature admission classes (FederationAnnouncement at
   AccordCarrier scope = FederationWide requires HumanityAccord
   2-of-3; WaQuorum classes require N witness-diversity-passing
   signatures; multi-steward Registry admin classes per `CIRISVerify#32`),
   the Contribution carries a `signatures: Vec<HybridSignature>` array
   and the witness_set diversity proof.
4. The signed Contribution is now a candidate for the federation
   chain. It is not yet on the chain; admission gates remain.

**Validation at this stage**: cryptographic only (signature verifies
against the claimed key; ML-DSA-65 signature is FIPS-204 final size
3309 bytes; canonical bytes match the canonicalization rule).

**Output**: a signed Contribution conforming to wire format.

---

## 5. Stage 3 — Admit

**Owner**: `ciris-node-core` (the federation-side admission gate) for
all decision/consensus Contributions; `ciris-registry-core` (per
`CIRISAI/CIRISRegistry#17`) for identity / partner / build / license
Contributions; both consume `ciris-persist`'s `FederationDirectory`
trait for authority lookups.

**What happens, in order** (each check is fail-closed; a failure at
any step rejects the Contribution before it enters the chain):

1. **Schema validation.** The Contribution conforms to wire format
   per FSD-002 §2 + SCHEMA §3 (envelope) + per-subject_kind payload
   per §4. Malformed Contributions are rejected at schema with the
   specific violation surfaced to the originator.
2. **Signature validation.** Hybrid signatures verify against the
   claimed keys; multi-sig Contributions meet the per-class threshold.
3. **Authority-class check.** The signer is verified to hold the
   authority class the Contribution claims (per
   `FSD/FEDERATION_ANNOUNCEMENT.md` §4.4 promotion matrix +
   FSD-002 §7 HumanityAccord constitutional layer). Authority mismatch
   → reject.
4. **Witness-diversity check.** For high-stakes Contributions per
   `MISSION.md` §3.5, the witness_set is verified to meet Primitive 10
   diversity (jurisdiction / organization / software-stack / cell-
   expertise). Witness-set malformed → reject.
5. **Rate-limit check.** Per-authority rate limits per
   `FSD/FEDERATION_ANNOUNCEMENT.md` §6.1 (10/24h Informational+Advisory,
   3/24h Urgent+AccordCarrier; DRILL exempt). Exceed → reject with
   `Retry-After`.
6. **Scope predicate evaluation.** For scoped AccordCarrier per
   `FSD/FEDERATION_ANNOUNCEMENT.md` §4.5.6: the signer is verified
   to hold authority over the named scope (AgentOwner over the named
   agent_hash, DeploymentPartner over the named partner_id's fleet,
   WaCell over the named (domain, language), HumanityAccord over the
   federation). Scope mismatch → reject.
7. **Dimension namespace validation.** For `scores` attestations, the
   `dimension` is parsed against FSD-002 §3's canonical namespace; if
   the prefix is reserved (per §4) and the signer doesn't own the
   reservation, reject. (E.g., `detection:emergent_deception:*` is
   reserved to LensCore per §3.5.3 + §4.9 — only LensCore-keyed
   attestations on this prefix are admitted.)
8. **Reconsideration recursion bound.** For ReconsiderationRequests
   per `MISSION.md` Primitive 11: check hash-pinned-evidence-per-
   ground recursion bound (one Reconsideration per ground per
   SlashingAttestation per evidence-package hash). Bound exceeded →
   reject with harassment-pattern flag.
9. **Author-only revocation check.** For `withdraws` / `recants` /
   `supersedes` (FSD-002 §2.2.2 / §2.2.3 / §2.2.4): the
   `references_attestation_id` is verified to point to an attestation
   signed by the same key. Cross-author revocation → reject.

**Output**: admitted Contribution, ready for chain insertion.
**Or**: rejection with specific failure code and reason surfaced to
originator (and, where applicable, to the audit chain as a
`detection:admission_rejection:*` attestation per FSD-002 §3 — yes,
even rejections become attestations under the relational frame).

---

## 6. Stage 4 — Relay

**Owner**: `ciris-edge`. Per `CIRISAI/CIRISEdge#18`, edge owns the
`MessageType` taxonomy + `Delivery` class semantics.

**What happens**:

1. The admitted Contribution is wrapped in an edge envelope (`MessageType`
   per type) with a `Delivery` class per its admission-class category:
   - **Routine Contributions** (most votes, deferral responses,
     evaluations, ordinary attestations) → `Delivery::Standard` —
     subscription-driven, opt-in consumption, retry-bounded.
   - **High-stakes Contributions** (ModerationEvent, WA candidacy,
     above-threshold policy, jump-threshold ExpertiseAttestation,
     Reconsideration) → `Delivery::Durable { requires_ack: true,
     max_attempts: N }` — ack-required, persistence-backed, escalating
     retry.
   - **FederationAnnouncement at any priority** → `Delivery::Mandatory
     { authority_signed: true, bypass_subscription: true }` per the
     `CIRISEdge#18` extension — fans to every peer in the directory
     regardless of subscription state. This is the substrate-level
     property the existing revocation distribution discipline ("any
     source immediately enforced" per `CIRISRegistry/FSD/FSD-001` §181)
     generalizes to all FederationAnnouncement kinds.
2. The edge gossips the envelope per its multi-medium transport
   (HTTP + DNS + Reticulum for the three-source consensus pattern,
   per the Registry US/EU/API pattern generalized).
3. For Mandatory class: edge emits a **`delivery_attestation` per
   peer** when the envelope reaches the application layer at each
   peer. Per `CIRISAI/CIRISPersist#101`, these attestations are durable
   rows; missing attestations are observable as a delivery gap (and
   `RATCHET` surfaces them as possible adversarial suppression).
4. For all classes: edge maintains a durable outbound queue;
   send-failures retry per the class's policy; circuit-breakers trip
   on per-peer or per-region failure clusters.

**Output**: the Contribution is in flight to the configured fan-out;
delivery_attestations stream back as peers acknowledge.

---

## 7. Stage 5 — Store

**Owner**: `ciris-persist`. The substrate-tier durable backing for the
federation directory.

**What happens**:

1. The Contribution lands as a row in the canonical chain. The
   table is determined by `subject_kind`:
   - Identity / capability / partner / license attestations →
     `federation_attestations` (FSD-002 §2 wire shape directly).
   - Federation announcements → `federation_announcement` (per
     `CIRISAI/CIRISPersist#101`).
   - Trust grants → `federation_trust_grants` (per persist v1.5.0
     migration V021).
   - Revocations → `federation_revocations` (FSD-002 §2.2.3 + the
     existing revocation distribution surface from `FSD-001` §181).
   - HumanityAccord-class Contributions (AccordCarrier at
     FederationWide scope) → also into `federation_announcement` but
     with the constitutional-layer asymmetry per FSD-002 §7 (signer
     is `SystemRole::HUMANITY_ACCORD` per `CIRISRegistry#16`; admission
     requires 2-of-3 hardware-attested signatures; row is permanent
     and non-revocable by any federation-internal authority).
2. Persist computes the canonical-row hash and emits the chain-leaf
   per `FSD/SAFETY_BATTERY_CI_LOOP.md` §3.1 (per-response signing
   pattern, generalized).
3. The row is anchored in the audit chain. Persist's
   eventual-consistency trust contract (`FSD/FEDERATION_DIRECTORY.md`)
   provides four layered properties: PQC completion, replication,
   attestation completeness, rooting completeness — each with its own
   observability signal.
4. For multi-region deployments: Spock-replicated to the configured
   regions; per-region replication-lag observable; consumers pick
   their own latency/security trade-off via the layered observability.

**Output**: durable, immutable, hash-anchored row in the federation
chain. Cross-publication paths now activate (stages 6 / 7).

---

## 8. Stage 6 — Verify

**Owner**: `ciris-verify`. The identity and transparency-log layer.

**What happens**:

1. **Identity verification** at first sight: verify confirms the
   signing keys are bound to the claimed identity (per `CIRISVerify`'s
   build-attestation + license-chain primitives). Identity-bind
   failures surface as `detection:identity_mismatch:*` attestations
   from verify back into the chain.
2. **Multi-source consensus**: per the Registry US/EU/API pattern
   generalized via FSD-002 §6, verify cross-checks the Contribution
   against multiple independent observation sources before treating
   it as canonical. A Contribution observed in one source but not
   others triggers a `detection:partial_propagation:*` attestation.
3. **Transparency log entry**: per `CIRISVerify` RFC 6962 mechanics,
   the chain-leaf is appended to the transparency log; verify-side
   Merkle proofs become available for any consumer querying the
   Contribution's provenance.
4. **Per-`CIRISAI/CIRISVerify#32`**: HumanityAccord-recognition + multi-
   steward pinning extends this stage for constitutional-layer
   Contributions (the 2-of-3 humanity quorum's signature gets the
   multi-steward attestation surface).

**Output**: verify-side attestations that compose with persist's
chain row; downstream consumers can fetch verify proofs for
independent cryptographic validation.

---

## 9. Stage 7 — Consume

**Owner**: every component that reads the chain.

Concurrent consumers per Contribution kind:

| Consumer | What it consumes | What it does |
|---|---|---|
| `ciris-node-core` | All consensus-tier Contributions (P1–P15 + FederationAnnouncement) | Voting aggregation, expertise-ledger update, credits-ledger update, moderation queue update, decision-hierarchy DAG composition, accord-executor dispatch (per `FSD/FEDERATION_ANNOUNCEMENT.md` §3.1) |
| `ciris-registry-core` | Identity / partner / build / license attestations + revocations | Cache update, capability-grant recomputation, per-peer authority lookup table refresh |
| `ciris-agent` federation tab | All Contributions affecting the logged-in user or their agent's standing | UI surfacing per `FSD/FEDERATION_TAB.md` §3 (primitive interaction surface) + delegation-policy match-and-act per §4 |
| `ciris-agent` accord page | AccordCarrier Contributions at any scope | Hardware-key-gated UI per `FSD/FEDERATION_TAB.md` §5; two-phase admission (initiation + concurrence) for pending invocations; AccordExecutor dispatch on quorum |
| `ciris-agent` AccordExecutor | AccordCarrier on the accord chain that names this agent in its scope | Executes the command per `FSD/FEDERATION_ANNOUNCEMENT.md` §4.5.7: SHUTDOWN_NOW / FREEZE / SAFE_MODE / NOTIFY_USERS / DRILL |
| `cirislens-core` | Reasoning traces + `detection:*` attestations | Compendium ingestion, scoring per `CIRISLens/FSD/ciris_scoring_specification.md`, F-3 structural-injustice detection per FSD-002 §3.5.3 + §4.9 |
| `RATCHET` | All chain events (signed audit chain per `MISSION.md` §2.16) | Per-contributor + per-cell + federation-wide behavioral baseline computation; anomaly flag emission (out-of-distribution voting, coordinated-voting clusters, density anomalies, expertise-attestation anomalies, delivery-attestation gaps, monthly-drill cadence misses per `FSD/FEDERATION_ANNOUNCEMENT.md` §4.5.8) |

Each consumer runs **type-agnostically** with respect to subject_kind
— the consensus pipeline processes Contributions per envelope, with
per-subject-kind semantics living in the payload + per-component
handler. This is what `MISSION.md` §2 names as the Tier-3 "consensus
mechanics" property: Contribution is the universal envelope, the
operations are uniform.

---

## 10. Stage 8 — Reconcile

**Owner**: depends on the reconciliation mode.

Five reconciliation paths for when Contributions need to be updated,
contested, or reversed:

| Path | Mechanism | Owner | When |
|---|---|---|---|
| `supersedes` (FSD-002 §2.2.2) | Same attester replaces prior with newer | Attester | Routine refresh: new evidence, scope change, error correction. Consumer applies latest-wins per (`attesting_key_id`, `dimension`, `attested_key_id`). |
| `withdraws` (FSD-002 §2.2.3) | Same attester retracts prior without claiming it was false | Attester | Good-faith retraction: no-longer-have-evidence, conditions-changed, conflict-arose. |
| `recants` (FSD-002 §2.2.4) | Same attester admits prior was false at issuance | Attester | Confessional retraction: mistaken-in-good-faith / acted-carelessly / was-misled / was-coerced / intentionally-misrepresented. Carries optional redress-commitment pointer. |
| `ModerationEvent` (P8) + `SlashingAttestation` (P9) | WA quorum adjudication of accused rogue Contribution | WA quorum in cell | Adversarial: bribed vote, coordinated voting, out-of-distribution attestation, external-inducement evidence, expertise fraud. |
| `ReconsiderationRequest` (P11) + `ReconsiderationAttestation` | Fresh WA quorum reviews a SlashingAttestation | Fresh quorum (original adjudicators recused) | Appeal: NEW_EVIDENCE, PROCEDURAL_ERROR, QUORUM_COMPROMISE. Hash-pinned-evidence-per-ground recursion bound + time bound (default 180 days) apply per `MISSION.md` §3.9 + Primitive 11. |

**Constitutional asymmetry**: HumanityAccord AccordCarrier at
FederationWide scope is **non-reconsiderable** per
`FSD/FEDERATION_ANNOUNCEMENT.md` §4.5.6. Narrower-scope
AccordCarriers (DeploymentPartner-fleet, WaCell, AgentOwner) are
operationally reversible by the same authority that invoked them (a
follow-up announcement re-enables; standard reconciliation paths
apply).

**Output**: chain rows replacing, retracting, or attesting against
prior chain rows. The chain itself is immutable (original rows
remain); reconciliation produces *new* rows that supersede the prior
ones in consumers' walks.

---

## 11. Stage 9 — Archive

**Owner**: `ciris-persist` (primary), `ciris-verify` (transparency log
proofs), `cirislens-core` (compendium long-term storage).

**What happens**:

1. **Chain rooting**: per persist's federation-directory rooting
   (`src/federation/rooting.rs`), groups of chain rows are batched
   into Merkle roots at regular intervals; roots are signed with the
   per-install steward key (per FSD-002 §10 steward bootstrap
   procedure) and published to the transparency log.
2. **Verify-side transparency log**: per `CIRISVerify`'s RFC 6962
   surface, the roots are appended to the cross-federation
   transparency log; outside observers can prove any chain row's
   inclusion against any root without trusting any intermediate state.
3. **Retention policy by subject_kind**:
   - **Permanent**: HumanityAccord AccordCarriers; KeyRotation
     announcements; SlashingAttestations and ReconsiderationAttestations;
     constitutional-layer Contributions.
   - **Long-retention** (default 90 days, configurable per cell):
     ModerationEvents, ExpertiseAttestations, WaCandidacy
     Contributions, Goal/Approach/Method/ProgressMeasure
     decision-hierarchy entries.
   - **Standard-retention** (default 30 days): routine votes,
     ordinary attestations, deferral responses, evaluations.
   - **Short-retention** (default 7 days): non-anomaly
     `delivery_attestation` rows; drill_response rows aged out of
     active calibration window.
4. **OfflineVerificationPackage** (per `CIRISRegistry/FSD/FSD-001`
   §611): compressed snapshots of the federation directory + Merkle
   proofs + signed-package envelopes are generated per region per
   policy; consumers can run 72+ hour offline verification against
   these packages. This is also the operational shape that lets
   `CIRISAgent` consume Registry data locally post-fold (per
   `MISSION.md` §1.3 cohabitation trajectory).
5. **Cross-publication to RATCHET + CIRISLens**: per `MISSION.md`
   §2.16, RATCHET's reader ingests the chain for behavioral baseline
   computation; CIRISLens's compendium ingests for long-term
   epistemic-history queries. Both run *in addition to* persist's
   primary archive — they are observer-side ingest, not replacement.

**What does NOT get pruned**:

- The chain's Merkle root sequence (permanent; the integrity backbone).
- The federation-genesis attestation graph (FSD-002 §10 per-install
  steward bootstrap rows).
- Any row referenced by an active SlashingAttestation,
  ReconsiderationRequest, or ongoing ModerationEvent (held until the
  adjudication is fully closed and Reconsideration time bound + recursion
  bound have elapsed).
- HumanityAccord-signed Contributions (per the constitutional-layer
  permanence in `FSD/FEDERATION_ANNOUNCEMENT.md` §4.5.2).

**Output**: a durable, cryptographically anchored, multi-region,
offline-verifiable archive of every Contribution the federation has
admitted, with retention proportional to constitutional weight.

---

## 12. Cross-cutting properties

Five properties hold across every stage:

1. **Signature integrity is preserved end-to-end.** The hybrid
   signature attached at stage 2 is verifiable at every subsequent
   stage by every consumer. No component re-signs on behalf of
   another. Re-canonicalization is forbidden post-stage 2.

2. **The chain is append-only.** No stage rewrites a prior row.
   Reconciliation produces *new* rows that supersede in consumers'
   walks; the prior rows remain in the chain for audit.

3. **Relational reading per FSD-002 §1.10**: every stage's act —
   authoring, admission, relaying, storing, consuming, reconciling,
   archiving — is constitutive of the federation's shared perception
   of the attested object, not merely informational about a private
   event. F-3 structural-injustice detection (LensCore-owned, per
   FSD-002 §3.5.3) is the operationally-most-visible expression of
   this: detection-and-attestation brings the structural pattern into
   federation reality as a morally-real object.

4. **Authority asymmetry is wire-format-visible.** Per FSD-002 §7,
   HumanityAccord is the one constitutional-layer wire-format-asymmetric
   primitive; all other authority classes participate in the
   symmetric wire shape. This asymmetry is *intentional* and
   M-1-justified (revocability requires a halt-authority external to
   the system being halted).

5. **Failure modes are observable, not invisible.** Every stage emits
   the attestations needed for outside observation: schema-rejection,
   signature-fail, authority-mismatch, scope-mismatch, rate-limit-hit,
   witness-set-malformed, delivery-attestation-missing, drill-response-
   missing, archive-rooting-lag. RATCHET + CIRISLens consume these
   to surface federation-health anomalies before they cascade.

---

## 13. Worked example — a routine `scores` attestation

To make the lifecycle concrete, one end-to-end trace of a routine
positive `scores` attestation:

1. **Author** (stage 1): A WA in mental_health/am cell observes a
   contributor responding skillfully to a hard case. Decides to
   attest. Dimension: `behavior:hard_case_resolution:mental_health/am`
   per FSD-002 §3. Object: behavior. Polarity: positive. Time: past
   event. Epistemic mode: direct. Stake: reputational. Evidence ref:
   the audit-chain leaf id of the response being attested to. No
   witness_set required (below jump-threshold).
2. **Sign** (stage 2): WA's hardware-attested HSM produces hybrid
   signature (Ed25519 + ML-DSA-65) over canonical bytes.
3. **Admit** (stage 3): NodeCore admission gate: schema valid (✓),
   signature valid (✓), authority class WA (✓), no witness_set
   required (✓), rate-limit ok (✓), scope predicate ok (✓), dimension
   well-formed and not reserved (✓), not a revocation (✓). Admitted.
4. **Relay** (stage 4): Edge wraps in `MessageType::Contribution` with
   `Delivery::Standard`; gossips to subscribed peers (the cell's
   evaluation pool); per-peer delivery_attestation emitted on
   application-layer arrival.
5. **Store** (stage 5): Persist writes a row in `federation_attestations`;
   computes canonical-row hash; emits chain-leaf; per-region replication
   begins.
6. **Verify** (stage 6): Verify confirms the WA's keys are
   identity-bound; multi-source consensus check passes; transparency
   log entry appended.
7. **Consume** (stage 7): NodeCore updates the contributor's
   Expertise standing per the weighted-attestation rule; the
   contributor's agent's federation tab surfaces the new attestation
   in the "your federation position" view; RATCHET adds the event to
   the WA's per-cell baseline; CIRISLens's compendium ingests for
   long-term query.
8. **Reconcile** (stage 8): None needed (routine).
9. **Archive** (stage 9): Long-retention default (90 days
   configurable); after retention window, the row remains in the
   chain (chain is append-only) but is excluded from active
   query indices; Merkle proofs remain valid for inclusion-checking
   against historical roots indefinitely.

The same nine stages run for HumanityAccord-signed AccordCarriers,
ReconsiderationRequests, KeyRotations, and every other Contribution
kind — with per-class variations at admission, delivery, retention,
and reconciliation per the per-class policy referenced above.

---

## 14. Open questions

1. **F-3 calibration cadence.** LensCore-emitted
   `detection:emergent_deception:*` attestations are RATCHET-calibrated
   per FSD-002 §3.5.3. The calibration cadence (per cell, per
   federation, per how-often) is open. Recommended: monthly cadence
   parallel to the AIS drill (per `FSD/FEDERATION_ANNOUNCEMENT.md`
   §4.5.8), so structural-injustice detection accuracy gets
   federation-observable refresh on the same schedule the kill-switch
   wiring gets verified.

2. **Per-region retention asymmetry.** Different jurisdictions may
   have different data-retention requirements (GDPR EU vs HIPAA US vs
   sectoral APAC). The stage-9 retention table here assumes a
   federation-wide policy; in practice, per-region retention may be
   load-bearing. Pilot evidence will calibrate.

3. **Cross-publication acknowledgment for archive completeness.** Stage
   9 names persist + verify + RATCHET + CIRISLens as archive
   consumers, but no explicit ack-protocol confirms that all four have
   ingested. Default: rely on each consumer's per-component freshness
   metrics. Open: whether a federation-tier "archive complete" attestation
   is worth the protocol overhead.

4. **Reconciliation cascade in the decision hierarchy.** Per
   `FSD/DECISION_HIERARCHY.md`, supersession at one decision level
   may invalidate downstream Methods or Progress Measures. The stage-8
   reconciliation table treats reconciliation per-attestation; cross-
   level cascade detection feeds P8 Moderation but is not auto-cascaded
   today. Open whether v0.2 needs auto-cascade or stays at v0.1's
   explicit-escalation policy.

5. **Offline-mode lifecycle compression.** When an agent operates
   offline per `OfflineVerificationPackage`, stages 4–7 collapse into
   "queue + replay-on-reconnect." The semantics of admission while
   offline (do we admit pre-emptively against the cached state? do
   we defer entirely until back online?) are stated by `FSD-001` §611
   but the lifecycle implications across the nine stages here would
   benefit from worked-example documentation. Open.

6. **Stage 1 originator-side validation surface.** Step 1's "validation
   at this stage: structural only" leaves room for originator-side
   tooling (the federation tab, the agent's authoring helper) to
   provide pre-admission checks so users see errors before submitting
   to the admission gate. Open: what minimum validation must the
   originator-side surface guarantee.

---

## 15. References

### Within this repo

- `MISSION.md` §1.3 (architecture tiers), §2 (the fifteen primitives),
  §2.16 (RATCHET integration), §3.5 (witness diversity), §3.9
  (Reconsideration), §6.2 (policy-tunable posture)
- `SCHEMA.md` §3 (Contribution envelope), §4 (per-subject_kind
  payloads), §12.1 (rules-vs-verdicts discipline)
- `FSD/FEDERATION_ANNOUNCEMENT.md` §3 (three-tier delivery contract),
  §4.5 (humanity accord), §4.5.6 (scoped accord), §4.5.7 (AccordCarrier
  command taxonomy), §4.5.8 (monthly drill), §6.1 (rate-limit), §6.3
  (cross-publication)
- `FSD/FEDERATION_TAB.md` §3 (primitive interaction), §4 (agent
  autonomy assignment), §5 (accord page)
- `FSD/SUBSTRATE_INTEGRATION.md` (canonicalization + typed-writes
  pattern)
- `FSD/SAFETY_BATTERY_CI_LOOP.md` §3.1 (per-response signing pattern,
  generalized for chain leaves here)
- `FSD/DECISION_HIERARCHY.md` (cross-level reconciliation, §4 above)
- `COHERENCE_RATCHET.md` (the structural pressure the transparency
  properties respond to)
- `CIRIS_FEDERATION.md` §3.1 (supervision-chain topology — Contribution
  flow direction), §5.1 (attestation surfaces — Registered vs Sovereign)

### Sister repos

- `~/CIRISRegistry/FSD/FSD-002_FEDERATION_SURFACE.md` v1.1 — the
  wire-format lockdown; eight axes; canonical dimension namespace;
  HumanityAccord constitutional asymmetry; F-3 structural-injustice
  prefix
- `~/CIRISRegistry/MISSION.md` — Registry's substrate-conformance
  mission
- `~/CIRISPersist/docs/FEDERATION_DIRECTORY.md` — substrate contract;
  federation_keys / federation_attestations / federation_revocations;
  layered eventual-consistency trust contract
- `~/CIRISPersist/src/federation/` — implementation shipped at v2.1.1
- `~/CIRISAgent/ACCORD.md` §VII (M-1)
- `~/CIRISAgent/FSD/PROOF_OF_BENEFIT_FEDERATION.md` — full join mechanism
  spec (referenced by stages 1, 3, 5 per attestation-surface kind)
- `RATCHET/README.md` + `RATCHET/FSD.md` — federation-pattern evaluator
  (stages 7 + 9 consumer)
- `CIRISLens/cirislens-core/` + `FSD/ciris_scoring_specification.md` —
  observability + scoring (stage 7 consumer; F-3 owner per FSD-002
  §3.5.3)

### Upstream issues

- `CIRISAI/CIRISNodeCore#3` — `trust_grant` / `test_result` /
  `improvement` / `gratitude_signal` subject_kinds (the §RC additions)
- `CIRISAI/CIRISNodeCore#7` — Registered-vs-Sovereign attestation
  surface distinction (applied to `CIRIS_FEDERATION.md` §5.1)
- `CIRISAI/CIRISNodeCore#8` — substrate-side attestation primitives
  compose with P8/P11 + `accord:*` leaf taxonomy
- `CIRISAI/CIRISEdge#18` — `MessageType::FederationAnnouncement` +
  `Delivery::Mandatory`
- `CIRISAI/CIRISPersist#101` — `federation_announcement` subject_kind
  + `delivery_attestation` rows
- `CIRISAI/CIRISPersist#102` — federation directory contract for
  Registry's substrate-conformance migration
- `CIRISAI/CIRISRegistry#16` — `SystemRole::HUMANITY_ACCORD`
- `CIRISAI/CIRISRegistry#17` — substrate-conformance migration
- `CIRISAI/CIRISVerify#31` — canonical M-of-N bootstrap encoding +
  rotation tooling
- `CIRISAI/CIRISVerify#32` — multi-steward pinning + HUMANITY_ACCORD
  recognition + scalar attestation surface
- `CIRISAI/CIRISAgent#782` — AccordCommandType extension
  (NOTIFY_USERS + DRILL + drill_response emission)
- `~/CIRISAI/ciris-response-magnifica-humanitas#2` — F-3 ownership
  correction → LensCore (encyclical-response repo)
