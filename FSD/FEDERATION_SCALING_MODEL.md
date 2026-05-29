# Federation Scaling Model

**Status:** model v0.2 — fetch-on-demand load-bearing; full-internet
scenario fits per-server gates.
**Empirical inputs:** Verify v2.7.0 / v2.8.0 measured on
ubuntu-latest CI; Edge v0.10.0 targets carrying into v1.0; Persist
v3.1.1 storage floor.
**Companion:** `examples/scale_model.rs` — `cargo run --example
scale_model`.

This is a design-search tool. The question is not "how big can it
get" — it's "what caching + replication policy makes CIRIS substrate
carry the entire internet from day one on commodity hardware?" The
toy lets you change knobs, see per-server feasibility flip from
✓ to ⚠, and converge on the v1 parameter set.

---

## 1. The CEG locality dividend

The biggest scaling win is structural and comes for free from the
wire format, not from policy or runtime enforcement:

> **`cohort_scope` is a CEG wire field. The substrate refuses to
> replicate Contributions outside their stated cohort. Local data
> stays local because the wire shape will not carry it elsewhere.**

The default cohort distribution (FSD §3) has 65% of activity at
`self` or `family` scope. That 65% is **structurally invisible to
the federation**:
- It never enters any peer's `federation_attestations`.
- It never crosses Edge as a `Contribution{Submit,Replicate}`.
- It never appears in any replication, caching, or discovery path.

The federation pays for the remaining 35%
(`σ_publishable = σ_community + σ_affiliations + σ_species +
σ_planet + σ_federation`). The model accounts for this as a hard
floor — anything `≤ family` does not appear in inter-host bandwidth,
storage, or CPU columns at all. Privacy and scale share the same
load-bearing primitive.

---

## 2. Empirical inputs (measured)

All numbers are wall-clock on GitHub Actions `ubuntu-latest`
runners — the conservative-by-design baseline. Dev-host CPUs run 2–3×
faster.

### 2.1 Crypto (CIRISVerify v2.8.0)

| Op | Cost |
|---|---|
| `hybrid_sign` (Ed25519 + ML-DSA-65) | **466 µs** (~2.15 K sign/s/core) |
| `hybrid_verify` | **276 µs** (~3.62 K verify/s/core) |
| `aes_gcm_encrypt` / 64 KiB | 11.2 µs (**5.45 GiB/s**) |
| `aes_gcm_decrypt` / 64 KiB | 10.3 µs (**5.91 GiB/s**) |
| `hkdf_sha256` | 548 ns |
| Merkle `root` (any N) | 19.7 ns (O(1)) |
| Merkle `append` @ 64K leaves | 1.33 µs (750 K/s) |

### 2.2 Edge (v0.10.0 targets → v1.0 contract)

| Op | Target |
|---|---|
| `envelope_canonicalize` slope | ~250 ns/KiB |
| `envelope_verify` (single) | ~280 µs (verify-dominated) |
| `dispatch_inbound` (256 B) | < 400 µs → ~2.5 K msg/s/thread |
| `outbound_enqueue Durable` | < 1.5 ms (incl. SQLite row) |
| `content_fetch_roundtrip` 4 KiB | < 2 ms |
| `content_fetch_roundtrip` 1 MiB | < 500 ms |
| `inline_text_pipeline` Classify+Scrub | **5–10 ns/byte** |

### 2.3 Persist (v3.1.1)

| Op | Cost |
|---|---|
| SQLite per-row write (incl. async wrapper) | ~1.5 ms |
| Ingest pipeline @ 768 rows (release) | ~9 ms (~85 K rows/s) |
| Software signer | ~100 µs/sign |
| Hardware signer (TPM) | ~30 µs/sign |
| H3ERE trace per agent decision | ~14 KB (≈14 components × 1 KB) |

### 2.4 Comparison anchors

| Service | Volume / day | Per-user / day |
|---|---|---|
| Twitter | ~500 M tweets / ~70 GB raw | ~70 B raw |
| Facebook | ~4 PB | ~2 MB |
| Wikipedia | ~7 M EN articles / ~80 GB compressed | (static corpus) |
| **Modeled `full_internet`** | ~250 PB/day | ~50 MB |

