//! Federation scaling model — companion to `FSD/FEDERATION_SCALING_MODEL.md`.
//!
//! Run: `cargo run --example scale_model`
//!
//! **Purpose:** find the v1 caching / replication parameters that
//! let CIRIS substrate carry the entire internet from day one. This
//! is a design search tool — change scenario knobs, watch per-server
//! feasibility checks pass or fail, converge on the parameter set
//! that makes 5B users work on commodity hardware.
//!
//! **Load-bearing v1 assumptions** (per Eric, this session):
//!
//! 1. **Fetch-on-demand is primary.** Server tier does NOT
//!    pre-replicate trust-set content broadly. Inbound content
//!    flows on `ContentFetch` only.
//! 2. **Direct-trust archive is the one pre-replicated set.**
//!    Server tier holds R first-order trusted peers' publishable
//!    content for `T_direct` days. R² second-order content is
//!    fetched on demand, NEVER pre-replicated.
//! 3. **Cache holds content for X minutes after last access** —
//!    `cache_ttl_minutes`. Encrypted at rest (AES-GCM), encrypted in
//!    transit (already true per Edge `InlineText`). LRU eviction
//!    bounded by `server_cache_max_bytes`.
//! 4. **Agent traces are first-class.** H3ERE pipeline produces ~14
//!    trace components per agent decision. Traces are scrubbed
//!    (PII / secret redaction) before storage; most stay local
//!    (`trace_publishable_fraction` controls what crosses to direct
//!    trust).
//!
//! **Feasibility = 1 TB disk + residential 1 Gbps + 1 core full-util
//! per server.** A scenario passes if average server-tier load stays
//! within all three. Failing scenarios print which knob to turn.
//!
//! Empirical constants baked in from CIRISVerify v2.8.0 + CIRISEdge
//! v0.10.0 + CIRISPersist v3.1.1 — see FSD §1.

// ─── Empirical constants ──────────────────────────────────────────────

/// Hybrid Ed25519 + ML-DSA-65 sign (CIRISVerify v2.8.0).
const HYBRID_SIGN_US: f64 = 466.0;

/// Hybrid verify (CIRISVerify v2.8.0).
const HYBRID_VERIFY_US: f64 = 276.0;

/// `dispatch_inbound` overhead on top of verify (Edge v0.10.0).
const DISPATCH_OVERHEAD_US: f64 = 120.0;

/// Canonicalization slope (Edge v0.10.0).
const CANONICALIZE_NS_PER_KIB: f64 = 250.0;

/// AES-GCM encrypt @ 64 KiB blocks — 5.45 GiB/s (CIRISVerify v2.8.0
/// `ring` backend). Used for cache-at-rest + transit encryption above
/// the already-encrypted inline text pipeline.
const AES_GCM_ENCRYPT_NS_PER_BYTE: f64 = 0.175; // 5.45 GiB/s

/// AES-GCM decrypt @ 64 KiB blocks — 5.91 GiB/s.
const AES_GCM_DECRYPT_NS_PER_BYTE: f64 = 0.161; // 5.91 GiB/s

/// Scrub regex pass (PII / secret redaction). CIRISEdge BENCHMARKS.md
/// says Classify/Scrub passes are 5–10 ns/byte; we use 10 ns/byte
/// (conservative; covers both classify + scrub + AES-GCM in one).
const SCRUB_NS_PER_BYTE: f64 = 10.0;

/// H3ERE pipeline trace bytes per agent decision. Persist
/// INTEGRATION_LENS: agent batch_size = 10 × ~14 components ≈ 14 KB
/// per agent decision (one "thought-then-act" cycle).
const H3ERE_TRACE_BYTES_PER_DECISION: f64 = 14.0 * KB;

// ─── Tier model (Edge v1.0 design) ───────────────────────────────────

/// Server-tier disk gate (the per-server budget the model checks
/// every scenario's avg-server-storage against).
const SERVER_DISK_GATE_BYTES: f64 = 1024.0 * GIB;

/// Per-server bandwidth budget — 1 Gbps residential fiber ≈ 125 MB/s
/// = 10.8 TB/day sustained. Real deployments share this with browsing
/// + everything else; the model treats this as the server's CIRIS
/// budget, not the total link.
const SERVER_BANDWIDTH_GATE_BYTES_PER_DAY: f64 = 10.8 * TB;

/// Per-server CPU budget — 1 core full utilization = 86400 cpu-sec/day.
/// Real servers have 4–16 cores; this is the per-process CIRIS share.
const SERVER_CPU_GATE_SECONDS_PER_DAY: f64 = 86_400.0;

