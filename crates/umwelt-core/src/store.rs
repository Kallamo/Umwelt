// SPDX-License-Identifier: AGPL-3.0-only
pub trait Clock: Send + Sync {
    fn now_utc(&self) -> String;
}

pub trait IdGen: Send + Sync {
    fn new_id(&self) -> String;
}

use crate::Result;
use crate::domain::{
    AxisDef, ChangeItem, CommitInput, CommitOutcome, EmbeddingDocRef, EmbeddingItem,
    EmbeddingSpace, Entity, EntityType, Event, EvidenceSource, ExternalRef, MAIN_TIMELINE_ID,
    PendingEmbedding, PredicateDef, Project, Proposal, ProposalReview, ProposalStatus, PurgeCounts,
    PutEmbeddingsResult, Record, RecordState, ReviewInput, SearchItem, Timeline, TimelineStatus,
    Transition, Version, Warning,
};

/// Which rows a read may see. In main: main rows only. In a draft: the draft's
/// own rows plus, for each ancestor up to main, the ancestor's rows recorded up
/// to the version at which the branch below it was opened.
///
/// `axis` names the additional axis a positioned read is bounded on; `None`
/// is the main axis. Reads without a position (by id) ignore it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineScope {
    pub timeline_id: String,
    /// The draft's ancestors, nearest first and ending with main. Empty for
    /// main.
    pub ancestors: Vec<ScopeAncestor>,
    pub axis: Option<String>,
}

/// An ancestor timeline of a draft and the highest version of its rows the
/// draft sees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeAncestor {
    pub timeline_id: String,
    pub max_version: Version,
}

impl ScopeAncestor {
    pub fn new(timeline_id: impl Into<String>, max_version: Version) -> Self {
        Self {
            timeline_id: timeline_id.into(),
            max_version,
        }
    }
}

impl TimelineScope {
    pub fn main() -> Self {
        Self {
            timeline_id: MAIN_TIMELINE_ID.to_owned(),
            ancestors: Vec::new(),
            axis: None,
        }
    }

    /// A draft of main opened at `base_version`.
    pub fn draft(timeline_id: impl Into<String>, base_version: Version) -> Self {
        Self::nested(
            timeline_id,
            vec![ScopeAncestor::new(MAIN_TIMELINE_ID, base_version)],
        )
    }

    /// A draft with its ancestors, nearest first and ending with main.
    pub fn nested(timeline_id: impl Into<String>, ancestors: Vec<ScopeAncestor>) -> Self {
        Self {
            timeline_id: timeline_id.into(),
            ancestors,
            axis: None,
        }
    }

    /// The same scope, with positioned reads bounded on `axis`.
    pub fn on_axis(mut self, axis: Option<String>) -> Self {
        self.axis = axis;
        self
    }

    pub fn is_main(&self) -> bool {
        self.ancestors.is_empty()
    }

    /// Whether a read may use main's materialized status: main, on the main
    /// axis (the caller still checks it reads main's head).
    pub fn allows_current_status(&self) -> bool {
        self.is_main() && self.axis.is_none()
    }
}

#[derive(Debug, Clone)]
pub struct PreparedCommit {
    pub input: CommitInput,
    pub fingerprint: String,
    pub event_id: String,
    pub record_ids: Vec<String>,
    pub transition_ids: Vec<String>,
}

