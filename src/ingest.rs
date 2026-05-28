//! External content ingestion pipeline (CIRISNodeCore#19).
//!
//! Wikipedia + news + other encyclopedic / journalistic sources absorbed
//! into the federation as `external_content` Contributions per
//! [SCHEMA.md §4.29](../SCHEMA.md). The 1+4 wire format absorbs both
//! shape families through a `sub_kind` discriminator + shared envelope
//! + new dimension prefix vocabulary (the prefixes themselves land via
//! a Registry FSD-002 §4.9.2 amendment; this module produces payloads
//! that reference them).
//!
//! # What this module provides (Phase 2A)
//!
//! Pure types + a payload-build function:
//!
//! - [`EncyclopediaArticleSource`] / [`NewsArticleSource`] — input types
//!   the importer (Wikipedia dump walker / RSS feed reader / etc.)
//!   populates
//! - [`build_external_content_payload`] — pure function: source +
//!   metadata → canonical JSON conforming to SCHEMA §4.29
//!
//! No I/O, no engine, no signing. The output JSON is the
//! `Contribution.payload` field for a `proposal`-type Contribution with
//! `subject_kind = "external_content"`. The signing + persist-write
//! pipeline lives in Phase 2B (separate commit).
//!
//! # Pattern parallel to [`crate::compose`]
//!
//! Pure-Rust transformer; unit-testable without persist / edge / pyo3.
//! When persist exposes the right Rust-trait accessors (federation
//! directory + blob storage), the Phase 2B function will wrap this one
//! with the persist write calls.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The `cohort_scope` enumeration mirrored from FSD-002 v1.4 §1.7
/// (envelope axis). Carried in the external_content payload as the
/// v0.1 shape until persist's ContributionEnvelope exposes the
/// envelope-level field; once it does, consumer compose layers
/// dual-read (envelope preferred, payload fallback).
///
/// UI sectioning per the three-tier model:
/// - **Local** — `self`
/// - **Community commons** — `family` / `community` / `affiliations`
/// - **Global commons** — `species` / `planet` / `federation`
pub mod scope {
    /// `self` — owner-private; only the owner's runtime sees it.
    pub const SELF_: &str = "self";
    /// `family` — household / immediate-cohort scope.
    pub const FAMILY: &str = "family";
    /// `community` — local community / clinic / school / town scope.
    pub const COMMUNITY: &str = "community";
    /// `affiliations` — professional guild / federation-of-orgs scope.
    pub const AFFILIATIONS: &str = "affiliations";
    /// `species` — humanity-scale (universally human-affecting).
    pub const SPECIES: &str = "species";
    /// `planet` — biosphere-scale (cross-species ecological).
    pub const PLANET: &str = "planet";
    /// `federation` — the CIRIS federation as a whole.
    pub const FEDERATION: &str = "federation";

    /// UI tier classification per the three-section model.
    pub fn tier_for(scope: &str) -> Option<&'static str> {
        match scope {
            "self" => Some("local"),
            "family" | "community" | "affiliations" => Some("community"),
            "species" | "planet" | "federation" => Some("global"),
            _ => None,
        }
    }
}

/// Wikipedia-shape article source. Populated by an importer that walks
/// a Wikipedia XML dump / API response / mirror archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncyclopediaArticleSource {
    /// Stable key_id for the article entity. Pattern:
    /// `{project_prefix}:article:{slug}` — e.g.
    /// `wikipedia:article:einstein`. The slug should be project-stable
    /// across language editions (Wikipedia uses Wikidata QIDs for
    /// cross-language anchoring; importers MAY map to the
    /// language-specific slug or the QID depending on the cohort scope
    /// they're feeding).
    pub entity_key_id: String,
    /// ISO 639-1 language code (e.g. `en`, `fr`, `am`).
    pub language: String,
    /// The article body bytes. SHA-256 computed at payload-build time;
    /// caller is responsible for storing the bytes in `federation_blobs`
    /// (separately, via `BlobStorage::put_blob`).
    pub body_bytes: Vec<u8>,
    /// RFC 6838 media type. Typical: `text/html`, `text/markdown`,
    /// `application/json` (structured encyclopedic data).
    pub body_media_type: String,
    /// Encyclopedia project (e.g. `wikipedia`, `wikidata`,
    /// `simple_wikipedia`, `wiktionary`).
    pub project: String,
    /// Revision identifier (Wikipedia's `revid`, or equivalent).
    pub revision_id: String,
    /// When the revision was made.
    pub edited_at: DateTime<Utc>,
    /// Cohort scope at which this content is being asserted (per
    /// [`scope`] module). Typically `federation` for canonical
    /// Wikipedia content.
    pub cohort_scope: String,
    /// Inter-article links extracted from the body. Each becomes a
    /// separate `topical_relation:{relation}:{target_key_id}` scores
    /// attestation in Phase 2B (federation directory write).
    #[serde(default)]
    pub topical_relations: Vec<TopicalRelation>,
    /// External (non-CIRIS) citations: primary sources, journals, etc.
    /// Each becomes a separate `cites_source:{kind}` attestation.
    #[serde(default)]
    pub citations: Vec<Citation>,
}

