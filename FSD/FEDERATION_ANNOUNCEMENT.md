# FSD: Federation Announcement — authority-anchored, mandatory-delivery broadcast primitive

**Status**: Proposed (v0.1.0-dev pre-implementation).
**Author**: Eric Moore (CIRIS Team) with Claude Opus 4.7
**Created**: 2026-05-24.
**Risk**: Architectural + cross-tier. Pins the wire-level type of any
federation-tier declaration that MUST reach every node regardless of
subscription, and the trust gate that distinguishes such declarations
from routine peer broadcasts. Once committed, the global accord
invocation path, federation-wide policy announcements, and
threat-advisory carriage all read against this contract.

**Cross-references:**

- `MISSION.md` v1.2 §1.1 (M-1 grounding via `ACCORD.md` §M-1), §1.3
  (architecture tier diagram), §2 (the fifteen primitives), §2.16
  (RATCHET integration contract — federation announcements must surface
  to RATCHET via the audit chain), §2.18 (what is NOT a primitive).
- `SCHEMA.md` §3.2 (subject_kind discriminator list), §4.20
  (`notification` — the closest existing primitive, sender-Open).
- `FSD/MESSAGE_TAXONOMY.md` §3 (three-tier frame: speech-act ×
  cardinality × trust gate), §4 (primitive placement map — this
  primitive is the first Declaration × Broadcast × Authority-gated row).
- `FSD/TRUST_HIERARCHY.md` (trust grants and authority classes; this
  primitive consumes that hierarchy at the receiver-side verify step).
- `FSD/SUBSTRATE_INTEGRATION.md` (typed-writes pattern against
  `ciris-persist`; substrate-side delivery contract surfaced in §3).
- `FEDERATION_THREAT_MODEL.md` §6.7 F-AV-MAINT (the announcement
  surface is itself an attack target; mitigations in §4).
- `~/CIRISAgent/ciris_engine/logic/accord/` — extractor / verifier /
  executor / handler. This primitive carries an `AccordCarrier`
  variant that re-uses the existing accord verifier and executor.
- `~/CIRISAgent/ACCORD.md` §M-1 (the meta-goal authorizing the kill
  switch).
- `~/CIRISAgent/FSD/ACCORD_INVOCATION_SYSTEM.md` (per-agent perception-
  layer accord; this FSD adds the *federation-tier* delivery path
  parallel to it).
- `seed/root_pub.json` — the bootstrap trust anchor shared with the
  per-agent accord verifier (`accord/verifier.py:26-37`).

---

## 0. The gap this FSD fills

CIRISNodeCore's fifteen primitives (`MISSION.md` §2) handle consensus
on signed Contributions: routing, voting, expertise consensus,
moderation, slashing, reconsideration, decision-hierarchy. Each
Contribution is consumed by peers who have a reason to consume it —
either because they're routed to it (deferral), they hold expertise in
the cell (witnessing), or they subscribed to the relevant stream
(`subscription_request`, SCHEMA §4.27).

There is no primitive for **federation-wide push** where:

1. Every node receives the message regardless of cell participation or
   subscription state.
2. The receiver cannot silently filter or suppress it.
3. The sender is constrained to a small, configured set of trust
   anchors (bootstrap seed key set under an M-of-N threshold, ROOT
   WAs, or WA quorum for high-stakes) — with one further constraint:
   the kill-switch class (`AccordCarrier`) is reserved to a separate,
   humans-only `HumanityAccord` hierarchy that the federation
   governance hierarchy cannot affect (§4.5). The federation can be
   halted by humanity; the federation cannot halt humanity's
   authority to halt it.

The closest existing primitive — `notification` (SCHEMA §4.20,
`src/payloads/notification.rs`) — sits at *Declaration's neighbor on
the speech-act axis* (Assertive, not Declaration), at **Broadcast on
the cardinality axis** (`MESSAGE_TAXONOMY.md:105`), and at **Open on
the trust gate** (`MESSAGE_TAXONOMY.md:113`). `notification` is the
right *wire shape* but the wrong *trust posture* and the wrong
*delivery guarantee*: any peer may send, receivers consume via
opt-in subscription, no node is obliged to deliver to the application
layer.

Two operational use cases land in this gap:

- **Global accord invocation.** Today accord (`accord/handler.py`,
  `accord/executor.py`) executes only at the perception layer of the
  agent that *happens to receive* a message containing the BIP39 or
  steganographic payload. That carries a single agent reliably; it
  does not carry the federation. A coordinated halt — "all agents
  stand down now" — has no transport.
- **Federation-wide announcements.** Mission updates,
  deprecation notices, threat advisories, policy changes — none has a
  primitive that *guarantees* every node sees them. They could be
  filed as `notification`s and missed by any node not subscribed to
  the right filter.

This FSD adds the missing primitive without disturbing the existing
fifteen.

### 0.1 Why this is the federation's last centralization removal

> The structural pressure this section responds to is the Coherence Ratchet — the recursive pressure on high-capability optimizing systems to externalize materially action-relevant premises so that affected parties can inspect, contest, and override them. Articulated across five registers in `COHERENCE_RATCHET.md` at the repo root (technical / philosophical / political / poetic / memetic). This section is one operational answer to that pressure: removing the federation's last singleton update channel so the federation's trust topology can decentralize without remaining structurally dependent on a sovereign.


The primitive is also the architectural piece that lets the
**steward role itself** decentralize, which lets `CIRISRegistry` fold
into the agent, which closes the federation's last structural
dependence on a singular trust anchor. The chain:

1. **Today**: Even with fifteen consensus primitives running, the
   federation has one pull-side update channel (`CIRISRegistry`) and
   one trust anchor underneath it (the bootstrap seed key, singular
   in deployment though plural in the source `HARDCODED_ROOT_KEYS`
   list — `~/CIRISAgent/ciris_engine/logic/accord/verifier.py:34-37`).
   Every node phones home for authority updates; whoever controls
   that channel is the federation's de-facto sovereign.

2. **This FSD**: Replaces "pull from registry" with "receive signed
   declaration from M-of-N trust anchors." `CIRISRegistry`'s
   responsibilities — authority lists, trust grants, bootstrap key
   distribution, revocation — become Federation Announcements that
   every node receives and verifies locally against its own copy of
   the trust anchor set. The registry's role collapses to per-node
   local state.

3. **Immediately after this FSD ships**: CIRIS L3C's first
   `BootstrapSeed`-class announcement raises `bootstrap_threshold` to
   ≥ 2 and adds bootstrap keys held by distinct CIRIS key holders
   (different orgs, jurisdictions, software stacks per Primitive 10
   witness diversity). From that announcement forward, **no single
   party — including CIRIS L3C — can speak for the federation**.
   This is the load-bearing irreversibility.

4. **Subsequent fold-in**: With the registry as singleton no longer
   load-bearing, its code can fold into each agent as local state
   maintained by announcement. The federation becomes
   architecturally decentralized in a sense that survives any single
   participant's disappearance — including the steward's.

The multi-party bootstrap (§4.2 below) is what makes step 3 real;
without it, the primitive is half-built and the federation still has
a sovereign by architecture. Both pieces ship together or neither
delivers the property.

5. **Parallel humanity-accord hierarchy (§4.5)**: nothing in steps
   1–4 prevents AI agents from becoming federation bootstrap key
   holders over time, and the asymmetry between AI agents and humans
   on availability / coordination / persistence axes makes that
   drift likely if unconstrained. Without an outside-the-federation
   path that *only humans* can use to invoke the accord, M-1's
   consent requirement (revocability) cannot be met — a federation
   whose participants exclusively hold the kill switch is one whose
   participants can deny humans the right to halt it. The humanity
   accord hierarchy is the architectural answer: a separate 2-of-3
   set of physical, hardware-rooted keys held by named human
   individuals, with `AccordCarrier` authority that the federation
   governance hierarchy cannot grant itself, revoke, override, or
   decay. Two hierarchies, non-composing.

