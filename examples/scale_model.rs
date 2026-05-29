//! Federation scaling model — companion to `FSD/FEDERATION_SCALING_MODEL.md`.
//!
//! Run: `cargo run --example scale_model`
//!
//! Toy that models storage / bandwidth / compute roll-up across the
//! Edge v1.0 client/proxy/server tier mix, parameterized by trust
//! radius + cohort distribution + per-user activity. Empirical
//! constants are baked in from CIRISVerify v2.8.0 + CIRISEdge v0.10.0
//! + CIRISPersist v3.1.1 — see the FSD for citations.
//!
//! The five preset scenarios in `scenarios()` are the reference
//! deployment shapes the FSD enumerates. Modify them, or add a
//! `custom_scenario()` function and call it from `main`, to play
//! `what-if`.

// ─── Empirical constants ──────────────────────────────────────────────

/// Hybrid Ed25519 + ML-DSA-65 sign (CIRISVerify v2.8.0).
const HYBRID_SIGN_US: f64 = 466.0;

/// Hybrid verify (CIRISVerify v2.8.0).
const HYBRID_VERIFY_US: f64 = 276.0;

/// `dispatch_inbound` overhead on top of verify (Edge v0.10.0 target).
const DISPATCH_OVERHEAD_US: f64 = 120.0;

/// Persist SQLite per-row write incl. async wrapper (Edge target).
#[allow(dead_code)]
const PERSIST_ROW_WRITE_MS: f64 = 1.5;

/// Canonicalization slope (Edge target).
const CANONICALIZE_NS_PER_KIB: f64 = 250.0;

// ─── Tier model (Edge v1.0 design) ───────────────────────────────────

/// Proxy LRU cache budget — default 4 GiB (deployment-config).
const PROXY_CACHE_BYTES: f64 = 4.0 * GIB;

/// Server-tier disk gate. Servers below this don't exist (in the
/// model); above it, we assume disk is not the bottleneck.
/// Bumped 256 GB → 1 TB per Eric — gives headroom for news-archive
/// + full-internet-replacement scenarios.
#[allow(dead_code)]
const SERVER_DISK_GATE_BYTES: f64 = 1024.0 * GIB;

/// Second-order trust discount — server tier replicates `T(T(host))`
/// at this fraction (most second-order content isn't relevant).
const SECOND_ORDER_DISCOUNT: f64 = 0.10;

