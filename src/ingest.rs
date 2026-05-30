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
#[allow(missing_docs)]
#[allow(missing_docs)]
#[allow(missing_docs)]
#[allow(missing_docs)]
#[allow(missing_docs)]
#[allow(missing_docs)]
#[allow(missing_docs)]
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
    /// Chat reply — this message replies to a prior message in a
    /// conversation thread.
    RepliesTo,
    /// Blog comment — this Contribution is a comment on a prior
    /// blog post (or a reply to a prior comment).
    CommentsOn,
    /// Quote / excerpt — this content quotes a specific span of the
    /// target. Distinct from `References` (which is a soft mention).
    Quotes,
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
            Self::RepliesTo => "replies_to",
            Self::CommentsOn => "comments_on",
            Self::Quotes => "quotes",
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

/// Chat-message source. One message in a conversational thread —
/// imported from Discord / Slack / Twitter / iMessage / SMS / XMPP /
/// IRC / etc. Each message is a Contribution; reply chains form via
/// `RepliesTo` topical_relations referencing prior messages.
///
/// Chat tends toward narrower cohort_scope than articles —
/// `community` for group channels, `family` for household chat, `self`
/// for DMs the user wants only their own runtime to see. Privacy
/// sensitivity is higher than articles; consumer policy should
/// downweight chat in cross-cohort aggregation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageSource {
    /// Stable key_id. Pattern:
    /// `chat:{platform}:{conversation_id}:{message_id}` — e.g.
    /// `chat:discord:123-456:789012345678`. The (platform,
    /// conversation_id, message_id) triple makes messages uniquely
    /// identifiable across imports.
    pub entity_key_id: String,
    /// ISO 639-1 language code (best-effort detection for chat).
    pub language: String,
    /// The message body bytes (the raw text or rich-content payload).
    pub body_bytes: Vec<u8>,
    /// RFC 6838 media type. Typical: `text/plain` (plain chat),
    /// `text/markdown` (formatted), `application/json` (rich
    /// embed-bearing).
    pub body_media_type: String,
    /// Chat platform tag (e.g. `discord`, `slack`, `twitter`,
    /// `imessage`, `sms`, `xmpp`, `irc`, `matrix`).
    pub platform: String,
    /// Conversation / thread / channel / DM identifier on the source
    /// platform. Multiple messages in the same conversation share
    /// this; consumers group by it for thread reconstruction.
    pub conversation_id: String,
    /// Platform-specific message identifier (Discord snowflake,
    /// Twitter status ID, etc.). Distinct from the federation
    /// `entity_key_id` which composes platform + conversation + this.
    pub message_id: String,
    /// Sender's display handle on the source platform.
    pub sender_handle: String,
    /// Optional federation key_id for the sender, when known + when
    /// the sender has a federation identity bridged to their chat
    /// handle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_key_id: Option<String>,
    /// When the message was sent on the source platform.
    pub sent_at: DateTime<Utc>,
    /// Sequence index within the conversation (0-based). Helps
    /// consumers order messages even when source timestamps are
    /// imprecise.
    pub message_index: u64,
    /// Cohort scope at which this message is being asserted (per
    /// [`scope`] module). Typically `family` (household chat),
    /// `community` (group channels), `affiliations` (professional
    /// chats), or `self` (DMs).
    pub cohort_scope: String,
    /// Topical relations. Use `RepliesTo` to point at the prior
    /// message in the thread (the federation `entity_key_id` of the
    /// message being replied to).
    #[serde(default)]
    pub topical_relations: Vec<TopicalRelation>,
    /// External citations referenced in the message body.
    #[serde(default)]
    pub citations: Vec<Citation>,
    /// Optional staleness contract. Chat can decay quickly (e.g.
    /// minutes for live-conversation channels) or stay indefinitely
    /// (e.g. permanent archive). Per-deployment policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<DateTime<Utc>>,
}

/// Blog-post source. Single-author published commentary / opinion /
/// long-form writing — imported from Medium / Substack / personal
/// blogs / Tumblr / LiveJournal / etc. Distinct from news (no
/// publisher editorial), distinct from encyclopedia (no
/// peer-consensus editing), distinct from chat (long-form, slower).
///
/// Comments on blog posts are themselves Contributions citing the
/// blog post via `CommentsOn` topical_relation. Reply chains within
/// comments use `RepliesTo`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogPostSource {
    /// Stable key_id. Pattern: `blog:{platform}:{blog_id}:{post_slug}` —
    /// e.g. `blog:medium:@ericmoore:on-coherence-2026-05-15`.
    pub entity_key_id: String,
    /// ISO 639-1 language code.
    pub language: String,
    /// The post body bytes.
    pub body_bytes: Vec<u8>,
    /// RFC 6838 media type. Typical: `text/markdown`, `text/html`,
    /// `application/x-mdx`.
    pub body_media_type: String,
    /// Blog platform tag (e.g. `medium`, `substack`, `wordpress`,
    /// `ghost`, `personal`, `tumblr`).
    pub platform: String,
    /// Blog identifier on the source platform (the BLOG itself, not
    /// the post — e.g. `@ericmoore` for a Medium handle,
    /// `example-blog` for a Substack subdomain).
    pub blog_id: String,
    /// Author's display handle on the source platform.
    pub author_handle: String,
    /// Optional federation key_id for the author, when known + when
    /// the author has a federation identity bridged to their blog
    /// handle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_key_id: Option<String>,
    /// When the post was published on the source platform.
    pub published_at: DateTime<Utc>,
    /// Post title.
    pub post_title: String,
    /// Canonical URL of the post on the source platform (for
    /// round-trip reference).
    pub post_url: String,
    /// User-supplied tags / categories from the source platform.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Cohort scope at which this post is being asserted (per [`scope`]
    /// module). Typical: `federation` (public blog), `community`
    /// (community-internal blog), `affiliations` (org-internal posts).
    pub cohort_scope: String,
    /// Topical relations. Posts may reference other posts /
    /// encyclopedia entries / news articles.
    #[serde(default)]
    pub topical_relations: Vec<TopicalRelation>,
    /// External citations.
    #[serde(default)]
    pub citations: Vec<Citation>,
    /// Optional staleness contract. Blog posts typically leave this
    /// unset (long shelf life) unless explicitly time-bound (e.g. a
    /// time-sensitive announcement post).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<DateTime<Utc>>,
}

// ───────────────────────────────────────────────────────────────────────
// Multimedia sub_kinds (FSD/MEDIA_SHARING.md §2.1-2.5)
//
// Five additions extending external_content with image / audio / video /
// film / model_3d sub_kinds. Each follows the same Phase 2B pattern as
// the six text-class sub_kinds: Source struct → pure build_*_payload →
// async ingest_* function using the shared finalize_external_content_ingest
// tail. No new substrate primitives.
// ───────────────────────────────────────────────────────────────────────

/// Multi-scheme content rating attestation declaration.
/// Per FSD/MEDIA_SHARING.md §3.1. Multiple ratings from different
/// schemes can coexist on the same content; consumer algorithms pick
/// which scheme to honor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ContentRating {
    /// The rating scheme name: `mpaa`, `bbfc`, `pegi`, `esrb`, `ifco`,
    /// `csm` (Common Sense Media), or operator-defined.
    pub scheme: String,
    /// The rating value within that scheme: e.g. `PG-13`, `R`, `NC-17`
    /// for MPAA; `12A`, `18`, `R18` for BBFC; etc.
    pub rating: String,
}

