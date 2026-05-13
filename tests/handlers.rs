//! Integration tests for [`NodeCore`]'s public service methods.
//!
//! Exercises the handler-side logic against [`support::MockEngine`].
//! No `Edge` instance is constructed — the `Handler<M>` impls in
//! `src/service.rs` are thin shells over these public methods, so
//! testing the methods covers the handler logic end-to-end.

mod support;

use std::sync::Arc;

use chrono::Utc;

use ciris_node_core::{
    Cell, ContributorId, Error, HybridSignature, NodeCore, NodeCoreEngine, Score, Vote,
};
use ciris_node_core::payloads::deferral::{
    DeferralRequest, DiversityPolicy, ResponseFormat, RoutingPreferences,
};

use support::MockEngine;

// ── Deferral routing ────────────────────────────────────────────────────

#[tokio::test]
async fn deferral_routes_to_expertise_holders_bounded_by_max() {
    let mock = Arc::new(MockEngine::new());
    let alice = ContributorId::new("alice");
    let bob = ContributorId::new("bob");
    let carol = ContributorId::new("carol");
    let dan = ContributorId::new("dan");

    mock.set_routable(
        "mental_health",
        "am",
        vec![alice.clone(), bob.clone(), carol.clone(), dan.clone()],
    );

    let core = NodeCore::new(mock.clone());
    let req = sample_deferral_request("mental_health", "am", Some(2));

    let routing = core.submit_deferral(req).await.unwrap();

    assert_eq!(routing.deferral_id, "def_01");
    assert_eq!(routing.routed_responders.len(), 2, "max_responders gate");
    assert_eq!(routing.routed_responders[0], alice);
    assert_eq!(routing.routed_responders[1], bob);
}

#[tokio::test]
async fn deferral_with_no_routable_responders_returns_empty_set() {
    let mock = Arc::new(MockEngine::new());
    // intentionally don't call set_routable

    let core = NodeCore::new(mock.clone());
    let routing = core
        .submit_deferral(sample_deferral_request("mental_health", "am", None))
        .await
        .unwrap();

    assert!(routing.routed_responders.is_empty());
}

#[tokio::test]
async fn deferral_defaults_to_max_9_when_preferences_omitted() {
    let mock = Arc::new(MockEngine::new());
    let many: Vec<ContributorId> = (0..15)
        .map(|i| ContributorId::new(format!("c{i}")))
        .collect();
    mock.set_routable("mental_health", "am", many);

    let core = NodeCore::new(mock.clone());
    let mut req = sample_deferral_request("mental_health", "am", None);
    req.routing_preferences = None; // no preferences → §3.3 default cap (9)

    let routing = core.submit_deferral(req).await.unwrap();
    assert_eq!(routing.routed_responders.len(), 9, "default max per §3.3 step 4");
}

// ── Vote weight ─────────────────────────────────────────────────────────

#[tokio::test]
async fn record_vote_returns_cast_time_weight() {
    let mock = Arc::new(MockEngine::new());
    let voter = ContributorId::new("voter");
    mock.set_credits(&voter, "mental_health", "am", "arc_question", 100.0);
    mock.set_expertise(&voter, "mental_health", "am", 1.5);
    mock.set_active(&voter, true);

    let core = NodeCore::new(mock.clone());

    let vote = Vote {
        vote_id: "v_01".into(),
        voter_id: voter.clone(),
        contribution_id: "c_01".into(),
        cell: Cell::credits("mental_health", "am", "arc_question"),
        score: Score(serde_json::json!({"score_kind": "proposal_adoption", "verdict": "approve", "magnitude": 1.0})),
        rationale: Some("LGTM".into()),
        signature: placeholder_sig(),
        cast_at: Utc::now(),
    };

    let ack = core.record_vote(vote).await.unwrap();
    assert_eq!(ack.weight.credits, 100.0);
    assert_eq!(ack.weight.expertise_multiplier, 1.5);
    assert_eq!(ack.weight.active_tier_multiplier, 1.0);
    assert_eq!(ack.weight.effective(), 150.0);
    assert_eq!(mock.votes().len(), 1, "vote was persisted");
}

#[tokio::test]
async fn vote_from_below_active_tier_contributor_carries_zero_weight() {
    let mock = Arc::new(MockEngine::new());
    let voter = ContributorId::new("dormant");
    mock.set_credits(&voter, "mental_health", "am", "arc_question", 50.0);
    mock.set_expertise(&voter, "mental_health", "am", 2.0);
    // explicitly NOT setting active

    let core = NodeCore::new(mock.clone());
    let vote = Vote {
        vote_id: "v_02".into(),
        voter_id: voter.clone(),
        contribution_id: "c_01".into(),
        cell: Cell::credits("mental_health", "am", "arc_question"),
        score: Score(serde_json::json!({"verdict": "approve"})),
        rationale: None,
        signature: placeholder_sig(),
        cast_at: Utc::now(),
    };

    let ack = core.record_vote(vote).await.unwrap();
    assert_eq!(
        ack.weight.effective(),
        0.0,
        "below-Active-tier zeros the weight per §3.8"
    );
}

// ── Ledger invariants ───────────────────────────────────────────────────

#[tokio::test]
async fn credits_ledger_enforces_non_negative_floor() {
    let mock = MockEngine::new();
    let alice = ContributorId::new("alice");
    let cell = Cell::credits("mental_health", "am", "arc_question");

    mock.update_credits_ledger(&alice, &cell, 10.0).await.unwrap();

    let try_underflow = mock.update_credits_ledger(&alice, &cell, -20.0).await;
    assert!(matches!(try_underflow, Err(Error::LedgerInvariant(_))));

    // Partial reduction stays above floor — OK.
    mock.update_credits_ledger(&alice, &cell, -5.0).await.unwrap();

    // Verify ledger ended at 5.0 via the read view.
    let ledger = mock.get_credits_ledger(&alice).await.unwrap();
    let entry = ledger
        .entries
        .iter()
        .find(|e| e.cell.subject.as_deref() == Some("arc_question"))
        .expect("entry exists");
    assert_eq!(entry.credits, 5.0);
}

#[tokio::test]
async fn expertise_ledger_enforces_non_negative_floor() {
    let mock = MockEngine::new();
    let alice = ContributorId::new("alice");
    let cell = Cell::expertise("mental_health", "am");

    mock.update_expertise_ledger(&alice, &cell, 0.5).await.unwrap();

    let try_underflow = mock.update_expertise_ledger(&alice, &cell, -1.0).await;
    assert!(matches!(try_underflow, Err(Error::LedgerInvariant(_))));
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn sample_deferral_request(
    domain: &str,
    language: &str,
    max_responders: Option<u32>,
) -> DeferralRequest {
    DeferralRequest {
        deferral_id: "def_01".into(),
        cell: Cell::expertise(domain, language),
        consumer_id: ContributorId::new("agent_pub"),
        agent_task_id: None,
        title: "Test deferral".into(),
        context: "Should this question route to Active-tier am-cell experts?".into(),
        response_format: ResponseFormat::Binary,
        deadline: None,
        routing_preferences: max_responders.map(|m| RoutingPreferences {
            min_responders: Some(1),
            max_responders: Some(m),
            diversity: Some(DiversityPolicy::None),
        }),
    }
}

fn placeholder_sig() -> HybridSignature {
    HybridSignature {
        ed25519: "placeholder".into(),
        ml_dsa_65: None,
        signed_at: Utc::now(),
    }
}