const KB: f64 = 1024.0;
const MB: f64 = 1024.0 * KB;
const GB: f64 = 1024.0 * MB;
const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
const TB: f64 = 1024.0 * GB;
const PB: f64 = 1024.0 * TB;

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
    /// Fraction of activity that's "publishable" — scope ≥ community.
    /// This is the fraction servers replicate from trusted peers.
    fn publishable(&self) -> f64 {
        self.community + self.affiliations + self.species + self.planet + self.federation
    }

    /// Fraction that stays local-only (self + family).
    fn local_only(&self) -> f64 {
        self.self_ + self.family
    }

    /// FSD §3 default distribution: 65% local, 35% publishable.
    fn default_model() -> Self {
        Self {
            self_: 0.50,
            family: 0.15,
            community: 0.15,
            affiliations: 0.10,
            species: 0.05,
            planet: 0.03,
            federation: 0.02,
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
    /// Average per-user daily activity volume.
    daily_bytes: f64,
    /// Average envelope size — sets the sign/verify call count for
    /// a given byte volume.
    avg_envelope_bytes: f64,
    /// Retention window (days) for the storage rollup. Own data is
    /// unbounded; this caps the archive for sizing purposes.
    retention_days: f64,
    cohort: CohortDist,
    /// Average daily ContentFetch traffic a user pulls (browse).
    daily_fetch_bytes: f64,
}

// ─── Per-actor formulae ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default)]
struct ActorCosts {
    storage_bytes: f64,
    bandwidth_out_bytes_per_day: f64,
    bandwidth_in_bytes_per_day: f64,
    /// Verify operations per day (verify is server-tier dominant).
    verify_ops_per_day: f64,
    sign_ops_per_day: f64,
    /// Aggregate CPU-seconds per day (sign + verify + dispatch + canon).
    cpu_seconds_per_day: f64,
}

fn per_actor(tier: Tier, s: &Scenario) -> ActorCosts {
    let envs_per_day = s.daily_bytes / s.avg_envelope_bytes;

    // OWN — every tier stores own contributions, signs them, sends
    // them outbound.
    let own_storage = s.daily_bytes * s.retention_days;
    let sign_ops = envs_per_day;

    // TIER-DEPENDENT.
    let (replicated_storage, replicated_in_bps, replicated_verify_ops, fanout_mult, proxy_cache) = match tier {
        Tier::Client => (0.0, 0.0, 0.0, 1.0, 0.0),
        Tier::Proxy => {
            // Default proxy doesn't long-term replicate; serves transit
            // from a bounded LRU. Modeling transit as 10% of daily
            // browse traffic accumulating in cache (LRU evicts the rest).
            let cache_steady_state = (PROXY_CACHE_BYTES).min(s.daily_fetch_bytes * 30.0);
            // Light verify load: only what we fetch on behalf of the
            // user; assume 1 verify per fetched envelope.
            let verify_from_fetch =
                s.daily_fetch_bytes / s.avg_envelope_bytes;
            (0.0, 0.0, verify_from_fetch, 2.0, cache_steady_state)
        }
        Tier::Server => {
            // First-order replication: trust radius × peer's
            // publishable activity × retention.
            let first_order_daily = s.trust_radius * s.daily_bytes * s.cohort.publishable();
            // Second-order: discounted heavily.
            // Approximation: second-order set ≈ R × R (with overlap),
            // discounted to SECOND_ORDER_DISCOUNT.
            let second_order_daily =
                s.trust_radius * s.trust_radius * s.daily_bytes * s.cohort.publishable() * SECOND_ORDER_DISCOUNT;
            let replicated_daily = first_order_daily + second_order_daily;
            let stored = replicated_daily * s.retention_days;

            // Verify cost: must verify every replicated envelope on
            // ingest + every served ContentFetch (modeled here as 2×
            // ingest for re-serve discipline).
            let verify_ops = (replicated_daily / s.avg_envelope_bytes) * 2.0;
            // Bandwidth in: same as replicated_daily (per second).
            let in_bps = replicated_daily;

            // Outbound fanout: own contributions × (1 + steward set
            // size for federation-scope content). Approximation:
            // steward set ≈ 4 for community/affiliations, 64 for
            // species/planet/federation scopes.
            // Weighted: σ_community+affiliations × 4 + σ_species+planet+federation × 64.
            let wide_scope = s.cohort.species + s.cohort.planet + s.cohort.federation;
            let narrow_scope = s.cohort.community + s.cohort.affiliations;
            let fanout = 1.0 + narrow_scope * 4.0 + wide_scope * 64.0;
            (stored, in_bps, verify_ops, fanout, 0.0)
        }
    };

    let outbound_bps = s.daily_bytes * fanout_mult;
    let inbound_bps = replicated_in_bps + s.daily_fetch_bytes;
    let verify_ops_per_day = replicated_verify_ops;

    // CPU time accounting.
    let sign_cpu_s = sign_ops * HYBRID_SIGN_US * 1e-6;
    let verify_cpu_s = verify_ops_per_day * HYBRID_VERIFY_US * 1e-6;
    let dispatch_cpu_s = verify_ops_per_day * DISPATCH_OVERHEAD_US * 1e-6;
    let total_envs_canon = (sign_ops + verify_ops_per_day) * (s.avg_envelope_bytes / KB);
    let canon_cpu_s = total_envs_canon * CANONICALIZE_NS_PER_KIB * 1e-9;

    ActorCosts {
        storage_bytes: own_storage + replicated_storage + proxy_cache,
        bandwidth_out_bytes_per_day: outbound_bps,
        bandwidth_in_bytes_per_day: inbound_bps,
        verify_ops_per_day,
        sign_ops_per_day: sign_ops,
        cpu_seconds_per_day: sign_cpu_s + verify_cpu_s + dispatch_cpu_s + canon_cpu_s,
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
    /// Effective CPU cores required across the federation if every
    /// node ran at 100% utilization for the day (an absurd ceiling —
    /// real utilization is single-digit %).
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

    let total_storage =
        n_cli * cli.storage_bytes + n_prx * prx.storage_bytes + n_srv * srv.storage_bytes;
    let total_in = n_cli * cli.bandwidth_in_bytes_per_day
        + n_prx * prx.bandwidth_in_bytes_per_day
        + n_srv * srv.bandwidth_in_bytes_per_day;
    let total_out = n_cli * cli.bandwidth_out_bytes_per_day
        + n_prx * prx.bandwidth_out_bytes_per_day
        + n_srv * srv.bandwidth_out_bytes_per_day;
    let total_verify =
        n_cli * cli.verify_ops_per_day + n_prx * prx.verify_ops_per_day + n_srv * srv.verify_ops_per_day;
    let total_sign =
        n_cli * cli.sign_ops_per_day + n_prx * prx.sign_ops_per_day + n_srv * srv.sign_ops_per_day;
    let total_cpu_s = n_cli * cli.cpu_seconds_per_day
        + n_prx * prx.cpu_seconds_per_day
        + n_srv * srv.cpu_seconds_per_day;
    let seconds_in_day = 86400.0;

    FedRollup {
        total_storage_bytes: total_storage,
        total_bandwidth_in_bytes_per_day: total_in,
        total_bandwidth_out_bytes_per_day: total_out,
        total_verify_ops_per_day: total_verify,
        total_sign_ops_per_day: total_sign,
        aggregate_cpu_cores_full_util: total_cpu_s / seconds_in_day,
        per_tier: [(Tier::Client, cli), (Tier::Proxy, prx), (Tier::Server, srv)],
    }
}

// ─── Preset scenarios ────────────────────────────────────────────────

fn scenarios() -> Vec<Scenario> {
    let cohort = CohortDist::default_model();
    vec![
        Scenario {
            name: "bootstrap",
            n_users: 10_000.0,
            tier_mix: TierMix { client: 0.30, proxy: 0.65, server: 0.05 },
            trust_radius: 50.0,
            daily_bytes: 20.0 * KB,
            avg_envelope_bytes: 1.5 * KB,
            retention_days: 365.0,
            cohort,
            daily_fetch_bytes: 5.0 * MB,
        },
        Scenario {
            name: "dunbar_steady",
            n_users: 1_000_000.0,
            tier_mix: TierMix { client: 0.40, proxy: 0.55, server: 0.05 },
            trust_radius: 150.0,
            daily_bytes: 50.0 * KB,
            avg_envelope_bytes: 1.5 * KB,
            retention_days: 365.0,
            cohort,
            daily_fetch_bytes: 50.0 * MB,
        },
        Scenario {
            name: "media_heavy",
            n_users: 1_000_000.0,
            tier_mix: TierMix { client: 0.30, proxy: 0.60, server: 0.10 },
            trust_radius: 150.0,
            daily_bytes: 500.0 * KB,
            avg_envelope_bytes: 8.0 * KB,
            retention_days: 365.0,
            cohort,
            daily_fetch_bytes: 200.0 * MB,
        },
        Scenario {
            name: "twitter_scale",
            n_users: 1_000_000_000.0,
            tier_mix: TierMix { client: 0.45, proxy: 0.50, server: 0.05 },
            trust_radius: 150.0,
            daily_bytes: 5.0 * KB,
            avg_envelope_bytes: 0.5 * KB,
            retention_days: 365.0,
            cohort,
            daily_fetch_bytes: 20.0 * MB,
        },
        Scenario {
            name: "news_replacement",
            n_users: 1_000_000_000.0,
            tier_mix: TierMix { client: 0.40, proxy: 0.55, server: 0.05 },
            trust_radius: 300.0,
            daily_bytes: 100.0 * KB,
            avg_envelope_bytes: 5.0 * KB,
            retention_days: 1825.0, // 5 years archive
            cohort,
            daily_fetch_bytes: 100.0 * MB,
        },
        // Full internet replacement: everything humans generate +
        // consume online — social posts, chat, photos, short video,
        // news, encyclopedia, blogs, collaborative docs. Excludes
        // streaming-CDN long-form video (Netflix / YouTube live
        // streaming) which is a transport problem, not a federation-
        // substrate problem; those would ride ContentFetch external_ref
        // pointers to S3-class stores, not inline blobs.
        //
        // 5B users (humans online); 50 MB/user/day across all forms
        // (text + photos + short clips); R=250 (wider trust nets when
        // a single substrate carries all your sources); 10% server
        // tier (more diversity needs more replication anchors); 10y
        // retention (the "permanent record" target — beats Internet
        // Archive's "best effort" model). Daily fetch 1 GB/user
        // ≈ Cisco's median-consumer internet pull (excluding raw
        // video streaming, which is the CDN tier).
        Scenario {
            name: "full_internet",
            n_users: 5_000_000_000.0,
            tier_mix: TierMix { client: 0.35, proxy: 0.55, server: 0.10 },
            trust_radius: 250.0,
            daily_bytes: 50.0 * MB,
            avg_envelope_bytes: 50.0 * KB,
            retention_days: 3650.0, // 10 years
            cohort,
            daily_fetch_bytes: 1.0 * GB,
        },
    ]
}

// ─── Formatting + comparison ─────────────────────────────────────────

fn fmt_bytes(b: f64) -> String {
    if b >= PB { format!("{:.2} PB", b / PB) }
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

fn print_scenario(s: &Scenario, r: &FedRollup) {
    println!();
    println!("══ {} ══", s.name);
    println!("  N users: {}   tier mix: client {:.0}% / proxy {:.0}% / server {:.0}%",
        fmt_count(s.n_users),
        s.tier_mix.client * 100.0,
        s.tier_mix.proxy * 100.0,
        s.tier_mix.server * 100.0);
    println!("  trust radius: {} peers   daily activity: {}   envelope avg: {}",
        s.trust_radius as u64,
        fmt_bytes(s.daily_bytes),
        fmt_bytes(s.avg_envelope_bytes));
    println!("  cohort: local-only {:.0}%   publishable {:.0}%   retention {} days",
        s.cohort.local_only() * 100.0,
        s.cohort.publishable() * 100.0,
        s.retention_days as u64);
    println!();
    println!("  Per-actor costs:");
    println!("    {:<8}  {:>11}  {:>12}  {:>12}  {:>10}  {:>11}",
        "tier", "storage", "out/day", "in/day", "verify/s", "cpu-s/day");
    for (tier, costs) in &r.per_tier {
        let tier_name = match tier { Tier::Client => "client", Tier::Proxy => "proxy", Tier::Server => "server" };
        println!("    {:<8}  {:>11}  {:>12}  {:>12}  {:>10}  {:>11.3}",
            tier_name,
            fmt_bytes(costs.storage_bytes),
            fmt_bytes(costs.bandwidth_out_bytes_per_day),
            fmt_bytes(costs.bandwidth_in_bytes_per_day),
            fmt_count(costs.verify_ops_per_day / 86400.0),
            costs.cpu_seconds_per_day);
    }
    println!();
    println!("  Federation totals:");
    println!("    storage          {}", fmt_bytes(r.total_storage_bytes));
    println!("    bandwidth in     {}/day  ({}/sec)",
        fmt_bytes(r.total_bandwidth_in_bytes_per_day),
        fmt_bytes(r.total_bandwidth_in_bytes_per_day / 86400.0));
    println!("    bandwidth out    {}/day  ({}/sec)",
        fmt_bytes(r.total_bandwidth_out_bytes_per_day),
        fmt_bytes(r.total_bandwidth_out_bytes_per_day / 86400.0));
    println!("    sign ops         {}/day  ({} /sec)",
        fmt_count(r.total_sign_ops_per_day),
        fmt_count(r.total_sign_ops_per_day / 86400.0));
    println!("    verify ops       {}/day  ({} /sec)",
        fmt_count(r.total_verify_ops_per_day),
        fmt_count(r.total_verify_ops_per_day / 86400.0));
    println!("    CPU @ full-util  {} cores  (avg @ 5% util: {} cores)",
        fmt_count(r.aggregate_cpu_cores_full_util),
        fmt_count(r.aggregate_cpu_cores_full_util / 0.05));
    println!();
    print_comparison(s, r);
}

fn print_comparison(s: &Scenario, r: &FedRollup) {
    // Web-scale anchors from FSD §1.4.
    let twitter_daily = 70.0 * GB;
    let facebook_daily = 4.0 * PB;
    let net_daily = r.total_bandwidth_in_bytes_per_day;

    println!("  vs web-scale anchors:");
    println!("    Twitter daily content (70 GB):     ratio = {:.2}×",
        net_daily / twitter_daily);
    println!("    Facebook daily content (4 PB):     ratio = {:.4}×",
        net_daily / facebook_daily);
    let per_user_storage = r.total_storage_bytes / s.n_users;
    println!("    Avg storage / user:                {}", fmt_bytes(per_user_storage));
    let per_user_bw = (r.total_bandwidth_in_bytes_per_day + r.total_bandwidth_out_bytes_per_day) / s.n_users;
    println!("    Avg bandwidth / user / day:        {}", fmt_bytes(per_user_bw));

    // Server-tier feasibility check.
    let avg_server_storage = r.per_tier[2].1.storage_bytes;
    let gate_tb = SERVER_DISK_GATE_BYTES / (1024.0 * 1024.0 * 1024.0 * 1024.0);
    let disk_gate_ok = avg_server_storage <= SERVER_DISK_GATE_BYTES * 10.0;
    let disk_gate_marker = if disk_gate_ok {
        format!("✓ within 10× the {} TB gate", gate_tb as u64)
    } else {
        format!("⚠ exceeds 10× the {} TB gate", gate_tb as u64)
    };
    println!("    Avg server-tier storage:           {} {}",
        fmt_bytes(avg_server_storage), disk_gate_marker);
}

fn main() {
    println!("CIRIS Federation Scaling Model — toy v0.1");
    println!("Empirical inputs: Verify v2.8.0 + Edge v0.10.0 + Persist v3.1.1");
    println!("See FSD/FEDERATION_SCALING_MODEL.md for derivations + assumptions.");

    for s in scenarios() {
        let r = rollup(&s);
        print_scenario(&s, &r);
    }

    println!();
    println!("── Knobs to play with (edit the scenarios above) ──");
    println!("  trust_radius         — bigger R → server storage scales linearly");
    println!("  cohort.publishable() — bigger σ_publishable → more replication");
    println!("  tier_mix.server      — more servers → more total replicated storage");
    println!("  avg_envelope_bytes   — bigger envelopes → fewer sign/verify ops per byte");
    println!();
}
