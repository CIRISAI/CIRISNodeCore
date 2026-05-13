//! Integration tests for [`ciris_node_core::routing::select_routed`].

mod support;

use std::collections::HashMap;

use ciris_node_core::payloads::deferral::{DiversityPolicy, RoutingPreferences};
use ciris_node_core::routing::{select_routed, ContributorMetadata};
use ciris_node_core::substrate::RoutableContributor;

use support::MockEngine;

fn rc(id: &str, expertise: f64) -> RoutableContributor {
    RoutableContributor {
        contributor_id: id.into(),
        expertise,
    }
}

fn dir() -> HashMap<&'static str, ContributorMetadata> {
    [
        ("alice", ContributorMetadata { jurisdiction: "ET".into(), operator: "org_a".into() }),
        ("bob",   ContributorMetadata { jurisdiction: "KE".into(), operator: "org_b".into() }),
        ("carol", ContributorMetadata { jurisdiction: "US".into(), operator: "org_a".into() }),
        ("dan",   ContributorMetadata { jurisdiction: "ET".into(), operator: "org_c".into() }),
        ("eve",   ContributorMetadata { jurisdiction: "KE".into(), operator: "org_d".into() }),
    ]
    .into_iter()
    .collect()
}

#[tokio::test]
async fn none_policy_takes_top_by_expertise() {
    let mock = MockEngine::new();
    mock.set_routable(
        "mental_health",
        "am",
        vec![rc("alice", 0.9), rc("bob", 0.5), rc("carol", 0.7), rc("dan", 0.8)],
    );
    let meta = dir();
    let lookup = |id: &str| meta.get(id).cloned();

    let prefs = RoutingPreferences {
        min_responders: Some(2),
        max_responders: Some(3),
        diversity: Some(DiversityPolicy::None),
    };
    let outcome = select_routed(&mock, "mental_health", "am", Some(&prefs), &lookup)
        .await
        .unwrap();

    assert_eq!(outcome.routed, vec!["alice".to_string(), "dan".into(), "carol".into()]);
    assert!(outcome.min_met);
}

#[tokio::test]
async fn jurisdictional_policy_diversifies_across_jurisdictions() {
    let mock = MockEngine::new();
    // alice(ET, 0.9), dan(ET, 0.85), bob(KE, 0.5), eve(KE, 0.4), carol(US, 0.7)
    mock.set_routable(
        "mental_health",
        "am",
        vec![
            rc("alice", 0.9),
            rc("dan", 0.85),
            rc("bob", 0.5),
            rc("eve", 0.4),
            rc("carol", 0.7),
        ],
    );
    let meta = dir();
    let lookup = |id: &str| meta.get(id).cloned();

    let prefs = RoutingPreferences {
        min_responders: Some(3),
        max_responders: Some(3),
        diversity: Some(DiversityPolicy::Jurisdictional),
    };
    let outcome = select_routed(&mock, "mental_health", "am", Some(&prefs), &lookup)
        .await
        .unwrap();

    // First sweep picks one per distinct jurisdiction by expertise:
    // alice (ET, 0.9), carol (US, 0.7), bob (KE, 0.5).
    assert_eq!(outcome.routed.len(), 3);
    assert_eq!(outcome.routed[0], "alice");
    assert_eq!(outcome.routed[1], "carol");
    assert_eq!(outcome.routed[2], "bob");
    assert_eq!(outcome.jurisdictions_distinct.len(), 3, "ET/KE/US");
    assert!(outcome.min_met);
}

