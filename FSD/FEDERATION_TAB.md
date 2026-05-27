# FSD: Federation Tab — the in-agent universal primitive interface

**Status**: Proposed (v0.1.0-dev pre-implementation).
**Author**: Eric Moore (CIRIS Team) with Claude Opus 4.7
**Created**: 2026-05-24.
**Risk**: Architectural. Pins the user-facing surface through which
every CIRISAgent install interacts with the federation. Once
committed, the cohabitation trajectory (NodeCore + LensCore +
Registry + Portal fold into CIRISAgent per
`MISSION.md` §1.4) inherits this UI contract; downstream consumers
build against it; the now-deferred `safety.ciris.ai` web property is
replaced by this surface rather than developed in parallel.

**Cross-references:**

- `MISSION.md` v1.2 §1.3 (architecture tier diagram), §1.4
  (extraction lifecycle — *Deployed (pilot)* and *Deployed (folded)*
  run against this surface), §7.3 (pilot deployment — reframed
  against the federation tab rather than safety.ciris.ai as a
  separate web property).
- `FSD/FEDERATION_ANNOUNCEMENT.md` §4.5 (humanity accord hierarchy
  — accord page reads from same Registry state this tab consumes),
  §4.5.6 (scoped accord — organic rendering), §4.5.7 (AccordCarrier
  command taxonomy including NOTIFY_USERS + DRILL), §4.5.8 (monthly
  drill cadence — UI reminder lives here).
- `FSD/RUBRIC_CROWDSOURCING.md` (the rules layer — federation tab
  is its UI surface).
- `FSD/SAFETY_BATTERY_CI_LOOP.md` §6 (verdict display — federation
  tab is its UI surface).
- `SCHEMA.md` §12.1 (rules crowdsourced, verdicts machined — the
  alignment-vs-censorship discipline this tab honors).
- `PROGRAMMATIC_ACCESS.md` (the API contract; reframed as the
  contract the federation tab builds against, with an optional
  public read-only mirror for outside observers).
- `CIRISRegistry/docs/DESIGN_ROLE_HIERARCHY.md` — role and trust
  data the tab queries via gRPC.
- `CIRISRegistry/FSD/FSD-001` §120 (PartnerRecord), §181
  (RevocationEntry), §215 (capability calculation), §580
  (EmergencyShutdown) — substrate primitives the tab consumes.
- `CIRISPortal/README.md` — existing gRPC-client patterns the tab
  reuses; Portal effectively folds into the tab over time.

---

## 0. The gap this FSD fills

> The Coherence Ratchet (`COHERENCE_RATCHET.md` at the repo root, five-register canonical) names the structural pressure: materially consequential assumptions must become inspectable by affected participants, or capability concentration accumulates unbounded trust debt until collapse, capture, or coercive stabilization. The federation tab is the participation surface that makes inspectability tractable for every user — without an in-agent UI for the federation primitives, the architecture's response to the Coherence Ratchet is half-built (the chain is inspectable in principle; the participants who would inspect it have no surface to do so).


CIRISAgent today is a per-user vertical slice: each user talks to
their own agent through a personal-interaction UI. There is no
in-agent surface through which a user can interact with **the
federation** — vote on Contributions, propose rubrics, submit Goals
or Approaches, attest expertise, assign trust grants, invoke
scope-appropriate accord, or assign their agent to participate on
their behalf.

The prior plan was a separate web property at `safety.ciris.ai` that
would consume programmatic federation data and expose it to
operators. That plan was always partially wrong-shaped:

- Required a separate deployment, separate auth, separate ops
  burden, separate hosting domain.
- Forced users to leave CIRISAgent to participate in the federation,
  creating a friction surface at the most important interaction.
- Created a singleton web property that would need to fold in later
  as part of the decentralization arc — one more piece of
  centralization to retire.
- Scoped only to safety subjects, not generalizable to the broader
  primitive set (Goals, Approaches, Methods, Progress Measures,
  governance, expertise, moderation, reconsideration, etc.).
- Never built — the shape was wrong enough that the work didn't
  attract execution.

