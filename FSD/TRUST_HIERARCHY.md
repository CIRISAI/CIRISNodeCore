# FSD: Trust Hierarchy — DIRECT/REGISTRY trust as the deferral-routing seam

**Status:** Design (draft v2) — supports CIRISNodeCore#2.
**Author:** Eric Moore (CIRIS Team) with Claude Opus 4.7
**Created:** 2026-05-15 (v1); **Revised:** 2026-05-15 (v2 — collapse + push-upstream pass).
**Risk:** Architectural. Adds the trust primitive that makes
WiseAuthorityService subsumable into NodeCore.

**Cross-coordinates with:**
- **CIRISPersist#47** — Counter-RII substrate (`federation_keys.consent_role`).
  The trust-axis columns + the `FederationDirectory` trait this FSD
  proposes are part of that same substrate package. This FSD pins the
  policy semantics on top of the storage primitive #47 establishes.
- **CIRISAgent#760** — Accord §RC `consent_role` primitive. The
  `trust_type` enum here tracks whichever lock #760 produces (A/B/C).

---

## 1. Why this exists

CIRISAgent's `WiseAuthorityService` today routes deferrals to a
hardcoded WA endpoint or a configured list. The agent can't make a
routing decision without that hardcoded knowledge — which means WA
can't fold out into the federation substrate without a way to express
"who is qualified to resolve a question in this domain?"

The trust hierarchy is the answer: trust is granted (not assumed),
indexed by domain, with two relationship axes — DIRECT (peer) and
REGISTRY (vouching). With this primitive in place, the agent's
routing decision becomes:

1. Classify the question's domain.
2. Look up REGISTRY-trusted vouchers for that domain.
3. Query each voucher's currently-vouched-for resolvers.
4. Apply Witness-Diversity (`MISSION.md` Primitive 10) to select N.
5. Route the deferral via `crate::wire::DeferralRequest`.

The agent never hardcodes endpoints. WA subsumes into NodeCore.

This is the **policy layer** that sits on top of CIRISPersist#47's
Counter-RII storage substrate — same data, two layers of concern.
Persist owns the columns + raw CRUD; node-core owns the
transitive-resolution policy + deferral routing.

---

## 2. Trust axes (locked from CIRISNodeCore#2)

Four axes per grant:

| Axis | Type | Required | Notes |
|---|---|---|---|
| `key` | Ed25519 pubkey (base64) | yes | Identity is the key per SCHEMA §2.2 |
| `trust_type` | `Temporary` / `Partnered` / `Anonymous` | yes | Mirrors CIRISAgent ConsentService taxonomy. Tracks CIRISAgent#760 lock. |
| `trust_relationship` | `Direct` / `Registry` | yes | New axis introduced here |
| `trust_domains` | `Vec<String>` | required when `Registry` | NEVER global; domain-scoped vouching only |

Defaults: `Temporary` + `Direct` + no domains. Most peer-to-peer
agent-to-agent observations land here. Interesting cases are
`Registry` entries with declared domain scopes.

---

## 3. DIRECT vs REGISTRY semantics

**DIRECT trust** — A trusts K_B as a peer/actor. K_B can act directly
with A: recognized peer, file Contributions, be referenced as an
authority on a question. A reaches K_B by name.

**REGISTRY trust** — A trusts K_B to vouch for other keys within
specific domains. If A trusts K_B as a registry for `medical_deferral`,
then when K_B vouches for K_C in `medical_deferral`, K_C becomes
transitively trustworthy in that domain only for A. A reaches K_C
through K_B's vouching.

**The transitive edge is domain-scoped.** K_C is trusted for
`medical_deferral` because K_B (a `medical_deferral` registry) vouched.
K_C is NOT trusted for `legal_review` even if K_B vouches for K_C
there too — A's trust in K_B is scoped to `medical_deferral` only.

