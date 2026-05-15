# FSD: Trust Hierarchy — DIRECT/REGISTRY trust as the deferral-routing seam

**Status:** Design (draft) — supports CIRISNodeCore#2.
**Author:** Eric Moore (CIRIS Team) with Claude Opus 4.7
**Created:** 2026-05-15
**Risk:** Architectural. Adds the trust primitive that makes
WiseAuthorityService subsumable into NodeCore. Cross-coordinates with
CIRISPersist#47 (federation_keys schema) and CIRISAgent#760 (Accord §RC
consent_role).

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

---

## 2. Trust axes (locked from CIRISNodeCore#2)

Four axes per grant:

| Axis | Type | Required | Notes |
|---|---|---|---|
| `key` | Ed25519 pubkey (base64) | yes | Identity is the key per SCHEMA §2.2 |
| `trust_type` | `Temporary` / `Partnered` / `Anonymous` | yes | Mirrors CIRISAgent ConsentService taxonomy |
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

---

## 4. Where the trust hierarchy lives

### 4.1 Storage — CIRISPersist (cross-link CIRISPersist#47)

Trust grants land as additive columns on `federation_keys`:

```sql
ALTER TABLE federation_keys ADD COLUMN
  trust_type            TEXT NOT NULL DEFAULT 'temporary'
                        CHECK (trust_type IN ('temporary','partnered','anonymous')),
  trust_relationship    TEXT NOT NULL DEFAULT 'direct'
                        CHECK (trust_relationship IN ('direct','registry')),
  trust_domains         TEXT[]                      -- nullable; required when relationship='registry'
                        CHECK (trust_relationship = 'direct' OR
                               (trust_relationship = 'registry' AND trust_domains IS NOT NULL
                                AND array_length(trust_domains, 1) > 0)),
  trusted_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  trusted_by            TEXT NOT NULL,              -- grantor pubkey
  expires_at            TIMESTAMPTZ                 -- nullable
;
```

Transitions (grant / revoke / extend / domain-add / domain-remove)
write to `cirisaudit` per CIRISAgent#756 Q4 — audit chain owns state
transitions.

### 4.2 Trait — CIRISNodeCore

Persist exposes the storage; node-core composes the policy. The
`TrustLedger` trait lives in node-core, NOT persist — same pattern as
the eleven-primitive surface that sits over persist's typed-write
methods.

```rust
// src/trust.rs (new module)

pub trait TrustLedger: Send + Sync {
    /// Grant trust to a key. `signed_by` MUST equal one of the agent's
    /// authorized grantor keys (typically the steward key). Audited
    /// via cirisaudit.
    fn grant_trust(
        &self,
        grant: TrustGrant,
    ) -> impl Future<Output = Result<(), SubstrateError>> + Send;

    /// Revoke a trust grant. Audited.
    fn revoke_trust(
        &self,
        key: &str,
        signed_by: &str,
    ) -> impl Future<Output = Result<(), SubstrateError>> + Send;

    /// Lookup: is K trusted for `domain`? Returns the resolved trust
    /// edge (Direct → trusted-as-peer, Registry → trusted-as-voucher,
    /// Transitive → vouched-for, None → untrusted).
    fn query_trust(
        &self,
        key: &str,
        domain: &str,
    ) -> impl Future<Output = Result<TrustEdge, SubstrateError>> + Send;

    /// Returns keys A trusts as registries for `domain`. Used by
    /// the deferral router to find vouchers.
    fn list_registries_for_domain(
        &self,
        domain: &str,
    ) -> impl Future<Output = Result<Vec<String>, SubstrateError>> + Send;

    /// Returns keys vouched-for by `registry_key` in `domain`. Reads
    /// from `cirisnode.contributions` filtered to
    /// `contribution_type = registry_vouch`.
    fn list_vouched_for(
        &self,
        registry_key: &str,
        domain: &str,
    ) -> impl Future<Output = Result<Vec<String>, SubstrateError>> + Send;
}

pub struct TrustGrant {
    pub key: String,
    pub trust_type: TrustType,
    pub trust_relationship: TrustRelationship,
    pub trust_domains: Option<Vec<String>>,
    pub signed_by: String,
    pub expires_at: Option<DateTime<Utc>>,
}

pub enum TrustEdge {
    Direct { trust_type: TrustType },
    Registry { trust_type: TrustType, domains: Vec<String> },
    /// K_C is trusted because K_B (a domain-registry) vouched for K_C.
    Transitive { via_registry: String, domain: String },
    Untrusted,
}
```

