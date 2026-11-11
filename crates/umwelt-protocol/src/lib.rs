// SPDX-License-Identifier: AGPL-3.0-only
//! Umwelt JSON protocol v1 dispatcher.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use umwelt_api::*;

pub const UMWELT_PROTOCOL_VERSION: u32 = 1;
pub const UMWELT_STORAGE_VERSION: u32 = STORAGE_VERSION;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    pub v: u32,
    pub id: String,
    pub op: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum ResponseEnvelope {
    Success {
        v: u32,
        id: Option<String>,
        ok: bool,
        result: Value,
    },
    Failure {
        v: u32,
        id: Option<String>,
        ok: bool,
        error: PublicError,
    },
}

impl ResponseEnvelope {
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct SuccessEnvelope {
    pub v: u32,
    pub id: Option<String>,
    pub ok: bool,
    pub result: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct FailureEnvelope {
    pub v: u32,
    pub id: Option<String>,
    pub ok: bool,
    pub error: PublicError,
}

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct PublicError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    ValidationFailed,
    NotFound,
    VersionConflict,
    Conflict,
    InvalidState,
    HandshakeRequired,
    UnsupportedProtocol,
    UnknownOp,
    FileLocked,
    NotUmweltFile,
    UnsupportedFileVersion,
    PayloadTooLarge,
    Internal,
}

impl ErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ValidationFailed => "VALIDATION_FAILED",
            Self::NotFound => "NOT_FOUND",
            Self::VersionConflict => "VERSION_CONFLICT",
            Self::Conflict => "CONFLICT",
            Self::InvalidState => "INVALID_STATE",
            Self::HandshakeRequired => "HANDSHAKE_REQUIRED",
            Self::UnsupportedProtocol => "UNSUPPORTED_PROTOCOL",
            Self::UnknownOp => "UNKNOWN_OP",
            Self::FileLocked => "FILE_LOCKED",
            Self::NotUmweltFile => "NOT_UMWELT_FILE",
            Self::UnsupportedFileVersion => "UNSUPPORTED_FILE_VERSION",
            Self::PayloadTooLarge => "PAYLOAD_TOO_LARGE",
            Self::Internal => "INTERNAL",
        }
    }
}

