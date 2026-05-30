# FSD: Media Sharing — Multimedia, Cinema, Content Discipline, Takedowns

**Status:** v0.1 design.
**Goal:** Extend `external_content` with multimedia (image / audio /
video / film / 3D) sub_kinds; codify the per-cohort content
discipline that makes CEWP a TikTok/YouTube replacement *where the
user controls the algorithm*; specify takedown notices as CEG-native
wire artifacts; ground the design in international standards (DMCA /
DSA / OSA / AVMSD / TVEC / NCMEC / Christchurch / EU AI Act) without
breaking the 1+4 CEG wire-format lockdown.
**Companion docs:**
* [CEWP.md](CEWP.md) — platform identity + premise/bet
* [FEDERATION_SCALING_MODEL.md](FEDERATION_SCALING_MODEL.md) — substrate scaling
* [ANONYMOUS_TIER.md](ANONYMOUS_TIER.md) — v2 deniability path
* [SCHEMA.md §4.29](../SCHEMA.md) — existing external_content sub_kinds
* [CEG 0.2 (CIRISRegistry)](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CEG)
  — wire-format authority

---

## 0. Premise

> "We are replacing TikTok / YouTube with an option where you control
> the algorithm."

The algorithm IS the trust × popularity × freshness math already in
the substrate. The user controls it by setting trust depth, cohort
filter, quality threshold, and freshness decay — every parameter
backed by a CEG-native wire artifact, every recommendation traceable
to (this content + this trust path + this quality signal).

This FSD extends that machinery to **multimedia at any scale**,
codifies the **per-cohort content discipline** that prevents the
substrate from becoming a BitChute-class cesspool while preserving
the **CEG locality dividend** (private speech stays private), and
specifies **takedown notices** as CEG-native wire artifacts so the
substrate is internationally compliant without inventing a separate
moderation tower.

The thesis: **the substrate refuses to amplify, but doesn't surveil.**
A defensible Signal-like line at the federation propagation event,
not at the user's device.

## 1. The content discipline

