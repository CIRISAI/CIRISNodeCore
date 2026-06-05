//! Trust hierarchy — DIRECT/TRANSITIVE policy over persist's
//! `federation_trust_grants` projection.
//!
//! Per [`FSD/TRUST_HIERARCHY.md`]. Persist owns the storage + the
//! signed-event substrate via the `federation_trust_grants` table
//! (CIRISPersist v1.5.0 — every grant is a signed Contribution event
//! that materializes a row); node-core owns the
//! transitive-resolution policy via [`resolve_trust`].
//!
//! # Substrate swap status (CIRISPersist v1.5.x — Step 10 of FSD §8)
//!
//! v1.5.0/v1.5.1 introduces the new signed-grant substrate
//! (`federation_trust_grants` with per-tenant Merkle transparency).
//! `resolve_trust` reads from this projection via
//! [`ciris_persist::audit::AuditService::lookup_trust_grant`] /
//! `list_trust_grants` rather than the v1.3.0 `federation_keys`
//! trust columns (which v1.6.0 will drop after backfill stabilizes
//! downstream).
//!
//! The old [`FederationDirectory`] trait is still re-exported here
//! for backward compatibility — agent and CIRISPortal flows that
//! still target v1.3.0 trust columns continue to compile. New code
//! should target [`AuditService`] for trust queries.
//!
//! Persist's audit-service [`AuditError`] is distinct from
//! [`crate::substrate::SubstrateError`] (the NodeCoreService failure
//! surface). [`audit_err`] maps the former to the latter so
//! consumers receive a single error type.
//!
//! [`FSD/TRUST_HIERARCHY.md`]: https://github.com/CIRISAI/CIRISNodeCore/blob/main/FSD/TRUST_HIERARCHY.md
//! [`AuditError`]: ciris_persist::audit::Error

use chrono::Utc;

// v1.3.0 FederationDirectory — still re-exported for the deprecated
// V020 trust-columns path. v1.6.0 of persist will drop the columns;
// our consumers should migrate to the AuditService surface below.
pub use ciris_persist::federation::{
    FederationDirectory, TrustFilter, TrustGrant, TrustRelationship, TrustRow, TrustType,
};

// v1.5.x trust-grants surface — now the canonical trust read path.
pub use ciris_persist::audit::AuditService;
pub use ciris_persist::federation::trust_grant::{
    TrustGrantFilter, TrustGrantRow, TrustPurpose, TRUST_GRANT_SUBJECT_KIND,
};

use crate::payloads::registry_vouch::{RegistryVouchPayload, SUBJECT_KIND as VOUCH_SUBJECT_KIND};
use crate::substrate::{ContributionType, ContributionsFilter, NodeCoreService, SubstrateError};

// ── Cross-error mapping ─────────────────────────────────────────────────

/// Map persist's [`ciris_persist::federation::Error`] (the v1.3.0
/// FederationDirectory failure surface) to [`SubstrateError`].
/// Retained for the deprecated V020 trust-columns path.
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
        // Persist v2.5.0+ added admission-discipline variants for the
        // federation directory's reserved-prefix enforcement + envelope
        // schema validation + hardware-attestation gates (FSD-002 v1.4
        // §4.9 + §7.4). NodeCore's deprecated V020 trust-columns path
        // shouldn't be exercising these in practice; catch-all maps to
        // Backend with the formatted variant for diagnostics. Specific
        // mappings can be added per-variant if any of these become
        // load-bearing for trust-columns callers.
        other => SubstrateError::Backend(format!("federation: {other:?}")),
    }
}

/// Map persist's [`ciris_persist::audit::Error`] (the v1.5.x
/// AuditService failure surface) to [`SubstrateError`].
pub fn audit_err(e: ciris_persist::audit::Error) -> SubstrateError {
    // Persist's audit Error variants are richer than substrate's; pin
    // a stable mapping that preserves the kind() token where possible.
    SubstrateError::Backend(format!("audit: {e}"))
}

// ── Node-core policy types + functions ──────────────────────────────────

