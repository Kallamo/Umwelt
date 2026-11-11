// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{Result, UmweltError};

/// The contract follows the product version: every package of one release
/// shares it (see `spec/versioning.md`).
pub const UMWELT_CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MAIN_TIMELINE_ID: &str = "main";
/// How many drafts may be nested below main: a draft of main has depth 1.
pub const MAX_DRAFT_DEPTH: u32 = 4;
pub const STATEMENT_PREDICATE_ID: &str = "umwelt:statement";
pub const MEMBER_OF_PREDICATE_ID: &str = "umwelt:member-of";
/// The name of the subject from the Fact's position on (a string value).
pub const NAME_PREDICATE_ID: &str = "umwelt:name";
/// The subject's title from the Fact's position on (a string value).
pub const TITLE_PREDICATE_ID: &str = "umwelt:title";
/// Another name the subject is known by (a string value; many).
pub const ALIAS_PREDICATE_ID: &str = "umwelt:alias";
/// Second-order knowledge: the subject (a holder) holds the claim whose key is
/// the value's `claimKey`. "Alice believes Bob knows X" is a Holding of Alice
/// with subject Bob, this predicate and X's claim key.
pub const HOLDS_PREDICATE_ID: &str = "umwelt:holds";
/// The main time axis, `Position.tick`. Reserved: no additional axis may use it.
pub const MAIN_AXIS_ID: &str = "tick";

pub type Version = u64;

/// A read position. `tick` is on the main axis unless `axis` names an
/// additional axis (`axis.define`); then it is a position on that axis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct At {
    /// An additional axis to read on. Absent: the main axis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_of_version: Option<Version>,
}

impl At {
    /// A position pinned to `tick` and `as_of_version` on `axis`.
    pub fn pinned(axis: Option<String>, tick: i64, as_of_version: Version) -> Self {
        Self {
            axis,
            tick: Some(tick),
            as_of_version: Some(as_of_version),
        }
    }
}

/// Positions of an event on additional axes, by axis id. Each is `>= 0`.
pub type AxisPositions = BTreeMap<String, i64>;

/// An additional order of time, such as reading order next to story order.
/// The main axis (`Position.tick`) is implicit and has no definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AxisDef {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefineAxisInput {
    pub project_id: String,
    pub axis: AxisDef,
}

/// Checks an additional axis id: not empty, not `tick`, not `umwelt:*`.
pub fn validate_axis_id(id: &str) -> Result<()> {
    if id.trim().is_empty() {
        return Err(UmweltError::validation("Axis ids cannot be empty."));
    }
    if id == MAIN_AXIS_ID || id.starts_with("umwelt:") {
        return Err(UmweltError::validation(format!(
            "Axis id '{id}' is reserved."
        )));
    }
    Ok(())
}

/// Checks the positions of an event: every tick is `>= 0`. Whether each axis
/// is defined is checked against the project.
pub fn validate_axis_positions(positions: &AxisPositions) -> Result<()> {
    for (axis, tick) in positions {
        if *tick < 0 {
            return Err(UmweltError::validation(format!(
                "Position on axis '{axis}' cannot be negative."
            )));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    #[serde(default = "default_page_limit")]
    pub limit: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            limit: default_page_limit(),
            cursor: None,
        }
    }
}

fn default_page_limit() -> u32 {
    100
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    JsonSchema,
    Default,
)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    #[schemars(range(min = 0))]
    pub tick: i64,
}