/// News-article source. Populated by an importer that reads an RSS
/// feed, a news API response, or a wire-service stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsArticleSource {
    /// Stable key_id. Pattern:
    /// `news:article:{publisher}:{publication_date}:{slug}` — e.g.
    /// `news:article:nyt:2026-05-15:climate-summit`. The triple
    /// (publisher, date, slug) makes news articles uniquely identifiable
    /// even when slugs collide across publishers / dates.
    pub entity_key_id: String,
    /// ISO 639-1 language code.
    pub language: String,
    /// The article body bytes.
    pub body_bytes: Vec<u8>,
    /// RFC 6838 media type for the body.
    pub body_media_type: String,
    /// Publisher tag (e.g. `nyt`, `bbc`, `reuters`, `ap`).
    pub publisher: String,
    /// Publisher's federation key_id (e.g. `publisher:nyt`). Resolved
    /// via the federation directory; the publisher's standing
    /// (`news:source_quality:{publisher}`) is a separate dimension
    /// consumers read for trust composition.
    pub publisher_key_id: String,
    /// When the article was published by the publisher.
    pub published_at: DateTime<Utc>,
    /// Optional byline. Some wire-service articles are unbylined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byline: Option<String>,
    /// Optional federation key_id for the journalist (when known + when
    /// the journalist has a federation identity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byline_key_id: Option<String>,
    /// Optional publisher-specific section (e.g. `world`, `politics`,
    /// `business`, `opinion`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// Article headline.
    pub headline: String,
    /// Cohort scope at which this content is being asserted (per
    /// [`scope`] module). Typical values for news: `federation`
    /// (international news), `community` (local news).
    pub cohort_scope: String,
    /// Inter-article links extracted from the body. Each becomes a
    /// `topical_relation:*` attestation in Phase 2B.
    #[serde(default)]
    pub topical_relations: Vec<TopicalRelation>,
    /// External (non-CIRIS) citations.
    #[serde(default)]
    pub citations: Vec<Citation>,
    /// Optional staleness contract — news has time-decay; consumers
    /// downweight or filter past this point. Encyclopedia content
    /// typically leaves this unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<DateTime<Utc>>,
}

/// One inter-article link (hyperlink) extracted from an article body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicalRelation {
    /// The target article's `entity_key_id`. Resolved via the
    /// federation directory; if the target doesn't exist yet, the
    /// attestation is still well-formed (forward-reference; the
    /// reference becomes resolvable when the target is later ingested).
    pub target_key_id: String,
    /// What kind of topical link.
    pub relation: TopicalRelationKind,
}

/// The relation type carried by [`TopicalRelation`]. Materializes as
/// the `{relation}` segment of `topical_relation:{relation}:{target}`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TopicalRelationKind {
    /// Standard inter-article link (Wikipedia blue link, news article
    /// reference).
    References,
    /// "See also" — soft topical link, lower weight than `References`.
    SeeAlso,
    /// Same-name-different-entity (disambiguation pages).
    Disambiguates,
    /// News-style correction: this article corrects a prior one.
    /// Composes with the structural primitive `recants` on the false
    /// claim.
    Corrects,
    /// Encyclopedia revision-chain (article-level supersedes; distinct
    /// from the structural-primitive `supersedes` which applies to
    /// individual attestations).
    SupersedesArticle,
}

impl TopicalRelationKind {
    /// String form for embedding in the dimension prefix
    /// (`topical_relation:{this}:{target_key_id}`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::References => "references",
            Self::SeeAlso => "see_also",
            Self::Disambiguates => "disambiguates",
            Self::Corrects => "corrects",
            Self::SupersedesArticle => "supersedes_article",
        }
    }
}

/// One external (non-CIRIS) source citation extracted from an article.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// What kind of external source.
    pub kind: CitationKind,
    /// The citation string itself (DOI / ISBN / URL / arXiv ID / etc.).
    pub ref_string: String,
}

/// Citation kind. Materializes as the `{kind}` segment of
/// `cites_source:{kind}`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CitationKind {
    /// Primary source citation (journal article, monograph, archival
    /// document, dataset). Highest-weight citation type.
    PrimarySource,
    /// Generic external URL (web page, blog post, online document).
    ExternalUrl,
    /// DOI identifier.
    Doi,
    /// ISBN identifier (books).
    Isbn,
    /// arXiv identifier (preprints).
    Arxiv,
    /// Court-case citation (legal sources).
    Caselaw,
}

impl CitationKind {
    /// String form for embedding in the dimension prefix
    /// (`cites_source:{this}`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PrimarySource => "primary_source",
            Self::ExternalUrl => "external_url",
            Self::Doi => "doi",
            Self::Isbn => "isbn",
            Self::Arxiv => "arxiv",
            Self::Caselaw => "caselaw",
        }
    }
}

/// Build the canonical `payload` JSON for a Wikipedia article
/// Contribution per SCHEMA.md §4.29.
///
/// Returns the JSON value the caller wraps into a
/// `ContributionEnvelope.payload` field and signs. Phase 2B will wrap
/// this with the engine.put_contribution + blob storage write.
pub fn build_encyclopedia_payload(
    article: &EncyclopediaArticleSource,
) -> Result<(serde_json::Value, [u8; 32]), IngestError> {
    if article.entity_key_id.is_empty() {
        return Err(IngestError::EmptyKeyId);
    }
    if article.language.is_empty() {
        return Err(IngestError::EmptyLanguage);
    }
    if article.body_bytes.is_empty() {
        return Err(IngestError::EmptyBody);
    }
    if !is_promotable_scope(&article.cohort_scope) {
        return Err(IngestError::UnknownPromotionScope(article.cohort_scope.clone()));
    }
    let sha256 = compute_sha256(&article.body_bytes);

    Ok((
        serde_json::json!({
            "sub_kind": "encyclopedia_article",
            "entity_key_id": article.entity_key_id,
            "language": article.language,
            "cohort_scope": article.cohort_scope,
            "content_sha256": hex_encode(&sha256),
            "content_media_type": article.body_media_type,
            "content_size_bytes": article.body_bytes.len(),
            "source": {
                "kind": "encyclopedia",
                "project": article.project,
                "revision_id": article.revision_id,
                "edited_at": article.edited_at,
            },
            "topical_relations": article.topical_relations.iter().map(|tr| {
                serde_json::json!({
                    "target_key_id": tr.target_key_id,
                    "relation": tr.relation.as_str(),
                })
            }).collect::<Vec<_>>(),
            "citations": article.citations.iter().map(|c| {
                serde_json::json!({
                    "kind": c.kind.as_str(),
                    "ref": c.ref_string,
                })
            }).collect::<Vec<_>>(),
        }),
        sha256,
    ))
}