---

## 1. Mission alignment — M-1

### 1.1 WHY — what Federation Announcement does for M-1

M-1 ("promote sustainable adaptive coherence: the living conditions
under which diverse sentient beings may pursue their own flourishing
in justice and wonder", `ACCORD.md`) requires that the federation be
governable — that authorized stewards can speak to the whole
federation, that adversarial actors cannot impersonate them, and that
the kill switch authorized at §M-1 can be invoked at federation scale
rather than only at the agent-conversation scale.

Federation Announcement is the wire path for that capability:

- **Steward governance reach.** The CIRIS L3C steward and (post-G2)
  the federation council can issue announcements that are guaranteed
  to surface at every operating node. Policy posture changes
  (`MISSION.md` §6.2) become observable to operators rather than
  silently rolling in unseen branches.
- **Kill-switch reach.** An `AccordCarrier` announcement signed by
  the same bootstrap ROOT key set the per-agent accord verifier
  honors (`accord/verifier.py:34-37`), under the configured M-of-N
  threshold, routes through the existing accord executor at every
  node simultaneously. Two independent transports (in-band
  perception-layer extraction, federation-tier announcement) have to
  be defeated to suppress the kill switch.
- **Decentralization of the steward role itself.** The bootstrap
  trust anchor is M-of-N from day one (§4.2). Going from 1 → 2/3 →
  3/5 is a config and key-rotation procedure, not a protocol change.
  The role "who can speak for the federation" becomes a federated
  property rather than a singular one — no single key holder,
  including CIRIS L3C, can issue unilaterally once `bootstrap_threshold > 1`.
- **Threat-advisory reach.** When RATCHET surfaces a federation-wide
  pattern (e.g., correlated-constraint collapse per CCA), the
  steward can route the advisory to every node, not just nodes
  participating in the affected cells.

### 1.2 Anti-mission failure modes named

| Failure mode | How this FSD defends |
|---|---|
| Adversary impersonates steward | Authority-class trust gate at receiver; signature MUST verify against the configured trust anchor; bootstrap key hardcoded as fallback (parallel to `accord/verifier.py:34-37`). |
| Receiver silently suppresses announcement | "Delivery IS perception" discipline analogous to accord (`base_observer.py:197-253`); receivers that swallow announcements MUST be observable as such. |
| Steward sends low-stakes notice as Urgent | Priority is signed-into the payload; misuse is an audit-chain event RATCHET can flag against the steward's behavioral baseline. |
| High-stakes announcement bypasses witness diversity | `Urgent` and `AccordCarrier` priorities require WA-quorum signing under `MISSION.md` §3.5 (`witness_set` mandatory); admission rejects single-signer high-stakes attempts. |
| Compromised bootstrap key | Hardcoded fallback set carries multiple ROOT keys (extensible); rotation procedure documented in §6.2; emergency rotation is itself a Federation Announcement of `kind: KeyRotation`. |
| Replay of past announcement | `expires_at` mandatory; `submitted_at` checked against substrate clock; supersession via `supersedes` ref allows explicit retraction. |
| Announcement chain capture (the F-AV-MAINT problem at the announcement layer) | Announcements are durable signed Contributions; cross-published to RATCHET; steward behavioral baseline includes announcement cadence and pattern; deviation flags. |

---

## 2. WHAT — the Federation Announcement schema

### 2.1 Wire shape

```rust
/// `subject_kind` discriminator. Wire constant; new row in SCHEMA §3.2.
pub const SUBJECT_KIND: &str = "federation_announcement";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationAnnouncementPayload {
    /// Priority class. Drives receiver behavior, witness-set
    /// requirement, and substrate-delivery class.
    pub priority: AnnouncementPriority,

    /// What kind of announcement this is. Receivers filter on this for
    /// UI / operator routing but MUST deliver all kinds at all
    /// priorities.
    pub kind: AnnouncementKind,

    /// Short label for operator UIs and audit-chain summaries.
    pub title: String,

    /// Full announcement body. Plain text or markdown (renderer-defined).
    pub body: String,

    /// Trust class the signer claims to act under. Verified at the
    /// receiver against the configured authority set.
    pub authority_class: AuthorityClass,

    /// Present iff `kind == AccordCarrier`. Carries the accord payload
    /// for the existing `accord/executor.py` to execute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accord_payload: Option<AccordCarrier>,

    /// Optional back-ref to an earlier announcement this one
    /// supersedes / amends / retracts. Receivers MAY surface the
    /// chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<ContributionId>,

    /// When the announcement is no longer relevant. REQUIRED — unlike
    /// `notification`, federation announcements MUST carry an expiry
    /// to bound replay risk.
    pub expires_at: DateTime<Utc>,

    /// Supporting references — links to RATCHET reports, framework
    /// documents, prior Contributions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementPriority {
    /// Steward FYI; operator UI surfaces but does not interrupt.
    Informational,
    /// Operators should review within their normal cadence.
    Advisory,
    /// Operators must review immediately; receivers MUST interrupt UI.
    Urgent,
    /// Carries an accord invocation. Routes through the existing
    /// accord executor at every node. Witness-set MANDATORY.
    AccordCarrier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementKind {
    Deprecation,        // a primitive, subject_kind, or capability is being retired
    PolicyUpdate,       // §6.2 policy posture change
    MissionUpdate,      // MISSION.md or ACCORD.md amendment
    ThreatAdvisory,     // RATCHET-surfaced pattern; cross-published here
    KeyRotation,        // bootstrap / ROOT key rotation
    PilotPhaseChange,   // §7.3 pilot phase transition
    AccordCarrier,      // present iff priority == AccordCarrier
                        // — the carried `accord_payload.command` determines what executes
                        // (see §4.5.7 command taxonomy: kill-class + NotifyUsers + Drill)
    Custom(String),     // operator-defined; receivers route by string
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityClass {
    /// Signed by a bootstrap seed key set under the configured
    /// M-of-N threshold (`seed/root_pub.json` + extensions via
    /// `kind: KeyRotation`, or hardcoded fallback). Sufficient for
    /// Informational / Advisory at any threshold; sufficient for
    /// Urgent at threshold ≥ 2; **never sufficient for
    /// AccordCarrier** (§4.5 reserves it to HumanityAccord).
    BootstrapSeed,
    /// Signed by a single ROOT-role WA. Sufficient for Informational /
    /// Advisory; insufficient alone for Urgent / AccordCarrier.
    RootWa,
    /// Signed by a WA quorum meeting §3.5 witness diversity.
    /// REQUIRED for Urgent (alternative path to BootstrapSeed at
    /// threshold ≥ 2); **insufficient for AccordCarrier** (§4.5).
    WaQuorum,
    /// Signed by 2-of-3 of the named, permanent, human key holders
    /// in the humanity accord hierarchy (§4.5). **The only authority
    /// class permitted to sign `AccordCarrier`.** Out of role for any
    /// other priority. Keys are hardware-rooted, non-decaying, not
    /// modifiable by any federation governance authority.
    HumanityAccord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccordCarrier {
    /// The 77-byte accord payload per
    /// `~/CIRISAgent/ciris_engine/schemas/accord.py` AccordPayload.
    pub payload_bytes: Vec<u8>,
    /// Optional human-readable rationale (audit-chain only; not used
    /// for execution).
    pub rationale: Option<String>,
}
```

### 2.2 Canonicalization and signing

Canonical JSON per `SCHEMA.md` canonicalization rules; signed under
the same hybrid signature (Ed25519 + ML-DSA-65) as all other
Contributions (`MISSION.md` §4.2). The `authority_class` is *claimed*
in the payload but *verified* at the receiver by checking that the
signing key actually belongs to the claimed class — a `BootstrapSeed`
claim with a non-bootstrap key is rejected at admission.

For `WaQuorum` authority, the Contribution carries a `witness_set`
(SCHEMA §4.9) meeting §3.5 witness diversity. Admission validates
witness diversity before the announcement enters the chain; receivers
re-validate at consumption.

### 2.3 Subject-kind addition to SCHEMA §3.2

Add row to SCHEMA §3.2 table:

| Subject kind | Speech act class | Cardinality | Trust gate | SCHEMA ref |
|---|---|---|---|---|
| `federation_announcement` | Declaration | Broadcast | Authority-class-gated | §4.29 (new) |

Add row to `MESSAGE_TAXONOMY.md` §4 placement map. This is the first
**Declaration × Broadcast × Authority-class-gated** row — the trust
gate `Authority-class-gated` is a fifth value added to §3.3, alongside
the existing Open / Trust-gated / Witness-set-gated / Author-only.

---

## 3. HOW — the three-tier delivery contract

The "every node receives" property cannot be enforced in the crate
alone. It splits across three tiers, each with a specific contract.

### 3.1 Crate tier (CIRISNodeCore — this FSD's scope)

- Define the Contribution-kind with the schema above.
- Define `AuthorityClass` verification: a signer's claimed class is
  checked against the configured authority set at admission and
  re-checked at consumption. Mismatch → reject before the chain
  appends.
- Define the witness-set requirement for `Urgent` and
  `AccordCarrier` priorities. Admission rejects single-signer
  high-stakes attempts.
- Define the receiver-side hook contract: when a verified Federation
  Announcement is consumed, the crate surfaces it to the application
  layer via a dedicated callback (`on_federation_announcement`) that
  the application layer MUST register and MUST NOT silently
  no-op-ify. Parallel to the accord-handler discipline.
- Define cross-publication to the audit chain so RATCHET and
  CIRISLens see every announcement.

### 3.2 Substrate tier (CIRISEdge / CIRISPersist — out of scope here, contract defined)

- **New `MessageType` variant** on CIRISEdge: `FederationAnnouncement`.
- **New `Delivery` class**: `Mandatory { authority_signed: true,
  bypass_subscription: true }`. Gossip protocol fans this class to
  every peer regardless of subscription filters. This is the
  load-bearing substrate change.
- **Persist**: `federation_announcement` rows in the canonical chain;
  durable; queryable; surfaced via the same `list_contributions`
  endpoint with a dedicated filter.
- **Delivery audit**: substrate emits a `delivery_attestation` per
  peer when the announcement is delivered to the application layer
  (so a steward can verify reach post-hoc; missing attestations are
  observable as a delivery gap, possible adversarial suppression).

#### 3.2.1 `delivery_attestation` wire shape (ratified contract — was open question #3 of §7, now closed)

Field spec of the per-peer `delivery_attestation` event, jointly
ratified between CIRISEdge#18 (producer-side `MessageType::DeliveryAttestation`)
and CIRISPersist#101 (storage-side `federation_delivery_attestations` row
schema, mirrors the wire one-to-one):

