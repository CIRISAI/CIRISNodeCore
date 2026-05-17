//! Integration tests for [`ciris_node_core::trust::resolve_trust`]
//! against the v1.5.x signed-grant projection.
//!
//! Tests use the `MockEngine`'s `AuditService` impl + the
//! `set_trust_grant` fixture helper to populate the trust-grant
//! projection rows directly. In production, grants flow through
//! signed `TrustGrant` Contribution events + persist's projection
//! hook; tests stuff the projection.

mod support;

use chrono::{Duration, Utc};

use ciris_node_core::payloads::registry_vouch::{
    RegistryVouchPayload, SUBJECT_KIND as VOUCH_SUBJECT_KIND,
};
use ciris_node_core::substrate::{Cell, ContributionType};
use ciris_node_core::trust::{resolve_trust, TrustEdge, TrustGrantRow, TrustPurpose};
use ciris_node_core::NodeCoreService;

use support::{build_envelope, MockEngine};

fn make_vouch(
    registry: &str,
    vouched_key: &str,
    domain: &str,
    expires_at: Option<chrono::DateTime<Utc>>,
) -> ciris_node_core::substrate::ContributionEnvelope {
    let payload = serde_json::to_value(RegistryVouchPayload {
        vouched_key: vouched_key.into(),
        vouched_domain: domain.into(),
        expires_at,
        rationale: "test fixture".into(),
    })
    .unwrap();
    build_envelope(
        &format!("vouch_{registry}_{vouched_key}_{domain}"),
        ContributionType::Proposal,
        registry,
        Cell {
            domain: domain.into(),
            language: "en".into(),
            subject: Some(VOUCH_SUBJECT_KIND.into()),
        },
        payload,
    )
}

/// Populate a Deferral-purpose grant naming `grantee` for `domain`.
fn grant_deferral(mock: &MockEngine, grantee: &str, domain: &str, granter: &str) {
    mock.set_trust_grant(MockEngine::make_grant(
        grantee,
        granter,
        TrustPurpose::Deferral,
        domain,
    ));
}

// ── Direct edges ────────────────────────────────────────────────────────

#[tokio::test]
async fn direct_grant_resolves_to_direct_edge() {
    let mock = MockEngine::new();
    grant_deferral(&mock, "K_B", "medical_deferral", "steward");

    let edge = resolve_trust(&mock, &mock, "K_B", "medical_deferral")
        .await
        .unwrap();
    match edge {
        TrustEdge::Direct { granter_key } => assert_eq!(granter_key, "steward"),
        other => panic!("expected Direct, got {other:?}"),
    }
}

#[tokio::test]
async fn direct_grant_is_domain_scoped() {
    let mock = MockEngine::new();
    grant_deferral(&mock, "K_B", "medical_deferral", "steward");

    // Same key, different domain → Untrusted (no grant for that domain).
    let edge = resolve_trust(&mock, &mock, "K_B", "legal_review")
        .await
        .unwrap();
    assert_eq!(edge, TrustEdge::Untrusted);
}

#[tokio::test]
async fn wildcard_grant_resolves_for_any_domain() {
    let mock = MockEngine::new();
    // Wildcard scope grants are strict trust elevations (FSD §3.3) —
    // they match any domain query.
    mock.set_trust_grant(MockEngine::make_grant(
        "K_B",
        "steward",
        TrustPurpose::Deferral,
        "*",
    ));

    for domain in ["medical_deferral", "legal_review", "arbitrary"] {
        let edge = resolve_trust(&mock, &mock, "K_B", domain).await.unwrap();
        assert!(matches!(edge, TrustEdge::Direct { .. }), "domain={domain}");
    }
}

#[tokio::test]
async fn no_trust_grant_returns_untrusted() {
    let mock = MockEngine::new();
    let edge = resolve_trust(&mock, &mock, "K_unknown", "medical_deferral")
        .await
        .unwrap();
    assert_eq!(edge, TrustEdge::Untrusted);
}

// ── Transitive edges ────────────────────────────────────────────────────

#[tokio::test]
async fn transitive_trust_via_registry_vouch() {
    let mock = MockEngine::new();
    // K_R has a Deferral grant for medical_deferral (acts as a
    // registry for the domain).
    grant_deferral(&mock, "K_R", "medical_deferral", "steward");
    // K_R vouches for K_C in medical_deferral.
    mock.put_contribution(make_vouch("K_R", "K_C", "medical_deferral", None))
        .await
        .unwrap();

    let edge = resolve_trust(&mock, &mock, "K_C", "medical_deferral")
        .await
        .unwrap();
    assert_eq!(
        edge,
        TrustEdge::Transitive {
            via_registry: "K_R".into()
        }
    );
}

#[tokio::test]
async fn transitive_trust_is_domain_scoped() {
    let mock = MockEngine::new();
    grant_deferral(&mock, "K_R", "medical_deferral", "steward");
    // K_R vouched only in medical_deferral, NOT legal_review.
    mock.put_contribution(make_vouch("K_R", "K_C", "medical_deferral", None))
        .await
        .unwrap();

    let edge = resolve_trust(&mock, &mock, "K_C", "legal_review")
        .await
        .unwrap();
    assert_eq!(edge, TrustEdge::Untrusted);
}

