//! Trust hierarchy — DIRECT/REGISTRY policy over persist's
//! `federation_keys` trust columns.
//!
//! Per [`FSD/TRUST_HIERARCHY.md`]. Persist owns the storage + raw
//! CRUD via the [`FederationDirectory`] trait (absorbed in
//! CIRISPersist v1.3.0); node-core owns the transitive-resolution
//! policy via [`resolve_trust`].
//!
//! # Substrate swap (CIRISPersist v1.3.0 — landed)
//!
//! The `FederationDirectory` trait + 5 supporting types
//! (`TrustType`, `TrustRelationship`, `TrustGrant`, `TrustRow`,
//! `TrustFilter`) are now re-exported from
//! `ciris_persist::federation`. Earlier versions of this module
//! defined the trait + types locally pending the v1.3.0 absorption;
//! that placeholder is gone.
//!
//! Persist's [`federation::Error`] is distinct from
//! [`crate::substrate::SubstrateError`] (which is the
//! `NodeCoreService` failure surface). The [`fed_err`] helper maps
//! the former to the latter so the transitive resolver can return a
//! single error type to consumers.
//!
//! [`FSD/TRUST_HIERARCHY.md`]: https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/TRUST_HIERARCHY.md
//! [`federation::Error`]: ciris_persist::federation::Error

use chrono::Utc;

pub use ciris_persist::federation::{
    FederationDirectory, TrustFilter, TrustGrant, TrustRelationship, TrustRow, TrustType,
};

use crate::payloads::registry_vouch::{RegistryVouchPayload, SUBJECT_KIND as VOUCH_SUBJECT_KIND};
use crate::substrate::{ContributionType, ContributionsFilter, NodeCoreService, SubstrateError};

// ── Cross-error mapping ─────────────────────────────────────────────────

/// Map persist's [`federation::Error`] to [`SubstrateError`] so the
/// transitive resolver can return a single error type to consumers.
/// Semantic mapping per the kind() tokens — keeps telemetry-relevant
/// failure context intact.
///
/// [`federation::Error`]: ciris_persist::federation::Error
pub fn fed_err(e: ciris_persist::federation::Error) -> SubstrateError {
    use ciris_persist::federation::Error as FE;
    match e {
        FE::InvalidArgument(s) => SubstrateError::InvalidArgument(s),
        FE::SignatureInvalid(s) => SubstrateError::Signature(s),
        FE::RateLimited {
            retry_after_seconds,
        } => SubstrateError::Backend(format!("rate limited: retry after {retry_after_seconds}s")),
        FE::Conflict(s) => SubstrateError::Conflict(s),
        FE::Backend(s) => SubstrateError::Backend(s),
    }
}

// ── Node-core policy types + functions ──────────────────────────────────

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

/// True if the row's `expires_at` is set and has passed.
pub fn is_expired(row: &TrustRow) -> bool {
    matches!(row.expires_at, Some(t) if t <= Utc::now())
}

/// Resolve the trust edge for `key` under `domain`.
///
/// Algorithm:
/// 1. Direct lookup — if `key` has a current trust row, return the
///    matching [`TrustEdge`] variant. For `Registry` rows, the
///    queried `domain` must be in `trust_domains`; otherwise fall
///    through to the transitive search.
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
    if let Some(row) = directory.lookup_trust(key).await.map_err(fed_err)? {
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
        .await
        .map_err(fed_err)?;
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
            continue;
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

/// List currently-vouched keys (`vouched_key`) that `registry_key`
/// vouches for in `domain`.
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
            if !out.contains(&payload.vouched_key) {
                out.push(payload.vouched_key);
            }
        }
    }
    Ok(out)
}