impl Position {
    pub fn validate(&self) -> Result<()> {
        if self.tick < 0 {
            return Err(UmweltError::validation("Position tick cannot be negative."));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum VocabularyMode {
    Strict,
    Open,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub timescale: String,
    pub vocabulary_mode: VocabularyMode,
    pub created_at: String,
    pub version: Version,
}

fn default_vocabulary_mode() -> VocabularyMode {
    VocabularyMode::Strict
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateProjectInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub timescale: String,
    #[serde(default = "default_vocabulary_mode")]
    pub vocabulary_mode: VocabularyMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum EntityKind {
    Agent,
    Group,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FieldType {
    Text,
    Number,
    Boolean,
    Date,
    Enum,
    EntityReference,
    EntityReferenceList,
    Json,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FieldOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct FieldConstraints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<FieldOption>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_entity_type_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FieldDef {
    pub id: String,
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraints: Option<FieldConstraints>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntityType {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub kind: EntityKind,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateEntityTypeInput {
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub kind: EntityKind,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
}

pub fn validate_entity_fields(fields: &Map<String, Value>, entity_type: &EntityType) -> Result<()> {
    for key in fields.keys() {
        if !entity_type.fields.iter().any(|field| field.key == *key) {
            return Err(UmweltError::validation(format!(
                "Unknown field '{key}' for Entity Type {}.",
                entity_type.id
            )));
        }
    }
    for definition in &entity_type.fields {
        if definition
            .constraints
            .as_ref()
            .and_then(|constraints| constraints.required)
            .unwrap_or(false)
            && fields
                .get(&definition.key)
                .is_none_or(|value| value.is_null() || value.as_str() == Some(""))
        {
            return Err(UmweltError::validation(format!(
                "Field '{}' is required.",
                definition.key
            )));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExternalRef {
    pub system: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    pub id: String,
    pub project_id: String,
    pub type_id: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub fields: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_ref: Option<ExternalRef>,
    pub created_at: Position,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateEntityInput {
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub type_id: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub fields: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_ref: Option<ExternalRef>,
    #[serde(default)]
    pub created_at: Position,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateEntityInput {
    pub project_id: String,
    pub entity_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aliases: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<Map<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_ref: Option<ExternalRef>,
}

/// One entity owned by another system (a game, a wiki, a host app), keyed by
/// its `externalRef`. Ensuring it twice is a no-op; changed fields are updated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnsureEntityInput {
    pub external_ref: ExternalRef,
    /// Id used only when the entity is created; an existing entity keeps its id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub type_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aliases: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<Map<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<Position>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnsureEntitiesInput {
    pub project_id: String,
    pub entities: Vec<EnsureEntityInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnsureEntityResult {
    pub entity: Entity,
    pub created: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedEntity {
    pub external_ref: ExternalRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ObjectKind {
    Entity,
    Value,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Cardinality {
    One,
    Many,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PredicateDef {
    pub id: String,
    pub label: String,
    pub object: ObjectKind,
    pub cardinality: Cardinality,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_types: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefinePredicateInput {
    pub project_id: String,
    pub predicate: PredicateDef,
}

pub fn reserved_predicates() -> [PredicateDef; 6] {
    let value = |id: &str, label: &str, cardinality: Cardinality| PredicateDef {
        id: id.to_owned(),
        label: label.to_owned(),
        object: ObjectKind::Value,
        cardinality,
        subject_types: None,
    };
    [
        PredicateDef {
            id: STATEMENT_PREDICATE_ID.to_owned(),
            label: "Statement".to_owned(),
            object: ObjectKind::Value,
            cardinality: Cardinality::Many,
            subject_types: None,
        },
        PredicateDef {
            id: MEMBER_OF_PREDICATE_ID.to_owned(),
            label: "Member of".to_owned(),
            object: ObjectKind::Entity,
            cardinality: Cardinality::Many,
            subject_types: None,
        },
        value(NAME_PREDICATE_ID, "Name", Cardinality::One),
        value(TITLE_PREDICATE_ID, "Title", Cardinality::One),
        value(ALIAS_PREDICATE_ID, "Alias", Cardinality::Many),
        value(HOLDS_PREDICATE_ID, "Holds", Cardinality::Many),
    ]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ClaimObject {
    Entity { id: String },
    Value { value: Value },
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Claim {
    pub subject: String,
    pub predicate: String,
    pub object: ClaimObject,
    #[serde(default)]
    pub qualifiers: BTreeMap<String, Value>,
    pub polarity: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged, rename_all_fields = "camelCase")]
pub enum ClaimRef {
    Claim { claim: Claim },
    ClaimKey { claim_key: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Attitude {
    Knows,
    Believes,
    Suspects,
    Remembers,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Accessibility {
    Available,
    Suppressed,
    Inaccessible,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Intent {
    Honest,
    Deceptive,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum Acquisition {
    Witnessed {
        event_id: String,
    },
    ToldBy {
        source_holder_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source_record_id: Option<String>,
        intent: Intent,
    },
    Evidence {
        evidence_id: String,
    },
    Inferred {
        from_record_ids: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rule: Option<String>,
    },
    Authored,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum Origin {
    Human {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        actor_id: Option<String>,
    },
    Model {
        model: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prompt_hash: Option<String>,
    },
    System {
        name: String,
    },
    Import,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    pub origin: Origin,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecordCommon {
    pub id: String,
    pub project_id: String,
    pub timeline_id: String,
    pub position: Position,
    pub recorded_at: Version,
    pub event_id: String,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum Record {
    Fact {
        #[serde(flatten)]
        common: RecordCommon,
        claim: Claim,
    },
    Holding {
        #[serde(flatten)]
        common: RecordCommon,
        holder: String,
        attitude: Attitude,
        claim: Claim,
        acquisition: Acquisition,
        confidence: f64,
        accessibility: Accessibility,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        /// The holder keeps this secret; only `sharedWith` may be told.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        secrecy: Option<Secrecy>,
    },
    Awareness {
        #[serde(flatten)]
        common: RecordCommon,
        holder: String,
        target: String,
    },
    Disposition {
        #[serde(flatten)]
        common: RecordCommon,
        holder: String,
        target: String,
        dimension: String,
        value: Value,
    },
}

/// Marks a Holding as a secret. The engine never blocks telling it: telling
/// anyone outside `sharedWith` (besides the holder) is recorded and reported
/// as a breach.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Secrecy {
    #[serde(default)]
    pub shared_with: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TransitionStatus {
    Retracted,
    Revoked,
    Forgotten,
    Abandoned,
    Superseded,
}

/// The statuses `transition.batch` applies. `superseded` is not one of them:
/// it needs a replacement record, which `holding.adjustBatch` writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum BatchTransitionStatus {
    Retracted,
    Revoked,
    Forgotten,
    Abandoned,
}

impl From<BatchTransitionStatus> for TransitionStatus {
    fn from(status: BatchTransitionStatus) -> Self {
        match status {
            BatchTransitionStatus::Retracted => Self::Retracted,
            BatchTransitionStatus::Revoked => Self::Revoked,
            BatchTransitionStatus::Forgotten => Self::Forgotten,
            BatchTransitionStatus::Abandoned => Self::Abandoned,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    pub id: String,
    pub project_id: String,
    pub timeline_id: String,
    pub target_record_id: String,
    pub status: TransitionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    pub position: Position,
    pub recorded_at: Version,
    pub ordinal: u32,
    pub event_id: String,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceRef {
    pub evidence_source_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceSource {
    pub id: String,
    pub project_id: String,
    #[serde(rename = "type")]
    pub source_type: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Map<String, Value>>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub project_id: String,
    pub timeline_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub position: Position,
    /// Positions on additional axes. Records and transitions of the event
    /// inherit them.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub on: AxisPositions,
    #[serde(default)]
    pub participants: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
    pub provenance: Provenance,
    pub recorded_at: Version,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub position: Position,
    /// Positions on additional axes (`axis.define`), each `>= 0`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub on: AxisPositions,
    #[serde(default)]
    pub participants: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default)]
    pub evidence: Vec<EvidenceRef>,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecordInputCommon {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Omitted: the position of the commit's event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    /// Omitted: the provenance of the commit's event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

impl RecordInputCommon {
    /// The record's position: its own, or the event's.
    pub fn position_in(&self, event: &EventInput) -> Position {
        self.position.unwrap_or(event.position)
    }

    /// The record's provenance: its own, or the event's.
    pub fn provenance_in<'a>(&'a self, event: &'a EventInput) -> &'a Provenance {
        self.provenance.as_ref().unwrap_or(&event.provenance)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum RecordInput {
    Fact {
        #[serde(flatten)]
        common: RecordInputCommon,
        claim: Claim,
    },
    Holding {
        #[serde(flatten)]
        common: RecordInputCommon,
        holder: String,
        attitude: Attitude,
        claim: Claim,
        acquisition: Acquisition,
        /// Omitted: 1.
        #[serde(default = "default_confidence")]
        confidence: f64,
        /// Omitted: `available`.
        #[serde(default = "default_accessibility")]
        accessibility: Accessibility,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        /// The holder keeps this secret; only `sharedWith` may be told.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        secrecy: Option<Secrecy>,
    },
    Awareness {
        #[serde(flatten)]
        common: RecordInputCommon,
        holder: String,
        target: String,
    },
    Disposition {
        #[serde(flatten)]
        common: RecordInputCommon,
        holder: String,
        target: String,
        dimension: String,
        value: Value,
    },
}

impl RecordInput {
    pub fn common(&self) -> &RecordInputCommon {
        match self {
            Self::Fact { common, .. }
            | Self::Holding { common, .. }
            | Self::Awareness { common, .. }
            | Self::Disposition { common, .. } => common,
        }
    }

    pub fn common_mut(&mut self) -> &mut RecordInputCommon {
        match self {
            Self::Fact { common, .. }
            | Self::Holding { common, .. }
            | Self::Awareness { common, .. }
            | Self::Disposition { common, .. } => common,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TransitionInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub target_record_id: String,
    pub status: TransitionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    /// Omitted: the position of the commit's event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    /// Omitted: the provenance of the commit's event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

impl TransitionInput {
    /// The transition's position: its own, or the event's.
    pub fn position_in(&self, event: &EventInput) -> Position {
        self.position.unwrap_or(event.position)
    }

    /// The transition's provenance: its own, or the event's.
    pub fn provenance_in<'a>(&'a self, event: &'a EventInput) -> &'a Provenance {
        self.provenance.as_ref().unwrap_or(&event.provenance)
    }
}

/// Fills every write default that comes from the event, so that a request
/// that omits a field and one that spells it out are the same request.
fn fill_write_defaults(
    event: &EventInput,
    records: &mut [RecordInput],
    transitions: &mut [TransitionInput],
) {
    for record in records {
        let common = record.common_mut();
        common.position.get_or_insert(event.position);
        if common.provenance.is_none() {
            common.provenance = Some(event.provenance.clone());
        }
    }
    for transition in transitions {
        transition.position.get_or_insert(event.position);
        if transition.provenance.is_none() {
            transition.provenance = Some(event.provenance.clone());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommitInput {
    pub project_id: String,
    pub attempt_id: String,
    /// Timeline to write to; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_version: Option<Version>,
    pub event: EventInput,
    #[serde(default)]
    pub records: Vec<RecordInput>,
    #[serde(default)]
    pub transitions: Vec<TransitionInput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub preconditions: Vec<Precondition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum Precondition {
    ActiveHolding {
        record_id: String,
        holder: String,
        at: Position,
        #[serde(skip)]
        #[schemars(skip)]
        copy_claim_to_operation: Option<usize>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProposeInput {
    pub project_id: String,
    /// Timeline to write to; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_version: Option<Version>,
    pub event: EventInput,
    #[serde(default)]
    pub records: Vec<RecordInput>,
    #[serde(default)]
    pub transitions: Vec<TransitionInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Warning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommitOutcome {
    pub event_id: String,
    pub record_ids: Vec<String>,
    pub transition_ids: Vec<String>,
    pub version: Version,
    #[serde(default)]
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ProposalStatus {
    Pending,
    Applied,
    PartiallyApplied,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewDecision {
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReviewInput {
    pub operation_index: usize,
    pub decision: ReviewDecision,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProposalReview {
    pub id: String,
    pub operation_index: usize,
    pub decision: ReviewDecision,
    pub rationale: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub id: String,
    pub status: ProposalStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_rationale: Option<String>,
    pub input: ProposeInput,
    #[serde(default)]
    pub reviews: Vec<ProposalReview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HoldsResult {
    /// The holder has a Holding about the claim, with either polarity.
    pub holds: bool,
    /// Whether that Holding affirms what was asked (`same`) or its negation
    /// (`opposite`). Absent when `holds` is false or the question gave only
    /// a `claimKey`, which has no polarity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holds_as: Option<HoldsAs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attitude: Option<Attitude>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polarity: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct HoldingFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicate: Option<String>,
    #[serde(default)]
    pub attitude: Vec<Attitude>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_confidence: Option<f64>,
    #[serde(default)]
    pub include_withheld: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HoldingItem {
    pub record: Record,
    pub acquisition: Acquisition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<TransitionStatus>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HoldingsResult {
    pub items: Vec<HoldingItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub withheld_count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct WhoHoldsFilter {
    #[serde(default)]
    pub attitude: Vec<Attitude>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polarity: Option<bool>,
    /// Only holders that are members of this group at the read position
    /// (an active `umwelt:member-of` Fact).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_of: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WhoHoldsItem {
    pub holder: String,
    pub record_id: String,
    pub attitude: Attitude,
    pub polarity: bool,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct CanonFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageResult<T> {
    pub items: Vec<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PerspectiveResult {
    pub holder: Entity,
    pub awareness: Vec<Record>,
    pub holdings: Vec<HoldingItem>,
    pub dispositions: Vec<Record>,
    pub memberships: Vec<Record>,
    pub withheld_count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompareEntry {
    pub claim_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a: Option<Record>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub b: Option<Record>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompareCounts {
    pub shared: u64,
    pub conflicting: u64,
    pub only_a: u64,
    pub only_b: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompareResult {
    pub shared: Vec<CompareEntry>,
    pub conflicting: Vec<CompareEntry>,
    pub only_a: Vec<CompareEntry>,
    pub only_b: Vec<CompareEntry>,
    pub counts: CompareCounts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ContradictionKind {
    VsCanon,
    VsSelf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ContradictionReason {
    OppositePolarity,
    DifferentValueInFunctionalSlot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Contradiction {
    pub holding_id: String,
    pub kind: ContradictionKind,
    pub other_record_id: String,
    pub reason: ContradictionReason,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChainLink {
    pub record: Record,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquisition: Option<Acquisition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TraceResult {
    pub record: Record,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<TransitionStatus>,
    pub event: Event,
    pub transitions: Vec<Transition>,
    pub chain: Vec<ChainLink>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangeItem {
    pub seq: u64,
    pub timeline_id: String,
    pub version: Version,
    #[serde(rename = "type")]
    pub change_type: String,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchItem {
    pub doc_type: String,
    pub doc_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record: Option<Record>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<Event>,
    /// Which search found the item.
    #[serde(rename = "match", default, skip_serializing_if = "Option::is_none")]
    pub matched: Option<SearchMatch>,
    /// The value the item was ranked by: the cosine similarity in a semantic
    /// search, the fused rank score in a hybrid one. Absent in a text search.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SearchMatch {
    Text,
    Semantic,
    Both,
}

/// The semantic part of a search: a query vector made by the client's model
/// in one of the project's embedding spaces.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchSemantic {
    pub space_id: String,
    pub vector: Vec<f32>,
}

/// The vectors of one embedding model. A vector is only compared with
/// vectors of the same space. `model` documents the model; the engine never
/// reads it and never runs a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmbeddingSpace {
    pub id: String,
    #[schemars(range(min = 1, max = 4096))]
    pub dimensions: u32,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefineEmbeddingSpaceInput {
    pub project_id: String,
    pub space: EmbeddingSpace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DropEmbeddingSpaceResult {
    /// Vectors removed with the space.
    pub removed: u64,
}

/// A searchable document: a record (a Fact, or a Holding with text) or an
/// event whose type does not begin with `umwelt:`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmbeddingDocRef {
    /// `record` or `event`.
    pub doc_type: String,
    pub doc_id: String,
}

/// Documents visible in a timeline whose vector in a space is missing or
/// stale, with the text to embed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingEmbeddingsInput {
    pub project_id: String,
    pub space_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    /// `record` and/or `event`; empty for both.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub doc_types: Vec<String>,
    #[serde(default)]
    pub page: Page,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PendingEmbedding {
    pub doc_type: String,
    pub doc_id: String,
    /// The text the search indexes for this document: embed exactly this.
    pub text: String,
    /// SHA-256 of `text`, to send back with the vector.
    pub text_hash: String,
    /// A vector exists but was made from another text.
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PendingEmbeddingsResult {
    pub items: Vec<PendingEmbedding>,
    /// More pending documents follow; pass it as `page.cursor`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Where a later call can start to see only documents written after this
    /// one. Starting without a cursor checks every document again.
    pub resume_cursor: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmbeddingItem {
    /// `record` or `event`.
    pub doc_type: String,
    pub doc_id: String,
    /// The `textHash` the vector was made from (see `embedding.pending`).
    pub text_hash: String,
    pub vector: Vec<f32>,
}

/// Stores or replaces vectors in one transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PutEmbeddingsInput {
    pub project_id: String,
    pub space_id: String,
    pub items: Vec<EmbeddingItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PutEmbeddingsResult {
    pub stored: u64,
    /// Stored vectors whose `textHash` is not the document's: they stay
    /// pending until replaced.
    pub stale: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteEmbeddingsInput {
    pub project_id: String,
    pub space_id: String,
    pub items: Vec<EmbeddingDocRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteEmbeddingsResult {
    pub removed: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransmitInput {
    pub project_id: String,
    pub attempt_id: String,
    /// Timeline to write to; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    pub position: Position,
    /// Positions of the generated event on additional axes.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub on: AxisPositions,
    pub from: String,
    pub to: String,
    pub source_record_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,
    #[serde(default = "default_transmit_attitude")]
    pub attitude: Attitude,
    pub confidence: f64,
    pub intent: Intent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distorted_claim: Option<Claim>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

fn default_transmit_attitude() -> Attitude {
    Attitude::Believes
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RecordState {
    pub record: Record,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition: Option<Transition>,
}

impl CommitInput {
    /// The timeline this commit writes to.
    pub fn timeline(&self) -> &str {
        self.timeline_id.as_deref().unwrap_or(MAIN_TIMELINE_ID)
    }

    /// Fills the position and provenance that records and transitions left
    /// out with the event's. Runs before the fingerprint and validation.
    pub fn normalize(&mut self) {
        fill_write_defaults(&self.event, &mut self.records, &mut self.transitions);
    }
}

impl ProposeInput {
    /// Same as [`CommitInput::normalize`], so a proposal shows what it writes.
    pub fn normalize(&mut self) {
        fill_write_defaults(&self.event, &mut self.records, &mut self.transitions);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TimelineKind {
    Main,
    Draft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TimelineStatus {
    Active,
    Promoted,
    Discarded,
}

/// A line of history. "main" is the confirmed history of a project; a draft
/// branches from its parent (main or another draft) at `baseVersion` and is
/// later promoted into that parent or discarded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Timeline {
    pub project_id: String,
    pub id: String,
    pub kind: TimelineKind,
    pub status: TimelineStatus,
    /// Timeline this draft branched from: "main" or another draft. Absent for
    /// main.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_timeline_id: Option<String>,
    /// Drafts from main down to this timeline: 0 for main, 1 for a draft of
    /// main, at most `MAX_DRAFT_DEPTH`.
    pub depth: u32,
    /// Head version of the parent when the draft was opened. Absent for main.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_version: Option<Version>,
    pub head_version: Version,
    pub max_tick: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purged_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenDraftInput {
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Timeline to branch from: "main" (the default) or an active draft.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_timeline_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PurgeCounts {
    pub events: u64,
    pub records: u64,
    pub transitions: u64,
    pub attempts: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PurgeResult {
    pub purged: bool,
    pub removed: PurgeCounts,
}

fn default_witness_attitude() -> Attitude {
    Attitude::Knows
}

fn default_confidence() -> f64 {
    1.0
}

fn default_accessibility() -> Accessibility {
    Accessibility::Available
}

/// One claim seen at a witnessed event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WitnessedClaim {
    pub claim: Claim,
    /// Also record the claim as canon (a Fact) in the same commit.
    #[serde(default)]
    pub canon: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fact_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Witness {
    pub holder: String,
    #[serde(default = "default_witness_attitude")]
    pub attitude: Attitude,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default = "default_accessibility")]
    pub accessibility: Accessibility,
    /// One Holding id per claim, in claim order. Empty: generated.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub record_ids: Vec<String>,
}

/// One event seen by several holders: every witness gets a `witnessed`
/// Holding for every claim, in one atomic commit. Who saw what, and how
/// clearly, is the client's decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WitnessInput {
    pub project_id: String,
    pub attempt_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_version: Option<Version>,
    pub event: EventInput,
    pub claims: Vec<WitnessedClaim>,
    pub witnesses: Vec<Witness>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GroupCoverageResult {
    pub group: String,
    pub members: Vec<String>,
    /// The group's own Holding, if the group itself holds the claim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_holding: Option<WhoHoldsItem>,
    /// Members holding the claim.
    pub holding: Vec<WhoHoldsItem>,
    /// Members holding nothing about the claim.
    pub missing: Vec<String>,
}

/// A new value for an active Holding. The old Holding is superseded by a copy
/// carrying the changed fields; its history stays readable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HoldingAdjustment {
    pub record_id: String,
    /// Id of the replacing Holding. Generated when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_record_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accessibility: Option<Accessibility>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attitude: Option<Attitude>,
}

/// Adjusts many Holdings in one commit, e.g. a night of memory decay. How much
/// anything decays is the client's policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdjustHoldingsInput {
    pub project_id: String,
    pub attempt_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_version: Option<Version>,
    pub position: Position,
    /// Positions of the generated event on additional axes.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub on: AxisPositions,
    pub adjustments: Vec<HoldingAdjustment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

/// One status change applied to many records in one commit, e.g. everything a
/// character forgets overnight. `superseded` needs a replacement and is not
/// accepted here; use `holding.adjustBatch`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransitionBatchInput {
    pub project_id: String,
    pub attempt_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_version: Option<Version>,
    pub position: Position,
    /// Positions of the generated event on additional axes.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub on: AxisPositions,
    pub status: BatchTransitionStatus,
    pub record_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DispositionSetting {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub holder: String,
    pub target: String,
    pub dimension: String,
    pub value: Value,
}

/// Records many Disposition values in one commit (trust, affection...). The
/// newest value per holder, target and dimension is the current one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetDispositionsInput {
    pub project_id: String,
    pub attempt_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_version: Option<Version>,
    pub position: Position,
    /// Positions of the generated event on additional axes.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub on: AxisPositions,
    pub dispositions: Vec<DispositionSetting>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
}

/// An entity as canon describes it at one position: its current name, title
/// and aliases from `umwelt:name`, `umwelt:title` and `umwelt:alias` Facts,
/// falling back to the entity's own name and aliases.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntityAtResult {
    pub entity: Entity,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub aliases: Vec<String>,
    /// The Facts the name, title and aliases come from.
    pub record_ids: Vec<String>,
}

/// An entity as one holder knows it: only what the holder's own active,
/// available Holdings say. A holder that knows no name gets none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntityKnownResult {
    pub entity_id: String,
    pub holder: String,
    /// The holder has an Awareness record of the entity.
    pub aware: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub aliases: Vec<String>,
    /// The Holdings the name, title and aliases come from.
    pub record_ids: Vec<String>,
}

/// What a holder thinks another holder knows (`umwelt:holds` Holdings), next
/// to what the other holder actually holds at the same position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BeliefAboutItem {
    /// The holder's `umwelt:holds` Holding.
    pub record_id: String,
    pub claim_key: String,
    /// True: the holder thinks the other holds the claim. False: thinks not.
    pub polarity: bool,
    pub attitude: Attitude,
    pub confidence: f64,
    /// Whether the other holder really holds the claim (active, available).
    pub actually_holds: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_record_id: Option<String>,
}

/// A secret told to someone outside its circle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Breach {
    /// The secret Holding.
    pub secret_record_id: String,
    /// Who keeps the secret.
    pub owner: String,
    pub told_by: String,
    pub recipient: String,
    /// The recipient's Holding acquired by being told.
    pub record_id: String,
    pub position: Position,
    pub event_id: String,
}

/// Claims a holder says or thinks, to check against what the holder can know.
/// The claims come from the client (or a model it runs): the engine never
/// reads free text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckUtteranceInput {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    pub holder: String,
    #[serde(default)]
    pub at: At,
    pub claims: Vec<Claim>,
}

/// How a claim relates to what the holder holds at the read position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum UtteranceVerdict {
    /// The holder holds the claim (see `attitude`).
    Holds,
    /// The holder holds the claim with the opposite polarity.
    HoldsOpposite,
    /// The holder holds another value in the same functional slot.
    HoldsOtherValue,
    /// The holder holds it, but suppressed or inaccessible.
    Withheld,
    /// The holder only comes to hold it at a later tick (on the read's axis):
    /// an anachronism.
    Future,
    /// Nothing the holder holds covers the claim.
    Unknown,
}

/// How a claim relates to canon at the read position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CanonStatus {
    Agrees,
    Contradicts,
    NoCanon,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UtteranceCheck {
    pub claim_key: String,
    pub verdict: UtteranceVerdict,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attitude: Option<Attitude>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// The Holding that decides the verdict.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,
    pub canon: CanonStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canon_record_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckUtteranceResult {
    pub items: Vec<UtteranceCheck>,
    /// Every claim has the verdict `holds` with the same polarity.
    pub consistent: bool,
}

fn default_cost() -> u32 {
    1
}

/// Something a client wants to put in a context (a prompt, a scene brief).
/// Exactly one of `recordId` and `eventId` is given.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextCandidate {
    /// The client's own id for the candidate.
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    /// Budget units the candidate costs (tokens, lines...). Defaults to 1.
    #[serde(default = "default_cost")]
    pub cost: u32,
    /// Higher priorities are admitted first when the budget is short.
    #[serde(default)]
    pub priority: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification: Option<String>,
}

/// Decides which candidates may enter a context seen from one perspective.
/// Without `holder` the perspective is the narrator's, who sees everything
/// visible at the position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextAdmitInput {
    pub project_id: String,
    /// Timeline to read; defaults to "main".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holder: Option<String>,
    #[serde(default)]
    pub at: At,
    pub candidates: Vec<ContextCandidate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<u64>,
    #[serde(default)]
    pub exclude_classifications: Vec<String>,
    /// Admit suppressed and inaccessible Holdings. Requires
    /// `authorizedNarrator`.
    #[serde(default)]
    pub include_withheld: bool,
    #[serde(default)]
    pub authorized_narrator: bool,
}

/// Why a candidate was refused. Checks run in this order and a candidate
/// reports the first one it fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ContextRejection {
    /// Neither or both of `recordId` and `eventId`.
    InvalidCandidate,
    /// No such record or event in this project and timeline.
    NotInProject,
    /// Recorded after the read position.
    Future,
    /// Read on an additional axis, and the candidate's event has no position
    /// on it: it is visible to nobody there.
    NotOnAxis,
    /// Retracted, revoked, forgotten, abandoned or superseded.
    Inactive,
    /// Not part of the holder's perspective.
    NotHeld,
    /// Suppressed or inaccessible for the holder.
    Withheld,
    /// The candidate's classification is excluded.
    ExcludedClassification,
    /// The same record or event was already admitted.
    Duplicate,
    /// Higher-priority candidates used the budget first.
    OverBudget,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContextAdmitted {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    pub cost: u32,
    pub priority: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContextRejected {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    pub reason: ContextRejection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContextAdmitResult {
    /// Admitted candidates, highest priority first (ties keep input order).
    pub admitted: Vec<ContextAdmitted>,
    /// Refused candidates, in input order.
    pub rejected: Vec<ContextRejected>,
    pub used_budget: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OperationCheck {
    /// Index in the proposal: records first, then transitions.
    pub operation_index: usize,
    pub valid: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<OperationError>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OperationError {
    pub code: String,
    pub message: String,
}

/// A pending proposal checked again against the current head of its timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProposalRebaseResult {
    pub proposal: Proposal,
    pub head_version: Version,
    pub operations: Vec<OperationCheck>,
}

/// Whether the Holding behind a `holds` answer has the polarity that was
/// asked about. A holder who believes the opposite still holds the claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum HoldsAs {
    /// The Holding has the polarity of the claim asked about.
    Same,
    /// The Holding has the opposite polarity: the holder holds the negation.
    Opposite,
}

/// The keys the engine derives from a claim (`spec/claim-key.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClaimKeyResult {
    /// Identifies the claim whatever its polarity.
    pub claim_key: String,
    /// Identifies the subject and predicate.
    pub slot_key: String,
}

impl ClaimKeyResult {
    pub fn of(claim: &Claim) -> Self {
        Self {
            claim_key: crate::claim_key(claim),
            slot_key: crate::slot_key(claim),
        }
    }
}

impl Acquisition {
    /// The acquisition as the holder knows it. The intent of whoever told
    /// them is the narrator's knowledge, so it reads as `unknown`.
    pub fn hide_intent(&mut self) {
        if let Self::ToldBy { intent, .. } = self {
            *intent = Intent::Unknown;
        }
    }
}

impl Record {
    /// The record as its holder may read it (see [`Acquisition::hide_intent`]).
    pub fn hide_intent(&mut self) {
        if let Self::Holding { acquisition, .. } = self {
            acquisition.hide_intent();
        }
    }
}

/// What every transmit of a batch has in common, written once. A field given
/// by an item wins over the shared one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransmitShared {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<AxisPositions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_record_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attitude: Option<Attitude>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<Intent>,
}

/// One transmit of a batch: a `TransmitInput` whose shareable fields may be
/// left to the batch's `shared`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransmitBatchItem {
    pub attempt_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<AxisPositions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_record_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,
    /// Neither here nor in `shared`: `believes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attitude: Option<Attitude>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<Intent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distorted_claim: Option<Claim>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl From<TransmitInput> for TransmitBatchItem {
    fn from(input: TransmitInput) -> Self {
        Self {
            attempt_id: input.attempt_id,
            project_id: Some(input.project_id),
            timeline_id: input.timeline_id,
            position: Some(input.position),
            on: (!input.on.is_empty()).then_some(input.on),
            from: Some(input.from),
            to: Some(input.to),
            source_record_id: Some(input.source_record_id),
            record_id: input.record_id,
            attitude: Some(input.attitude),
            confidence: Some(input.confidence),
            intent: Some(input.intent),
            distorted_claim: input.distorted_claim,
            text: input.text,
        }
    }
}

impl TransmitBatchItem {
    /// The complete transmit: this item, with what it leaves out taken from
    /// `shared`. The engine supplies nothing itself except the attitude's
    /// documented default; a field missing in both places is refused.
    pub fn complete(self, shared: &TransmitShared) -> Result<TransmitInput> {
        let attempt_id = self.attempt_id;
        let missing = |field: &str| {
            UmweltError::validation(format!(
                "Transmit '{attempt_id}' has no `{field}`, in the item or in `shared`."
            ))
        };
        Ok(TransmitInput {
            project_id: self
                .project_id
                .or_else(|| shared.project_id.clone())
                .ok_or_else(|| missing("projectId"))?,
            timeline_id: self.timeline_id.or_else(|| shared.timeline_id.clone()),
            position: self
                .position
                .or(shared.position)
                .ok_or_else(|| missing("position"))?,
            on: self.on.or_else(|| shared.on.clone()).unwrap_or_default(),
            from: self
                .from
                .or_else(|| shared.from.clone())
                .ok_or_else(|| missing("from"))?,
            to: self
                .to
                .or_else(|| shared.to.clone())
                .ok_or_else(|| missing("to"))?,
            source_record_id: self
                .source_record_id
                .or_else(|| shared.source_record_id.clone())
                .ok_or_else(|| missing("sourceRecordId"))?,
            record_id: self.record_id,
            attitude: self
                .attitude
                .or_else(|| shared.attitude.clone())
                .unwrap_or_else(default_transmit_attitude),
            confidence: self
                .confidence
                .or(shared.confidence)
                .ok_or_else(|| missing("confidence"))?,
            intent: self
                .intent
                .or_else(|| shared.intent.clone())
                .ok_or_else(|| missing("intent"))?,
            distorted_claim: self.distorted_claim,
            text: self.text,
            attempt_id: attempt_id.clone(),
        })
    }
}

/// The project of a world manifest. Unlike `project.create`, the id is
/// required: it is what makes the manifest repeatable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorldProject {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub timescale: String,
    #[serde(default = "default_vocabulary_mode")]
    pub vocabulary_mode: VocabularyMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorldEntityType {
    pub id: String,
    pub name: String,
    pub kind: EntityKind,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorldEntity {
    pub id: String,
    pub type_id: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub fields: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_ref: Option<ExternalRef>,
    #[serde(default)]
    pub created_at: Position,
}

/// A manifest of what must exist: the project and its entity types, entities,
/// predicates and axes. Ensuring it creates what is missing, in one
/// transaction, and leaves what is already there untouched; anything that
/// exists with another definition refuses the whole manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnsureWorldInput {
    pub project: WorldProject,
    #[serde(default)]
    pub entity_types: Vec<WorldEntityType>,
    #[serde(default)]
    pub entities: Vec<WorldEntity>,
    #[serde(default)]
    pub predicates: Vec<PredicateDef>,
    #[serde(default)]
    pub axes: Vec<AxisDef>,
}

/// What a manifest created; everything else was already there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorldCreated {
    pub project: bool,
    pub entity_types: Vec<String>,
    pub entities: Vec<String>,
    pub predicates: Vec<String>,
    pub axes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnsureWorldResult {
    pub project: Project,
    pub created: WorldCreated,
}

/// One thing a manifest asks for that already exists with another definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorldDivergence {
    /// `project`, `entityType`, `entity`, `predicate` or `axis`.
    pub kind: String,
    pub id: String,
    /// The field whose stored value differs.
    pub field: String,
}

/// What a manifest has to create, decided against the stored world.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorldWrite {
    pub project: Option<Project>,
    pub entity_types: Vec<EntityType>,
    pub entities: Vec<Entity>,
    /// The project the predicates and axes belong to.
    pub project_id: String,
    pub predicates: Vec<PredicateDef>,
    pub axes: Vec<AxisDef>,
}

/// The world as stored, as far as a manifest needs to compare against it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StoredWorld {
    pub project: Option<Project>,
    pub entity_types: Vec<EntityType>,
    /// Stored entities among those the manifest names.
    pub entities: Vec<Entity>,
    pub predicates: Vec<PredicateDef>,
    pub axes: Vec<AxisDef>,
}

fn unique_ids<'a>(kind: &str, ids: impl Iterator<Item = &'a str>) -> Result<()> {
    let mut seen = std::collections::BTreeSet::new();
    for id in ids {
        if id.trim().is_empty() {
            return Err(UmweltError::validation(format!(
                "A manifest {kind} has an empty id."
            )));
        }
        if !seen.insert(id) {
            return Err(UmweltError::validation(format!(
                "The manifest lists {kind} '{id}' twice."
            )));
        }
    }
    Ok(())
}

/// Compares a manifest with the stored world: what is missing, and what
/// exists with another definition. An existing entity only has to keep its
/// type: its name, aliases and fields change during play and are left alone.
pub fn plan_world(
    input: &EnsureWorldInput,
    stored: &StoredWorld,
    created_at: &str,
) -> Result<(WorldWrite, Vec<WorldDivergence>)> {
    let project_id = &input.project.id;
    unique_ids("project", std::iter::once(project_id.as_str()))?;
    unique_ids(
        "entity type",
        input.entity_types.iter().map(|value| value.id.as_str()),
    )?;
    unique_ids(
        "entity",
        input.entities.iter().map(|value| value.id.as_str()),
    )?;
    unique_ids(
        "predicate",
        input.predicates.iter().map(|value| value.id.as_str()),
    )?;
    unique_ids("axis", input.axes.iter().map(|value| value.id.as_str()))?;

    let mut write = WorldWrite {
        project_id: project_id.clone(),
        ..WorldWrite::default()
    };
    let mut divergences = Vec::new();
    let mut differ = |kind: &str, id: &str, field: &str, differs: bool| {
        if differs {
            divergences.push(WorldDivergence {
                kind: kind.to_owned(),
                id: id.to_owned(),
                field: field.to_owned(),
            });
        }
    };
    match &stored.project {
        Some(project) => {
            let wanted = &input.project;
            differ("project", project_id, "name", project.name != wanted.name);
            differ(
                "project",
                project_id,
                "description",
                project.description != wanted.description,
            );
            differ(
                "project",
                project_id,
                "timescale",
                project.timescale != wanted.timescale,
            );
            differ(
                "project",
                project_id,
                "vocabularyMode",
                project.vocabulary_mode != wanted.vocabulary_mode,
            );
        }
        None => {
            write.project = Some(Project {
                id: project_id.clone(),
                name: input.project.name.clone(),
                description: input.project.description.clone(),
                timescale: input.project.timescale.clone(),
                vocabulary_mode: input.project.vocabulary_mode.clone(),
                created_at: created_at.to_owned(),
                version: 0,
            });
        }
    }
    for wanted in &input.entity_types {
        match stored
            .entity_types
            .iter()
            .find(|value| value.id == wanted.id)
        {
            Some(existing) => {
                differ(
                    "entityType",
                    &wanted.id,
                    "name",
                    existing.name != wanted.name,
                );
                differ(
                    "entityType",
                    &wanted.id,
                    "kind",
                    existing.kind != wanted.kind,
                );
                differ(
                    "entityType",
                    &wanted.id,
                    "fields",
                    existing.fields != wanted.fields,
                );
            }
            None => write.entity_types.push(EntityType {
                id: wanted.id.clone(),
                project_id: project_id.clone(),
                name: wanted.name.clone(),
                kind: wanted.kind.clone(),
                fields: wanted.fields.clone(),
            }),
        }
    }
    for wanted in &input.entities {
        match stored.entities.iter().find(|value| value.id == wanted.id) {
            Some(existing) => differ(
                "entity",
                &wanted.id,
                "typeId",
                existing.type_id != wanted.type_id,
            ),
            None => write.entities.push(Entity {
                id: wanted.id.clone(),
                project_id: project_id.clone(),
                type_id: wanted.type_id.clone(),
                name: wanted.name.clone(),
                aliases: wanted.aliases.clone(),
                fields: wanted.fields.clone(),
                external_ref: wanted.external_ref.clone(),
                created_at: wanted.created_at,
                archived: false,
            }),
        }
    }
    for wanted in &input.predicates {
        if wanted.id.starts_with("umwelt:") {
            return Err(UmweltError::validation(
                "Predicate ids beginning with 'umwelt:' are reserved.",
            ));
        }
        match stored.predicates.iter().find(|value| value.id == wanted.id) {
            Some(existing) => {
                differ(
                    "predicate",
                    &wanted.id,
                    "label",
                    existing.label != wanted.label,
                );
                differ(
                    "predicate",
                    &wanted.id,
                    "object",
                    existing.object != wanted.object,
                );
                differ(
                    "predicate",
                    &wanted.id,
                    "cardinality",
                    existing.cardinality != wanted.cardinality,
                );
                differ(
                    "predicate",
                    &wanted.id,
                    "subjectTypes",
                    existing.subject_types != wanted.subject_types,
                );
            }
            None => write.predicates.push(wanted.clone()),
        }
    }
    for wanted in &input.axes {
        validate_axis_id(&wanted.id)?;
        if wanted.label.trim().is_empty() {
            return Err(UmweltError::validation("Axis labels cannot be empty."));
        }
        match stored.axes.iter().find(|value| value.id == wanted.id) {
            Some(existing) => differ("axis", &wanted.id, "label", existing.label != wanted.label),
            None => write.axes.push(wanted.clone()),
        }
    }
    Ok((write, divergences))
}
