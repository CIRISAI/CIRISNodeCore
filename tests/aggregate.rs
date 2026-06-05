//! Integration tests for [`ciris_node_core::aggregate::weighted_aggregate`].

mod support;

use chrono::Utc;

use ciris_node_core::aggregate::{
    cohort_weighted_aggregate, weighted_aggregate, Aggregate, OccurrenceCohort,
};
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
    mock.cast_vote(vote("alice", contribution_id, "approve"))
        .await
        .unwrap();
    mock.cast_vote(vote("bob", contribution_id, "approve"))
        .await
        .unwrap();
    mock.cast_vote(vote("carol", contribution_id, "reject"))
        .await
        .unwrap();

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
    mock.cast_vote(vote("alice", contribution_id, "approve"))
        .await
        .unwrap();

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
    assert!(matches!(
        agg,
        Aggregate::BelowQuorum {
            votes_counted: 0,
            ..
        }
    ));
}

// ─── Occurrence-cohort aggregation tests (NodeCore#16) ────────────────

#[tokio::test]
async fn cohort_aggregate_rolls_up_three_occurrences() {
    let mock = MockEngine::new();
    seed_voters(&mock).await;

    // Three occurrences of the same agent template, each with its own
    // Contribution receiving approve/reject votes from the same voter pool.
    let occ_a = "occ-a-key";
    let occ_b = "occ-b-key";
    let occ_c = "occ-c-key";
    let c_a = "01HXC0000000000000000000A";
    let c_b = "01HXC0000000000000000000B";
    let c_c = "01HXC0000000000000000000C";

    // Occurrence A: 100% approval (alice+bob approve, no rejects)
    mock.cast_vote(vote("alice", c_a, "approve")).await.unwrap();
    mock.cast_vote(vote("bob", c_a, "approve")).await.unwrap();
    mock.cast_vote(vote("carol", c_a, "approve")).await.unwrap();
    // Occurrence B: ~70% approval (alice+bob approve = 230; carol rejects = 100)
    mock.cast_vote(vote("alice", c_b, "approve")).await.unwrap();
    mock.cast_vote(vote("bob", c_b, "approve")).await.unwrap();
    mock.cast_vote(vote("carol", c_b, "reject")).await.unwrap();
    // Occurrence C: 0% approval (all reject)
    mock.cast_vote(vote("alice", c_c, "reject")).await.unwrap();
    mock.cast_vote(vote("bob", c_c, "reject")).await.unwrap();
    mock.cast_vote(vote("carol", c_c, "reject")).await.unwrap();

    let cohort = OccurrenceCohort {
        agent_template_id: "ciris-agent-template-v1".into(),
        included_occurrences: vec![occ_a.into(), occ_b.into(), occ_c.into()],
        expected_occurrence_count: Some(3),
    };
    let mapping = vec![
        (occ_a.into(), c_a.into()),
        (occ_b.into(), c_b.into()),
        (occ_c.into(), c_c.into()),
    ];

    let agg = cohort_weighted_aggregate(&mock, &cohort, &mapping, 1)
        .await
        .unwrap();
    assert_eq!(agg.agent_template_id, "ciris-agent-template-v1");
    assert_eq!(agg.included_occurrences.len(), 3);
    assert_eq!(agg.coverage, Some(1.0));
    // Per-occurrence ratios: 1.0, 230/330 ≈ 0.697, 0.0
    let ratios = &agg.per_occurrence_approval_ratio;
    assert!((ratios[0].unwrap() - 1.0).abs() < 1e-9);
    assert!((ratios[1].unwrap() - 230.0 / 330.0).abs() < 1e-9);
    assert!((ratios[2].unwrap() - 0.0).abs() < 1e-9);
    // Mean: (1.0 + 0.697 + 0.0) / 3 ≈ 0.566
    let expected_mean = (1.0 + 230.0 / 330.0 + 0.0) / 3.0;
    assert!((agg.mean_approval_ratio.unwrap() - expected_mean).abs() < 1e-9);
    assert!((agg.min_approval_ratio.unwrap() - 0.0).abs() < 1e-9);
    assert!((agg.max_approval_ratio.unwrap() - 1.0).abs() < 1e-9);
    // Total fleet weight = 330 × 3 = 990
    assert!((agg.total_fleet_weight - 990.0).abs() < 1e-9);
}

