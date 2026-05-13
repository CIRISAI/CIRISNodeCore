//! In-memory `MockEngine` — implements persist's
//! [`NodeCoreService`] (v0.7.4) for handler tests.
//!
//! Uses RPITIT (`impl Future + Send`) per persist's trait — no
//! `async_trait` dep. Single `Mutex<MockState>` for write-set
//! inspection. Fixture setters pre-load read views; inspectors
//! return clones of recorded writes.
//!
//! Tracks all 9 typed writes (contributions, votes, ledger updates,
//! moderation, slashing, reconsideration request + attestation,
//! promotion attestation) and the `is_canonical` flip on target rows.

#![allow(dead_code)] // each integration-test file uses a subset

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::Mutex;

use chrono::Utc;

use ciris_node_core::substrate::{
    Cell, ContributionEnvelope, ContributionListPage, ContributionType, ContributionsFilter,
    CreditsLedgerEntry, CreditsUpdate, ExpertiseLedgerEntry, ExpertiseUpdate, HybridSignature,
    ListCursor, ModerationEvent, NodeCoreService, PromotionAttestation, ReconsiderationAttestation,
    ReconsiderationRequest, RoutableContributor, SlashingAttestation, SubstrateError,
    TargetRowKind, VoteEnvelope, VoteListPage, VoteWeight, VotesFilter,
};

#[derive(Default)]
struct MockState {
    contributions: Vec<ContributionEnvelope>,
    votes: Vec<VoteEnvelope>,
    moderation_events: Vec<ModerationEvent>,
    slashing_attestations: Vec<SlashingAttestation>,
    reconsideration_requests: Vec<ReconsiderationRequest>,
    reconsideration_attestations: Vec<ReconsiderationAttestation>,
    promotion_attestations: Vec<PromotionAttestation>,

    // Canonical-promotion state — mirrors V011's `is_canonical` column.
    canonical: HashMap<String, bool>,

    // Fixture-loaded read views.
    // Key for routable: (domain, language)
    routable: HashMap<(String, String), Vec<RoutableContributor>>,
    // Key for credits: (contributor, domain, language, subject)
    credits: HashMap<(String, String, String, String), f64>,
    // Key for expertise: (contributor, domain, language) → (expertise, active_tier)
    expertise: HashMap<(String, String, String), (f64, bool)>,
    // Active-tier override (in addition to per-cell). Present means active.
    active_set: HashSet<String>,
}

/// In-memory `NodeCoreService` impl for tests.
pub struct MockEngine {
    state: Mutex<MockState>,
}

impl MockEngine {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(MockState::default()),
        }
    }

    // ── Fixture setters ──────────────────────────────────────────────────

    pub fn set_routable(&self, domain: &str, language: &str, who: Vec<RoutableContributor>) {
        self.state
            .lock()
            .unwrap()
            .routable
            .insert((domain.into(), language.into()), who);
    }

    pub fn set_credits(
        &self,
        contributor: &str,
        domain: &str,
        language: &str,
        subject: &str,
        credits: f64,
    ) {
        self.state.lock().unwrap().credits.insert(
            (
                contributor.into(),
                domain.into(),
                language.into(),
                subject.into(),
            ),
            credits,
        );
    }

    pub fn set_expertise(
        &self,
        contributor: &str,
        domain: &str,
        language: &str,
        standing: f64,
        active: bool,
    ) {
        self.state
            .lock()
            .unwrap()
            .expertise
            .insert(
                (contributor.into(), domain.into(), language.into()),
                (standing, active),
            );
        if active {
            self.state.lock().unwrap().active_set.insert(contributor.into());
        }
    }

    // ── Inspectors ───────────────────────────────────────────────────────

    pub fn contributions(&self) -> Vec<ContributionEnvelope> {
        self.state.lock().unwrap().contributions.clone()
    }

    pub fn votes(&self) -> Vec<VoteEnvelope> {
        self.state.lock().unwrap().votes.clone()
    }

    pub fn promotion_attestations(&self) -> Vec<PromotionAttestation> {
        self.state.lock().unwrap().promotion_attestations.clone()
    }

    pub fn is_canonical(&self, id: &str) -> bool {
        self.state
            .lock()
            .unwrap()
            .canonical
            .get(id)
            .copied()
            .unwrap_or(false)
    }

    pub fn write_count(&self) -> usize {
        let st = self.state.lock().unwrap();
        st.contributions.len()
            + st.votes.len()
            + st.moderation_events.len()
            + st.slashing_attestations.len()
            + st.reconsideration_requests.len()
            + st.reconsideration_attestations.len()
            + st.promotion_attestations.len()
    }
}

