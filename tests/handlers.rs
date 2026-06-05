//! Integration tests for [`NodeCore`]'s public service methods,
//! against the v0.7.4 substrate.
//!
//! Handler `Handler<M>` impls in `src/service.rs` are thin shells over
//! these public methods, so testing the methods covers the handler
//! logic end-to-end.

mod support;

use std::sync::Arc;

use chrono::Utc;

use ciris_node_core::substrate::{
    Cell, ContributionType, CreditsUpdate, ExpertiseUpdate, RoutableContributor, SubstrateError,
    VoteEnvelope,
};
use ciris_node_core::NodeCore;

use support::{build_envelope, placeholder_signature, MockEngine};

// ── Deferral routing ────────────────────────────────────────────────────

#[tokio::test]
async fn deferral_routes_bounded_by_max() {
    let mock = Arc::new(MockEngine::new());
    mock.set_routable(
        "mental_health",
        "am",
        vec![
            RoutableContributor {
                contributor_id: "alice".into(),
                expertise: 0.9,
            },
            RoutableContributor {
                contributor_id: "bob".into(),
                expertise: 0.7,
            },
            RoutableContributor {
                contributor_id: "carol".into(),
                expertise: 0.5,
            },
            RoutableContributor {
                contributor_id: "dan".into(),
                expertise: 0.4,
            },
        ],
    );

    let core = NodeCore::new(mock.clone());

    let payload = serde_json::json!({
        "title": "Stage-2 register check",
        "context": "...",
        "response_format": "binary",
        "routing_preferences": { "min_responders": 1, "max_responders": 2, "diversity": "none" }
    });
    let env = build_envelope(
        "01HXDEFER0000000000000001",
        ContributionType::DeferralRequest,
        "agent_pub",
        Cell {
            domain: "mental_health".into(),
            language: "am".into(),
            subject: None,
        },
        payload,
    );

    let routing = core.submit_deferral(env).await.unwrap();

    assert_eq!(routing.deferral_id, "01HXDEFER0000000000000001");
    assert_eq!(routing.routed_responders.len(), 2);
    assert_eq!(routing.routed_responders[0], "alice");
    assert_eq!(routing.routed_responders[1], "bob");

    // The request envelope itself got persisted as a Contribution row.
    assert_eq!(mock.contributions().len(), 1);
}

#[tokio::test]
async fn deferral_with_no_routable_contributors_returns_empty() {
    let mock = Arc::new(MockEngine::new());
    let core = NodeCore::new(mock.clone());

    let env = build_envelope(
        "01HXDEFER0000000000000002",
        ContributionType::DeferralRequest,
        "agent_pub",
        Cell {
            domain: "mental_health".into(),
            language: "ur".into(),
            subject: None,
        },
        serde_json::json!({
            "title": "...",
            "context": "...",
            "response_format": "binary"
        }),
    );
    let routing = core.submit_deferral(env).await.unwrap();
    assert!(routing.routed_responders.is_empty());
}

#[tokio::test]
async fn deferral_defaults_to_max_9_when_preferences_omitted() {
    let mock = Arc::new(MockEngine::new());
    mock.set_routable(
        "mental_health",
        "am",
        (0..15)
            .map(|i| RoutableContributor {
                contributor_id: format!("c{i}"),
                expertise: 0.5,
            })
            .collect(),
    );

    let core = NodeCore::new(mock.clone());
    let env = build_envelope(
        "01HXDEFER0000000000000003",
        ContributionType::DeferralRequest,
        "agent_pub",
        Cell {
            domain: "mental_health".into(),
            language: "am".into(),
            subject: None,
        },
        serde_json::json!({"title": "x", "context": "x", "response_format": "binary"}),
    );
    let routing = core.submit_deferral(env).await.unwrap();
    assert_eq!(
        routing.routed_responders.len(),
        9,
        "default cap per §3.3 step 4"
    );
}

// ── Vote weight ─────────────────────────────────────────────────────────

#[tokio::test]
async fn record_vote_returns_cast_time_weight() {
    let mock = Arc::new(MockEngine::new());
    mock.set_credits("voter", "mental_health", "am", "arc_question", 100.0);
    mock.set_expertise("voter", "mental_health", "am", 1.5, true);

    let core = NodeCore::new(mock.clone());

    let vote = VoteEnvelope {
        vote_id: "01HXVOTE10000000000000000".into(),
        voter_id: "voter".into(),
        contribution_id: Some("01HXC0000000000000000000C".into()),
        cell: Cell {
            domain: "mental_health".into(),
            language: "am".into(),
            subject: Some("arc_question".into()),
        },
        score: serde_json::json!({"verdict": "approve", "magnitude": 1.0}),
        rationale: Some("LGTM".into()),
        signature: placeholder_signature(),
        cast_at: Utc::now(),
    };

    let ack = core.record_vote(vote).await.unwrap();
    assert_eq!(ack.weight.credits, 100.0);
    assert_eq!(ack.weight.expertise_multiplier, 1.5);
    assert_eq!(ack.weight.active_tier_multiplier, 1.0);
    assert_eq!(ack.weight.weight, 150.0);
    assert_eq!(mock.votes().len(), 1);
}

#[tokio::test]
async fn below_active_tier_vote_has_zero_weight() {
    let mock = Arc::new(MockEngine::new());
    mock.set_credits("dormant", "mental_health", "am", "arc_question", 50.0);
    mock.set_expertise("dormant", "mental_health", "am", 2.0, false);

    let core = NodeCore::new(mock.clone());
    let vote = VoteEnvelope {
        vote_id: "01HXVOTE20000000000000000".into(),
        voter_id: "dormant".into(),
        contribution_id: Some("01HXC0000000000000000000C".into()),
        cell: Cell {
            domain: "mental_health".into(),
            language: "am".into(),
            subject: Some("arc_question".into()),
        },
        score: serde_json::json!({"verdict": "approve"}),
        rationale: None,
        signature: placeholder_signature(),
        cast_at: Utc::now(),
    };
    let ack = core.record_vote(vote).await.unwrap();
    assert_eq!(ack.weight.weight, 0.0, "below-Active zeros weight per §3.8");
}

// ── Ledger invariants ───────────────────────────────────────────────────

#[tokio::test]
async fn credits_update_rejects_negative_balance() {
    use ciris_node_core::NodeCoreService;
    let mock = MockEngine::new();
    let result = mock
        .update_credits_ledger(CreditsUpdate {
            contributor_id: "alice".into(),
            domain: "mental_health".into(),
            language: "am".into(),
            subject: "arc_question".into(),
            new_balance: -5.0,
            source_contribution: "01HX00000000000000000000C".into(),
        })
        .await;
    assert!(matches!(result, Err(SubstrateError::InvalidArgument(_))));
}

#[tokio::test]
async fn expertise_update_rejects_negative() {
    use ciris_node_core::NodeCoreService;
    let mock = MockEngine::new();
    let result = mock
        .update_expertise_ledger(ExpertiseUpdate {
            contributor_id: "alice".into(),
            domain: "mental_health".into(),
            language: "am".into(),
            new_expertise: -0.1,
            new_active_tier: true,
            source_contribution: "01HX00000000000000000000E".into(),
        })
        .await;
    assert!(matches!(result, Err(SubstrateError::InvalidArgument(_))));
}
