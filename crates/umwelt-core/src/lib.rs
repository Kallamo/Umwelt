// SPDX-License-Identifier: AGPL-3.0-only
#![deny(clippy::unwrap_used, clippy::expect_used)]
//! Pure Umwelt domain contracts, projections, search, and context planning.

mod claim;
mod embedding;
mod error;
mod store;
mod validation;

pub mod domain;

pub use claim::{canonical_json, claim_key, commit_fingerprint, request_fingerprint, slot_key};
pub use domain::UMWELT_CONTRACT_VERSION;
pub use embedding::{
    DocRef, EVENT_DOC, MAX_EMBEDDING_DIMENSIONS, MAX_SEMANTIC_WINDOW, RECORD_DOC, RRF_K, RankedDoc,
    event_search_text, fuse_rankings, normalize_vector, rank_order, record_search_text, similarity,
    text_hash, validate_doc_type, validate_embedding_space, validate_text_hash,
};
pub use error::{Result, UmweltError, UmweltErrorCode};
pub use store::{
    ChangePage, Clock, EntityPage, EntityWrite, EventCursor, EventPage, EventQuery, HoldingSummary,
    IdGen, PendingPage, PendingQuery, PerspectiveRecords, PreparedCommit, RecordCursor, RecordPage,
    RecordQuery, ScopeAncestor, SearchPage, SearchQuery, SemanticQuery, Store, StoreCounts,
    TimelineScope, latest_reviews,
};
pub use validation::{ValidatedCommit, ValidationView, materialize_commit, validate_commit};