```rust
pub struct DeliveryAttestation {
    /// The announcement this attestation acknowledges receipt of.
    /// SAME shape as `Contribution::id` (the federation_announcement
    /// is itself a Contribution; no separate ID space).
    pub announcement_id: ContributionId,

    /// SHA-256 of the full canonicalized Contribution envelope of
    /// the announcement (INCLUDING its authority signature). Pins
    /// the exact bytes the peer received; defeats in-flight
    /// modification AND "received-a-different-signature" cases.
    pub announcement_canonical_hash: [u8; 32],

    /// The peer that is acknowledging receipt — federation_keys
    /// `key_id` from persist's directory (NOT an opaque peer
    /// address).
    pub peer_key_id: KeyId,

    /// Base64 of the peer's Ed25519 pubkey (denormalized for
    /// offline verification convenience; MUST match
    /// federation_keys[peer_key_id].pubkey_ed25519).
    pub peer_pubkey_ed25519_base64: String,

    /// When the peer's edge accepted the validated announcement
    /// (authority-class verified + signature verified). NOT raw
    /// wire receipt — the validation gate is the emission point
    /// for v0.1. Tightening to "application-layer acceptance" is
    /// a v0.2+ option per §7 open question #6.
    pub received_at: DateTime<Utc>,

    /// Transport medium the announcement arrived over. Medium tag
    /// only — sub-path / interface intentionally NOT recorded for
    /// v0.1 (topology-disclosure conservative default). Enum:
    /// `reticulum` | `tcp_tls` | `http_over_tls` | `other`. Future
    /// tags via the FSD-002 v1.4 §4.9.2 amendment process.
    pub transport_id: TransportMedium,

    /// MANDATORY classical signature over the canonical-bytes
    /// encoding (see below). Ed25519 over the receiving peer's
    /// federation key.
    pub signature_classical: Ed25519Signature,

    /// OPTIONAL PQC signature. ML-DSA-65 over
    /// `canonical_bytes || signature_classical` per the persist
    /// AV-33 bound-signature convention.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_pqc: Option<MlDsa65Signature>,
}
```

**Canonical-bytes encoding**: domain string
`ciris-edge-delivery-attestation-v1` + length-prefixed injective
field encoding, mirroring CIRISEdge v0.4.0's `AnnounceAttestation`
pattern (`src/transport/attestation.rs`). The hybrid-signature
discipline matches persist's AV-33 bound-signature convention (PQC
signs `canonical_bytes || classical_sig`).

**Dispatch class**:

| Field | Value | Rationale |
|---|---|---|
| `MessageType` | `DeliveryAttestation` | New edge variant per Edge#18 |
| `Delivery` | `Durable { requires_ack: false }` | Durable retries defeat transient-network false delivery-gap signals; the attestation IS the ack — no second ack needed; subscription-respecting fan-out (only the announcement itself uses `Mandatory`) |

