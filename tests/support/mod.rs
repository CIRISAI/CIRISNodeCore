//! In-memory `MockEngine` — implements [`NodeCoreEngine`] for tests.
//!
//! Closure of FSD/SUBSTRATE_INTEGRATION.md OQ-3 ("tests/support/ in this
//! crate; promote to a separate crate only when downstream consumers
//! want it"). Lives in `tests/support/` so each integration test file
//! at the top of `tests/` can include it via `mod support;`.
//!
//! Discipline:
//!
//! - **Records writes verbatim.** Each typed-write method appends to an
//!   internal Vec; the persisted id returned is a deterministic ULID
//!   built from the test's mock counter so assertions are stable.
//! - **Canned reads.** Fixture setters (`set_routable`, `set_credits`,
//!   `set_expertise`) pre-load the read views the handler calls into.
//! - **Inspectors for assertions.** `contributions()`, `votes()`,
//!   `moderation_events()`, etc. return clones of recorded writes so
//!   tests can verify what the handler persisted.
//! - **No real crypto.** `steward_sign` returns a deterministic
//!   `HybridSignature` shaped like the real thing but with placeholder
//!   bytes — never use this mock to validate signatures, only to
//!   verify handler logic invokes the signing path.

#![allow(dead_code)] // each integration test file uses a subset of the helpers

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;

use ciris_node_core::{
    Cell, ContributionEnvelope, ContributorId, Error, HybridSignature, NodeCoreEngine, Result,
    Vote,
};
use ciris_node_core::ledger::{
    CommonsCreditsLedger, CreditsEntry, ExpertiseEntry, ExpertiseLedger, VoteWeight,
};

/// Test fixture state. All mutable state lives behind a single
/// `Mutex` — fine for unit-test workloads, simple to reason about.
#[derive(Default)]
struct MockState {
    // Recorded writes
    contributions: Vec<ContributionEnvelope>,
    votes: Vec<Vote>,

    // Canned read views — populated by fixture setters
    // Key: (domain, language)
    routable: HashMap<(String, String), Vec<ContributorId>>,
    // Key: (contributor, domain, language, subject)
    credits: HashMap<(ContributorId, String, String, String), f64>,
    // Key: (contributor, domain, language)
    expertise: HashMap<(ContributorId, String, String), f64>,
    // Active tier — present means active
    active_tier: std::collections::HashSet<ContributorId>,

    // Counter for deterministic generated ids
    next_id: u32,
}

/// In-memory [`NodeCoreEngine`] for handler tests.
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

    /// Pre-load routable-contributors response for `(domain, language)`.
    pub fn set_routable(&self, domain: &str, language: &str, who: Vec<ContributorId>) {
        let mut st = self.state.lock().unwrap();
        st.routable
            .insert((domain.into(), language.into()), who);
    }

    pub fn set_credits(
        &self,
        contributor: &ContributorId,
        domain: &str,
        language: &str,
        subject: &str,
        credits: f64,
    ) {
        let mut st = self.state.lock().unwrap();
        st.credits.insert(
            (contributor.clone(), domain.into(), language.into(), subject.into()),
            credits,
        );
    }

    pub fn set_expertise(
        &self,
        contributor: &ContributorId,
        domain: &str,
        language: &str,
        standing: f64,
    ) {
        let mut st = self.state.lock().unwrap();
        st.expertise.insert(
            (contributor.clone(), domain.into(), language.into()),
            standing,
        );
    }

    pub fn set_active(&self, contributor: &ContributorId, active: bool) {
        let mut st = self.state.lock().unwrap();
        if active {
            st.active_tier.insert(contributor.clone());
        } else {
            st.active_tier.remove(contributor);
        }
    }

    // ── Inspectors for assertions ────────────────────────────────────────

    pub fn contributions(&self) -> Vec<ContributionEnvelope> {
        self.state.lock().unwrap().contributions.clone()
    }

    pub fn votes(&self) -> Vec<Vote> {
        self.state.lock().unwrap().votes.clone()
    }

    pub fn write_count(&self) -> usize {
        let st = self.state.lock().unwrap();
        st.contributions.len() + st.votes.len()
    }

    // ── Internal helpers ─────────────────────────────────────────────────

    fn next_id(&self, prefix: &str) -> String {
        let mut st = self.state.lock().unwrap();
        st.next_id += 1;
        format!("{prefix}_mock_{:08x}", st.next_id)
    }
}

#[async_trait]
impl NodeCoreEngine for MockEngine {
    async fn put_contribution(&self, envelope: ContributionEnvelope) -> Result<String> {
        let id = self.next_id("ctrb");
        let mut st = self.state.lock().unwrap();
        st.contributions.push(envelope);
        Ok(id)
    }

