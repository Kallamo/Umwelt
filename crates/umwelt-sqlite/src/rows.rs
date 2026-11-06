// SPDX-License-Identifier: AGPL-3.0-only
//! Row layouts: how records, events, transitions and provenance map to
//! columns, and back.
//!
//! Nothing that has a column is stored twice. A record row keeps its common
//! fields, its claim, its acquisition and its holder in columns; `extra` keeps
//! only what no query filters on and is NULL when that is empty.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use umwelt_core::domain::*;
use umwelt_core::{Result, UmweltError, UmweltErrorCode, claim_key, slot_key};

use crate::{database_error, enum_name, from_json, to_i64, to_json, to_u64};

/// Columns of a record read, in `RecordRow::read` order, for alias `r`. A
/// macro so SQL constants can embed it with `concat!`.
macro_rules! record_columns {
    () => {
        "r.id, r.project_id, r.timeline_id, r.kind, r.tick, r.recorded_version, r.event_id, \
         r.provenance_id, r.holder_id, r.subject_id, r.predicate, r.object_entity_id, \
         r.polarity, r.attitude, r.confidence, r.accessibility, r.shared_with, \
         r.acquisition_kind, r.source_holder_id, r.source_record_id, r.acquisition_ref, \
         r.target_id, r.dimension, r.extra"
    };
}

/// Columns of an event read, in `EventRow::read` order, for alias `e`.
macro_rules! event_columns {
    () => {
        "e.id, e.project_id, e.timeline_id, e.tick, e.recorded_version, e.event_type, e.title, \
         e.provenance_id, e.extra"
    };
}

/// Columns of a transition read, in `TransitionRow::read` order, for alias `t`.
macro_rules! transition_columns {
    () => {
        "t.id, t.project_id, t.timeline_id, t.target_id, t.status, t.superseded_by, t.tick, \
         t.recorded_version, t.ordinal, t.event_id, t.provenance_id"
    };
}

pub(crate) const RECORD_COLUMNS: &str = record_columns!();
/// How many columns `RECORD_COLUMNS` reads.
pub(crate) const RECORD_COLUMN_COUNT: usize = 24;

pub(crate) const INSERT_RECORD: &str = "INSERT INTO records (project_id, id, timeline_id, kind, \
     tick, recorded_version, event_id, provenance_id, holder_id, claim_key, slot_key, subject_id, \
     predicate, object_entity_id, polarity, attitude, confidence, accessibility, shared_with, \
     acquisition_kind, source_holder_id, source_record_id, acquisition_ref, target_id, dimension, \
     extra) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, \
     ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)";

pub(crate) const EVENT_COLUMNS: &str = event_columns!();
/// How many columns `EVENT_COLUMNS` reads.
pub(crate) const EVENT_COLUMN_COUNT: usize = 9;
pub(crate) const TRANSITION_COLUMNS: &str = transition_columns!();

/// Claim and slot keys are stored as the first 16 bytes of their SHA-256
/// value: 128 bits, the size of a UUID. The public keys stay 64 hex.
pub(crate) const STORED_KEY_BYTES: usize = 16;