impl NodeCoreService for MockEngine {
    fn put_contribution(
        &self,
        env: ContributionEnvelope,
    ) -> impl Future<Output = Result<(), SubstrateError>> + Send {
        let envelope = env;
        async move {
            self.state.lock().unwrap().contributions.push(envelope);
            Ok(())
        }
    }

    fn cast_vote(
        &self,
        env: VoteEnvelope,
    ) -> impl Future<Output = Result<(), SubstrateError>> + Send {
        let envelope = env;
        async move {
            self.state.lock().unwrap().votes.push(envelope);
            Ok(())
        }
    }

    fn update_credits_ledger(
        &self,
        update: CreditsUpdate,
    ) -> impl Future<Output = Result<(), SubstrateError>> + Send {
        async move {
            if update.new_balance < 0.0 {
                return Err(SubstrateError::InvalidArgument(format!(
                    "credits non-negative invariant: new_balance {} < 0",
                    update.new_balance
                )));
            }
            let key = (
                update.contributor_id,
                update.domain,
                update.language,
                update.subject,
            );
            self.state.lock().unwrap().credits.insert(key, update.new_balance);
            Ok(())
        }
    }

    fn update_expertise_ledger(
        &self,
        update: ExpertiseUpdate,
    ) -> impl Future<Output = Result<(), SubstrateError>> + Send {
        async move {
            if update.new_expertise < 0.0 {
                return Err(SubstrateError::InvalidArgument(format!(
                    "expertise non-negative invariant: new_expertise {} < 0",
                    update.new_expertise
                )));
            }
            let key = (update.contributor_id.clone(), update.domain, update.language);
            self.state
                .lock()
                .unwrap()
                .expertise
                .insert(key, (update.new_expertise, update.new_active_tier));
            if update.new_active_tier {
                self.state.lock().unwrap().active_set.insert(update.contributor_id);
            }
            Ok(())
        }
    }

    fn put_moderation_event(
        &self,
        event: ModerationEvent,
    ) -> impl Future<Output = Result<(), SubstrateError>> + Send {
        let envelope = event;
        async move {
            self.state.lock().unwrap().moderation_events.push(envelope);
            Ok(())
        }
    }

    fn put_slashing_attestation(
        &self,
        att: SlashingAttestation,
    ) -> impl Future<Output = Result<(), SubstrateError>> + Send {
        let envelope = att;
        async move {
            self.state.lock().unwrap().slashing_attestations.push(envelope);
            Ok(())
        }
    }

    fn put_reconsideration_request(
        &self,
        req: ReconsiderationRequest,
    ) -> impl Future<Output = Result<(), SubstrateError>> + Send {
        let envelope = req;
        async move {
            self.state.lock().unwrap().reconsideration_requests.push(envelope);
            Ok(())
        }
    }

    fn put_reconsideration_attestation(
        &self,
        att: ReconsiderationAttestation,
    ) -> impl Future<Output = Result<(), SubstrateError>> + Send {
        let envelope = att;
        async move {
            self.state.lock().unwrap().reconsideration_attestations.push(envelope);
            Ok(())
        }
    }

    fn put_promotion_attestation(
        &self,
        att: PromotionAttestation,
    ) -> impl Future<Output = Result<(), SubstrateError>> + Send {
        let attestation = att;
        async move {
            let mut st = self.state.lock().unwrap();
            if let TargetRowKind::Contribution = attestation.target_kind {
                for tid in &attestation.target_ids {
                    if !st.contributions.iter().any(|c| &c.contribution_id == tid) {
                        return Err(SubstrateError::InvalidArgument(format!(
                            "target_id {tid} not in contributions table"
                        )));
                    }
                }
            }
            for tid in &attestation.target_ids {
                st.canonical.insert(tid.clone(), true);
            }
            st.promotion_attestations.push(attestation);
            Ok(())
        }
    }

    fn routable_contributors(
        &self,
        domain: &str,
        language: &str,
    ) -> impl Future<Output = Result<Vec<RoutableContributor>, SubstrateError>> + Send {
        let domain = domain.to_owned();
        let language = language.to_owned();
        async move {
            Ok(self
                .state
                .lock()
                .unwrap()
                .routable
                .get(&(domain, language))
                .cloned()
                .unwrap_or_default())
        }
    }

