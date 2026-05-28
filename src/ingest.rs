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
    let sha256 = compute_sha256(&article.body_bytes);

    Ok((
        serde_json::json!({
            "sub_kind": "encyclopedia_article",
            "entity_key_id": article.entity_key_id,
            "language": article.language,
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
            topical_relations: vec![],
            citations: vec![],
        };
        let (payload, sha) = build_encyclopedia_payload(&src).unwrap();
        assert_eq!(payload["content_sha256"], expected);
        assert_eq!(hex_encode(&sha), expected);
    }
}
