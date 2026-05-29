# Federation Scaling Model

**Status:** model v0.1 — empirical inputs measured (Verify v2.7.0/v2.8.0
ubuntu-latest CI); Edge v0.10.0 targets carry forward to v1.0; tier
definitions are the v1.0 design Edge is holding the release for.
**Companion:** `examples/scale_model.rs` runs the model on preset
scenarios + accepts overrides — `cargo run --example scale_model`.

This is the model we use to answer "what does it take to replace
Twitter / Facebook / news at the CIRIS substrate?". The question
sounds large; the empirical inputs make it small. Most data is local;
replication only happens for sources you trust (or trust to be
trusted). The toy below lets us play `what-if` on tier mix, trust
radius, and cohort distribution and see storage / bandwidth / compute
roll up.

---

## 1. Empirical inputs (measured)

All numbers are wall-clock on GitHub Actions `ubuntu-latest` runners
unless noted — the conservative-by-design baseline. Dev-host CPUs run
2–3× faster.

### 1.1 Crypto (CIRISVerify v2.8.0)

| Op | Cost |
|---|---|
| `hybrid_sign` (Ed25519 + ML-DSA-65) | **466 µs** (~2.15 K sign/s/core) |
| `hybrid_verify` | **276 µs** (~3.62 K verify/s/core) |
| `aes_gcm_encrypt` / 256 B | 428 ns (570 MiB/s) |
| `aes_gcm_encrypt` / 64 KiB | 11.2 µs (**5.45 GiB/s**) |
| `aes_gcm_decrypt` / 64 KiB | 10.3 µs (**5.91 GiB/s**) |
| `hkdf_sha256` | 548 ns |
| `hmac_sha256` | 242 ns |
| Merkle `root` (any N) | 19.7 ns (O(1)) |
| Merkle `append` @ 64K leaves | ~1.33 µs (750 K appends/s) |
| Merkle `inclusion_proof` @ 64K | 171 ns |
| Merkle `verify_inclusion` @ 64K | 1.99 µs |

### 1.2 Edge (v0.10.0 targets → v1.0 contract)

| Op | Target |
|---|---|
| `envelope_canonicalize` (slope) | ~250 ns/KiB |
| `envelope_verify` (single) | ~280 µs (verify-dominated) |
| `dispatch_inbound` (256 B) | < 400 µs → ~2.5 K msg/s/thread |
| `outbound_enqueue Ephemeral` | < 600 µs (sign-dominated) |
| `outbound_enqueue Durable` | < 1.5 ms (+1 SQLite row) |
| `content_fetch_roundtrip` 4 KiB | < 2 ms |
| `content_fetch_roundtrip` 64 KiB | < 30 ms |
| `content_fetch_roundtrip` 1 MiB | < 500 ms |
| `transport_reticulum_loopback` 256 B | < 500 µs |
| `transport_http_loopback` 1 KiB | < 1 ms |
| `subscription_throughput` (1 sub) | > 50 K events/sec |

### 1.3 Persist (v3.1.1)

| Op | Cost |
|---|---|
| SQLite per-row write (incl. async wrapper) | ~1.5 ms |
| Ingest pipeline @ 768 rows (release) | ~9 ms → ~85 K rows/s |
| Software signer (no HW backing) | ~100 µs/sign |
| Hardware signer (TPM/Keystore) | ~30 µs/sign |
| NATS JetStream comparator | ~100 µs/row (15× faster — different durability story) |

### 1.4 Comparison anchors (web-scale baselines)

| Service | Volume / day | Per-user / day | Notes |
|---|---|---|---|
| Twitter | ~500 M tweets, ~70 GB raw | ~0.5 tweet × 140 B ≈ **70 B raw / 1 KB w/ metadata** | Avg active user |
| Facebook | ~4 PB new content | **~2 MB / user / day** | ~2 B users, all media |
| News (major publisher) | ~10 K articles, ~100 MB | (~1 publisher, not per user) | ~10 KB avg article |
| Wikipedia | ~7 M EN articles, ~80 GB compressed | (static corpus) | ~11 KB compressed avg |

---

## 2. Topology — v1.0 tier model

Edge v1.0 ships a global `agent_mode` switch. Per Eric:

| Tier | Role | Disk gate | Behavior |
|---|---|---|---|
| **client** | Personal device only | none | No inbound serving. No replication. Outbound own contributions; fetches via proxy. Phone / tablet / low-end laptop. |
| **proxy** | **Default tier** | none | Caches transit blobs (best-effort, LRU). Responds to ContentFetch from cache. Forwards on miss. Laptop / desktop. |
| **server** | Full federation node | **≥ 256 GB** | Long-term replicates trust-set content per cohort overlap. Responds to ContentFetch from full archive. Home server / VPS. |

Tier mix is a **deployment-population** parameter, not a per-user
choice mid-session. The model parameterizes federation behavior as
`(α_client, α_proxy, α_server)` where the three sum to 1.

---

## 3. Replication semantics

Every Contribution carries a `cohort_scope`. Replication policy is
the join of (host tier, host trust set, cohort scope):

**Tier behavior:**
- **client** — replicates *nothing* inbound; fetches own data only.
- **proxy** — caches transit, no long-term replication. Bounded by
  `proxy_cache_bytes` (deployment-config; default model: 4 GiB LRU).
- **server** — long-term replicates `T(host)`'s contributions where
  `cohort_scope ≥ community` (i.e. anything publishable). Personal-
  scope content (`self` / `family`) is NEVER replicated by anyone
  else — that's the discipline that makes "most data is local"
  arithmetically true.

**Trust set `T(host)`:** the directly-trusted peers — typically
Dunbar-shaped, default model `R = 150`. Server tier additionally
replicates the second-order set (`T(T(host))`) at a discount factor
`δ = 0.1` (10% of second-order content because most isn't relevant
to my circle). Higher orders are not pre-replicated; fetched on
demand via ContentFetch.

**Cohort scope distribution `σ`:** what fraction of a typical
user's daily activity lands at each scope. Default model:

| Scope | Fraction | Replicated by |
|---|---|---|
| `self` (private notes / drafts) | 0.50 | nobody — local only |
| `family` | 0.15 | nobody outside family — local + family-trusted only |
| `community` | 0.15 | server-tier members of community |
| `affiliations` (work / projects) | 0.10 | server-tier members of affiliations |
| `species` (broadcast-to-humans) | 0.05 | every server in federation that trusts the author transitively |
| `planet` (environmental data, multispecies) | 0.03 | same |
| `federation` (governance, P1-P11 primitives) | 0.02 | every server in federation (gossiped via FederationAnnouncement) |

The "most data is local" property holds because **65% of typical
activity is self+family** — never replicated off-device. The
remaining 35% goes through the trust-radius funnel.

---

## 4. Per-actor cost model

For a user `U` running tier `t ∈ {client, proxy, server}`:

### 4.1 Storage (bytes resident on `U`'s persist)

```
storage(U, t) = own(U) + replicated(U, t) + proxy_cache(t)
```

Where:
- `own(U)` = `U`'s contributions × payload size × retention (T days).
  Default model: `D = 50 KB/day` activity (chat-heavy use); retention
  unbounded for own data, capped at `T = 365` days for archive sizing.
- `replicated(U, server)` = sum over `T(U)`'s server-publishable
  activity × cohort-overlap × T. With default model and `R = 150`:
  `150 peers × 50 KB/day × 0.35 community+ × T`.
  `replicated(U, client | proxy) = 0`.
- `proxy_cache(t)`:
  - client: 0
  - proxy: 4 GiB (LRU; deployment-config)
  - server: equal to or greater than `replicated` (no separate cap)

### 4.2 Bandwidth (bytes/sec on `U`'s link)

```
outbound(U) = D(U) × fanout(t, σ)
inbound(U)  = trust_inbound(U, t) + fetch_inbound(U)
```

Where:
- `fanout(client) = 1` (own → upstream proxy)
- `fanout(proxy) = 1 + transit` (own + relay-through)
- `fanout(server) = 1 + steward_set_size × σ_publishable + transit`
- `trust_inbound(server) = R × D × σ_publishable`
- `trust_inbound(client | proxy) ≈ 0` (proxy fetches on demand only)
- `fetch_inbound(U)` ≈ user's daily browse traffic (model: 100 MB/day
  for media-heavy use)

