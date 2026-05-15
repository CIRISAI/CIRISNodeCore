//! Trust hierarchy — DIRECT/REGISTRY policy over persist's
//! `federation_keys` trust columns.
//!
//! Per [`FSD/TRUST_HIERARCHY.md`]. Persist owns the storage + raw
//! CRUD via the [`FederationDirectory`] trait; node-core owns the
//! transitive-resolution policy via [`resolve_trust`].
//!
//! # v0.1.0-dev status
//!
//! [`FederationDirectory`] is **defined locally** in this crate
//! pending CIRISPersist v1.3.0 (the M2 cut on the v1.3.0 roadmap).
//! Persist will absorb this trait + the 4 supporting types into its
//! `cirisnode` module alongside the V020 migration. When that lands,
//! this module replaces the trait definition with:
//!
//! ```ignore
//! pub use ciris_persist::cirisnode::{
//!     FederationDirectory, TrustGrant, TrustRow, TrustFilter,
//! };
//! ```
//!
//! Consumer code written against today's trait keeps compiling
//! unchanged.
//!
//! [`FSD/TRUST_HIERARCHY.md`]: https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/TRUST_HIERARCHY.md

use std::future::Future;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::payloads::registry_vouch::{RegistryVouchPayload, SUBJECT_KIND as VOUCH_SUBJECT_KIND};
use crate::substrate::{ContributionType, ContributionsFilter, NodeCoreService, SubstrateError};

// ── Types (proposed upstream to persist) ────────────────────────────────

/// Trust type axis. Mirrors CIRISAgent ConsentService taxonomy;
/// tracks CIRISAgent#760 §RC consent_role lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustType {
    /// Default. Most peer-to-peer agent-to-agent observations.
    Temporary,
    /// Bilateral approval (CIRISAgent#760 / LensCore ConsentService scope).
    Partnered,
    /// Anonymous trust grant.
    Anonymous,
}

/// Trust relationship axis. New axis introduced by
/// `FSD/TRUST_HIERARCHY.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustRelationship {
    /// Peer trust — K_B can act directly with the grantor.
    Direct,
    /// Vouching delegation — K_B can vouch for other keys within
    /// `trust_domains` only.
    Registry,
}

/// A trust grant — what the grantor declared.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustGrant {
    /// Subject of the grant — the trusted key.
    pub key: String,
    /// Type axis.
    pub trust_type: TrustType,
    /// Relationship axis.
    pub trust_relationship: TrustRelationship,
    /// Domain scope. Required when `trust_relationship = Registry`.
    /// MUST be `None` for `Direct` grants.
    pub trust_domains: Option<Vec<String>>,
    /// Grantor key. Must differ from `key` per the
    /// `trusted_by != key` integrity rule (no self-trust).
    pub trusted_by: String,
    /// `None` = open-ended.
    pub expires_at: Option<DateTime<Utc>>,
}

/// A row from the directory — the grant + its `trusted_at` timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustRow {
    /// Subject of the grant.
    pub key: String,
    /// Type axis.
    pub trust_type: TrustType,
    /// Relationship axis.
    pub trust_relationship: TrustRelationship,
    /// Domain scope (`Some` when relationship = Registry).
    pub trust_domains: Option<Vec<String>>,
    /// Grantor key.
    pub trusted_by: String,
    /// When the grant was created.
    pub trusted_at: DateTime<Utc>,
    /// `None` = open-ended.
    pub expires_at: Option<DateTime<Utc>>,
}

/// Filter for [`FederationDirectory::list_trusted_keys`]. All fields
/// AND-composed; every field optional.
#[derive(Debug, Clone, Default)]
pub struct TrustFilter {
    /// Narrow by type axis.
    pub trust_type: Option<TrustType>,
    /// Narrow by relationship axis.
    pub trust_relationship: Option<TrustRelationship>,
    /// Narrow to registries vouching for `domain`. Only meaningful
    /// with `trust_relationship = Some(Registry)`.
    pub domain: Option<String>,
    /// If `false` (default), expired rows are filtered server-side.
    pub include_expired: bool,
}

