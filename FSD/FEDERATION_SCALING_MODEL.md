# Federation Scaling Model

**Status:** model v0.3 — single-pool, CEG-organic, trust-depth recursive.
**Empirical inputs:** Verify v2.7.0 / v2.8.0 measured on
ubuntu-latest CI; Edge v0.10.0 targets carrying into v1.0; Persist
v3.3.0 storage floor.
**Companion:** `examples/scale_model.rs` — `cargo run --example
scale_model`.

This is a design-search tool. The question is not "how big can it
get" — it's "what intake + eviction discipline makes CIRIS substrate
carry the entire internet from day one on commodity hardware,
without inventing a replication layer above CEG's primitives?"

---

## 1. The CEG-organic replication discipline

CEG already has every primitive the replication policy needs. There
is no separate "replication layer" — reading the existing wire format
honestly IS the policy.

### 1.1 The intake gate (every node, every byte-attempt)

```
hold? = trust(source) ≥ local_threshold  AND  capacity_available
```

| Element | CEG primitive |
|---|---|
| `trust(source)` | weighted_aggregate over `scores` attestations targeting source's key (P1 + P7) — already computed for vote-weight |
| `capacity_available` | persist's disk budget vs configured cap |
| Push terminates here | `Contribution{Submit}` → persist `put_blob` / `put_contribution` |
| Pull terminates here | `ContentFetch` → `ContentBody` → recipient's `put_blob` |

Push and pull are mechanically distinct paths but the *decision*
is identical: same trust score, same capacity check, same answer.

### 1.2 The eviction sweeper (every node, on capacity pressure)

```
evict_score(blob) = popularity(blob) × freshness(blob)
                  = access_count_since(T) × decay(now − last_accessed_at)
```

| Element | CEG primitive |
|---|---|
| Popularity (local) | `last_accessed_at` + access counter on persist's blob rows |
| Popularity (federation-observable) | Count of `holds_bytes:sha256:{prefix}` advertisers for this content_sha |
| Freshness floor | 24h TTL on `holds_bytes` per CEG §10.1.2 |
| Eviction wire-event | `withdraws` against own prior `holds_bytes` — the §10.1.2 ContentMiss feedback loop |
| Decay curve | Per-deployment config (Pi-at-home wants slow decay; phone wants aggressive) |

### 1.3 The locality dividend (structural, free)

```
cohort_scope ∈ {self, family}  ⇒  no holds_bytes emission
                                ⇒  undiscoverable
                                ⇒  intake gate never reached
                                ⇒  ZERO inter-host cost
```

The 65% local-only fraction in the default cohort distribution
disappears from every inter-host bandwidth, storage, and CPU column
because the wire format will not carry it. Privacy and scale share
the same load-bearing primitive.

### 1.4 Trust recursion depth (operator-side, no CEG enhancement)

**People can trust an entity to be trusted, and choose a depth of
recursion for that trust.** This is a **local operator-config knob**,
not a CEG wire-format addition — nothing is advertised, no new field
on `delegates_to`, no schema change. The federation's existing
`scores` + `delegates_to` attestations already carry the entire
trust graph; each operator independently chooses how deep their
server walks that graph when admitting inbound content.

- `depth=0`: admit only direct trust — strict
- `depth=1`: also admit content from peers your direct trust trusts
- `depth=N`: walk the chain to depth N before admitting

The effective trust set whose bytes can pass the server's intake
gate is the **transitive closure within depth-N hops**. CEG stays
minimal (no new wire surface); the trust graph is already there for
anyone who wants to walk it; operators just choose how far.

**Tier-tied defaults:**

| Tier | Default depth | Rationale |
|---|---|---|
| client | **0** (always) | Phone / tablet holds own + explicit fetches only. No recursion, no admission of trust-chain content. |
| **proxy = L0 server** | **0** (default) | Entry-level federation participant — admits direct trust only (no recursion); 256 GB budget. Same admission discipline as L1, narrower depth. |
| **server = L1** | **1** (default) | Full federation node — admits friend-of-friends; 1 TB budget. Operator-tunable to 0 (strict) or 2-3 (extended). |

