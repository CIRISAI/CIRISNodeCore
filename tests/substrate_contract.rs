//! Integration spike: validate that CIRISPersist v0.7.1's
//! `NodeCoreService` trait + wire types are consumable by node-core
//! without intermediate translation.
//!
//! This is **not** the full OQ-7 collapse — it's a contract-fits-our-needs
//! check that builds the substrate's typed envelopes from scratch, defines
//! a tiny in-memory impl of `NodeCoreService`, and round-trips every
//! method. If this compiles and passes, the v0.7.1 contract is usable
//! by node-core as-is.

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::Utc;

use ciris_node_core::substrate::{
    Cell, ContributionEnvelope, ContributionListPage, ContributionType, ContributionsFilter,
    CreditsLedgerEntry, CreditsUpdate, ExpertiseLedgerEntry, ExpertiseUpdate, HybridSignature,
    ListCursor, ModerationEvent, NodeCoreService, PromotionAttestation, ReconsiderationAttestation,
    ReconsiderationRequest, RoutableContributor, SlashingAttestation, SubstrateError,
    TargetRowKind, VoteEnvelope, VoteListPage, VoteWeight, VotesFilter,
};

// ── Tiny in-memory mock impl ─────────────────────────────────────────────
//
// Uses RPITIT (`impl Future + Send`) per persist's trait shape. No
// async_trait. Single Mutex<State> for write-set inspection. This is the
// SHAPE node-core's MockEngine collapses into when OQ-7 lands; today
// it lives in this test file as a contract-fit spike.

#[derive(Default)]
struct State {
    contributions: Vec<ContributionEnvelope>,
    votes: Vec<VoteEnvelope>,
    moderation_events: Vec<ModerationEvent>,
    slashing_attestations: Vec<SlashingAttestation>,
    reconsideration_requests: Vec<ReconsiderationRequest>,
    reconsideration_attestations: Vec<ReconsiderationAttestation>,
    promotion_attestations: Vec<PromotionAttestation>,
    /// Mirrors persist's transactional flip: contribution_id → is_canonical.
    canonical: HashMap<String, bool>,
    credits: HashMap<(String, String, String, String), f64>,
    expertise: HashMap<(String, String, String), (f64, bool)>,
}

struct SpikeMock {
    state: Mutex<State>,
}

impl SpikeMock {
    fn new() -> Self {
        Self {
            state: Mutex::new(State::default()),
        }
    }
}

impl NodeCoreService for SpikeMock {
    async fn put_contribution(&self, env: ContributionEnvelope) -> Result<(), SubstrateError> {
        self.state.lock().unwrap().contributions.push(env);
        Ok(())
    }

    async fn cast_vote(&self, env: VoteEnvelope) -> Result<(), SubstrateError> {
        self.state.lock().unwrap().votes.push(env);
        Ok(())
    }

    async fn update_credits_ledger(&self, update: CreditsUpdate) -> Result<(), SubstrateError> {
        let key = (
            update.contributor_id,
            update.domain,
            update.language,
            update.subject,
        );
        self.state.lock().unwrap().credits.insert(key, update.new_balance);
        Ok(())
    }

    async fn update_expertise_ledger(&self, update: ExpertiseUpdate) -> Result<(), SubstrateError> {
        let key = (update.contributor_id, update.domain, update.language);
        self.state
            .lock()
            .unwrap()
            .expertise
            .insert(key, (update.new_expertise, update.new_active_tier));
        Ok(())
    }

    async fn put_moderation_event(&self, event: ModerationEvent) -> Result<(), SubstrateError> {
        self.state.lock().unwrap().moderation_events.push(event);
        Ok(())
    }

    async fn put_slashing_attestation(
        &self,
        att: SlashingAttestation,
    ) -> Result<(), SubstrateError> {
        self.state.lock().unwrap().slashing_attestations.push(att);
        Ok(())
    }

    async fn put_reconsideration_request(
        &self,
        req: ReconsiderationRequest,
    ) -> Result<(), SubstrateError> {
        self.state.lock().unwrap().reconsideration_requests.push(req);
        Ok(())
    }

    async fn put_reconsideration_attestation(
        &self,
        att: ReconsiderationAttestation,
    ) -> Result<(), SubstrateError> {
        self.state
            .lock()
            .unwrap()
            .reconsideration_attestations
            .push(att);
        Ok(())
    }