/// A vector as stored: little-endian `f32`.
pub(crate) fn vector_blob(vector: &[f32]) -> Vec<u8> {
    vector
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

/// A stored vector back to `f32`.
pub(crate) fn blob_vector(blob: &[u8]) -> Result<Vec<f32>> {
    if blob.len() % 4 != 0 {
        return Err(UmweltError::public(
            UmweltErrorCode::InvalidState,
            "Stored vector has a partial value.",
        ));
    }
    Ok(blob
        .chunks_exact(4)
        .map(|bytes| f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        .collect())
}

/// The stored form of a text hash: the first 16 bytes of its SHA-256, the
/// same truncation as claim keys.
pub(crate) fn stored_text_hash(text: &str) -> Vec<u8> {
    Sha256::digest(text.as_bytes())[..STORED_KEY_BYTES].to_vec()
}

/// A stored key from a public 64-hex key. A key that is not valid hex cannot
/// match anything, so it is not an error: the caller simply gets no rows.
pub(crate) fn key_blob(hex: &str) -> Option<Vec<u8>> {
    if hex.len() != 64 {
        return None;
    }
    let bytes = (0..64)
        .step_by(2)
        .map(|index| u8::from_str_radix(hex.get(index..index + 2)?, 16).ok())
        .collect::<Option<Vec<u8>>>()?;
    Some(bytes[..STORED_KEY_BYTES].to_vec())
}

fn computed_key(hex: &str) -> Result<Vec<u8>> {
    key_blob(hex).ok_or_else(|| {
        UmweltError::public(
            UmweltErrorCode::InvalidState,
            "Computed claim key is not 64 hex characters.",
        )
    })
}

/// Content-derived provenance id: the first 8 bytes of the SHA-256 of its
/// JSON. The same provenance always gets the same id, so an id read once can
/// be cached for the life of the store.
pub(crate) fn provenance_id(json: &str) -> i64 {
    let digest = Sha256::digest(json.as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    i64::from_be_bytes(bytes)
}

/// Provenance values already read, by project and id. Safe to keep for the
/// whole life of the store because ids are derived from content.
#[derive(Debug, Default)]
pub(crate) struct ProvenanceCache {
    values: RefCell<HashMap<(String, i64), Provenance>>,
}

impl ProvenanceCache {
    pub(crate) fn get(
        &self,
        connection: &Connection,
        project_id: &str,
        id: i64,
    ) -> Result<Provenance> {
        let key = (project_id.to_owned(), id);
        if let Some(value) = self.values.borrow().get(&key) {
            return Ok(value.clone());
        }
        let payload: String = connection
            .prepare_cached("SELECT payload FROM provenances WHERE project_id = ?1 AND id = ?2")
            .map_err(database_error)?
            .query_row(params![project_id, id], |row| row.get(0))
            .optional()
            .map_err(database_error)?
            .ok_or_else(|| {
                UmweltError::public(
                    UmweltErrorCode::InvalidState,
                    format!("Provenance {id} is missing."),
                )
            })?;
        let value: Provenance = from_json(&payload)?;
        self.values.borrow_mut().insert(key, value.clone());
        Ok(value)
    }
}

/// Provenance ids written in the current transaction. Not kept across
/// transactions: a rolled-back insert must be written again next time.
#[derive(Debug, Default)]
pub(crate) struct ProvenanceWrites {
    written: RefCell<HashSet<(String, i64)>>,
}

impl ProvenanceWrites {
    /// Stores the provenance once and returns its id.
    pub(crate) fn intern(
        &self,
        connection: &Connection,
        project_id: &str,
        provenance: &Provenance,
    ) -> Result<i64> {
        let json = to_json(provenance)?;
        let id = provenance_id(&json);
        let key = (project_id.to_owned(), id);
        if self.written.borrow().contains(&key) {
            return Ok(id);
        }
        connection
            .prepare_cached(
                "INSERT INTO provenances (project_id, id, payload) VALUES (?1, ?2, ?3) \
                 ON CONFLICT (project_id, id) DO NOTHING",
            )
            .map_err(database_error)?
            .execute(params![project_id, id, json])
            .map_err(database_error)?;
        self.written.borrow_mut().insert(key);
        Ok(id)
    }
}

/// Record fields without a column.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordExtra {
    /// The claim object when it is not an entity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    object: Option<ClaimObject>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    qualifiers: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    about: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    intent: Option<Intent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    from_record_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rule: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    /// A Disposition's value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    value: Option<Value>,
}

impl RecordExtra {
    fn is_empty(&self) -> bool {
        self.object.is_none()
            && self.qualifiers.is_empty()
            && self.about.is_none()
            && self.intent.is_none()
            && self.from_record_ids.is_none()
            && self.rule.is_none()
            && self.text.is_none()
            && self.value.is_none()
    }

    fn encode(&self) -> Result<Option<String>> {
        if self.is_empty() {
            Ok(None)
        } else {
            to_json(self).map(Some)
        }
    }

    fn decode(text: Option<&str>) -> Result<Self> {
        text.map(from_json)
            .transpose()
            .map(Option::unwrap_or_default)
    }
}

/// The columns of one record, ready to bind to `INSERT_RECORD`.
pub(crate) struct RecordColumns<'a> {
    common: &'a RecordCommon,
    kind: &'static str,
    provenance_id: i64,
    holder: Option<&'a str>,
    claim_key: Option<Vec<u8>>,
    slot_key: Option<Vec<u8>>,
    subject: Option<&'a str>,
    predicate: Option<&'a str>,
    object_entity: Option<&'a str>,
    polarity: Option<bool>,
    attitude: Option<String>,
    confidence: Option<f64>,
    accessibility: Option<String>,
    shared_with: Option<String>,
    acquisition_kind: Option<&'static str>,
    source_holder: Option<&'a str>,
    source_record: Option<&'a str>,
    acquisition_ref: Option<&'a str>,
    target: Option<&'a str>,
    dimension: Option<&'a str>,
    extra: Option<String>,
}

impl<'a> RecordColumns<'a> {
    pub(crate) fn from_record(record: &'a Record, provenance_id: i64) -> Result<Self> {
        let mut columns = Self {
            common: record_common(record),
            kind: "fact",
            provenance_id,
            holder: None,
            claim_key: None,
            slot_key: None,
            subject: None,
            predicate: None,
            object_entity: None,
            polarity: None,
            attitude: None,
            confidence: None,
            accessibility: None,
            shared_with: None,
            acquisition_kind: None,
            source_holder: None,
            source_record: None,
            acquisition_ref: None,
            target: None,
            dimension: None,
            extra: None,
        };
        let mut extra = RecordExtra::default();
        match record {
            Record::Fact { claim, .. } => columns.claim(claim, &mut extra)?,
            Record::Holding {
                holder,
                attitude,
                claim,
                acquisition,
                confidence,
                accessibility,
                text,
                secrecy,
                ..
            } => {
                columns.kind = "holding";
                columns.holder = Some(holder);
                columns.claim(claim, &mut extra)?;
                columns.attitude = Some(enum_name(attitude)?);
                columns.confidence = Some(*confidence);
                columns.accessibility = Some(enum_name(accessibility)?);
                columns.shared_with = secrecy
                    .as_ref()
                    .map(|value| to_json(&value.shared_with))
                    .transpose()?;
                extra.text = text.clone();
                match acquisition {
                    Acquisition::Witnessed { event_id } => {
                        columns.acquisition_kind = Some("witnessed");
                        columns.acquisition_ref = Some(event_id);
                    }
                    Acquisition::ToldBy {
                        source_holder_id,
                        source_record_id,
                        intent,
                    } => {
                        columns.acquisition_kind = Some("told-by");
                        columns.source_holder = Some(source_holder_id);
                        columns.source_record = source_record_id.as_deref();
                        extra.intent = Some(intent.clone());
                    }
                    Acquisition::Evidence { evidence_id } => {
                        columns.acquisition_kind = Some("evidence");
                        columns.acquisition_ref = Some(evidence_id);
                    }
                    Acquisition::Inferred {
                        from_record_ids,
                        rule,
                    } => {
                        columns.acquisition_kind = Some("inferred");
                        extra.from_record_ids = Some(from_record_ids.clone());
                        extra.rule = rule.clone();
                    }
                    Acquisition::Authored => columns.acquisition_kind = Some("authored"),
                }
            }
            Record::Awareness { holder, target, .. } => {
                columns.kind = "awareness";
                columns.holder = Some(holder);
                columns.target = Some(target);
            }
            Record::Disposition {
                holder,
                target,
                dimension,
                value,
                ..
            } => {
                columns.kind = "disposition";
                columns.holder = Some(holder);
                columns.target = Some(target);
                columns.dimension = Some(dimension);
                extra.value = Some(value.clone());
            }
        }
        columns.extra = extra.encode()?;
        Ok(columns)
    }

    fn claim(&mut self, claim: &'a Claim, extra: &mut RecordExtra) -> Result<()> {
        self.claim_key = Some(computed_key(&claim_key(claim))?);
        self.slot_key = Some(computed_key(&slot_key(claim))?);
        self.subject = Some(&claim.subject);
        self.predicate = Some(&claim.predicate);
        self.polarity = Some(claim.polarity);
        match &claim.object {
            ClaimObject::Entity { id } => self.object_entity = Some(id),
            other => extra.object = Some(other.clone()),
        }
        extra.qualifiers = claim.qualifiers.clone();
        extra.about = claim.about.clone();
        Ok(())
    }

    pub(crate) fn insert(&self, connection: &Connection) -> Result<()> {
        connection
            .prepare_cached(INSERT_RECORD)
            .map_err(database_error)?
            .execute(params![
                self.common.project_id,
                self.common.id,
                self.common.timeline_id,
                self.kind,
                self.common.position.tick,
                to_i64(self.common.recorded_at)?,
                self.common.event_id,
                self.provenance_id,
                self.holder,
                self.claim_key,
                self.slot_key,
                self.subject,
                self.predicate,
                self.object_entity,
                self.polarity,
                self.attitude,
                self.confidence,
                self.accessibility,
                self.shared_with,
                self.acquisition_kind,
                self.source_holder,
                self.source_record,
                self.acquisition_ref,
                self.target,
                self.dimension,
                self.extra,
            ])
            .map_err(database_error)?;
        Ok(())
    }
}

pub(crate) fn record_common(record: &Record) -> &RecordCommon {
    match record {
        Record::Fact { common, .. }
        | Record::Holding { common, .. }
        | Record::Awareness { common, .. }
        | Record::Disposition { common, .. } => common,
    }
}

/// One record as read from `RECORD_COLUMNS`.
pub(crate) struct RecordRow {
    id: String,
    project_id: String,
    timeline_id: String,
    kind: String,
    tick: i64,
    recorded_version: i64,
    event_id: String,
    provenance_id: i64,
    holder: Option<String>,
    subject: Option<String>,
    predicate: Option<String>,
    object_entity: Option<String>,
    polarity: Option<bool>,
    attitude: Option<String>,
    confidence: Option<f64>,
    accessibility: Option<String>,
    shared_with: Option<String>,
    acquisition_kind: Option<String>,
    source_holder: Option<String>,
    source_record: Option<String>,
    acquisition_ref: Option<String>,
    target: Option<String>,
    dimension: Option<String>,
    extra: Option<String>,
}

impl RecordRow {
    /// Reads `RECORD_COLUMNS` starting at column `start`.
    pub(crate) fn read(row: &Row<'_>, start: usize) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(start)?,
            project_id: row.get(start + 1)?,
            timeline_id: row.get(start + 2)?,
            kind: row.get(start + 3)?,
            tick: row.get(start + 4)?,
            recorded_version: row.get(start + 5)?,
            event_id: row.get(start + 6)?,
            provenance_id: row.get(start + 7)?,
            holder: row.get(start + 8)?,
            subject: row.get(start + 9)?,
            predicate: row.get(start + 10)?,
            object_entity: row.get(start + 11)?,
            polarity: row.get(start + 12)?,
            attitude: row.get(start + 13)?,
            confidence: row.get(start + 14)?,
            accessibility: row.get(start + 15)?,
            shared_with: row.get(start + 16)?,
            acquisition_kind: row.get(start + 17)?,
            source_holder: row.get(start + 18)?,
            source_record: row.get(start + 19)?,
            acquisition_ref: row.get(start + 20)?,
            target: row.get(start + 21)?,
            dimension: row.get(start + 22)?,
            extra: row.get(start + 23)?,
        })
    }

    /// Rebuilds the record.
    pub(crate) fn into_record(
        self,
        connection: &Connection,
        provenances: &ProvenanceCache,
    ) -> Result<Record> {
        let provenance = provenances.get(connection, &self.project_id, self.provenance_id)?;
        let mut extra = RecordExtra::decode(self.extra.as_deref())?;
        let common = RecordCommon {
            id: self.id,
            project_id: self.project_id,
            timeline_id: self.timeline_id,
            position: Position { tick: self.tick },
            recorded_at: to_u64(self.recorded_version)?,
            event_id: self.event_id,
            provenance,
        };
        let holder = self.holder;
        let required =
            |value: Option<String>, what: &str| value.ok_or_else(|| corrupt(&common.id, what));
        let record = match self.kind.as_str() {
            "fact" | "holding" => {
                let claim = Claim {
                    subject: required(self.subject, "subject")?,
                    predicate: required(self.predicate, "predicate")?,
                    object: match (self.object_entity, extra.object.take()) {
                        (Some(id), _) => ClaimObject::Entity { id },
                        (None, Some(object)) => object,
                        (None, None) => ClaimObject::None,
                    },
                    qualifiers: std::mem::take(&mut extra.qualifiers),
                    polarity: self
                        .polarity
                        .ok_or_else(|| corrupt(&common.id, "polarity"))?,
                    about: extra.about.take(),
                };
                if self.kind == "fact" {
                    Record::Fact { common, claim }
                } else {
                    let acquisition = match self.acquisition_kind.as_deref() {
                        Some("witnessed") => Acquisition::Witnessed {
                            event_id: required(self.acquisition_ref, "witnessed event")?,
                        },
                        Some("told-by") => Acquisition::ToldBy {
                            source_holder_id: required(self.source_holder, "source holder")?,
                            source_record_id: self.source_record,
                            intent: extra.intent.take().unwrap_or(Intent::Unknown),
                        },
                        Some("evidence") => Acquisition::Evidence {
                            evidence_id: required(self.acquisition_ref, "evidence")?,
                        },
                        Some("inferred") => Acquisition::Inferred {
                            from_record_ids: extra.from_record_ids.take().unwrap_or_default(),
                            rule: extra.rule.take(),
                        },
                        _ => Acquisition::Authored,
                    };
                    Record::Holding {
                        holder: required(holder, "holder")?,
                        attitude: from_name(&required(self.attitude, "attitude")?)?,
                        claim,
                        acquisition,
                        confidence: self
                            .confidence
                            .ok_or_else(|| corrupt(&common.id, "confidence"))?,
                        accessibility: from_name(&required(self.accessibility, "accessibility")?)?,
                        text: extra.text.take(),
                        secrecy: self
                            .shared_with
                            .map(|text| from_json(&text).map(|shared_with| Secrecy { shared_with }))
                            .transpose()?,
                        common,
                    }
                }
            }
            "awareness" => Record::Awareness {
                holder: required(holder, "holder")?,
                target: required(self.target, "target")?,
                common,
            },
            "disposition" => Record::Disposition {
                holder: required(holder, "holder")?,
                target: required(self.target, "target")?,
                dimension: required(self.dimension, "dimension")?,
                value: extra.value.take().unwrap_or(Value::Null),
                common,
            },
            other => {
                return Err(UmweltError::public(
                    UmweltErrorCode::InvalidState,
                    format!("Unknown record kind '{other}'."),
                ));
            }
        };
        Ok(record)
    }
}