#[tokio::test]
async fn jurisdictional_policy_cycles_when_short_on_jurisdictions() {
    let mock = MockEngine::new();
    // All in ET — one sweep exhausts the diversity bucket, then second
    // sweep picks the next-best.
    mock.set_routable(
        "mental_health",
        "am",
        vec![rc("alice", 0.9), rc("dan", 0.85), rc("frank", 0.6)],
    );
    let meta: HashMap<_, _> = [
        ("alice", ContributorMetadata { jurisdiction: "ET".into(), operator: "a".into() }),
        ("dan", ContributorMetadata { jurisdiction: "ET".into(), operator: "b".into() }),
        ("frank", ContributorMetadata { jurisdiction: "ET".into(), operator: "c".into() }),
    ]
    .into_iter()
    .collect();
    let lookup = |id: &str| meta.get(id).cloned();

    let prefs = RoutingPreferences {
        min_responders: Some(2),
        max_responders: Some(3),
        diversity: Some(DiversityPolicy::Jurisdictional),
    };
    let outcome = select_routed(&mock, "mental_health", "am", Some(&prefs), &lookup)
        .await
        .unwrap();
    // Sweep 1: alice (ET) — exhausts the ET bucket. Sweep 2: dan (ET).
    // Sweep 3: frank (ET).
    assert_eq!(outcome.routed, vec!["alice".to_string(), "dan".into(), "frank".into()]);
    assert_eq!(outcome.jurisdictions_distinct, vec!["ET".to_string()]);
}

#[tokio::test]
async fn organizational_policy_uses_operator_buckets() {
    let mock = MockEngine::new();
    // alice(org_a, 0.9), carol(org_a, 0.7), bob(org_b, 0.5), dan(org_c, 0.85)
    mock.set_routable(
        "mental_health",
        "am",
        vec![
            rc("alice", 0.9),
            rc("dan", 0.85),
            rc("carol", 0.7),
            rc("bob", 0.5),
        ],
    );
    let meta = dir();
    let lookup = |id: &str| meta.get(id).cloned();

    let prefs = RoutingPreferences {
        min_responders: Some(3),
        max_responders: Some(3),
        diversity: Some(DiversityPolicy::Organizational),
    };
    let outcome = select_routed(&mock, "mental_health", "am", Some(&prefs), &lookup)
        .await
        .unwrap();

    // First sweep picks one per operator by expertise: alice (org_a, 0.9),
    // dan (org_c, 0.85), bob (org_b, 0.5). carol (org_a) waits for sweep 2.
    assert_eq!(outcome.routed[0], "alice");
    assert_eq!(outcome.routed[1], "dan");
    assert_eq!(outcome.routed[2], "bob");
    assert_eq!(outcome.operators_distinct.len(), 3);
}

#[tokio::test]
async fn min_not_met_when_routable_set_too_small() {
    let mock = MockEngine::new();
    mock.set_routable("mental_health", "am", vec![rc("alice", 0.9)]);
    let meta = dir();
    let lookup = |id: &str| meta.get(id).cloned();

    let prefs = RoutingPreferences {
        min_responders: Some(3),
        max_responders: Some(9),
        diversity: Some(DiversityPolicy::None),
    };
    let outcome = select_routed(&mock, "mental_health", "am", Some(&prefs), &lookup)
        .await
        .unwrap();
    assert_eq!(outcome.routed.len(), 1);
    assert!(!outcome.min_met, "fewer responders than minimum");
}

#[tokio::test]
async fn empty_routable_returns_empty_routing() {
    let mock = MockEngine::new();
    let meta = dir();
    let lookup = |id: &str| meta.get(id).cloned();
    let outcome = select_routed(&mock, "mental_health", "ur", None, &lookup)
        .await
        .unwrap();
    assert!(outcome.routed.is_empty());
    assert!(!outcome.min_met);
}

#[tokio::test]
async fn default_preferences_apply_when_none_passed() {
    let mock = MockEngine::new();
    let many: Vec<_> = (0..20).map(|i| rc(&format!("c{i}"), 1.0 - (i as f64 / 100.0))).collect();
    mock.set_routable("mental_health", "am", many);
    let lookup = |_id: &str| None::<ContributorMetadata>;

    let outcome = select_routed(&mock, "mental_health", "am", None, &lookup)
        .await
        .unwrap();
    assert_eq!(outcome.routed.len(), 9, "default max per §3.3 step 4");
    assert!(outcome.min_met, "default min=5 met");
}
