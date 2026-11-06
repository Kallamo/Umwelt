// SPDX-License-Identifier: AGPL-3.0-only
#![deny(clippy::unwrap_used, clippy::expect_used)]
//! Public Rust facade for Umwelt.

mod engine;
mod queries;
mod runtime;

pub use engine::Engine;
pub use queries::TimelineReader;
pub use runtime::{FixedClock, SeqIdGen, SystemClock, UuidV7Gen};
pub use umwelt_core::domain::*;
pub use umwelt_core::{
    ChangePage, Clock, EntityPage, EventCursor, EventPage, EventQuery, IdGen, PerspectiveRecords,
    RecordCursor, RecordPage, RecordQuery, ScopeAncestor, SearchPage, SearchQuery, Store,
    TimelineScope, UMWELT_CONTRACT_VERSION, canonical_json, claim_key, commit_fingerprint, domain,
    slot_key,
};
pub use umwelt_sqlite::{Durability, STORAGE_VERSION, SqliteStore};
