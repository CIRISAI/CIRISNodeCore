//! Deferral-routing diversity selection per `MISSION.md` §3.3.
//!
//! Persist's `NodeCoreService::routable_contributors` returns
//! `RoutableContributor { contributor_id, expertise }` — Expertise-
//! non-zero × Active-tier filter applied. Steps 3-4 of §3.3
//! (diversity preferences + bounded count) are policy that lives
//! here.
//!
//! Per-contributor metadata (jurisdiction, operator) isn't part of
//! the substrate's `RoutableContributor` — the federation directory
//! holds that. Callers supply a [`ContributorMetadataProvider`]
//! closure / impl that looks up metadata by contributor id.

use crate::payloads::deferral::{DiversityPolicy, RoutingPreferences};
use crate::substrate::{NodeCoreService, RoutableContributor, SubstrateError};

/// Per-contributor metadata that the federation directory holds.
/// Mirrors the same fields `Witness` carries on the
/// witness-diversity path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContributorMetadata {
    /// Jurisdiction code (e.g. `"ET"`, `"KE"`, `"US"`).
    pub jurisdiction: String,
    /// Operator id (organization or self).
    pub operator: String,
}

/// Trait for looking up per-contributor metadata. Implementations
/// live outside node-core (federation-directory client).
pub trait ContributorMetadataProvider: Send + Sync {
    /// Returns `None` if the contributor has no recorded metadata —
    /// they fall to the bottom of diversity tie-breaks.
    fn metadata(&self, contributor_id: &str) -> Option<ContributorMetadata>;
}

/// Closure-shaped convenience impl.
impl<F> ContributorMetadataProvider for F
where
    F: Fn(&str) -> Option<ContributorMetadata> + Send + Sync,
{
    fn metadata(&self, contributor_id: &str) -> Option<ContributorMetadata> {
        self(contributor_id)
    }
}

/// Outcome of a routing selection — what got routed plus the
/// diversity summary the consumer can persist or display.
#[derive(Debug, Clone, PartialEq)]
pub struct RoutingOutcome {
    /// Federation identities selected, in selection order.
    pub routed: Vec<String>,
    /// Distinct jurisdictions represented in `routed`.
    pub jurisdictions_distinct: Vec<String>,
    /// Distinct operators represented in `routed`.
    pub operators_distinct: Vec<String>,
    /// Whether the selection met the request's `min_responders` (or
    /// the §3.3 default of 5 if unset). When `false`, the consumer
    /// should fall back to a wider net or surface the gap.
    pub min_met: bool,
}

const DEFAULT_MIN: u32 = 5;
const DEFAULT_MAX: u32 = 9;

