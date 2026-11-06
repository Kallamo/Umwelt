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
    pub position: Position,
    pub provenance: Provenance,
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
    pub position: Position,
    pub provenance: Provenance,
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
    pub holds: bool,
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
    pub status: TransitionStatus,
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
