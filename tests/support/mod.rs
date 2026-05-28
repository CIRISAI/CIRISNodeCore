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
use ciris_node_core::trust::{
    AuditService, FederationDirectory, TrustFilter, TrustGrant, TrustGrantFilter, TrustGrantRow,
    TrustPurpose, TrustRelationship, TrustRow,
};

// Other FederationDirectory types — needed for the 14 stubbed methods
// (public-key + attestation + revocation + PQC fill-in surfaces).
use ciris_persist::federation::{
    Attestation, HybridPendingRow, KeyRecord, Revocation, SignedAttestation, SignedKeyRecord,
    SignedRevocation,
};

// AuditService support types — needed for the 3 required stubs
// (record_entry / list_entries / verify_chain).
use ciris_persist::audit::{AuditEntry, AuditFilter, AuditListPage, ChainVerification};
use ciris_persist::audit::types::AuditCursor;

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

    // FederationDirectory state — V020 trust rows keyed by `key`.
    // Deprecated in v1.6.0; kept for back-compat with tests targeting
    // the old API while the swap to AuditService stabilizes.
    trust_rows: HashMap<String, TrustRow>,

    // AuditService state — v1.5.x trust-grant projection rows. Keyed
    // by grant_id. Populated via `set_trust_grant` fixture helper;
    // queried via the AuditService impl.
    trust_grants: HashMap<uuid::Uuid, TrustGrantRow>,
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

    /// Insert a v1.5.x trust-grant projection row. Test fixture for
    /// the `AuditService` surface — production grants flow through
    /// signed Contribution events + persist's projection hook, but
    /// tests stuff the projection directly.
    pub fn set_trust_grant(&self, grant: TrustGrantRow) {
        let id = grant.grant_id;
        self.state.lock().unwrap().trust_grants.insert(id, grant);
    }

    /// Convenience: build a `TrustGrantRow` from the load-bearing
    /// fields. Defaults `tenant_id="test"`, `chain_event_id=0`,
    /// `chain_event_hash=[]`, `granted_at=now()`.
    pub fn make_grant(
        grantee: &str,
        granter: &str,
        purpose: TrustPurpose,
        scope: &str,
    ) -> TrustGrantRow {
        TrustGrantRow {
            grant_id: uuid::Uuid::new_v4(),
            grantee_key: grantee.into(),
            granter_key: granter.into(),
            purpose,
            scope: scope.into(),
            granted_at: Utc::now(),
            expires_at: None,
            revoked_at: None,
            revoked_by: None,
            chain_event_id: 0,
            chain_event_hash: Vec::new(),
            tenant_id: "test".into(),
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
        filter: ContributionsFilter,
        _cursor: Option<ListCursor>,
        _limit: i64,
    ) -> impl Future<Output = Result<ContributionListPage, SubstrateError>> + Send {
        let filter = filter;
        async move {
            let st = self.state.lock().unwrap();
            let items: Vec<ContributionEnvelope> = st
                .contributions
                .iter()
                .filter(|env| {
                    if let Some(ct) = filter.contribution_type {
                        if env.contribution_type != ct {
                            return false;
                        }
                    }
                    if let Some(ref dom) = filter.domain {
                        if &env.subject.domain != dom {
                            return false;
                        }
                    }
                    if let Some(ref lang) = filter.language {
                        if &env.subject.language != lang {
                            return false;
                        }
                    }
                    if let Some(ref sk) = filter.subject_kind {
                        if env.subject.subject.as_deref() != Some(sk.as_str()) {
                            return false;
                        }
                    }
                    if let Some(ref author) = filter.author_id {
                        if &env.author_id != author {
                            return false;
                        }
                    }
                    if let Some(canonical) = filter.is_canonical {
                        let is_can = st.canonical.get(&env.contribution_id).copied().unwrap_or(false);
                        if is_can != canonical {
                            return false;
                        }
                    }
                    true
                })
                .cloned()
                .collect();
            Ok(ContributionListPage {
                items,
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

    // FederationAnnouncement delivery_attestation surface (CIRISPersist v2.2.0+
    // shipped per #101). Mock is empty / no-op; tests that exercise the
    // delivery-audit path would populate via dedicated fixtures.
    fn put_delivery_attestation(
        &self,
        _attestation: ciris_persist::cirisnode::federation_announcement::DeliveryAttestation,
    ) -> impl Future<Output = Result<(), SubstrateError>> + Send {
        async { Ok(()) }
    }

    fn list_delivery_attestations(
        &self,
        _announcement_id: &str,
    ) -> impl Future<
        Output = Result<Vec<ciris_persist::cirisnode::federation_announcement::DeliveryAttestation>, SubstrateError>,
    > + Send {
        async { Ok(Vec::new()) }
    }

    fn count_delivery_attestations(
        &self,
        _announcement_id: &str,
    ) -> impl Future<Output = Result<u64, SubstrateError>> + Send {
        async { Ok(0) }
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

// Convenience alias for the federation directory's error type.
type FedErr = ciris_persist::federation::Error;

/// Stub helper — returns Backend("not used in tests") for the 14
/// non-trust methods on the persist v1.3.0 FederationDirectory trait.
/// Tests that exercise public-key / attestation / revocation / PQC
/// paths must use persist's actual MemoryBackend; node-core tests
/// touch only the trust subset.
fn fed_stub(method: &'static str) -> FedErr {
    FedErr::Backend(format!("MockEngine: {method} not implemented in node-core test fixtures"))
}

#[async_trait::async_trait]
impl FederationDirectory for MockEngine {
    // ── Trust methods — real impls (mirror persist's MemoryBackend
    //    validate_trust_grant rules so tests exercise the same
    //    contract). Per persist v2.6.0+, these are default-method
    //    overrides; the impl stays here so node-core's trust tests
    //    exercise the validation contract rather than fall through
    //    to the trait's "not implemented" default. ────────────────

    async fn grant_trust(&self, grant: TrustGrant) -> Result<(), FedErr> {
        // Mirror persist::store::memory::validate_trust_grant.
        if grant.key.is_empty() {
            return Err(FedErr::InvalidArgument("grant.key must be non-empty".into()));
        }
        if grant.trusted_by.is_empty() {
            return Err(FedErr::InvalidArgument(
                "grant.trusted_by must be non-empty".into(),
            ));
        }
        if grant.trusted_by == grant.key {
            return Err(FedErr::InvalidArgument(format!(
                "grant.trusted_by must differ from grant.key (no self-trust); got {}",
                grant.key
            )));
        }
        match grant.trust_relationship {
            TrustRelationship::Registry => {
                let n = grant.trust_domains.as_ref().map(|d| d.len()).unwrap_or(0);
                if n == 0 {
                    return Err(FedErr::InvalidArgument(
                        "Registry-relationship grants require a non-empty trust_domains list"
                            .into(),
                    ));
                }
            }
            TrustRelationship::Direct => {
                if grant.trust_domains.is_some() {
                    return Err(FedErr::InvalidArgument(
                        "Direct-relationship grants must have trust_domains=None".into(),
                    ));
                }
            }
        }
        let row = TrustRow {
            key: grant.key.clone(),
            trust_type: grant.trust_type,
            trust_relationship: grant.trust_relationship,
            trust_domains: grant.trust_domains,
            trusted_by: grant.trusted_by,
            trusted_at: Utc::now(),
            expires_at: grant.expires_at,
        };
        self.state.lock().unwrap().trust_rows.insert(grant.key, row);
        Ok(())
    }

    async fn revoke_trust(&self, key: &str, _revoked_by: &str) -> Result<(), FedErr> {
        let mut st = self.state.lock().unwrap();
        if let Some(row) = st.trust_rows.get_mut(key) {
            row.expires_at = Some(Utc::now());
        }
        Ok(())
    }

    async fn lookup_trust(&self, key: &str) -> Result<Option<TrustRow>, FedErr> {
        Ok(self.state.lock().unwrap().trust_rows.get(key).cloned())
    }

    async fn list_trusted_keys(&self, filter: TrustFilter) -> Result<Vec<TrustRow>, FedErr> {
        let st = self.state.lock().unwrap();
        let now = Utc::now();
        let mut out: Vec<TrustRow> = st
            .trust_rows
            .values()
            .filter(|row| {
                if !filter.include_expired {
                    if let Some(t) = row.expires_at {
                        if t <= now {
                            return false;
                        }
                    }
                }
                if let Some(tt) = filter.trust_type {
                    if row.trust_type != tt {
                        return false;
                    }
                }
                if let Some(tr) = filter.trust_relationship {
                    if row.trust_relationship != tr {
                        return false;
                    }
                }
                if let Some(ref d) = filter.domain {
                    let in_domain = row
                        .trust_domains
                        .as_ref()
                        .map(|v| v.iter().any(|x| x == d))
                        .unwrap_or(false);
                    if !in_domain {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();
        // Deterministic ordering for tests.
        out.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(out)
    }

    // ── 15 stubs for the rest of the FederationDirectory surface ────
    //    (14 v2.5.0 methods + list_keys_by_identity_type added per
    //    CIRISPersist#105 in v2.6.0). Node-core's M1 tests don't
    //    exercise these paths. Anything that does should use
    //    `ciris_persist::store::MemoryBackend` directly.

    async fn put_public_key(&self, _record: SignedKeyRecord) -> Result<(), FedErr> {
        Err(fed_stub("put_public_key"))
    }

    async fn lookup_public_key(&self, _key_id: &str) -> Result<Option<KeyRecord>, FedErr> {
        Err(fed_stub("lookup_public_key"))
    }

    async fn lookup_keys_for_identity(
        &self,
        _identity_ref: &str,
    ) -> Result<Vec<KeyRecord>, FedErr> {
        Err(fed_stub("lookup_keys_for_identity"))
    }

    async fn list_keys_by_identity_type(
        &self,
        _identity_type: &str,
    ) -> Result<Vec<KeyRecord>, FedErr> {
        Err(fed_stub("list_keys_by_identity_type"))
    }

    async fn put_attestation(&self, _attestation: SignedAttestation) -> Result<(), FedErr> {
        Err(fed_stub("put_attestation"))
    }

    async fn list_attestations_for(
        &self,
        _attested_key_id: &str,
    ) -> Result<Vec<Attestation>, FedErr> {
        Err(fed_stub("list_attestations_for"))
    }

    async fn list_attestations_by(
        &self,
        _attesting_key_id: &str,
    ) -> Result<Vec<Attestation>, FedErr> {
        Err(fed_stub("list_attestations_by"))
    }

    async fn put_revocation(&self, _revocation: SignedRevocation) -> Result<(), FedErr> {
        Err(fed_stub("put_revocation"))
    }

    async fn revocations_for(&self, _revoked_key_id: &str) -> Result<Vec<Revocation>, FedErr> {
        Err(fed_stub("revocations_for"))
    }

    async fn attach_key_pqc_signature(
        &self,
        _key_id: &str,
        _pubkey_ml_dsa_65_base64: &str,
        _scrub_signature_pqc: &str,
    ) -> Result<(), FedErr> {
        Err(fed_stub("attach_key_pqc_signature"))
    }

    async fn attach_attestation_pqc_signature(
        &self,
        _attestation_id: &str,
        _pqc_signature_base64: &str,
    ) -> Result<(), FedErr> {
        Err(fed_stub("attach_attestation_pqc_signature"))
    }

    async fn attach_revocation_pqc_signature(
        &self,
        _revocation_id: &str,
        _pqc_signature_base64: &str,
    ) -> Result<(), FedErr> {
        Err(fed_stub("attach_revocation_pqc_signature"))
    }

    async fn list_hybrid_pending_keys(
        &self,
        _limit: i64,
    ) -> Result<Vec<HybridPendingRow>, FedErr> {
        Err(fed_stub("list_hybrid_pending_keys"))
    }

    async fn list_hybrid_pending_attestations(
        &self,
        _limit: i64,
    ) -> Result<Vec<HybridPendingRow>, FedErr> {
        Err(fed_stub("list_hybrid_pending_attestations"))
    }

    async fn list_hybrid_pending_revocations(
        &self,
        _limit: i64,
    ) -> Result<Vec<HybridPendingRow>, FedErr> {
        Err(fed_stub("list_hybrid_pending_revocations"))
    }
}

// ── AuditService impl (v1.5.x trust-grant projection surface) ───────────
//
// Three required methods (record_entry / list_entries / verify_chain)
// return Backend errors — node-core tests don't exercise the audit
// chain ingest path directly. The other 12 trait methods use defaults
// from the upstream trait (most return NotImplemented). We override
// only `lookup_trust_grant` + `list_trust_grants` with real test
// logic against `trust_grants` state populated via `set_trust_grant`.

impl AuditService for MockEngine {
    fn record_entry(
        &self,
        _entry: AuditEntry,
    ) -> impl Future<Output = Result<(), ciris_persist::audit::Error>> + Send {
        async {
            Err(ciris_persist::audit::Error::Backend(
                "MockEngine: record_entry not implemented in node-core test fixtures".into(),
            ))
        }
    }

    fn list_entries(
        &self,
        _filter: AuditFilter,
        _cursor: Option<AuditCursor>,
        _limit: i64,
    ) -> impl Future<Output = Result<AuditListPage, ciris_persist::audit::Error>> + Send {
        async {
            Err(ciris_persist::audit::Error::Backend(
                "MockEngine: list_entries not implemented in node-core test fixtures".into(),
            ))
        }
    }

    fn verify_chain(
        &self,
        _tenant_id: &str,
        _from_sequence: i64,
        _to_sequence: Option<i64>,
    ) -> impl Future<Output = Result<ChainVerification, ciris_persist::audit::Error>> + Send {
        async {
            Err(ciris_persist::audit::Error::Backend(
                "MockEngine: verify_chain not implemented in node-core test fixtures".into(),
            ))
        }
    }

    // Real impls for the trust-grant read path —
    // `crate::trust::resolve_trust` + `crate::routing::route_deferral`
    // consume these.

    fn lookup_trust_grant(
        &self,
        grantee_key: &str,
        purpose: TrustPurpose,
        scope: &str,
        include_revoked: bool,
        include_expired: bool,
    ) -> impl Future<Output = Result<Vec<TrustGrantRow>, ciris_persist::audit::Error>> + Send
    {
        let grantee = grantee_key.to_owned();
        let scope = scope.to_owned();
        async move {
            let st = self.state.lock().unwrap();
            let now = Utc::now();
            let rows: Vec<TrustGrantRow> = st
                .trust_grants
                .values()
                .filter(|g| g.grantee_key == grantee && g.purpose == purpose && g.scope == scope)
                .filter(|g| include_revoked || g.revoked_at.is_none())
                .filter(|g| match g.expires_at {
                    Some(t) if t <= now => include_expired,
                    _ => true,
                })
                .cloned()
                .collect();
            Ok(rows)
        }
    }

    fn list_trust_grants(
        &self,
        filter: TrustGrantFilter,
    ) -> impl Future<Output = Result<Vec<TrustGrantRow>, ciris_persist::audit::Error>> + Send
    {
        async move {
            let st = self.state.lock().unwrap();
            let now = Utc::now();
            let mut rows: Vec<TrustGrantRow> = st
                .trust_grants
                .values()
                .filter(|g| {
                    if let Some(ref k) = filter.grantee_key {
                        if &g.grantee_key != k {
                            return false;
                        }
                    }
                    if let Some(ref k) = filter.granter_key {
                        if &g.granter_key != k {
                            return false;
                        }
                    }
                    if let Some(p) = filter.purpose {
                        if g.purpose != p {
                            return false;
                        }
                    }
                    if let Some(ref pfx) = filter.scope_prefix {
                        if !g.scope.starts_with(pfx) {
                            return false;
                        }
                    }
                    if !filter.include_revoked && g.revoked_at.is_some() {
                        return false;
                    }
                    if !filter.include_expired {
                        if let Some(t) = g.expires_at {
                            if t <= now {
                                return false;
                            }
                        }
                    }
                    true
                })
                .cloned()
                .collect();
            // Deterministic ordering for tests.
            rows.sort_by(|a, b| a.grant_id.cmp(&b.grant_id));
            Ok(rows)
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