Default impl bridges to persist via the engine handle — same shape as
`crate::service::NodeCore<E: NodeCoreService>`.

---

## 5. Registry-vouching as a Contribution kind

A registry vouching for a resolver is itself a federation event. New
variant added to `SCHEMA.md` §3.1 `contribution_type` enum:

```
| `registry_vouch` | §4.13 | A registry vouches for a key in a domain |
```

Typed payload (node-core's `crate::payloads::registry_vouch`):

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
count past a policy-tunable threshold (mirrors the ExpertiseAttestation
jump-threshold gate at §3.5).

`TrustLedger::list_vouched_for` reads from `cirisnode.contributions`
filtered to this `contribution_type` + `subject.subject = vouched_domain`.

---

## 6. DeferralRouter — the seam that makes WA eliminable

```rust
// src/router.rs (new module)

pub struct DeferralRouter<E: NodeCoreService, T: TrustLedger> {
    engine: Arc<E>,
    trust: Arc<T>,
    classifier: Arc<dyn DomainClassifier>,
    witness_diversity: WitnessDiversityPolicy,
}

impl<E: NodeCoreService, T: TrustLedger> DeferralRouter<E, T> {
    /// Pure-policy method — no envelope construction. Caller signs +
    /// submits via crate::sign + crate::service::NodeCore.
    pub async fn select_resolvers(
        &self,
        question_context: &str,
    ) -> Result<RoutingDecision, SubstrateError> {
        // 1. Classify the domain
        let domain = self.classifier.classify(question_context)?;

        // 2. Find registries A trusts for this domain
        let registries = self.trust.list_registries_for_domain(&domain).await?;

        // 3. Union of vouched-for resolvers across registries
        let mut candidates: Vec<String> = Vec::new();
        for registry in &registries {
            let vouched = self.trust.list_vouched_for(registry, &domain).await?;
            candidates.extend(vouched);
        }

        // 4. Apply Witness-Diversity — reuse crate::routing::select_routed's
        //    diversity algorithm. Resolver metadata = persist's federation_keys
        //    row's jurisdiction + operator fields.
        let routing_outcome = apply_diversity(
            candidates,
            &self.witness_diversity,
            self.engine.as_ref(),
        ).await?;

        Ok(RoutingDecision {
            domain,
            registries_consulted: registries,
            selected_resolvers: routing_outcome.routed,
            diversity_summary: routing_outcome,
        })
    }
}

pub trait DomainClassifier: Send + Sync {
    fn classify(&self, question_context: &str) -> Result<String, SubstrateError>;
}
```

The router returns a `RoutingDecision`; the caller (agent shim or
NodeClient method) constructs the signed `DeferralRequest`
Contribution per existing `crate::wire::DeferralRequest` shape, sends
it via `engine.put_contribution`, gets the routed-set ack back.

---

## 7. Multi-resolver aggregation reuses existing primitives

When N resolvers are routed (N > 1 — the typical case for high-stakes
deferrals), each resolver casts a Vote on the deferral_request
Contribution. The §7 weighted aggregate (`crate::aggregate`) we
already built computes the rolling tally. Threshold-crossing for "the
deferral is resolved" is policy:

- **Unanimous** (strict): all resolvers must approve.
- **Quorum-weighted**: `approval_ratio() >= 0.66` per
  `Aggregate::Resolved`.
- **First-N**: first N substantive responses; abstains don't count.

Selectable at consumer policy time. Default = quorum-weighted (matches
the rubric crowdsourcing flow's threshold gate).

---

## 8. Reconsideration applies cleanly

If the agent disagrees with the resolved deferral, it files a
`ReconsiderationRequest` per `SCHEMA.md` §4.12 / §9. Existing
primitive — bounds enforced by the engine (180-day time bound for
NewEvidence / ProceduralError, unlimited for QuorumCompromise; 3
filings trips harassment review).

No new spec work for this — the path already exists.

---

## 9. Migration — agent's hardcoded WAs become DIRECT trust grants

On first boot of the post-fold agent:

1. Read agent's current `cirisnode_url` + WA endpoint config from
   the pre-fold state.
2. For each configured endpoint, derive the WA's Ed25519 pubkey (from
   its registration record in CIRISRegistry).
3. Issue a `TrustGrant { key, trust_type: Temporary, trust_relationship:
   Direct, trust_domains: None, signed_by: steward_key, expires_at:
   Some(now + 90d) }` — Temporary + Direct + 90-day expiry forces a
   review pass before automatic expiration.
4. Audit the migration grant in `cirisaudit`.

Existing deployments don't change deferral routing behavior; they just
flow the decision through the trust ledger. After the migration
window, the steward can upgrade to `Partnered` + `Registry` grants if
the WA is part of a multi-resolver structure.

---

## 10. Out of scope (this FSD)

- **Domain taxonomy ownership** — the canonical set of domain
  identifiers (e.g. `medical_deferral`, `legal_review`,
  `ethical_arbitration`) is federation policy, not encoded here.
  Either a CIRISRegistry-published manifest or a CIRISAgent-side
  config list. Lean: registry manifest matching the `manifest.json`
  pattern for languages. Decision deferred to the v0.1.0 cut.
- **`DomainClassifier` impl** — the heuristic that maps question
  context to a domain identifier. Plausibly a CIRISLensCore-side
  scoring task (lens-core already classifies trace content). Out of
  this FSD's scope.
- **ConsentService fold into LensCore** — separate issue
  (CIRISAgent#760 + forthcoming LensCore issue). Shares the
  `trust_type` column shape but owns its own decay protocol +
  bilateral PARTNERED approval loop.

---

## 11. Implementation order

| # | Step | Dep | Repo |
|---|---|---|---|
| 1 | `federation_keys` trust columns (V020+ migration) | CIRISAgent#760 §RC consent_role lock | CIRISPersist (#47 absorbs) |
| 2 | `registry_vouch` Contribution variant — SCHEMA + payload struct + impl Message + handler | (1) | CIRISNodeCore (this repo) |
| 3 | `TrustLedger` trait + persist-backed impl + in-memory mock | (1) + (2) | CIRISNodeCore |
| 4 | `DeferralRouter` + `DomainClassifier` trait + stub classifier | (3) | CIRISNodeCore |
| 5 | PyO3 surface for the above (extends CIRISNodeCore#1 Phase 1) | (4) + #1 Phase 1 | CIRISNodeCore |
| 6 | WiseAuthority shim at agent — delegates to NodeClient through the trust ledger | (5) | CIRISAgent |
| 7 | Migration: hardcoded WA endpoints → DIRECT trust grants | (6) | CIRISAgent |

Steps 2-5 are all node-core work and can land in successive commits
once (1) ships in persist. Steps 6-7 are cross-repo + need CIRISAgent
team coordination.

---

## 12. References

- CIRISNodeCore#1 — adapter swap (PyO3 surface this FSD extends)
- CIRISNodeCore#2 — the umbrella issue this FSD details
- CIRISPersist#47 — federation_keys schema for trust columns
- CIRISAgent#760 — Accord §RC consent_role primitive (trust_type origin)
- `MISSION.md` — eleven primitives (Identity, Vote, Moderation,
  Witness-Diversity, Reconsideration, Truth-Grounding, Contribution,
  Expertise — all consumed by the design)
- `SCHEMA.md` §3.1 — adds `registry_vouch` to the contribution_type
  enum; §13.2 pending/canonical split applies unchanged.