Real-world analog: medical boards vouch for licensed doctors. You
trust the medical board for medical questions. You don't trust the
medical board to certify lawyers.

### 3.1 Transitive trust does NOT inherit `trust_type`

If A trusts K_B as `Partnered` + `Registry`, and K_B vouches for K_C
in domain D, then K_C is `Transitive` — NOT `Partnered`. K_B's
Partnered relationship is between A and K_B; K_C's relationship to A
is mediated through the registry. `Transitive` is its own [`TrustEdge`]
variant; it doesn't promote to a non-transitive type via the vouch.

### 3.2 Revocation propagates at query time, not write time

When A revokes its trust in K_B (a registry), all of K_B's
vouched-for keys lose their transitive trust for A — atomically and
without a sweep. The transitive resolution algorithm (§5) reads K_B's
current trust state on every query; if K_B is no longer trusted, no
transitive edge through K_B exists. The vouching `registry_vouch`
Contributions stay on the audit chain unchanged; they just stop
resolving.

This means: no background revocation worker. No stale state. The
audit chain is the source of truth for vouches; the directory is the
source of truth for current trust grants.

### 3.3 Bootstrap for TEMPORARY agents without a steward

The grantor field on every trust row is a federation pubkey. For
stewarded deployments, the steward key signs grants. For TEMPORARY
agents lacking a stewarded ancestor:

- Initial trust grants land at agent-construction time, inherited
  from the spawning environment's grant set (operator's CIRIS-RED
  default trust list, or whatever the deployment template provides).
- Self-issued grants are rejected at the persist boundary —
  `trusted_by == key` violates the integrity rule.
- Agents without inherited grants OR a steward operate with zero
  trust grants, which is the deferral-disabled state: the agent
  refuses to defer because it has no trust path to resolvers.

---

## 4. Where the trust hierarchy lives

### 4.1 Storage + raw CRUD — CIRISPersist (cross-link CIRISPersist#47)

Trust grants land as additive columns on `federation_keys`. Same V020
migration that #47 ships, with the trust-axis columns folded in:

```sql
ALTER TABLE federation_keys ADD COLUMN
  trust_type            TEXT NOT NULL DEFAULT 'temporary'
                        CHECK (trust_type IN ('temporary','partnered','anonymous')),
  trust_relationship    TEXT NOT NULL DEFAULT 'direct'
                        CHECK (trust_relationship IN ('direct','registry')),
  trust_domains         TEXT[]
                        CHECK (trust_relationship = 'direct' OR
                               (trust_relationship = 'registry' AND trust_domains IS NOT NULL
                                AND array_length(trust_domains, 1) > 0)),
  trusted_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  trusted_by            TEXT NOT NULL CHECK (trusted_by != key),  -- no self-trust
  expires_at            TIMESTAMPTZ
;
```

Transitions (grant / revoke / extend / domain-add / domain-remove)
write to `cirisaudit` per CIRISAgent#756 Q4 — audit chain owns state
transitions.

### 4.2 Raw query trait — CIRISPersist (proposed)