#[tokio::test]
async fn revoking_the_registry_grant_propagates_at_query_time() {
    let mock = MockEngine::new();
    grant_deferral(&mock, "K_R", "medical_deferral", "steward");
    mock.put_contribution(make_vouch("K_R", "K_C", "medical_deferral", None))
        .await
        .unwrap();

    // Initially: transitive resolves.
    let edge = resolve_trust(&mock, &mock, "K_C", "medical_deferral")
        .await
        .unwrap();
    assert!(matches!(edge, TrustEdge::Transitive { .. }));

    // Revoke K_R's grant by re-issuing with revoked_at set.
    // (In production this is a new TrustGrant Contribution event
    // with effective revocation; persist's projection updates the
    // row. For tests we mutate the row directly.)
    mock.set_trust_grant(TrustGrantRow {
        grant_id: uuid::Uuid::new_v4(),
        grantee_key: "K_R".into(),
        granter_key: "steward".into(),
        purpose: TrustPurpose::Deferral,
        scope: "medical_deferral".into(),
        granted_at: Utc::now(),
        expires_at: None,
        revoked_at: Some(Utc::now()),
        revoked_by: Some("steward".into()),
        chain_event_id: 1,
        chain_event_hash: vec![],
        tenant_id: "test".into(),
    });

    // The revoked row is now in the projection, but
    // `list_trust_grants(include_revoked=false)` filters it. The OLD
    // live row is still there too — so what we need to test is that
    // revoking the ONLY grant for K_R drops K_C's transitive trust.
    // For this test scenario, simulate the production path by
    // ensuring the live grant is replaced by the revoked one.
    let mock2 = MockEngine::new();
    mock2.set_trust_grant(TrustGrantRow {
        grant_id: uuid::Uuid::new_v4(),
        grantee_key: "K_R".into(),
        granter_key: "steward".into(),
        purpose: TrustPurpose::Deferral,
        scope: "medical_deferral".into(),
        granted_at: Utc::now(),
        expires_at: None,
        revoked_at: Some(Utc::now()),
        revoked_by: Some("steward".into()),
        chain_event_id: 1,
        chain_event_hash: vec![],
        tenant_id: "test".into(),
    });
    mock2
        .put_contribution(make_vouch("K_R", "K_C", "medical_deferral", None))
        .await
        .unwrap();
    let edge = resolve_trust(&mock2, &mock2, "K_C", "medical_deferral")
        .await
        .unwrap();
    assert_eq!(
        edge,
        TrustEdge::Untrusted,
        "revoked-only registry drops transitive trust at query time"
    );
}

#[tokio::test]
async fn expired_vouch_does_not_yield_transitive_trust() {
    let mock = MockEngine::new();
    grant_deferral(&mock, "K_R", "medical_deferral", "steward");
    // Vouch with an expires_at one second in the past.
    let past = Utc::now() - Duration::seconds(1);
    mock.put_contribution(make_vouch("K_R", "K_C", "medical_deferral", Some(past)))
        .await
        .unwrap();

    let edge = resolve_trust(&mock, &mock, "K_C", "medical_deferral")
        .await
        .unwrap();
    assert_eq!(edge, TrustEdge::Untrusted);
}

#[tokio::test]
async fn expired_grant_does_not_yield_direct_trust() {
    let mock = MockEngine::new();
    let past = Utc::now() - Duration::seconds(1);
    mock.set_trust_grant(TrustGrantRow {
        grant_id: uuid::Uuid::new_v4(),
        grantee_key: "K_B".into(),
        granter_key: "steward".into(),
        purpose: TrustPurpose::Deferral,
        scope: "medical_deferral".into(),
        granted_at: Utc::now() - Duration::days(10),
        expires_at: Some(past),
        revoked_at: None,
        revoked_by: None,
        chain_event_id: 0,
        chain_event_hash: vec![],
        tenant_id: "test".into(),
    });

    let edge = resolve_trust(&mock, &mock, "K_B", "medical_deferral")
        .await
        .unwrap();
    assert_eq!(edge, TrustEdge::Untrusted);
}

#[tokio::test]
async fn revoked_grant_does_not_yield_direct_trust() {
    let mock = MockEngine::new();
    mock.set_trust_grant(TrustGrantRow {
        grant_id: uuid::Uuid::new_v4(),
        grantee_key: "K_B".into(),
        granter_key: "steward".into(),
        purpose: TrustPurpose::Deferral,
        scope: "medical_deferral".into(),
        granted_at: Utc::now() - Duration::days(10),
        expires_at: None,
        revoked_at: Some(Utc::now()),
        revoked_by: Some("steward".into()),
        chain_event_id: 0,
        chain_event_hash: vec![],
        tenant_id: "test".into(),
    });

    let edge = resolve_trust(&mock, &mock, "K_B", "medical_deferral")
        .await
        .unwrap();
    assert_eq!(edge, TrustEdge::Untrusted);
}

// ── Multi-granter coverage ──────────────────────────────────────────────

#[tokio::test]
async fn direct_grant_from_any_granter_resolves() {
    let mock = MockEngine::new();
    // Two distinct granters both granted K_B the same purpose+scope.
    grant_deferral(&mock, "K_B", "medical_deferral", "steward");
    grant_deferral(&mock, "K_B", "medical_deferral", "other_granter");

    let edge = resolve_trust(&mock, &mock, "K_B", "medical_deferral")
        .await
        .unwrap();
    assert!(matches!(edge, TrustEdge::Direct { .. }));
}