/// Select routed responders per `MISSION.md` §3.3 steps 3-4.
///
/// Algorithm:
/// 1. Pull `routable_contributors(domain, language)` from the engine.
/// 2. Sort by expertise descending.
/// 3. Apply diversity policy via greedy selection — pick the highest-
///    expertise candidate not yet represented in the diversity bucket
///    being enforced; cycle the bucket as it fills.
/// 4. Bound by `preferences.max_responders` (default 9).
pub async fn select_routed<E, M>(
    engine: &E,
    domain: &str,
    language: &str,
    preferences: Option<&RoutingPreferences>,
    metadata: &M,
) -> Result<RoutingOutcome, SubstrateError>
where
    E: NodeCoreService,
    M: ContributorMetadataProvider,
{
    let mut candidates = engine.routable_contributors(domain, language).await?;
    candidates.sort_by(|a, b| {
        b.expertise
            .partial_cmp(&a.expertise)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let max = preferences
        .and_then(|p| p.max_responders)
        .unwrap_or(DEFAULT_MAX) as usize;
    let min = preferences
        .and_then(|p| p.min_responders)
        .unwrap_or(DEFAULT_MIN) as usize;
    let policy = preferences.and_then(|p| p.diversity).unwrap_or(DiversityPolicy::None);

    let routed = select_with_diversity(candidates, policy, max, metadata);

    let mut jurisdictions: Vec<String> = Vec::new();
    let mut operators: Vec<String> = Vec::new();
    for id in &routed {
        if let Some(m) = metadata.metadata(id) {
            if !jurisdictions.contains(&m.jurisdiction) {
                jurisdictions.push(m.jurisdiction);
            }
            if !operators.contains(&m.operator) {
                operators.push(m.operator);
            }
        }
    }

    Ok(RoutingOutcome {
        min_met: routed.len() >= min,
        routed,
        jurisdictions_distinct: jurisdictions,
        operators_distinct: operators,
    })
}

/// Greedy diversity-aware selection. For `DiversityPolicy::None`,
/// take the top `max` by expertise. For `Jurisdictional` /
/// `Organizational`, sweep the candidate list multiple times: each
/// sweep picks one candidate per distinct bucket value not yet
/// represented in the current sweep; once every candidate is either
/// selected or has its bucket represented, reset the seen-buckets
/// set and sweep again. Stops at `max` selected or candidates
/// exhausted.
fn select_with_diversity<M: ContributorMetadataProvider>(
    candidates: Vec<RoutableContributor>,
    policy: DiversityPolicy,
    max: usize,
    metadata: &M,
) -> Vec<String> {
    if max == 0 {
        return Vec::new();
    }
    if let DiversityPolicy::None = policy {
        return candidates.into_iter().take(max).map(|c| c.contributor_id).collect();
    }

    let mut remaining: Vec<(RoutableContributor, Option<String>)> = candidates
        .into_iter()
        .map(|c| {
            let bucket = metadata.metadata(&c.contributor_id).map(|m| match policy {
                DiversityPolicy::Jurisdictional => m.jurisdiction,
                DiversityPolicy::Organizational => m.operator,
                DiversityPolicy::None => unreachable!("handled above"),
            });
            (c, bucket)
        })
        .collect();

    let mut routed: Vec<String> = Vec::new();
    while routed.len() < max && !remaining.is_empty() {
        let mut seen_buckets: Vec<String> = Vec::new();
        let mut i = 0;
        while i < remaining.len() && routed.len() < max {
            let bucket = remaining[i].1.clone();
            let take = match &bucket {
                Some(b) if !seen_buckets.contains(b) => true,
                None => false, // contributors w/ no metadata wait for the next sweep
                Some(_) => false,
            };
            if take {
                if let Some(b) = bucket {
                    seen_buckets.push(b);
                }
                let (cand, _) = remaining.remove(i);
                routed.push(cand.contributor_id);
            } else {
                i += 1;
            }
        }
        // If a sweep made no progress (all remaining have None metadata
        // or their bucket was already seen), break to avoid an infinite
        // loop. Pick contributors without metadata to fill remaining
        // slots.
        if seen_buckets.is_empty() && !remaining.is_empty() {
            while routed.len() < max && !remaining.is_empty() {
                let (cand, _) = remaining.remove(0);
                routed.push(cand.contributor_id);
            }
            break;
        }
    }
    routed
}

// ── Deferral routing (composes trust + diversity) ────────────────────────

use crate::trust::{
    list_vouched_for, FederationDirectory, TrustFilter, TrustRelationship,
};

/// Result of a full deferral-routing pass. Captures the resolver
/// set plus the audit-trail metadata callers persist.
#[derive(Debug, Clone, PartialEq)]
pub struct RoutingDecision {
    /// Domain the question was classified to.
    pub domain: String,
    /// Registries consulted (currently trusted for the domain).
    pub registries_consulted: Vec<String>,
    /// Resolvers chosen after diversity selection.
    pub selected_resolvers: Vec<String>,
    /// Diversity summary (jurisdictions, operators, min_met).
    pub diversity_summary: RoutingOutcome,
}

/// Compose: classify → consult trusted registries → expand vouched
/// resolvers → apply Witness-Diversity selection.
///
/// Per `MISSION.md` §3.3 + `FSD/TRUST_HIERARCHY.md` §6.
///
/// `classifier` is a closure (single method, no state); pass any
/// `Fn(&str) -> Result<String, SubstrateError>`.
pub async fn route_deferral<E, D, C, M>(
    engine: &E,
    directory: &D,
    classifier: C,
    question_context: &str,
    preferences: Option<&crate::payloads::deferral::RoutingPreferences>,
    metadata: &M,
) -> Result<RoutingDecision, crate::substrate::SubstrateError>
where
    E: NodeCoreService,
    D: FederationDirectory,
    C: Fn(&str) -> Result<String, crate::substrate::SubstrateError>,
    M: ContributorMetadataProvider,
{
    let domain = classifier(question_context)?;

    // Trusted registries for this domain.
    let registries = directory
        .list_trusted_keys(TrustFilter {
            trust_relationship: Some(TrustRelationship::Registry),
            domain: Some(domain.clone()),
            include_expired: false,
            ..Default::default()
        })
        .await?;

    // Union of vouched-for resolvers across registries.
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut candidates: Vec<RoutableContributor> = Vec::new();
    for registry in &registries {
        let vouched = list_vouched_for(engine, &registry.key, &domain).await?;
        for key in vouched {
            if seen.insert(key.clone()) {
                // Pull each candidate's expertise via routable_contributors —
                // call once per (domain, language) cell rather than per-key.
                // For v0.1.0-dev: assume one cell at a time; multi-language
                // routing is a v0.1.0-cut concern.
                candidates.push(RoutableContributor {
                    contributor_id: key,
                    expertise: 0.0, // populated below
                });
            }
        }
    }

    // Enrich expertise from routable_contributors for the cell. We use
    // the first registry's perceived cell language — domain alone is
    // the routing key here. Language enrichment + cross-language
    // routing is on the v0.1.0-cut roadmap.
    let language = preferences
        .and_then(|_p| None::<&str>) // RoutingPreferences doesn't carry language today
        .unwrap_or("");
    if !language.is_empty() {
        let routable_with_expertise = engine.routable_contributors(&domain, language).await?;
        for c in &mut candidates {
            if let Some(found) = routable_with_expertise.iter().find(|r| r.contributor_id == c.contributor_id) {
                c.expertise = found.expertise;
            }
        }
    }

    let diversity_summary = select_with_diversity_outcome(candidates, preferences, metadata);

    let registries_consulted = registries.into_iter().map(|r| r.key).collect();
    let selected_resolvers = diversity_summary.routed.clone();

    Ok(RoutingDecision {
        domain,
        registries_consulted,
        selected_resolvers,
        diversity_summary,
    })
}

/// Inner diversity selection that takes a pre-built candidate set.
/// Mirrors [`select_routed`]'s sort + diversity sweep over an
/// arbitrary candidate vec, without re-querying the engine.
fn select_with_diversity_outcome<M: ContributorMetadataProvider>(
    mut candidates: Vec<RoutableContributor>,
    preferences: Option<&crate::payloads::deferral::RoutingPreferences>,
    metadata: &M,
) -> RoutingOutcome {
    candidates.sort_by(|a, b| {
        b.expertise
            .partial_cmp(&a.expertise)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let max = preferences
        .and_then(|p| p.max_responders)
        .unwrap_or(DEFAULT_MAX) as usize;
    let min = preferences
        .and_then(|p| p.min_responders)
        .unwrap_or(DEFAULT_MIN) as usize;
    let policy = preferences
        .and_then(|p| p.diversity)
        .unwrap_or(crate::payloads::deferral::DiversityPolicy::None);

    let routed = select_with_diversity(candidates, policy, max, metadata);

    let mut jurisdictions: Vec<String> = Vec::new();
    let mut operators: Vec<String> = Vec::new();
    for id in &routed {
        if let Some(m) = metadata.metadata(id) {
            if !jurisdictions.contains(&m.jurisdiction) {
                jurisdictions.push(m.jurisdiction);
            }
            if !operators.contains(&m.operator) {
                operators.push(m.operator);
            }
        }
    }

    RoutingOutcome {
        min_met: routed.len() >= min,
        routed,
        jurisdictions_distinct: jurisdictions,
        operators_distinct: operators,
    }
}