const KB: f64 = 1024.0;
const MB: f64 = 1024.0 * KB;
const GB: f64 = 1024.0 * MB;
const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
const TB: f64 = 1024.0 * GB;
const PB: f64 = 1024.0 * TB;
const EB: f64 = 1024.0 * PB;

#[derive(Debug, Clone, Copy)]
enum Tier {
    Client,
    Proxy,
    Server,
}

/// Cohort scope distribution — what fraction of a user's daily
/// activity lands at each scope. Sums to 1.0.
#[derive(Debug, Clone, Copy)]
struct CohortDist {
    self_: f64,
    family: f64,
    community: f64,
    affiliations: f64,
    species: f64,
    planet: f64,
    federation: f64,
}

impl CohortDist {
    fn publishable(&self) -> f64 {
        self.community + self.affiliations + self.species + self.planet + self.federation
    }
    fn local_only(&self) -> f64 {
        self.self_ + self.family
    }
    fn default_model() -> Self {
        Self {
            self_: 0.50, family: 0.15, community: 0.15, affiliations: 0.10,
            species: 0.05, planet: 0.03, federation: 0.02,
        }
    }

    /// Heavy-local / light-global. Most activity stays within
    /// family + community trust-set; very little crosses to
    /// species/planet/federation scopes. This is the small-town /
    /// tight-knit-community shape — and arithmetically the most
    /// favorable for federation scaling because narrow-scope content
    /// has fanout 4 vs wide-scope's 64.
    ///
    /// Local-only: 70%   Publishable: 30%   Wide-global: only 3%
    fn local_heavy() -> Self {
        Self {
            self_: 0.45, family: 0.25, community: 0.20, affiliations: 0.07,
            species: 0.02, planet: 0.005, federation: 0.005,
        }
    }