/// Mechanism-descriptive declaration of *what kind of content this is*.
/// Per FSD/MEDIA_SHARING.md §3.3. Multi-class content allowed (a
/// documentary-art-piece carries both).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum ContentClass {
    Film,
    ShortFilm,
    Documentary,
    ArtPiece,
    Theatre,
    Performance,
    Animation,
    Experimental,
    Educational,
    Tutorial,
    Lecture,
    Talk,
    News,
    CurrentEvents,
    Journalism,
    Entertainment,
    Vlog,
    SocialVideo,
    Gameplay,
    Commentary,
    Adult,
    Generated,
    Photograph,
    Illustration,
    Screenshot,
    Meme,
    Infographic,
    Music,
    Podcast,
    Audiobook,
    Soundscape,
    StaticObject,
    Scene,
    Character,
    VolumetricCapture,
}

impl ContentClass {
    /// True if the class can carry R/X-rated content at species+
    /// scope per FSD/MEDIA_SHARING.md §3.3.
    pub fn art_class(&self) -> bool {
        matches!(
            self,
            Self::Film | Self::ShortFilm | Self::Documentary | Self::ArtPiece
                | Self::Theatre | Self::Performance | Self::Animation | Self::Experimental
        )
    }
    /// True if the class is news-editorial-framing (current-events
    /// path for adult-relevance material).
    pub fn news_class(&self) -> bool {
        matches!(self, Self::News | Self::CurrentEvents | Self::Journalism)
    }
}

/// Image source — photo / illustration / screenshot / meme / infographic.
/// FSD/MEDIA_SHARING.md §2.1.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ImageSource {
    pub entity_key_id: String,
    pub body_bytes: Vec<u8>,
    /// MIME type: image/jpeg, image/png, image/webp, image/avif,
    /// image/svg+xml, image/gif.
    pub body_media_type: String,
    pub cohort_scope: String,
    /// Multi-scheme rating declarations. At least one required for
    /// federation-scope publication.
    #[serde(default)]
    pub content_rating: Vec<ContentRating>,
    /// Content classification. Required for federation-scope.
    pub content_class: Option<ContentClass>,
    /// Pixel dimensions.
    pub width_px: u32,
    pub height_px: u32,
    /// Alt-text. Mandatory for federation scope (accessibility).
    pub alt_text: String,
    pub captured_at: Option<DateTime<Utc>>,
    /// Creator's federation key_id (if known + different from
    /// federation submitter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator_key_id: Option<String>,
    /// AI-generation declaration. EU AI Act Article 50 mandatory.
    #[serde(default)]
    pub is_ai_generated: bool,
    /// AI model name (when is_ai_generated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_by: Option<String>,
    #[serde(default)]
    pub topical_relations: Vec<TopicalRelation>,
    #[serde(default)]
    pub citations: Vec<Citation>,
}

/// Audio source — music, podcast, lecture, audiobook chapter,
/// sound sample, generated audio. FSD/MEDIA_SHARING.md §2.2.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioSource {
    pub entity_key_id: String,
    pub body_bytes: Vec<u8>,
    /// MIME type: audio/opus, audio/mpeg, audio/flac, audio/aac,
    /// audio/ogg.
    pub body_media_type: String,
    pub cohort_scope: String,
    #[serde(default)]
    pub content_rating: Vec<ContentRating>,
    pub content_class: Option<ContentClass>,
    /// Duration in seconds.
    pub duration_seconds: f64,
    pub sample_rate_hz: Option<u32>,
    pub bit_rate_kbps: Option<u32>,
    /// Transcript text. Required for cohort_scope ≥ community
    /// (accessibility).
    pub transcript: String,
    /// ISO 639-1 language of the audio + transcript.
    pub language: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator_key_id: Option<String>,
    /// License: cc0 / cc-by / cc-by-sa / cc-by-nc / proprietary /
    /// public_domain.
    pub license: String,
    #[serde(default)]
    pub is_ai_generated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_by: Option<String>,
    #[serde(default)]
    pub topical_relations: Vec<TopicalRelation>,
    #[serde(default)]
    pub citations: Vec<Citation>,
}

/// Video source — general video (vlog, tutorial, social, gameplay,
/// screen recording, talking head). For cinema/art-bearing video,
/// use FilmSource. FSD/MEDIA_SHARING.md §2.3.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct VideoSource {
    pub entity_key_id: String,
    pub body_bytes: Vec<u8>,
    /// MIME type: video/mp4, video/webm, video/x-matroska, video/av1.
    pub body_media_type: String,
    pub cohort_scope: String,
    #[serde(default)]
    pub content_rating: Vec<ContentRating>,
    pub content_class: Option<ContentClass>,
    /// Duration in seconds.
    pub duration_seconds: f64,
    /// Frame dimensions.
    pub width_px: u32,
    pub height_px: u32,
    pub frame_rate: Option<f64>,
    /// Captions text + format. Mandatory for cohort_scope ≥ community.
    pub captions: String,
    /// ISO 639-1 language(s) of the captions.
    pub language: String,
    /// SHA-256 hex of the thumbnail blob (separate Contribution).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator_key_id: Option<String>,
    pub license: String,
    #[serde(default)]
    pub is_ai_generated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_by: Option<String>,
    #[serde(default)]
    pub topical_relations: Vec<TopicalRelation>,
    #[serde(default)]
    pub citations: Vec<Citation>,
}

/// Film source — cinematic / art-bearing video. Full-length cinema,
/// short films, documentaries, theatre recordings, performance art.
/// Distinguished from general `video` by the content_class +
/// distributor attestation chain that adjudicates the art-bearing
/// nature. FSD/MEDIA_SHARING.md §2.4.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct FilmSource {
    pub entity_key_id: String,
    pub body_bytes: Vec<u8>,
    pub body_media_type: String,
    pub cohort_scope: String,
    #[serde(default)]
    pub content_rating: Vec<ContentRating>,
    /// MANDATORY for film sub_kind. Must be art-bearing (art_class()).
    pub content_class: ContentClass,
    pub duration_seconds: f64,
    pub width_px: u32,
    pub height_px: u32,
    pub frame_rate: Option<f64>,
    /// Captions mandatory.
    pub captions: String,
    pub language: String,
    pub languages_available: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_sha256: Option<String>,
    /// Distributor federation key (Disney / A24 / Criterion / Studio
    /// Ghibli / a community film festival / etc.). Drives the
    /// trust-graph adjudication of the art-claim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distributor_key_id: Option<String>,
    /// External canonical IDs for cross-reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imdb_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tmdb_id: Option<String>,
    pub release_year: Option<u16>,
    /// Production credits — director, writer, cinematographer, etc.
    /// JSON object freeform; structure per industry convention.
    #[serde(default)]
    pub production_credits: serde_json::Value,
    pub license: String,
    #[serde(default)]
    pub is_ai_generated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_by: Option<String>,
    #[serde(default)]
    pub topical_relations: Vec<TopicalRelation>,
    #[serde(default)]
    pub citations: Vec<Citation>,
}