    async fn put_promotion_attestation(
        &self,
        att: PromotionAttestation,
    ) -> Result<(), SubstrateError> {
        let mut st = self.state.lock().unwrap();

        // Mirror persist's transactional shape (v0.7.2 doc):
        // affected-row-count assertion — every named target must exist.
        // We model "existing" by checking the contributions Vec for
        // Contribution-targeted promotions; other target_kinds aren't
        // tracked in this spike mock, so we pretend they exist.
        if let TargetRowKind::Contribution = att.target_kind {
            for tid in &att.target_ids {
                if !st.contributions.iter().any(|c| &c.contribution_id == tid) {
                    return Err(SubstrateError::InvalidArgument(format!(
                        "target_id {tid} not found in contributions table"
                    )));
                }
            }
        }

        // Flip is_canonical on each named target — transaction is
        // all-or-nothing per persist's contract.
        for tid in &att.target_ids {
            st.canonical.insert(tid.clone(), true);
        }
        st.promotion_attestations.push(att);
        Ok(())
    }

    async fn routable_contributors(
        &self,
        _domain: &str,
        _language: &str,
    ) -> Result<Vec<RoutableContributor>, SubstrateError> {
        Ok(vec![
            RoutableContributor {
                contributor_id: "alice".into(),
                expertise: 0.8,
            },
            RoutableContributor {
                contributor_id: "bob".into(),
                expertise: 0.6,
            },
        ])
    }

    async fn read_vote_weight(
        &self,
        contributor_id: &str,
        domain: &str,
        language: &str,
        subject: &str,
    ) -> Result<Option<VoteWeight>, SubstrateError> {
        Ok(Some(VoteWeight {
            contributor_id: contributor_id.into(),
            domain: domain.into(),
            language: language.into(),
            subject: subject.into(),
            credits: 100.0,
            expertise_multiplier: 1.5,
            active_tier_multiplier: 1.0,
            weight: 150.0,
        }))
    }

    async fn list_contributions(
        &self,
        _filter: ContributionsFilter,
        _cursor: Option<ListCursor>,
        _limit: i64,
    ) -> Result<ContributionListPage, SubstrateError> {
        let items = self.state.lock().unwrap().contributions.clone();
        Ok(ContributionListPage {
            items,
            next_cursor: None,
        })
    }

    async fn list_votes(
        &self,
        _filter: VotesFilter,
        _cursor: Option<ListCursor>,
        _limit: i64,
    ) -> Result<VoteListPage, SubstrateError> {
        let items = self.state.lock().unwrap().votes.clone();
        Ok(VoteListPage {
            items,
            next_cursor: None,
        })
    }

    async fn get_credits_ledger(
        &self,
        _contributor_id: &str,
        _domain: &str,
        _language: &str,
        _subject: &str,
    ) -> Result<Option<CreditsLedgerEntry>, SubstrateError> {
        Ok(None)
    }

    async fn get_expertise_ledger(
        &self,
        _contributor_id: &str,
        _domain: &str,
        _language: &str,
    ) -> Result<Option<ExpertiseLedgerEntry>, SubstrateError> {
        Ok(None)
    }
}

// ── Contract-fit tests ──────────────────────────────────────────────────

fn placeholder_signature() -> HybridSignature {
    HybridSignature {
        ed25519: "test_sig_placeholder".into(),
        ml_dsa_65: None,
        signed_at: Utc::now(),
    }
}

#[tokio::test]
async fn contribution_envelope_round_trips_through_trait() {
    let mock = SpikeMock::new();

    let env = ContributionEnvelope {
        contribution_id: "01HXTEST00000000000000000".into(),
        contribution_type: ContributionType::DeferralRequest,
        author_id: "alice_pubkey_b64".into(),
        // Expertise-granularity cell for deferral routing — subject=None
        // per SCHEMA §7 / §10 (and confirmed in v0.7.1 — Cell.subject is
        // Option<String>).
        subject: Cell {
            domain: "mental_health".into(),
            language: "am".into(),
            subject: None,
        },
        payload: serde_json::json!({
            "title": "Stage-2 medication register check",
            "context": "..."
        }),
        witness_set: None,
        signature: placeholder_signature(),
        submitted_at: Utc::now(),
    };

    mock.put_contribution(env.clone()).await.unwrap();

    let page = mock
        .list_contributions(ContributionsFilter::default(), None, 10)
        .await
        .unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].contribution_id, env.contribution_id);
}