/// Resolved trust edge — the answer to "is `key` trusted for `domain`?".
#[derive(Debug, Clone, PartialEq)]
pub enum TrustEdge {
    /// `key` has a direct trust row of relationship = Direct.
    Direct {
        /// The grant's type axis.
        trust_type: TrustType,
    },
    /// `key` has a direct trust row of relationship = Registry whose
    /// `trust_domains` includes the queried domain.
    Registry {
        /// The grant's type axis.
        trust_type: TrustType,
        /// All declared domains on the grant (the queried domain is
        /// one of them).
        domains: Vec<String>,
    },
    /// `key` is trusted because a currently-trusted registry vouched
    /// for it in the queried domain. Queried domain is implicit from
    /// the resolve call.
    Transitive {
        /// Registry key that vouched.
        via_registry: String,
    },
    /// No trust edge resolves.
    Untrusted,
}

// ── FederationDirectory trait (proposed upstream) ────────────────────────

/// Raw CRUD + simple queries on the `federation_keys` trust columns.
///
/// Currently defined in this crate as a placeholder; CIRISPersist
/// v1.3.0 absorbs the trait per the M2 step in
/// [`FSD/TRUST_HIERARCHY.md`] §10. When that ships, this module
/// replaces the local trait with a `pub use` from persist.
pub trait FederationDirectory: Send + Sync {
    /// Insert or update a trust row. Implementations enforce
    /// `grant.trusted_by != grant.key` (no self-trust) at the
    /// boundary and write the state transition to the audit chain.
    fn grant_trust(
        &self,
        grant: TrustGrant,
    ) -> impl Future<Output = Result<(), SubstrateError>> + Send;

    /// Soft-delete a trust row by setting `expires_at = NOW()`.
    /// Audit row written. Idempotent.
    fn revoke_trust(
        &self,
        key: &str,
        revoked_by: &str,
    ) -> impl Future<Output = Result<(), SubstrateError>> + Send;

    /// Point lookup — the raw row, no transitive resolution. `None`
    /// if no row exists for `key`.
    fn lookup_trust(
        &self,
        key: &str,
    ) -> impl Future<Output = Result<Option<TrustRow>, SubstrateError>> + Send;

    /// All currently-trusted keys matching `filter`. Server-side
    /// filtering for relationship + domain; expired rows excluded
    /// unless `filter.include_expired = true`.
    fn list_trusted_keys(
        &self,
        filter: TrustFilter,
    ) -> impl Future<Output = Result<Vec<TrustRow>, SubstrateError>> + Send;
}

// ── Transitive resolution (node-core policy) ─────────────────────────────

/// True if the row's `expires_at` is set and has passed.
pub fn is_expired(row: &TrustRow) -> bool {
    matches!(row.expires_at, Some(t) if t <= Utc::now())
}

/// Resolve the trust edge for `key` under `domain`.
///
/// Algorithm:
/// 1. Direct lookup — if `key` has a current trust row, return the
///    matching [`TrustEdge`] variant. For `Registry` rows, the
///    queried `domain` must be in `trust_domains`; otherwise the
///    direct edge does NOT apply to this query.
/// 2. Transitive — search currently-trusted registries for the
///    queried domain; for each registry, look for an active
///    `registry_vouch` Contribution naming `key` in `domain`. First
///    match wins.
/// 3. Otherwise return [`TrustEdge::Untrusted`].
///
/// Revocation propagates at query time — see
/// `FSD/TRUST_HIERARCHY.md` §3.2.
pub async fn resolve_trust<D, E>(
    directory: &D,
    engine: &E,
    key: &str,
    domain: &str,
) -> Result<TrustEdge, SubstrateError>
where
    D: FederationDirectory,
    E: NodeCoreService,
{
    if let Some(row) = directory.lookup_trust(key).await? {
        if !is_expired(&row) {
            return Ok(match row.trust_relationship {
                TrustRelationship::Direct => TrustEdge::Direct {
                    trust_type: row.trust_type,
                },
                TrustRelationship::Registry => {
                    let domains = row.trust_domains.unwrap_or_default();
                    if domains.iter().any(|d| d == domain) {
                        TrustEdge::Registry {
                            trust_type: row.trust_type,
                            domains,
                        }
                    } else {
                        // Registry row exists but doesn't cover this domain.
                        // Fall through to transitive search — maybe a
                        // *different* registry vouches.
                        return resolve_transitive(engine, directory, key, domain).await;
                    }
                }
            });
        }
    }
    resolve_transitive(engine, directory, key, domain).await
}