pub type ProtocolErrorCode = ErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityKind {
    Command,
    Query,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Capability {
    pub name: &'static str,
    pub kind: CapabilityKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProtocolRange {
    pub min: u32,
    pub max: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HelloPayload {
    pub protocol: ProtocolRange,
    pub client: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HelloResult {
    pub protocol: u32,
    pub contract: String,
    pub storage: u32,
    pub engine: String,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmptyPayload {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommitBatchPayload {
    pub commits: Vec<CommitInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransmitBatchPayload {
    /// What the transmits have in common, written once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared: Option<TransmitShared>,
    pub transmits: Vec<TransmitBatchItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaimKeyPayload {
    pub claim: Claim,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DispositionsPayload {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    pub holder: String,
    /// Only the Dispositions toward this entity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Only this dimension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimension: Option<String>,
    #[serde(default)]
    pub at: At,
}

pub type ProjectListResult = Vec<Project>;
pub type EntityTypeListResult = Vec<EntityType>;
pub type PredicateListResult = Vec<PredicateDef>;
pub type AxisListResult = Vec<AxisDef>;
pub type EmbeddingSpaceListResult = Vec<EmbeddingSpace>;
pub type ProposalListResult = Vec<Proposal>;
pub type CommitBatchResult = Vec<CommitOutcome>;
pub type TransmitBatchResult = Vec<CommitOutcome>;
pub type WhoHoldsResult = Vec<WhoHoldsItem>;
pub type ContradictionsResult = Vec<Contradiction>;
pub type BeliefsAboutResult = Vec<BeliefAboutItem>;
pub type BreachesResult = Vec<Breach>;
pub type EnsureEntitiesResult = Vec<EnsureEntityResult>;
pub type ResolveEntitiesResult = Vec<ResolvedEntity>;
pub type TimelineListResult = Vec<Timeline>;
pub type DispositionsResult = Vec<Record>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationSchema {
    pub name: &'static str,
    pub kind: CapabilityKind,
    pub payload: &'static str,
    pub result: &'static str,
}

pub const OP_SCHEMAS: &[OperationSchema] = &[
    op(
        "project.create",
        CapabilityKind::Command,
        "CreateProjectInput",
        "Project",
    ),
    op("project.get", CapabilityKind::Query, "ProjectId", "Project"),
    op(
        "project.list",
        CapabilityKind::Query,
        "EmptyPayload",
        "ProjectListResult",
    ),
    op(
        "entityType.create",
        CapabilityKind::Command,
        "CreateEntityTypeInput",
        "EntityType",
    ),
    op(
        "entityType.list",
        CapabilityKind::Query,
        "ProjectId",
        "EntityTypeListResult",
    ),
    op(
        "entity.create",
        CapabilityKind::Command,
        "CreateEntityInput",
        "Entity",
    ),
    op(
        "entity.update",
        CapabilityKind::Command,
        "UpdateEntityInput",
        "Entity",
    ),
    op(
        "entity.archive",
        CapabilityKind::Command,
        "EntityId",
        "Entity",
    ),
    op("entity.get", CapabilityKind::Query, "EntityId", "Entity"),
    op(
        "entity.list",
        CapabilityKind::Query,
        "PagedProject",
        "EntityPageResult",
    ),
    op(
        "predicate.define",
        CapabilityKind::Command,
        "DefinePredicateInput",
        "PredicateDef",
    ),
    op(
        "predicate.list",
        CapabilityKind::Query,
        "ProjectId",
        "PredicateListResult",
    ),
    op(
        "axis.define",
        CapabilityKind::Command,
        "DefineAxisInput",
        "AxisDef",
    ),
    op(
        "axis.list",
        CapabilityKind::Query,
        "ProjectId",
        "AxisListResult",
    ),
    op(
        "embedding.defineSpace",
        CapabilityKind::Command,
        "DefineEmbeddingSpaceInput",
        "EmbeddingSpace",
    ),
    op(
        "embedding.listSpaces",
        CapabilityKind::Query,
        "ProjectId",
        "EmbeddingSpaceListResult",
    ),
    op(
        "embedding.dropSpace",
        CapabilityKind::Command,
        "EmbeddingSpaceRef",
        "DropEmbeddingSpaceResult",
    ),
    op(
        "embedding.pending",
        CapabilityKind::Query,
        "PendingEmbeddingsInput",
        "PendingEmbeddingsResult",
    ),
    op(
        "embedding.put",
        CapabilityKind::Command,
        "PutEmbeddingsInput",
        "PutEmbeddingsResult",
    ),
    op(
        "embedding.delete",
        CapabilityKind::Command,
        "DeleteEmbeddingsInput",
        "DeleteEmbeddingsResult",
    ),
    op(
        "evidence.register",
        CapabilityKind::Command,
        "EvidenceSource",
        "EvidenceSource",
    ),
    op(
        "evidence.get",
        CapabilityKind::Query,
        "EvidenceId",
        "EvidenceSource",
    ),
    op(
        "commit",
        CapabilityKind::Command,
        "CommitInput",
        "CommitOutcome",
    ),
    op(
        "commitBatch",
        CapabilityKind::Command,
        "CommitBatchPayload",
        "CommitBatchResult",
    ),
    op(
        "propose",
        CapabilityKind::Command,
        "ProposeInput",
        "Proposal",
    ),
    op(
        "proposal.review",
        CapabilityKind::Command,
        "ReviewPayload",
        "Proposal",
    ),
    op(
        "proposal.apply",
        CapabilityKind::Command,
        "ApplyPayload",
        "CommitOutcome",
    ),
    op(
        "proposal.reject",
        CapabilityKind::Command,
        "RejectPayload",
        "Proposal",
    ),
    op(
        "proposal.get",
        CapabilityKind::Query,
        "ProposalId",
        "Proposal",
    ),
    op(
        "proposal.list",
        CapabilityKind::Query,
        "ProjectId",
        "ProposalListResult",
    ),
    op(
        "transmit",
        CapabilityKind::Command,
        "TransmitInput",
        "CommitOutcome",
    ),
    op(
        "transmitBatch",
        CapabilityKind::Command,
        "TransmitBatchPayload",
        "TransmitBatchResult",
    ),
    op(
        "timeline.openDraft",
        CapabilityKind::Command,
        "OpenDraftInput",
        "Timeline",
    ),
    op(
        "timeline.promote",
        CapabilityKind::Command,
        "TimelineRef",
        "Timeline",
    ),
    op(
        "timeline.discard",
        CapabilityKind::Command,
        "DiscardPayload",
        "Timeline",
    ),
    op(
        "timeline.purge",
        CapabilityKind::Command,
        "TimelineRef",
        "PurgeResult",
    ),
    op(
        "timeline.get",
        CapabilityKind::Query,
        "TimelineRef",
        "Timeline",
    ),
    op(
        "timeline.list",
        CapabilityKind::Query,
        "TimelineListPayload",
        "TimelineListResult",
    ),
    op(
        "holds",
        CapabilityKind::Query,
        "HoldsPayload",
        "HoldsResult",
    ),
    op(
        "holdings",
        CapabilityKind::Query,
        "HoldingsPayload",
        "HoldingsResult",
    ),
    op(
        "whoHolds",
        CapabilityKind::Query,
        "WhoHoldsPayload",
        "WhoHoldsResult",
    ),
    op(
        "canon",
        CapabilityKind::Query,
        "CanonPayload",
        "RecordPageResult",
    ),
    op(
        "perspective",
        CapabilityKind::Query,
        "PerspectivePayload",
        "PerspectiveResult",
    ),
    op(
        "compare",
        CapabilityKind::Query,
        "ComparePayload",
        "CompareResult",
    ),
    op(
        "contradictions",
        CapabilityKind::Query,
        "HolderAt",
        "ContradictionsResult",
    ),
    op("trace", CapabilityKind::Query, "RecordId", "TraceResult"),
    op(
        "timeline",
        CapabilityKind::Query,
        "TimelinePayload",
        "EventPageResult",
    ),
    op("record", CapabilityKind::Query, "RecordId", "Record"),
    op(
        "changes",
        CapabilityKind::Query,
        "ChangesPayload",
        "ChangePageResult",
    ),
    op(
        "search",
        CapabilityKind::Query,
        "SearchPayload",
        "SearchPageResult",
    ),
    op(
        "entity.ensureBatch",
        CapabilityKind::Command,
        "EnsureEntitiesInput",
        "EnsureEntitiesResult",
    ),
    op(
        "entity.resolve",
        CapabilityKind::Query,
        "ResolveEntityPayload",
        "Entity",
    ),
    op(
        "entity.resolveBatch",
        CapabilityKind::Query,
        "ResolveEntitiesPayload",
        "ResolveEntitiesResult",
    ),
    op(
        "witness",
        CapabilityKind::Command,
        "WitnessInput",
        "CommitOutcome",
    ),
    op(
        "groupCoverage",
        CapabilityKind::Query,
        "GroupCoveragePayload",
        "GroupCoverageResult",
    ),
    op(
        "holding.adjustBatch",
        CapabilityKind::Command,
        "AdjustHoldingsInput",
        "CommitOutcome",
    ),
    op(
        "transition.batch",
        CapabilityKind::Command,
        "TransitionBatchInput",
        "CommitOutcome",
    ),
    op(
        "disposition.setBatch",
        CapabilityKind::Command,
        "SetDispositionsInput",
        "CommitOutcome",
    ),
    op(
        "entity.at",
        CapabilityKind::Query,
        "EntityAtPayload",
        "EntityAtResult",
    ),
    op(
        "entity.asKnownBy",
        CapabilityKind::Query,
        "EntityKnownPayload",
        "EntityKnownResult",
    ),
    op(
        "beliefsAbout",
        CapabilityKind::Query,
        "BeliefsAboutPayload",
        "BeliefsAboutResult",
    ),
    op(
        "breaches",
        CapabilityKind::Query,
        "BreachesPayload",
        "BreachesResult",
    ),
    op(
        "check.utterance",
        CapabilityKind::Query,
        "CheckUtteranceInput",
        "CheckUtteranceResult",
    ),
    op(
        "context.admit",
        CapabilityKind::Query,
        "ContextAdmitInput",
        "ContextAdmitResult",
    ),
    op(
        "proposal.rebase",
        CapabilityKind::Command,
        "ProposalId",
        "ProposalRebaseResult",
    ),
    op(
        "timeline.rebase",
        CapabilityKind::Command,
        "TimelineRef",
        "Timeline",
    ),
    op(
        "claim.key",
        CapabilityKind::Query,
        "ClaimKeyPayload",
        "ClaimKeyResult",
    ),
    op(
        "world.ensure",
        CapabilityKind::Command,
        "EnsureWorldInput",
        "EnsureWorldResult",
    ),
    op(
        "dispositions",
        CapabilityKind::Query,
        "DispositionsPayload",
        "DispositionsResult",
    ),
    op(
        "shutdown",
        CapabilityKind::Command,
        "EmptyPayload",
        "EmptyPayload",
    ),
];

const fn op(
    name: &'static str,
    kind: CapabilityKind,
    payload: &'static str,
    result: &'static str,
) -> OperationSchema {
    OperationSchema {
        name,
        kind,
        payload,
        result,
    }
}

pub const CAPABILITIES: &[Capability] = &[
    command("project.create"),
    query("project.get"),
    query("project.list"),
    command("entityType.create"),
    query("entityType.list"),
    command("entity.create"),
    command("entity.update"),
    command("entity.archive"),
    query("entity.get"),
    query("entity.list"),
    command("predicate.define"),
    query("predicate.list"),
    command("axis.define"),
    query("axis.list"),
    command("embedding.defineSpace"),
    query("embedding.listSpaces"),
    command("embedding.dropSpace"),
    query("embedding.pending"),
    command("embedding.put"),
    command("embedding.delete"),
    command("evidence.register"),
    query("evidence.get"),
    command("commit"),
    command("commitBatch"),
    command("propose"),
    command("proposal.review"),
    command("proposal.apply"),
    command("proposal.reject"),
    query("proposal.get"),
    query("proposal.list"),
    command("transmit"),
    command("transmitBatch"),
    command("timeline.openDraft"),
    command("timeline.promote"),
    command("timeline.discard"),
    command("timeline.purge"),
    query("timeline.get"),
    query("timeline.list"),
    query("holds"),
    query("holdings"),
    query("whoHolds"),
    query("canon"),
    query("perspective"),
    query("compare"),
    query("contradictions"),
    query("trace"),
    query("timeline"),
    query("record"),
    query("changes"),
    query("search"),
    command("entity.ensureBatch"),
    query("entity.resolve"),
    query("entity.resolveBatch"),
    command("witness"),
    query("groupCoverage"),
    command("holding.adjustBatch"),
    command("transition.batch"),
    command("disposition.setBatch"),
    query("entity.at"),
    query("entity.asKnownBy"),
    query("beliefsAbout"),
    query("breaches"),
    query("check.utterance"),
    query("context.admit"),
    command("proposal.rebase"),
    command("timeline.rebase"),
    query("claim.key"),
    command("world.ensure"),
    query("dispositions"),
    command("shutdown"),
];
const fn command(name: &'static str) -> Capability {
    Capability {
        name,
        kind: CapabilityKind::Command,
    }
}
const fn query(name: &'static str) -> Capability {
    Capability {
        name,
        kind: CapabilityKind::Query,
    }
}

pub struct Dispatcher<S: Store> {
    engine: Engine<S>,
    handshaken: bool,
    shutdown_requested: bool,
}

impl<S: Store> Dispatcher<S> {
    pub fn new(engine: Engine<S>) -> Self {
        Self {
            engine,
            handshaken: false,
            shutdown_requested: false,
        }
    }
    pub fn shutdown_requested(&self) -> bool {
        self.shutdown_requested
    }
    pub fn shutdown(&mut self) -> Result<(), PublicError> {
        self.engine.checkpoint().map_err(map_engine_error)
    }

    pub fn handle_line(&mut self, line: &str) -> ResponseEnvelope {
        let partial_id = extract_id(line);
        let request = match serde_json::from_str::<RequestEnvelope>(line) {
            Ok(request) => request,
            Err(error) => {
                return failure(
                    partial_id,
                    ProtocolErrorCode::ValidationFailed,
                    format!("Invalid request JSON: {error}"),
                    None,
                );
            }
        };
        let id = Some(request.id.clone());
        if request.v != 1 {
            return failure(
                id,
                ProtocolErrorCode::UnsupportedProtocol,
                "The requested protocol version is not supported.",
                Some(json!({"supported":{"min":1,"max":1}})),
            );
        }
        if !self.handshaken {
            if request.op != "hello" {
                return failure(
                    id,
                    ProtocolErrorCode::HandshakeRequired,
                    "The first request must be hello.",
                    None,
                );
            }
            return self.hello(id, request.payload);
        }
        if request.op == "hello" {
            return failure(
                id,
                ProtocolErrorCode::Conflict,
                "The protocol handshake is already complete.",
                None,
            );
        }
        match self.dispatch(&request.op, request.payload) {
            Ok(result) => success(id, result),
            Err(error) => ResponseEnvelope::Failure {
                v: 1,
                id,
                ok: false,
                error,
            },
        }
    }

    fn hello(&mut self, id: Option<String>, payload: Value) -> ResponseEnvelope {
        let hello: HelloPayload = match parse(payload) {
            Ok(value) => value,
            Err(error) => {
                return ResponseEnvelope::Failure {
                    v: 1,
                    id,
                    ok: false,
                    error,
                };
            }
        };
        if hello.client.trim().is_empty() {
            return failure(
                id,
                ProtocolErrorCode::ValidationFailed,
                "hello.client must not be empty.",
                None,
            );
        }
        if hello.protocol.min > 1 || hello.protocol.max < 1 {
            return failure(
                id,
                ProtocolErrorCode::UnsupportedProtocol,
                "The requested protocol range is not supported.",
                Some(json!({"supported":{"min":1,"max":1}})),
            );
        }
        self.handshaken = true;
        match value_json(HelloResult {
            protocol: 1,
            contract: UMWELT_CONTRACT_VERSION.to_owned(),
            storage: UMWELT_STORAGE_VERSION,
            engine: env!("CARGO_PKG_VERSION").to_owned(),
            capabilities: CAPABILITIES.to_vec(),
        }) {
            Ok(result) => success(id, result),
            Err(error) => ResponseEnvelope::Failure {
                v: 1,
                id,
                ok: false,
                error,
            },
        }
    }

    fn dispatch(&mut self, op: &str, payload: Value) -> Result<Value, PublicError> {
        match op {
            "project.create" => {
                let v = self
                    .engine
                    .create_project(parse(payload)?)
                    .map_err(map_engine_error)?;
                value_json(v)
            }
            "project.get" => {
                let p: ProjectId = parse(payload)?;
                value_json(
                    self.engine
                        .project(&p.project_id)
                        .map_err(map_engine_error)?,
                )
            }
            "project.list" => {
                let _: EmptyPayload = parse(payload)?;
                value_json(self.engine.projects().map_err(map_engine_error)?)
            }
            "entityType.create" => {
                let v = self
                    .engine
                    .create_entity_type(parse(payload)?)
                    .map_err(map_engine_error)?;
                value_json(v)
            }
            "entityType.list" => {
                let p: ProjectId = parse(payload)?;
                value_json(
                    self.engine
                        .entity_types(&p.project_id)
                        .map_err(map_engine_error)?,
                )
            }
            "entity.create" => {
                let v = self
                    .engine
                    .create_entity(parse(payload)?)
                    .map_err(map_engine_error)?;
                value_json(v)
            }
            "entity.update" => {
                let v = self
                    .engine
                    .update_entity(parse(payload)?)
                    .map_err(map_engine_error)?;
                value_json(v)
            }
            "entity.archive" => {
                let p: EntityId = parse(payload)?;
                value_json(
                    self.engine
                        .archive_entity(&p.project_id, &p.entity_id)
                        .map_err(map_engine_error)?,
                )
            }
            "entity.get" => {
                let p: EntityId = parse(payload)?;
                value_json(
                    self.engine
                        .entity(&p.project_id, &p.entity_id)
                        .map_err(map_engine_error)?,
                )
            }
            "entity.list" => {
                let p: PagedProject = parse(payload)?;
                value_json(
                    self.engine
                        .entities(&p.project_id, p.page)
                        .map_err(map_engine_error)?,
                )
            }
            "predicate.define" => {
                let value = self
                    .engine
                    .define_predicate(parse(payload)?)
                    .map_err(map_engine_error)?;
                value_json(value)
            }
            "predicate.list" => {
                let p: ProjectId = parse(payload)?;
                value_json(
                    self.engine
                        .predicates(&p.project_id)
                        .map_err(map_engine_error)?,
                )
            }
            "axis.define" => {
                let value = self
                    .engine
                    .define_axis(parse(payload)?)
                    .map_err(map_engine_error)?;
                value_json(value)
            }
            "axis.list" => {
                let p: ProjectId = parse(payload)?;
                value_json(self.engine.axes(&p.project_id).map_err(map_engine_error)?)
            }
            "embedding.defineSpace" => value_json(
                self.engine
                    .define_embedding_space(parse(payload)?)
                    .map_err(map_engine_error)?,
            ),
            "embedding.listSpaces" => {
                let p: ProjectId = parse(payload)?;
                value_json(
                    self.engine
                        .embedding_spaces(&p.project_id)
                        .map_err(map_engine_error)?,
                )
            }
            "embedding.dropSpace" => {
                let p: EmbeddingSpaceRef = parse(payload)?;
                value_json(
                    self.engine
                        .drop_embedding_space(&p.project_id, &p.space_id)
                        .map_err(map_engine_error)?,
                )
            }
            "embedding.pending" => {
                let p: PendingEmbeddingsInput = parse(payload)?;
                value_json(
                    self.engine
                        .pending_embeddings(&p)
                        .map_err(map_engine_error)?,
                )
            }
            "embedding.put" => value_json(
                self.engine
                    .put_embeddings(parse(payload)?)
                    .map_err(map_engine_error)?,
            ),
            "embedding.delete" => value_json(
                self.engine
                    .delete_embeddings(parse(payload)?)
                    .map_err(map_engine_error)?,
            ),
            "evidence.register" => {
                let v: EvidenceSource = parse(payload)?;
                self.engine
                    .put_evidence(v.clone())
                    .map_err(map_engine_error)?;
                value_json(v)
            }
            "evidence.get" => {
                let p: EvidenceId = parse(payload)?;
                value_json(
                    self.engine
                        .evidence(&p.project_id, &p.evidence_id)
                        .map_err(map_engine_error)?,
                )
            }
            "commit" => value_json(
                self.engine
                    .commit(parse(payload)?)
                    .map_err(map_engine_error)?,
            ),
            "commitBatch" => value_json(
                self.engine
                    .commit_batch(parse::<CommitBatchPayload>(payload)?.commits)
                    .map_err(map_engine_error)?,
            ),
            "propose" => value_json(
                self.engine
                    .propose(parse(payload)?)
                    .map_err(map_engine_error)?,
            ),
            "proposal.review" => {
                let p: ReviewPayload = parse(payload)?;
                value_json(
                    self.engine
                        .review(&p.proposal_id, p.reviews)
                        .map_err(map_engine_error)?,
                )
            }
            "proposal.apply" => {
                let p: ApplyPayload = parse(payload)?;
                value_json(
                    self.engine
                        .apply(&p.proposal_id, p.attempt_id)
                        .map_err(map_engine_error)?,
                )
            }
            "proposal.reject" => {
                let p: RejectPayload = parse(payload)?;
                value_json(
                    self.engine
                        .reject(&p.proposal_id, &p.rationale)
                        .map_err(map_engine_error)?,
                )
            }
            "proposal.get" => {
                let p: ProposalId = parse(payload)?;
                value_json(
                    self.engine
                        .proposal(&p.proposal_id)
                        .map_err(map_engine_error)?,
                )
            }
            "proposal.list" => {
                let p: ProjectId = parse(payload)?;
                value_json(
                    self.engine
                        .proposals(&p.project_id)
                        .map_err(map_engine_error)?,
                )
            }
            "transmit" => value_json(
                self.engine
                    .transmit(parse(payload)?)
                    .map_err(map_engine_error)?,
            ),
            "transmitBatch" => {
                let p: TransmitBatchPayload = parse(payload)?;
                value_json(
                    self.engine
                        .transmit_batch_shared(&p.shared.unwrap_or_default(), p.transmits)
                        .map_err(map_engine_error)?,
                )
            }
            "timeline.openDraft" => value_json(
                self.engine
                    .open_draft(parse(payload)?)
                    .map_err(map_engine_error)?,
            ),
            "timeline.promote" => {
                let p: TimelineRef = parse(payload)?;
                value_json(
                    self.engine
                        .promote_draft(&p.project_id, &p.timeline_id)
                        .map_err(map_engine_error)?,
                )
            }
            "timeline.discard" => {
                let p: DiscardPayload = parse(payload)?;
                value_json(
                    self.engine
                        .discard_draft(&p.project_id, &p.timeline_id, p.purge)
                        .map_err(map_engine_error)?,
                )
            }
            "timeline.purge" => {
                let p: TimelineRef = parse(payload)?;
                value_json(
                    self.engine
                        .purge_draft(&p.project_id, &p.timeline_id)
                        .map_err(map_engine_error)?,
                )
            }
            "timeline.get" => {
                let p: TimelineRef = parse(payload)?;
                value_json(
                    self.engine
                        .get_timeline(&p.project_id, &p.timeline_id)
                        .map_err(map_engine_error)?,
                )
            }
            "timeline.list" => {
                let p: TimelineListPayload = parse(payload)?;
                value_json(
                    self.engine
                        .timelines(&p.project_id, p.status)
                        .map_err(map_engine_error)?,
                )
            }
            "holds" => {
                let p: HoldsPayload = parse(payload)?;
                let claim_ref = p.claim_ref()?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .holds(&p.project_id, &p.holder, claim_ref, p.at)
                        .map_err(map_engine_error)?,
                )
            }
            "holdings" => {
                let p: HoldingsPayload = parse(payload)?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .holdings(&p.project_id, &p.holder, p.filter, p.at, p.page)
                        .map_err(map_engine_error)?,
                )
            }
            "whoHolds" => {
                let p: WhoHoldsPayload = parse(payload)?;
                let claim_ref = p.claim_ref()?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .who_holds(&p.project_id, claim_ref, p.at, p.filter)
                        .map_err(map_engine_error)?,
                )
            }
            "canon" => {
                let p: CanonPayload = parse(payload)?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .canon(&p.project_id, p.filter, p.at, p.page)
                        .map_err(map_engine_error)?,
                )
            }
            "perspective" => {
                let p: PerspectivePayload = parse(payload)?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .perspective(&p.project_id, &p.holder, p.at, p.page)
                        .map_err(map_engine_error)?,
                )
            }
            "compare" => {
                let p: ComparePayload = parse(payload)?;
                let at_b = p.at_b.unwrap_or_else(|| p.at.clone());
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .compare_across(&p.project_id, &p.a, &p.b, p.at, at_b)
                        .map_err(map_engine_error)?,
                )
            }
            "contradictions" => {
                let p: HolderAt = parse(payload)?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .contradictions(&p.project_id, &p.holder, p.at)
                        .map_err(map_engine_error)?,
                )
            }
            "trace" => {
                let p: RecordId = parse(payload)?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .trace(&p.project_id, &p.record_id)
                        .map_err(map_engine_error)?,
                )
            }
            "timeline" => {
                let p: TimelinePayload = parse(payload)?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .timeline(&p.project_id, p.at, p.page)
                        .map_err(map_engine_error)?,
                )
            }
            "record" => {
                let p: RecordId = parse(payload)?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .record(&p.project_id, &p.record_id)
                        .map_err(map_engine_error)?,
                )
            }
            "changes" => {
                let p: ChangesPayload = parse(payload)?;
                value_json(
                    self.engine
                        .changes_in(
                            &p.project_id,
                            p.since_seq,
                            p.limit,
                            p.timeline_id.as_deref(),
                        )
                        .map_err(map_engine_error)?,
                )
            }
            "search" => {
                let p: SearchPayload = parse(payload)?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .search_with(
                            &p.project_id,
                            p.text.as_deref(),
                            p.semantic,
                            p.filter,
                            p.at,
                            p.page,
                        )
                        .map_err(map_engine_error)?,
                )
            }
            "entity.ensureBatch" => value_json(
                self.engine
                    .ensure_entities(parse(payload)?)
                    .map_err(map_engine_error)?,
            ),
            "entity.resolve" => {
                let p: ResolveEntityPayload = parse(payload)?;
                value_json(
                    self.engine
                        .resolve_entity(&p.project_id, &p.external_ref)
                        .map_err(map_engine_error)?,
                )
            }
            "entity.resolveBatch" => {
                let p: ResolveEntitiesPayload = parse(payload)?;
                value_json(
                    self.engine
                        .resolve_entities(&p.project_id, p.external_refs)
                        .map_err(map_engine_error)?,
                )
            }
            "witness" => value_json(
                self.engine
                    .witness(parse(payload)?)
                    .map_err(map_engine_error)?,
            ),
            "groupCoverage" => {
                let p: GroupCoveragePayload = parse(payload)?;
                let claim_ref = claim_ref(p.claim, p.claim_key)?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .group_coverage(&p.project_id, &p.group, claim_ref, p.at)
                        .map_err(map_engine_error)?,
                )
            }
            "holding.adjustBatch" => value_json(
                self.engine
                    .adjust_holdings(parse(payload)?)
                    .map_err(map_engine_error)?,
            ),
            "transition.batch" => value_json(
                self.engine
                    .transition_batch(parse(payload)?)
                    .map_err(map_engine_error)?,
            ),
            "disposition.setBatch" => value_json(
                self.engine
                    .set_dispositions(parse(payload)?)
                    .map_err(map_engine_error)?,
            ),
            "entity.at" => {
                let p: EntityAtPayload = parse(payload)?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .entity_at(&p.project_id, &p.entity_id, p.at)
                        .map_err(map_engine_error)?,
                )
            }
            "entity.asKnownBy" => {
                let p: EntityKnownPayload = parse(payload)?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .entity_known_by(&p.project_id, &p.entity_id, &p.holder, p.at)
                        .map_err(map_engine_error)?,
                )
            }
            "beliefsAbout" => {
                let p: BeliefsAboutPayload = parse(payload)?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .beliefs_about(&p.project_id, &p.holder, &p.about, p.at)
                        .map_err(map_engine_error)?,
                )
            }
            "breaches" => {
                let p: BreachesPayload = parse(payload)?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .breaches(&p.project_id, p.owner.as_deref(), p.at)
                        .map_err(map_engine_error)?,
                )
            }
            "check.utterance" => {
                let p: CheckUtteranceInput = parse(payload)?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .check_utterance(&p.project_id, &p.holder, &p.claims, p.at)
                        .map_err(map_engine_error)?,
                )
            }
            "context.admit" => {
                let p: ContextAdmitInput = parse(payload)?;
                value_json(self.engine.context_admit(&p).map_err(map_engine_error)?)
            }
            "proposal.rebase" => {
                let p: ProposalId = parse(payload)?;
                value_json(
                    self.engine
                        .rebase_proposal(&p.proposal_id)
                        .map_err(map_engine_error)?,
                )
            }
            "timeline.rebase" => {
                let p: TimelineRef = parse(payload)?;
                value_json(
                    self.engine
                        .rebase_draft(&p.project_id, &p.timeline_id)
                        .map_err(map_engine_error)?,
                )
            }
            "claim.key" => {
                let p: ClaimKeyPayload = parse(payload)?;
                value_json(ClaimKeyResult::of(&p.claim))
            }
            "world.ensure" => value_json(
                self.engine
                    .ensure_world(parse(payload)?)
                    .map_err(map_engine_error)?,
            ),
            "dispositions" => {
                let p: DispositionsPayload = parse(payload)?;
                value_json(
                    self.engine
                        .in_timeline(timeline(&p.timeline_id))
                        .dispositions(
                            &p.project_id,
                            &p.holder,
                            p.target.as_deref(),
                            p.dimension.as_deref(),
                            p.at,
                        )
                        .map_err(map_engine_error)?,
                )
            }
            "shutdown" => {
                let _: EmptyPayload = parse(payload)?;
                self.shutdown()?;
                self.shutdown_requested = true;
                Ok(json!({}))
            }
            _ => Err(PublicError {
                code: ProtocolErrorCode::UnknownOp,
                message: format!("Unknown operation '{op}'."),
                details: Some(json!({"op":op})),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectId {
    pub project_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PagedProject {
    pub project_id: String,
    #[serde(default)]
    pub page: Page,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntityId {
    pub project_id: String,
    pub entity_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceId {
    pub project_id: String,
    pub evidence_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordId {
    pub project_id: String,
    pub record_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProposalId {
    pub proposal_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewPayload {
    pub proposal_id: String,
    pub reviews: Vec<ReviewInput>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyPayload {
    pub proposal_id: String,
    pub attempt_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RejectPayload {
    pub proposal_id: String,
    pub rationale: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HoldsPayload {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    pub holder: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim: Option<Claim>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_key: Option<String>,
    #[serde(default)]
    pub at: At,
}

impl HoldsPayload {
    fn claim_ref(&self) -> Result<ClaimRef, PublicError> {
        claim_ref(self.claim.clone(), self.claim_key.clone())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HoldingsPayload {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    pub holder: String,
    #[serde(default)]
    pub filter: HoldingFilter,
    #[serde(default)]
    pub at: At,
    #[serde(default)]
    pub page: Page,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WhoHoldsPayload {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim: Option<Claim>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_key: Option<String>,
    #[serde(default)]
    pub at: At,
    #[serde(default)]
    pub filter: WhoHoldsFilter,
}

impl WhoHoldsPayload {
    fn claim_ref(&self) -> Result<ClaimRef, PublicError> {
        claim_ref(self.claim.clone(), self.claim_key.clone())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CanonPayload {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    #[serde(default)]
    pub filter: CanonFilter,
    #[serde(default)]
    pub at: At,
    #[serde(default)]
    pub page: Page,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PerspectivePayload {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    pub holder: String,
    #[serde(default)]
    pub at: At,
    #[serde(default)]
    pub page: Page,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComparePayload {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    pub a: String,
    pub b: String,
    #[serde(default)]
    pub at: At,
    /// Where `b` is read; defaults to `at`. It may be on another axis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_b: Option<At>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HolderAt {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    pub holder: String,
    #[serde(default)]
    pub at: At,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimelinePayload {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    #[serde(default)]
    pub at: At,
    #[serde(default)]
    pub page: Page,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChangesPayload {
    pub project_id: String,
    /// Narrows the feed to one timeline. The feed is global by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    #[serde(default)]
    pub since_seq: u64,
    #[serde(default = "default_limit")]
    pub limit: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchPayload {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    /// Full-text query. At least one of `text` and `semantic` is required;
    /// with both, the search is hybrid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// A query vector from the client's model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic: Option<SearchSemantic>,
    #[serde(default)]
    pub filter: SearchFilter,
    #[serde(default)]
    pub at: At,
    #[serde(default)]
    pub page: Page,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmbeddingSpaceRef {
    pub project_id: String,
    pub space_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimelineRef {
    pub project_id: String,
    pub timeline_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscardPayload {
    pub project_id: String,
    pub timeline_id: String,
    /// Also remove the draft's rows (`timeline.purge`) in the same call.
    #[serde(default)]
    pub purge: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimelineListPayload {
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<TimelineStatus>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveEntityPayload {
    pub project_id: String,
    pub external_ref: ExternalRef,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveEntitiesPayload {
    pub project_id: String,
    pub external_refs: Vec<ExternalRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GroupCoveragePayload {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    pub group: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim: Option<Claim>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_key: Option<String>,
    #[serde(default)]
    pub at: At,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntityAtPayload {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    pub entity_id: String,
    #[serde(default)]
    pub at: At,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntityKnownPayload {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    pub entity_id: String,
    pub holder: String,
    #[serde(default)]
    pub at: At,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BeliefsAboutPayload {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    pub holder: String,
    /// The other holder whose knowledge `holder` has beliefs about.
    pub about: String,
    #[serde(default)]
    pub at: At,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BreachesPayload {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    /// Only the secrets kept by this holder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default)]
    pub at: At,
}
fn default_limit() -> u32 {
    100
}

fn claim_ref(claim: Option<Claim>, claim_key: Option<String>) -> Result<ClaimRef, PublicError> {
    match (claim, claim_key) {
        (Some(claim), None) => Ok(ClaimRef::Claim { claim }),
        (None, Some(claim_key)) => Ok(ClaimRef::ClaimKey { claim_key }),
        _ => Err(PublicError {
            code: ProtocolErrorCode::ValidationFailed,
            message: "Exactly one of claim or claimKey is required.".to_owned(),
            details: None,
        }),
    }
}

fn timeline(timeline_id: &Option<String>) -> &str {
    timeline_id.as_deref().unwrap_or(MAIN_TIMELINE_ID)
}

fn parse<T: DeserializeOwned>(payload: Value) -> Result<T, PublicError> {
    serde_json::from_value(payload).map_err(|error| PublicError {
        code: ProtocolErrorCode::ValidationFailed,
        message: format!("Invalid payload: {error}"),
        details: error
            .to_string()
            .strip_prefix("unknown field `")
            .and_then(|text| text.split('`').next())
            .map(|key| json!({"unknownField": key})),
    })
}
fn value_json(value: impl Serialize) -> Result<Value, PublicError> {
    serde_json::to_value(value).map_err(|error| PublicError {
        code: ProtocolErrorCode::Internal,
        message: format!("Could not serialize response: {error}"),
        details: None,
    })
}
fn map_engine_error(error: umwelt_core::UmweltError) -> PublicError {
    let code = match error.code {
        umwelt_core::UmweltErrorCode::ValidationFailed
        | umwelt_core::UmweltErrorCode::ProjectIsolationViolation => {
            ProtocolErrorCode::ValidationFailed
        }
        umwelt_core::UmweltErrorCode::InvalidState => ProtocolErrorCode::InvalidState,
        umwelt_core::UmweltErrorCode::NotFound => ProtocolErrorCode::NotFound,
        umwelt_core::UmweltErrorCode::VersionConflict => ProtocolErrorCode::VersionConflict,
        umwelt_core::UmweltErrorCode::Conflict => ProtocolErrorCode::Conflict,
        umwelt_core::UmweltErrorCode::FileLocked => ProtocolErrorCode::FileLocked,
        umwelt_core::UmweltErrorCode::NotUmweltFile => ProtocolErrorCode::NotUmweltFile,
        umwelt_core::UmweltErrorCode::UnsupportedFileVersion => {
            ProtocolErrorCode::UnsupportedFileVersion
        }
        _ => ProtocolErrorCode::Internal,
    };
    PublicError {
        code,
        message: error.message,
        details: error.details,
    }
}
fn success(id: Option<String>, result: Value) -> ResponseEnvelope {
    ResponseEnvelope::Success {
        v: 1,
        id,
        ok: true,
        result,
    }
}
pub fn failure(
    id: Option<String>,
    code: ProtocolErrorCode,
    message: impl Into<String>,
    details: Option<Value>,
) -> ResponseEnvelope {
    ResponseEnvelope::Failure {
        v: 1,
        id,
        ok: false,
        error: PublicError {
            code,
            message: message.into(),
            details,
        },
    }
}
fn extract_id(line: &str) -> Option<String> {
    let marker = "\"id\"";
    let start = line.find(marker)? + marker.len();
    let tail = line.get(start..)?;
    let colon = tail.find(':')?;
    let value = tail.get(colon + 1..)?.trim_start();
    let mut escaped = false;
    let mut end = None;
    if !value.starts_with('"') {
        return None;
    }
    for (index, character) in value.char_indices().skip(1) {
        if character == '"' && !escaped {
            end = Some(index + 1);
            break;
        }
        escaped = character == '\\' && !escaped;
        if character != '\\' {
            escaped = false;
        }
    }
    serde_json::from_str(value.get(..end?)?).ok()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{CAPABILITIES, OP_SCHEMAS, extract_id};
    #[test]
    fn extracts_id_from_incomplete_json() {
        assert_eq!(
            extract_id(r#"{"v":1,"id":"request-7","op":"#),
            Some("request-7".to_owned())
        );
    }

    #[test]
    fn capability_and_schema_operations_are_identical() {
        let capabilities = CAPABILITIES
            .iter()
            .map(|value| value.name)
            .collect::<BTreeSet<_>>();
        let schemas = OP_SCHEMAS
            .iter()
            .map(|value| value.name)
            .collect::<BTreeSet<_>>();
        assert_eq!(capabilities, schemas);
        assert_eq!(CAPABILITIES.len(), OP_SCHEMAS.len());
    }

    #[test]
    fn generated_schema_covers_every_capability_and_reference() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../spec/schema/umwelt-protocol-v1.schema.json"
        ))
        .expect("protocol schema should be valid JSON");
        let operations = schema["x-ops"]
            .as_object()
            .expect("x-ops should be an object");
        let definitions = schema["$defs"]
            .as_object()
            .expect("$defs should be an object");
        let expected = CAPABILITIES
            .iter()
            .map(|value| value.name)
            .collect::<BTreeSet<_>>();
        let actual = operations
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        assert_eq!(expected, actual);
        for operation in operations.values() {
            for field in ["payload", "result"] {
                let reference = operation[field]
                    .as_str()
                    .expect("operation reference should be text");
                let name = reference
                    .strip_prefix("#/$defs/")
                    .expect("operation should reference $defs");
                assert!(definitions.contains_key(name), "missing definition {name}");
            }
        }
    }
}