/// 3D content source. Static models, scenes, volumetric video,
/// scanned environments, character rigs.
/// FSD/MEDIA_SHARING.md §2.5.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct Model3dSource {
    pub entity_key_id: String,
    pub body_bytes: Vec<u8>,
    /// MIME type: model/gltf+json, model/gltf-binary, model/vnd.usdz+zip,
    /// model/obj, model/ply, application/x-gaussian-splat (proposed).
    pub body_media_type: String,
    pub cohort_scope: String,
    #[serde(default)]
    pub content_rating: Vec<ContentRating>,
    pub content_class: Option<ContentClass>,
    /// Vertex count (for mesh-based formats).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertex_count: Option<u64>,
    /// Triangle count (for mesh-based formats).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triangle_count: Option<u64>,
    /// Includes animations.
    #[serde(default)]
    pub has_animations: bool,
    /// Highest texture resolution (longest side in pixels).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_texture_resolution: Option<u32>,
    /// Intended renderer class: webgl / webgpu / vr / ar / mobile /
    /// desktop / cinema.
    pub intended_renderer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator_key_id: Option<String>,
    pub license: String,
    #[serde(default)]
    pub is_ai_generated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_by: Option<String>,
    #[serde(default)]
    pub topical_relations: Vec<TopicalRelation>,
    #[serde(default)]
    pub citations: Vec<Citation>,
}

/// Shared validation hook for multimedia sources at federation scope.
/// Per FSD/MEDIA_SHARING.md §3.3 + §8 (EU AI Act Art. 50).
fn validate_multimedia_federation_constraints(
    cohort_scope: &str,
    content_class: Option<&ContentClass>,
    is_ai_generated: bool,
    has_authenticity_declaration: bool,
) -> Result<(), IngestError> {
    let is_federation_scope = matches!(
        cohort_scope,
        "community" | "affiliations" | "species" | "planet" | "federation"
    );
    if is_federation_scope && content_class.is_none() {
        return Err(IngestError::MissingContentClass);
    }
    if is_ai_generated && !has_authenticity_declaration && is_federation_scope {
        return Err(IngestError::UndisclosedAiGenerated);
    }
    Ok(())
}

/// Serialize ContentRating attestations into a JSON array for
/// inclusion in payload.
fn serialize_content_ratings(ratings: &[ContentRating]) -> Vec<serde_json::Value> {
    ratings
        .iter()
        .map(|r| serde_json::json!({ "scheme": r.scheme, "rating": r.rating }))
        .collect()
}

/// Build the canonical `payload` JSON for an image Contribution.
pub fn build_image_payload(
    source: &ImageSource,
) -> Result<(serde_json::Value, [u8; 32]), IngestError> {
    if source.entity_key_id.is_empty() {
        return Err(IngestError::EmptyKeyId);
    }
    if source.body_bytes.is_empty() {
        return Err(IngestError::EmptyBody);
    }
    if !is_promotable_scope(&source.cohort_scope) {
        return Err(IngestError::UnknownPromotionScope(source.cohort_scope.clone()));
    }
    let is_federation_scope = matches!(
        source.cohort_scope.as_str(),
        "community" | "affiliations" | "species" | "planet" | "federation"
    );
    if is_federation_scope && source.alt_text.trim().is_empty() {
        return Err(IngestError::MissingAccessibilityText);
    }
    validate_multimedia_federation_constraints(
        &source.cohort_scope,
        source.content_class.as_ref(),
        source.is_ai_generated,
        source.generated_by.is_some(), // self-declares authenticity:ai_generated
    )?;
    let sha256 = compute_sha256(&source.body_bytes);

    let mut payload = serde_json::json!({
        "sub_kind": "image",
        "entity_key_id": source.entity_key_id,
        "cohort_scope": source.cohort_scope,
        "content_sha256": hex_encode(&sha256),
        "content_media_type": source.body_media_type,
        "content_size_bytes": source.body_bytes.len(),
        "width_px": source.width_px,
        "height_px": source.height_px,
        "alt_text": source.alt_text,
        "content_rating": serialize_content_ratings(&source.content_rating),
        "is_ai_generated": source.is_ai_generated,
        "topical_relations": source.topical_relations.iter().map(|tr| serde_json::json!({
            "target_key_id": tr.target_key_id,
            "relation": tr.relation.as_str(),
        })).collect::<Vec<_>>(),
        "citations": source.citations.iter().map(|c| serde_json::json!({
            "kind": c.kind.as_str(),
            "ref": c.ref_string,
        })).collect::<Vec<_>>(),
    });
    if let Some(cc) = source.content_class {
        payload["content_class"] = serde_json::json!(cc);
    }
    if let Some(t) = source.captured_at {
        payload["captured_at"] = serde_json::json!(t);
    }
    if let Some(ref ck) = source.creator_key_id {
        payload["creator_key_id"] = serde_json::Value::String(ck.clone());
    }
    if let Some(ref g) = source.generated_by {
        payload["generated_by"] = serde_json::Value::String(g.clone());
    }
    Ok((payload, sha256))
}

/// Build the canonical `payload` JSON for an audio Contribution.
pub fn build_audio_payload(
    source: &AudioSource,
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
    if !is_promotable_scope(&source.cohort_scope) {
        return Err(IngestError::UnknownPromotionScope(source.cohort_scope.clone()));
    }
    let is_federation_scope = matches!(
        source.cohort_scope.as_str(),
        "community" | "affiliations" | "species" | "planet" | "federation"
    );
    if is_federation_scope && source.transcript.trim().is_empty() {
        return Err(IngestError::MissingAccessibilityText);
    }
    validate_multimedia_federation_constraints(
        &source.cohort_scope,
        source.content_class.as_ref(),
        source.is_ai_generated,
        source.generated_by.is_some(),
    )?;
    let sha256 = compute_sha256(&source.body_bytes);

    let mut payload = serde_json::json!({
        "sub_kind": "audio",
        "entity_key_id": source.entity_key_id,
        "language": source.language,
        "cohort_scope": source.cohort_scope,
        "content_sha256": hex_encode(&sha256),
        "content_media_type": source.body_media_type,
        "content_size_bytes": source.body_bytes.len(),
        "duration_seconds": source.duration_seconds,
        "transcript": source.transcript,
        "license": source.license,
        "content_rating": serialize_content_ratings(&source.content_rating),
        "is_ai_generated": source.is_ai_generated,
        "topical_relations": source.topical_relations.iter().map(|tr| serde_json::json!({
            "target_key_id": tr.target_key_id,
            "relation": tr.relation.as_str(),
        })).collect::<Vec<_>>(),
        "citations": source.citations.iter().map(|c| serde_json::json!({
            "kind": c.kind.as_str(),
            "ref": c.ref_string,
        })).collect::<Vec<_>>(),
    });
    if let Some(cc) = source.content_class {
        payload["content_class"] = serde_json::json!(cc);
    }
    if let Some(sr) = source.sample_rate_hz {
        payload["sample_rate_hz"] = serde_json::json!(sr);
    }
    if let Some(br) = source.bit_rate_kbps {
        payload["bit_rate_kbps"] = serde_json::json!(br);
    }
    if let Some(ref ck) = source.creator_key_id {
        payload["creator_key_id"] = serde_json::Value::String(ck.clone());
    }
    if let Some(ref g) = source.generated_by {
        payload["generated_by"] = serde_json::Value::String(g.clone());
    }
    Ok((payload, sha256))
}