async fn resolve_transitive<D, E>(
    engine: &E,
    directory: &D,
    key: &str,
    domain: &str,
) -> Result<TrustEdge, SubstrateError>
where
    D: FederationDirectory,
    E: NodeCoreService,
{
    let registries = directory
        .list_trusted_keys(TrustFilter {
            trust_relationship: Some(TrustRelationship::Registry),
            domain: Some(domain.to_owned()),
            include_expired: false,
            ..Default::default()
        })
        .await?;
    for registry in registries {
        if registry_vouches_for(engine, &registry.key, key, domain).await? {
            return Ok(TrustEdge::Transitive {
                via_registry: registry.key,
            });
        }
    }
    Ok(TrustEdge::Untrusted)
}

/// True if `registry_key` has filed an active `registry_vouch`
/// Contribution naming `vouched_key` in `domain`.
///
/// Reads from `cirisnode.contributions` via
/// [`NodeCoreService::list_contributions`] filtered to
/// `contribution_type = Proposal AND subject_kind = "registry_vouch"
/// AND author_id = $registry_key`. Vouched-key + domain match
/// happens in-memory because the substrate filter doesn't reach into
/// the payload JSON.
pub async fn registry_vouches_for<E: NodeCoreService>(
    engine: &E,
    registry_key: &str,
    vouched_key: &str,
    domain: &str,
) -> Result<bool, SubstrateError> {
    let filter = ContributionsFilter {
        contribution_type: Some(ContributionType::Proposal),
        subject_kind: Some(VOUCH_SUBJECT_KIND.to_owned()),
        author_id: Some(registry_key.to_owned()),
        ..Default::default()
    };
    let page = engine.list_contributions(filter, None, 10_000).await?;

    let now = Utc::now();
    for env in page.items {
        let Ok(payload) = serde_json::from_value::<RegistryVouchPayload>(env.payload) else {
            continue; // malformed; skip
        };
        if payload.vouched_key == vouched_key
            && payload.vouched_domain == domain
            && payload.is_active_at(now)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

// ── Vouched-resolver enumeration (used by route_deferral) ───────────────

/// List currently-vouched keys (`vouched_key`) that `registry_key`
/// vouches for in `domain`. Reads + filters the same path as
/// [`registry_vouches_for`].
pub async fn list_vouched_for<E: NodeCoreService>(
    engine: &E,
    registry_key: &str,
    domain: &str,
) -> Result<Vec<String>, SubstrateError> {
    let filter = ContributionsFilter {
        contribution_type: Some(ContributionType::Proposal),
        subject_kind: Some(VOUCH_SUBJECT_KIND.to_owned()),
        author_id: Some(registry_key.to_owned()),
        ..Default::default()
    };
    let page = engine.list_contributions(filter, None, 10_000).await?;

    let now = Utc::now();
    let mut out: Vec<String> = Vec::new();
    for env in page.items {
        let Ok(payload) = serde_json::from_value::<RegistryVouchPayload>(env.payload) else {
            continue;
        };
        if payload.vouched_domain == domain && payload.is_active_at(now) {
            // Latest vouch wins per (registry, key, domain). Dedupe.
            if !out.contains(&payload.vouched_key) {
                out.push(payload.vouched_key);
            }
        }
    }
    Ok(out)
}