    async fn cast_vote(&self, vote: Vote) -> Result<String> {
        let id = self.next_id("vote");
        let mut st = self.state.lock().unwrap();
        st.votes.push(vote);
        Ok(id)
    }

    async fn update_credits_ledger(
        &self,
        contributor: &ContributorId,
        cell: &Cell,
        delta: f64,
    ) -> Result<()> {
        let subject = cell.subject.clone().unwrap_or_default();
        let key = (contributor.clone(), cell.domain.clone(), cell.language.clone(), subject);
        let mut st = self.state.lock().unwrap();
        let entry = st.credits.entry(key).or_insert(0.0);
        let next = *entry + delta;
        if next < 0.0 {
            return Err(Error::LedgerInvariant(format!(
                "credits would go negative: {} + {} = {}",
                *entry, delta, next
            )));
        }
        *entry = next;
        Ok(())
    }

    async fn update_expertise_ledger(
        &self,
        contributor: &ContributorId,
        cell: &Cell,
        delta: f64,
    ) -> Result<()> {
        let key = (contributor.clone(), cell.domain.clone(), cell.language.clone());
        let mut st = self.state.lock().unwrap();
        let entry = st.expertise.entry(key).or_insert(0.0);
        let next = *entry + delta;
        if next < 0.0 {
            return Err(Error::LedgerInvariant(format!(
                "expertise would go negative: {} + {} = {}",
                *entry, delta, next
            )));
        }
        *entry = next;
        Ok(())
    }

    async fn read_vote_weight(
        &self,
        contributor: &ContributorId,
        cell: &Cell,
    ) -> Result<VoteWeight> {
        let st = self.state.lock().unwrap();
        let subject = cell.subject.clone().unwrap_or_default();
        let credits = st
            .credits
            .get(&(contributor.clone(), cell.domain.clone(), cell.language.clone(), subject))
            .copied()
            .unwrap_or(0.0);
        let expertise = st
            .expertise
            .get(&(contributor.clone(), cell.domain.clone(), cell.language.clone()))
            .copied()
            .unwrap_or(0.0);
        let active = st.active_tier.contains(contributor);
        Ok(VoteWeight {
            credits,
            expertise_multiplier: expertise,
            active_tier_multiplier: if active { 1.0 } else { 0.0 },
        })
    }

    async fn get_credits_ledger(&self, contributor: &ContributorId) -> Result<CommonsCreditsLedger> {
        let st = self.state.lock().unwrap();
        let entries = st
            .credits
            .iter()
            .filter(|((c, _, _, _), _)| c == contributor)
            .map(|((_, d, l, s), credits)| CreditsEntry {
                cell: Cell::credits(d, l, s),
                credits: *credits,
                updated_at: Utc::now(),
            })
            .collect();
        Ok(CommonsCreditsLedger {
            contributor_id: contributor.clone(),
            entries,
            ledger_signature: mock_signature(),
        })
    }

    async fn get_expertise_ledger(&self, contributor: &ContributorId) -> Result<ExpertiseLedger> {
        let st = self.state.lock().unwrap();
        let active = st.active_tier.contains(contributor);
        let entries = st
            .expertise
            .iter()
            .filter(|((c, _, _), _)| c == contributor)
            .map(|((_, d, l), standing)| ExpertiseEntry {
                cell: Cell::expertise(d, l),
                standing: *standing,
                active_tier: active,
                updated_at: Utc::now(),
            })
            .collect();
        Ok(ExpertiseLedger {
            contributor_id: contributor.clone(),
            entries,
            ledger_signature: mock_signature(),
        })
    }

    async fn routable_contributors(
        &self,
        domain: &str,
        language: &str,
    ) -> Result<Vec<ContributorId>> {
        let st = self.state.lock().unwrap();
        Ok(st
            .routable
            .get(&(domain.into(), language.into()))
            .cloned()
            .unwrap_or_default())
    }

    async fn steward_sign(&self, _canonical_bytes: &[u8]) -> Result<HybridSignature> {
        Ok(mock_signature())
    }

    fn canonicalize(&self, value: &serde_json::Value) -> Result<Vec<u8>> {
        // Deterministic for tests; real persist canonicalizer is the
        // PythonJsonDumpsCanonicalizer shape per CIRISPersist#7.
        Ok(serde_json::to_vec(value)?)
    }
}

fn mock_signature() -> HybridSignature {
    HybridSignature {
        ed25519: "mock_ed25519_placeholder".into(),
        ml_dsa_65: None,
        signed_at: Utc::now(),
    }
}
