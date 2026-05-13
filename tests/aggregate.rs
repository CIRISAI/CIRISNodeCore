//! Integration tests for [`ciris_node_core::aggregate::weighted_aggregate`].

mod support;

use chrono::Utc;

use ciris_node_core::aggregate::{weighted_aggregate, Aggregate};
use ciris_node_core::substrate::{Cell, VoteEnvelope};
use ciris_node_core::NodeCoreService;

use support::{placeholder_signature, MockEngine};

fn vote(voter: &str, contribution_id: &str, verdict: &str) -> VoteEnvelope {
    VoteEnvelope {
        vote_id: format!("v_{voter}"),
        voter_id: voter.into(),
        contribution_id: Some(contribution_id.into()),
        cell: Cell {
            domain: "mental_health".into(),
            language: "am".into(),
            subject: Some("arc_question".into()),
        },
        score: serde_json::json!({"verdict": verdict, "magnitude": 1.0}),
        rationale: None,
        signature: placeholder_signature(),
        cast_at: Utc::now(),
    }
}

async fn seed_voters(mock: &MockEngine) {
    // Three voters with different weight profiles.
    mock.set_credits("alice", "mental_health", "am", "arc_question", 100.0);
    mock.set_expertise("alice", "mental_health", "am", 1.5, true);

    mock.set_credits("bob", "mental_health", "am", "arc_question", 80.0);
    mock.set_expertise("bob", "mental_health", "am", 1.0, true);

    mock.set_credits("carol", "mental_health", "am", "arc_question", 50.0);
    mock.set_expertise("carol", "mental_health", "am", 2.0, true);

    // alice weight = 100 * 1.5 * 1.0 = 150
    // bob   weight =  80 * 1.0 * 1.0 = 80
    // carol weight =  50 * 2.0 * 1.0 = 100
}

#[tokio::test]
async fn resolved_aggregate_sums_weights_by_verdict() {
    let mock = MockEngine::new();
    seed_voters(&mock).await;

    let contribution_id = "01HXC0000000000000000000A";
    mock.cast_vote(vote("alice", contribution_id, "approve")).await.unwrap();
    mock.cast_vote(vote("bob", contribution_id, "approve")).await.unwrap();
    mock.cast_vote(vote("carol", contribution_id, "reject")).await.unwrap();

    let agg = weighted_aggregate(&mock, contribution_id, 3).await.unwrap();
    match agg {
        Aggregate::Resolved {
            votes_counted,
            approve_weight,
            reject_weight,
            abstain_weight,
            ..
        } => {
            assert_eq!(votes_counted, 3);
            assert!((approve_weight - 230.0).abs() < 1e-9, "alice+bob = 150+80");
            assert!((reject_weight - 100.0).abs() < 1e-9, "carol = 100");
            assert_eq!(abstain_weight, 0.0);
        }
        other => panic!("expected Resolved, got {other:?}"),
    }
}

#[tokio::test]
async fn below_quorum_does_not_emit_numeric_aggregate() {
    let mock = MockEngine::new();
    seed_voters(&mock).await;

    let contribution_id = "01HXC0000000000000000000B";
    mock.cast_vote(vote("alice", contribution_id, "approve")).await.unwrap();

    let agg = weighted_aggregate(&mock, contribution_id, 3).await.unwrap();
    match agg {
        Aggregate::BelowQuorum {
            votes_counted,
            minimum_required,
            ..
        } => {
            assert_eq!(votes_counted, 1);
            assert_eq!(minimum_required, 3);
        }
        other => panic!("expected BelowQuorum, got {other:?}"),
    }
    // Fail-secure: aggregate methods return None for non-Resolved.
    assert!(agg.total_weight().is_none());
    assert!(agg.approval_ratio().is_none());
}

#[tokio::test]
async fn approval_ratio_excludes_abstains() {
    let mock = MockEngine::new();
    seed_voters(&mock).await;

    let cid = "01HXC0000000000000000000C";
    mock.cast_vote(vote("alice", cid, "approve")).await.unwrap();
    mock.cast_vote(vote("bob", cid, "approve")).await.unwrap();
    mock.cast_vote(vote("carol", cid, "abstain")).await.unwrap();

    let agg = weighted_aggregate(&mock, cid, 3).await.unwrap();
    // approve=230, reject=0, abstain=100
    // ratio = 230 / (230 + 0) = 1.0 (abstain excluded from denom)
    assert_eq!(agg.approval_ratio(), Some(1.0));
    assert_eq!(agg.total_weight(), Some(330.0));
}

#[tokio::test]
async fn malformed_score_is_silently_skipped() {
    let mock = MockEngine::new();
    seed_voters(&mock).await;

    let cid = "01HXC0000000000000000000D";
    // One valid + one malformed (no verdict field).
    mock.cast_vote(vote("alice", cid, "approve")).await.unwrap();
    let bad = VoteEnvelope {
        vote_id: "v_bad".into(),
        voter_id: "bob".into(),
        contribution_id: Some(cid.into()),
        cell: Cell {
            domain: "mental_health".into(),
            language: "am".into(),
            subject: Some("arc_question".into()),
        },
        score: serde_json::json!({"score_kind": "battery_response", "outcome": "pass"}),
        rationale: None,
        signature: placeholder_signature(),
        cast_at: Utc::now(),
    };
    mock.cast_vote(bad).await.unwrap();

    // Quorum=1 because only alice's vote counted; bob's was skipped.
    let agg = weighted_aggregate(&mock, cid, 1).await.unwrap();
    match agg {
        Aggregate::Resolved {
            votes_counted,
            approve_weight,
            ..
        } => {
            assert_eq!(votes_counted, 1, "malformed score skipped");
            assert!((approve_weight - 150.0).abs() < 1e-9, "only alice counted");
        }
        other => panic!("expected Resolved, got {other:?}"),
    }
}

#[tokio::test]
async fn empty_vote_set_is_below_quorum() {
    let mock = MockEngine::new();
    let agg = weighted_aggregate(&mock, "01HXC0000000000000000000E", 1)
        .await
        .unwrap();
    assert!(matches!(agg, Aggregate::BelowQuorum { votes_counted: 0, .. }));
}