    /// Light-local / heavy-global. The "global commons" shape —
    /// open-source maintainers, federation governance regulars,
    /// scientific collaborators. Less personal/family, more
    /// species/planet/federation. The hardest case for scaling
    /// because wide-scope fanout dominates outbound bandwidth.
    ///
    /// Local-only: 40%   Publishable: 60%   Wide-global: 25%
    fn global_heavy() -> Self {
        Self {
            self_: 0.30, family: 0.10, community: 0.15, affiliations: 0.20,
            species: 0.10, planet: 0.08, federation: 0.07,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TierMix {
    client: f64,
    proxy: f64,
    server: f64,
}

#[derive(Debug, Clone)]
struct Scenario {
    name: &'static str,
    n_users: f64,
    tier_mix: TierMix,
    /// Average direct trust set size per user.
    trust_radius: f64,
    /// Average per-user daily activity volume (excluding agent traces).
    daily_bytes: f64,
    /// Average envelope size — sets sign/verify call count per byte.
    avg_envelope_bytes: f64,
    /// Own-data retention (days). Personal archive — typically long.
    own_retention_days: f64,
    /// Direct-trust archive retention (days). Server tier pre-stores
    /// R first-order trusted peers' publishable content for this
    /// window. Beyond it, fetch-on-demand.
    direct_trust_archive_days: f64,
    cohort: CohortDist,
    /// Average daily ContentFetch traffic a user pulls (browse).
    daily_fetch_bytes: f64,
    /// Cache TTL — server keeps fetched content for this many minutes
    /// after last access. Hot content (continuous demand) stays
    /// cached indefinitely until LRU evicts.
    cache_ttl_minutes: f64,
    /// Cache cap — LRU evicts past this regardless of TTL.
    server_cache_max_bytes: f64,
    /// Fraction of fetches served from local cache (saves a network
    /// hop + a verify).
    cache_hit_rate: f64,
    /// Agent decisions per user per day. 0 = no agent.
    agent_decisions_per_day: f64,
    /// Fraction of agent traces that cross to direct-trust archive
    /// (most are personal-scope deliberations; only collaborative /
    /// community decisions cross).
    trace_publishable_fraction: f64,
    /// Agent trace retention (days).
    trace_retention_days: f64,
}

// ─── Per-actor formulae ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct ActorCosts {
    storage_own: f64,
    storage_direct_trust_archive: f64,
    storage_cache: f64,
    storage_traces: f64,
    storage_total: f64,
    bandwidth_out_per_day: f64,
    bandwidth_in_per_day: f64,
    sign_ops_per_day: f64,
    verify_ops_per_day: f64,
    cpu_seconds_per_day: f64,
}

fn per_actor(tier: Tier, s: &Scenario) -> ActorCosts {
    let trace_bytes_per_day = s.agent_decisions_per_day * H3ERE_TRACE_BYTES_PER_DECISION;
    let envs_per_day = s.daily_bytes / s.avg_envelope_bytes;
    let trace_envs_per_day = trace_bytes_per_day / s.avg_envelope_bytes;

    // OWN — every tier stores own contributions + own traces.
    let storage_own = s.daily_bytes * s.own_retention_days;
    let storage_traces_own = trace_bytes_per_day * s.trace_retention_days;
    let sign_ops_own = envs_per_day + trace_envs_per_day;
    let scrub_bytes_own = trace_bytes_per_day; // scrub own traces

    // TIER-DEPENDENT BEHAVIOR.
    let (
        storage_direct_trust,
        storage_cache,
        storage_traces_extra,
        in_bps,
        verify_ops,
        scrub_extra,
        fanout,
    ) = match tier {
        Tier::Client => {
            // Phone / tablet — no inbound serving, no cache. Just
            // own data + own traces. Verifies only what it explicitly
            // fetches.
            let verify_from_fetch = s.daily_fetch_bytes / s.avg_envelope_bytes;
            (0.0, 0.0, 0.0, s.daily_fetch_bytes, verify_from_fetch, 0.0, 1.0)
        }
        Tier::Proxy => {
            // Default tier — cache only, no archive, no agent
            // traces beyond own. The cache holds a bounded LRU of
            // recently-fetched content for cache_ttl_minutes,
            // encrypted at rest.
            //
            // Steady-state cache size approximation:
            //   min(server_cache_max, daily_fetch × (TTL/1440)) × (1 + revisit factor)
            // Cache hit rate amortizes the inbound bandwidth.
            let cache_residency = (s.cache_ttl_minutes / 1440.0).min(1.0);
            let cache_steady = (s.server_cache_max_bytes * 0.25)
                .min(s.daily_fetch_bytes * cache_residency * 2.0);
            // Inbound = (1 - hit_rate) × daily_fetch. Cache absorbs
            // the rest.
            let inbound = s.daily_fetch_bytes * (1.0 - s.cache_hit_rate);
            // Verify only on cache miss + on own fetches.
            let verify = (s.daily_fetch_bytes / s.avg_envelope_bytes) * (1.0 - s.cache_hit_rate);
            // Scrub: proxy doesn't usually own agent traces; assume
            // 0 here (any traces are already own_traces above).
            (0.0, cache_steady, 0.0, inbound, verify, 0.0, 2.0)
        }
        Tier::Server => {
            // Full node — direct-trust archive + cache + replicated
            // publishable agent traces from direct trust.
            //
            // (1) Direct-trust archive: R peers × D × σ_publishable
            //     × T_direct. NO second-order — that's fetch-on-demand.
            let direct_archive_daily = s.trust_radius * s.daily_bytes * s.cohort.publishable();
            let direct_archive = direct_archive_daily * s.direct_trust_archive_days;

            // (2) Cache: same as proxy but bigger budget.
            let cache_residency = (s.cache_ttl_minutes / 1440.0).min(1.0);
            let cache_steady = s.server_cache_max_bytes
                .min(s.daily_fetch_bytes * cache_residency * 3.0);

            // (3) Replicated agent traces from direct trust set
            //     (only publishable-scope deliberations).
            let traces_in_per_day = s.trust_radius
                * trace_bytes_per_day
                * s.trace_publishable_fraction;
            let traces_replicated = traces_in_per_day * s.trace_retention_days;

            // Bandwidth in: replicated direct-trust archive + cache
            // misses + agent trace ingest.
            let cache_miss_inbound = s.daily_fetch_bytes * (1.0 - s.cache_hit_rate);
            let inbound = direct_archive_daily + traces_in_per_day + cache_miss_inbound;

            // Verify ops: every replicated envelope + every cache
            // miss (verify on receive). Re-serves from cache are
            // signature-checked once at admission, not re-verified
            // per serve — that's the cache's job.
            let direct_archive_envs = direct_archive_daily / s.avg_envelope_bytes;
            let traces_envs = traces_in_per_day / s.avg_envelope_bytes;
            let cache_miss_envs = cache_miss_inbound / s.avg_envelope_bytes;
            let verify = direct_archive_envs + traces_envs + cache_miss_envs;

            // Scrub: replicated traces from direct trust (we scrub
            // before storage so PII can't slip through cross-replica).
            let scrub_bytes = traces_in_per_day;

            // Fanout: own × wide-scope steward set + steward stewardship.
            let wide = s.cohort.species + s.cohort.planet + s.cohort.federation;
            let narrow = s.cohort.community + s.cohort.affiliations;
            let fanout = 1.0 + narrow * 4.0 + wide * 64.0;

            (direct_archive, cache_steady, traces_replicated, inbound, verify, scrub_bytes, fanout)
        }
    };

    let outbound_bps = s.daily_bytes * fanout;
    let storage_total = storage_own + storage_direct_trust + storage_cache
        + storage_traces_own + storage_traces_extra;

    // CPU accounting.
    let sign_cpu = sign_ops_own * HYBRID_SIGN_US * 1e-6;
    let verify_cpu = verify_ops * HYBRID_VERIFY_US * 1e-6;
    let dispatch_cpu = verify_ops * DISPATCH_OVERHEAD_US * 1e-6;

    // Canonicalize cost — bytes touched by sign + verify.
    let canon_bytes = (sign_ops_own + verify_ops) * s.avg_envelope_bytes;
    let canon_cpu = (canon_bytes / KB) * CANONICALIZE_NS_PER_KIB * 1e-9;

    // Scrub cost — own traces + replicated traces.
    let scrub_total_bytes = scrub_bytes_own + scrub_extra;
    let scrub_cpu = scrub_total_bytes * SCRUB_NS_PER_BYTE * 1e-9;

    // AES-GCM cost — every cache write + every cache read. Cache
    // turnover ≈ daily_fetch (writes); cache reads ≈ daily_fetch ×
    // hit_rate. Plus outbound traffic encryption (in-transit).
    let cache_write_bytes = match tier {
        Tier::Client => 0.0,
        Tier::Proxy | Tier::Server => s.daily_fetch_bytes,
    };
    let cache_read_bytes = match tier {
        Tier::Client => 0.0,
        Tier::Proxy | Tier::Server => s.daily_fetch_bytes * s.cache_hit_rate,
    };
    let encrypt_cpu = (cache_write_bytes + outbound_bps) * AES_GCM_ENCRYPT_NS_PER_BYTE * 1e-9;
    let decrypt_cpu = (cache_read_bytes + in_bps) * AES_GCM_DECRYPT_NS_PER_BYTE * 1e-9;

    let cpu_total = sign_cpu + verify_cpu + dispatch_cpu + canon_cpu + scrub_cpu
        + encrypt_cpu + decrypt_cpu;

    ActorCosts {
        storage_own,
        storage_direct_trust_archive: storage_direct_trust,
        storage_cache,
        storage_traces: storage_traces_own + storage_traces_extra,
        storage_total,
        bandwidth_out_per_day: outbound_bps,
        bandwidth_in_per_day: in_bps,
        sign_ops_per_day: sign_ops_own,
        verify_ops_per_day: verify_ops,
        cpu_seconds_per_day: cpu_total,
    }
}

// ─── Federation rollup ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct FedRollup {
    total_storage_bytes: f64,
    total_bandwidth_in_bytes_per_day: f64,
    total_bandwidth_out_bytes_per_day: f64,
    total_verify_ops_per_day: f64,
    total_sign_ops_per_day: f64,
    aggregate_cpu_cores_full_util: f64,
    per_tier: [(Tier, ActorCosts); 3],
}

fn rollup(s: &Scenario) -> FedRollup {
    let cli = per_actor(Tier::Client, s);
    let prx = per_actor(Tier::Proxy, s);
    let srv = per_actor(Tier::Server, s);

    let n_cli = s.n_users * s.tier_mix.client;
    let n_prx = s.n_users * s.tier_mix.proxy;
    let n_srv = s.n_users * s.tier_mix.server;

    let total_storage = n_cli * cli.storage_total
        + n_prx * prx.storage_total
        + n_srv * srv.storage_total;
    let total_in = n_cli * cli.bandwidth_in_per_day
        + n_prx * prx.bandwidth_in_per_day
        + n_srv * srv.bandwidth_in_per_day;
    let total_out = n_cli * cli.bandwidth_out_per_day
        + n_prx * prx.bandwidth_out_per_day
        + n_srv * srv.bandwidth_out_per_day;
    let total_verify = n_cli * cli.verify_ops_per_day
        + n_prx * prx.verify_ops_per_day
        + n_srv * srv.verify_ops_per_day;
    let total_sign = n_cli * cli.sign_ops_per_day
        + n_prx * prx.sign_ops_per_day
        + n_srv * srv.sign_ops_per_day;
    let total_cpu_s = n_cli * cli.cpu_seconds_per_day
        + n_prx * prx.cpu_seconds_per_day
        + n_srv * srv.cpu_seconds_per_day;

    FedRollup {
        total_storage_bytes: total_storage,
        total_bandwidth_in_bytes_per_day: total_in,
        total_bandwidth_out_bytes_per_day: total_out,
        total_verify_ops_per_day: total_verify,
        total_sign_ops_per_day: total_sign,
        aggregate_cpu_cores_full_util: total_cpu_s / 86_400.0,
        per_tier: [(Tier::Client, cli), (Tier::Proxy, prx), (Tier::Server, srv)],
    }
}

// ─── Preset scenarios ────────────────────────────────────────────────

fn scenarios() -> Vec<Scenario> {
    let cohort = CohortDist::default_model();

    // V1 design candidates — calibrated to fit per-server gates while
    // representing real workloads. Walk them top to bottom: each
    // scenario adds load + the model surfaces what knobs need to give.
    vec![
        Scenario {
            name: "bootstrap",
            n_users: 10_000.0,
            tier_mix: TierMix { client: 0.30, proxy: 0.65, server: 0.05 },
            trust_radius: 50.0,
            daily_bytes: 20.0 * KB,
            avg_envelope_bytes: 1.5 * KB,
            own_retention_days: 365.0,
            direct_trust_archive_days: 365.0,
            cohort,
            daily_fetch_bytes: 5.0 * MB,
            cache_ttl_minutes: 60.0,
            server_cache_max_bytes: 4.0 * GB,
            cache_hit_rate: 0.30,
            agent_decisions_per_day: 20.0,
            trace_publishable_fraction: 0.15,
            trace_retention_days: 90.0,
        },
        Scenario {
            name: "dunbar_steady",
            n_users: 1_000_000.0,
            tier_mix: TierMix { client: 0.40, proxy: 0.55, server: 0.05 },
            trust_radius: 150.0,
            daily_bytes: 50.0 * KB,
            avg_envelope_bytes: 1.5 * KB,
            own_retention_days: 365.0,
            direct_trust_archive_days: 365.0,
            cohort,
            daily_fetch_bytes: 50.0 * MB,
            cache_ttl_minutes: 60.0,
            server_cache_max_bytes: 32.0 * GB,
            cache_hit_rate: 0.40,
            agent_decisions_per_day: 50.0,
            trace_publishable_fraction: 0.15,
            trace_retention_days: 180.0,
        },
        Scenario {
            name: "media_heavy",
            n_users: 1_000_000.0,
            tier_mix: TierMix { client: 0.30, proxy: 0.60, server: 0.10 },
            trust_radius: 150.0,
            daily_bytes: 500.0 * KB,
            avg_envelope_bytes: 8.0 * KB,
            own_retention_days: 365.0,
            direct_trust_archive_days: 365.0,
            cohort,
            daily_fetch_bytes: 200.0 * MB,
            cache_ttl_minutes: 60.0,
            server_cache_max_bytes: 100.0 * GB,
            cache_hit_rate: 0.50,
            agent_decisions_per_day: 100.0,
            trace_publishable_fraction: 0.15,
            trace_retention_days: 180.0,
        },
        Scenario {
            name: "twitter_scale",
            n_users: 1_000_000_000.0,
            tier_mix: TierMix { client: 0.45, proxy: 0.50, server: 0.05 },
            trust_radius: 150.0,
            daily_bytes: 5.0 * KB,
            avg_envelope_bytes: 0.5 * KB,
            own_retention_days: 365.0,
            direct_trust_archive_days: 365.0,
            cohort,
            daily_fetch_bytes: 20.0 * MB,
            cache_ttl_minutes: 60.0,
            server_cache_max_bytes: 50.0 * GB,
            cache_hit_rate: 0.50,
            agent_decisions_per_day: 30.0,
            trace_publishable_fraction: 0.10,
            trace_retention_days: 180.0,
        },
        Scenario {
            name: "news_replacement",
            n_users: 1_000_000_000.0,
            tier_mix: TierMix { client: 0.40, proxy: 0.55, server: 0.05 },
            trust_radius: 300.0,
            daily_bytes: 100.0 * KB,
            avg_envelope_bytes: 5.0 * KB,
            own_retention_days: 1825.0,
            direct_trust_archive_days: 365.0, // archive 1y, fetch older on demand
            cohort,
            daily_fetch_bytes: 100.0 * MB,
            cache_ttl_minutes: 120.0,
            server_cache_max_bytes: 200.0 * GB,
            cache_hit_rate: 0.50,
            agent_decisions_per_day: 50.0,
            trace_publishable_fraction: 0.15,
            trace_retention_days: 365.0,
        },
        // The v1 target — every human, every UGC content form, every
        // day, day one. Calibrated to fit 1 TB per server.
        //
        // What gives:
        //   - Direct-trust archive shrinks to 30d (rest fetch-on-demand)
        //   - Server cache 200 GB (the hot-set budget)
        //   - Cache hit rate 60% (trust-graph locality of interest)
        //   - Second-order replication: 0 (was 10% in v0.1; that was
        //     the 396-TB-per-server cliff)
        //   - Trace publishable fraction 10% (most agent deliberations
        //     stay personal — privacy AND scale)
        Scenario {
            name: "full_internet_v1",
            n_users: 5_000_000_000.0,
            tier_mix: TierMix { client: 0.35, proxy: 0.55, server: 0.10 },
            trust_radius: 250.0,
            daily_bytes: 50.0 * MB,
            avg_envelope_bytes: 50.0 * KB,
            own_retention_days: 3650.0, // 10y own archive
            direct_trust_archive_days: 30.0, // hot direct-trust only
            cohort,
            daily_fetch_bytes: 1.0 * GB,
            cache_ttl_minutes: 60.0,
            server_cache_max_bytes: 200.0 * GB,
            cache_hit_rate: 0.60,
            agent_decisions_per_day: 200.0,
            trace_publishable_fraction: 0.10,
            trace_retention_days: 365.0,
        },
        // Heavy-local population — same 5B humans but the typical
        // user is tight-knit family + community, light global. This
        // is what most actual human attention looks like
        // (Robin-Dunbar-shaped). The wire-shape locality dividend
        // means very little of this even crosses to direct-trust
        // archive — most of it never enters the federation at all.
        Scenario {
            name: "full_internet_local_heavy",
            n_users: 5_000_000_000.0,
            tier_mix: TierMix { client: 0.35, proxy: 0.55, server: 0.10 },
            trust_radius: 250.0,
            daily_bytes: 50.0 * MB,
            avg_envelope_bytes: 50.0 * KB,
            own_retention_days: 3650.0,
            direct_trust_archive_days: 90.0, // can afford more — less publishable
            cohort: CohortDist::local_heavy(),
            daily_fetch_bytes: 1.0 * GB,
            cache_ttl_minutes: 60.0,
            server_cache_max_bytes: 200.0 * GB,
            cache_hit_rate: 0.75, // tight communities = much higher cache locality
            agent_decisions_per_day: 200.0,
            trace_publishable_fraction: 0.05,
            trace_retention_days: 365.0,
        },
        // Light-local / heavy-global — federation governance, open-
        // source maintainers, scientific collaboration. Hardest
        // shape because wide-scope fanout dominates outbound, and
        // direct-trust archive is bigger (60% publishable instead
        // of 35%).
        Scenario {
            name: "full_internet_global_heavy",
            n_users: 5_000_000_000.0,
            tier_mix: TierMix { client: 0.30, proxy: 0.55, server: 0.15 }, // more servers for the load
            trust_radius: 250.0,
            daily_bytes: 50.0 * MB,
            avg_envelope_bytes: 50.0 * KB,
            own_retention_days: 3650.0,
            direct_trust_archive_days: 14.0, // brutal — most goes fetch-on-demand
            cohort: CohortDist::global_heavy(),
            daily_fetch_bytes: 1.0 * GB,
            cache_ttl_minutes: 60.0,
            server_cache_max_bytes: 200.0 * GB,
            cache_hit_rate: 0.40, // wider scope = lower interest locality
            agent_decisions_per_day: 200.0,
            trace_publishable_fraction: 0.20,
            trace_retention_days: 365.0,
        },
        // Single-village microcosm — 1K people, R=50, very dense
        // local trust. The favorable end: everyone knows everyone,
        // cohort is 80% local-only, cache hits are constant. Useful
        // for sizing a Pi-class home server in a community mesh.
        Scenario {
            name: "village_dense",
            n_users: 1_000.0,
            tier_mix: TierMix { client: 0.40, proxy: 0.40, server: 0.20 },
            trust_radius: 50.0,
            daily_bytes: 30.0 * MB,
            avg_envelope_bytes: 8.0 * KB,
            own_retention_days: 3650.0,
            direct_trust_archive_days: 730.0, // can afford 2y archive for the village
            cohort: CohortDist::local_heavy(),
            daily_fetch_bytes: 200.0 * MB,
            cache_ttl_minutes: 240.0, // longer TTL — small audience, content cycles slowly
            server_cache_max_bytes: 50.0 * GB,
            cache_hit_rate: 0.85, // tiny world = very high cache locality
            agent_decisions_per_day: 100.0,
            trace_publishable_fraction: 0.10,
            trace_retention_days: 730.0,
        },
        // Stretch: what happens if everyone wants 1 year of direct
        // trust hot-archive instead of 30 days? This is where the
        // model says "no, you need either smaller R or specialization."
        Scenario {
            name: "full_internet_stretch",
            n_users: 5_000_000_000.0,
            tier_mix: TierMix { client: 0.35, proxy: 0.55, server: 0.10 },
            trust_radius: 250.0,
            daily_bytes: 50.0 * MB,
            avg_envelope_bytes: 50.0 * KB,
            own_retention_days: 3650.0,
            direct_trust_archive_days: 365.0,
            cohort,
            daily_fetch_bytes: 1.0 * GB,
            cache_ttl_minutes: 60.0,
            server_cache_max_bytes: 200.0 * GB,
            cache_hit_rate: 0.60,
            agent_decisions_per_day: 200.0,
            trace_publishable_fraction: 0.10,
            trace_retention_days: 365.0,
        },
    ]
}

// ─── Formatting + feasibility report ─────────────────────────────────

fn fmt_bytes(b: f64) -> String {
    if b >= EB { format!("{:.2} EB", b / EB) }
    else if b >= PB { format!("{:.2} PB", b / PB) }
    else if b >= TB { format!("{:.2} TB", b / TB) }
    else if b >= GB { format!("{:.2} GB", b / GB) }
    else if b >= MB { format!("{:.2} MB", b / MB) }
    else if b >= KB { format!("{:.2} KB", b / KB) }
    else { format!("{:.0} B", b) }
}

fn fmt_count(c: f64) -> String {
    if c >= 1e12 { format!("{:.2} T", c / 1e12) }
    else if c >= 1e9 { format!("{:.2} B", c / 1e9) }
    else if c >= 1e6 { format!("{:.2} M", c / 1e6) }
    else if c >= 1e3 { format!("{:.2} K", c / 1e3) }
    else { format!("{:.1}", c) }
}

#[derive(Debug)]
struct Feasibility {
    storage_ok: bool, storage_ratio: f64,
    bandwidth_ok: bool, bandwidth_ratio: f64,
    cpu_ok: bool, cpu_ratio: f64,
}

fn check_server_feasibility(srv: &ActorCosts) -> Feasibility {
    let s_ratio = srv.storage_total / SERVER_DISK_GATE_BYTES;
    let bw_total = srv.bandwidth_in_per_day + srv.bandwidth_out_per_day;
    let bw_ratio = bw_total / SERVER_BANDWIDTH_GATE_BYTES_PER_DAY;
    let cpu_ratio = srv.cpu_seconds_per_day / SERVER_CPU_GATE_SECONDS_PER_DAY;
    Feasibility {
        storage_ok: s_ratio <= 1.0, storage_ratio: s_ratio,
        bandwidth_ok: bw_ratio <= 1.0, bandwidth_ratio: bw_ratio,
        cpu_ok: cpu_ratio <= 1.0, cpu_ratio,
    }
}

fn fmt_check(ok: bool) -> &'static str { if ok { "✓" } else { "⚠" } }

fn print_scenario(s: &Scenario, r: &FedRollup) {
    let srv = &r.per_tier[2].1;
    let feas = check_server_feasibility(srv);

    println!();
    println!("══ {} ══", s.name);
    println!("  N users: {}   tier mix: client {:.0}% / proxy {:.0}% / server {:.0}%",
        fmt_count(s.n_users), s.tier_mix.client * 100.0,
        s.tier_mix.proxy * 100.0, s.tier_mix.server * 100.0);
    println!("  R={}  D={}/day  env={}  σ_pub={:.0}%  fetch={}/day",
        s.trust_radius as u64, fmt_bytes(s.daily_bytes), fmt_bytes(s.avg_envelope_bytes),
        s.cohort.publishable() * 100.0, fmt_bytes(s.daily_fetch_bytes));
    println!("  cache: max={} ttl={}min hit_rate={:.0}%",
        fmt_bytes(s.server_cache_max_bytes), s.cache_ttl_minutes, s.cache_hit_rate * 100.0);
    println!("  archive: own={}d  direct_trust={}d  agent_decisions={}/day  trace_pub={:.0}%",
        s.own_retention_days as u64, s.direct_trust_archive_days as u64,
        s.agent_decisions_per_day as u64, s.trace_publishable_fraction * 100.0);

    println!();
    println!("  Server-tier breakdown:");
    println!("    own data            {}", fmt_bytes(srv.storage_own));
    println!("    direct-trust arch.  {}", fmt_bytes(srv.storage_direct_trust_archive));
    println!("    cache (hot)         {}", fmt_bytes(srv.storage_cache));
    println!("    agent traces        {}", fmt_bytes(srv.storage_traces));
    println!("    ─────────────────────────────");
    println!("    storage TOTAL       {}", fmt_bytes(srv.storage_total));
    println!("    bandwidth in/day    {}", fmt_bytes(srv.bandwidth_in_per_day));
    println!("    bandwidth out/day   {}", fmt_bytes(srv.bandwidth_out_per_day));
    println!("    verify ops/sec      {}",
        fmt_count(srv.verify_ops_per_day / 86400.0));
    println!("    CPU sec/day         {:.1}", srv.cpu_seconds_per_day);

    println!();
    println!("  v1 feasibility (per-server gates: 1 TB / 1 Gbps / 1 core):");
    println!("    {} storage    {:>5.1}% of 1 TB     ({})",
        fmt_check(feas.storage_ok), feas.storage_ratio * 100.0,
        fmt_bytes(srv.storage_total));
    let bw_total = srv.bandwidth_in_per_day + srv.bandwidth_out_per_day;
    println!("    {} bandwidth  {:>5.1}% of 1 Gbps   ({}/day, ≈ {}/sec)",
        fmt_check(feas.bandwidth_ok), feas.bandwidth_ratio * 100.0,
        fmt_bytes(bw_total), fmt_bytes(bw_total / 86400.0));
    println!("    {} cpu        {:>5.1}% of 1 core   ({:.1} cpu-sec/day)",
        fmt_check(feas.cpu_ok), feas.cpu_ratio * 100.0, srv.cpu_seconds_per_day);

    println!();
    println!("  Federation totals:");
    println!("    storage          {}", fmt_bytes(r.total_storage_bytes));
    println!("    bandwidth in     {}/day  ({}/sec)",
        fmt_bytes(r.total_bandwidth_in_bytes_per_day),
        fmt_bytes(r.total_bandwidth_in_bytes_per_day / 86400.0));
    println!("    sign/verify ops  {} sign/sec    {} verify/sec",
        fmt_count(r.total_sign_ops_per_day / 86400.0),
        fmt_count(r.total_verify_ops_per_day / 86400.0));
    println!("    CPU @ 5% util    {} cores",
        fmt_count(r.aggregate_cpu_cores_full_util / 0.05));

    if !feas.storage_ok || !feas.bandwidth_ok || !feas.cpu_ok {
        println!();
        println!("  ⚠ NOT FEASIBLE on per-server gates. Knobs to turn:");
        if !feas.storage_ok {
            print_storage_advice(s, srv);
        }
        if !feas.bandwidth_ok {
            println!("    • bandwidth: raise cache_hit_rate, lower trust_radius,");
            println!("                 or specialize servers (topical / regional)");
        }
        if !feas.cpu_ok {
            println!("    • cpu: lower agent_decisions_per_day or specialize.");
            println!("           (verify/scrub dominate at high agent activity.)");
        }
    } else {
        println!();
        println!("  ✓ v1 feasible per-server. Replicates to {} servers globally.",
            fmt_count(s.n_users * s.tier_mix.server));
    }
}

fn print_storage_advice(s: &Scenario, srv: &ActorCosts) {
    let dominant = if srv.storage_direct_trust_archive >= srv.storage_cache
        && srv.storage_direct_trust_archive >= srv.storage_traces
    { "direct-trust archive" }
    else if srv.storage_cache >= srv.storage_traces { "cache" }
    else { "traces" };
    println!("    • storage dominant: {} ({})", dominant,
        match dominant {
            "direct-trust archive" => format!(
                "lower direct_trust_archive_days from {} or trust_radius from {}",
                s.direct_trust_archive_days as u64, s.trust_radius as u64),
            "cache" => format!("lower server_cache_max_bytes from {}",
                fmt_bytes(s.server_cache_max_bytes)),
            _ => format!("lower trace_retention_days from {} or trace_publishable_fraction from {:.0}%",
                s.trace_retention_days as u64, s.trace_publishable_fraction * 100.0),
        });
}

fn main() {
    println!("CIRIS Federation Scaling Model — toy v0.2");
    println!("Empirical inputs: Verify v2.8.0 + Edge v0.10.0 + Persist v3.1.1");
    println!("Load-bearing assumptions:");
    println!("  • fetch-on-demand primary (no R² pre-replication)");
    println!("  • direct-trust archive = R first-order × σ_publishable × T_direct");
    println!("  • cache LRU bounded + TTL-decayed, encrypted at rest + in transit");
    println!("  • agent traces scrubbed before storage; trace_publishable_fraction × R replicated");
    println!("Per-server v1 gates: 1 TB disk / 1 Gbps bandwidth / 1 core full-util");

    for s in scenarios() {
        let r = rollup(&s);
        print_scenario(&s, &r);
    }

    println!();
    println!("── Design search knobs (edit scenarios() in this file) ──");
    println!("  direct_trust_archive_days  — pre-replicated window before fetch-on-demand");
    println!("  cache_ttl_minutes          — cache residency after last access");
    println!("  server_cache_max_bytes     — LRU cap (the hot-set budget)");
    println!("  cache_hit_rate             — measured later; assumption now");
    println!("  trace_publishable_fraction — what cross-replicates from agent traces");
    println!("  trust_radius (R)           — direct trust set; quadratic in storage if 2nd-order on");
    println!();
}