**Empirical hop-expansion** (small-world / six-degrees research,
calibrated in `effective_trust_set_multiplier()`):

| Depth | Effective multiplier | Reach |
|---|---|---|
| 0 | 1× | direct only |
| 1 | 4× | close friend-of-friends (heavy overlap) |
| 2 | 20× | extended community |
| 3 | 100× | most of the network |
| ≥3 | gentle extrapolation | saturation |

The geometric growth is dampened by friend-of-friend overlap — in
small-world graphs, my friend's friends are mostly already my
friends, so the unique-set growth per hop is far less than `R^depth`.

---

## 2. Empirical inputs (measured)

All numbers wall-clock on GitHub Actions `ubuntu-latest` —
conservative-by-design baseline. Dev-host CPUs run 2–3× faster.

### 2.1 Crypto (CIRISVerify v2.8.0)

| Op | Cost |
|---|---|
| `hybrid_sign` (Ed25519 + ML-DSA-65) | **466 µs** (~2.15 K sign/s/core) |
| `hybrid_verify` | **276 µs** (~3.62 K verify/s/core) |
| `aes_gcm_encrypt` @ 64 KiB | 11.2 µs (**5.45 GiB/s**) — cache encryption is free |
| `aes_gcm_decrypt` @ 64 KiB | 10.3 µs (**5.91 GiB/s**) |

### 2.2 Edge (v0.10.0 → v1.0 contract)

| Op | Target |
|---|---|
| `envelope_canonicalize` slope | ~250 ns/KiB |
| `envelope_verify` (single) | ~280 µs |
| `dispatch_inbound` (256 B) | < 400 µs → ~2.5 K msg/s/thread |
| `outbound_enqueue Durable` | < 1.5 ms |
| `content_fetch_roundtrip` 1 MiB | < 500 ms |
| `inline_text_pipeline` Classify+Scrub | **5–10 ns/byte** |

### 2.3 Persist (v3.3.0)

| Op | Cost |
|---|---|
| SQLite per-row write (incl. async wrapper) | ~1.5 ms |
| Ingest pipeline @ 768 rows (release) | ~9 ms (~85 K rows/s) |
| Software signer | ~100 µs/sign |
| H3ERE trace per agent decision | ~14 KB |

---

## 3. v1 tier model (L0 / L1 server gradient)

The tier model collapses to **server-gradient + client**. "Proxy"
isn't a distinct architecture — it's an L0 server, the entry-level
storage gradient. Same trust+capacity admission discipline as L1
server, just smaller disk and shallower default trust depth.

| Tier | Storage gradient | Default disk | Default depth | Behavior |
|---|---|---|---|---|
| **client** | n/a | n/a | 0 | No inbound serving. Holds own contributions + own traces. Fetches via L0/L1 proxy/server. Phone / tablet. |
| **proxy = L0 server** | L0 | **256 GB** | 0 (strict) | Full trust+capacity admission. Holds direct-trust content + hot cache. No agent-trace replication. Laptop / desktop. |
| **server = L1** | L1 | **1 TB** | 1 (friend-of-friends) | Full participant. Holds own + admitted-trust + hot cache + replicated agent traces. Home server / VPS. |
| (future) L2+ | L2+ | TBD | 2+ | "Fat servers" with deeper recursion + more disk. Not in v1. |

Each tier still runs the same `trust(source) ≥ threshold AND
capacity_available` intake discipline; the gradient just changes
(budget, depth). All three storage tiers (L0/L1/L2) are real
federation participants — they admit content, sign holds_bytes
attestations, become discoverable as holders.

**Per-server feasibility gates** (the model checks at every tier):