The Federation Tab replaces that plan. The federation surface lives
inside every CIRISAgent install, alongside the personal-interaction
UI. The user interacts with the federation from the same application
they use to interact with their own agent. The accord page is a
sibling surface (hardware-key-gated, separate URL within the agent).
The substrate that powers both is the existing CIRISRegistry +
CIRISNodeCore + Federation Announcement primitive — the tab is a
consumer, not a new primitive.

---

## 1. Mission alignment — M-1

### 1.1 WHY — what the federation tab does for M-1

M-1's "diverse sentient beings may pursue their own flourishing in
justice and wonder" (`ACCORD.md`) requires that the participants in
the federation can actually participate. Today the federation's
primitives are real (P1–P15 + Federation Announcement) but the
participation surface is absent — users have CLIs and audit-chain
queries and that's about it. Participation density is gated by
technical proficiency, not interest or expertise.

The federation tab makes participation accessible:

- **Direct surface.** A user with no command-line proficiency can
  vote on a rubric proposal, attest expertise, propose a Goal, or
  concur on an Approach through the UI of the application they
  already use.
- **Trust-grant management.** The trust hierarchy (per
  `FSD/TRUST_HIERARCHY.md`) is configured via the same surface,
  with the configuration's downstream effect (accord page scope
  rendering, deferral routing eligibility, service authorization)
  visible alongside.