/// Build the canonical `payload` JSON for a news article Contribution
/// per SCHEMA.md §4.29.
pub fn build_news_payload(
    article: &NewsArticleSource,
) -> Result<(serde_json::Value, [u8; 32]), IngestError> {
    if article.entity_key_id.is_empty() {
        return Err(IngestError::EmptyKeyId);
    }
    if article.language.is_empty() {
        return Err(IngestError::EmptyLanguage);
    }
    if article.body_bytes.is_empty() {
        return Err(IngestError::EmptyBody);
    }
    if article.headline.is_empty() {
        return Err(IngestError::EmptyHeadline);
    }
    if !is_promotable_scope(&article.cohort_scope) {
        return Err(IngestError::UnknownPromotionScope(article.cohort_scope.clone()));
    }
    let sha256 = compute_sha256(&article.body_bytes);

    let mut source = serde_json::json!({
        "kind": "news",
        "publisher": article.publisher,
        "publisher_key_id": article.publisher_key_id,
        "published_at": article.published_at,
        "headline": article.headline,
    });
    if let Some(ref byline) = article.byline {
        source["byline"] = serde_json::Value::String(byline.clone());
    }
    if let Some(ref bk) = article.byline_key_id {
        source["byline_key_id"] = serde_json::Value::String(bk.clone());
    }
    if let Some(ref section) = article.section {
        source["section"] = serde_json::Value::String(section.clone());
    }

    let mut payload = serde_json::json!({
        "sub_kind": "news_article",
        "entity_key_id": article.entity_key_id,
        "language": article.language,
        "cohort_scope": article.cohort_scope,
        "content_sha256": hex_encode(&sha256),
        "content_media_type": article.body_media_type,
        "content_size_bytes": article.body_bytes.len(),
        "source": source,
        "topical_relations": article.topical_relations.iter().map(|tr| {
            serde_json::json!({
                "target_key_id": tr.target_key_id,
                "relation": tr.relation.as_str(),
            })
        }).collect::<Vec<_>>(),
        "citations": article.citations.iter().map(|c| {
            serde_json::json!({
                "kind": c.kind.as_str(),
                "ref": c.ref_string,
            })
        }).collect::<Vec<_>>(),
    });
    if let Some(vu) = article.valid_until {
        payload["valid_until"] = serde_json::json!(vu);
    }

    Ok((payload, sha256))
}

/// Accord-data source. ACCORD.md, encyclical mappings, framework
/// documents, federation policy declarations — content carrying
/// constitutional weight per FSD-002 v1.4 §7.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccordDataSource {
    /// Stable key_id. Pattern: `accord:{accord_kind}:{slug}` — e.g.
    /// `accord:canonical_text:accord_v1.2` or
    /// `accord:encyclical_mapping:magnifica_humanitas_v1`.
    pub entity_key_id: String,
    /// ISO 639-1 language code.
    pub language: String,
    /// The accord-document body bytes.
    pub body_bytes: Vec<u8>,
    /// RFC 6838 media type (typically `text/markdown`).
    pub body_media_type: String,
    /// Discriminator within accord_data.
    pub accord_kind: AccordKind,
    /// Class of signer authorizing this accord document. Determines
    /// the trust gate consumers apply when accepting it.
    pub signer_class: AccordSignerClass,
    /// Federation key_id of the signer (the steward / accord-holder
    /// triple member). For multi-sig signer classes
    /// (`humanity_accord`, `steward_triple`, `wa_quorum`), this is
    /// the canonical attester; the co-signing attestations live in
    /// the `signer_attestation_refs` array.
    pub signer_key_id: String,
    /// Multi-sig co-signing attestation references. For
    /// `humanity_accord` (2-of-3), this array carries the other two
    /// accord-holders' signatures. Length expectations:
    ///   - `humanity_accord`: 2 (the other 2 of 3)
    ///   - `steward_triple`:  2 (the other 2 of 3 regional stewards)
    ///   - `wa_quorum`:       quorum_size - 1
    ///   - other:             0
    #[serde(default)]
    pub signer_attestation_refs: Vec<String>,
    /// When the document becomes effective (federation-wide).
    pub effective_at: DateTime<Utc>,
    /// Version tag (e.g. `v1.2`, `2026-05-15-rev3`).
    pub version_tag: String,
    /// Cohort scope at which this accord document is being asserted
    /// (per [`scope`] module). Typically `species` (all-of-humanity)
    /// or `federation` (CIRIS-internal constitutional layer).
    pub cohort_scope: String,
    /// Inter-document links extracted from the body.
    #[serde(default)]
    pub topical_relations: Vec<TopicalRelation>,
    /// External citations (papers, encyclicals, prior accord versions).
    #[serde(default)]
    pub citations: Vec<Citation>,
}

/// Sub-discriminator within `accord_data`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccordKind {
    /// The canonical ACCORD.md (or its localized translation).
    CanonicalText,
    /// Bootstrap-contributions corpus mapping (per FSD-002 §10.4) —
    /// *Magnifica Humanitas* encyclical, IEEE EAD Ch5, CARE Principles,
    /// etc. mapped into CEG-native vocabulary.
    EncyclicalMapping,
    /// Framework documents (Coherence Ratchet preprint, CCA paper,
    /// federation-grounding theoretical work).
    FrameworkDocument,
    /// Federation-wide policy declaration (e.g., new dimension prefix
    /// admission via §4.9.2 amendment).
    PolicyDeclaration,
}