/// Build the canonical `payload` JSON for a video Contribution.
pub fn build_video_payload(
    source: &VideoSource,
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
    if !is_promotable_scope(&source.cohort_scope) {
        return Err(IngestError::UnknownPromotionScope(source.cohort_scope.clone()));
    }
    let is_federation_scope = matches!(
        source.cohort_scope.as_str(),
        "community" | "affiliations" | "species" | "planet" | "federation"
    );
    if is_federation_scope && source.captions.trim().is_empty() {
        return Err(IngestError::MissingAccessibilityText);
    }
    validate_multimedia_federation_constraints(
        &source.cohort_scope,
        source.content_class.as_ref(),
        source.is_ai_generated,
        source.generated_by.is_some(),
    )?;
    let sha256 = compute_sha256(&source.body_bytes);

    let mut payload = serde_json::json!({
        "sub_kind": "video",
        "entity_key_id": source.entity_key_id,
        "language": source.language,
        "cohort_scope": source.cohort_scope,
        "content_sha256": hex_encode(&sha256),
        "content_media_type": source.body_media_type,
        "content_size_bytes": source.body_bytes.len(),
        "duration_seconds": source.duration_seconds,
        "width_px": source.width_px,
        "height_px": source.height_px,
        "captions": source.captions,
        "license": source.license,
        "content_rating": serialize_content_ratings(&source.content_rating),
        "is_ai_generated": source.is_ai_generated,
        "topical_relations": source.topical_relations.iter().map(|tr| serde_json::json!({
            "target_key_id": tr.target_key_id,
            "relation": tr.relation.as_str(),
        })).collect::<Vec<_>>(),
        "citations": source.citations.iter().map(|c| serde_json::json!({
            "kind": c.kind.as_str(),
            "ref": c.ref_string,
        })).collect::<Vec<_>>(),
    });
    if let Some(cc) = source.content_class {
        payload["content_class"] = serde_json::json!(cc);
    }
    if let Some(fr) = source.frame_rate {
        payload["frame_rate"] = serde_json::json!(fr);
    }
    if let Some(ref t) = source.thumbnail_sha256 {
        payload["thumbnail_sha256"] = serde_json::Value::String(t.clone());
    }
    if let Some(ref ck) = source.creator_key_id {
        payload["creator_key_id"] = serde_json::Value::String(ck.clone());
    }
    if let Some(ref g) = source.generated_by {
        payload["generated_by"] = serde_json::Value::String(g.clone());
    }
    Ok((payload, sha256))
}

/// Build the canonical `payload` JSON for a film Contribution.
pub fn build_film_payload(
    source: &FilmSource,
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
    if !is_promotable_scope(&source.cohort_scope) {
        return Err(IngestError::UnknownPromotionScope(source.cohort_scope.clone()));
    }
    if source.captions.trim().is_empty() {
        return Err(IngestError::MissingAccessibilityText);
    }
    // Film requires art_class — that's the whole point of the sub_kind.
    if !source.content_class.art_class() {
        return Err(IngestError::MissingContentClass);
    }
    if source.is_ai_generated && source.generated_by.is_none() {
        return Err(IngestError::UndisclosedAiGenerated);
    }
    let sha256 = compute_sha256(&source.body_bytes);

    let mut payload = serde_json::json!({
        "sub_kind": "film",
        "entity_key_id": source.entity_key_id,
        "language": source.language,
        "cohort_scope": source.cohort_scope,
        "content_sha256": hex_encode(&sha256),
        "content_media_type": source.body_media_type,
        "content_size_bytes": source.body_bytes.len(),
        "content_class": source.content_class,
        "duration_seconds": source.duration_seconds,
        "width_px": source.width_px,
        "height_px": source.height_px,
        "captions": source.captions,
        "languages_available": source.languages_available,
        "production_credits": source.production_credits,
        "license": source.license,
        "content_rating": serialize_content_ratings(&source.content_rating),
        "is_ai_generated": source.is_ai_generated,
        "topical_relations": source.topical_relations.iter().map(|tr| serde_json::json!({
            "target_key_id": tr.target_key_id,
            "relation": tr.relation.as_str(),
        })).collect::<Vec<_>>(),
        "citations": source.citations.iter().map(|c| serde_json::json!({
            "kind": c.kind.as_str(),
            "ref": c.ref_string,
        })).collect::<Vec<_>>(),
    });
    if let Some(fr) = source.frame_rate {
        payload["frame_rate"] = serde_json::json!(fr);
    }
    if let Some(ref t) = source.thumbnail_sha256 {
        payload["thumbnail_sha256"] = serde_json::Value::String(t.clone());
    }
    if let Some(ref dk) = source.distributor_key_id {
        payload["distributor_key_id"] = serde_json::Value::String(dk.clone());
    }
    if let Some(ref i) = source.imdb_id {
        payload["imdb_id"] = serde_json::Value::String(i.clone());
    }
    if let Some(ref t) = source.tmdb_id {
        payload["tmdb_id"] = serde_json::Value::String(t.clone());
    }
    if let Some(y) = source.release_year {
        payload["release_year"] = serde_json::json!(y);
    }
    if let Some(ref g) = source.generated_by {
        payload["generated_by"] = serde_json::Value::String(g.clone());
    }
    Ok((payload, sha256))
}

/// Build the canonical `payload` JSON for a 3D model Contribution.
pub fn build_model_3d_payload(
    source: &Model3dSource,
) -> Result<(serde_json::Value, [u8; 32]), IngestError> {
    if source.entity_key_id.is_empty() {
        return Err(IngestError::EmptyKeyId);
    }
    if source.body_bytes.is_empty() {
        return Err(IngestError::EmptyBody);
    }
    if !is_promotable_scope(&source.cohort_scope) {
        return Err(IngestError::UnknownPromotionScope(source.cohort_scope.clone()));
    }
    validate_multimedia_federation_constraints(
        &source.cohort_scope,
        source.content_class.as_ref(),
        source.is_ai_generated,
        source.generated_by.is_some(),
    )?;
    let sha256 = compute_sha256(&source.body_bytes);

    let mut payload = serde_json::json!({
        "sub_kind": "model_3d",
        "entity_key_id": source.entity_key_id,
        "cohort_scope": source.cohort_scope,
        "content_sha256": hex_encode(&sha256),
        "content_media_type": source.body_media_type,
        "content_size_bytes": source.body_bytes.len(),
        "intended_renderer": source.intended_renderer,
        "has_animations": source.has_animations,
        "license": source.license,
        "content_rating": serialize_content_ratings(&source.content_rating),
        "is_ai_generated": source.is_ai_generated,
        "topical_relations": source.topical_relations.iter().map(|tr| serde_json::json!({
            "target_key_id": tr.target_key_id,
            "relation": tr.relation.as_str(),
        })).collect::<Vec<_>>(),
        "citations": source.citations.iter().map(|c| serde_json::json!({
            "kind": c.kind.as_str(),
            "ref": c.ref_string,
        })).collect::<Vec<_>>(),
    });
    if let Some(cc) = source.content_class {
        payload["content_class"] = serde_json::json!(cc);
    }
    if let Some(v) = source.vertex_count {
        payload["vertex_count"] = serde_json::json!(v);
    }
    if let Some(t) = source.triangle_count {
        payload["triangle_count"] = serde_json::json!(t);
    }
    if let Some(r) = source.max_texture_resolution {
        payload["max_texture_resolution"] = serde_json::json!(r);
    }
    if let Some(ref ck) = source.creator_key_id {
        payload["creator_key_id"] = serde_json::Value::String(ck.clone());
    }
    if let Some(ref g) = source.generated_by {
        payload["generated_by"] = serde_json::Value::String(g.clone());
    }
    Ok((payload, sha256))
}

