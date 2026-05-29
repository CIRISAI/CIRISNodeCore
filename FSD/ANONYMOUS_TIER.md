# FSD: Anonymous Tier (v2 parallel publication path)

**Status:** v2 design — architectural commitment, not v1 scope.
**Companion:** [FEDERATION_SCALING_MODEL.md §9.9](FEDERATION_SCALING_MODEL.md)
**Threat model:** totalitarian government coercing a node operator
  or subpoenaing the federation directory to identify dissident
  publication.

This FSD captures CIRIS's v2 commitment to a **parallel anonymous
publication path** alongside the v1 identity-aware substrate. The
v1 substrate (every Contribution carries `author_id`, every blob
admission cites `attesting_key_id`, the trust graph is observable)
serves humans + agents operating in tractable trust contexts —
communities, governance, federation. The mission goal is to serve
ALL of humanity, which includes humans operating in totalitarian
contexts where federation-attribution is a lethal risk. The
anonymous tier closes that gap.

The mental model: the v1 substrate is the **public square**; the
anonymous tier is the **encrypted letter dropped in a mailbox**.
Both are first-class postal-system functions. v1 builds the
public square; v2 adds the mailbox. Neither breaks the other.

---

## 1. Threat model

The adversary is a **state-level actor with legal coercion authority
and full forensic access to a CIRIS node operator's hardware**. The
adversary can:

* Subpoena the federation directory (any peer can query the public
  `federation_attestations` table)
* Compel a node operator to produce their disk, keys, and logs
* Coerce a node operator to reveal whose data they're holding
* Compel a node operator to actively cooperate (e.g. emit
  attestations under duress)