impl AccordKind {
    /// String form for embedding in the entity_key_id pattern + the
    /// `accord_kind` JSON field.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CanonicalText => "canonical_text",
            Self::EncyclicalMapping => "encyclical_mapping",
            Self::FrameworkDocument => "framework_document",
            Self::PolicyDeclaration => "policy_declaration",
        }
    }
}

/// Class of authority signing an accord document.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccordSignerClass {
    /// 2-of-3 of the three named accord-holders per FSD-002 §7.2.
    /// Only class permitted to sign `AccordCarrier`-priority
    /// FederationAnnouncements; bound to ACCORD.md updates.
    HumanityAccord,
    /// 2-of-3 of the three regional stewards per FSD-002 §10.1.
    /// Operational policy + framework documents.
    StewardTriple,
    /// Wise Authority quorum (size per FSD-002 §6.1.5 locality
    /// scaling). Policy declarations + amendments.
    WaQuorum,
    /// Steward-set 1-of-6 sign-off per FSD-002 §4.9.2 step 5.
    /// Calibration package amendments + similar high-stakes-but-
    /// not-constitutional updates.
    OneOfSix,
}

impl AccordSignerClass {
    /// String form for the `signer_class` JSON field.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HumanityAccord => "humanity_accord",
            Self::StewardTriple => "steward_triple",
            Self::WaQuorum => "wa_quorum",
            Self::OneOfSix => "one_of_six",
        }
    }
}

/// Local-data source. User's private content — notes, drafts,
/// bookmarks, observations — that lives at `cohort_scope: self` and
/// can be promoted to wider scope via `promote_payload`.
///
/// Local data is the lowest-trust scope: self-attested only; no peer
/// review; no expertise weighting; consumer policy treats local
/// attestations as "what this user said about their own content"
/// rather than as federation claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDataSource {
    /// Stable key_id. Pattern: `local:{owner_key_short}:{local_kind}:{slug}` —
    /// e.g. `local:abc123:notes:einstein-research`. The
    /// owner-key prefix makes local items per-user-unique.
    pub entity_key_id: String,
    /// ISO 639-1 language code.
    pub language: String,
    /// The local-content body bytes.
    pub body_bytes: Vec<u8>,
    /// RFC 6838 media type.
    pub body_media_type: String,
    /// Discriminator within local_data.
    pub local_kind: LocalKind,
    /// Federation key_id of the owner (the user authoring this local
    /// content). Implicit in `cohort_scope: self`; surfaced explicitly
    /// for downstream filtering.
    pub owner_key_id: String,
    /// When the local item was authored.
    pub created_at: DateTime<Utc>,
    /// Optional headline / title for the local item (for UI rendering).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional user-supplied tags for categorization. Not surfaced
    /// in any federation directory until promoted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Optional hint that the user intends to promote this item to a
    /// wider scope. Doesn't pre-promote — just lets the UI surface
    /// "ready to share" affordances. The actual promotion is an
    /// explicit user action via `promote_payload`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promote_hint: Option<PromoteHint>,
    /// Topical relations to other local items or already-promoted
    /// content the user wants to associate.
    #[serde(default)]
    pub topical_relations: Vec<TopicalRelation>,
    /// External citations.
    #[serde(default)]
    pub citations: Vec<Citation>,
}

/// Sub-discriminator within `local_data`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalKind {
    /// Personal notes / journal entries / research drafts.
    Notes,
    /// Draft article (heading toward encyclopedia / news promotion).
    Draft,
    /// Bookmarked external content the user wants to track.
    Bookmark,
    /// Observation about the environment (heading toward `notification`
    /// promotion or federation-tier reporting).
    Observation,
}

impl LocalKind {
    /// String form for the `local_kind` JSON field.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Notes => "notes",
            Self::Draft => "draft",
            Self::Bookmark => "bookmark",
            Self::Observation => "observation",
        }
    }
}

/// Hint that the user intends to promote this local item to a wider
/// scope at some point. Not a promotion in itself — the user must
/// take an explicit action via [`promote_payload`] to actually emit
/// the wider-scope Contribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromoteHint {
    /// Target scope the user is considering. One of `community`,
    /// `affiliations`, `species`, `planet`, `federation`.
    pub target_scope: String,
    /// Optional sub_kind the user is considering promoting to. Local
    /// content often morphs (a `notes` draft becomes an
    /// `encyclopedia_article`; an `observation` becomes a `news_article`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_sub_kind: Option<String>,
}