/// Build the canonical `payload` JSON for a chat-message Contribution
/// per SCHEMA.md §4.29.
pub fn build_chat_payload(
    source: &ChatMessageSource,
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
    if source.platform.is_empty() {
        return Err(IngestError::EmptyPlatform);
    }
    if source.conversation_id.is_empty() {
        return Err(IngestError::EmptyConversationId);
    }
    if source.sender_handle.is_empty() {
        return Err(IngestError::EmptySenderHandle);
    }
    if !is_promotable_scope(&source.cohort_scope) {
        return Err(IngestError::UnknownPromotionScope(source.cohort_scope.clone()));
    }
    let sha256 = compute_sha256(&source.body_bytes);

    let mut source_block = serde_json::json!({
        "kind": "chat",
        "platform": source.platform,
        "conversation_id": source.conversation_id,
        "message_id": source.message_id,
        "sender_handle": source.sender_handle,
        "sent_at": source.sent_at,
        "message_index": source.message_index,
    });
    if let Some(ref sk) = source.sender_key_id {
        source_block["sender_key_id"] = serde_json::Value::String(sk.clone());
    }

    let mut payload = serde_json::json!({
        "sub_kind": "chat_message",
        "entity_key_id": source.entity_key_id,
        "language": source.language,
        "cohort_scope": source.cohort_scope,
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
    });
    if let Some(vu) = source.valid_until {
        payload["valid_until"] = serde_json::json!(vu);
    }

    Ok((payload, sha256))
}