    fn read_vote_weight(
        &self,
        contributor_id: &str,
        domain: &str,
        language: &str,
        subject: &str,
    ) -> impl Future<Output = Result<Option<VoteWeight>, SubstrateError>> + Send {
        let cid = contributor_id.to_owned();
        let d = domain.to_owned();
        let l = language.to_owned();
        let s = subject.to_owned();
        async move {
            let st = self.state.lock().unwrap();
            let credits = st
                .credits
                .get(&(cid.clone(), d.clone(), l.clone(), s.clone()))
                .copied()
                .unwrap_or(0.0);
            let (expertise, active) = st
                .expertise
                .get(&(cid.clone(), d.clone(), l.clone()))
                .copied()
                .unwrap_or((0.0, false));
            let active_mult = if active { 1.0 } else { 0.0 };
            Ok(Some(VoteWeight {
                contributor_id: cid,
                domain: d,
                language: l,
                subject: s,
                credits,
                expertise_multiplier: expertise,
                active_tier_multiplier: active_mult,
                weight: credits * expertise * active_mult,
            }))
        }
    }

    fn list_contributions(
        &self,
        _filter: ContributionsFilter,
        _cursor: Option<ListCursor>,
        _limit: i64,
    ) -> impl Future<Output = Result<ContributionListPage, SubstrateError>> + Send {
        async move {
            Ok(ContributionListPage {
                items: self.state.lock().unwrap().contributions.clone(),
                next_cursor: None,
            })
        }
    }

    fn list_votes(
        &self,
        _filter: VotesFilter,
        _cursor: Option<ListCursor>,
        _limit: i64,
    ) -> impl Future<Output = Result<VoteListPage, SubstrateError>> + Send {
        async move {
            Ok(VoteListPage {
                items: self.state.lock().unwrap().votes.clone(),
                next_cursor: None,
            })
        }
    }

    fn get_credits_ledger(
        &self,
        contributor_id: &str,
        domain: &str,
        language: &str,
        subject: &str,
    ) -> impl Future<Output = Result<Option<CreditsLedgerEntry>, SubstrateError>> + Send {
        let cid = contributor_id.to_owned();
        let d = domain.to_owned();
        let l = language.to_owned();
        let s = subject.to_owned();
        async move {
            let balance = self
                .state
                .lock()
                .unwrap()
                .credits
                .get(&(cid.clone(), d.clone(), l.clone(), s.clone()))
                .copied();
            Ok(balance.map(|b| CreditsLedgerEntry {
                contributor_id: cid,
                domain: d,
                language: l,
                subject: s,
                balance: b,
                last_update_contribution: None,
                last_updated_at: Utc::now(),
                created_at: Utc::now(),
            }))
        }
    }

    fn get_expertise_ledger(
        &self,
        contributor_id: &str,
        domain: &str,
        language: &str,
    ) -> impl Future<Output = Result<Option<ExpertiseLedgerEntry>, SubstrateError>> + Send {
        let cid = contributor_id.to_owned();
        let d = domain.to_owned();
        let l = language.to_owned();
        async move {
            let entry = self
                .state
                .lock()
                .unwrap()
                .expertise
                .get(&(cid.clone(), d.clone(), l.clone()))
                .copied();
            Ok(entry.map(|(e, active)| ExpertiseLedgerEntry {
                contributor_id: cid,
                domain: d,
                language: l,
                expertise: e,
                is_active: active,
                last_updated_at: Utc::now(),
                last_update_contribution: None,
                created_at: Utc::now(),
            }))
        }
    }
}

pub fn placeholder_signature() -> HybridSignature {
    HybridSignature {
        ed25519: "test_sig_placeholder".into(),
        ml_dsa_65: None,
        signed_at: Utc::now(),
    }
}

pub fn build_envelope(
    contribution_id: &str,
    contribution_type: ContributionType,
    author_id: &str,
    cell: Cell,
    payload: serde_json::Value,
) -> ContributionEnvelope {
    ContributionEnvelope {
        contribution_id: contribution_id.into(),
        contribution_type,
        author_id: author_id.into(),
        subject: cell,
        payload,
        witness_set: None,
        signature: placeholder_signature(),
        submitted_at: Utc::now(),
    }
}