fn corrupt(id: &str, what: &str) -> UmweltError {
    UmweltError::public(
        UmweltErrorCode::InvalidState,
        format!("Stored record '{id}' has no {what}."),
    )
}

fn from_name<T: for<'de> Deserialize<'de>>(name: &str) -> Result<T> {
    from_json(&format!("\"{name}\""))
}

/// Event fields without a column.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventExtra {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    participants: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    location: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    evidence: Vec<EvidenceRef>,
}

pub(crate) fn insert_event_row(
    connection: &Connection,
    event: &Event,
    provenance_id: i64,
) -> Result<()> {
    let extra = EventExtra {
        description: event.description.clone(),
        participants: event.participants.clone(),
        location: event.location.clone(),
        evidence: event.evidence.clone(),
    };
    let extra = if extra.description.is_none()
        && extra.participants.is_empty()
        && extra.location.is_none()
        && extra.evidence.is_empty()
    {
        None
    } else {
        Some(to_json(&extra)?)
    };
    connection
        .prepare_cached(
            "INSERT INTO events (project_id, id, timeline_id, tick, recorded_version, event_type, \
             title, provenance_id, extra) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .map_err(database_error)?
        .execute(params![
            event.project_id,
            event.id,
            event.timeline_id,
            event.position.tick,
            to_i64(event.recorded_at)?,
            event.event_type,
            event.title,
            provenance_id,
            extra
        ])
        .map_err(database_error)?;
    for (axis, tick) in &event.on {
        connection
            .prepare_cached(
                "INSERT INTO event_positions (project_id, event_id, axis_id, tick,                  recorded_version) VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .map_err(database_error)?
            .execute(params![
                event.project_id,
                event.id,
                axis,
                tick,
                to_i64(event.recorded_at)?
            ])
            .map_err(database_error)?;
    }
    Ok(())
}

/// An event's positions on additional axes, through the primary key of
/// `event_positions`.
fn event_positions(
    connection: &Connection,
    project_id: &str,
    event_id: &str,
) -> Result<AxisPositions> {
    let mut statement = connection
        .prepare_cached(
            "SELECT axis_id, tick FROM event_positions WHERE project_id = ?1 AND event_id = ?2",
        )
        .map_err(database_error)?;
    let rows = statement
        .query_map(params![project_id, event_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(database_error)?;
    rows.map(|row| row.map_err(database_error)).collect()
}

/// One event as read from `EVENT_COLUMNS`.
pub(crate) struct EventRow {
    id: String,
    project_id: String,
    timeline_id: String,
    tick: i64,
    recorded_version: i64,
    event_type: String,
    title: String,
    provenance_id: i64,
    extra: Option<String>,
}

impl EventRow {
    pub(crate) fn read(row: &Row<'_>, start: usize) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(start)?,
            project_id: row.get(start + 1)?,
            timeline_id: row.get(start + 2)?,
            tick: row.get(start + 3)?,
            recorded_version: row.get(start + 4)?,
            event_type: row.get(start + 5)?,
            title: row.get(start + 6)?,
            provenance_id: row.get(start + 7)?,
            extra: row.get(start + 8)?,
        })
    }

    pub(crate) fn into_event(
        self,
        connection: &Connection,
        provenances: &ProvenanceCache,
    ) -> Result<Event> {
        let extra: EventExtra = self
            .extra
            .as_deref()
            .map(from_json)
            .transpose()?
            .unwrap_or_default();
        let on = event_positions(connection, &self.project_id, &self.id)?;
        Ok(Event {
            on,
            provenance: provenances.get(connection, &self.project_id, self.provenance_id)?,
            id: self.id,
            project_id: self.project_id,
            timeline_id: self.timeline_id,
            event_type: self.event_type,
            title: self.title,
            description: extra.description,
            position: Position { tick: self.tick },
            participants: extra.participants,
            location: extra.location,
            evidence: extra.evidence,
            recorded_at: to_u64(self.recorded_version)?,
        })
    }
}

pub(crate) fn insert_transition_row(
    connection: &Connection,
    transition: &Transition,
    provenance_id: i64,
) -> Result<()> {
    connection
        .prepare_cached(
            "INSERT INTO transitions (project_id, id, timeline_id, target_id, status, \
             superseded_by, tick, recorded_version, ordinal, event_id, provenance_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .map_err(database_error)?
        .execute(params![
            transition.project_id,
            transition.id,
            transition.timeline_id,
            transition.target_record_id,
            enum_name(&transition.status)?,
            transition.superseded_by,
            transition.position.tick,
            to_i64(transition.recorded_at)?,
            transition.ordinal,
            transition.event_id,
            provenance_id
        ])
        .map_err(database_error)?;
    Ok(())
}

/// One transition as read from `TRANSITION_COLUMNS`.
pub(crate) struct TransitionRow {
    id: String,
    project_id: String,
    timeline_id: String,
    target: String,
    status: String,
    superseded_by: Option<String>,
    tick: i64,
    recorded_version: i64,
    ordinal: u32,
    event_id: String,
    provenance_id: i64,
}

impl TransitionRow {
    pub(crate) fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            project_id: row.get(1)?,
            timeline_id: row.get(2)?,
            target: row.get(3)?,
            status: row.get(4)?,
            superseded_by: row.get(5)?,
            tick: row.get(6)?,
            recorded_version: row.get(7)?,
            ordinal: row.get(8)?,
            event_id: row.get(9)?,
            provenance_id: row.get(10)?,
        })
    }

    pub(crate) fn into_transition(
        self,
        connection: &Connection,
        provenances: &ProvenanceCache,
    ) -> Result<Transition> {
        Ok(Transition {
            provenance: provenances.get(connection, &self.project_id, self.provenance_id)?,
            id: self.id,
            project_id: self.project_id,
            timeline_id: self.timeline_id,
            target_record_id: self.target,
            status: from_name(&self.status)?,
            superseded_by: self.superseded_by,
            position: Position { tick: self.tick },
            recorded_at: to_u64(self.recorded_version)?,
            ordinal: self.ordinal,
            event_id: self.event_id,
        })
    }
}