| Resource | L0 gate | L1 gate | Source |
|---|---|---|---|
| Disk | 256 GB | 1 TB | Eric's spec |
| Bandwidth | 1 Gbps (10.8 TB/day sustained) | 1 Gbps | Residential fiber |
| CPU | 1 full-utilization core (86.4 K cpu-sec/day) | Per-process CIRIS share |

---

## 4. Per-actor formula (the v0.3 model)

```
effective_R = trust_radius × effective_trust_set_multiplier(trust_depth_avg)
daily_admitted = effective_R × daily_bytes × σ_publishable
held(t) = min(disk_budget(t),
              own_unbounded
              + admitted_trust_at_steady_state
              + hot_cache_at_steady_state
              + replicated_traces)
```

Where each term is bounded by its share of `disk_budget × utilization`
(0.92 default), and the eviction sweeper maintains the bound.

**`effective_retention_days`** is what the eviction sweeper produces
at steady state — a *derived* quantity:

```
effective_retention_days = trust_budget_share / daily_admitted
```

Higher trust depth (wider effective set) → higher daily inbound →
shorter per-source retention at the same disk budget. The
eviction-popularity-weighting determines *which* sources keep their
content fresh in the held set.

---

## 5. v1 scenarios (server depth 1 default)

All scenarios use the **server-tier default depth=1** (friend-of-friends).
The implied retention is what the eviction sweeper produces at steady
state — derived from (disk_budget, trust topology, demand), not
configured.

| Scenario | N | Tier (c/p/s) | R | effective | D/user | σ_pub | Storage / BW / CPU | Implied retention |
|---|---|---|---|---|---|---|---|---|
| `bootstrap` | 10⁴ | 30/65/5 | 50 | 200 | 20 KB | 35% | 235 GB / tiny / tiny | ✓ ~234 yr |
| `dunbar_steady` | 10⁶ | 40/55/5 | 150 | 600 | 50 KB | 35% | 235 GB / tiny / tiny | ✓ ~31 yr |
| `media_heavy` | 10⁶ | 30/60/10 | 150 | 600 | 500 KB | 35% | 485 GB / 1 KB/s / 1 sec/d | ✓ ~10 yr |
| `twitter_scale` | 10⁹ | 45/50/5 | 150 | 600 | 5 KB | 35% | 152 GB / tiny / tiny | ✓ ~87 yr |
| `news_replacement` | 10⁹ | 40/55/5 | 300 | 1.2K | 100 KB | 35% | 321 GB / 1 MB/s / 6 s/d | ✓ ~14 yr |
| **`full_internet_v1`** | **5×10⁹** | **35/55/10** | **250** | **1K** | **50 MB** | **35%** | **741 GB / 62 KB/s / 43 s/d** | **✓ 37 d** |
| `full_internet_local_heavy` | 5×10⁹ | 35/55/10 | 250 | 1K | 50 MB | 30% | 736 GB / 50 KB/s / 36 s/d | ✓ 44 d |
| `full_internet_global_heavy` | 5×10⁹ | 30/55/15 | 250 | 1K | 50 MB | 60% | 743 GB / 109 KB/s / 73 s/d | ✓ 22 d |
| `village_dense` | 10³ | 40/40/20 | 50 | 200 | 30 MB | 30% | 721 GB / 8 KB/s / 27 s/d | ✓ ~1.1 yr |

### 5.1 What the numbers say

**Compute and bandwidth never gate.** Even at 5 B users with depth 1
(effective_sources = R × 4 ≈ 1K), per-server CPU stays below 0.2% of
1 core and bandwidth below 0.2% of 1 Gbps. The hybrid PQC verify
cost (276 µs) is invisible in aggregate.

**Disk budget is the only knob that bites — and the model shows
exactly why.** Two terms compete for the budget:
1. Own content (priority — your data is always yours)
2. Admitted-trust content at `daily_admitted × T_effective`

The eviction sweeper sets `T_effective` so the held set equals 92%
of disk. Wider trust set or higher activity → shorter `T_effective`.
Smaller → longer.