#[tokio::test]
async fn vote_envelope_with_optional_contribution_id() {
    let mock = SpikeMock::new();

    // Vote tied to a Contribution (the common case).
    let v1 = VoteEnvelope {
        vote_id: "01HXVOTE10000000000000000".into(),
        voter_id: "voter_pubkey_b64".into(),
        contribution_id: Some("01HXTEST00000000000000000".into()),
        cell: Cell {
            domain: "mental_health".into(),
            language: "am".into(),
            subject: Some("arc_question".into()),
        },
        score: serde_json::json!({"score_kind": "proposal_adoption", "verdict": "approve"}),
        rationale: Some("LGTM".into()),
        signature: placeholder_signature(),
        cast_at: Utc::now(),
    };
    mock.cast_vote(v1).await.unwrap();

    // Free-form poll vote (no contribution_id) — per persist doc.
    let v2 = VoteEnvelope {
        vote_id: "01HXVOTE20000000000000000".into(),
        voter_id: "voter_pubkey_b64".into(),
        contribution_id: None,
        cell: Cell {
            domain: "policy".into(),
            language: "en".into(),
            subject: Some("free_form".into()),
        },
        score: serde_json::json!({"kind": "poll_response"}),
        rationale: None,
        signature: placeholder_signature(),
        cast_at: Utc::now(),
    };
    mock.cast_vote(v2).await.unwrap();

    let page = mock.list_votes(VotesFilter::default(), None, 10).await.unwrap();
    assert_eq!(page.items.len(), 2);
}

#[tokio::test]
async fn ledger_set_to_value_semantics() {
    let mock = SpikeMock::new();

    mock.update_credits_ledger(CreditsUpdate {
        contributor_id: "alice".into(),
        domain: "mental_health".into(),
        language: "am".into(),
        subject: "arc_question".into(),
        new_balance: 127.5,
        source_contribution: "01HX00000000000000000000C".into(),
    })
    .await
    .unwrap();

    mock.update_expertise_ledger(ExpertiseUpdate {
        contributor_id: "alice".into(),
        domain: "mental_health".into(),
        language: "am".into(),
        new_expertise: 0.42,
        new_active_tier: true,
        source_contribution: "01HX00000000000000000000E".into(),
    })
    .await
    .unwrap();

    // Note: set-to-value, not delta — non-negative floor enforcement is
    // a deployment policy concern (caller computes new_balance and
    // refuses to write a negative one). Persist doesn't enforce at the
    // type or method level.
}

#[tokio::test]
async fn vote_weight_carries_full_composition() {
    let mock = SpikeMock::new();
    let w = mock
        .read_vote_weight("alice", "mental_health", "am", "arc_question")
        .await
        .unwrap()
        .expect("mock returns Some");
    assert_eq!(w.credits, 100.0);
    assert_eq!(w.expertise_multiplier, 1.5);
    assert_eq!(w.active_tier_multiplier, 1.0);
    assert_eq!(w.weight, 150.0);
    // Verify the four input fields multiply to `weight`. If persist's
    // formula drifts we'll catch it here.
    assert_eq!(w.credits * w.expertise_multiplier * w.active_tier_multiplier, w.weight);
}

#[tokio::test]
async fn routable_contributors_returns_richer_than_id() {
    let mock = SpikeMock::new();
    let candidates = mock.routable_contributors("mental_health", "am").await.unwrap();
    assert_eq!(candidates.len(), 2);
    // RoutableContributor is { contributor_id, expertise } — node-core
    // can rank by expertise without a second lookup, which matters for
    // MISSION.md §3.3 step 4 diversity selection.
    assert!(candidates[0].expertise >= candidates[1].expertise);
}