---

## 3. v1 tier model

| Tier | Disk gate | Behavior |
|---|---|---|
| **client** | none | No inbound serving. No cache. Own contributions + own traces. Phone / tablet. |
| **proxy (default)** | none | LRU-bounded encrypted cache (TTL after last access). Serves cache hits; forwards misses. Laptop / desktop. |
| **server** | **≥ 1 TB** | Direct-trust archive (R first-order × σ_publishable × T_direct), encrypted cache, replicated publishable agent traces. Home server / VPS. |

**Feasibility budgets per server (the gates the model checks):**

| Resource | Per-server gate | Source |
|---|---|---|
| Disk | 1 TB | Eric's spec, this session |
| Bandwidth | 1 Gbps (10.8 TB/day sustained) | Residential fiber |
| CPU | 1 full-utilization core (86.4 K cpu-sec/day) | Per-process CIRIS share |

---

## 4. v1 caching + replication policy (load-bearing)

### 4.1 Replication

- **`self` / `family` scope: never replicates** — CEG wire-format
  guarantee, §1.
- **`community` / `affiliations` / `species` / `planet` /
  `federation` scope** (σ_publishable, default 35%): subject to the
  policy below.
- **Direct-trust pre-replication only.** Server tier pre-stores
  publishable content from R first-order trusted peers for
  `direct_trust_archive_days` (default v1: 30 d at internet scale).
- **No second-order pre-replication.** R²-scaled replication is the
  cliff the v0.1 model fell off; v1 sets it to zero. Second-order
  content is fetched on demand via discovery + ContentFetch.

### 4.2 Caching

- **Fetch-on-demand is the primary inbound path.** A user (any tier)
  asking for content not in local archive issues `ContentFetch`.
- **TTL after last access.** Cache holds an item for
  `cache_ttl_minutes` after the most recent serve. Continuous demand
  resets the timer; hot content stays warm until LRU evicts.
- **LRU bound:** `server_cache_max_bytes`. Cache never exceeds this
  even if everything is hot.
- **Encrypted at rest.** AES-256-GCM via persist's `ring` backend
  (5.45 GiB/s write, 5.91 GiB/s read — essentially free).
- **Encrypted in transit.** Already true via Edge's
  `InlineText`-style pipeline; cached bytes never cross the wire
  in cleartext.
- **Cache hit rate** is an assumption now (default 60% for
  trust-graph topologies where interest is locally clustered);
  measured later from real deployments.

### 4.3 Agent traces

The H3ERE pipeline produces ~14 KB of trace components per agent
decision (per CIRISPersist `INTEGRATION_LENS.md`). Traces are:

- **Scrubbed before storage.** PII / secret redaction via the
  Classify + Scrub pipeline (10 ns/byte — negligible at any scale).
- **Stored locally.** Own traces × `trace_retention_days`.
- **Replicated to direct trust only at `trace_publishable_fraction`.**
  Most agent traces are personal deliberation (drafts, planning,
  private reasoning) and stay at `self` scope. The minority that
  cross to community/affiliations decisions (governance,
  collaborative work) are replicated via the same direct-trust
  archive as content. Default: 10% publishable.

---

## 5. Per-actor cost model

For tier `t ∈ {client, proxy, server}` with scenario params:

```
storage(t)   = own + direct_trust_archive(t) + cache(t) + traces(t)
bandwidth_in = fetch_misses + replicated_direct_trust + replicated_traces
bandwidth_out= D × fanout(σ)
sign_ops/d   = (D + trace_bytes) / env_size
verify_ops/d = direct_trust_envs + trace_envs + cache_miss_envs
cpu_sec/d    = sign + verify + dispatch + canon + scrub + encrypt + decrypt
```

Where the v1 model collapses prior cost cliffs:

```
direct_trust_archive(server) = R × D × σ_publishable × T_direct
direct_trust_archive(client | proxy) = 0
cache(t) = min(cache_max, daily_fetch × (TTL_min / 1440) × cache_factor)
traces(t) = own_traces + (direct_trust × trace_publishable × R if server)
```