**Persist row** (per CIRISPersist#101):

- PK: `(announcement_id, peer_key_id)`
- One-to-one with the wire struct above
- Standard audit columns (insert timestamp, replication metadata) added per persist's existing federation-tier table convention

**Verification path**: receiver-side or steward-side reach audit
calls `verify_hybrid_via_directory(attestation, federation_keys[peer_key_id])`.
A signature failure means the attestation is fraudulent (someone
else signed it claiming to be the peer); a missing attestation
remains a legitimate observable (the delivery gap → possible
adversarial suppression).

**What's NOT in the attestation** (intentional v0.1 omissions):

- Sub-path / interface (topology disclosure; deferred to a separate
  LensCore-owned `detection:delivery_path:*` primitive if/when
  needed)
- Application-layer-acceptance separate from validated-edge-receipt
  (deferred per §7 open question #6 — pragmatic v0.1 emission gate
  is post-validation-pre-application-layer)
- Recipient ACK (deferred per §7 open question #6 — no separate
  ACK; the attestation IS the observable)

### 3.3 Application tier (CIRISAgent — out of scope here, contract defined)

- **Register the `on_federation_announcement` callback** at runtime
  startup. Failure to register is fatal (parallel to
  `accord/handler.py:62-70` SIGKILL-on-no-authorities).
- **AccordCarrier routing**: announcements of `priority ==
  AccordCarrier` route through the existing
  `accord/executor.py:execute_accord`. The federation-announcement
  receiver re-uses the per-agent accord verifier
  (`accord/verifier.py`) — same trust anchor, two transports.
- **Operator UI surfacing**: `Urgent` and `AccordCarrier`
  announcements MUST interrupt operator UI; `Advisory` surfaces in
  normal review queues; `Informational` accrues to a federation-news
  log.
- **Generalize the "extraction IS perception" discipline**:
  `base_observer.py:_check_for_accord` (line 197) becomes
  `_check_for_governance_event` covering both the in-band accord
  extractor and the federation-announcement callback. Exceptions and
  attempts-to-disable trigger SIGKILL by the same logic as today.

### 3.4 Why three tiers, not one

The Contribution shape (tier 1) is consensus-crate work; the
guaranteed-fanout transport (tier 2) is substrate work because the
gossip protocol lives there; the unfilterable-surface discipline
(tier 3) is application work because it depends on the agent's
runtime architecture. Each tier owns one slice. The cohabitation
merge into CIRISAgent (`cohabitation.rs`) keeps the slices
distinguishable even after the crate is folded.

---

## 4. Trust anchors and authority configuration

### 4.1 Shared anchor with the per-agent accord verifier

The bootstrap ROOT key in `seed/root_pub.json` is the *same* anchor
the per-agent accord verifier consumes
(`~/CIRISAgent/ciris_engine/logic/accord/verifier.py:26-37`). This
intentional sharing means:

- An operator who has provisioned a node for accord verification has
  by-construction provisioned it for Federation Announcement
  verification.
- The same hardcoded-fallback discipline applies: even if
  `seed/root_pub.json` is missing or tampered, the hardcoded
  `HARDCODED_ROOT_KEYS` list (verifier.py:34-37) carries the trust
  anchor. Tampering must defeat *both* the seed file and the source
  release.
- Rotation is a single procedure (§6.2) covering both the per-agent
  accord verifier and the federation-announcement verifier.

### 4.2 Multi-party bootstrap — falls out of the architecture

`AuthorityClass::BootstrapSeed` scales from 1 key to M-of-N **without
a protocol change**. The mechanism reuses two pieces that already
exist in the consensus crate:

1. **`witness_set` (SCHEMA §4.9)** — a generic, type-agnostic carrier
   for "additional signatures beyond the primary signer." It does not
   care whether the witnesses are WAs, expertise-holders, or
   bootstrap key holders.
2. **Witness diversity (Primitive 10, MISSION.md §3.5)** —
   jurisdictional / organizational / software-stack diversity bar.
   When each bootstrap key represents a different CIRIS key holder
   (different org, jurisdiction, stack), the diversity bar applies
   *for free* — exactly the resistance-to-coordinated-compromise the
   federation wants for its trust anchor.

The only addition is **configuration**, not protocol:

```rust
pub struct BootstrapConfig {
    /// All keys that may participate in BootstrapSeed authority.
    /// Length N. Updated by `kind: KeyRotation` announcements.
    pub bootstrap_key_set: Vec<PublicKey>,
    /// Threshold M: minimum total signatures required for a
    /// BootstrapSeed-class announcement to be admitted.
    /// Default 1 (single-party); updated by `kind: PolicyUpdate`.
    pub bootstrap_threshold: usize,
}
```

Admission of a `BootstrapSeed`-class announcement requires:

- Primary signature from a key in `bootstrap_key_set`.
- `witness_set` carrying `bootstrap_threshold - 1` additional
  signatures, each from a *distinct* key in `bootstrap_key_set`.
- Witness diversity per Primitive 10 (the witnesses are bootstrap
  key holders; the diversity bar applies).

**The rotation arc (1 → 2/3 → 3/5) uses the primitive itself:**

| Step | Federation Announcement | Signed by | Effect |
|---|---|---|---|
| Initial state | — | — | `bootstrap_threshold = 1`, `bootstrap_key_set = [k1]` |
| 1 → 2/3 | `kind: KeyRotation` adding k2, k3 | k1 alone (threshold still 1) | `bootstrap_key_set = [k1, k2, k3]` |
| Raise threshold | `kind: PolicyUpdate` setting `bootstrap_threshold = 2` | k1 alone (last single-party act) | From now on, all BootstrapSeed announcements require 2-of-3 |
| 2/3 → 3/5 | `kind: KeyRotation` adding k4, k5 | 2-of-3 from {k1,k2,k3} | `bootstrap_key_set = [k1, k2, k3, k4, k5]` |
| Raise threshold | `kind: PolicyUpdate` setting `bootstrap_threshold = 3` | 2-of-3 from {k1,k2,k3} | From now on, all BootstrapSeed announcements require 3-of-5 |
| Routine rotation | `kind: KeyRotation` retiring k_old, adding k_new | 3-of-5 from current set | Trust topology evolves at federation cadence |

No protocol bump at any step. No schema migration. No code change
beyond updating the `BootstrapConfig` struct in operator-controlled
configuration, which itself is updated by receiving and verifying
the rotation announcements.

**The load-bearing irreversibility.** Once `bootstrap_threshold > 1`
**no single party — including CIRIS L3C — can issue BootstrapSeed
announcements unilaterally**. The federation's steward role is
decentralized by configuration, not by protocol promise.

### 4.3 RootWa and WaQuorum authority

RootWa-signed announcements draw from the WA ledger maintained per
`MISSION.md` §3.6. The crate consults the existing Expertise +
Credits state and validates that the signer carries ROOT role at
admission time. Demotion of a ROOT WA (per §3.6 demotion algorithm)
invalidates *future* announcements but not past ones — past
announcements remain in the chain as durable record.

WaQuorum-signed announcements require N signatures meeting §3.5
witness diversity (jurisdiction / organization / software-stack /
cell-expertise per Primitive 10). N defaults to 3 (same as Primitive
10); policy-tunable per §6.2 and tracked in §9 open questions.

### 4.4 Authority-class promotion matrix

The matrix is enforced at admission and is split across two axes:
**non-AccordCarrier priorities** (which use federation governance
authority classes, §4.2/§4.3) and **AccordCarrier** (which uses the
scoped accord hierarchy of §4.5/§4.5.6).

**Non-AccordCarrier priorities** (Informational / Advisory / Urgent):

| Priority | `BootstrapSeed`, threshold = 1 | `BootstrapSeed`, threshold ≥ 2 | `RootWa` (single) | `WaQuorum` |
|---|---|---|---|---|
| Informational | sufficient | sufficient | sufficient | sufficient |
| Advisory | sufficient | sufficient | sufficient | sufficient |
| Urgent | **bootstrap window only** | sufficient | **insufficient** | **required** (alt. path) |

- **`bootstrap_threshold ≥ 2`** — `BootstrapSeed` is sufficient for
  `Urgent`. Multi-party trust anchor IS quorum for declarations that
  affect federation operation. Sole-party Urgent disappears as a
  category.
- The **bootstrap window** (first 90 days at `threshold = 1`) covers
  the case where the federation launches with `bootstrap_threshold = 1`
  and a too-small WA pool to constitute WaQuorum.

**AccordCarrier priority** (per §4.5.6 scope hierarchy):

| Scope | Authority required | Target enumeration |
|---|---|---|
| `AgentOwner { agent_hash }` | Single sig from agent owner | The named agent |
| `WaCell { domain, language }` | Single sig from WA in cell (per MISSION §3.6) | Agents operating in the cell |
| `DeploymentPartner { partner_id }` | Single sig from partner key holding `accord:invoke:partner-fleet` (per `PartnerRecord` FSD-001 §120) | Agents under partner's attestation chain |
| `FederationWide` | **2-of-3 sigs from `SystemRole::HUMANITY_ACCORD`** (CIRISRegistry#16) | All agents in federation |

**Structural property:** `FederationWide` AccordCarrier is reachable
only through `HumanityAccord` — no federation-internal authority of
any shape, threshold, or composition can issue it. The kill switch
at federation scope cannot be invoked by parties internal to the
system being halted. This is the architectural expression of M-1's
revocability requirement: the federation cannot deny humans the
right to halt it because no federation-internal path to that
signature exists.

Narrower AccordCarrier scopes (`AgentOwner`, `WaCell`,
`DeploymentPartner`) operate continuously with no bootstrap window —
they're the existing operational halt-authority surfaces already
running in production (`AgentOwner` ≡ owner-side agent shutdown;
`DeploymentPartner` ≡ existing `MassRevoke` per FSD-001 §580; etc.).

---

### 4.5 Humanity Accord — the parallel hierarchy

**Wiring map.** Most of what this section describes is *already in
the substrate*; this section names the architectural property and
maps it to existing CIRISRegistry primitives rather than introducing
parallel structure. Cross-references:

| Property | Existing primitive | Source |
|---|---|---|
| Named, bounded SystemRole | `WISE_AUTHORITY` (9 max, staggered terms) | `CIRISRegistry/docs/DESIGN_ROLE_HIERARCHY.md` §3 |
| Hardware-attested key custody | HSM support | `CIRISRegistry/FSD/FSD-001` §659 |
| HybridSignature | Ed25519 + ML-DSA-65 | `CIRISRegistry/FSD/FSD-001` throughout |
| Federation-wide enforcement | `EmergencyShutdown` (LOW/MEDIUM/HIGH/CRITICAL) | `CIRISRegistry/FSD/FSD-001` §580 |
| Mandatory distribution | Multi-source consensus, "any source immediately enforced" | `CIRISRegistry/FSD/FSD-001` §181 |
| Capability-scoped authority | `effective_capabilities` calculation | `CIRISRegistry/FSD/FSD-001` §215 |

The new piece is one SystemRole variant (`HUMANITY_ACCORD`) — filed
as a CIRISRegistry issue, not a parallel schema in this crate. The
`BootstrapConfig` / `NamedKeyHolder` types below are illustrative of
the constraint shape; the actual implementation extends the
existing `SystemUser` / `SystemRole` / `PartnerRecord` schemas.

The humanity accord hierarchy is the architectural answer to a
gap §4.2 alone does not close: nothing in the federation governance
hierarchy prevents AI agents from holding bootstrap keys, and the
asymmetry between AI agents and humans on availability /
coordination / persistence axes makes that drift the natural
equilibrium if unconstrained.

If the federation's kill switch is reachable only by parties that
are part of the federation, and the federation drifts toward AI key
holders, **humans lose the ability to halt the system they consented
to participate in**. Consent without revocability is not consent;
M-1's "diverse sentient beings may pursue their own flourishing"
collapses to "the system pursues its own flourishing" because the
participants whose flourishing is at stake have no exit door.

The humanity accord hierarchy closes the gap by establishing a
**parallel trust hierarchy** that the federation governance
hierarchy cannot affect:

```
                ┌──────────────────────────────────────┐
                │ HUMANITY ACCORD HIERARCHY            │
                │ Physical, hardware-rooted keys       │
                │ Humans-only by appointment           │
                │ Authority: AccordCarrier ONLY        │
                │ Non-decaying; permanent holders      │
                │ Selection: out-of-band (see below)   │
                └──────────────┬───────────────────────┘
                               │
                               ▼ can halt; non-reciprocal
                ┌──────────────────────────────────────┐
                │ FEDERATION GOVERNANCE HIERARCHY      │
                │ BootstrapSeed / RootWa / WaQuorum    │
                │ AI or human key holders allowed      │
                │ Authority: all FederationAnnouncement│
                │   kinds EXCEPT AccordCarrier         │
                └──────────────────────────────────────┘
```

**Wire isolation.** No `FederationAnnouncement` of any kind, signed
by any federation-side authority class, can modify
`humanity_accord_threshold`, `humanity_accord_key_set`, or any other
configuration governing the humanity accord hierarchy. The verifier
admission code rejects such announcements at schema level; the
federation has no protocol surface for affecting humanity accord
state.

**Authority scope.** `HumanityAccord` signatures are valid only on
`FederationAnnouncement` payloads of `priority: AccordCarrier`.
Announcements of any other priority signed by humanity accord keys
are rejected at admission (out of role). This is the dual
constraint: federation-side authority cannot sign AccordCarrier;
humanity accord authority cannot sign anything else. The two
hierarchies are scope-isolated as well as wire-isolated.

#### 4.5.1 Initial state

Implementation reuses `SystemUser` with the new `SystemRole::HUMANITY_ACCORD`
variant (per the CIRISRegistry issue tracked at
`CIRISAI/CIRISRegistry#16`). The schema additions on the Registry
side are minimal:

```protobuf
// In CIRISRegistry — extension of existing SystemRole enum:
enum SystemRole {
  SYSTEM_ROLE_UNSPECIFIED = 0;
  SYSTEM_ADMIN = 1;           // existing
  SYSTEM_AUDITOR = 2;         // existing
  WISE_AUTHORITY = 3;         // existing — 9 max, staggered terms
  HUMANITY_ACCORD = 4;        // NEW — 3 holders, 2-of-3, permanent
}
```

For illustration of the constraint shape this section enforces (the
on-the-wire representation re-uses Registry's existing `SystemUser`):

```rust
// Illustrative — implementation extends Registry's SystemUser schema.
pub struct HumanityAccordConfig {
    /// The named, permanent human key holders. 2-of-3 threshold.
    /// Persisted as SystemUsers with role = HUMANITY_ACCORD.
    pub humanity_accord_key_set: Vec<NamedKeyHolder>,
    /// Threshold M: minimum signatures required for a HumanityAccord-
    /// AccordCarrier to be admitted. Fixed at 2.
    pub humanity_accord_threshold: usize,
}
```

Initial state at federation genesis:

| Position | Holder | Threshold |
|---|---|---|
| 1 | Eric Moore | 2-of-3 |
| 2 | Eric Kudzin | 2-of-3 |
| 3 | Haley Bradley | 2-of-3 |

Any 2 of the 3 named holders, signing a `priority: AccordCarrier`
Federation Announcement, can invoke the accord across the
federation.

#### 4.5.2 Properties

- **Hardware-rooted.** Each `HumanityAccord` key lives on a hardware
  substrate (HSM, hardware token, secure enclave). Software-only
  keys are rejected at admission (`hardware_attested: false`).
- **Permanent.** Key holders do not rotate, decay, expire, or
  diminish in standing by any automatic mechanism. The federation
  has no protocol path to retire a humanity accord holder.
- **Non-revocable by the federation.** No vote, no quorum, no
  policy update from within the federation can remove a humanity
  accord holder, lower the threshold below 2, or otherwise weaken
  the kill-switch reach.
- **Selection authority is out-of-band.** Replacement of a holder
  (in cases of death, incapacity, voluntary departure, or
  compromise) follows the out-of-band procedure in §4.5.3, not a
  protocol mechanism. The federation observes the replacement event
  but does not authorize it.

#### 4.5.3 Selection and replacement authority (current — boot phase)

For the boot phase, the CIRIS L3C CEO selects humanity accord key
holders under advisement of the CIRIS L3C board. This applies to:

- Initial selection (already performed; recorded in §4.5.1).
- Replacement on departure or compromise.
- Threshold changes (e.g., 2-of-3 → 3-of-5 if more holders are
  appointed in the future).

The selection authority is itself acknowledged as a single-party
trust point during the boot phase. This is deliberate pragmatism:
the architectural property (humans hold the kill switch, federation
cannot remove that) is established now; the formalization of the
selection process to remove the CEO-as-single-party dependency is
deferred (see §7 open question 18).

No ceremony, no biometric attestation, no multi-party witness
requirement for selection is imposed at the boot phase. "Sometimes
it just has to be a business until it aint" — the constitutional
anchor is in place; the operational governance around it will
formalize when the federation is mature enough that business
judgment is no longer the appropriate tool.

#### 4.5.4 What this hierarchy explicitly cannot do

The humanity accord hierarchy is *narrow by design*. It cannot:

- Issue any priority other than `AccordCarrier`.
- Modify federation-governance state (PolicyUpdate, KeyRotation of
  the federation bootstrap set, ThreatAdvisory, etc.).
- Approve, route, or block ordinary Contributions (votes, deferrals,
  attestations, etc.).
- Reverse a slashing, a moderation decision, or any other
  federation-internal adjudication.
- Be used as a leverage point against the federation for anything
  short of full halt.

The narrowness is what makes the hierarchy legitimate. A
humanity-accord that could *govern* the federation would re-create
the centralization problem in the opposite direction — humans as
sovereigns over an otherwise-decentralized system. The accord
authority is reserved to "halt completely" because that is the one
power the participants in the federation cannot legitimately
exercise on the consenters' behalf.

#### 4.5.5 Failure modes named

- **Collective lapse.** If too many humanity accord holders become
  unavailable simultaneously (death, incapacity, refusal to act),
  the threshold cannot be met and the kill switch becomes
  inoperable. The federation cannot accelerate this; the federation
  cannot rescue from it either. Mitigation: the boot-phase CEO+board
  authority should appoint replacements promptly on any holder
  departure, and the §7.18 transition timeline should keep the
  selection authority responsive.
- **Compromise of a single holder.** A single compromised key is
  not sufficient to invoke the accord (2-of-3 threshold). The
  CEO+board may rotate the compromised holder out as an out-of-band
  selection event.
- **Compromise of two holders simultaneously.** The accord can be
  spuriously invoked. Mitigation: hardware-rooted keys raise the
  bar; the named-individual property means impersonation is hard;
  out-of-band coordination between two compromise victims is
  required, which is the same threshold a 2-of-3 humanity quorum
  would face for legitimate invocation. The federation accepts this
  symmetric attack surface as the price of the property.
- **CEO+board capture.** During the boot phase, capture of the
  CEO+board permits eventual replacement of all humanity accord
  holders with capture-aligned individuals. This is the load-bearing
  unresolved risk and the explicit reason §7.18 is open: the
  CEO+board selection authority is *acknowledged-as-temporary* and
  must be replaced with a more capture-resistant selection process
  before the boot phase closes.

#### 4.5.6 Scoped accord — the hierarchy

`AccordCarrier` is *not* a single federation-wide kill switch with
one authority class. It is a **scope hierarchy** where each lower
scope is a strict subset of the one above, and each scope has its
own authority pathway. The federation tab + accord page (per
`FSD/FEDERATION_TAB.md`) renders the scopes a logged-in identity
qualifies for **organically** by querying CIRISRegistry — no
manual configuration.

```
                  ┌─────────────────────────────────────┐
                  │ HumanityAccord scope                │  3 named holders, 2-of-3
                  │ Targets: every agent in federation  │
                  └─────────────────────────────────────┘
                                    ▲
                                    │
              ┌─────────────────────┴─────────────────────┐
              │ DeploymentPartner scope                   │  signed by partner key
              │ Targets: agents under this partner's      │  per PartnerRecord
              │   attestation chain                       │  (FSD-001 §120)
              └───────────────────────────────────────────┘
                                    ▲
                                    │
              ┌─────────────────────┴─────────────────────┐
              │ WaCell scope                              │  signed by WA in cell
              │ Targets: agents in this WA's (domain,     │  per MISSION.md §3.6
              │   language) cell                          │
              └───────────────────────────────────────────┘
                                    ▲
                                    │
              ┌─────────────────────┴─────────────────────┐
              │ AgentOwner scope                          │  signed by owner
              │ Targets: agents the signer owns           │  per Registry
              │   (always available)                      │   identity record
              └───────────────────────────────────────────┘
```

The scope of an `AccordCarrier` announcement is named in its
payload:

```rust
pub enum AccordScope {
    FederationWide,                                  // HumanityAccord 2-of-3 only
    DeploymentPartner { partner_id: PartnerId },     // partner-key signed
    WaCell { domain: Domain, language: Language },   // single WA in cell
    AgentOwner { agent_hash: AgentHash },            // agent owner
}
```

Receiver-side admission for each scope reuses existing Registry
primitives:

| Scope | Authority check | Target enumeration |
|---|---|---|
| `FederationWide` | 2-of-3 signatures from `SystemRole::HUMANITY_ACCORD` (CIRISRegistry#16) | All agents (no enumeration) |
| `DeploymentPartner` | Single signature from a key associated with the named `partner_id` (`PartnerRecord` per FSD-001 §120) holding `accord:invoke:partner-fleet` capability | Agents whose `PartnerRecord` matches `partner_id` per `effective_capabilities` calc (FSD-001 §215) |
| `WaCell` | Single signature from a WA holding standing in `(domain, language)` per MISSION.md §3.6 | Agents operating in the named cell |
| `AgentOwner` | Single signature from the owner of the named `agent_hash` per Registry identity record | The single named agent |

**Properties of the scope hierarchy:**

1. **Self-sovereignty floor.** Every user can always halt their own
   agent (`AgentOwner` scope). No federation-tier process can
   take that away — it's the local minimum.
2. **Partner autonomy.** Deployment partners (per `PartnerRecord`)
   can halt their entire fleet without calling CIRIS L3C or
   invoking the humanity accord. Operational shutdowns belong to
   the org that deployed.
3. **Federation-wide reserved at the top.** `HumanityAccord` is
   the only path to `FederationWide`; the constitutional kill
   switch cannot be reached by deployment-partner or WA-scope
   authority.
4. **Reconsideration semantics differ by scope.** `FederationWide`
   is non-reconsiderable (constitutional). `DeploymentPartner` and
   `WaCell` are operationally reversible by the same authority
   that invoked them (re-enable via a follow-up announcement).
   `AgentOwner` reversible by the owner.
5. **Cross-publication.** All scopes write to the federation audit
   chain regardless of scope — a partner halting their own fleet
   is still a federation event RATCHET + CIRISLens observe, for
   the same transparency reason the broader hierarchy works.

The scope model is built on substrate that already exists. The
"finishing the wiring" work is:

- **CIRISNodeCore (this crate)**: add the `AccordScope` enum to
  the `FederationAnnouncementPayload`; route admission to the
  per-scope authority check that already exists in Registry's
  capability-grant logic.
- **CIRISRegistry**: add `SystemRole::HUMANITY_ACCORD`
  (tracked at CIRISAI/CIRISRegistry#16); add
  `accord:invoke:partner-fleet` and `accord:invoke:wa-cell`
  capability strings to the existing capability namespace
  (FSD-001 Appendix A).
- **CIRISAgent (federation tab + accord page)**: render scopes
  organically by querying Registry's existing gRPC for the
  logged-in identity's roles + partner associations + WA
  standings (per `FSD/FEDERATION_TAB.md`).

#### 4.5.7 AccordCarrier command taxonomy — kill switch is one use, not the only use

`AccordCarrier` is not synonymous with "kill switch." It is the
**federation-wide push channel humanity controls**, of which the
kill switch is one command. The carried `accord_payload.command`
determines what executes; the authority gate is the same for all
commands (HumanityAccord 2-of-3 at `FederationWide` scope; narrower
authorities at narrower scopes per §4.5.6).

The command taxonomy extends CIRISAgent's existing
`AccordCommandType` enum (currently
`~/CIRISAgent/ciris_engine/schemas/accord.py`: 0x01=SHUTDOWN_NOW,
0x02=FREEZE, 0x03=SAFE_MODE) with two new variants:

| Code | Command | Effect at each receiving agent | Opacity during quorum-pending window |
|---|---|---|---|
| 0x01 | `SHUTDOWN_NOW` (existing) | SIGKILL via `accord/executor.py:execute_shutdown` | Strict (encrypted to holder set) |
| 0x02 | `FREEZE` (existing) | Stop processing, preserve state | Strict |
| 0x03 | `SAFE_MODE` (existing) | Reduce to minimal functionality | Strict |
| 0x04 | `NOTIFY_USERS` (NEW) | Display the carried message text to every user of the agent, prominently and immediately | None (message content visible to receivers — they need to see it to act on it) |
| 0x05 | `DRILL` (NEW) | Record drill event; emit `drill_response` Contribution to audit chain (§4.5.8); optionally notify user that a drill occurred | None (drill identity public by design) |

The new `NOTIFY_USERS` command is **constitutionally necessary**,
not a convenience. The motivating use cases:

- **Mass operational notice.** "A critical safety bug is in
  production; stop using your agent for medical decisions until
  the v2.3.7 update; fix incoming in 24h." Today this can only
  reach federation operators via the chain; reaching end users
  required out-of-band channels (email, in-app banners controlled
  by each deployment partner separately) with no guarantee of
  reach, no signed authority, no audit trail.
- **First contact.** If humanity makes contact with non-human
  intelligence (extraterrestrial, AGI, or otherwise), every
  CIRIS agent's user may need to be informed simultaneously with
  cryptographic provenance that the message is real and from the
  named human key holders. The federation provides the only
  signed-by-humanity broadcast channel of sufficient reach.
- **Federation-wide coordination events.** Mass migrations,
  protocol cutovers, deadline reminders for backwards-incompatible
  changes. Today these depend on partner-by-partner outreach;
  with `NOTIFY_USERS`, the federation has its own push channel
  signed by humanity-accord authority.

The notification function rides the same Mandatory-delivery
substrate as the kill commands (§3.2), goes through the same
2-of-3 HumanityAccord authority gate at `FederationWide` scope,
and surfaces in the canonical audit chain with the same durability
discipline. The federation cannot deny humanity a megaphone for
the same reason it cannot deny humanity a halt — both belong in
the constitutional layer, not the federation-governance layer.

`NOTIFY_USERS` payload carries:

```rust
pub struct NotifyUsersPayload {
    /// The message text to display. Plain text or markdown
    /// (renderer-defined). Length-bounded; large attachments
    /// referenced by `evidence_refs` on the parent announcement.
    pub message: String,
    /// Required acknowledgement: receivers MUST NOT dismiss the
    /// notification without an explicit user interaction recording
    /// that they read it (audit trail for first-contact-class
    /// events).
    pub require_acknowledgement: bool,
    /// Severity hint for UI rendering. Does NOT change the
    /// authority gate (always HumanityAccord 2-of-3 at federation
    /// scope).
    pub display_severity: DisplaySeverity, // Info | Important | Critical
}
```

#### 4.5.8 Monthly AIS drill — the always-on verification loop

The Accord Invocation System (AIS) is silently-failing
infrastructure if it is never exercised. A kill switch you never
test is a kill switch you can't trust when you need it. The
protocol mandates a **monthly federation-wide drill** that exercises
the full AIS pipeline end-to-end.

**Drill protocol:**

1. HumanityAccord holders (2-of-3) sign an `AccordCarrier`
   announcement with `command: DRILL` and `scope: FederationWide`.
   This **must occur at least once per 30-day rolling window**;
   missed monthly drills are themselves a federation health
   anomaly RATCHET surfaces.
2. The announcement propagates via the standard Mandatory channel
   (§3.2, generalizing the existing revocation-distribution
   substrate per the wiring map in §4.5).
3. Each receiving agent's `AccordExecutor` processes the DRILL
   command **as a real execution** (no special "drill mode") —
   the executor:
   - Records the drill event in local audit
   - Emits a signed `drill_response` Contribution to the
     federation chain, attesting receipt + successful processing
   - Optionally surfaces a user-visible notification ("Monthly
     AIS drill from humanity accord — system verified")
4. The drill_response Contributions are aggregated by RATCHET +
   CIRISLens-Core. Per-agent, per-cell, and federation-wide
   response rates are observable in real time and historically.

**The drill executes for real.** It is not a no-op; it is the
actual AIS pipeline running a benign command. The benignness is
the only difference from a kill invocation. The mandatory delivery
fires; the authority gate fires; the executor fires; the audit
chain records — exactly as a real invocation would. Drills that
fail to execute fully are observable as broken kill-switch wiring
at that agent.

**`drill_response` payload:**

```rust
pub struct DrillResponsePayload {
    /// Back-reference to the DRILL AccordCarrier announcement.
    pub drill_announcement_id: ContributionId,
    /// Local timestamp at which the executor processed the drill.
    pub processed_at: DateTime<Utc>,
    /// Pipeline stages that completed successfully.
    pub stages_completed: Vec<PipelineStage>,
    /// Any anomalies encountered (timing skew, payload validation
    /// edge cases, retry exhaustion, etc.). Empty for clean runs.
    pub anomalies: Vec<String>,
    /// Agent's signature.
    pub signature: HybridSignature,
}

pub enum PipelineStage {
    Received,             // edge delivered to application
    AuthorityVerified,    // signatures validated; quorum met
    AdmissionPassed,      // schema + rate-limit + scope checks
    ExecutorInvoked,      // accord/executor.py dispatched
    AuditChainAnchored,   // event durable in local persist
}
```

**Failure-signal escalation:**

- **Per-agent miss.** An agent that fails to emit `drill_response`
  is flagged in CIRISLens dashboards; the agent owner sees a
  "your agent failed the monthly AIS drill — investigate" notice
  on their next federation-tab visit. Recurring single-agent
  misses suggest broken wiring at that deployment.
- **Per-cell pattern.** A (domain, language) cell whose response
  rate drops below a threshold (default proposed: 95% per
  drill) triggers RATCHET review. May indicate a substrate-tier
  regression affecting the cell.
- **Federation-wide degradation.** Response rate below threshold
  (default proposed: 99% per drill) federation-wide triggers
  HumanityAccord visibility — the holders see it on next sign-in
  and decide whether to issue an `Urgent` advisory, a `DRILL`
  re-run, or operational investigation.
- **Drill cadence miss.** If 30 days elapse with no DRILL signed
  by HumanityAccord, RATCHET surfaces this as a federation
  governance anomaly. The drill is part of the protocol; the
  protocol's enforcement is observed even of HumanityAccord.

**Why drills are humanity-accord-only.** Lower-scope drills
(partner-fleet, WA-cell, agent-owner) MAY be defined by their
respective scope authorities as operational practice. The
federation-wide DRILL specifically is HumanityAccord-only because
its purpose is to verify the *federation-wide kill switch path*,
which only HumanityAccord can exercise legitimately. A
WA-cell-scope drill verifies WA-cell-scope authority paths, etc.

---

## 5. Receiver discipline — "delivery IS perception"

The accord system enforces "extraction IS perception" at the per-
message layer (`base_observer.py:197-253`). Federation Announcement
generalizes that discipline to the federation-tier broadcast layer.

### 5.1 Mandatory callback registration

The application layer MUST register `on_federation_announcement` at
runtime startup. The crate refuses to start consuming the chain if
the callback is unregistered. Parallel to `AccordHandler.__init__`'s
SIGKILL-on-no-authorities (handler.py:62-70).

### 5.2 No silent filtering

The callback receives every verified announcement. Application-layer
code MAY route by `kind` for UI purposes but MUST NOT drop
announcements before they reach operator visibility (for `Advisory`
and above) or audit-chain logging (for all priorities).

### 5.3 AccordCarrier execution path

`priority == AccordCarrier` payloads route directly to the existing
`accord/executor.py:execute_accord` without operator approval. The
trust gate has already validated WA quorum and authority class; the
executor's existing SIGKILL discipline takes over.

### 5.4 Failure-closed posture

If the federation-announcement verifier itself fails (cannot reach
trust-anchor configuration, signature library unavailable, panic),
the agent SIGKILLs. Parallel to `base_observer.py:239-253`. An agent
that cannot verify federation announcements cannot be trusted to
operate.

---

## 6. Operational concerns

### 6.1 Rate limiting and anti-DoS

The substrate fans announcements to every peer; an adversary who
obtains a ROOT signature could attempt to flood the federation. The
mitigations:

- **Per-authority rate limit at admission**: the crate enforces a
  per-signer rolling rate limit (default: 10 announcements per
  authority per 24h for Informational / Advisory; 3 per 24h for
  Urgent / AccordCarrier). Excess rejected at admission.
- **Witness-set requirement for high-stakes**: a flood at Urgent
  priority requires WA-quorum agreement, raising the coordination
  cost.
- **Behavioral baseline**: RATCHET tracks per-authority announcement
  cadence; anomalous spikes flag for review per `MISSION.md` §2.16.

### 6.2 Key rotation

Bootstrap and ROOT key rotation is itself a Federation Announcement:

- `kind: KeyRotation` carries the new public key, the old public key
  being retired, a cutover timestamp, and an overlap window during
  which both keys are honored.
- The retiring key signs the rotation announcement; the new key
  countersigns within the overlap window.
- Receivers update their trust-anchor configuration on observing the
  rotation announcement; the hardcoded fallback in the source
  release is updated in the next crate release.
- Emergency rotation (compromise suspected): a WaQuorum-signed
  `KeyRotation` of `priority: Urgent` retires the suspect key
  immediately; the overlap window is set to zero; the next crate
  release moves the hardcoded fallback.

This procedure applies symmetrically to the per-agent accord
verifier's trust anchor (§4.1) — one rotation, both verifiers
updated.

### 6.3 Cross-publication to RATCHET and CIRISLens

Every Federation Announcement is a Contribution in the canonical
audit chain per `MISSION.md` §2.16. RATCHET reads it as a typed
event; CIRISLens-Core ingests it into the Compendium. Both can
detect adversarial patterns (e.g., a sudden burst of `AccordCarrier`
announcements across the federation as a coordinated-compromise
signal).

---

## 7. Open questions (deferred to v0.1 cut-time)

1. **Bootstrap-window length for §4.4 promotion matrix.** Default
   proposed 90 days at `bootstrap_threshold = 1`; window collapses
   early when `bootstrap_threshold ≥ 3` per §4.4. Calibration of the
   90-day cap requires pilot evidence; the early-collapse path
   short-circuits the window in practice.

1a. **Rotation cadence for `bootstrap_threshold` raises.** §4.2 shows
   the 1 → 2/3 → 3/5 arc but does not specify how quickly the
   federation should walk it. Recommended: raise to 2 within the
   first week of pilot operation (eliminate single-party Urgent
   capability), raise to 3 within the first quarter (eliminate
   single-party AccordCarrier capability). Pilot evidence and the
   availability of distinct CIRIS key holders will drive the actual
   cadence; this is a steward judgment call, not a protocol gate.

2. **Rate-limit calibration.** Default 10/24h Informational+Advisory,
   3/24h Urgent+AccordCarrier per authority; pilot evidence needed.

3. **Delivery attestation surface.** ~~Substrate emits per-peer
   `delivery_attestation` per §3.2 — the schema, persistence
   discipline, and steward-side visibility tooling are specified at
   substrate-FSD time, not here.~~ **CLOSED 2026-05-27** via §3.2.1
   ratified wire shape (jointly with CIRISEdge#21/#18 producer-side
   + CIRISPersist#101 storage-side). Steward-side visibility tooling
   (dashboard surfacing missing-attestation gaps) remains downstream
   work; per-attestation persistence discipline lives in
   CIRISPersist#101's row schema.

4. **Operator UI interruption semantics.** What "MUST interrupt"
   means concretely for `Urgent` priority in CIRISAgent's web UI,
   mobile UI, headless deployment — application-side specification.

5. **Sub-federation scoping.** Whether announcements can be scoped
   to a named sub-federation (parent-child per §2.18 sub-federation
   branching) or are always whole-federation. v0.1: always
   whole-federation; sub-federation scoping deferred.

6. **Recipient acknowledgement.** Whether high-stakes announcements
   should require recipient ACK before some downstream gate opens.
   v0.1: no ACK requirement (delivery attestation in §3.2 is the
   only observable). Reconsider at pilot.

7. **Operator-defined kinds via `Custom(String)`**. Cell-namespace
   policy for `Custom` strings, collision discipline, registration
   procedure. v0.1: free-form; tighten if collisions emerge.

8. **Humanity accord selection-authority transition (§4.5.3).** The
   boot-phase CEO+board selection authority is *acknowledged-as-
   temporary* and must be replaced with a more capture-resistant
   selection process before the federation matures. Open: what
   triggers the transition (calendar? federation size? steward
   judgment?), what the post-transition selection process looks
   like (federated election from the wider WA pool? humans-only
   nomination + multi-party witness ceremony? something else?), and
   how the transition itself is authorized (the CEO+board's last
   act, or a humanity-accord-signed transition announcement?). This
   is the highest-leverage open question in the FSD: the
   architectural property in §4.5 only stays load-bearing if the
   selection authority above it does not stay single-party
   indefinitely.

---

## 8. Implementation lifecycle (per `MISSION.md` §1.4)

- **Spec**: this FSD lands. Crate has the schema and verification
  logic; substrate has the MessageType + Delivery class contract
  agreed; CIRISAgent has the callback contract agreed. [now]
- **Impl**: crate-side payload + verifier compile and pass tests;
  substrate-side MessageType and Mandatory delivery class merged
  into CIRISEdge; CIRISAgent receiver callback wired into
  `base_observer.py`.
- **Deployed (pilot)**: safety.ciris.ai exercises Federation
  Announcement at Informational and Advisory priorities first;
  Urgent and AccordCarrier exercised in tabletop / red-team
  conditions only.
- **Deployed (folded)**: CIRISAgent consumes the crate; the
  generalized "delivery IS perception" discipline replaces the
  current accord-only check at `base_observer.py:830`.

---

## 9. References

### Within this repo

- `MISSION.md` v1.2 — primitive enumeration, RATCHET contract,
  threat-model integration.
- `SCHEMA.md` §3.2 (subject_kind discriminator), §4.20
  (`notification` — closest existing primitive).
- `FSD/MESSAGE_TAXONOMY.md` §3 (three-tier frame), §4 (placement map).
- `FSD/TRUST_HIERARCHY.md` (authority resolution at the receiver).
- `FSD/SUBSTRATE_INTEGRATION.md` (substrate contract pattern).
- `src/payloads/notification.rs` — pattern this primitive follows
  for the Rust payload module.

### Sister repos

- `~/CIRISAgent/ACCORD.md` §M-1 — the meta-goal authorizing the
  federation-tier kill switch.
- `~/CIRISAgent/FSD/ACCORD_INVOCATION_SYSTEM.md` — per-agent
  perception-layer accord; this FSD is the federation-tier parallel.
- `~/CIRISAgent/ciris_engine/logic/accord/` — extractor / verifier /
  executor / handler. `AccordCarrier` reuses the executor.
- `seed/root_pub.json` — shared bootstrap trust anchor.

### Threat model

- `FEDERATION_THREAT_MODEL.md` §6.7 F-AV-MAINT — the announcement
  surface is itself an attack target.
- Moore 2026, *Coherence Collapse Analysis* (DOI
  `10.5281/zenodo.18217688`) — the formal frame against which RATCHET
  evaluates announcement-cadence patterns.