**Trust depth is a meaningful operator knob.** At `full_internet_v1`:
- depth 0 → ~150 days retention (strict, direct trust only)
- depth 1 → 37 days (default — friend-of-friends, R × 4 sources)
- depth 2 → ~7 days (extended community, R × 20 sources)
- depth 3 → single-digit days (most of the network)

Operators trade reach for retention. The federation doesn't dictate
the trade-off; each server picks its own depth as a local config.

**Cache hit rate is bounded sensitivity.** The sensitivity sweep
(`print_cache_sensitivity`) on `full_internet_v1` shows < 1 GB/day
bandwidth variation from 0.3 (pessimistic) to 0.85 (optimistic) and
no change in implied retention. At v1 scale, admitted-trust inbound
(~17 GB/day) dominates cache-miss inbound (~0.4 GB/day) by ~40×;
the cache assumption barely moves the needle. Real telemetry will
matter more on tiers where trust admission is small (client / proxy)
or in low-R deployments.

**Village-scale is essentially free.** R=50 + depth 1 (200 effective
sources) gives ~1.1 years of admitted-trust content held on a Pi-class
home server. Substrate is deployable in small communities day one.

---

## 6. What's NEW in v0.3 vs v0.2

- **No more `direct_trust_archive_days` / `cache_ttl_minutes` /
  `cache_hit_rate` / `server_cache_max_bytes` knobs.** Those were
  modeling a policy layer that doesn't need to exist. Replaced by
  one knob: `disk_budget_server`. The composition is derived.
- **Trust recursion depth** is first-class: `trust_depth_avg` per
  scenario, with the `effective_trust_set_multiplier()` curve
  reflecting small-world hop expansion + overlap dampening.
- **Implied retention** is a derived output, not a configured input.
  The model now shows what the eviction sweeper *will* produce at
  steady state for each scenario.
- **CEG-primitive mapping** (§1) — every model element is mapped to
  the existing wire-format primitive it rides on. No new mechanisms
  invented.

---

## 7. What this model is NOT

- **Not a capacity guarantee.** Bench numbers are CI ubuntu-latest.
- **Not a network simulator.** Steady-state averages; not congestion
  or Reticulum reachability dynamics.
- **Not a privacy certifier.** The CEG-locality dividend is enforced
  by the wire format; the model only costs it.
- **Not a final answer.** Trust-depth multiplier curve is empirically
  anchored but not measured against real CIRIS topology; cache-hit
  rates are assumptions; agent decision rate at population scale is
  a guess. All inputs are honest about being inputs.

The model is a planning tool. Run scenarios, see where the knobs
matter, calibrate against real federation data as it accumulates.

---

## 8. What this implies upstream

The substrate primitives needed to execute this discipline:

**Persist** (filed as CIRISPersist replication-policy issue):
- Trust-score lookup at `put_blob` / `put_attestation` /
  `put_contribution` admission
- `last_accessed_at` + access counter on `federation_blobs` rows
- Eviction sweeper computing `popularity × freshness`
- Configurable disk budget + steady-state utilization watermark
- Encrypted-at-rest for cache content (already shipped in persist)

**Edge** (filed as CIRISEdge trust-gate issue):
- Trust-score short-circuit at `dispatch_inbound` (before handler)
- `cohort_scope` check at `outbound_enqueue` (refuse self/family
  outbound — the wire-format locality enforcement)

**Trust recursion depth needs NO upstream change** — it's a local
operator config consuming the existing `scores` + `delegates_to`
attestation graph. CEG's 1+4 wire format stays locked.