/// Build the canonical `payload` JSON for a blog-post Contribution
/// per SCHEMA.md §4.29.
pub fn build_blog_payload(
    source: &BlogPostSource,
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
    if source.platform.is_empty() {
        return Err(IngestError::EmptyPlatform);
    }
    if source.blog_id.is_empty() {
        return Err(IngestError::EmptyBlogId);
    }
    if source.author_handle.is_empty() {
        return Err(IngestError::EmptyAuthorHandle);
    }
    if source.post_title.is_empty() {
        return Err(IngestError::EmptyPostTitle);
    }
    if !is_promotable_scope(&source.cohort_scope) {
        return Err(IngestError::UnknownPromotionScope(source.cohort_scope.clone()));
    }
    let sha256 = compute_sha256(&source.body_bytes);

    let mut source_block = serde_json::json!({
        "kind": "blog",
        "platform": source.platform,
        "blog_id": source.blog_id,
        "author_handle": source.author_handle,
        "published_at": source.published_at,
        "post_title": source.post_title,
        "post_url": source.post_url,
    });
    if let Some(ref ak) = source.author_key_id {
        source_block["author_key_id"] = serde_json::Value::String(ak.clone());
    }
    if !source.tags.is_empty() {
        source_block["tags"] = serde_json::json!(source.tags);
    }

    let mut payload = serde_json::json!({
        "sub_kind": "blog_post",
        "entity_key_id": source.entity_key_id,
        "language": source.language,
        "cohort_scope": source.cohort_scope,
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
    });
    if let Some(vu) = source.valid_until {
        payload["valid_until"] = serde_json::json!(vu);
    }

    Ok((payload, sha256))
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
        "encyclopedia_article"
            | "news_article"
            | "accord_data"
            | "local_data"
            | "chat_message"
            | "blog_post"
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
    /// Chat / blog requires a non-empty platform tag.
    #[error("platform must be non-empty")]
    EmptyPlatform,
    /// Chat requires a non-empty conversation_id.
    #[error("conversation_id must be non-empty for chat_message")]
    EmptyConversationId,
    /// Chat requires a non-empty sender_handle.
    #[error("sender_handle must be non-empty for chat_message")]
    EmptySenderHandle,
    /// Blog requires a non-empty blog_id.
    #[error("blog_id must be non-empty for blog_post")]
    EmptyBlogId,
    /// Blog requires a non-empty author_handle.
    #[error("author_handle must be non-empty for blog_post")]
    EmptyAuthorHandle,
    /// Blog requires a non-empty post_title.
    #[error("post_title must be non-empty for blog_post")]
    EmptyPostTitle,
    /// Image / video require non-empty alt text or captions for
    /// `cohort_scope ≥ community` (accessibility, per FSD/MEDIA_SHARING
    /// §2.1 + §2.3).
    #[error("alt_text / captions must be non-empty for accessible multimedia at federation scope")]
    MissingAccessibilityText,
    /// Multimedia at federation scope must declare `content_class`
    /// (FSD/MEDIA_SHARING §3.3).
    #[error("content_class must be declared for multimedia at federation scope")]
    MissingContentClass,
    /// Multimedia carrying `is_ai_generated: true` without an
    /// `authenticity:ai_generated` attestation in the source
    /// declarations (EU AI Act Article 50 — substrate enforces
    /// disclosure on AI-generated content at community+ scope).
    #[error("AI-generated content must declare authenticity:ai_generated (EU AI Act Art. 50)")]
    UndisclosedAiGenerated,
    /// `BlobStorage::put_blob_signing` rejected the write — hash
    /// mismatch, inline size cap exceeded, FK violation on
    /// `attesting_key_id`, or signer error.
    #[error("put_blob: {0}")]
    PutBlob(String),
    /// Envelope canonicalization or signing failed during
    /// [`crate::sign::build_contribution`].
    #[error("build_envelope: {0}")]
    BuildEnvelope(String),
    /// `NodeCoreService::put_contribution` rejected the typed write —
    /// signature verification, conflict on contribution_id,
    /// authorization, or backend error.
    #[error("put_contribution: {0}")]
    PutContribution(String),
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

// ───────────────────────────────────────────────────────────────────────
// Phase 2B I/O layer — full sign + put_blob + put_contribution sequence
//
// The pure `build_*_payload` builders above produce the wire JSON.
// The functions below add the side effects:
//   1. compute content_sha256 over body_bytes
//   2. persist's `put_blob_signing` — atomic blob commit + holds_bytes
//      attestation (CIRISPersist#121 v3.3.0; persist owns the
//      canonicalizer to avoid the JCS-vs-Python silent-correctness trap)
//   3. build the Contribution envelope (sub_kind discriminated via
//      payload, ContributionType is always `Proposal` for external_content)
//   4. sign the envelope via `LocalSignerAdapter` (the host's own
//      `LocalSigner` — no proxy-signing; observation/witness claims
//      about third parties live in the payload, not in the envelope
//      identity)
//   5. `NodeCoreService::put_contribution` — typed write, verify +
//      insert
//
// Per the v0.2 scaling-model discipline: this path produces the wire
// artifacts CEG's replication primitives flow on. The substrate
// (persist + edge) then decides where the bytes propagate based on
// trust × capacity + popularity × freshness. Node-core's job ends
// at `put_contribution` returning Ok.
// ───────────────────────────────────────────────────────────────────────

use std::sync::Arc;

use ciris_persist::cirisnode::NodeCoreService;
use ciris_persist::federation::{BlobBody, BlobStorage};
use ciris_persist::signing::{LocalSigner, LocalSignerHardwareAdapter};

use crate::sign::{build_contribution, LocalSignerAdapter};
use crate::substrate::{Cell, ContributionType};

/// Injected substrate handles + signer for the full ingest path.
///
/// Per CIRISNodeCore#4: node-core NEVER constructs the substrate. The
/// host process owns `Engine` (persist) + `Edge`, hands node-core
/// these references via the cohabitation install path, and node-core
/// borrows them for ingest. `Arc<LocalSigner>` is what
/// `PyEngine::local_signer_capsule` (CIRISPersist#119) exposes — the
/// agent's federation identity, used for BOTH:
///
/// * the holds_bytes attestation envelope (via persist's
///   `LocalSignerHardwareAdapter`)
/// * the Contribution envelope (via node-core's [`LocalSignerAdapter`])
///
/// One identity, two adapter shapes, because persist's blob-storage
/// surface takes `&dyn HardwareSigner` and node-core's envelope-build
/// surface takes `&dyn EnvelopeSigner`. Both wrap the same key.
pub struct IngestContext<'a, B, N>
where
    B: BlobStorage,
    N: NodeCoreService,
{
    /// Blob storage substrate — `Engine::blob_storage_capsule` /
    /// `Engine::federation_directory().Postgres(_)` / etc., depending
    /// on the cohabitation install path.
    pub blob_storage: &'a B,
    /// NodeCore write surface — `Engine::node_core_service()` per
    /// CIRISPersist#90.
    pub node_core: &'a N,
    /// Host's `LocalSigner` (CIRISPersist#119). Cheap `Arc` clone is
    /// expected at call sites.
    pub signer: Arc<LocalSigner>,
    /// Host's `federation_keys.key_id` — the directory FK that the
    /// emitted `holds_bytes` attestation cites as `attesting_key_id`.
    /// Typically `LocalSigner::key_id()`.
    pub author_key_id: String,
}

/// What a successful ingest call returns. The `content_sha256_hex` is
/// the cite-able blob identifier (`agent_files:*:{sha256}` /
/// `holds_bytes:sha256:{prefix}` reference); the `contribution_id`
/// is the ULID of the envelope persist stored.
#[derive(Debug, Clone)]
pub struct IngestOutcome {
    /// ULID per SCHEMA §2.2.
    pub contribution_id: String,
    /// Hex of the content SHA-256 (lowercase, 64 chars).
    pub content_sha256_hex: String,
}

/// Ingest an encyclopedia article — the Phase 2B template all six
/// `external_content` sub_kinds follow.
///
/// # The sequence
///
/// 1. `build_encyclopedia_payload(&source)` — pure payload + SHA-256.
/// 2. `BlobStorage::put_blob_signing(...)` — atomic blob+holder
///    commit. Persist canonicalizes + signs the holds_bytes envelope
///    via its production `PythonJsonDumpsCanonicalizer` (NOT JCS RFC
///    8785; the trap CIRISPersist#121 closed). The hardware-signer
///    adapter wraps our `Arc<LocalSigner>` so persist can drive its
///    `&dyn HardwareSigner` interface.
/// 3. `build_contribution(...)` — Contribution envelope.
///    `ContributionType::Proposal` is the persist enum variant;
///    sub_kind discrimination (`encyclopedia_article` vs
///    `news_article` vs …) lives in the payload per SCHEMA §3.1 +
///    §4.29.
/// 4. `NodeCoreService::put_contribution(...)` — verify + insert
///    into the `cirisnode.contributions` row.
///
/// # Identity discipline
///
/// `author_id` on the Contribution envelope is the host's own
/// `contributor_id()` (base64 Ed25519 pubkey of `LocalSigner`). The
/// host signs as itself; the article being *about* Wikipedia /
/// Einstein / a third-party publisher is encoded in the payload
/// fields (`source.project`, `entity_key_id`, citations,
/// `topical_relations`), not by spoofing the envelope identity.
/// "X wrote this" is an observation claim by the host, not a
/// signature on behalf of X.
///
/// # Cell parameter
///
/// `Cell { domain, language, subject }` is per-call because the same
/// content could be ingested into different domain cells depending
/// on the deployment's classification policy. `subject` should be
/// `Some("external_content".to_string())` for this surface.
/// Shared I/O tail for every `external_content` sub_kind ingest.
/// All six sub_kinds — encyclopedia / news / accord / local / chat /
/// blog — produce the same `(payload, content_sha, body_bytes,
/// media_type)` tuple shape from their per-kind builder; the put_blob
/// + sign + put_contribution sequence is byte-for-byte identical.
async fn finalize_external_content_ingest<B, N>(
    payload: serde_json::Value,
    content_sha: [u8; 32],
    body_bytes: Vec<u8>,
    media_type: String,
    cell: Cell,
    ctx: &IngestContext<'_, B, N>,
) -> Result<IngestOutcome, IngestError>
where
    B: BlobStorage + Sync,
    N: NodeCoreService,
{
    let hw_signer = LocalSignerHardwareAdapter::new(ctx.signer.clone());
    ctx.blob_storage
        .put_blob_signing(
            &content_sha,
            BlobBody::Inline(body_bytes),
            Some(media_type.as_str()),
            &ctx.author_key_id,
            &hw_signer,
            Utc::now(),
            uuid::Uuid::new_v4(),
        )
        .await
        .map_err(|e| IngestError::PutBlob(format!("{e}")))?;

    let env_signer = LocalSignerAdapter::new(ctx.signer.clone());
    let contribution_id = ulid::Ulid::new().to_string();
    let envelope = build_contribution(
        contribution_id.clone(),
        ContributionType::Proposal,
        env_signer.contributor_id(),
        cell,
        payload,
        None,
        &env_signer,
    )
    .map_err(|e| IngestError::BuildEnvelope(format!("{e}")))?;

    ctx.node_core
        .put_contribution(envelope)
        .await
        .map_err(|e| IngestError::PutContribution(format!("{e}")))?;

    Ok(IngestOutcome {
        contribution_id,
        content_sha256_hex: hex_encode(&content_sha),
    })
}

/// Ingest an encyclopedia article (SCHEMA §4.29 `sub_kind:
/// encyclopedia_article`) — the Phase 2B template the other five
/// sub_kind ingest functions follow. See [`IngestContext`] for the
/// substrate injection discipline + [`finalize_external_content_ingest`]
/// for the shared I/O tail.
pub async fn ingest_encyclopedia_article<B, N>(
    source: EncyclopediaArticleSource,
    cell: Cell,
    ctx: &IngestContext<'_, B, N>,
) -> Result<IngestOutcome, IngestError>
where
    B: BlobStorage + Sync,
    N: NodeCoreService,
{
    let (payload, content_sha) = build_encyclopedia_payload(&source)?;
    finalize_external_content_ingest(
        payload,
        content_sha,
        source.body_bytes,
        source.body_media_type,
        cell,
        ctx,
    )
    .await
}

/// Ingest a news article (SCHEMA §4.29 `sub_kind: news_article`).
/// Same I/O discipline as [`ingest_encyclopedia_article`]; sub_kind
/// discrimination lives in the payload.
pub async fn ingest_news_article<B, N>(
    source: NewsArticleSource,
    cell: Cell,
    ctx: &IngestContext<'_, B, N>,
) -> Result<IngestOutcome, IngestError>
where
    B: BlobStorage + Sync,
    N: NodeCoreService,
{
    let (payload, content_sha) = build_news_payload(&source)?;
    finalize_external_content_ingest(
        payload,
        content_sha,
        source.body_bytes,
        source.body_media_type,
        cell,
        ctx,
    )
    .await
}

/// Ingest accord data (SCHEMA §4.29 `sub_kind: accord_data`) —
/// canonical text / encyclical mapping / framework / policy under
/// a HumanityAccord / StewardTriple / WaQuorum / OneOfSix signer
/// class. Same I/O discipline.
pub async fn ingest_accord_data<B, N>(
    source: AccordDataSource,
    cell: Cell,
    ctx: &IngestContext<'_, B, N>,
) -> Result<IngestOutcome, IngestError>
where
    B: BlobStorage + Sync,
    N: NodeCoreService,
{
    let (payload, content_sha) = build_accord_payload(&source)?;
    finalize_external_content_ingest(
        payload,
        content_sha,
        source.body_bytes,
        source.body_media_type,
        cell,
        ctx,
    )
    .await
}

/// Ingest local data (SCHEMA §4.29 `sub_kind: local_data`) — notes /
/// draft / bookmark / observation. Cohort scope is typically `self`
/// or `family`; the CEG locality dividend means this content never
/// crosses to inter-host paths via the `holds_bytes` advertisement
/// in those cases — `put_blob_signing` still emits the holder
/// attestation locally (own-substrate authoritative record), but no
/// peer can discover it.
pub async fn ingest_local_data<B, N>(
    source: LocalDataSource,
    cell: Cell,
    ctx: &IngestContext<'_, B, N>,
) -> Result<IngestOutcome, IngestError>
where
    B: BlobStorage + Sync,
    N: NodeCoreService,
{
    let (payload, content_sha) = build_local_payload(&source)?;
    finalize_external_content_ingest(
        payload,
        content_sha,
        source.body_bytes,
        source.body_media_type,
        cell,
        ctx,
    )
    .await
}

/// Ingest a chat message (SCHEMA §4.29 `sub_kind: chat_message`) —
/// discord / slack / twitter / imessage / sms / xmpp / irc / matrix.
/// Same I/O discipline.
pub async fn ingest_chat_message<B, N>(
    source: ChatMessageSource,
    cell: Cell,
    ctx: &IngestContext<'_, B, N>,
) -> Result<IngestOutcome, IngestError>
where
    B: BlobStorage + Sync,
    N: NodeCoreService,
{
    let (payload, content_sha) = build_chat_payload(&source)?;
    finalize_external_content_ingest(
        payload,
        content_sha,
        source.body_bytes,
        source.body_media_type,
        cell,
        ctx,
    )
    .await
}

/// Ingest a blog post (SCHEMA §4.29 `sub_kind: blog_post`) —
/// medium / substack / wordpress / ghost / personal / tumblr.
/// Same I/O discipline.
pub async fn ingest_blog_post<B, N>(
    source: BlogPostSource,
    cell: Cell,
    ctx: &IngestContext<'_, B, N>,
) -> Result<IngestOutcome, IngestError>
where
    B: BlobStorage + Sync,
    N: NodeCoreService,
{
    let (payload, content_sha) = build_blog_payload(&source)?;
    finalize_external_content_ingest(
        payload,
        content_sha,
        source.body_bytes,
        source.body_media_type,
        cell,
        ctx,
    )
    .await
}

/// Ingest an image (FSD/MEDIA_SHARING.md §2.1 `sub_kind: image`).
/// Same I/O discipline as text sub_kinds; adds federation-scope
/// validation (alt_text mandatory at community+, content_class
/// required, EU AI Act Art. 50 AI-disclosure check).
pub async fn ingest_image<B, N>(
    source: ImageSource,
    cell: Cell,
    ctx: &IngestContext<'_, B, N>,
) -> Result<IngestOutcome, IngestError>
where
    B: BlobStorage + Sync,
    N: NodeCoreService,
{
    let (payload, content_sha) = build_image_payload(&source)?;
    finalize_external_content_ingest(
        payload, content_sha, source.body_bytes, source.body_media_type, cell, ctx,
    )
    .await
}

/// Ingest audio (FSD/MEDIA_SHARING.md §2.2 `sub_kind: audio`) —
/// music / podcast / lecture / audiobook / generated audio.
pub async fn ingest_audio<B, N>(
    source: AudioSource,
    cell: Cell,
    ctx: &IngestContext<'_, B, N>,
) -> Result<IngestOutcome, IngestError>
where
    B: BlobStorage + Sync,
    N: NodeCoreService,
{
    let (payload, content_sha) = build_audio_payload(&source)?;
    finalize_external_content_ingest(
        payload, content_sha, source.body_bytes, source.body_media_type, cell, ctx,
    )
    .await
}

/// Ingest video (FSD/MEDIA_SHARING.md §2.3 `sub_kind: video`) —
/// general video; for cinema/art-bearing video use [`ingest_film`].
pub async fn ingest_video<B, N>(
    source: VideoSource,
    cell: Cell,
    ctx: &IngestContext<'_, B, N>,
) -> Result<IngestOutcome, IngestError>
where
    B: BlobStorage + Sync,
    N: NodeCoreService,
{
    let (payload, content_sha) = build_video_payload(&source)?;
    finalize_external_content_ingest(
        payload, content_sha, source.body_bytes, source.body_media_type, cell, ctx,
    )
    .await
}

/// Ingest a film (FSD/MEDIA_SHARING.md §2.4 `sub_kind: film`) —
/// cinematic / art-bearing video. R/X-rated cinema circulates at
/// federation scope because the content_class + content_rating +
/// distributor attestation chain make the art-bearing nature
/// adjudicable by the trust graph (FSD §1.3).
pub async fn ingest_film<B, N>(
    source: FilmSource,
    cell: Cell,
    ctx: &IngestContext<'_, B, N>,
) -> Result<IngestOutcome, IngestError>
where
    B: BlobStorage + Sync,
    N: NodeCoreService,
{
    let (payload, content_sha) = build_film_payload(&source)?;
    finalize_external_content_ingest(
        payload, content_sha, source.body_bytes, source.body_media_type, cell, ctx,
    )
    .await
}

/// Ingest 3D content (FSD/MEDIA_SHARING.md §2.5
/// `sub_kind: model_3d`) — glTF / USDZ / FBX / OBJ / Gaussian splat /
/// volumetric video.
pub async fn ingest_model_3d<B, N>(
    source: Model3dSource,
    cell: Cell,
    ctx: &IngestContext<'_, B, N>,
) -> Result<IngestOutcome, IngestError>
where
    B: BlobStorage + Sync,
    N: NodeCoreService,
{
    let (payload, content_sha) = build_model_3d_payload(&source)?;
    finalize_external_content_ingest(
        payload, content_sha, source.body_bytes, source.body_media_type, cell, ctx,
    )
    .await
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

    // --- chat_message --------------------------------------------------

    #[test]
    fn chat_payload_with_reply_chain() {
        let src = ChatMessageSource {
            entity_key_id: "chat:discord:123:456".into(),
            language: "en".into(),
            body_bytes: b"hey, the coherence post was great".to_vec(),
            body_media_type: "text/plain".into(),
            platform: "discord".into(),
            conversation_id: "123-channel".into(),
            message_id: "456".into(),
            sender_handle: "alice#1234".into(),
            sender_key_id: Some("user-alice-fed-key".into()),
            sent_at: parse_dt("2026-05-29T08:00:00Z"),
            message_index: 42,
            cohort_scope: "community".into(),
            topical_relations: vec![TopicalRelation {
                target_key_id: "chat:discord:123:455".into(),
                relation: TopicalRelationKind::RepliesTo,
            }],
            citations: vec![],
            valid_until: Some(parse_dt("2026-06-29T08:00:00Z")),
        };
        let (payload, _sha) = build_chat_payload(&src).unwrap();
        assert_eq!(payload["sub_kind"], "chat_message");
        assert_eq!(payload["cohort_scope"], "community");
        assert_eq!(payload["source"]["platform"], "discord");
        assert_eq!(payload["source"]["conversation_id"], "123-channel");
        assert_eq!(payload["source"]["sender_handle"], "alice#1234");
        assert_eq!(payload["source"]["sender_key_id"], "user-alice-fed-key");
        assert_eq!(payload["source"]["message_index"], 42);
        // reply chain captured as topical_relation
        let trs = payload["topical_relations"].as_array().unwrap();
        assert_eq!(trs.len(), 1);
        assert_eq!(trs[0]["relation"], "replies_to");
        assert!(payload["valid_until"].is_string());
    }

    #[test]
    fn chat_payload_without_sender_key_omits_field() {
        let src = ChatMessageSource {
            entity_key_id: "chat:irc:freenode#ciris:1".into(),
            language: "en".into(),
            body_bytes: b"a wild IRC user appears".to_vec(),
            body_media_type: "text/plain".into(),
            platform: "irc".into(),
            conversation_id: "#ciris".into(),
            message_id: "1".into(),
            sender_handle: "anon-user".into(),
            sender_key_id: None,
            sent_at: parse_dt("2026-05-29T08:00:00Z"),
            message_index: 0,
            cohort_scope: "community".into(),
            topical_relations: vec![],
            citations: vec![],
            valid_until: None,
        };
        let (payload, _) = build_chat_payload(&src).unwrap();
        assert!(payload["source"].get("sender_key_id").is_none());
        assert!(payload.get("valid_until").is_none());
    }

    #[test]
    fn chat_rejects_empty_required_fields() {
        let mut src = ChatMessageSource {
            entity_key_id: "chat:discord:123:456".into(),
            language: "en".into(),
            body_bytes: b"body".to_vec(),
            body_media_type: "text/plain".into(),
            platform: "".into(),
            conversation_id: "conv".into(),
            message_id: "1".into(),
            sender_handle: "alice".into(),
            sender_key_id: None,
            sent_at: parse_dt("2026-05-29T08:00:00Z"),
            message_index: 0,
            cohort_scope: "community".into(),
            topical_relations: vec![],
            citations: vec![],
            valid_until: None,
        };
        assert!(matches!(build_chat_payload(&src), Err(IngestError::EmptyPlatform)));

        src.platform = "discord".into();
        src.conversation_id = "".into();
        assert!(matches!(
            build_chat_payload(&src),
            Err(IngestError::EmptyConversationId)
        ));

        src.conversation_id = "conv".into();
        src.sender_handle = "".into();
        assert!(matches!(
            build_chat_payload(&src),
            Err(IngestError::EmptySenderHandle)
        ));
    }

    // --- blog_post -----------------------------------------------------

    #[test]
    fn blog_payload_with_full_metadata() {
        let src = BlogPostSource {
            entity_key_id: "blog:substack:@ericmoore:coherence-2026-05".into(),
            language: "en".into(),
            body_bytes: b"# On Coherence\n\nLong-form post body...".to_vec(),
            body_media_type: "text/markdown".into(),
            platform: "substack".into(),
            blog_id: "@ericmoore".into(),
            author_handle: "Eric Moore".into(),
            author_key_id: Some("author-key-eric".into()),
            published_at: parse_dt("2026-05-15T12:00:00Z"),
            post_title: "On Coherence".into(),
            post_url: "https://ericmoore.substack.com/p/on-coherence".into(),
            tags: vec!["coherence".into(), "epistemology".into()],
            cohort_scope: "federation".into(),
            topical_relations: vec![TopicalRelation {
                target_key_id: "wikipedia:article:coherence".into(),
                relation: TopicalRelationKind::References,
            }],
            citations: vec![Citation {
                kind: CitationKind::Doi,
                ref_string: "10.5281/zenodo.18217688".into(),
            }],
            valid_until: None,
        };
        let (payload, _sha) = build_blog_payload(&src).unwrap();
        assert_eq!(payload["sub_kind"], "blog_post");
        assert_eq!(payload["cohort_scope"], "federation");
        assert_eq!(payload["source"]["kind"], "blog");
        assert_eq!(payload["source"]["platform"], "substack");
        assert_eq!(payload["source"]["blog_id"], "@ericmoore");
        assert_eq!(payload["source"]["author_handle"], "Eric Moore");
        assert_eq!(payload["source"]["author_key_id"], "author-key-eric");
        assert_eq!(payload["source"]["post_title"], "On Coherence");
        assert_eq!(payload["source"]["tags"].as_array().unwrap().len(), 2);
        assert!(payload.get("valid_until").is_none());  // typical blog: indefinite
    }

    #[test]
    fn blog_rejects_empty_required_fields() {
        let mut src = BlogPostSource {
            entity_key_id: "blog:medium:test:1".into(),
            language: "en".into(),
            body_bytes: b"body".to_vec(),
            body_media_type: "text/markdown".into(),
            platform: "medium".into(),
            blog_id: "".into(),
            author_handle: "Author".into(),
            author_key_id: None,
            published_at: parse_dt("2026-05-15T12:00:00Z"),
            post_title: "Title".into(),
            post_url: "https://example.com/post".into(),
            tags: vec![],
            cohort_scope: "federation".into(),
            topical_relations: vec![],
            citations: vec![],
            valid_until: None,
        };
        assert!(matches!(build_blog_payload(&src), Err(IngestError::EmptyBlogId)));

        src.blog_id = "blog-1".into();
        src.author_handle = "".into();
        assert!(matches!(
            build_blog_payload(&src),
            Err(IngestError::EmptyAuthorHandle)
        ));

        src.author_handle = "Author".into();
        src.post_title = "".into();
        assert!(matches!(
            build_blog_payload(&src),
            Err(IngestError::EmptyPostTitle)
        ));
    }

    #[test]
    fn blog_comment_chain_via_topical_relations() {
        // A comment on a blog post is itself a Contribution (chat_message
        // OR blog_post depending on platform conventions) citing the
        // post via CommentsOn. Verify the relation enum is wired.
        let comment_src = ChatMessageSource {
            entity_key_id: "chat:medium:comment-789".into(),
            language: "en".into(),
            body_bytes: "Great post! I disagree with §3 though.".as_bytes().to_vec(),
            body_media_type: "text/plain".into(),
            platform: "medium".into(),
            conversation_id: "comments-on-post-456".into(),
            message_id: "comment-789".into(),
            sender_handle: "reader-alice".into(),
            sender_key_id: None,
            sent_at: parse_dt("2026-05-16T09:00:00Z"),
            message_index: 0,
            cohort_scope: "federation".into(),
            topical_relations: vec![TopicalRelation {
                target_key_id: "blog:medium:@ericmoore:coherence-2026-05".into(),
                relation: TopicalRelationKind::CommentsOn,
            }],
            citations: vec![],
            valid_until: None,
        };
        let (payload, _) = build_chat_payload(&comment_src).unwrap();
        assert_eq!(payload["topical_relations"][0]["relation"], "comments_on");
    }

    #[test]
    fn promote_blog_to_global_works() {
        let payload = serde_json::json!({
            "sub_kind": "blog_post",
            "entity_key_id": "blog:medium:author:post",
            "content_sha256": "abc",
            "cohort_scope": "community",
        });
        let promoted = promote_payload(&payload, "c-1", None, "federation").unwrap();
        assert_eq!(promoted["sub_kind"], "blog_post");
        assert_eq!(promoted["cohort_scope"], "federation");
    }

    #[test]
    fn promote_chat_to_news_works() {
        // An observation chat message ("breaking news in my city")
        // promoted to a news_article at federation scope.
        let payload = serde_json::json!({
            "sub_kind": "chat_message",
            "entity_key_id": "chat:slack:newsroom:42",
            "content_sha256": "abc",
            "cohort_scope": "community",
        });
        let promoted =
            promote_payload(&payload, "c-1", Some("news_article"), "federation").unwrap();
        assert_eq!(promoted["sub_kind"], "news_article");
        assert_eq!(promoted["cohort_scope"], "federation");
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