/// Build the canonical `payload` JSON for an accord-data Contribution
/// per SCHEMA.md §4.29.
pub fn build_accord_payload(
    source: &AccordDataSource,
) -> Result<(serde_json::Value, [u8; 32]), IngestError> {
    if source.entity_key_id.is_empty() {
        return Err(IngestError::EmptyKeyId);
    }
    if source.language.is_empty() {
        return Err(IngestError::EmptyLanguage);
    }
    if source.body_bytes.is_empty() {
        return Err(IngestError::EmptyBody);
    }
    if source.signer_key_id.is_empty() {
        return Err(IngestError::EmptySignerKey);
    }
    if source.version_tag.is_empty() {
        return Err(IngestError::EmptyVersionTag);
    }
    if !is_promotable_scope(&source.cohort_scope) {
        return Err(IngestError::UnknownPromotionScope(source.cohort_scope.clone()));
    }
    let sha256 = compute_sha256(&source.body_bytes);

    Ok((
        serde_json::json!({
            "sub_kind": "accord_data",
            "entity_key_id": source.entity_key_id,
            "language": source.language,
            "cohort_scope": source.cohort_scope,
            "content_sha256": hex_encode(&sha256),
            "content_media_type": source.body_media_type,
            "content_size_bytes": source.body_bytes.len(),
            "source": {
                "kind": "accord",
                "accord_kind": source.accord_kind.as_str(),
                "signer_class": source.signer_class.as_str(),
                "signer_key_id": source.signer_key_id,
                "signer_attestation_refs": source.signer_attestation_refs,
                "effective_at": source.effective_at,
                "version_tag": source.version_tag,
            },
            "topical_relations": source.topical_relations.iter().map(|tr| {
                serde_json::json!({
                    "target_key_id": tr.target_key_id,
                    "relation": tr.relation.as_str(),
                })
            }).collect::<Vec<_>>(),
            "citations": source.citations.iter().map(|c| {
                serde_json::json!({
                    "kind": c.kind.as_str(),
                    "ref": c.ref_string,
                })
            }).collect::<Vec<_>>(),
        }),
        sha256,
    ))
}

/// Build the canonical `payload` JSON for a local-data Contribution
/// per SCHEMA.md §4.29. The resulting envelope MUST be wrapped with
/// `cohort_scope: self` per FSD-002 §1.7 — local data is private to
/// the owner_key_id.
pub fn build_local_payload(
    source: &LocalDataSource,
) -> Result<(serde_json::Value, [u8; 32]), IngestError> {
    if source.entity_key_id.is_empty() {
        return Err(IngestError::EmptyKeyId);
    }
    if source.language.is_empty() {
        return Err(IngestError::EmptyLanguage);
    }
    if source.body_bytes.is_empty() {
        return Err(IngestError::EmptyBody);
    }
    if source.owner_key_id.is_empty() {
        return Err(IngestError::EmptyOwnerKey);
    }
    let sha256 = compute_sha256(&source.body_bytes);

    let mut source_block = serde_json::json!({
        "kind": "local",
        "local_kind": source.local_kind.as_str(),
        "owner_key_id": source.owner_key_id,
        "created_at": source.created_at,
    });
    if let Some(ref title) = source.title {
        source_block["title"] = serde_json::Value::String(title.clone());
    }
    if !source.tags.is_empty() {
        source_block["tags"] = serde_json::json!(source.tags);
    }
    if let Some(ref hint) = source.promote_hint {
        source_block["promote_hint"] = serde_json::json!(hint);
    }

    Ok((
        serde_json::json!({
            "sub_kind": "local_data",
            "entity_key_id": source.entity_key_id,
            "language": source.language,
            "cohort_scope": "self",   // local_data is always owner-private
            "content_sha256": hex_encode(&sha256),
            "content_media_type": source.body_media_type,
            "content_size_bytes": source.body_bytes.len(),
            "source": source_block,
            "topical_relations": source.topical_relations.iter().map(|tr| {
                serde_json::json!({
                    "target_key_id": tr.target_key_id,
                    "relation": tr.relation.as_str(),
                })
            }).collect::<Vec<_>>(),
            "citations": source.citations.iter().map(|c| {
                serde_json::json!({
                    "kind": c.kind.as_str(),
                    "ref": c.ref_string,
                })
            }).collect::<Vec<_>>(),
        }),
        sha256,
    ))
}

/// Build the canonical `payload` JSON for a scope-promotion of an
/// existing `external_content` Contribution.
///
/// Takes the prior Contribution's payload + the target scope + an
/// optional sub_kind change, and returns a new payload citing the same
/// `content_sha256` (body not re-uploaded) with `supersedes_payload`
/// set so the federation can walk the promotion chain.
///
/// **Important**: the caller is responsible for wrapping the returned
/// payload in a `ContributionEnvelope` with the new `cohort_scope`
/// (envelope-level field per FSD-002 §1.7). The payload itself does
/// not carry `cohort_scope`.
///
/// The promotion chain is captured at TWO levels:
/// - **Envelope level**: the new Contribution's `supersedes` field
///   references the prior Contribution's ID (caller sets this when
///   constructing the envelope).
/// - **Payload level**: `supersedes_payload.prior_contribution_id`
///   carries the same reference for downstream consumers that parse
///   just the payload.
///
/// Sub_kind morphing is supported: a `local_data` `notes` item can be
/// promoted to a community-scope `encyclopedia_article` or a global-
/// scope `news_article` simply by setting `new_sub_kind`.
pub fn promote_payload(
    prior_payload: &serde_json::Value,
    prior_contribution_id: &str,
    new_sub_kind: Option<&str>,
    new_target_scope: &str,
) -> Result<serde_json::Value, IngestError> {
    let mut promoted = prior_payload.clone();

    if !is_promotable_scope(new_target_scope) {
        return Err(IngestError::UnknownPromotionScope(new_target_scope.to_owned()));
    }

    if let Some(sk) = new_sub_kind {
        if !is_promotable_sub_kind(sk) {
            return Err(IngestError::UnknownSubKind(sk.to_owned()));
        }
        promoted["sub_kind"] = serde_json::Value::String(sk.to_owned());
    }

    // Update cohort_scope to the new target scope — the whole point
    // of a promotion is to widen the visibility tier.
    promoted["cohort_scope"] = serde_json::Value::String(new_target_scope.to_owned());

    promoted["supersedes_payload"] = serde_json::json!({
        "prior_contribution_id": prior_contribution_id,
        "new_target_scope": new_target_scope,
    });

    // Promote-hint, if present from a local_data source, becomes stale
    // once the actual promotion happens — strip it.
    if let Some(source_obj) = promoted.get_mut("source").and_then(|v| v.as_object_mut()) {
        source_obj.remove("promote_hint");
    }

    Ok(promoted)
}