`σ_local_only × D × T` never appears anywhere except `own` — the CEG
locality dividend.

---

## 6. v1 scenarios

The companion binary runs ten scenarios. Each prints a per-server
feasibility report against the three gates.

| Scenario | N | Tier (c/p/s) | R | D/user | σ_pub | T_direct | Feasible? |
|---|---|---|---|---|---|---|---|
| `bootstrap` | 10⁴ | (30/65/5) | 50 | 20 KB | 35% | 365 d | ✓ |
| `dunbar_steady` | 10⁶ | (40/55/5) | 150 | 50 KB | 35% | 365 d | ✓ |
| `media_heavy` | 10⁶ | (30/60/10) | 150 | 500 KB | 35% | 365 d | ✓ |
| `twitter_scale` | 10⁹ | (45/50/5) | 150 | 5 KB | 35% | 365 d | ✓ |
| `news_replacement` | 10⁹ | (40/55/5) | 300 | 100 KB | 35% | 365 d | ✓ |
| **`full_internet_v1`** | **5×10⁹** | **(35/55/10)** | **250** | **50 MB** | **35%** | **30 d** | **✓ 332 GB / 5 GB/d / 43 s/d** |
| `full_internet_local_heavy` | 5×10⁹ | (35/55/10) | 250 | 50 MB | 30% | 90 d | ✓ 521 GB / 4 GB/d / 36 s/d |
| `full_internet_global_heavy` | 5×10⁹ | (30/55/15) | 250 | 50 MB | 60% | 14 d | ✓ 330 GB / 9 GB/d / 73 s/d |
| `village_dense` | 10³ | (40/40/20) | 50 | 30 MB | 30% | 730 d | ✓ 433 GB / 0.6 GB/d / 27 s/d |
| `full_internet_stretch` | 5×10⁹ | (35/55/10) | 250 | 50 MB | 35% | 365 d | ⚠ 1.72 TB (>1 TB gate) |

### 6.1 `full_internet_v1` — the v1 target (default cohort)

5 B users, 50 MB/user/day across all UGC content forms
(text + photos + short clips; excludes long-form video streaming,
which rides external_ref blob pointers to S3-class stores).

Per-server load:
- Storage 332 GB (32% of 1 TB) — 178 GB own + 128 GB direct-trust
  archive + 25 GB agent traces + 128 MB cache
- Bandwidth 5.15 GB/day (~62 KB/sec)
- CPU 43 sec/day (0.05% of 1 core)

Replicates to 500 M servers globally — roughly one server per
ten humans, which is the order of magnitude where today's home
internet / IoT densities already sit.

### 6.2 `full_internet_local_heavy` — the natural human shape

Same 5 B users with the cohort distribution that matches how human
attention actually clusters (Robin-Dunbar — tight family / community
trust, light global). σ_publishable drops from 35% → 30%, but the
real win is that we can afford a 90-day direct-trust archive (3× v1)
because the publishable slice is smaller AND lighter on wide-scope
content (3% vs 10% on species/planet/federation).

Per-server: 521 GB / 4 GB/day / 36 sec/day. Cache hit rate climbs to
75% (tight communities of interest cluster).

### 6.3 `full_internet_global_heavy` — the governance / OSS shape

The other extreme: federation governance regulars, open-source
maintainers, scientific collaborators. σ_publishable jumps to 60%,
forcing the direct-trust archive down to 14 days. Server tier mix
bumps to 15% to absorb the bandwidth + CPU load.

Per-server: 330 GB / 9 GB/day / 73 sec/day — still fits, just
working harder. This is the demanding case the model says we CAN
serve, but it costs more servers.

### 6.4 `village_dense` — the Pi-class anchor

1 000-person village, R = 50, σ_publishable 30% (local-heavy). Tight
community = high cache locality (85% hit rate), low fanout, slow
content cycling. The model says a Pi-class home server runs a
**730-day** direct-trust archive (433 GB) for the entire village on
~7 KB/sec sustained bandwidth. This is what "the substrate is
deployable everywhere, including small communities, day one" looks
like in numbers.