/// Resolved trust edge — the answer to "is `key` trusted for
/// `domain`?". v1.5.x semantics:
///
/// - [`TrustEdge::Direct`] — a live trust grant exists naming `key`
///   directly with `purpose=Deferral` and matching scope.
/// - [`TrustEdge::Transitive`] — a trusted registry has filed a
///   `registry_vouch` Contribution naming `key` in this domain.
/// - [`TrustEdge::Untrusted`] — no trust edge resolves.
///
/// The v1.3.0-era `TrustEdge::Registry` variant (key has a Registry-
/// relationship V020 row) is gone — the new grant model collapses
/// peer-trust and registry-trust into a single grant shape, with the
/// transitive case mediated by `registry_vouch` Contributions.
#[derive(Debug, Clone, PartialEq)]
pub enum TrustEdge {
    /// `key` has a live `purpose=Deferral` grant covering this domain.
    Direct {
        /// Granter who issued the grant (for audit-trail surfacing).
        granter_key: String,
    },
    /// `key` is trusted because a currently-trusted registry vouched
    /// for it in the queried domain.
    Transitive {
        /// Registry key that vouched. The granter chain that put this
        /// registry in trusted standing is recoverable via
        /// `AuditService::lookup_trust_grant`.
        via_registry: String,
    },
    /// No trust edge resolves.
    Untrusted,
}

/// True if the grant row is currently in force.
pub fn is_active(row: &TrustGrantRow) -> bool {
    let now = Utc::now();
    if row.revoked_at.is_some() {
        return false;
    }
    if let Some(t) = row.expires_at {
        if t <= now {
            return false;
        }
    }
    true
}

/// Resolve the trust edge for `key` under `domain` against the
/// v1.5.x signed-grant projection.
///
/// Algorithm:
/// 1. **Direct** — query `AuditService::lookup_trust_grant` for live
///    grants matching `(grantee=key, purpose=Deferral, scope=domain)`.
///    If any exists (with `revoked_at IS NULL` and not expired),
///    return [`TrustEdge::Direct`].
/// 2. **Transitive** — enumerate live `Deferral` grants for this
///    domain via `list_trust_grants`; for each grantee (potential
///    registry), check whether they've filed an active `registry_vouch`
///    Contribution naming `key` in `domain` via the engine's
///    `list_contributions`. First match wins.
/// 3. Otherwise return [`TrustEdge::Untrusted`].
///
/// Revocation propagates at query time: revoked + expired rows are
/// excluded server-side by `lookup_trust_grant` /
/// `list_trust_grants`. No background sweep needed.
pub async fn resolve_trust<A, E>(
    audit: &A,
    engine: &E,
    key: &str,
    domain: &str,
) -> Result<TrustEdge, SubstrateError>
where
    A: AuditService,
    E: NodeCoreService,
{
    // 1. Direct grant: K is trusted for `purpose=Deferral, scope=domain`.
    let direct_rows = audit
        .lookup_trust_grant(key, TrustPurpose::Deferral, domain, false, false)
        .await
        .map_err(audit_err)?;
    if let Some(row) = direct_rows.into_iter().find(is_active) {
        return Ok(TrustEdge::Direct {
            granter_key: row.granter_key,
        });
    }

    // Wildcard scope `*` for the same purpose also counts as direct.
    let wildcard_rows = audit
        .lookup_trust_grant(key, TrustPurpose::Deferral, "*", false, false)
        .await
        .map_err(audit_err)?;
    if let Some(row) = wildcard_rows.into_iter().find(is_active) {
        return Ok(TrustEdge::Direct {
            granter_key: row.granter_key,
        });
    }

    // 2. Transitive — currently-trusted registries for this domain.
    resolve_transitive(audit, engine, key, domain).await
}

async fn resolve_transitive<A, E>(
    audit: &A,
    engine: &E,
    key: &str,
    domain: &str,
) -> Result<TrustEdge, SubstrateError>
where
    A: AuditService,
    E: NodeCoreService,
{
    let candidates = audit
        .list_trust_grants(TrustGrantFilter {
            purpose: Some(TrustPurpose::Deferral),
            scope_prefix: Some(domain.to_owned()),
            include_revoked: false,
            include_expired: false,
            ..Default::default()
        })
        .await
        .map_err(audit_err)?;
    for grant in candidates {
        // Only exact-scope matches count for transitive (wildcards
        // handled above via direct path).
        if grant.scope != domain {
            continue;
        }
        if registry_vouches_for(engine, &grant.grantee_key, key, domain).await? {
            return Ok(TrustEdge::Transitive {
                via_registry: grant.grantee_key,
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
/// vouches for in `domain`. Used by `route_deferral` to enumerate
/// the candidate set across all trusted registries.
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
        if payload.vouched_domain == domain
            && payload.is_active_at(now)
            && !out.contains(&payload.vouched_key)
        {
            out.push(payload.vouched_key);
        }
    }
    Ok(out)
}