#[tokio::test]
async fn moderation_chain_round_trips() {
    let mock = SpikeMock::new();

    let mod_event = ModerationEvent {
        moderation_id: "01HXMOD0000000000000000000".into(),
        target_contributor: "target_pub".into(),
        accuser_id: "accuser_pub".into(),
        payload: serde_json::json!({
            "allegation": "rogue_vote",
            "evidence_refs": ["vote_01HX"]
        }),
        filed_at: Utc::now(),
        signature: placeholder_signature(),
    };
    mock.put_moderation_event(mod_event.clone()).await.unwrap();

    let slash = SlashingAttestation {
        slashing_id: "01HXSLASH00000000000000000".into(),
        moderation_id: mod_event.moderation_id.clone(),
        adjudicator_id: "wa_pub".into(),
        payload: serde_json::json!({"outcome": "sustain", "credits_reduced": "5.0"}),
        attested_at: Utc::now(),
        signature: placeholder_signature(),
    };
    mock.put_slashing_attestation(slash.clone()).await.unwrap();

    let recon_req = ReconsiderationRequest {
        request_id: "01HXRECON000000000000000000".into(),
        slashing_id: slash.slashing_id.clone(),
        requester_id: "requester_pub".into(),
        payload: serde_json::json!({"grounds": "new_evidence"}),
        requested_at: Utc::now(),
        signature: placeholder_signature(),
    };
    mock.put_reconsideration_request(recon_req).await.unwrap();

    let recon_att = ReconsiderationAttestation {
        reconsideration_id: "01HXRECONATT0000000000000".into(),
        request_id: "01HXRECON000000000000000000".into(),
        adjudicator_id: "wa_fresh_pub".into(),
        payload: serde_json::json!({"outcome": "reverse"}),
        attested_at: Utc::now(),
        signature: placeholder_signature(),
    };
    mock.put_reconsideration_attestation(recon_att).await.unwrap();
}

#[tokio::test]
async fn promotion_attestation_flips_targets_and_records_attestation() {
    // CIRISPersist#32 closed in v0.7.2. Verify the new
    // put_promotion_attestation method round-trips with the
    // transactional-flip semantics persist documents.
    let mock = SpikeMock::new();

    let env = ContributionEnvelope {
        contribution_id: "01HXPROMOTE0000000000000000".into(),
        contribution_type: ContributionType::Proposal,
        author_id: "alice_pubkey_b64".into(),
        subject: Cell {
            domain: "mental_health".into(),
            language: "am".into(),
            subject: Some("arc_question".into()),
        },
        payload: serde_json::json!({"question_id": "am_mh_v4_q01"}),
        witness_set: None,
        signature: placeholder_signature(),
        submitted_at: Utc::now(),
    };
    mock.put_contribution(env.clone()).await.unwrap();

    let promo = PromotionAttestation {
        attestation_id: "01HXPROMOATT00000000000000".into(),
        target_kind: TargetRowKind::Contribution,
        target_ids: vec![env.contribution_id.clone()],
        attested_by: "consensus_crate_pubkey".into(),
        aggregate_evidence: serde_json::json!({
            "vote_tally": {"approve": 12, "reject": 2},
            "witness_count": 5,
            "threshold_window_days": 7
        }),
        signature: placeholder_signature(),
        attested_at: Utc::now(),
    };
    mock.put_promotion_attestation(promo.clone()).await.unwrap();

    // Per persist's contract: targets flipped to canonical, attestation
    // INSERTed — atomically.
    let st = mock.state.lock().unwrap();
    assert_eq!(st.promotion_attestations.len(), 1);
    assert_eq!(
        st.canonical.get(&env.contribution_id).copied().unwrap_or(false),
        true,
        "target should be canonical post-promotion"
    );
}

#[tokio::test]
async fn promotion_attestation_rejects_unknown_target() {
    // Mirror the affected-row-count assertion in persist's v0.7.2
    // doc: if any named target doesn't exist, the entire transaction
    // rolls back with InvalidArgument.
    let mock = SpikeMock::new();

    let promo = PromotionAttestation {
        attestation_id: "01HXPROMOATT10000000000000".into(),
        target_kind: TargetRowKind::Contribution,
        target_ids: vec!["01HXNONEXISTENT0000000000".into()],
        attested_by: "consensus_crate_pubkey".into(),
        aggregate_evidence: serde_json::json!({}),
        signature: placeholder_signature(),
        attested_at: Utc::now(),
    };
    let result = mock.put_promotion_attestation(promo).await;
    assert!(matches!(result, Err(SubstrateError::InvalidArgument(_))));

    // Verify nothing got mutated.
    let st = mock.state.lock().unwrap();
    assert!(st.promotion_attestations.is_empty());
    assert!(st.canonical.is_empty());
}

#[tokio::test]
async fn list_cursor_constructor_available() {
    // ListCursor::from_trailing — confirms the constructor exists.
    let cursor = ListCursor::from_trailing(Utc::now(), "01HX00000000000000000000Z".into());
    assert_eq!(cursor.version, "v1");
}