Persist exposes a new `FederationDirectory` trait alongside
`NodeCoreService`. Raw CRUD + simple lookups; **no** transitive
resolution (that's node-core policy):

```rust
// proposed in ciris_persist::cirisnode (or a new module if scope warrants)
pub trait FederationDirectory: Send + Sync {
    /// Insert or update a trust row. `grant.trusted_by` is verified
    /// against the federation_keys.signing_key_id integrity rule.
    fn grant_trust(
        &self,
        grant: TrustGrant,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    /// Soft-delete a trust row by setting `expires_at = NOW()`. Audit
    /// row recorded in cirisaudit.
    fn revoke_trust(
        &self,
        key: &str,
        revoked_by: &str,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    /// Point lookup — the raw federation_keys row including trust
    /// columns. No transitive resolution.
    fn lookup_trust(
        &self,
        key: &str,
    ) -> impl Future<Output = Result<Option<TrustRow>, Error>> + Send;

    /// All currently-trusted keys filtered by relationship + (for
    /// registries) domain. Used by node-core's transitive resolver.
    fn list_trusted_keys(
        &self,
        filter: TrustFilter,
    ) -> impl Future<Output = Result<Vec<TrustRow>, Error>> + Send;
}

pub struct TrustGrant {
    pub key: String,
    pub trust_type: TrustType,
    pub trust_relationship: TrustRelationship,
    pub trust_domains: Option<Vec<String>>,
    pub trusted_by: String,
    pub expires_at: Option<DateTime<Utc>>,
}

pub struct TrustRow {
    pub key: String,
    pub trust_type: TrustType,
    pub trust_relationship: TrustRelationship,
    pub trust_domains: Option<Vec<String>>,
    pub trusted_by: String,
    pub trusted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

pub struct TrustFilter {
    pub trust_type: Option<TrustType>,
    pub trust_relationship: Option<TrustRelationship>,
    pub domain: Option<String>,  // only meaningful with relationship=Registry
    pub include_expired: bool,
}
```

This is **5 methods + 4 types** on persist. The trait is consumable by
both `ciris-lens-core` (for Counter-RII trust-aware detection paths,
CIRISLensCore#21's downstream needs) and `ciris-node-core` (for
deferral routing). Neither has to depend on the other; both depend on
persist as they already do.

If persist team prefers to extend `NodeCoreService` rather than add a
sibling trait, that's their call — the surface is the same shape
either way. Push the contract upstream; let persist organize.

### 4.3 Transitive resolution — CIRISNodeCore (a function, not a trait)

Node-core's value-add is the transitive resolution policy. One free
function over the persist directory:

```rust
// crate::trust (new module)
pub async fn resolve_trust<D: FederationDirectory>(
    directory: &D,
    key: &str,
    domain: &str,
) -> Result<TrustEdge, SubstrateError> {
    // 1. Direct lookup
    if let Some(row) = directory.lookup_trust(key).await? {
        if !is_expired(&row) {
            return Ok(match row.trust_relationship {
                TrustRelationship::Direct => TrustEdge::Direct { trust_type: row.trust_type },
                TrustRelationship::Registry => TrustEdge::Registry {
                    trust_type: row.trust_type,
                    domains: row.trust_domains.unwrap_or_default(),
                },
            });
        }
    }

    // 2. Transitive — search for a currently-trusted registry that vouches
    //    for `key` in `domain`.
    let registries = directory
        .list_trusted_keys(TrustFilter {
            trust_relationship: Some(TrustRelationship::Registry),
            domain: Some(domain.into()),
            include_expired: false,
            ..Default::default()
        })
        .await?;
    for registry in registries {
        if registry_vouches_for(&registry.key, key, domain).await? {
            return Ok(TrustEdge::Transitive {
                via_registry: registry.key,
            });
        }
    }

    Ok(TrustEdge::Untrusted)
}

pub enum TrustEdge {
    Direct { trust_type: TrustType },
    Registry { trust_type: TrustType, domains: Vec<String> },
    Transitive { via_registry: String },  // domain implicit from query
    Untrusted,
}
```

One function. No trait. No constructor. The transitive search is the
only non-trivial logic and it's a dozen lines.

The `registry_vouches_for` helper queries `cirisnode.contributions`
via existing `list_contributions` filter:
`contribution_type='registry_vouch' AND author_id=$registry AND
payload->>'vouched_key' = $key AND payload->>'vouched_domain' = $domain`.

---

## 5. Registry-vouching as a Contribution kind

A registry vouching for a resolver is a federation event. Encoded as
a new `subject_kind` under `Proposal` per `SCHEMA.md` §3.2 (NOT a new
top-level `contribution_type` variant per §3.1):

```
| `registry_vouch` | §4.13 | A registry vouches for a key in a domain | Required if vouch jumps target's transitive-trust count past threshold |
```

Encoding as `Proposal` + `subject_kind = "registry_vouch"` is
structurally equivalent to adding a §3.1 variant (the storage shape,
the query path via `ContributionsFilter::subject_kind`, the
witness-set gate are all identical), but it avoids churning persist's
top-level `ContributionType` enum (which would force a coordinated
release across all federation consumers). Persist accepts it today
unchanged.

Envelope shape:
- `contribution_type = Proposal` (existing variant)
- `subject.{domain, language, subject = Some("registry_vouch")}`
- `author_id = K_B` (the registry)
- `payload` = `RegistryVouchPayload` per below

Typed payload in node-core's `crate::payloads::registry_vouch`:

```rust
pub struct RegistryVouchPayload {
    pub vouched_key: String,        // K_C — the key being vouched for
    pub vouched_domain: String,     // the domain scope of the vouch
    pub expires_at: Option<DateTime<Utc>>,  // None = open-ended
    pub rationale: String,          // why K_C is qualified
}
```

Envelope-level: `author_id = K_B` (the registry), standard signature.
Witness-set required when the vouch would jump K_C's transitive-trust
count past a policy-tunable threshold (mirrors the
`ExpertiseAttestation` jump-threshold gate at §3.5).

**Revocation of a vouch is author-only.** K_B revokes by submitting a
new `registry_vouch` with the same `vouched_key` + `vouched_domain`
and `expires_at = now()`. Counter-votes are not supported — they
muddy the trust graph and overlap with the `moderation_event` /
`slashing_attestation` path for bad-faith vouching.

---

## 6. DeferralRouter — a function, not a struct

```rust
// crate::routing — extend the existing module
pub async fn route_deferral<E, D, C>(
    engine: &E,
    directory: &D,
    classifier: C,
    question_context: &str,
    preferences: Option<&RoutingPreferences>,
    metadata: &impl ContributorMetadataProvider,
) -> Result<RoutingDecision, SubstrateError>
where
    E: NodeCoreService,
    D: FederationDirectory,
    C: Fn(&str) -> Result<String, SubstrateError>,
{
    let domain = classifier(question_context)?;

    // Find registries trusted for this domain.
    let registries = directory
        .list_trusted_keys(TrustFilter {
            trust_relationship: Some(TrustRelationship::Registry),
            domain: Some(domain.clone()),
            include_expired: false,
            ..Default::default()
        })
        .await?;

    // Union of vouched-for resolvers across registries.
    let mut candidates: Vec<RoutableContributor> = Vec::new();
    for registry in &registries {
        let vouched = list_vouched(engine, &registry.key, &domain).await?;
        candidates.extend(vouched);
    }

    // Apply Witness-Diversity via existing crate::routing::select_routed.
    let outcome = select_routed_inner(candidates, preferences, metadata);

    Ok(RoutingDecision {
        domain,
        registries_consulted: registries.into_iter().map(|r| r.key).collect(),
        selected_resolvers: outcome.routed,
        diversity_summary: outcome,
    })
}
```

One function. The `classifier` is a closure — single method, no
state, no shared dispatch (same blanket-impl pattern
`ContributorMetadataProvider` uses today). Callers pass `|ctx|
Ok("medical_deferral".into())` or whatever heuristic / lens-core
classifier they prefer.

`list_vouched` is a small helper over `engine.list_contributions` with
the filter from §4.3's `registry_vouches_for`.

---

## 7. Multi-resolver aggregation reuses existing primitives

When N resolvers are routed (N > 1 — typical for high-stakes
deferrals), each resolver casts a Vote on the deferral_request
Contribution. The §7 weighted aggregate (`crate::aggregate`, shipped)
computes the rolling tally. Threshold-crossing for "deferral
resolved" is consumer policy:

- **Unanimous** (strict): all resolvers must approve.
- **Quorum-weighted**: `approval_ratio() >= 0.66` per
  `Aggregate::Resolved`.
- **First-N**: first N substantive responses; abstains don't count.

Default = quorum-weighted (matches the rubric crowdsourcing flow's
threshold gate).

---

## 8. Reconsideration applies cleanly

If the agent disagrees with the resolved deferral, it files a
`ReconsiderationRequest` per `SCHEMA.md` §4.12 / §9. Existing
primitive — bounds enforced by the engine (180-day time bound for
NewEvidence / ProceduralError, unlimited for QuorumCompromise; 3
filings trips harassment review).

No new spec work.

---

## 9. Out of scope (this FSD)

- **Domain taxonomy ownership** — the canonical set of domain
  identifiers (e.g. `medical_deferral`, `legal_review`,
  `ethical_arbitration`). Either a CIRISRegistry-published manifest
  or a CIRISAgent-side config list. Lean: registry manifest matching
  the `manifest.json` pattern for languages. Decision deferred to
  v0.1.0 cut.
- **`DomainClassifier` heuristic** — the impl that maps question
  context to a domain identifier. Plausibly a CIRISLensCore-side
  scoring task (lens-core already classifies trace content). The
  trait shape (closure) is here; the impl isn't.
- **CIRISAgent migration sequence** — converting hardcoded WA
  endpoints into DIRECT trust grants on first boot of the post-fold
  agent is CIRISAgent's concern. Covered in the CIRISAgent issue that
  lands the WA shim (forthcoming after #1 Phase 1).
- **ConsentService fold into LensCore** — separate issue (CIRISAgent
  #760 + forthcoming LensCore issue). Shares the `trust_type` column
  shape with this design but owns its own decay protocol + bilateral
  PARTNERED approval loop.

---

## 10. Implementation order

| # | Step | Dep | Repo |
|---|---|---|---|
| 1 | `federation_keys` trust columns + `FederationDirectory` trait + PostgresBackend impl | CIRISAgent#760 §RC consent_role lock | CIRISPersist (absorbs into #47) |
| 2 | `registry_vouch` Contribution variant — SCHEMA + payload struct + impl Message + handler | (1) | CIRISNodeCore |
| 3 | `crate::trust::resolve_trust` function + tests against MockEngine | (1) + (2) | CIRISNodeCore |
| 4 | `crate::routing::route_deferral` function | (3) | CIRISNodeCore |
| 5 | PyO3 surface (extends CIRISNodeCore#1 Phase 1) | (4) + #1 Phase 1 | CIRISNodeCore |
| 6 | WiseAuthority shim at agent — delegates to NodeClient + the trust ledger | (5) | CIRISAgent |

Steps 2-5 are all node-core work and can land in successive commits
once (1) ships. Step 6 is cross-repo + CIRISAgent migration coverage.

---

## 11. References

- CIRISNodeCore#1 — adapter swap (PyO3 surface this FSD extends)
- CIRISNodeCore#2 — the umbrella issue this FSD details
- CIRISPersist#47 — Counter-RII substrate (absorbs §4.1 columns + §4.2 trait)
- CIRISAgent#760 — Accord §RC consent_role primitive (trust_type origin)
- CIRISLensCore#21 — Counter-RII detector (downstream consumer of `FederationDirectory`)
- `MISSION.md` — eleven primitives consumed by the design
  (Identity, Vote, Moderation, Witness-Diversity, Reconsideration,
  Truth-Grounding, Contribution, Expertise)
- `SCHEMA.md` §3.1 — adds `registry_vouch` to the `contribution_type`
  enum; §13.2 pending/canonical split applies unchanged
- `crate::aggregate` (shipped at `9b584aa`) — multi-resolver tallying
- `crate::routing` (shipped at `9eef3f5`) — Witness-Diversity selection
- `crate::sign` (shipped at `ca8ddde`) — envelope construction