### 6.5 What broke the stretch scenario

`full_internet_stretch` keeps default σ but pushes
`direct_trust_archive_days` from 30 → 365. Per-server storage hits
1.72 TB. The model flags the dominant cost (direct-trust archive)
and the knob to turn — exactly the design-tool behavior we want.

---

## 6.6 Cohort distribution as a scaling lever

The three cohort shapes the model carries:

| Distribution | self+family | publishable | wide-global | Storage shape |
|---|---|---|---|---|
| `default` | 65% | 35% | 10% | Balanced |
| `local_heavy` | 70% | 30% | 3% | Most favorable — tight trust |
| `global_heavy` | 40% | 60% | 25% | Hardest — wide fanout |

A 5%-point shift in `σ_publishable` changes server-tier storage
linearly: at R=250, D=50 MB, T=30 d, every 1% of σ_publishable is
~3.6 GB of direct-trust archive. **The cohort distribution chosen by
the population is the dominant scaling lever after R.** Public
narratives about "go global, share everywhere" are scaling
*adversarial*; "tight communities of interest" is scaling
*favorable*. CIRIS's design pushes the population toward
locality because the wire format makes it the path of least
resistance, not because policy enforces it.

### 6.1 `full_internet_v1` — the v1 target

5 B users, 50 MB/user/day across all UGC content forms
(text + photos + short clips; excludes long-form video streaming,
which rides external_ref blob pointers to S3-class stores).

Per-server load:
- Storage 332 GB (32% of 1 TB) — 178 GB own + 128 GB direct-trust
  archive + 25 GB agent traces + 128 MB cache
- Bandwidth 5.15 GB/day (~62 KB/sec)
- CPU 43 sec/day (0.05% of 1 core)

Replicates to 500 M servers globally — roughly one server per
ten humans, which is the order of magnitude where today's home
internet / IoT densities already sit.

### 6.2 What broke the stretch scenario

Bumping `direct_trust_archive_days` from 30 → 365 pushes per-server
storage to 1.72 TB. The model flags the dominant cost (direct-trust
archive) and the knob to turn (`direct_trust_archive_days` or
`trust_radius`). This is the model working as a design tool — it
exposes the trade-off curve without anyone having to argue about
it from intuition.

---

## 7. Why the three resources behave so differently

**Compute is free at every scale.** Even at 5 B users with 200 agent
decisions/day each, federation-aggregate CPU at 5% utilization sits
at ~11 M cores — ~0.02 core/server across 500 M servers. The hybrid
PQC verify (276 µs) does not bend the curve.

**Bandwidth is the second-cheapest resource.** Server-tier inbound at
`full_internet_v1` is 5 GB/day per server (~62 KB/sec sustained,
~0.05% of 1 Gbps). The "billion small pipes" topology pays
residential broadband prices, not datacenter-egress prices.

**Storage is the resource that bites — and only one knob bends it.**
The `direct_trust_archive_days × trust_radius × σ_publishable × D`
term is what occupies most of the 1 TB server budget. v1 controls it
by capping the pre-replicated window (30 d at internet scale; the
rest is fetch-on-demand). The model's job is to surface that knob
honestly.

---

## 8. What this model is NOT

- **Not a capacity guarantee.** Bench numbers are CI ubuntu-latest;
  production hardware varies 2–10×.
- **Not a network simulator.** Steady-state averages; not
  congestion, packet loss, or Reticulum reachability dynamics.
- **Not a privacy certifier.** Locality is enforced by CEG wire
  shape + cohort_scope discipline + tier behavior, all modeled here;
  the guarantees themselves are in `MISSION.md §1.6` +
  `THREAT_MODEL.md`. This doc costs them; it doesn't certify them.
- **Not a final answer.** Cache hit rate is an assumption pending
  measurement. Second-order fetch-on-demand latency / availability
  needs Reticulum-network simulation. Agent decision rate at
  population scale is a guess.

The model is a planning tool. Run scenarios, see where the knobs
matter, calibrate against real federation data as it accumulates.