### 4.3 Compute (CPU-seconds/day on `U`)

```
sign(U)   = D / avg_envelope_size × 466 µs    # hybrid_sign cost
verify(U) = inbound_envelopes × 276 µs        # hybrid_verify cost
canon(U)  = (sign + verify envelopes) × env_size × 250 ns/KiB
```

Modeling notes:
- Inbound envelope count for `server`: every replicated contribution
  + every ContentFetch hit. The verify path dominates server CPU.
- `dispatch_inbound` adds ~120 µs per envelope (canonicalize +
  replay-window + handler) on top of the 276 µs verify.

---

## 5. Federation rollup

Total federation storage:
```
S_fed = N × Σ_t α_t × storage(U, t)
```

Server tier dominates the storage rollup; client+proxy contribute
own-data only. The "trust radius" knob is what bounds server
storage — bigger `R` means more inbound replication per server.

Total federation bandwidth:
```
B_fed = N × Σ_t α_t × (outbound + inbound)(U, t)
```

The replication multiplier appears here as `α_server × R × D`
(server inbound is the dominant term).

---

## 6. Why this scales (the asymmetry)

The CIRIS substrate is NOT trying to be a global CDN. Twitter /
Facebook serve every user from every datacenter. CIRIS serves:

1. **Your own content from your own device** (always; tier-independent).
2. **Trusted peers' published content from your trust-set's servers**
   (server tier only; bounded by `R`, not by `N`).
3. **Anything else, on demand**, via ContentFetch through proxies and
   servers along the discovery path.

**The federation-wide replication factor for any single piece of
content is bounded by trust-graph reach, NOT by `N`.** A community-
scoped contribution replicates to the union of `T⁻¹(author)` server-
tier members — typically < 10⁴ servers even at `N = 10⁹`. A
federation-scoped (P1-P11 governance) contribution replicates more
broadly, but those are also rare (`σ_federation = 0.02`).

**Compare to Twitter:** a viral tweet hits every datacenter and rides
~5K-fanout pubsub. CIRIS doesn't have viral as a primitive; "viral
content" emerges only through the trust graph re-citing it
(re-quoting the SHA in their own signed payloads), which creates
new attestations that themselves only propagate via trust links.
Virality is rate-limited by the trust-fabric topology — by design.

---

## 7. Preset scenarios (run via `examples/scale_model.rs`)

The companion binary models five reference scenarios:

| Scenario | N | (α_client, α_proxy, α_server) | R | D / user | Notes |
|---|---|---|---|---|---|
| `bootstrap` | 10⁴ | (0.30, 0.65, 0.05) | 50 | 20 KB/d | Early federation, light activity |
| `dunbar_steady` | 10⁶ | (0.40, 0.55, 0.05) | 150 | 50 KB/d | "Normal" tier mix |
| `media_heavy` | 10⁶ | (0.30, 0.60, 0.10) | 150 | 500 KB/d (incl. blobs) | Facebook-style activity |
| `twitter_scale` | 10⁹ | (0.45, 0.50, 0.05) | 150 | 5 KB/d | Tweet-sized contributions, planetary |
| `news_replacement` | 10⁹ | (0.40, 0.55, 0.05) | 300 | 100 KB/d | Wider trust nets, longer articles |

Each scenario prints:
- Storage per tier (per-user × tier proportion × N)
- Bandwidth per tier (avg + peak estimate)
- Compute per server tier (verify ops/sec, sign ops/sec)
- Federation totals
- Comparison ratio against Twitter / Facebook anchors

---

## 8. What this model is NOT

- **Not a capacity guarantee.** Bench numbers are CI ubuntu-latest;
  production hardware varies 2-10×.
- **Not a network simulator.** We model steady-state averages, not
  congestion, packet loss, or Reticulum reachability dynamics.
- **Not a privacy model.** "Most data is local" is enforced by
  cohort_scope discipline + tier behavior, both modeled here, but
  the privacy guarantees themselves are in `MISSION.md §1.6` +
  `THREAT_MODEL.md` — this doc costs them, doesn't certify them.

The model is a planning tool. Run scenarios, see where the knobs
matter, calibrate deployment guidance against measured workloads as
real federation data accumulates.