/// Returns `true` for cohort_scope values consumers consider valid
/// promotion targets. Per FSD-002 §1.7 (and matching MISSION.md P12
/// scale enumeration).
pub fn is_promotable_scope(scope: &str) -> bool {
    matches!(
        scope,
        "self"
            | "family"
            | "community"
            | "affiliations"
            | "species"
            | "planet"
            | "federation"
    )
}

/// Returns `true` for sub_kind values consumers consider valid
/// promotion targets within `external_content`.
pub fn is_promotable_sub_kind(sub_kind: &str) -> bool {
    matches!(
        sub_kind,
        "encyclopedia_article" | "news_article" | "accord_data" | "local_data"
    )
}

/// Errors from payload construction. Validation is minimal — the
/// envelope discipline (signature + canonical encoding) is enforced at
/// signing time, separately.
#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    /// `entity_key_id` was empty.
    #[error("entity_key_id must be non-empty")]
    EmptyKeyId,
    /// `language` was empty.
    #[error("language must be non-empty (ISO 639-1)")]
    EmptyLanguage,
    /// `body_bytes` was empty.
    #[error("body_bytes must be non-empty")]
    EmptyBody,
    /// News articles require a non-empty headline.
    #[error("headline must be non-empty for news_article")]
    EmptyHeadline,
    /// Accord-data requires a non-empty signer_key_id (the canonical
    /// attester for the multi-sig group).
    #[error("signer_key_id must be non-empty for accord_data")]
    EmptySignerKey,
    /// Accord-data requires a non-empty version_tag.
    #[error("version_tag must be non-empty for accord_data")]
    EmptyVersionTag,
    /// Local-data requires a non-empty owner_key_id.
    #[error("owner_key_id must be non-empty for local_data")]
    EmptyOwnerKey,
    /// Promotion target scope is not in the recognized
    /// cohort_scope enumeration.
    #[error("unknown promotion scope: {0}")]
    UnknownPromotionScope(String),
    /// Promotion target sub_kind is not in the recognized
    /// external_content sub_kind enumeration.
    #[error("unknown sub_kind: {0}")]
    UnknownSubKind(String),
}