pub trait Store: Send {
    fn create_project(&mut self, project: &Project) -> Result<()>;
    fn project(&self, project_id: &str) -> Result<Project>;
    fn projects(&self) -> Result<Vec<Project>>;
    fn put_entity_type(&mut self, entity_type: &EntityType) -> Result<()>;
    fn entity_types(&self, project_id: &str) -> Result<Vec<EntityType>>;
    fn put_entity(&mut self, entity: &Entity) -> Result<()>;
    fn update_entity(&mut self, entity: &Entity) -> Result<()>;
    /// Creates and updates entities in one transaction.
    fn write_entities(&mut self, writes: &[EntityWrite]) -> Result<()>;
    fn entity_by_external(
        &self,
        project_id: &str,
        reference: &ExternalRef,
    ) -> Result<Option<Entity>>;
    fn put_predicate(&mut self, project_id: &str, predicate: &PredicateDef) -> Result<()>;
    /// Defines an additional axis. Defining the same id with the same label
    /// again returns the stored definition; another label is a `CONFLICT`.
    fn define_axis(&mut self, project_id: &str, axis: &AxisDef) -> Result<AxisDef>;
    fn axes(&self, project_id: &str) -> Result<Vec<AxisDef>>;
    /// The highest position on `axis` among the events visible in `scope`,
    /// at the timeline's head.
    fn axis_max_tick(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        axis: &str,
    ) -> Result<Option<i64>>;
    /// Defines an embedding space. The same definition again returns it;
    /// another definition for the id is a `CONFLICT`.
    fn define_embedding_space(
        &mut self,
        project_id: &str,
        space: &EmbeddingSpace,
    ) -> Result<EmbeddingSpace>;
    fn embedding_spaces(&self, project_id: &str) -> Result<Vec<EmbeddingSpace>>;
    /// Removes a space and its vectors; returns how many vectors went.
    fn drop_embedding_space(&mut self, project_id: &str, space_id: &str) -> Result<u64>;
    /// Stores or replaces vectors in one transaction. Vectors are already
    /// normalized to the space's size. Every document must exist and be
    /// searchable; a vector whose `text_hash` is not its document's is stored
    /// as stale.
    fn put_embeddings(
        &mut self,
        project_id: &str,
        space_id: &str,
        items: &[EmbeddingItem],
    ) -> Result<PutEmbeddingsResult>;
    fn delete_embeddings(
        &mut self,
        project_id: &str,
        space_id: &str,
        items: &[EmbeddingDocRef],
    ) -> Result<u64>;
    /// Searchable documents visible in a timeline whose vector is missing or
    /// stale, in the order they were indexed.
    fn pending_embeddings(&self, query: &PendingQuery) -> Result<PendingPage>;
    fn put_evidence(&mut self, evidence: &EvidenceSource) -> Result<()>;
    fn evidence(&self, project_id: &str, evidence_id: &str) -> Result<EvidenceSource>;
    fn commit_batch(&mut self, commits: &[PreparedCommit]) -> Result<Vec<CommitOutcome>>;
    fn apply_proposal(
        &mut self,
        prepared: &PreparedCommit,
        proposal_id: &str,
        status: ProposalStatus,
    ) -> Result<CommitOutcome>;
    fn timeline(&self, project_id: &str, timeline_id: &str) -> Result<Timeline>;
    fn timelines(&self, project_id: &str, status: Option<TimelineStatus>) -> Result<Vec<Timeline>>;
    /// The rows a read of this timeline may see. Fails like a read for a
    /// promoted or purged draft.
    fn timeline_scope(&self, project_id: &str, timeline: &Timeline) -> Result<TimelineScope>;
    /// Opens a draft from the current head of `parent_id`: main or an active
    /// draft, at most `MAX_DRAFT_DEPTH` drafts below main.
    fn open_draft(
        &mut self,
        project_id: &str,
        timeline_id: &str,
        parent_id: &str,
        label: Option<&str>,
        created_at: &str,
    ) -> Result<Timeline>;
    /// Moves every row of an active draft into its parent, if the parent is
    /// still at the draft's base version. The draft's children move to the
    /// parent and keep their base versions.
    fn promote_draft(
        &mut self,
        project_id: &str,
        timeline_id: &str,
        closed_at: &str,
    ) -> Result<Timeline>;
    fn discard_draft(
        &mut self,
        project_id: &str,
        timeline_id: &str,
        closed_at: &str,
        purge: bool,
    ) -> Result<Timeline>;
    /// Physically removes the rows of a discarded draft.
    fn purge_draft(
        &mut self,
        project_id: &str,
        timeline_id: &str,
        purged_at: &str,
    ) -> Result<PurgeCounts>;
    /// Secret Holdings (with `secrecy`) recorded up to the read position,
    /// whatever their current status, optionally of one holder.
    fn secret_holdings(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        holder: Option<&str>,
        tick: i64,
        as_of_version: Version,
    ) -> Result<Vec<Record>>;
    /// Holdings acquired by being told `source_record_id`, up to the read
    /// position, whatever their current status.
    fn holdings_told_from(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        source_record_id: &str,
        tick: i64,
        as_of_version: Version,
    ) -> Result<Vec<Record>>;
    /// Replays an active draft's commits on top of its parent's current head,
    /// in one transaction: all of them apply again, or nothing changes.
    fn rebase_draft(&mut self, project_id: &str, timeline_id: &str) -> Result<Timeline>;
    /// Validates a commit against the current state without writing it.
    fn check_commit(&mut self, prepared: &PreparedCommit) -> Result<Vec<Warning>>;
    fn replace_proposal(&mut self, proposal: &Proposal) -> Result<()>;
    /// Whether `holder` has an active, available Holding witnessed at
    /// `event_id` at the read position.
    fn witnessed_by(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        holder: &str,
        event_id: &str,
        tick: i64,
        as_of_version: Version,
    ) -> Result<bool>;
    fn event(&self, project_id: &str, scope: &TimelineScope, event_id: &str) -> Result<Event>;
    fn record(&self, project_id: &str, scope: &TimelineScope, record_id: &str) -> Result<Record>;
    fn transition(&self, project_id: &str, transition_id: &str) -> Result<Transition>;
    fn record_state(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        record_id: &str,
        tick: i64,
        as_of_version: Version,
    ) -> Result<RecordState>;
    fn records_at(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        holder: Option<&str>,
        tick: i64,
        as_of_version: Version,
    ) -> Result<Vec<RecordState>>;
    fn query_records(&self, query: &RecordQuery) -> Result<RecordPage>;
    /// The most recent active, available Holding of `holder` for `claim_key`,
    /// read from index columns only.
    fn latest_holding(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        holder_id: &str,
        claim_key: &str,
        tick: i64,
        as_of_version: Version,
    ) -> Result<Option<HoldingSummary>>;
    /// How many active Holdings matching `query` are withheld (suppressed or
    /// inaccessible). Ignores `include_withheld`, `cursor` and `limit`.
    fn count_withheld(&self, query: &RecordQuery) -> Result<u64>;
    fn perspective_records(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        holder_id: &str,
        tick: i64,
        as_of_version: Version,
    ) -> Result<PerspectiveRecords>;
    fn project_window(&self, project_id: &str) -> Result<(Version, i64)>;
    fn entity(&self, project_id: &str, entity_id: &str) -> Result<Entity>;
    fn entities(&self, project_id: &str, limit: u32, cursor: Option<&str>) -> Result<EntityPage>;
    fn predicates(&self, project_id: &str) -> Result<Vec<PredicateDef>>;
    fn transitions_for_record(
        &self,
        project_id: &str,
        scope: &TimelineScope,
        record_id: &str,
    ) -> Result<Vec<Transition>>;
    fn events(&self, query: &EventQuery) -> Result<EventPage>;
    /// The change feed is global; `timeline_id` optionally narrows it.
    fn changes(
        &self,
        project_id: &str,
        since_seq: u64,
        limit: u32,
        timeline_id: Option<&str>,
    ) -> Result<ChangePage>;
    fn search(&self, query: &SearchQuery) -> Result<SearchPage>;
    fn insert_proposal(&mut self, proposal: &Proposal) -> Result<()>;
    fn proposal(&self, proposal_id: &str) -> Result<Proposal>;
    fn proposals(&self, project_id: &str) -> Result<Vec<Proposal>>;
    fn append_reviews(&mut self, proposal_id: &str, reviews: &[ProposalReview])
    -> Result<Proposal>;
    fn set_proposal_status(
        &mut self,
        proposal_id: &str,
        status: ProposalStatus,
        rationale: Option<&str>,
    ) -> Result<Proposal>;
    fn counts(&self, project_id: &str) -> Result<StoreCounts>;
    fn checkpoint(&mut self) -> Result<()>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntityWrite {
    Create(Entity),
    Update(Entity),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreCounts {
    pub events: u64,
    pub records: u64,
    pub transitions: u64,
    pub attempts: u64,
    pub proposals: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordQuery {
    pub project_id: String,
    pub timeline_id: String,
    /// The draft's ancestors (see `TimelineScope`); empty when reading main.
    pub ancestors: Vec<ScopeAncestor>,
    /// Additional axis `tick` is on; `None` for the main axis.
    pub axis: Option<String>,
    pub holder_id: Option<String>,
    pub record_kind: Option<String>,
    pub claim_key: Option<String>,
    pub slot_key: Option<String>,
    pub subject_id: Option<String>,
    pub predicate: Option<String>,
    pub object_entity_id: Option<String>,
    pub about_entity_id: Option<String>,
    pub attitudes: Vec<crate::domain::Attitude>,
    pub minimum_confidence: Option<f64>,
    pub include_withheld: bool,
    pub tick: i64,
    pub as_of_version: Version,
    pub limit: u32,
    pub cursor: Option<RecordCursor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordCursor {
    pub tick: i64,
    pub recorded_version: Version,
    pub record_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordPage {
    pub records: Vec<Record>,
    pub next_cursor: Option<RecordCursor>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoldingSummary {
    pub record_id: String,
    pub attitude: crate::domain::Attitude,
    pub polarity: bool,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PerspectiveRecords {
    pub awareness: Vec<Record>,
    pub dispositions: Vec<Record>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityPage {
    pub entities: Vec<Entity>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventCursor {
    pub tick: i64,
    pub recorded_version: Version,
    pub event_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventQuery {
    pub project_id: String,
    pub timeline_id: String,
    /// The draft's ancestors (see `TimelineScope`); empty when reading main.
    pub ancestors: Vec<ScopeAncestor>,
    /// Additional axis `tick` is on; `None` for the main axis. On an axis the
    /// page is ordered by the axis position and the cursor's tick is one.
    pub axis: Option<String>,
    pub tick: i64,
    pub as_of_version: Version,
    pub limit: u32,
    pub cursor: Option<EventCursor>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventPage {
    pub events: Vec<Event>,
    pub next_cursor: Option<EventCursor>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChangePage {
    pub changes: Vec<ChangeItem>,
    pub next_seq: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchQuery {
    pub project_id: String,
    pub timeline_id: String,
    /// The draft's ancestors (see `TimelineScope`); empty when reading main.
    pub ancestors: Vec<ScopeAncestor>,
    /// Additional axis `tick` is on; `None` for the main axis.
    pub axis: Option<String>,
    /// Full-text query; empty for a semantic-only search.
    pub text: String,
    /// The semantic part, with a normalized vector of the space's size.
    pub semantic: Option<SemanticQuery>,
    pub holder_id: Option<String>,
    pub doc_type: Option<String>,
    pub tick: i64,
    pub as_of_version: Version,
    pub limit: u32,
    pub offset: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticQuery {
    pub space_id: String,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingQuery {
    pub project_id: String,
    pub timeline_id: String,
    /// The draft's ancestors (see `TimelineScope`); empty when reading main.
    pub ancestors: Vec<ScopeAncestor>,
    pub space_id: String,
    /// `record` and/or `event`; empty for both.
    pub doc_types: Vec<String>,
    /// Only documents indexed after this position.
    pub after: i64,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingPage {
    pub items: Vec<PendingEmbedding>,
    /// Position to continue from when more pending documents follow.
    pub next: Option<i64>,
    /// Position a later call can resume from to see only newer documents.
    pub resume: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchPage {
    pub items: Vec<SearchItem>,
    pub next_offset: Option<u64>,
}

pub fn latest_reviews(
    reviews: &[ProposalReview],
) -> std::collections::BTreeMap<usize, ReviewInput> {
    reviews
        .iter()
        .map(|review| {
            (
                review.operation_index,
                ReviewInput {
                    operation_index: review.operation_index,
                    decision: review.decision,
                    rationale: review.rationale.clone(),
                },
            )
        })
        .collect()
}