#[tokio::test]
async fn cohort_coverage_below_threshold_is_visible() {
    let mock = MockEngine::new();
    seed_voters(&mock).await;
    let c_a = "01HXC0000000000000000000A";
    mock.cast_vote(vote("alice", c_a, "approve")).await.unwrap();
    mock.cast_vote(vote("bob", c_a, "approve")).await.unwrap();
    mock.cast_vote(vote("carol", c_a, "reject")).await.unwrap();

    // Declare fleet of 9 occurrences but only include 1 — coverage = 1/9
    let cohort = OccurrenceCohort {
        agent_template_id: "ciris-agent-template-v1".into(),
        included_occurrences: vec!["occ-a-key".into()],
        expected_occurrence_count: Some(9),
    };
    let mapping = vec![("occ-a-key".into(), c_a.into())];
    let agg = cohort_weighted_aggregate(&mock, &cohort, &mapping, 1)
        .await
        .unwrap();
    assert!((agg.coverage.unwrap() - 1.0 / 9.0).abs() < 1e-9);
    // Selective-inclusion rejection at 0.8 threshold
    assert!(!agg.meets_coverage_threshold(0.8));
    // Passes at a permissive threshold
    assert!(agg.meets_coverage_threshold(0.1));
}

#[tokio::test]
async fn cohort_skips_occurrences_without_contribution_mapping() {
    let mock = MockEngine::new();
    seed_voters(&mock).await;
    let c_a = "01HXC0000000000000000000A";
    mock.cast_vote(vote("alice", c_a, "approve")).await.unwrap();
    mock.cast_vote(vote("bob", c_a, "approve")).await.unwrap();

    // Declare 3 occurrences but supply mapping for only 1 — coverage 1/3
    let cohort = OccurrenceCohort {
        agent_template_id: "ciris-agent-template-v1".into(),
        included_occurrences: vec!["occ-a-key".into(), "occ-b-key".into(), "occ-c-key".into()],
        expected_occurrence_count: Some(3),
    };
    let mapping = vec![("occ-a-key".into(), c_a.into())];
    let agg = cohort_weighted_aggregate(&mock, &cohort, &mapping, 1)
        .await
        .unwrap();
    assert_eq!(agg.included_occurrences.len(), 1);
    assert!((agg.coverage.unwrap() - 1.0 / 3.0).abs() < 1e-9);
}

#[tokio::test]
async fn cohort_empty_set_yields_no_statistics() {
    let mock = MockEngine::new();
    let cohort = OccurrenceCohort {
        agent_template_id: "ciris-agent-template-v1".into(),
        included_occurrences: vec![],
        expected_occurrence_count: Some(0),
    };
    let agg = cohort_weighted_aggregate(&mock, &cohort, &[], 1)
        .await
        .unwrap();
    assert!(agg.included_occurrences.is_empty());
    assert!(agg.mean_approval_ratio.is_none());
    assert!(agg.stddev_approval_ratio.is_none());
    assert_eq!(agg.coverage, Some(0.0));
}

#[tokio::test]
async fn cohort_below_quorum_per_occurrence_excluded_from_stats() {
    let mock = MockEngine::new();
    seed_voters(&mock).await;
    let c_a = "01HXC0000000000000000000A";
    let c_b = "01HXC0000000000000000000B";
    // Occurrence A: 3 votes, passes quorum of 2
    mock.cast_vote(vote("alice", c_a, "approve")).await.unwrap();
    mock.cast_vote(vote("bob", c_a, "approve")).await.unwrap();
    mock.cast_vote(vote("carol", c_a, "approve")).await.unwrap();
    // Occurrence B: 1 vote only, BELOW quorum of 2
    mock.cast_vote(vote("alice", c_b, "approve")).await.unwrap();

    let cohort = OccurrenceCohort {
        agent_template_id: "ciris-agent-template-v1".into(),
        included_occurrences: vec!["occ-a-key".into(), "occ-b-key".into()],
        expected_occurrence_count: Some(2),
    };
    let mapping = vec![
        ("occ-a-key".into(), c_a.into()),
        ("occ-b-key".into(), c_b.into()),
    ];
    let agg = cohort_weighted_aggregate(&mock, &cohort, &mapping, 2)
        .await
        .unwrap();
    // Both occurrences included as cohort members, but B yields no ratio
    assert_eq!(agg.included_occurrences.len(), 2);
    assert_eq!(agg.per_occurrence_approval_ratio[0], Some(1.0));
    assert_eq!(agg.per_occurrence_approval_ratio[1], None);
    // Mean computed only over resolved entries
    assert!((agg.mean_approval_ratio.unwrap() - 1.0).abs() < 1e-9);
}