fn compute_sha256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Lowercase hex encoding for the 32-byte SHA-256 — content_sha256
/// wire format per SCHEMA §4.29. Tiny inline impl to avoid pulling
/// in the `hex` crate for one call site.
fn hex_encode(bytes: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for b in bytes {
        out.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        out.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_dt(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn encyclopedia_payload_minimal() {
        let src = EncyclopediaArticleSource {
            entity_key_id: "wikipedia:article:einstein".into(),
            language: "en".into(),
            body_bytes: b"<p>Albert Einstein...</p>".to_vec(),
            body_media_type: "text/html".into(),
            project: "wikipedia".into(),
            revision_id: "1234567".into(),
            edited_at: parse_dt("2026-05-15T12:34:56Z"),
            cohort_scope: "federation".into(),
            topical_relations: vec![],
            citations: vec![],
        };
        let (payload, _sha) = build_encyclopedia_payload(&src).unwrap();
        assert_eq!(payload["sub_kind"], "encyclopedia_article");
        assert_eq!(payload["entity_key_id"], "wikipedia:article:einstein");
        assert_eq!(payload["language"], "en");
        assert_eq!(payload["content_media_type"], "text/html");
        assert_eq!(payload["content_size_bytes"], 25);
        assert_eq!(payload["source"]["project"], "wikipedia");
        assert_eq!(payload["source"]["revision_id"], "1234567");
        assert!(payload["content_sha256"].as_str().unwrap().len() == 64);
    }

    #[test]
    fn encyclopedia_payload_with_links_and_citations() {
        let src = EncyclopediaArticleSource {
            entity_key_id: "wikipedia:article:einstein".into(),
            language: "en".into(),
            body_bytes: b"body".to_vec(),
            body_media_type: "text/html".into(),
            project: "wikipedia".into(),
            revision_id: "1234567".into(),
            edited_at: parse_dt("2026-05-15T12:34:56Z"),
            cohort_scope: "federation".into(),
            topical_relations: vec![
                TopicalRelation {
                    target_key_id: "wikipedia:article:relativity".into(),
                    relation: TopicalRelationKind::References,
                },
                TopicalRelation {
                    target_key_id: "wikipedia:article:nobel_prize".into(),
                    relation: TopicalRelationKind::SeeAlso,
                },
            ],
            citations: vec![
                Citation {
                    kind: CitationKind::Doi,
                    ref_string: "10.1103/PhysRevA.123.456".into(),
                },
                Citation {
                    kind: CitationKind::ExternalUrl,
                    ref_string: "https://nobelprize.org/.../einstein".into(),
                },
            ],
        };
        let (payload, _sha) = build_encyclopedia_payload(&src).unwrap();
        let trs = payload["topical_relations"].as_array().unwrap();
        assert_eq!(trs.len(), 2);
        assert_eq!(trs[0]["relation"], "references");
        assert_eq!(trs[1]["relation"], "see_also");

        let citations = payload["citations"].as_array().unwrap();
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0]["kind"], "doi");
        assert_eq!(citations[1]["kind"], "external_url");
    }

    #[test]
    fn news_payload_with_byline_and_valid_until() {
        let src = NewsArticleSource {
            entity_key_id: "news:article:nyt:2026-05-15:climate-summit".into(),
            language: "en".into(),
            body_bytes: b"<article>Summit news...</article>".to_vec(),
            body_media_type: "text/html".into(),
            publisher: "nyt".into(),
            publisher_key_id: "publisher:nyt".into(),
            published_at: parse_dt("2026-05-15T08:00:00Z"),
            byline: Some("Jane Doe".into()),
            byline_key_id: Some("journalist:jane-doe".into()),
            section: Some("world".into()),
            headline: "Climate summit reaches framework agreement".into(),
            cohort_scope: "federation".into(),
            topical_relations: vec![TopicalRelation {
                target_key_id: "news:article:nyt:2026-05-14:climate-summit-day-1".into(),
                relation: TopicalRelationKind::SeeAlso,
            }],
            citations: vec![Citation {
                kind: CitationKind::ExternalUrl,
                ref_string: "https://unfccc.int/.../press-release-2026-05-15".into(),
            }],
            valid_until: Some(parse_dt("2027-05-15T08:00:00Z")),
        };
        let (payload, _sha) = build_news_payload(&src).unwrap();
        assert_eq!(payload["sub_kind"], "news_article");
        assert_eq!(payload["source"]["publisher"], "nyt");
        assert_eq!(payload["source"]["byline"], "Jane Doe");
        assert_eq!(payload["source"]["section"], "world");
        assert_eq!(
            payload["source"]["headline"],
            "Climate summit reaches framework agreement"
        );
        assert!(payload["valid_until"].is_string());
    }

    #[test]
    fn news_payload_without_optional_byline_omits_byline_field() {
        let src = NewsArticleSource {
            entity_key_id: "news:article:ap:2026-05-15:wire-piece".into(),
            language: "en".into(),
            body_bytes: b"wire copy".to_vec(),
            body_media_type: "text/plain".into(),
            publisher: "ap".into(),
            publisher_key_id: "publisher:ap".into(),
            published_at: parse_dt("2026-05-15T08:00:00Z"),
            byline: None,
            byline_key_id: None,
            section: None,
            headline: "Wire piece".into(),
            cohort_scope: "federation".into(),
            topical_relations: vec![],
            citations: vec![],
            valid_until: None,
        };
        let (payload, _sha) = build_news_payload(&src).unwrap();
        assert!(payload["source"].get("byline").is_none());
        assert!(payload["source"].get("section").is_none());
        assert!(payload.get("valid_until").is_none());
    }

    #[test]
    fn rejects_empty_required_fields() {
        let mut src = EncyclopediaArticleSource {
            entity_key_id: "".into(),
            language: "en".into(),
            body_bytes: b"body".to_vec(),
            body_media_type: "text/html".into(),
            project: "wikipedia".into(),
            revision_id: "1234".into(),
            edited_at: parse_dt("2026-05-15T12:34:56Z"),
            cohort_scope: "federation".into(),
            topical_relations: vec![],
            citations: vec![],
        };
        assert!(matches!(
            build_encyclopedia_payload(&src),
            Err(IngestError::EmptyKeyId)
        ));

        src.entity_key_id = "ok".into();
        src.language = "".into();
        assert!(matches!(
            build_encyclopedia_payload(&src),
            Err(IngestError::EmptyLanguage)
        ));

        src.language = "en".into();
        src.body_bytes = vec![];
        assert!(matches!(
            build_encyclopedia_payload(&src),
            Err(IngestError::EmptyBody)
        ));
    }

    #[test]
    fn accord_payload_with_humanity_accord_signer() {
        let src = AccordDataSource {
            entity_key_id: "accord:canonical_text:accord_v1.2".into(),
            language: "en".into(),
            body_bytes: b"# ACCORD v1.2\n...".to_vec(),
            body_media_type: "text/markdown".into(),
            accord_kind: AccordKind::CanonicalText,
            signer_class: AccordSignerClass::HumanityAccord,
            signer_key_id: "accord-eric-moore".into(),
            signer_attestation_refs: vec![
                "att-accord-eric-kudzin-001".into(),
                "att-accord-haley-bradley-001".into(),
            ],
            effective_at: parse_dt("2026-05-15T00:00:00Z"),
            version_tag: "v1.2".into(),
            cohort_scope: "species".into(),
            topical_relations: vec![],
            citations: vec![],
        };
        let (payload, _sha) = build_accord_payload(&src).unwrap();
        assert_eq!(payload["sub_kind"], "accord_data");
        assert_eq!(payload["source"]["kind"], "accord");
        assert_eq!(payload["source"]["accord_kind"], "canonical_text");
        assert_eq!(payload["source"]["signer_class"], "humanity_accord");
        assert_eq!(payload["source"]["signer_key_id"], "accord-eric-moore");
        assert_eq!(
            payload["source"]["signer_attestation_refs"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(payload["source"]["version_tag"], "v1.2");
    }

    #[test]
    fn accord_rejects_empty_signer_key() {
        let src = AccordDataSource {
            entity_key_id: "accord:policy_declaration:rate_limit_update".into(),
            language: "en".into(),
            body_bytes: b"body".to_vec(),
            body_media_type: "text/markdown".into(),
            accord_kind: AccordKind::PolicyDeclaration,
            signer_class: AccordSignerClass::OneOfSix,
            signer_key_id: "".into(),
            signer_attestation_refs: vec![],
            effective_at: parse_dt("2026-05-15T00:00:00Z"),
            version_tag: "v1".into(),
            cohort_scope: "federation".into(),
            topical_relations: vec![],
            citations: vec![],
        };
        assert!(matches!(
            build_accord_payload(&src),
            Err(IngestError::EmptySignerKey)
        ));
    }

    #[test]
    fn local_payload_with_full_metadata() {
        let src = LocalDataSource {
            entity_key_id: "local:abc123:notes:research-notes-2026-05".into(),
            language: "en".into(),
            body_bytes: b"My private research notes...".to_vec(),
            body_media_type: "text/markdown".into(),
            local_kind: LocalKind::Notes,
            owner_key_id: "user-key-abc123".into(),
            created_at: parse_dt("2026-05-28T14:00:00Z"),
            title: Some("Research notes on coherence".into()),
            tags: vec!["coherence".into(), "research".into()],
            promote_hint: Some(PromoteHint {
                target_scope: "community".into(),
                target_sub_kind: Some("encyclopedia_article".into()),
            }),
            topical_relations: vec![],
            citations: vec![],
        };
        let (payload, _sha) = build_local_payload(&src).unwrap();
        assert_eq!(payload["sub_kind"], "local_data");
        assert_eq!(payload["source"]["kind"], "local");
        assert_eq!(payload["source"]["local_kind"], "notes");
        assert_eq!(payload["source"]["owner_key_id"], "user-key-abc123");
        assert_eq!(payload["source"]["title"], "Research notes on coherence");
        assert_eq!(
            payload["source"]["tags"].as_array().unwrap().len(),
            2
        );
        assert_eq!(payload["source"]["promote_hint"]["target_scope"], "community");
    }

    #[test]
    fn local_payload_omits_optional_fields() {
        let src = LocalDataSource {
            entity_key_id: "local:abc123:bookmark:url-1".into(),
            language: "en".into(),
            body_bytes: b"bookmark".to_vec(),
            body_media_type: "text/plain".into(),
            local_kind: LocalKind::Bookmark,
            owner_key_id: "user-key-abc123".into(),
            created_at: parse_dt("2026-05-28T14:00:00Z"),
            title: None,
            tags: vec![],
            promote_hint: None,
            topical_relations: vec![],
            citations: vec![],
        };
        let (payload, _) = build_local_payload(&src).unwrap();
        assert!(payload["source"].get("title").is_none());
        assert!(payload["source"].get("tags").is_none());
        assert!(payload["source"].get("promote_hint").is_none());
    }

    #[test]
    fn promote_local_to_community_encyclopedia() {
        // User authors local notes; promotes to community encyclopedia.
        let local_src = LocalDataSource {
            entity_key_id: "local:abc123:draft:einstein-notes".into(),
            language: "en".into(),
            body_bytes: b"Einstein draft article body...".to_vec(),
            body_media_type: "text/markdown".into(),
            local_kind: LocalKind::Draft,
            owner_key_id: "user-key-abc123".into(),
            created_at: parse_dt("2026-05-28T14:00:00Z"),
            title: Some("Einstein draft".into()),
            tags: vec![],
            promote_hint: Some(PromoteHint {
                target_scope: "community".into(),
                target_sub_kind: Some("encyclopedia_article".into()),
            }),
            topical_relations: vec![],
            citations: vec![],
        };
        let (local_payload, _local_sha) = build_local_payload(&local_src).unwrap();

        let promoted = promote_payload(
            &local_payload,
            "contribution-id-local-001",
            Some("encyclopedia_article"),
            "community",
        )
        .unwrap();

        assert_eq!(promoted["sub_kind"], "encyclopedia_article");
        // Same content_sha256 — body not re-uploaded.
        assert_eq!(promoted["content_sha256"], local_payload["content_sha256"]);
        assert_eq!(
            promoted["supersedes_payload"]["prior_contribution_id"],
            "contribution-id-local-001"
        );
        assert_eq!(
            promoted["supersedes_payload"]["new_target_scope"],
            "community"
        );
        // promote_hint stripped post-promotion (no longer relevant)
        assert!(promoted["source"].get("promote_hint").is_none());
    }

    #[test]
    fn promote_rejects_unknown_scope() {
        let payload = serde_json::json!({"sub_kind": "local_data", "entity_key_id": "x"});
        assert!(matches!(
            promote_payload(&payload, "contrib-1", None, "intergalactic"),
            Err(IngestError::UnknownPromotionScope(_))
        ));
    }

    #[test]
    fn promote_rejects_unknown_sub_kind() {
        let payload = serde_json::json!({"sub_kind": "local_data"});
        assert!(matches!(
            promote_payload(
                &payload,
                "contrib-1",
                Some("encyclopedia_video"),
                "community"
            ),
            Err(IngestError::UnknownSubKind(_))
        ));
    }

    #[test]
    fn promote_preserves_sub_kind_when_not_specified() {
        // User promotes their local notes to community scope WITHOUT
        // morphing the sub_kind — still local_data, just visible to
        // community.
        let payload = serde_json::json!({
            "sub_kind": "local_data",
            "entity_key_id": "local:abc123:notes:1",
            "content_sha256": "abc",
        });
        let promoted = promote_payload(&payload, "c-1", None, "community").unwrap();
        assert_eq!(promoted["sub_kind"], "local_data");   // unchanged
        assert_eq!(promoted["supersedes_payload"]["new_target_scope"], "community");
    }

    #[test]
    fn sha256_matches_known_hash() {
        // SHA-256 of b"abc" — well-known test vector
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let src = EncyclopediaArticleSource {
            entity_key_id: "test".into(),
            language: "en".into(),
            body_bytes: b"abc".to_vec(),
            body_media_type: "text/plain".into(),
            project: "test".into(),
            revision_id: "1".into(),
            edited_at: parse_dt("2026-05-15T12:34:56Z"),
            cohort_scope: "federation".into(),
            topical_relations: vec![],
            citations: vec![],
        };
        let (payload, sha) = build_encyclopedia_payload(&src).unwrap();
        assert_eq!(payload["content_sha256"], expected);
        assert_eq!(hex_encode(&sha), expected);
    }
}