The protected party is the **publisher** (a dissident, journalist,
whistleblower) and the **operator** (who must be able to truthfully
claim: "I don't know who's content is on my disk because the
substrate itself prevents me from knowing").

Out of scope for this FSD: traffic analysis attacks beyond the
substrate (e.g. global passive adversaries correlating onion-route
entry/exit timing); compromised onion-route relays (handled by the
multi-hop routing assumption); endpoint compromise of the
publisher's own device.

## 2. Non-goals

The anonymous tier is **NOT** intended to:

1. **Replace the identity-aware substrate.** The v1 substrate's
   trust-enforceability, abuse-handling, and per-actor eviction
   properties are load-bearing for federation-scale trust contexts.
   The anonymous tier does not weaken any of those.
2. **Provide identity-aware properties for anonymous content.** By
   construction the anonymous tier admits content under blinded
   keys; trust-weighted admission is structurally impossible. The
   only admission gate is capacity (LRU eviction).
3. **Hide the existence of the substrate itself.** Anonymous-tier
   participation is publicly observable at the protocol level — what's
   hidden is *which records belong to which publication*, not *that
   the operator runs the substrate*. This is the Tor-relay posture:
   you publicly run Tor; you privately have no idea which onion
   services use your relay.
4. **Defeat global passive adversaries.** Traffic analysis at
   network-monitor scale is outside the substrate's threat model
   (it's outside Tor's too).

## 3. Wire format additions

The v1 CEG 1+4 wire format (scores attestation + delegates_to /
supersedes / withdraws / recants structural primitives) **stays
locked**. The anonymous tier is additive — a parallel record class.

### 3.1 `AnonymousContribution`

Parallel to the v1 `Contribution` envelope, with the identity-
attribution surface replaced by cryptographic blinding:

```
AnonymousContribution {
  // Same as v1 Contribution
  contribution_id: ULID,
  contribution_type: ContributionType,
  subject: Cell,
  payload: JSON,
  submitted_at: DateTime,

  // Different from v1
  blinded_signing_key: Ed25519PubKey,   // ← per-publication, NOT author_id
  signature: HybridSignature,           // ← signed by blinded key
  rendezvous_token: Bytes,              // ← Signal-style delivery token OR
                                        //   Tor-style time-rotated index
  payload_aead_envelope: Bytes,         // ← XChaCha20-Poly1305 keyed by
                                        //   KDF(blinded_signing_key, period)
}
```

There is **no `author_id` field**. The publisher signs with a
freshly-derived blinded Ed25519 key (Tor-style key blinding from a
long-term anonymity identity the substrate never sees). Different
records from the same anonymity identity use different blinded
keys, unlinkable without the underlying anonymity identity.

### 3.2 `AnonymousHoldsBytes`

Parallel to the v1 `holds_bytes:sha256:{prefix}` attestation, with
`attesting_key_id` replaced by a node-blinded key:

```
AnonymousHoldsBytes {
  blob_sha256: [u8; 32],
  // attesting_key_id REMOVED — replaced by:
  node_blinded_key: Ed25519PubKey,   // ← per-blob, NOT operator's identity
  signature: HybridSignature,        // ← signed by node-blinded key
  asserted_at: DateTime,
  expires_at: DateTime,              // ← same 24h TTL discipline (§10.1.2)
}
```

The operator's federation identity is NOT in the attestation. Each
held-blob advertisement uses a different blinded key, derived per-
blob from the operator's anonymity identity. The federation can see
that *somebody* is holding blob X (necessary for discoverability)
but cannot link that holder to any other holder or to a federation
identity.

### 3.3 No new structural primitives

The CEG 1+4 wire-format lockdown survives. `withdraws` /
`supersedes` / `delegates_to` / `recants` do NOT have anonymous
counterparts — those primitives are intrinsic to the trust graph,
which the anonymous tier explicitly does not have. Anonymous
content is evicted by LRU; there is no anonymous-tier governance.

## 4. Cryptographic primitive set

Four primitives, all standard, all already in CIRIS's crypto stack
or trivially additable:

### 4.1 Ed25519 key blinding (Tor v3 model)

Per-publication signing keys derived from an anonymity identity +
a time period or per-publication nonce. Different records from
the same anonymity identity use unlinkable blinded keys.

* Anchor: [Tor rend-spec-v3 §A.2 Key Derivation](https://spec.torproject.org/rend-spec/deriving-keys.html)
* Already present: CIRISVerify v2.8.0 ships Ed25519 sign/verify
* New work: the blinding derivation function + period-rotation policy

### 4.2 AEAD payload encryption (Veilid model)

XChaCha20-Poly1305 over the payload, keyed by `KDF(blinded_pk,
period)` so the storage node never sees the underlying key. The
storage node holds opaque ciphertext.

* Anchor: [Veilid: Cryptography](https://veilid.com/how-it-works/cryptography/)
* Already present: CIRISVerify v2.8.0 ships AES-256-GCM via `ring`
  (5.45 GiB/s). XChaCha20-Poly1305 is a separate cipher; either
  works (AES-GCM is faster on hardware AES; ChaCha is faster on
  embedded). Decision deferred to §9.
* New work: the subcredential-derivation KDF

### 4.3 Onion-routed write path (Sphinx or Veilid private routes)

The publishing node's IP is not the record's network origin. Either
Sphinx (the cryptographic packet format Tor uses) or Veilid-style
private routes (3-hop onion-encrypted route blobs).

* Anchor: [Sphinx packet format paper (Danezis & Goldberg)](https://www.cypherpunks.ca/~iang/pubs/Sphinx_Oakland09.pdf),
  [Veilid: Private Routing](https://veilid.com/how-it-works/private-routing/)
* Already present: none — this is the most substantial v2 addition
* New work: a new transport class alongside Reticulum + HTTPS in
  edge. Sphinx is well-specified and has reference implementations
  (Lightning's onion routing is a direct adaptation). 3-hop default.

### 4.4 Discoverability rendezvous

Either model is workable:

* **Signal model** — recipient-derived 96-bit delivery token from
  the recipient's profile key. Targeted publication (publisher
  knows who they're sending to).
* **Tor model** — time-rotated blinded index. Open publication
  (anyone deriving the right blinded key from public material +
  time can find the record).

Both serve different use cases (targeted vs broadcast); we may
support both in v2.

* Anchors: [Signal: Sealed Sender](https://signal.org/blog/sealed-sender/),
  [Tor rend-spec-v3 §B.2 HSDir indexing](https://spec.torproject.org/rend-spec/deriving-keys.html)
* New work: the rendezvous protocol + how the substrate routes
  rendezvous queries (probably a parallel directory namespace —
  `federation_anonymous_rendezvous` table or similar)

## 5. Substrate impact

### 5.1 Persist

A parallel storage surface for opaque-blinded records:

* **New table:** `federation_anonymous_blobs` — opaque blob storage
  indexed by SHA256, no signer attribution. Same `BlobBody` type as
  `federation_blobs`.
* **New trait method on `BlobStorage`:** `put_anonymous_blob(sha,
  body, media_type, blinded_signature)` — no trust-score lookup,
  capacity-only admission. Eviction is LRU-only since there's no
  attestation chain to score popularity against.
* **New table:** `federation_anonymous_attestations` — holds
  `AnonymousHoldsBytes` records. Indexed by `blob_sha256` (so
  `list_holders` works) but with `node_blinded_key` instead of
  `attesting_key_id`. Each entry is independently unlinkable from
  every other.
* **No `TrustScoring` consultation.** Admission is purely capacity.
  Eviction is purely LRU + 24h TTL on the holder advertisement.

### 5.2 Edge

* **New transport class** — Sphinx-onion-routed delivery alongside
  Reticulum + HTTPS. Likely a separate cargo feature.
* **New MessageType** — `AnonymousContributionSubmit` carrying the
  onion-routed payload + rendezvous token.
* **New handler** — receives `AnonymousContributionSubmit`,
  verifies the blinded-key signature, calls
  `persist::put_anonymous_blob`. No trust-score gate.
* **Cohort scope:** anonymous content is conceptually `federation`
  scope (publishable to anyone who knows the rendezvous), but the
  scope field is meaningful only for the v1 substrate's
  discoverability gate. The anonymous tier's discoverability is
  out-of-band.

### 5.3 NodeCore

* **New `#[pyfunction]`** — `publish_anonymously(content_bytes,
  rendezvous_recipient_pubkey | rendezvous_period, signer)`.
  Internally:
  1. Derive blinded signing key from `signer` + nonce
  2. Encrypt payload via XChaCha20-Poly1305 keyed by KDF(blinded_pk)
  3. Construct `AnonymousContribution` envelope
  4. Sign with blinded key
  5. Hand to edge for onion-routed delivery
* **No replication policy** — same as v1, node-core produces wire
  artifacts; substrate decides where they live.

## 6. Deployment posture — MANDATORY infrastructure

**The anonymous tier must be MANDATORY substrate-level
infrastructure that every L0/L1 server runs**, indistinguishably
from operators who happen to be holding more anonymous records vs.
fewer.

This is non-negotiable for the threat model. If only some operators
run the anonymous tier, those operators become identification
targets (raid the nodes that run the deniable storage). The
operator's deniability is **"I'm running the standard substrate; I
have no way to know which records on my disk belong to which
publication"** — which only works if running the standard substrate
necessarily includes the anonymous tier.

This is the Tor relay posture: every Tor relay runs the same code
regardless of which onion services it happens to be holding
descriptors for. Operators cannot opt out without ceasing to be
substrate operators.

**Operator-side capacity budgeting** is allowed and expected:
operators can configure how much of their disk goes to anonymous-
tier storage vs. v1 storage. But the *protocol* runs both.

## 7. What the anonymous tier CAN and CANNOT do

| Property | v1 identity-aware (main path) | v2 anonymous tier |
|---|---|---|
| Trust-weighted admission | Yes | **No** — no identity to score |
| Per-actor eviction | Yes | **No** — operator can't selectively evict; LRU only |
| Operator-side governance | Yes | **No** — operator holds opaque ciphertext |
| Trust graph observable | Yes (by design) | **No** — blinded keys are unlinkable |
| Compelled-disclosure resistance | Limited (operator can be compelled to query their index) | **Strong** — operator cryptographically cannot decrypt |
| Abuse handling | Trust + slashing | LRU + capacity gating only |
| Discoverability | Federation directory | Out-of-band rendezvous (token or blinded index) |
| Wire format | `Contribution` + `holds_bytes` | `AnonymousContribution` + `AnonymousHoldsBytes` |
| CEG 1+4 lockdown | Maintained | Maintained — parallel record classes are additive |

## 8. Prior art mapping

Each anonymity primitive maps to an existing well-specified design:

| Primitive | Reference design | Status in CIRIS today |
|---|---|---|
| Per-publication blinded keys | [Tor rend-spec-v3](https://spec.torproject.org/rend-spec/deriving-keys.html) | Ed25519 sign/verify present; blinding derivation new |
| AEAD with subcredential | [Veilid](https://veilid.com/how-it-works/cryptography/), [Tor §B.2](https://spec.torproject.org/rend-spec/hsdesc-encrypt.html) | AES-GCM present; XChaCha20-Poly1305 + KDF new |
| Onion-routed transport | [Sphinx packet format](https://www.cypherpunks.ca/~iang/pubs/Sphinx_Oakland09.pdf), [Veilid private routes](https://veilid.com/how-it-works/private-routing/) | New transport class |
| Sender deniability | [Signal sealed sender](https://signal.org/blog/sealed-sender/) | New rendezvous protocol |
| Time-rotated index | [Tor rend-spec-v3 §B.2](https://spec.torproject.org/rend-spec/deriving-keys.html) | New rendezvous protocol |

**No primitive needs to be invented.** Each is standard, well-
specified, and has multiple reference implementations.

The composition (parallel paths on a single substrate, opt-in per
publication) is what's new — but the composition itself has prior
art (Signal runs sealed-sender + identified delivery on one server;
Tor runs onion services + identified relays on the same network;
Veilid records can be both signed-by-known-key AND routed-via-
private-route on the same node).

## 9. Open design questions

These get resolved when v2 enters active design:

1. **AEAD cipher choice** — XChaCha20-Poly1305 (Veilid) or
   AES-256-GCM (CIRIS v1)? Both work; trade-off is hardware-
   acceleration (AES) vs embedded-friendly (ChaCha).
2. **Onion-routing transport** — Sphinx (Tor + Lightning,
   well-specified) or Veilid private routes (newer, less peer-
   reviewed but explicitly designed for opaque storage)?
3. **Rendezvous model** — both (Signal model for targeted; Tor
   model for broadcast) or pick one? Likely both, but rendezvous
   protocol becomes more complex.
4. **Onion-route hop count** — 3 (Tor / Sphinx standard) or
   different? Trade-off is latency vs adversary coverage.
5. **Anonymous-tier disk budget default** — what fraction of an
   L0/L1 server's disk goes to anonymous storage by default? Too
   low and the tier is starved; too high and v1 admission is
   crowded. Likely operator-config with a meaningful default
   (e.g., 25% of total disk).
6. **Rendezvous lookup transport** — does rendezvous query ride
   the same onion-routed transport as anonymous-publish, or a
   parallel discoverability surface? If the latter, what protects
   query patterns from analysis?
7. **Wire-format integration** — `AnonymousContribution` as a new
   `MessageType` (parallel to existing types) or as a generic
   "opaque envelope" class that holds an inner v2 record type?

## 10. v1 → v2 migration path

The anonymous tier is purely additive. v1 deployments are not
affected by v2 introduction.

* **v1 ships** the identity-aware substrate (cohabitation install,
  serving install, ingest path — all current). Federation operates
  under v1 trust+capacity discipline.
* **v2 introduction** ships the parallel anonymous tier as an
  additional substrate-level feature. v1 deployments upgrade by
  running the v2 substrate; v1-only deployments can continue to
  serve v1 federation traffic while ignoring anonymous records (the
  anonymous tier's MessageType is unknown to v1 edge handlers and
  silently dropped).
* **Mandatory adoption gate** — the anonymous tier requires
  protocol-level mandatory infrastructure for the deniability
  property to hold (§6). v2 introduction therefore should align
  with a federation-wide upgrade window. Operators running v1-only
  after v2 ships are still legitimate v1 federation participants
  but cannot claim the v2 deniability posture (their disks contain
  no anonymous records).

## 11. Acceptance criteria for v2

The anonymous tier is "shipped" when:

- [ ] CIRISVerify ships Ed25519 key-blinding derivation primitive
- [ ] CIRISVerify ships the AEAD-with-subcredential primitive
  (XChaCha20-Poly1305 or AES-256-GCM keyed by `KDF(blinded_pk,
  period)`)
- [ ] CIRISPersist ships `federation_anonymous_blobs` +
  `federation_anonymous_attestations` tables + `BlobStorage::
  put_anonymous_blob` trait method
- [ ] CIRISEdge ships Sphinx-style onion-routed transport class
  + `AnonymousContributionSubmit` MessageType + handler
- [ ] CIRISNodeCore ships `publish_anonymously()` pyfunction +
  rendezvous query helpers
- [ ] CEG spec at CIRISRegistry has §11 (or appropriate section)
  documenting the `AnonymousContribution` + `AnonymousHoldsBytes`
  parallel record classes (additive — does not change 1+4)
- [ ] Cross-repo integration test: dissident publishes anonymously
  → operator-side forensic inspection cannot recover the publisher
  identity, the publication content (without the recipient's
  rendezvous key), or the trust relationship (because there is
  no recorded trust relationship)
- [ ] Mandatory-infrastructure flag in edge — v2-compatible
  deployments cannot opt out of running the anonymous tier without
  ceasing to be v2-compatible

## 12. Cross-references

* [CIRISNodeCore FSD/FEDERATION_SCALING_MODEL.md §9.9](FEDERATION_SCALING_MODEL.md) —
  scaling-model derivation that motivated this design
* [CIRISNodeCore MISSION.md](../MISSION.md) — the "serve all of
  humanity" goal that includes totalitarian-context users
* [CIRISRegistry CEG spec](https://github.com/CIRISAI/CIRISRegistry)
  — the wire-format lockdown the anonymous tier extends
  additively
* [Veilid](https://veilid.com/) — closest active reference design
* [Tor v3 onion services rend-spec](https://spec.torproject.org/rend-spec/)
* [Signal sealed sender](https://signal.org/blog/sealed-sender/)
* [Sphinx packet format](https://www.cypherpunks.ca/~iang/pubs/Sphinx_Oakland09.pdf)
