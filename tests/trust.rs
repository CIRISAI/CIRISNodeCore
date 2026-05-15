//! Integration tests for [`ciris_node_core::trust::resolve_trust`]
//! against the in-memory [`MockEngine`] (which impls both
//! `NodeCoreService` AND `FederationDirectory`).

mod support;

use chrono::{Duration, Utc};

use ciris_node_core::payloads::registry_vouch::{
    RegistryVouchPayload, SUBJECT_KIND as VOUCH_SUBJECT_KIND,
};
use ciris_node_core::substrate::{Cell, ContributionType};
use ciris_node_core::trust::{
    resolve_trust, FederationDirectory, TrustEdge, TrustGrant, TrustRelationship, TrustType,
};

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

async fn grant(
    mock: &MockEngine,
    key: &str,
    rel: TrustRelationship,
    domains: Option<Vec<&str>>,
    trusted_by: &str,
) {
    mock.grant_trust(TrustGrant {
        key: key.into(),
        trust_type: TrustType::Temporary,
        trust_relationship: rel,
        trust_domains: domains.map(|v| v.into_iter().map(String::from).collect()),
        trusted_by: trusted_by.into(),
        expires_at: None,
    })
    .await
    .unwrap();
}

// ── Direct edges ────────────────────────────────────────────────────────

#[tokio::test]
async fn direct_grant_resolves_to_direct_edge() {
    let mock = MockEngine::new();
    grant(&mock, "K_B", TrustRelationship::Direct, None, "steward").await;

    let edge = resolve_trust(&mock, &mock, "K_B", "medical_deferral")
        .await
        .unwrap();
    assert!(matches!(edge, TrustEdge::Direct { trust_type: TrustType::Temporary }));
}

#[tokio::test]
async fn direct_grant_resolves_for_any_domain() {
    let mock = MockEngine::new();
    grant(&mock, "K_B", TrustRelationship::Direct, None, "steward").await;

    // Direct trust isn't domain-scoped; the same edge resolves for
    // any domain query.
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

// ── Registry edges ──────────────────────────────────────────────────────

#[tokio::test]
async fn registry_grant_resolves_within_declared_domain() {
    let mock = MockEngine::new();
    grant(
        &mock,
        "K_R",
        TrustRelationship::Registry,
        Some(vec!["medical_deferral"]),
        "steward",
    )
    .await;

    let edge = resolve_trust(&mock, &mock, "K_R", "medical_deferral")
        .await
        .unwrap();
    assert!(matches!(
        edge,
        TrustEdge::Registry { trust_type: TrustType::Temporary, .. }
    ));
}

#[tokio::test]
async fn registry_grant_does_not_resolve_outside_declared_domains() {
    let mock = MockEngine::new();
    grant(
        &mock,
        "K_R",
        TrustRelationship::Registry,
        Some(vec!["medical_deferral"]),
        "steward",
    )
    .await;

    // Same registry asked about a domain it doesn't cover.
    let edge = resolve_trust(&mock, &mock, "K_R", "legal_review")
        .await
        .unwrap();
    assert_eq!(edge, TrustEdge::Untrusted);
}

// ── Transitive edges (the WA-routing seam) ──────────────────────────────

#[tokio::test]
async fn transitive_trust_via_registry_vouch() {
    use ciris_node_core::NodeCoreService;
    let mock = MockEngine::new();

    // A trusts K_R as a medical-deferral registry.
    grant(
        &mock,
        "K_R",
        TrustRelationship::Registry,
        Some(vec!["medical_deferral"]),
        "steward",
    )
    .await;
    // K_R vouches for K_C in medical_deferral.
    mock.put_contribution(make_vouch("K_R", "K_C", "medical_deferral", None))
        .await
        .unwrap();

    let edge = resolve_trust(&mock, &mock, "K_C", "medical_deferral")
        .await
        .unwrap();
    assert_eq!(edge, TrustEdge::Transitive { via_registry: "K_R".into() });
}

#[tokio::test]
async fn transitive_trust_is_domain_scoped() {
    use ciris_node_core::NodeCoreService;
    let mock = MockEngine::new();
    grant(
        &mock,
        "K_R",
        TrustRelationship::Registry,
        Some(vec!["medical_deferral"]),
        "steward",
    )
    .await;
    // K_R only vouched in medical_deferral, NOT legal_review.
    mock.put_contribution(make_vouch("K_R", "K_C", "medical_deferral", None))
        .await
        .unwrap();

    // Query for K_C in a DIFFERENT domain → Untrusted (the
    // domain-scoping rule from FSD §3).
    let edge = resolve_trust(&mock, &mock, "K_C", "legal_review")
        .await
        .unwrap();
    assert_eq!(edge, TrustEdge::Untrusted);
}

#[tokio::test]
async fn revoking_the_registry_propagates_at_query_time() {
    use ciris_node_core::NodeCoreService;
    let mock = MockEngine::new();
    grant(
        &mock,
        "K_R",
        TrustRelationship::Registry,
        Some(vec!["medical_deferral"]),
        "steward",
    )
    .await;
    mock.put_contribution(make_vouch("K_R", "K_C", "medical_deferral", None))
        .await
        .unwrap();

    // Initially: transitive trust resolves.
    let edge = resolve_trust(&mock, &mock, "K_C", "medical_deferral")
        .await
        .unwrap();
    assert!(matches!(edge, TrustEdge::Transitive { .. }));

    // Revoke K_R. The vouch row stays on the audit chain; the
    // resolver re-reads K_R's current trust on each query.
    mock.revoke_trust("K_R", "steward").await.unwrap();

    let edge = resolve_trust(&mock, &mock, "K_C", "medical_deferral")
        .await
        .unwrap();
    assert_eq!(
        edge,
        TrustEdge::Untrusted,
        "revoking K_R drops K_C's transitive trust at query time"
    );
}

#[tokio::test]
async fn expired_vouch_does_not_yield_transitive_trust() {
    use ciris_node_core::NodeCoreService;
    let mock = MockEngine::new();
    grant(
        &mock,
        "K_R",
        TrustRelationship::Registry,
        Some(vec!["medical_deferral"]),
        "steward",
    )
    .await;
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

// ── Integrity rules ─────────────────────────────────────────────────────

#[tokio::test]
async fn self_trust_grant_rejected_at_directory_boundary() {
    let mock = MockEngine::new();
    let result = mock
        .grant_trust(TrustGrant {
            key: "K_self".into(),
            trust_type: TrustType::Temporary,
            trust_relationship: TrustRelationship::Direct,
            trust_domains: None,
            trusted_by: "K_self".into(), // == key → integrity violation
            expires_at: None,
        })
        .await;
    assert!(matches!(
        result,
        Err(ciris_persist::federation::Error::InvalidArgument(_))
    ));
}

#[tokio::test]
async fn registry_grant_requires_non_empty_domains() {
    let mock = MockEngine::new();
    let result = mock
        .grant_trust(TrustGrant {
            key: "K_R".into(),
            trust_type: TrustType::Temporary,
            trust_relationship: TrustRelationship::Registry,
            trust_domains: None, // registry without domains → invalid
            trusted_by: "steward".into(),
            expires_at: None,
        })
        .await;
    assert!(matches!(
        result,
        Err(ciris_persist::federation::Error::InvalidArgument(_))
    ));
}
