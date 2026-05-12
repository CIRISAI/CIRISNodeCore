//! Typed payloads per SCHEMA.md §4.
//!
//! The Contribution envelope (`crate::contribution::ContributionEnvelope`)
//! carries `payload: serde_json::Value`. These typed structs are what
//! that JSON deserializes into, discriminated by the envelope's
//! `contribution_type` + `subject.subject_kind`.
//!
//! v0.1.0-dev coverage:
//!
//! - §4.7 [`deferral::DeferralRequest`] — full shape pinned.
//! - §4.8 [`deferral::DeferralResponse`] — full shape pinned.
//! - §4.1–§4.6, §4.9–§4.12 — stubs; full shapes land as implementation
//!   matures. See `SCHEMA.md` for the wire definitions.

pub mod deferral;

// TODO: typed payloads for the remaining SCHEMA.md §4 subjects.
// Tracking module declarations follow once each gets a typed struct;
// for now each Contribution validates its raw `serde_json::Value`
// payload at the validation boundary.
//
// pub mod arc_question;          // §4.1
// pub mod proposed_battery;      // §4.2
// pub mod prompt_edit;           // §4.3
// pub mod guide_edit;            // §4.4
// pub mod accord_edit;           // §4.5
// pub mod failure_pattern;       // §4.6 (ticket)
// pub mod wa_candidacy;          // §4.9
// pub mod expertise_attestation; // §4.10
// pub mod moderation_event;      // §4.11
// pub mod reconsideration;       // §4.12