**NodeCore** (already done):
- The Phase 2B ingest path (CIRISNodeCore#19) produces the wire
  artifacts that ride these primitives. Node-core does not own
  any replication policy — by design.

---

## 9. Why this works at all — the identity-aware-storage property

> **Eric's thesis:** "What makes this work is that you know whose
> data you are storing, and can evict their data at any time if
> you choose."

This isn't just a feature — it's the load-bearing property the
entire discipline rests on. The whole `trust(source) ≥ threshold
AND capacity_available` intake + `popularity × freshness` eviction
model presumes the substrate can answer two questions at any moment:

1. **Whose bytes am I holding?**
2. **Can I evict everything from a specific actor right now?**

### 9.1 How CIRIS guarantees both

Every blob admission is one atomic call:

```rust
BlobStorage::put_blob_signing(
    sha256, body, media_type,
    attesting_key_id,        // ← identity of the holder
    signer,                  // ← cryptographic witness
    now, attestation_id,
)
```

The call commits THREE things atomically:
* the bytes (`federation_blobs` row)
* the holder attestation (`federation_attestations` row with
  `attesting_key_id`, `attestation_type=holds_bytes:sha256:{prefix}`,
  `evidence_refs` containing the SHA)
* the signature over the canonical envelope (persist's
  `PythonJsonDumpsCanonicalizer`, per CIRISPersist#121's
  identity-pin)

After this returns, the substrate can answer:
* "whose bytes do I hold?" — `SELECT attesting_key_id FROM
  federation_attestations WHERE attestation_type LIKE
  'holds_bytes:%' AND blob_sha = ?`
* "evict everything from author X" — query `federation_blobs` JOIN
  `federation_attestations` ON the holder's `attesting_key_id`,
  delete the rows + emit `withdraws` against each `holds_bytes`
  attestation

The attribution chain is **at the byte level**, not the application
layer. Eviction granularity is **per-actor**, not just LRU-tail.
Both properties ride the same atomic primitive.

### 9.2 Prior art — no deployed system has this as a unified mechanism

Surveyed against IPFS, Veilid, Hypercore, SSB, Storj, Filecoin, Sia,
Tahoe-LAFS, Mastodon, Tor, Freenet. The two-property combination
(identity-aware byte-level storage + per-actor eviction granularity
as a substrate primitive) does not appear unified anywhere:

| System | Identity-aware bytes? | Per-actor eviction? | Pattern |
|---|---|---|---|
| **IPFS / Kubo** | No | No | Anonymous content-addressing; LRU watermark only |
| **IPFS Cluster** | Partial | Partial | Knows "the peer who asked us to pin," not the author |
| **Veilid** | Partial | Couldn't verify | Signed DHT records; block storage hash-addressed |
| **Hypercore / Holepunch** | Yes (feed-level) | Yes (feed-level) | Identity rides the feed; cross-feed blobs re-attributed |
| **Storj** | Partial (satellite) | Partial | Nodes see only erasure-coded ciphertext |
| **Filecoin** | Partial | **No** (by design) | Contract binds host to keep data; eviction = slashed |
| **Sia** | Partial | **No** (by design) | Same — contract-bound hosting |
| **Tor exit relay** | No (by design) | No | Unlinkability is the threat model |
| **Freenet** | No (by design) | No | "Infeasible to discover origin" — by design |
| **Tahoe-LAFS** | Partial (planned) | Partial | Accounting design proposed, not deployed |
| **Mastodon / ActivityPub** | Yes (object-level) | Yes | But at **application layer**, not byte-storage substrate |
| **SSB (Scuttlebutt)** | Yes (feed-level) | Partial | Replicated blobs decouple from feed identity |

**The closest analogs are SSB and Hypercore** (feed-level identity)
and **Mastodon** (object-level identity at the application layer).
None weld attribution and eviction into a single byte-storage-
substrate primitive the way `put_blob_signing` does.

### 9.3 Why the contract-storage systems explicitly REJECT this

**Filecoin / Sia / Storj are the inverse design.** Their entire
commercial value proposition is that the host *cannot* evict the
renter — the host signs a contract, posts collateral, and gets
slashed if they drop data. Operator-side per-actor eviction is the
threat model they sell against.

CIRIS makes the opposite call because it's a **federation of
mutually-attesting peers**, not a paid marketplace. Trust changes
over time (a peer slashed today should not have their content
held indefinitely); the substrate's authority to evict is exactly
what makes federation governance enforceable at the storage layer.

### 9.4 Why anonymous content-addressing (IPFS, Freenet) hits the wall

The well-documented scaling pains in those systems are exactly the
failures this property prevents:

* **IPFS pin-set bloat** — no popularity-or-trust signal to drive
  eviction; pinning services manually curate at the operator level
  outside the protocol
* **Freenet inability to handle abuse** — by-design anonymity means
  operators legally hold opaque content with no surface to refuse
  specific actors
* **IPFS Cluster's "untrusted peers lying about free space"** —
  resource attestation has no identity-tied recourse

These are the failure modes CIRIS's `holds_bytes` +
`attesting_key_id` + admission gate forecloses *structurally*. The
substrate doesn't need a curation layer above it; the substrate
itself is curatable because every byte carries its provenance.

### 9.5 Privacy trade-off (intentional, explicit)

The cost: the holder graph is observable. Peers can query "who's
holding content from author X?" via `list_holders` against the
public `federation_attestations` rows. This is the design's
privacy-vs-trust trade-off:

* IPFS / Freenet / Tor: **anonymity-preserving**, abuse-impossible-
  to-handle, scaling pains from indiscriminate replication
* CIRIS: **identity-aware**, trust-enforceable, governable, scaling
  works precisely because admission is selective

For content that needs anonymity (private deliberation, family-scope
content), the CEG locality dividend (§1.3) is the answer: those
cohort scopes never emit `holds_bytes` advertisements, never become
discoverable, never enter the identity-aware substrate at all.
Anonymity-preserving content stays self-hosted; trust-aware content
rides the federation.

### 9.6 Summary

| Property | CIRIS substrate guarantee | Achieved by |
|---|---|---|
| Identity-aware at the byte level | Yes | `put_blob_signing` atomic commit |
| Per-actor eviction granularity | Yes | `federation_attestations` index on `attesting_key_id` + `withdraws` primitive |
| Operator authority to evict | Yes | Local config consuming `scores` trust graph |
| Anonymity for sensitive content | Yes (via opt-out) | `cohort_scope ∈ {self, family}` blocks `holds_bytes` emission |
| Per-byte attribution at app layer | NO | Substrate property, not app concern |

This is the load-bearing property. Without it, the trust × capacity
intake + popularity × freshness eviction discipline collapses to
"LRU on opaque blobs" — which is exactly what IPFS does, and
exactly the regime the scaling model says doesn't work at
full-internet scale.

Sources for the prior-art comparison (§9.2):

- [IPFS Kubo garbage collection](https://docs.ipfs.tech/how-to/kubo-garbage-collection/)
- [IPFS Cluster allocator](https://github.com/ipfs-cluster/ipfs-cluster/blob/master/allocate.go)
- [Veilid cryptography](https://veilid.com/how-it-works/cryptography/)
- [Hypercore DEP-0002](https://www.datprotocol.com/deps/0002-hypercore/)
- [Storj v3 whitepaper](https://static.storj.io/storjv3.pdf)
- [Filecoin Storage Market spec](https://spec.filecoin.io/systems/filecoin_markets/storage_market/)
- [Sia hosting best practices](https://sia.tech/hosting-best-practices)
- [Tahoe-LAFS Accounting design](https://tahoe-lafs.org/trac/tahoe-lafs/wiki/NewAccountingDesign)
- [Mastodon ActivityPub federation](https://docs.joinmastodon.org/spec/activitypub/)
- [Scuttlebutt protocol guide](https://ssbc.github.io/scuttlebutt-protocol-guide/)
- [Freenet paper (Clarke et al.)](https://www.cs.cornell.edu/people/egs/615/freenet.pdf)
- [Tor intro spec](https://spec.torproject.org/intro/index.html)