The discipline is structural at the wire-format level — refusal to
carry is enforced at the federation propagation gate (where
`holds_bytes` would otherwise be emitted), not by per-instance
volunteer admins. This is the central difference from Mastodon /
PeerTube prior art, which the [Stanford 2023 fediverse CSAM study](https://stacks.stanford.edu/file/druid:vb515nd6874/20230724-fediverse-csam-report.pdf)
and the [IFTAS 2024 Needs Assessment](https://about.iftas.org/2024/12/17/the-2024-iftas-needs-assessment-report-is-here/)
empirically showed cannot scale with volunteer per-instance
moderation alone.

### 1.1 The per-cohort matrix

| `cohort_scope` | Default content rule | Mechanism |
|---|---|---|
| `self` | Anything. Substrate doesn't see, doesn't carry, doesn't help distribute. | CEG locality dividend (§1.3 scaling model). No `holds_bytes` emission. Same defensible position as [Signal](https://signal.org/bigbrother/) on E2EE messaging — two parties' private exchange is their concern; the substrate isn't their accomplice because it isn't their distribution channel. |
| `family` | Anything. Same as `self`. | Same — no inter-host federation; family-trust-path only. |
| `community` (general) | **PG-13 + community-decency**. Refused at intake if `content_rating > PG-13` (any declared scheme) or if no rating declared on multimedia content. | Trust × capacity intake gate, with content_rating consulted. |
| `community` (CW) | **The CW community's declared content classification**. Membership = consent. Warning is wire-format-mandatory. | A community-creation attestation declares `cw_class:{horror,art_cinema,erotic,political,...}` ; joining the community requires explicit `delegates_to:cw_community` attestation acknowledging the warning. |
| `affiliations` | Same as `community` (general). | Federation-publishable but PG-13 default. |
| `species` / `planet` / `federation` | **PG-13 + R/X allowed for declared cinema/art with consumer opt-in; R/X via news editorial framing otherwise.** | `content_class ∈ {film, short_film, documentary, art_piece, theatre, performance}` + declared `content_rating` admits R/X at federation scope, conditional on consumer trust depth + quality threshold + age-gate clearance. |

### 1.2 What this enables (and what it deliberately doesn't)

**Enabled:**

* Tarantino's Pulp Fiction (R, cinema, federation-publishable to
  consenting adults with consumer opt-in) circulates as
  `external_content` `sub_kind: film` `content_rating:mpaa:R`
* A horror short by a small filmmaker carries
  `content_rating:bbfc:18` and circulates via a CW horror community
  where members consented to graphic content
* PornHub-as-trusted-publisher: you emit `delegates_to:pornhub` from
  your federation key; their content reaches your personal feed via
  the trust-graph admission gate; never enters general community/
  global feeds
* OnlyFans creator-as-trusted-publisher: same mechanism, finer
  grain — you trust specific creators individually
* A current-events news segment that includes adult-relevance
  material rides `sub_kind: news_article` with content warnings;
  federation-publishable with consumer opt-in
* Two people sharing private content at `self`/`family` scope:
  substrate doesn't see, doesn't carry, doesn't help. Same
  cryptographic-deniability position [Signal](https://signal.org/bigbrother/)
  takes; same threat model the [Apple NeuralHash failure](https://arxiv.org/abs/2111.06628)
  showed cannot be solved by client-side scanning without breaking
  the rest

**Deliberately not enabled at federation scope:**

* Adult content without cinematic/artistic context that doesn't
  ride the trusted-publisher path
* "Free speech" pure-amplification feeds — the [BitChute trajectory](https://dl.acm.org/doi/10.1145/3710950)
  the prior-art research shows happens when "we don't moderate" is
  the differentiator
* Engagement-optimized algorithmic feed promoting whatever
  maximizes time-on-platform — the YouTube/TikTok pathology being
  replaced

### 1.2.1 The trusted-publisher path — adult content, in detail

The cohort-scope matrix in §1.1 carries "trusted-publisher path" as
the route by which content the federation refuses to amplify at
community/global scope nonetheless reaches subscribers who've opted
in. This is the same mechanism Netflix-class streaming uses (§4 of
[the scaling model](FEDERATION_SCALING_MODEL.md), `netflix_replacement`
scenario), and **the same mechanism a CEWP-native adult content
publisher (call it `AdultHUB`, or any deployment of OnlyFans / PornHub
shape) uses**. The substrate doesn't need a separate mechanism for
adult content; the publisher-trust path is the canonical way any
content provider with a subscriber model operates on CEWP.

**The mechanics, end-to-end:**

1. **The publisher operates an L1 (or hyperscale-equivalent) server**
   carrying their content catalog. Their content envelopes carry:
   * `author_id` = the publisher's federation key (verified-business
     identity; via `delegates_to:business_verifier:adulthub_inc` and
     equivalent for OnlyFans, PornHub, Netflix, a community film
     festival, etc.)
   * `content_rating` attestations per the established schemes
     (`content_rating:mpaa:NC-17`, `content_rating:bbfc:R18`, etc.)
   * `content_class:adult` (or `content_class:film` for cinema, etc.)
   * For UGC-class publishers (OnlyFans, PornHub): per-creator
     identity attestations via `delegates_to:creator:{creator_key_id}`
     where each creator went through Ondato/Persona-class
     identity-verification, mirroring the
     [OnlyFans Ondato pattern](https://ondato.com/case-studies/onlyfans-case-study/)
     and [PornHub's December 2020 verified-uploader purge](https://www.eff.org/deeplinks/2020/12/visa-and-mastercard-are-trying-dictate-what-you-can-watch-pornhub)
   * For large videos: `BlobBody::External(ExternalRef)` pointing to
     the publisher's own S3-class store; federation carries metadata
     + ACL only (§2.6)

2. **The subscriber opts in** by emitting `delegates_to:publisher:
   adulthub` from their federation key, where the attestation
   declares:
   * `age_assurance:provider:{verifier_key}:adult` is present on
     subscriber's key (operator-required for adult publisher trust;
     §4 of this FSD; UK OSA / French SREN compliance)
   * Subscriber consents to the publisher's content classification
     (the `cw_class:adult` warning is wire-format-mandatory; §3.4)

3. **The substrate routes content** via the standard trust × capacity
   intake gate. Publisher's content reaches subscribers whose trust
   graphs admit it; never propagates into community / global feeds
   because the cohort-scope-content-discipline (§1.1) refuses
   non-CW community content above PG-13. Subscribers' substrates
   admit it because the *trust path* qualifies, not because the
   *cohort scope* would otherwise admit it. This is the same
   bilateral mechanism Signal uses for sealed-sender messaging —
   content delivery is permissioned by the recipient's intake gate,
   not by federation-wide propagation.

4. **Per-creator eviction is native**: if a specific creator violates
   terms (NCII, copyright, takedown_notice from third party), the
   substrate evicts via `evict_actor(creator_key)` (CIRISPersist
   v3.5.0 #125). The §9 identity-aware-storage thesis is what makes
   this trivial — the publisher's substrate knows whose bytes it's
   holding; can drop one creator's entire content set as a single
   operation; doesn't have to discover-then-purge.

5. **Children never see it.** Children's federation keys lack the
   `age_assurance:provider:adult` attestation, so they cannot
   emit a `delegates_to:publisher:adulthub` (operator-config refuses
   to admit the delegation), so the publisher's content never
   reaches their substrate, so the substrate-level "I am not a
   child" gate is enforced by the trust-graph admission mechanism
   itself. No separate content-filtering layer required.

**What this means structurally:** the substrate is not a CSAM-
distribution platform because (a) `self`/`family`-scope content is
never federation-discoverable (§1.1 self/family row — the Signal
position), and (b) federation-discoverable content at community/
global scope is gated by content_rating + content_class + trust path,
none of which CSAM can pass. **Adult content with proper publisher
identity + per-creator verification + age-gated subscribers + content
rating + content class IS distributable on the substrate — without
any of the moderation pathologies BitChute / DTube / LBRY hit by
positioning as "free speech" amplification engines**.

The publisher operates inside the substrate's discipline. The
substrate's discipline is what makes the publisher's compliance
posture legible to consumers (subscribers know what they're getting
because the cryptographic provenance + attestation graph proves it).
This is OnlyFans-class identity-aware-publishing operationalized in
CEG-native terms.

**Operator-config knobs for adult publishers:**

* Required `age_assurance` level (self / provider / government)
* Required `delegates_to:business_verifier` chain (compliance
  certifications: 2257 records, KYC creator verification, etc.)
* Geographic restrictions (per-jurisdiction admission rules; e.g.,
  refuse subscribers whose `key_id` claims a jurisdiction where
  the content is illegal)
* Per-content-class CW requirements (members must explicitly opt
  into specific CW classes; granular consent)
* Takedown response SLA (DSA Art. 16 timeliness; substrate-mandatory
  for NCII / CSAM-hash-match / TVEC bases)

### 1.3 The "cinema is art" exception (and why it doesn't generalize to porn)

R/X-rated cinema circulates at federation scope because it carries
two things the substrate can verify:

1. **A declared `content_rating`** from an established scheme (MPAA /
   BBFC / PEGI / IFCO / etc.) — auditable claim about how the
   content has been classified
2. **A declared `content_class`** of art-bearing form (film,
   short_film, documentary, art_piece, theatre, performance) —
   auditable claim about *what kind of content* it is

The substrate doesn't adjudicate "is this art" — the federation's
attestation graph does. A film with a major distributor's
canonical attestation (Disney, A24, Criterion, Studio Ghibli,
international cinema houses, registered film festivals) is
unambiguously cinema. An "art piece" claim from a low-trust source
won't pass quality thresholds set by typical consumers, even if
nominally permitted at the substrate level.

This is why the discipline holds for "porn-disguised-as-art" attacks:
the trust graph + quality attestations adjudicate, not the
substrate. A consumer's algorithm config (trust depth + quality
threshold) refuses content whose art-claim doesn't have substantial
attestation backing. The substrate provides the wire-format surface
the attestation graph operates on; the federation does the work.

## 2. New external_content sub_kinds

Five additions to SCHEMA §4.29:

### 2.1 `image`

Photo, illustration, graphic, screenshot, meme, infographic.

**Source struct** (`crate::ingest::ImageSource`):
```
pub struct ImageSource {
    pub entity_key_id: String,
    pub body_bytes: Vec<u8>,
    pub body_media_type: String,        // image/jpeg, image/png, image/webp, image/avif, image/svg+xml
    pub cohort_scope: String,
    pub content_rating: Vec<ContentRating>,  // multi-scheme; see §3.1
    pub content_class: Option<String>,  // photograph / illustration / screenshot / meme / infographic / generated
    pub width_px: u32,
    pub height_px: u32,
    pub orientation: Option<String>,    // for camera-EXIF respect
    pub alt_text: String,               // accessibility; mandatory
    pub captured_at: Option<DateTime<Utc>>,
    pub creator_key_id: Option<String>, // if different from federation submitter
    pub generated_by: Option<String>,   // for AI-generated: model name per EU AI Act Art. 50 disclosure
    pub topical_relations: Vec<TopicalRelation>,
    pub citations: Vec<Citation>,
}
```

**Dimensions** the federation can attest on this content:
* `image:accuracy:{topic}` — factual accuracy when image makes claims
* `image:authenticity:provenance_chain` — chain-of-custody attestations (cf. C2PA)
* `image:authenticity:ai_generated` — explicit signal per EU AI Act Art. 50
* `image:content_warning:{class}` — gore / nudity / strobe / etc.
* `image:license:{class}` — Creative Commons / proprietary / public_domain

### 2.2 `audio`

Music, podcast episode, lecture, audiobook chapter, sound
sample, generative music piece.

**Source struct** carries: codec (opus / mp3 / flac / aac), duration,
sample rate, bit rate, transcript (mandatory for `cohort_scope ≥
community` per accessibility), `content_class ∈ {music, podcast,
lecture, audiobook, soundscape, generated}`, `creator_key_id`,
`license` info.

**Dimensions**:
* `audio:fidelity:{topic}` — audio quality attestations
* `audio:source_quality:{publisher}` — publisher reputation
* `audio:authenticity:ai_generated`
* `audio:accuracy:{topic}` — for podcasts / lectures
* `audio:content_warning:{class}` — explicit language / mature themes

### 2.3 `video`

General video. Short clips, social videos, talking heads, screen
recordings, vlogs, gameplay, tutorials.

**Source struct** carries: codec (av1 / h264 / h265 / vp9), duration,
resolution (width × height), frame rate, captions (transcript +
SRT/WebVTT mandatory for `cohort_scope ≥ community`),
`content_class`, `creator_key_id`, `license`, **`is_ai_generated:
bool` (per EU AI Act Art. 50 — mandatory disclosure)**, `thumbnail_sha256`
(separate blob).

**Dimensions** include all the image+audio dimension families plus:
* `video:source_quality:{publisher}` — publisher reputation
* `video:accuracy:{topic}` — factual accuracy
* `video:content_warning:{class}` — graphic content / flashing / etc.

### 2.4 `film` — cinematic / art-bearing video

Full-length cinema, short films, documentaries, theatre recordings,
performance art. Distinguished from general `video` by the
`content_class` claim + the trust-graph attestations that adjudicate
the art-bearing nature of the content.

**Source struct** carries everything `video` does plus:
* `content_class: String` (mandatory — film / short_film / documentary
  / art_piece / theatre / performance / animation / experimental)
* `production_credits: ProductionCredits` (director / writer /
  cinematographer / etc.)
* `distributor_key_id: Option<String>` (Disney / A24 / Criterion /
  Studio Ghibli / etc. — canonical-attester chain for the
  distributor's "this is legitimate film" claim)
* `imdb_id` / `tmdb_id` / equivalent external canonical IDs
  (optional but reduces trust-graph friction)
* `release_year`
* `runtime_seconds`
* `languages: Vec<String>` (audio + subtitle availability)

**Dimensions** (in addition to `video:*`):
* `film:critical_reception:{aggregator}` — Rotten Tomatoes / Metacritic / Letterboxd-equivalent
* `film:director_reputation:{director_key_id}`
* `film:distributor_reputation:{distributor_key_id}`
* `film:cultural_significance:{measure}` — preservation society
  attestations, archive nominations, etc.

### 2.5 `model_3d` — three-dimensional content

Static models, scenes, volumetric video, scanned environments,
character rigs.

**Source struct** carries: codec (`gltf` / `usdz` / `fbx` / `obj` /
`ply` / `gaussian_splat` / `nerf`), MIME type, vertex_count,
triangle_count (when applicable), bounding_box, animations
(yes/no), texture_resolution, license, content_class (static_object
/ scene / character / volumetric_capture / generated / etc.),
intended_renderer (web_gl / web_gpu / vr / ar / mobile / desktop).

**Dimensions**:
* `model_3d:fidelity:{measure}` — geometric / texture fidelity
* `model_3d:authenticity:provenance_chain` (cf. C2PA-3D)
* `model_3d:authenticity:ai_generated`
* `model_3d:rendering_class:{class}` — what hardware can handle it
* `model_3d:license:{class}`

3D content is treated identically to other multimedia under the
content discipline — declared `content_rating` + cohort_scope gate
both apply.

### 2.6 Inline vs external — NO new chunking primitive

The substrate's `BlobBody` enum (already shipped in CIRISPersist
`federation/blobs.rs`) has two variants — `Inline(Vec<u8>)` and
`External(ExternalRef)`. **The multimedia tier uses both; no new
wire primitive is introduced**, no chunking layer added, no streaming
manifest format invented at the substrate layer.

**Inline (`BlobBody::Inline`, bounded by edge's `MAX_BODY_BYTES`
= 16 MiB):**

| Content class | Typical size | Inline? |
|---|---|---|
| Photo (full-resolution smartphone, JPEG) | 2-5 MiB | ✓ |
| Image (illustration, infographic, screenshot) | < 1 MiB | ✓ |
| Audio clip (≤ 5 min at 96-128 kbps) | < 5 MiB | ✓ |
| Audio podcast episode (≤ 25 min at 96 kbps) | ≤ 15 MiB | ✓ (borderline) |
| Short-form video (TikTok-class, 15-60 sec @ 1080p H.264) | 3-15 MiB | ✓ |
| YouTube Shorts (60 sec @ 1080p H.264) | ~10-30 MiB | sometimes ✗ |
| Animated GIF / short clip (≤ 15 MiB) | 2-15 MiB | ✓ |
| 3D model (small static, ≤ 15 MiB) | varies | mostly ✓ |

The substrate carries inline content end-to-end: bytes ride
federation transport, get admitted at every recipient's intake gate,
contribute to each holder's `bytes_held` accounting, fan out via
demand-driven replication (§2.7).

**External (`BlobBody::External(ExternalRef)`, no size limit):**

| Content class | Typical size | External? |
|---|---|---|
| Audio podcast episode (1-3 hour at high bitrate) | 50-300 MiB | ✓ |
| Long-form video (YouTube general, 10-60 min) | 100 MiB - 2 GiB | ✓ |
| Films (90-180 min cinema) | 2-10 GiB | ✓ |
| Episodes (TV streaming class) | 1-5 GiB | ✓ |
| Volumetric video / scanned environments | 100 MiB - 10 GiB | ✓ |
| Large 3D scenes / character rigs | 50 MiB - 5 GiB | ✓ |

For external content, **the federation carries (SHA, holds_bytes
attestation, the ExternalRef pointing to the publisher's S3-class
store, content access control). The bytes themselves ride that
external store directly**, federation-mediated by trust + ACL but
not federation-stored. This is structurally how Netflix Open Connect
+ CloudFront work today — we just expose the discoverability +
trust-graph + ACL as CEG-native instead of as proprietary AWS APIs.

The S3-class store can be:
- The publisher's own infrastructure (Netflix runs Open Connect; a
  small film co-op runs MinIO on a home server)
- A community-shared object store (e.g., a film festival cooperatively
  operating storage for member submissions)
- An IPFS / Filecoin / Sia gateway exposing content via standard
  protocols (the substrate doesn't care; ExternalRef is just a URI)
- A CEWP-aware distributed object store (follow-up; not v1)

The substrate's "no datacenters required" claim still holds:
publishers' existing storage IS their substrate. CEWP doesn't acquire
a new infrastructure dependency; it composes with what publishers
already run.

### 2.7 Demand-driven replication — substrate's existing pattern, applied to multimedia

Replication for multimedia content follows the same CEG-organic
pattern as all other CEWP content
([FEDERATION_SCALING_MODEL.md §1.1](FEDERATION_SCALING_MODEL.md)).
**Nothing new is required for multimedia.**

**For inline content:**

```
T=0     Publisher emits the Contribution + put_blob_signing.
        Publisher's L1 is the sole `holds_bytes:sha256:X` advertiser.
T=ε     A user (a friend in the publisher's trust set, or a
        delegates_to:publisher subscriber) fetches via ContentFetch.
        Receiver's intake gate: trust(publisher) ≥ threshold AND
        capacity_available. On pass: bytes admitted to receiver's
        federation_blobs; receiver emits THEIR OWN
        holds_bytes:sha256:X. Receiver is now a holder.
T=...   Every subsequent successful fetch creates a new holder.
        list_holders(X) grows as demand spreads through trust graph
        layers; popular content fan-outs to many holders;
        unpopular content stays at the publisher.
T=...   Eviction sweeper (popularity × freshness) drops content
        from holders whose users have lost interest. Cold-metro
        copies evict; hot-metro copies are kept. The substrate
        produces CDN-shape topology without a CDN — it's just the
        intake/eviction discipline applied to demand.
```

**For external content:**

```
T=0     Publisher emits the Contribution + put_blob_signing where
        BlobBody::External points to the publisher's S3 / CDN.
        Publisher's L1 emits the holds_bytes attestation.
T=ε     A user fetches the metadata via ContentFetch (small;
        federation-native). The metadata includes the ExternalRef.
        User's edge fetches the bytes directly from the publisher's
        S3 (NOT through the substrate). No federation byte cost.
T=...   Optional: an L1 server CAN opt in to becoming a CDN edge —
        it full-fetches the external bytes, stores locally (now
        BlobBody::Inline if ≤ 16 MiB OR its own BlobBody::External
        pointing to its own S3), and emits its own holds_bytes
        attestation. Now there are two effective holders; other
        peers fetching can use either. This is the same
        demand-driven replication pattern applied to external
        content; it's opt-in by the L1 operator, gated by the
        operator's disk budget + the operator's policy.
```

**Implication: no preemptive replication.** The substrate doesn't
push content to L1s in advance. Replication is the side effect of
demand passing through the trust graph; popular content ends up
where demand is, cold content stays at origin. This is exactly the
CDN topology Netflix / YouTube / Cloudflare maintain via centralized
control, produced naturally from CEG primitives + trust topology +
the popularity × freshness eviction sweeper.

**Implication for the toy:** the only new model knob needed is the
fraction of fetch volume that's external_ref vs inline. External
fetch contributes to bandwidth but NOT to bytes_held; inline fetch
contributes to both. Storage gates are determined by inline content
(own + admitted-trust + hot cache); bandwidth gates have an external
multiplier (the publisher's own S3 capacity becomes the substrate's
effective storage tier for that content).

### 2.8 `live_stream` — real-time broadcast

A live broadcast is a streaming sub_kind that emits chunks as it
goes. Wire format: each chunk is a Contribution; the stream
manifest (mpeg-dash / hls / similar) is the parent Contribution
referencing chunks via `topical_relation:has_chunk`.

**Source struct** for the manifest: codec, anticipated_runtime,
chunks_per_minute, latency_class (low/standard/archival),
content_class (news / sports / event / gameplay / vlog /
performance), age_gate_requirement (operator config; §4), takedown
escalation contacts (mandatory for federation scope per EU DSA
Art. 16).

This is the lowest-priority sub_kind for v1 (substantially more
complexity for the live-streaming wire format). Spec'd here for
completeness; implementation Phase 2.

## 3. Content classification dimension family

Multi-scheme coexistence per CEG §1.10.1 mechanism-descriptive-name
gate.

### 3.1 The `content_rating:*` family

Same content can carry multiple rating attestations from different
schemes. Consumers' algorithms pick which scheme to honor.

| Scheme | Dimension prefix | Ratings |
|---|---|---|
| MPAA (US film) | `content_rating:mpaa:*` | G, PG, PG-13, R, NC-17, X |
| BBFC (UK film) | `content_rating:bbfc:*` | U, PG, 12, 12A, 15, 18, R18 |
| PEGI (EU games) | `content_rating:pegi:*` | 3, 7, 12, 16, 18 |
| ESRB (US games) | `content_rating:esrb:*` | E, E10+, T, M, AO |
| IFCO (Ireland) | `content_rating:ifco:*` | G, PG, 12A, 15A, 16, 18 |
| [Common Sense Media](https://www.commonsensemedia.org/about-us/our-mission/about-our-ratings) | `content_rating:csm:age_appropriate` | 2-18 (age) |
| Operator-defined community CW | `content_rating:cw:{community}:*` | per community policy |

The federation does not pick a canonical scheme. Each rating is a
`scores` attestation on the content; consumer algorithms select
which schemes to honor.

### 3.2 Mapping ratings to cohort_scope gate

The substrate enforces this default rule at federation scope
(`cohort_scope ≥ community` general):

```
admits_to_general_community(content) =
  (content_rating:mpaa <= PG-13 across all attestations) OR
  (content_rating:bbfc <= 12A across all attestations) OR
  (content_rating:pegi <= 12 across all attestations) OR
  (any scheme's equivalent <= PG-13)
```

For `cohort_scope = community (CW)`: the CW community's declared
class determines admission. A `cw_class:art_cinema` community
admits up to NC-17/18; a `cw_class:horror` community admits up to
the horror-tolerance threshold the community declares.

For `cohort_scope ≥ species`: `content_class ∈ {film, short_film,
documentary, art_piece, theatre, performance, animation,
experimental}` AND consumer opt-in lifts the PG-13 ceiling. News
framing remains a path for current-events.

### 3.3 The `content_class:*` family

Mechanism-descriptive declarations of *what kind of content this is*.
Mandatory for federation-scope multimedia. Multi-class allowed (a
documentary-art-piece can carry both).

| Class | Federation scope allowed (R/X)? |
|---|---|
| `film`, `short_film`, `documentary`, `art_piece`, `theatre`, `performance`, `animation`, `experimental` | Yes, with consumer opt-in |
| `educational`, `tutorial`, `lecture`, `talk` | Yes, content_rating gate applies |
| `news`, `current_events`, `journalism` | Yes (editorial-framing exception) |
| `entertainment`, `vlog`, `social_video`, `gameplay`, `commentary` | PG-13 gate strict |
| `adult` | No (publisher-trust-path only) |
| `generated` (AI-generated) | Always carries `is_ai_generated: true`; EU AI Act Art. 50 disclosure |

### 3.4 The `cw_class:*` family (CW community declarations)

A CW community declares its content class at creation; members
explicitly opt in via `delegates_to:cw_community:{community_id}`
attestation.

| CW class | What members opt into |
|---|---|
| `cw_class:art_cinema` | NC-17/18 cinema, art-bearing form |
| `cw_class:horror` | Graphic horror content |
| `cw_class:political` | Politically charged / partisan content |
| `cw_class:erotic` | Erotic content (still distinct from porn-as-utility) |
| `cw_class:violence` | Graphic violence |
| `cw_class:medical` | Medical content (surgical / autopsy / etc.) |
| `cw_class:nsfw_text` | Text-only NSFW (sexual themes in writing) |

Operators can define additional CW classes; established schemes
(MPAA / BBFC / PEGI / etc.) cover the canonical descriptors.

## 4. The age gate — operator-managed, standards-compliant

Per Eric: **operator-managed**, not federation-mandated. Each
deployment chooses its age-verification posture appropriate to its
jurisdiction and audience.

### 4.1 The wire-format surface

A user's `federation_keys` row can carry an `age_assurance:*`
attestation. Operators decide which level is acceptable for
admission to age-gated content.

| Dimension | What it attests | Standard alignment |
|---|---|---|
| `age_assurance:self:adult` | User self-declared age. Lowest assurance. | Not OSA-"highly effective"; acceptable for low-stakes deployments |
| `age_assurance:provider:{provider_key}:adult` | Third-party AV provider attests adult. Provider's reputation is in the trust graph. | UK OSA "highly effective" if provider meets [Ofcom criteria](https://uklitigation.cooley.com/uk-online-safety-act-age-assurance-and-childrens-access-statement/); French [SREN](https://www.twobirds.com/en/insights/2024/france/la-loi-sren-et-la-protection-des-mineurs) "double anonymity" if provider implements that protocol |
| `age_assurance:government:{credential_class}:adult` | Government-issued credential (eIDAS / national ID / passport) | OSA-"highly effective"; strongest assurance |

The substrate doesn't require any specific provider. Operators
config their deployment: required assurance level + accepted
providers + default-deny vs default-allow.

### 4.2 Provider integration pattern

Providers (Yoti / Persona / Onfido / Open Banking-rooted AV /
national identity systems) emit a `delegates_to:age_verifier:{level}`
attestation against a user's federation key. The user's key now
carries the attestation; the federation can see "this provider
attested this user is adult."

[French SREN's "double anonymity"](https://www.twobirds.com/en/insights/2024/france/la-loi-sren-et-la-protection-des-mineurs)
is satisfied because the provider attests against a federation key,
not against the user's real-world identity directly; the platform
sees only the attestation, not the underlying identity proof.

### 4.3 The platform "I am not a child" gate

User-facing gate is operator-implemented and consumes the
`age_assurance:*` attestation. UK / EU deployments default to
requiring `age_assurance:provider:...:adult`; US deployments
might default to `age_assurance:self:adult` with provider upgrade
required for specific content; private home deployments
(CIRISHome) can default to no gate within the family scope.

The substrate ships the WIRE FORMAT for the attestation. Each
deployment ships the GATE.

## 5. Takedown notices — CEG-native, internationally-aligned

The architectural foundation: a takedown notice is a **signed
Contribution by a claimant against a content SHA**. The
**`withdraws`-against-`holds_bytes`** structural primitive (already
in CEG 0.2's 1+4 lockdown) is the propagation mechanism. **No new
wire primitives needed.**

This composes cleanly with [Bluesky / AT Protocol's labeler
architecture](https://docs.bsky.app/blog/blueskys-moderation-architecture)
(detection / accusation / propagation / user-side-action as
composable signed artifacts) — the validated prior-art design.

### 5.1 The takedown notice envelope

A new `Contribution` `subject_kind: takedown_notice` (additive to
the §3.1 discriminator table) with payload:

```
{
  "content_sha256": "<hex>",                  // the content being challenged
  "content_holder_key_ids": ["..."],          // optional: specific holders to notify
  "claimant_key_id": "<key_id>",              // submitter's federation identity
  "legal_basis": "...",                       // see §5.2
  "jurisdiction": "...",                      // ISO 3166-1 country / region
  "good_faith_statement": "...",              // text per DMCA §512(c)(3) requirement
  "claim_text": "...",                        // human-readable description
  "evidence_refs": ["..."],                    // optional supporting attestations
  "perceptual_hash": "...",                    // for PhotoDNA/PDQ/hash-match takedowns
  "counter_notice_channel": "...",            // contact for DMCA §512(g) restoration
  "asserted_at": "<datetime>",
  "expires_at": "<datetime?>"                 // optional auto-expiry
}
```

### 5.2 Legal basis enumeration

The legal_basis field is mechanism-descriptive (per CEG §1.10.1)
and maps to the international standards landscape:

| Legal basis | Source | Action discipline |
|---|---|---|
| `dmca_512` | [US DMCA §512(c)(3)](https://www.copyright.gov/512/) | Notice → expeditious removal → 10-14 day counter-notice window → restoration if uncontested |
| `dsa_article_16` | [EU DSA Art. 16](https://www.cms-digitallaws.com/en/dsa/article-16/) | Notice + receipt confirmation "without undue delay" → timely / diligent / non-arbitrary decision → internal complaint mechanism (Art. 20) for restoration disputes |
| `tvec_terrorist` | [EU TVEC 2021/784](https://www.europarl.europa.eu/news/en/press-room/20210422IPR02621/new-rules-adopted-for-quick-and-smooth-removal-of-terrorist-content-online) | 1-hour removal from competent-authority order |
| `ncmec_csam` | [US NCMEC CyberTipline](https://www.missingkids.org/gethelpnow/cybertipline) | Mandatory reporting (US-based providers); emergency takedown, no counter-notice path |
| `gifct_cip` | [GIFCT Content Incident Protocol](https://gifct.org/2022/06/23/debrief-cip-activation-buffalo/) | Fast-path takedown during active incidents (e.g., Buffalo CIP) |
| `community_standards` | Per-cohort decency / CW community rules | Cohort-trust admission gate; eviction sweeper handles propagation |
| `perceptual_hash_csam` | [PhotoDNA](https://technologycoalition.org/resources/update-on-voluntary-detection-of-csam/) / [PDQ](https://github.com/facebook/ThreatExchange/tree/main/pdq) | Substrate-level hash match at federation propagation event |
| `osa_illegal_content` | [UK OSA 2023](https://uklitigation.cooley.com/uk-online-safety-act-age-assurance-and-childrens-access-statement/) | Illegal content removal per Ofcom timelines |
| `avmsd_age_inappropriate` | [EU AVMSD](https://digital-strategy.ec.europa.eu/en/policies/audiovisual-and-media-services) | Age-gate enforcement |
| `court_order` | Jurisdiction-specific court order | Per-jurisdiction handling |

### 5.3 Substrate-protective vs third-party-claim discipline (NodeCore#24)

Two structurally distinct emit paths. Per the answer to
[NodeCore#24](https://github.com/CIRISAI/CIRISNodeCore/issues/24)
(which unblocks CIRISPersist#134's takedown_handler):

**Owner-side enforcement (bypasses the standard trust × capacity
admission gate):**

* The operator's own substrate emitting a takedown against content
  in its OWN holds. The operator can always evict their own
  holdings — this is just a `withdraws` against their own
  `holds_bytes` and doesn't depend on third-party trust
* Operator-configured trusted clearinghouses (NCMEC, GIFCT-CIP,
  regional equivalents: IWF, Project Arachnid, INHOPE national
  hotlines). These are pre-trusted via operator config, NOT via
  runtime trust-graph admission. Their takedowns are treated as
  substrate-trusted by configuration assertion
* Substrate-emitted auto-detection at `put_blob_signing` (perceptual
  hash match per §5.6). The substrate signs these with its own
  operator key; the takedown enters as an owner-side artifact

**Third-party claims (full trust × capacity gate applies):**

* DMCA §512 claimant notices
* DSA Article 16 third-party reports
* Community standards reports
* Any claim where `claimant_key_id` is an arbitrary federation key

**Why this split:** substrate operators have legal liability for
content held under their own keys (NCMEC reporting obligation; TVEC
1-hour deadline; OSA illegal-content duty). The operator's
authority to police their own holdings doesn't depend on whether
the federation's trust graph says someone with sufficient trust
score "told them to." A fresh edge with no trusted-clearinghouse
keys still has the operator's own key — they can hash-match against
their own configured databases and emit owner-side takedowns
immediately. The trust gate remains the chokepoint for
*third-party claims to ACT on someone else's substrate*.

### 5.4 The propagation mechanism

1. **Owner-side enforcement** (§5.3 first category): operator's
   substrate emits the `takedown_notice` + the `withdraws` against
   own `holds_bytes` in one transaction. No federation admission
   gate; this is policing own holdings.
2. **Third-party claim** (§5.3 second category): claimant signs
   `takedown_notice` Contribution. Federation admits it via standard
   trust × capacity gate; recipients (substrate operators holding
   the content) evaluate per their operator policy.
3. Recipients (in both cases above) apply per-LegalBasis discipline:
   * For substrate-mandatory bases (NcmecCsam, PerceptualHashCsam,
     TvecTerrorist, GifctCip): immediate eviction; emit
     `withdraws:holds_bytes:sha256:{prefix}` against the content;
     no counter-notice path
   * For copyright/DSA/OSA/AVMSD/community bases: expeditious removal
     pending the configurable hold window (§5.5); emit conditional
     `withdraws` with scheduled restoration if uncontested
4. The `withdraws` propagation rides the existing CEG §10.1.2
   ContentMiss feedback loop. Other operators receive the
   `withdraws` against the prior `holds_bytes`, drop their copies,
   emit their own `withdraws` if they had advertised holding.
5. Federation-wide eviction is structural: per-actor `evict_actor`
   (persist v3.5.0) enforces the bulk-eviction case (slashed
   actor's entire content set).

### 5.5 Counter-notice / reconsideration + hold-window schedule

The existing CEG `ReconsiderationRequest` structural primitive (1+4
lockdown member) handles counter-notices. The party whose content
was taken down can file a `ReconsiderationRequest` against the
takedown_notice; if WA-quorum adjudicates in their favor, a
`ReconsiderationAttestation` reverses the takedown and restores
the content's `holds_bytes` advertisements.

Per-LegalBasis hold window (configurable; defaults shipped):

| LegalBasis | Default hold | Counter-notice path | Discipline source |
|---|---|---|---|
| `NcmecCsam` / `PerceptualHashCsam` | Immediate (no hold) | None (mandatory reporting) | NCMEC CyberTipline |
| `TvecTerrorist` / `GifctCip` | Immediate (no hold) | None (1-hour regulatory deadline) | TVEC + Christchurch Call |
| `Dmca512` | 14 calendar days | `ReconsiderationRequest` (§512(g)) | DMCA §512 safe harbor |
| `DsaArticle16` | 7 calendar days | `ReconsiderationRequest` (Art. 20) | DSA |
| `OsaIllegalContent` | 14 calendar days | `ReconsiderationRequest` | UK OSA Ofcom guidance |
| `AvmsdAgeInappropriate` | 7 calendar days | `ReconsiderationRequest` | EU AVMSD |
| `CommunityStandards` | Per-cohort policy (default immediate) | `ReconsiderationRequest` | Per-cohort discipline |
| `CourtOrder` | Immediate (default) | Jurisdiction-specific appeal | Per-jurisdiction |

**Business-day mapping note:** DMCA §512(g) specifies "10-14 *business*
days" between counter-notice and restoration. The substrate ships
"14 *calendar* days" as the default DMCA hold to provide safe-harbor
coverage in all jurisdictions without requiring per-jurisdiction
business-calendar tracking (an operator concern, not a substrate
concern). Operators with strict business-day requirements MAY lower
their hold window to 10 calendar days; with strict counter-notice
windows MAY raise to 21 calendar days. The substrate enforces the
operator's configured value uniformly; operators choose strictness
appropriate to their jurisdiction.

This is structurally identical to the DMCA §512(g) restoration
protocol but with federation-cryptographic provenance throughout.

### 5.6 Substrate-level hash-matching

For CSAM specifically, PhotoDNA-class perceptual hash matching
happens at the federation propagation event (the moment
`put_blob_signing` would emit a `holds_bytes` advertisement) — NOT
at the user's device. This is the architecturally-defensible line
that [Apple's NeuralHash 2021 proposal](https://arxiv.org/abs/2111.06628)
failed to draw and that [Signal](https://signal.org/bigbrother/)
correctly refuses on the user's device.

Implementation:
* Operator-config: which hash databases to consult ([NCMEC's
  PhotoDNA](https://technologycoalition.org/resources/update-on-voluntary-detection-of-csam/),
  [Project Arachnid](https://protectchildren.ca/en/programs-and-initiatives/project-arachnid/),
  [Meta's PDQ-open-source](https://github.com/facebook/ThreatExchange/tree/main/pdq))
* On match at federation-handoff: refuse the `holds_bytes`
  advertisement; emit a `takedown_notice` with `legal_basis:
  perceptual_hash_csam`; trigger NCMEC reporting per US
  jurisdiction; evict any existing copies via the per-actor
  pathway

Locally-scoped content (self/family) is never federation-propagated
and so never hash-matched — same Signal-equivalent posture. The
defensible line: at the federation handoff, hash-match is
substrate-protective; at the user's device, scanning is
surveillance.

## 6. Content encryption + key distribution

The substrate is intentionally agnostic about content encryption — it
provides **identity** (federation keys) + **atomic delivery**
(`put_blob_signing` + Contributions). Content-level encryption is an
application-layer pattern that rides on top of the substrate's
identity surface, parameterized by *who's authorized to read*.

### 6.1 Three encryption postures, by content class

| Content class | Encryption model | Substrate surface used |
|---|---|---|
| **Public** (encyclopedia, news, generally-published, PG-13 community/global) | **None at content level.** Bytes are hash-addressed cleartext. Substrate encrypts at rest via persist's AES-GCM (5.45 GiB/s; protects against disk theft). Access control = discovery + admission, not decryption. | put_blob_signing + holds_bytes; persist's at-rest encryption |
| **Restricted-group** (CW community, family chat, members-only research) | **Group DEK + per-member-wrapped DEK**. Content body encrypted with random DEK; DEK wrapped under each member's federation pubkey. DEK rotates on membership change. | put_blob_signing for ciphertext + `key_grant` Contributions (§6.2) for wrapped DEKs |
| **Subscription / publisher-routed** (AdultHUB, OnlyFans, Netflix, paid community) | **Per-content DEK + per-subscriber-wrapped DEK**. Publisher generates DEK per content; wraps for each subscriber's pubkey when they're authorized; revocation = stop wrapping for revoked subscribers. | Same as restricted-group; usually with `BlobBody::External` for the ciphertext + key_grants for the DEKs |
| **Person-to-person** (direct messages, self-scope addressed delivery) | **Ephemeral X25519 + AES-GCM** (Signal-class E2E). Single recipient, no DEK distribution needed beyond the sealed-envelope wrap. | Edge's `inline_text_pipeline` (Classify + Scrub + AES-GCM) extended for content carriage |

### 6.2 The `key_grant` subject_kind (additive — 1+4 stays locked)

A new Contribution subject_kind, additive to SCHEMA §3.1 (same shape
as `takedown_notice` added in §5.1). **No new structural attestation
primitive** — `key_grant` is a payload variant, not a 6th member of
the 1+4 wire-format lockdown.

**Payload schema:**

```
{
  "subject_kind": "key_grant",
  "recipient_key_id": "<grantee federation pubkey b64>",
  "content_sha256": "<sha256 of the ciphertext blob>",
  "wrapped_dek": "<base64 — see §6.3>",
  "wrap_algorithm": "x25519-aes256-gcm-hkdf-sha256",
  "ratchet_version": 1,
  "key_validity_window": {
    "not_before": "<RFC 3339 datetime>",
    "not_after":  "<RFC 3339 datetime or null>"
  },
  "scope": "single_content" | "group_member" | "subscription_tier",
  "scope_id": "<group_id or subscription_id, optional>",
  "rotation_chain": ["<prior key_grant content_sha?>"]
}
```

The Contribution envelope itself is signed by the grantor (publisher
/ group admin / sender) with their federation key. The substrate
admits the grant via the standard trust × capacity gate; the recipient
fetches it via `ContentFetch` (or it's pushed via `Contribution{Submit}`
addressed to recipient); the recipient's application unwraps the DEK
locally and decrypts the referenced content.

### 6.3 The wrap algorithm — `x25519-aes256-gcm-hkdf-sha256`

The wrap is hybrid (ephemeral DH key agreement + AEAD encryption of
the DEK). Identical pattern to NaCl `crypto_box_seal` / libsodium
sealed boxes / [HPKE RFC 9180](https://datatracker.ietf.org/doc/html/rfc9180)
base mode.

```
For each (recipient_pubkey, DEK):
  ephemeral_priv = random 32 bytes (curve25519 scalar)
  ephemeral_pub  = X25519_basepoint × ephemeral_priv
  shared        = X25519(ephemeral_priv, recipient_pubkey)
  kek           = HKDF-SHA256(shared, salt = ephemeral_pub || recipient_pubkey, info = "cewp-key-grant/v1")
  nonce         = random 12 bytes
  ciphertext    = AES-256-GCM(kek, nonce, DEK)
  wrapped_dek   = ephemeral_pub || nonce || ciphertext
```

**Why this primitive set:**

* X25519 is in [CIRISVerify](https://github.com/CIRISAI/CIRISVerify)
  already (the Curve25519 ECDH that Reticulum uses at the transport
  layer); reuses one primitive across substrate + content layers
* AES-256-GCM is in CIRISVerify's `ring`-backed crypto crate at
  5.45 GiB/s; wrapping cost is dominated by the X25519 op (~50 µs)
* HKDF-SHA256 is shipped (548 ns per derivation)
* No pairing-based or lattice primitives required at the wrap layer;
  the structural attack surface is exactly the same as Signal sealed
  sender / HPKE base mode (well-studied, deployed at scale)
* **Substrate signature stays hybrid PQC (Ed25519 + ML-DSA-65)** —
  the wrap is for the DEK; the Contribution carrying the wrap is
  signed by the grantor's full hybrid key; long-term provenance is
  PQC-secure

**A note on PQC wrap:** v1 uses X25519 for the wrap because adding
ML-KEM (FIPS 203) at the content-encryption layer is substantial new
substrate work and the ciphertext-stealing horizon (the time after
which past wraps could be retroactively decrypted by future quantum
attackers) is much further out than the signature-forging horizon
(which is why the substrate already signs hybrid). Wrap algorithm
identifier in `wrap_algorithm` is versioned (`v1` = X25519; future
`v2` could be hybrid X25519 + ML-KEM or pure ML-KEM); the
substrate's existing primitive-rotation discipline (CEG §0.x → 0.(x+1)
wire-breaks enumerated) handles future migration.

### 6.4 Group key rotation for CW communities

CW community uses a **shared symmetric group DEK** rotated on
membership change. The rotation pattern:

```
T=0   Community admin generates DEK_0; emits key_grants to all
      initial members under DEK_0. Past content (none) uses DEK_0.
T=k   New member M joins. Admin emits a key_grant to M for the
      CURRENT DEK_0 (admitting them to existing content) and
      simultaneously generates DEK_1, emits key_grants to ALL
      members (including M) under DEK_1. All NEW content from T=k+ε
      onward uses DEK_1.
T=l   Member X leaves / is removed. Admin generates DEK_2, emits
      key_grants to remaining members under DEK_2. New content uses
      DEK_2. X retains DEK_0 and DEK_1 for content they were
      authorized for; cannot decrypt DEK_2 content.
```

**Forward-secrecy posture:** X (the leaver) can still decrypt
content they had legitimate access to under DEK_0 / DEK_1. This is
the same posture as Signal group messaging (departed members keep
past message keys). For **full forward secrecy** (X cannot decrypt
ANY past content after leaving), [MLS (RFC 9420)](https://datatracker.ietf.org/doc/html/rfc9420)
tree-based ratcheting is the application-layer composition —
implementable on top of the `key_grant` primitive but not required
at the substrate level. Operators can choose: simple rotation
(this FSD's v1) or MLS (a community-attestation extension layer).

### 6.5 Per-subscriber wrap for publisher routes (AdultHUB / OnlyFans / Netflix)

Subscription-routed content uses **per-content DEK with per-
subscriber wrap**:

```
For each content C:
  Publisher generates DEK_C; encrypts content with DEK_C; stores
  ciphertext at BlobBody::External (publisher's S3) OR BlobBody::Inline
  if ≤ 16 MiB.
  Publisher emits the content Contribution with content_sha256 = SHA(ciphertext).

For each active subscriber S (at content publish time):
  Publisher emits a key_grant Contribution to S under DEK_C with
  scope = "subscription_tier", scope_id = "<their subscription tier>".

For each NEW subscriber S' (joining after content publish):
  Publisher's substrate processes their delegates_to:publisher attestation;
  enumerates content they're now authorized for (per their tier);
  emits key_grants for accessible back-catalog content.

For revoked subscriber R:
  Publisher's substrate stops emitting NEW key_grants for R.
  R retains key_grants already emitted (can decrypt content authorized
  during their active subscription window — same as Netflix
  downloaded-while-active titles remaining playable; deliberate UX
  posture for revocation).
```

**At AdultHUB scale (100M subscribers, 1K new videos/day, ~1 KB per
key_grant):** ~100B key_grants/day federation-aggregate = ~100 GB/day
federation-aggregate. Spread across the publisher's L1 server fleet,
this is bounded by the scaling-model toy's existing federation
bandwidth budget. (`adulthub_replacement` scenario in
[FEDERATION_SCALING_MODEL.md §5](FEDERATION_SCALING_MODEL.md) shows
the publisher-side load fits per-server.)

### 6.6 Revocation, per-actor eviction, and the substrate's identity-aware-storage thesis

The §9 identity-aware-storage property of CEWP
([scaling model §9](FEDERATION_SCALING_MODEL.md))
makes per-actor revocation of key_grants trivial — the substrate
knows which key_grants were issued to which grantee key. Persist
v3.5.0's `evict_actor(key_id)` (CIRISPersist#125) handles the
case of mass-revocation (e.g., a subscriber's key is compromised
and the publisher needs to invalidate all outstanding grants to
that key in one operation).

This is the cryptographic enforcement complement to the trust-graph
admission gate. Together:
* Trust gate refuses new admission of content from / to revoked
  parties
* `evict_actor` retires existing key_grants to revoked parties
* `withdraws`-against-`holds_bytes` propagates the content-side
  eviction
* `ReconsiderationRequest` reverses if the revocation was wrong

The substrate provides the complete revocation surface — at the
discoverability layer, the admission layer, the eviction layer, and
the reconsideration layer — using primitives that already exist.

## 7. The user-controlled algorithm — operationalized

The user's "for you" surface is computed locally from these
already-shipped + this-FSD-extended primitives:

| Knob | Wire-format primitive | Already shipped? |
|---|---|---|
| Trust depth (0/1/2/3) | `delegates_to` graph traversal via NodeCore trust-depth oracle | ✅ NodeCore#21 (this session) |
| Cohort filter (local/community/global) | `cohort_scope` field at admission + UI feed selector | ✅ NodeCore Phase 2B compose surfaces |
| Quality threshold | `compose_article_quality` aggregation thresholding | ✅ NodeCore #19 Phase 3 sub-1 |
| Freshness decay | persist EvictionSweeper decay curve config | ✅ Persist v3.4.0 |
| Content_rating filter | `content_rating:*` dimensions admitted at consumer threshold | This FSD |
| Content_class filter | `content_class:*` dimension at consumer policy | This FSD |
| CW community subscriptions | `delegates_to:cw_community:*` attestations | This FSD |
| Trusted publisher subscriptions | `delegates_to:publisher:*` attestations | This FSD |
| Age gate | `age_assurance:*` attestation + operator gate | This FSD |

The user's algorithm is the tuple of these settings. Default
presets (PG-13 family / standard adult / news-only / academic-only)
ship with the unified-client (CIRISAgent). Custom configs are
exportable as signed attestations the user can carry across
deployments.

This is the "composable moderation" pattern Bluesky / Ozone
pioneered ([Bluesky's Moderation Architecture](https://docs.bsky.app/blog/blueskys-moderation-architecture)),
generalized to the full algorithm rather than just labeling.

## 8. AI-generated content disclosure

[EU AI Act Article 50](https://artificialintelligenceact.eu/article/50/)
requires AI-generated media to be marked as such. This is a
mandatory dimension on AI-generated multimedia:

* `image:authenticity:ai_generated`
* `audio:authenticity:ai_generated`
* `video:authenticity:ai_generated`
* `model_3d:authenticity:ai_generated`

The substrate refuses to carry undisclosed AI-generated content at
`cohort_scope ≥ community` for any sub_kind where `generated_by`
or equivalent metadata indicates AI generation but no
`authenticity:ai_generated` attestation is present. This is
substrate-level enforcement of the EU AI Act disclosure
requirement.

## 9. International standards mapping (full coverage)

| Regulation | What it requires | CEG-native mechanism |
|---|---|---|
| [US DMCA §512(c)(3)](https://www.copyright.gov/512/) | Notice/counter-notice, designated agent, expeditious removal | `takedown_notice` Contribution + `ReconsiderationRequest` |
| [EU DSA Art. 16](https://www.cms-digitallaws.com/en/dsa/article-16/) | Notice mechanism, receipt confirmation, timely decision | `takedown_notice` + receipt-attestation timestamp + decision audit log |
| [EU DSA Art. 20](https://eur-lex.europa.eu/eli/reg/2022/2065/oj/eng) | Internal complaint mechanism | `ReconsiderationRequest` pathway |
| [EU TVEC 2021/784](https://www.europarl.europa.eu/news/en/press-room/20210422IPR02621/new-rules-adopted-for-quick-and-smooth-removal-of-terrorist-content-online) | 1-hour terrorist content removal | `takedown_notice` with `legal_basis: tvec_terrorist`; substrate-mandatory immediate eviction |
| [UK OSA 2023](https://uklitigation.cooley.com/uk-online-safety-act-age-assurance-and-childrens-access-statement/) | Highly effective age assurance for harmful content | `age_assurance:provider:...:adult` from Ofcom-compliant provider |
| [France SREN](https://www.twobirds.com/en/insights/2024/france/la-loi-sren-et-la-protection-des-mineurs) | Double-anonymity age verification | `age_assurance:provider:*` pattern; provider attests against federation key, not real identity |
| [EU AVMSD](https://digital-strategy.ec.europa.eu/en/policies/audiovisual-and-media-services) | Age verification proportionate to risk | Cohort-scope + content_rating + age_assurance composition |
| [US COPPA](https://www.ftc.gov/legal-library/browse/rules/childrens-online-privacy-protection-rule-coppa) | Children's privacy under 13 | Substrate has no behavioral tracking; CEG locality dividend for under-13 substrates |
| [EU GDPR Art. 8](https://gdpr-info.eu/art-8-gdpr/) | Child consent age (13-16 per member state) | Same as COPPA — substrate doesn't process personal data for advertising |
| [EU AI Act Art. 50](https://artificialintelligenceact.eu/article/50/) | AI-generated content disclosure | `authenticity:ai_generated` mandatory dimension |
| [US KOSA (pending)](https://www.congress.gov/bill/118th-congress/senate-bill/1409) | Duty of care for minors | Substrate refuses engagement-optimization; user-controlled algorithm; PG-13 default at federation scope |
| [Christchurch Call](https://www.christchurchcall.org/) / [GIFCT](https://gifct.org/) | Terrorist content removal coordination | `legal_basis: gifct_cip` substrate-mandatory takedown |
| [NCMEC CyberTipline](https://www.missingkids.org/gethelpnow/cybertipline) | Mandatory CSAM reporting (US providers) | `legal_basis: ncmec_csam` substrate-mandatory takedown + report emission |

The substrate doesn't replace any regulation. It provides the
wire-format substrate the regulatory regime can plug into.

## 10. What this requires across the stack

### 10.1 NodeCore (this repo)

* Five new `external_content` sub_kind ingest paths
  (`ingest_image`, `ingest_audio`, `ingest_video`, `ingest_film`,
  `ingest_model_3d`) — same `finalize_external_content_ingest`
  shared tail as existing six sub_kinds
* New source structs + `build_*_payload` builders for each
* Extension to `compose_article_quality` to handle multimedia
  dimension families
* New `takedown_notice` subject_kind + ingest path
* CW community membership tracking via `delegates_to:cw_community:*`
* Age gate consumer policy in compose layer (filter content per
  user's age_assurance + content_rating thresholds)
* SCHEMA.md §4.29 extension (five new sub_kinds)
* SCHEMA.md new section for `takedown_notice` + `age_assurance:*`
  attestation classes

### 10.2 Persist

* Substrate-level perceptual-hash check at `put_blob_signing` for
  CSAM-class content (operator-config which hash databases to
  consult; PhotoDNA / PDQ / Project Arachnid)
* `takedown_notice` substrate enforcement on receive
* `withdraws`-against-`holds_bytes` propagation already exists
  (CEG §10.1.2); just wired up at the takedown_notice handler

### 10.3 Edge

* MessageType for takedown_notice dispatch + counter-notice
* No new transport work — rides existing Contribution MessageType
  family

### 10.4 LensCore

* AI-generation detection model (helps enforce `authenticity:
  ai_generated` even when content lacks self-disclosure)
* Detection events for content-class-misclassification (porn
  claiming to be "art")

### 10.5 Registry

* CEG §X.Y addition codifying the multimedia sub_kinds + content
  classification + takedown_notice + age_assurance attestation
  families
* 1+4 wire-format lockdown survives — this is all dimension
  additions + new subject_kinds, NOT new structural primitives

### 10.6 Agent runtime

* Default user-algorithm presets (PG-13 family / standard adult /
  news-only / academic-only / horror-CW / art-cinema-CW)
* "I am not a child" gate UI implementation (operator-config which
  provider; substrate provides the attestation surface)
* CW community discovery + opt-in UX
* Trusted publisher discovery + opt-in UX (PornHub / OnlyFans / etc.
  appear as discoverable publishers; user opts in via
  `delegates_to:publisher:*` attestation)

## 11. What CEWP takes from prior art (and rejects)

### 11.1 Validates / adopts

* [Bluesky / Ozone composable moderation](https://docs.bsky.app/blog/blueskys-moderation-architecture) — labels-as-signed-artifacts pattern → CEWP's takedown_notice + withdraws structure
* [Nostr NIP-11 relay policy declaration](https://nips.nostr.com/11) — declare substrate discipline at protocol level → CEWP's per-cohort content discipline
* [PornHub Dec 2020 verified-uploader purge](https://www.eff.org/deeplinks/2020/12/visa-and-mastercard-are-trying-dictate-what-you-can-watch-pornhub) → identity-aware uploads make per-actor eviction trivial
* [OnlyFans Ondato-based verification](https://ondato.com/case-studies/onlyfans-case-study/) → provider-attestation pattern works
* [Signal's defensible E2EE line](https://signal.org/bigbrother/) → self/family scope is the substrate's "we don't see, we don't carry" position
* [PhotoDNA + PDQ at federation handoff](https://github.com/facebook/ThreatExchange/tree/main/pdq) → substrate-protective hash-matching at propagation event (NOT user device)
* [French SREN double-anonymity](https://www.twobirds.com/en/insights/2024/france/la-loi-sren-et-la-protection-des-mineurs) → provider attests against federation key, not real identity
* [DMCA §512(g) counter-notice](https://www.copyright.gov/512/) → maps to existing CEG `ReconsiderationRequest`

### 11.2 Rejects / avoids

* [Vidme ad-funded death](https://variety.com/2017/digital/news/vidme-shuts-down-video-app-1202628367/) → no ad-funded UGC substrate model; mission-locked AGPL
* [DTube / Steem / LBRY token rewards → farming + SEC risk](https://knightcolumbia.org/blog/mapping-social-media-crypto-logic-platforms-and-the-cautionary-tale-of-steemit) → no monetization primitives at substrate layer
* [BitChute "free speech" → extremist concentration](https://dl.acm.org/doi/10.1145/3710950) → positioning is "we don't centralize," not "we don't have rules"
* [Stanford Mastodon CSAM finding](https://stacks.stanford.edu/file/druid:vb515nd6874/20230724-fediverse-csam-report.pdf) — volunteer per-instance moderation can't scale → substrate-level structural refusal + perceptual hash match at federation handoff
* [Apple NeuralHash failure](https://arxiv.org/abs/2111.06628) — adversarial collisions on client-side scanning → hash-match at federation event, not user device
* TikTok/YouTube engagement-optimization → no centralized recommender; user controls the algorithm

## 12. Open questions

* **Live-streaming wire format details** — `live_stream` sub_kind sketched but implementation deferred to Phase 2.
* **C2PA integration for image/video provenance** — should `authenticity:provenance_chain` attestations carry [C2PA](https://c2pa.org/) manifests, or define a CEG-native provenance-chain dimension that interoperates with C2PA on import?
* **Perceptual hash database access** — PhotoDNA is gated to vetted orgs; PDQ is open-source; CEWP-operator-coordination for shared hash access at federation scale needs a Registry-side governance decision.
* **CW community vs trusted-publisher tradeoff** — both paths can route adult content; do they coexist (yes per this FSD) or should one be the canonical pattern? Current answer: both coexist; CW communities for community-of-interest; publishers for one-to-many.
* **Operator coordination for fast-path takedowns** (TVEC 1-hour, GIFCT CIP) — needs Registry-side cross-operator notification protocol; sketched in this FSD but specification belongs in CEG governance §11.

## 13. References

### Internal

* [CEWP.md](CEWP.md) — platform identity
* [FEDERATION_SCALING_MODEL.md](FEDERATION_SCALING_MODEL.md) — substrate scaling
* [ANONYMOUS_TIER.md](ANONYMOUS_TIER.md) — v2 deniability path
* [SCHEMA.md §4.29](../SCHEMA.md) — existing external_content
* [CEG 0.2](https://github.com/CIRISAI/CIRISRegistry/tree/main/FSD/CEG) — wire-format authority

### External — moderation architecture prior art

* [Bluesky's Moderation Architecture](https://docs.bsky.app/blog/blueskys-moderation-architecture)
* [Stackable Approach to Moderation (Bluesky)](https://bsky.social/about/blog/03-12-2024-stackable-moderation)
* [Ozone — GitHub](https://github.com/bluesky-social/ozone)
* [Nostr NIP-11 — Relay Information Document](https://nips.nostr.com/11)
* [Mastodon — Moderation actions documentation](https://docs.joinmastodon.org/admin/moderation/)
* [Stanford — Child Safety on Federated Social Media (2023)](https://stacks.stanford.edu/file/druid:vb515nd6874/20230724-fediverse-csam-report.pdf)
* [IFTAS 2024 Needs Assessment](https://about.iftas.org/2024/12/17/the-2024-iftas-needs-assessment-report-is-here/)
* [Collaborative Content Moderation in the Fediverse (arXiv)](https://arxiv.org/html/2501.05871v1)
* [Will Admins Cope? Decentralized Moderation in the Fediverse (CHI 2023)](https://dl.acm.org/doi/fullHtml/10.1145/3543507.3583487)

### External — content rating + age verification

* [MPA Film Ratings](https://www.filmratings.com/ratings-guide/)
* [BBFC Classification Guidelines](https://www.bbfc.co.uk/what-we-do/classification-guidelines)
* [PEGI](https://pegi.info/)
* [Common Sense Media](https://www.commonsensemedia.org/about-us/our-mission/about-our-ratings)
* [UK Online Safety Act age assurance (Cooley)](https://uklitigation.cooley.com/uk-online-safety-act-age-assurance-and-childrens-access-statement/)
* [Arcom — French SREN technical guidelines](https://www.arcom.fr/en/find-out-more/legal-area/legal-resources/technical-guidelines-age-verification-protection-persons-under-18-online-pornography)
* [France SREN child protection (Bird & Bird)](https://www.twobirds.com/en/insights/2024/france/la-loi-sren-et-la-protection-des-mineurs)
* [Yoti — Age verification privacy](https://www.yoti.com/privacy/age-verification/)

### External — CSAM / takedown / standards

* [US DMCA §512](https://www.copyright.gov/512/)
* [EU DSA Article 16 (CMS)](https://www.cms-digitallaws.com/en/dsa/article-16/)
* [EU TVEC Regulation 2021/784](https://www.europarl.europa.eu/news/en/press-room/20210422IPR02621/new-rules-adopted-for-quick-and-smooth-removal-of-terrorist-content-online)
* [EU AVMSD](https://digital-strategy.ec.europa.eu/en/policies/audiovisual-and-media-services)
* [PhotoDNA — Technology Coalition](https://technologycoalition.org/resources/update-on-voluntary-detection-of-csam/)
* [PhotoDNA Analysis (ACM 2023)](https://dl.acm.org/doi/fullHtml/10.1145/3600160.3605048)
* [Meta PDQ — GitHub](https://github.com/facebook/ThreatExchange/tree/main/pdq)
* [Project Arachnid](https://protectchildren.ca/en/programs-and-initiatives/project-arachnid/)
* [NeuralHash collision attacks (arXiv)](https://arxiv.org/abs/2111.06628)
* [Signal — Government Communication](https://signal.org/bigbrother/)
* [GIFCT Hash-Sharing Database](https://gifct.org/hsdb/)
* [GIFCT CIP Buffalo debrief](https://gifct.org/2022/06/23/debrief-cip-activation-buffalo/)
* [EU AI Act Article 50](https://artificialintelligenceact.eu/article/50/)
* [US KOSA (S.1409)](https://www.congress.gov/bill/118th-congress/senate-bill/1409)

### External — prior art (failures + cautionary tales)

* [Variety — Vidme shutdown](https://variety.com/2017/digital/news/vidme-shuts-down-video-app-1202628367/)
* [BitChute Community Guidelines](https://support.bitchute.com/policy/guidelines/)
* [Content Moderation and Hate Speech on Alternative Platforms (PACM HCI)](https://dl.acm.org/doi/10.1145/3710950)
* [Pew Research on BitChute](https://www.pewresearch.org/short-reads/2023/02/17/key-facts-about-bitchute/)
* [Knight Institute — Steemit cautionary tale](https://knightcolumbia.org/blog/mapping-social-media-crypto-logic-platforms-and-the-cautionary-tale-of-steemit)
* [LBRY — Wikipedia](https://en.wikipedia.org/wiki/LBRY)
* [EFF — Visa/Mastercard vs PornHub](https://www.eff.org/deeplinks/2020/12/visa-and-mastercard-are-trying-dictate-what-you-can-watch-pornhub)
* [Ondato — OnlyFans Case Study](https://ondato.com/case-studies/onlyfans-case-study/)