/// What an attempt keeps besides its fingerprint, event and version.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AttemptExtra {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) record_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) transition_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) warnings: Vec<Warning>,
    /// Checked again when a draft is rebased.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) preconditions: Vec<Precondition>,
}

impl AttemptExtra {
    pub(crate) fn encode(&self) -> Result<Option<String>> {
        if self.record_ids.is_empty()
            && self.transition_ids.is_empty()
            && self.warnings.is_empty()
            && self.preconditions.is_empty()
        {
            Ok(None)
        } else {
            to_json(self).map(Some)
        }
    }

    pub(crate) fn decode(text: Option<&str>) -> Result<Self> {
        text.map(from_json)
            .transpose()
            .map(Option::unwrap_or_default)
    }
}

/// A 64-hex fingerprint as 32 bytes.
pub(crate) fn fingerprint_blob(hex: &str) -> Result<Vec<u8>> {
    if hex.len() != 64 {
        return Err(invalid_fingerprint());
    }
    (0..64)
        .step_by(2)
        .map(|index| {
            hex.get(index..index + 2)
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                .ok_or_else(invalid_fingerprint)
        })
        .collect()
}

fn invalid_fingerprint() -> UmweltError {
    UmweltError::public(
        UmweltErrorCode::InvalidState,
        "A request fingerprint is not 64 hex characters.",
    )
}

pub(crate) fn fingerprint_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