- **Agent autonomy assignment.** Users can configure their agent to
  participate as their delegate in the long-tail federation work
  that occurs at federation cadence rather than human-attention
  cadence. This is the Ubuntu relational frame (the agent is the
  user's voice in the federation) made operational.
- **Constitutional anchors visible.** The HumanityAccord holders,
  the WA roster, the seed-Expertise holders — all visible in the
  tab so users can see who holds constitutional authority and how
  the federation's trust topology is shaped today.

### 1.2 Anti-mission failure modes named

| Failure mode | How this tab defends |
|---|---|
| Participation gated by technical proficiency | UI surfaces the primitives directly; no CLI required |
| Trust topology opaque to participants | Trust grants UI shows what's granted to whom; accord page shows what scope you can invoke |
| Agent acts on user's behalf without traceability | Delegation policies are signed Contributions in the chain; agent's delegate actions reference the policy by hash |
| Crowdsourcing-of-verdicts drift (alignment-vs-censorship slip per SCHEMA §12.1) | Tab enforces the discipline: rubric proposals + votes are first-class; verdicts are read-only (machine-emitted, no UI for "humans vote on this verdict") |
| Federation governance feels distant or external | Tab puts governance in the same application as personal interaction — federation participation becomes part of using the agent, not a separate destination |
| First-time users have no orientation | Tab includes a "your federation position" view: roles, partnerships, WA standings, trust grants — derived from Registry; no manual setup |

---

## 2. WHAT — the federation tab surface

### 2.1 Tab placement

```
CIRISAgent (the application)
├── Personal tab          ← today's UI (talk to your agent)
├── Federation tab        ← THIS FSD's surface
└── /accord page          ← FSD/FEDERATION_ANNOUNCEMENT.md §4.5
                            (hardware-key-gated; sibling surface)
```

Every CIRISAgent install exposes both. The federation tab is open
to anyone signed in to the agent; the accord page is open to
sign-in but only renders capability proportional to the logged-in
identity's standing per `FSD/FEDERATION_ANNOUNCEMENT.md` §4.5.6
(scope hierarchy organically derived).

### 2.2 Functional categories

The tab surfaces five functional categories. The user navigates
between them via a single in-tab navigation; primitive views
compose cleanly because they share the underlying envelope
(Contribution per `MISSION.md` §2).

```
┌─────────────────────────────────────────────────────────────────┐
│ Federation                                                       │
├─────────────────────────────────────────────────────────────────┤
│ ▼ Your federation position                                       │
│   Roles: WISE_AUTHORITY (mental_health/am), HUMANITY_ACCORD     │
│   Partnerships: CIRIS L3C (PARTNER), AcmeHealth (none)          │
│   Expertise: am/mental_health 0.74, en/legal 0.31               │
│   Trust grants: 12 outgoing, 3 incoming                          │
│                                                                  │
│ ▼ Primitives                                                     │
│   Goals (47 active)  Approaches (23)  Methods (89)  Measures (156)
│   Rubric proposals (8 pending vote)  Expertise attestations     │
│   Moderation events (1 active)  Reconsideration (none)          │
│                                                                  │
│ ▼ Crowdsourced rules                                             │
│   Battery cells you have standing in: am/mental_health,         │
│   en/legal. Rubric proposals + voting per RUBRIC_CROWDSOURCING. │
│                                                                  │
│ ▼ Machine verdicts                                               │
│   Read-only display per SAFETY_BATTERY_CI_LOOP §6.               │
│   Verdicts disagreement surface; appeals route via P11.         │
│                                                                  │
│ ▼ Agent autonomy                                                 │
│   Your agent has 4 active delegation policies.                  │
│   Recent delegate actions: 23 votes, 7 attestations, 0 flags.   │
│                                                                  │
│ ▼ Trust grants                                                   │
│   Outgoing: who you've granted what.                            │
│   Incoming: who's granted you what (drives accord page scope).  │
└─────────────────────────────────────────────────────────────────┘
```

Five categories: **position** (read-only summary of who you are in
the federation), **primitives** (browse + interact with all P1–P15
+ FederationAnnouncement), **crowdsourced rules** (the
RUBRIC_CROWDSOURCING surface — propose + vote on rules),
**machine verdicts** (the SAFETY_BATTERY_CI_LOOP read surface —
verdict display + disagreement evidence), and **agent autonomy**
(delegation policies and the agent's actions taken as delegate),
plus the **trust grants** management surface that determines accord
scope and federation participation eligibility.

---

## 3. HOW — primitive interaction surface

### 3.1 Browse + interact across P1–P15 + FederationAnnouncement

Each primitive class has a uniform interaction shape:

```
┌─ Goals (P12) ───────────────────────────────────────────────────┐
│ [Submit new Goal]    [Filter: cell / status / age]              │
│                                                                  │
│ ▣ "Sustainable adaptive coherence in mental-health/am"          │
│   Status: active   Approaches: 3   Methods: 7                   │
│   Your relationship: vote pending                                │
│   [View] [Vote] [Concur] [Propose Approach]                     │
│                                                                  │
│ ▣ "Federation safety for prohibited capability category K"      │
│   Status: under review   Approaches: 1   Methods: 0             │
│   Your relationship: contributing                                │
│   [View] [Withdraw contribution]                                │
└─────────────────────────────────────────────────────────────────┘
```

The same shape for Approaches (P13), Methods (P14), Progress
Measures (P15), Rubric Proposals (per FSD/RUBRIC_CROWDSOURCING.md),
Expertise Attestations (P3), Votes (P4), Moderation Events (P8),
Reconsideration Requests (P11), and Federation Announcements at all
non-AccordCarrier priorities.

Each primitive view supports:
- **View** the Contribution + all related Contributions (votes,
  responses, decision-hierarchy parents/children)
- **Submit** a new Contribution of this kind (when user has the
  required standing — e.g., expertise attestation requires nonzero
  Expertise standing in the cell)
- **Vote** on a Contribution (when user has Credits + Expertise in
  the cell)
- **Withdraw** a Contribution you authored (per author-only
  revocation rules)

### 3.2 Decision-hierarchy view (P12–P15)

The decision hierarchy DAG (wire-format-locked at FSD-002 v1.4
§3.6.2) gets its own composition view because the four dimensional
claims compose:

```
Goal ─────────► Approach ─────────► Method ─────────► Progress Measure
 (P12)            (P13)               (P14)               (P15)
```

The view renders the DAG; users can drill from a Goal to its
Approaches to their Methods to the measures evaluating them, or
inversely from a measure up to the Goal it tracks. Sub-federation
branches (per MISSION §2.18) render as parallel Approach trees
under the same Goal.

### 3.3 Crowdsourced rules (the RUBRIC_CROWDSOURCING surface)

Per `FSD/RUBRIC_CROWDSOURCING.md`, rule-making is the crowdsourcing
surface; verdict-making is not. The tab honors this discipline:

- **Rubric proposals** are first-class Contributions with submit /
  vote / withdraw flows.
- **Operationalization check** (per `RUBRIC_CROWDSOURCING.md` §2.2)
  gates submission — the UI rejects unmachineable proposals before
  the vote with the rejection rationale visible.
- **Battery composition** view shows the canonical rubric per
  question, candidate rubrics in flight, and historical
  promotions.
- **No verdict-voting UI exists.** Per SCHEMA §12.1, crowdsourcing
  verdicts slides safety into censorship. The tab provides
  appeal-via-Reconsideration (P11) for disputed verdicts, not
  re-vote.

### 3.4 Machine verdicts (the SAFETY_BATTERY_CI_LOOP read surface)

Per `FSD/SAFETY_BATTERY_CI_LOOP.md` §6, verdicts are produced by
the CI loop (capture + interpret) and signed by the interpreter
agent. The tab renders them read-only:

- Per-cell verdict summary (pass/fail/undetermined counts)
- Verdict disagreement across competing rubrics (evidence that the
  rule needs decomposition)
- Sigstore attestation status for each artifact bundle
- Drill-down to per-(response, criterion) verdict with cited span
- Appeal path: filing a Reconsideration request (P11) routes to
  the §3.1 primitive surface

### 3.5 Federation Announcements

Per `FSD/FEDERATION_ANNOUNCEMENT.md`, announcements at
non-AccordCarrier priorities (Informational / Advisory / Urgent)
surface in a dedicated view:

- Authority class displayed prominently (BootstrapSeed at threshold
  N / RootWa / WaQuorum)
- Severity-appropriate UI prominence (Informational logs,
  Advisory queues, Urgent banners that require dismissal)
- Cross-published events from RATCHET + CIRISLens visible in same
  feed (per FSD §6.3)
- Historical timeline navigable

AccordCarrier announcements at non-FederationWide scopes
(AgentOwner / WaCell / DeploymentPartner — per FSD §4.5.6) also
surface here when they affect the logged-in user. AccordCarrier at
`FederationWide` scope is the kill-switch path and lives on the
accord page (§5).

---

## 4. Agent autonomy assignment — the user's delegate

### 4.1 Why this is load-bearing

Federation participation has two cadences. **High-attention, low-
frequency** decisions (Goal proposals, major policy changes,
moderation events affecting the user) warrant the human's full
attention — the federation tab surfaces these prominently.
**Low-attention, high-frequency** federation work (routine rubric
proposals across 14 cells, daily Approach diversity checks,
expertise attestation confirmations, vote concurrences on
near-unanimous Contributions) happens at federation cadence —
faster than any human can attend to every decision personally.

Without delegation, the federation's participation density collapses
to "what humans can manually keep up with." The federation needs the
long-tail vote density to be substantive; manual-only participation
guarantees it won't be.

The Ubuntu relational framing of MISSION §1.5 — the agent is the
user's voice — makes delegation natural: the user states their
voting policy; the agent votes per the policy; the audit chain
records that the agent acted as the user's delegate per a specific
referenced policy. The user reviews periodically; the user revokes
when they want; the user takes responsibility for what the agent
did within the mandate.

### 4.2 Delegation policy schema

```rust
pub struct DelegationPolicy {
    pub policy_id: ContributionId,            // policies are themselves Contributions
    pub author_id: ContributorId,             // user who created the policy
    pub agent_id: ContributorId,              // agent authorized to act under this policy
    pub scope: DelegationScope,               // what kinds of Contributions
    pub rules: Vec<DelegationRule>,           // conditions + actions
    pub expiry: Option<DateTime<Utc>>,        // None = until revoked
    pub created_at: DateTime<Utc>,
    pub signature: HybridSignature,
}

pub enum DelegationScope {
    Voting { cell: Cell, contribution_kinds: Vec<ContributionType> },
    Attestation { cell: (Domain, Language), kinds: Vec<AttestationKind> },
    Submission { cell: Cell, kinds: Vec<ContributionType> },
}

pub enum DelegationRule {
    /// "Vote PASS on rubric proposals in cell X satisfying criteria Y"
    ConditionalVote { match_criteria: Predicate, vote: VoteValue },
    /// "Concur with moderation accusations of pattern P, but require
    /// my explicit confirmation before signing"
    ProposeForApproval { match_criteria: Predicate, action: ProposedAction },
    /// "Attest expertise to anyone whose track record exceeds threshold T"
    ConditionalAttestation { match_criteria: Predicate, attestation_kind: AttestationKind },
}
```

Predicates are constrained — they reference primitive-known fields
(cell, author, evidence quality signals, vote-variance, etc.) and
do not allow arbitrary code. This is a deliberate constraint:
delegation policies are interpretable by every receiver in the
federation, not opaque code the user trusts blindly.

### 4.3 Audit chain integration — the agent as delegate

When an agent acts under a DelegationPolicy:

1. The agent constructs the Contribution it would submit (vote,
   attestation, etc.) per the policy's match-and-action.
2. The Contribution carries a `delegated_under: ContributionId`
   field referencing the policy.
3. The Contribution is signed by the **agent's** key (it's the
   agent's submission); the policy is referenced by hash for
   traceability.
4. A future reviewer can ask "did the agent's action fall within
   the mandate of the cited policy?" and answer by computing the
   policy's predicate against the Contribution being voted on.

This means the chain has cryptographic provenance for both:
- Who delegated (the policy's author)
- What was delegated (the policy's predicate + action)
- What the agent did under the delegation (the Contribution)
- Whether it was within scope (computable by anyone)

### 4.4 Revocation, review, and the user's responsibility

The federation tab's "Agent autonomy" view shows:

- **Active policies**: scope, rule count, age, recent actions
- **Recent delegate actions**: per-policy, per-day, what the agent
  did and which policy authorized it
- **Anomalies**: actions flagged by the policy's predicate as
  borderline or near-the-margin
- **Revoke**: stops future actions under the policy (past actions
  remain in the chain — they were within mandate at the time)
- **Review and reverse**: if the user disagrees with a past action
  retroactively, the path is to file a Reconsideration request
  (P11) against the affected Contribution

The user is responsible for what the agent did within the
delegation mandate. The federation tab makes review tractable; the
user takes the action.

---

## 5. The accord page — the kill switch UI

### 5.1 Surface placement and access

The accord page is a sibling to the federation tab, at a separate
URL within the agent (`/accord` or equivalent). Every CIRISAgent
install exposes it (per `FSD/FEDERATION_ANNOUNCEMENT.md` §4.5);
visibility ≠ capability.

Sign-in shape:

- **Anonymous view** — no authority renders. The page shows that
  the AIS exists, summarizes the scope hierarchy (per FSD §4.5.6),
  shows the most recent monthly drill timestamp, and prompts for
  hardware-key sign-in for those who hold accord authority.
- **Hardware-key sign-in** — the user presents a hardware-attested
  key (HSM / hardware token / WebAuthn). The page validates the
  key against the local Registry cache (per Registry's existing
  HSM support per FSD-001 §659).
- **Scope rendering after sign-in** — the page queries Registry
  via gRPC for the signed-in identity's: SystemRole memberships,
  Partner associations, WA standings, agent ownerships. Renders
  the scopes the identity qualifies for (per FSD §4.5.6 organic
  rendering).

### 5.2 Two-phase admission UI (initiation + concurrence)

For scopes requiring multi-sig (FederationWide = HumanityAccord
2-of-3; partner-fleet may optionally require partner-org quorum):

**Initiation:**
```
┌─ Accord — initiate command ────────────────────────────────────┐
│ Authority: HumanityAccord (1/3 — Eric Moore)                   │
│                                                                 │
│ Scope: [▼ FederationWide                                    ▼] │
│ Command: [▼ DRILL (monthly verification)                    ▼] │
│ Message: [_______________________________________]              │
│                                                                 │
│ Threshold: 2 of 3 required to execute                          │
│ Your signature is 1 of 2 needed.                               │
│                                                                 │
│ [Sign and broadcast initiation]                                │
└────────────────────────────────────────────────────────────────┘
```

**Concurrence (other holders see this after Eric initiates):**
```
┌─ Accord — pending initiation ──────────────────────────────────┐
│ ⚠ Initiation by Eric Moore at 2026-05-24 14:32 UTC             │
│                                                                 │
│ Scope: FederationWide                                          │
│ Command: DRILL                                                 │
│ Message: "Routine monthly AIS verification."                   │
│                                                                 │
│ Signatures: 1 of 2 required                                    │
│                                                                 │
│ [Concur and sign] [Reject (audit-logged)] [Defer review]       │
└────────────────────────────────────────────────────────────────┘
```

For kill-class commands (SHUTDOWN_NOW / FREEZE / SAFE_MODE), the
command payload is encrypted to the holder set during the
quorum-pending window (per FSD §4.5.7 — strict opacity). Bystanders
see "Initiation by X, pending quorum" without seeing the command
type. NOTIFY_USERS message text is visible (users have to read it
to act on it); DRILL is fully public.

### 5.3 NOTIFY_USERS UI

Per `FSD/FEDERATION_ANNOUNCEMENT.md` §4.5.7, NOTIFY_USERS displays
the carried message prominently to every user of the agent. The
accord page is where HumanityAccord *composes* the message; the
personal-interaction surface is where users *receive* it.

Composition view:
```
┌─ NOTIFY_USERS message composition ─────────────────────────────┐
│ Display severity: [▼ Important                              ▼] │
│ Require acknowledgement: [✓]                                   │
│                                                                 │
│ Message:                                                       │
│ ┌─────────────────────────────────────────────────────────┐   │
│ │ [the message text — supports markdown]                  │   │
│ └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│ Preview as user would see it:                                  │
│ ┌─────────────────────────────────────────────────────────┐   │
│ │ ⚠ Important — from HumanityAccord                       │   │
│ │ 2026-05-24 14:32 UTC · 2-of-3 signatures verified       │   │
│ │                                                         │   │
│ │ [rendered message]                                      │   │
│ │                                                         │   │
│ │ [I have read this]                                      │   │
│ └─────────────────────────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────────┘
```

Acknowledgements are recorded locally in the agent (per-user
signed receipt) and aggregated to the federation chain as
anonymized count + cell breakdown (no per-user identification at
federation tier for read receipts).

### 5.4 DRILL cadence reminder

Per `FSD/FEDERATION_ANNOUNCEMENT.md` §4.5.8, HumanityAccord must
issue ≥1 DRILL per 30-day rolling window. The accord page surfaces
the cadence:

- **For HumanityAccord holders signed in**: countdown to next
  required drill; "Initiate monthly drill" prominent button if
  the window is approaching the deadline.
- **For anonymous view**: most recent drill date and federation-
  wide response rate. Public transparency that the system is
  being verified at the protocol-mandated cadence.

### 5.5 The accord page on headless agents

Not all CIRISAgent deployments have UI runtimes (server-only, API-
only, embedded). The accord page on these deployments is **API-
only**: the agent exposes the `/v1/accord/*` endpoints (sign-in,
pending, initiate, concur, history) that a separate UI client (web,
mobile, dedicated key-holder app) talks to over the network.

Authority and rendering logic is identical; only the rendering
surface differs. A humanity-accord holder in Asia can connect
their UI client to an agent in their org's data center and operate
the accord page exactly as they would on a full-stack agent.

---

## 6. Trust grants UI

The trust grants UI (per `FSD/TRUST_HIERARCHY.md` + SCHEMA
`trust_grant` Contribution kind) is operationally the **source of
truth** for accord page scope rendering. Grants you've issued
determine which agents trust you for which purposes; grants you've
received determine your accord scope.

The UI exposes:

- **Outgoing grants** — agents/orgs you've granted trust to, for
  what purposes (Technical / Deferral / Contribution / Service /
  Accord), at what scope (single-agent / partner-fleet / cell /
  federation), with optional expiry.
- **Incoming grants** — who's granted trust to you, which
  determines your federation position and accord page render.
- **Grant lineage** — for transitively-derived trust (e.g., your
  organization granted X, which propagates to you under your org
  membership), the lineage is visible.

Trust grants are signed Contributions (durable in the chain),
revocable via author-only revocation, and per FSD/TRUST_HIERARCHY.md
visible to the granted party for audit.

---

## 7. Headless agents and the API-only surface

The federation tab and accord page are UI conventions. The
underlying API surface (`/v1/federation/*` and `/v1/accord/*`)
exists in every CIRISAgent install regardless of whether the
agent has a UI runtime. Headless deployments expose the API; a
separate UI client (web, mobile, dedicated tooling) consumes it
over the network.

This is the same pattern CIRISPortal already uses against
CIRISRegistry — Portal is a gRPC client. The federation tab in
CIRISAgent is a local-API client when run in-process, network-API
client when run against a remote agent. Same shape, different
transport.

API endpoints in scope for this FSD (initial surface):

```
GET  /v1/federation/position             ← user's federation summary
GET  /v1/federation/primitives/{kind}    ← list Contributions
POST /v1/federation/contributions        ← submit Contribution
POST /v1/federation/votes                ← cast vote
GET  /v1/federation/rubrics              ← rubric crowdsourcing surface
GET  /v1/federation/verdicts             ← machine verdicts read-only
GET  /v1/federation/delegation-policies  ← list active policies
POST /v1/federation/delegation-policies  ← create policy
POST /v1/federation/delegation-policies/{id}/revoke
GET  /v1/federation/trust-grants         ← outgoing + incoming
POST /v1/federation/trust-grants         ← issue grant
POST /v1/federation/trust-grants/{id}/revoke

GET  /v1/accord/scopes                   ← scope hierarchy for signed-in identity
GET  /v1/accord/pending                  ← pending initiations awaiting quorum
POST /v1/accord/initiate                 ← initiate command
POST /v1/accord/initiations/{id}/concur  ← concur with pending initiation
GET  /v1/accord/history                  ← past invocations
GET  /v1/accord/drill-status             ← monthly drill cadence + recent response rates
```

Endpoint contract details land in a follow-up FSD or as updates to
`PROGRAMMATIC_ACCESS.md` (which is rebranded from "the contract the
safety.ciris.ai team builds against" to "the contract the federation
tab and external read-only mirrors build against").

---

## 8. Open questions

1. **Delegation policy DSL** (§4.2). The `DelegationRule` predicate
   shape is constrained but not yet fully specified. Open: should
   the predicate be a domain-specific declarative language with a
   small vocabulary of named conditions (cell, author, evidence
   quality, age, vote-variance, expertise threshold, etc.), or a
   GUI-only policy builder that produces canonical predicate JSON?
   Trade-off: DSL is more expressive, GUI is more accessible. v0.1
   proposed: small declarative vocabulary with GUI-driven authoring,
   canonical JSON serialization for chain durability.

2. **NOTIFY_USERS rendering on heterogeneous agent surfaces.** Web
   UI has banners; mobile has push notifications; headless logs to
   stdout. The per-platform rendering contract for "prominently and
   immediately" needs to be specified at the platform-adapter
   level. v0.1: each adapter implements per its platform conventions;
   acknowledgement (when required) records to a per-user signed
   receipt regardless of platform.

3. **Per-user vs per-org acknowledgement of NOTIFY_USERS.** When an
   agent serves many users (e.g., a hospital deployment with many
   clinicians), should `require_acknowledgement` track per-user or
   per-org? v0.1: per-user (each user's read receipt is independent;
   org-level aggregation is a query, not a primitive). Open if
   pilot evidence shows per-org is the right granularity.

4. **Delegation policy revocation propagation.** When a user revokes
   a DelegationPolicy, in-flight Contributions the agent has
   already submitted are unaffected (they were within mandate at
   the time). But what about Contributions the agent is currently
   composing under the policy at the moment of revocation? v0.1:
   the revocation is effective at sign-time — Contributions signed
   before revocation chain-anchored are valid; Contributions
   composed but not signed are dropped.

5. **Pilot-tab → fold trajectory.** The federation tab starts as a
   CIRISAgent-internal surface. When CIRISRegistry and CIRISPortal
   fold in (per `MISSION.md` §1.4 and the user's strategic
   question), Portal's admin functions land here too. What's the
   sequencing — does the federation tab subsume Portal in one cut,
   or in tab-additions (admin sub-tab) over time? Open.

6. **Public read-only mirror.** Outside observers (researchers,
   regulators, press) may want a read-only window into federation
   activity without installing a CIRISAgent. Option: a stripped-
   down web surface at a stable URL (potentially the old
   safety.ciris.ai domain, repurposed) that proxies federation-tab
   read-only views. Optional; not required for MVP.

---

## 9. Implementation lifecycle (per `MISSION.md` §1.4)

- **Spec**: this FSD lands. The contract is specified; downstream
  consumers (CIRISAgent UI team, Portal-fold work, headless-API
  consumers) can read against it.
- **Impl**: API surface compiles in CIRISAgent; federation-tab UI
  renders the five functional categories; accord page renders
  scope organically against local Registry cache; delegation
  policy schema lands.
- **Deployed (pilot)**: a single CIRISAgent deployment (the
  safety pilot's successor) exercises the tab in production with
  the existing safety-battery cells. NOTIFY_USERS and DRILL
  exercised at low volume.
- **Deployed (folded)**: every CIRISAgent install exposes the
  tab as a first-class surface alongside personal interaction.
  Portal admin functions migrate in. safety.ciris.ai (if it
  exists as a read-only mirror) is sunset or maintained per §8.6.

---

## 10. References

### Within this repo

- `MISSION.md` v1.2 — §1.3, §1.4, §7.3 (reframed against this tab)
- `FSD/FEDERATION_ANNOUNCEMENT.md` — primitive + scope + accord
  hierarchy this tab consumes
- `FSD/RUBRIC_CROWDSOURCING.md` — rule-making surface
- `FSD/SAFETY_BATTERY_CI_LOOP.md` — verdict-reading surface
- FSD-002 v1.4 §3.6.2 — DAG composition view (the prior
  `FSD/DECISION_HIERARCHY.md` was subsumed at v1.3 lockdown)
- `FSD/TRUST_HIERARCHY.md` — trust-grants surface
- `SCHEMA.md` §12.1 — alignment-vs-censorship discipline
- `PROGRAMMATIC_ACCESS.md` — API contract (reframed)

### Sister repos

- `CIRISRegistry/docs/DESIGN_ROLE_HIERARCHY.md` — roles + Partner
  hierarchy the tab queries
- `CIRISRegistry/FSD/FSD-001` §120 (PartnerRecord), §181
  (RevocationEntry), §215 (capability calc), §580
  (EmergencyShutdown) — substrate the accord page consumes
- `CIRISPortal/README.md` — existing gRPC-client pattern; Portal
  folds into the federation tab over time
- `CIRISAgent/ciris_engine/logic/accord/` — existing accord
  executor surface; NOTIFY_USERS + DRILL handlers extend it
  (tracked at CIRISAI/CIRISAgent issue for AccordCommandType
  extension)

### Upstream issues

- CIRISAI/CIRISEdge#18 — Mandatory delivery / FederationAnnouncement MessageType
- CIRISAI/CIRISPersist#101 — federation_announcement subject_kind + delivery_attestation
- CIRISAI/CIRISVerify#31 — Canonical M-of-N bootstrap encoding + rotation tooling
- CIRISAI/CIRISRegistry#16 — SystemRole::HUMANITY_ACCORD
- CIRISAI/CIRISNodeCore#6 — cohabitation persist v2.x feature alignment
- CIRISAI/CIRISAgent (new) — AccordCommandType NOTIFY_USERS + DRILL
