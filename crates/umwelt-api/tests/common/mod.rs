// SPDX-License-Identifier: AGPL-3.0-only
//! Shared fixtures: project "p" with agents alice, bob, carol, a group "guild"
//! and an item; predicates "related" (many) and "location" (one).
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::Arc;

use umwelt_api::{Engine, FixedClock, SeqIdGen, SqliteStore};
use umwelt_core::domain::*;
use umwelt_core::{UmweltError, UmweltErrorCode};

pub type TestEngine = Engine<SqliteStore>;

pub fn engine() -> TestEngine {
    let mut engine = Engine::new(
        SqliteStore::in_memory().expect("create store"),
        Arc::new(FixedClock("2026-10-08T12:00:00Z".to_owned())),
        Arc::new(SeqIdGen::new()),
    );
    engine
        .create_project(CreateProjectInput {
            id: Some("p".to_owned()),
            name: "Test".to_owned(),
            description: None,
            timescale: "day".to_owned(),
            vocabulary_mode: VocabularyMode::Strict,
        })
        .expect("create project");
    for (id, kind) in [
        ("agent", EntityKind::Agent),
        ("group", EntityKind::Group),
        ("other", EntityKind::Other),
    ] {
        engine
            .create_entity_type(CreateEntityTypeInput {
                project_id: "p".to_owned(),
                id: Some(id.to_owned()),
                name: id.to_owned(),
                kind,
                fields: Vec::new(),
            })
            .expect("create entity type");
    }
    for (id, type_id) in [
        ("alice", "agent"),
        ("bob", "agent"),
        ("carol", "agent"),
        ("guild", "group"),
        ("item", "other"),
        ("town", "other"),
        ("castle", "other"),
    ] {
        engine
            .create_entity(CreateEntityInput {
                project_id: "p".to_owned(),
                id: Some(id.to_owned()),
                type_id: type_id.to_owned(),
                name: id.to_owned(),
                aliases: Vec::new(),
                fields: serde_json::Map::new(),
                external_ref: None,
                created_at: Position { tick: 0 },
            })
            .expect("create entity");
    }
    for (id, cardinality) in [
        ("related", Cardinality::Many),
        ("location", Cardinality::One),
    ] {
        engine
            .define_predicate(DefinePredicateInput {
                project_id: "p".to_owned(),
                predicate: PredicateDef {
                    id: id.to_owned(),
                    label: id.to_owned(),
                    object: ObjectKind::Entity,
                    cardinality,
                    subject_types: None,
                },
            })
            .expect("define predicate");
    }
    engine
}

pub fn provenance() -> Provenance {
    Provenance {
        origin: Origin::Human { actor_id: None },
        reason: None,
        metadata: None,
    }
}

pub fn claim(subject: &str, predicate: &str, object: &str) -> Claim {
    Claim {
        subject: subject.to_owned(),
        predicate: predicate.to_owned(),
        object: ClaimObject::Entity {
            id: object.to_owned(),
        },
        qualifiers: BTreeMap::new(),
        polarity: true,
        about: None,
    }
}

pub fn negated(mut value: Claim) -> Claim {
    value.polarity = !value.polarity;
    value
}

pub fn common(id: &str, tick: i64) -> RecordInputCommon {
    RecordInputCommon {
        id: Some(id.to_owned()),
        position: Position { tick },
        provenance: provenance(),
    }
}

pub fn holding(id: &str, holder: &str, tick: i64, value: Claim) -> RecordInput {
    RecordInput::Holding {
        common: common(id, tick),
        holder: holder.to_owned(),
        attitude: Attitude::Believes,
        claim: value,
        acquisition: Acquisition::Authored,
        confidence: 1.0,
        accessibility: Accessibility::Available,
        text: None,
        secrecy: None,
    }
}

pub fn fact(id: &str, tick: i64, value: Claim) -> RecordInput {
    RecordInput::Fact {
        common: common(id, tick),
        claim: value,
    }
}

pub fn event(id: &str, tick: i64) -> EventInput {
    EventInput {
        on: Default::default(),
        id: Some(id.to_owned()),
        event_type: "scene".to_owned(),
        title: id.to_owned(),
        description: None,
        position: Position { tick },
        participants: Vec::new(),
        location: None,
        evidence: Vec::new(),
        provenance: provenance(),
    }
}

/// A commit whose event id is `e-<attempt>`.
pub fn commit(attempt: &str, tick: i64, records: Vec<RecordInput>) -> CommitInput {
    CommitInput {
        project_id: "p".to_owned(),
        attempt_id: attempt.to_owned(),
        timeline_id: None,
        expected_version: None,
        event: event(&format!("e-{attempt}"), tick),
        records,
        transitions: Vec::new(),
        preconditions: Vec::new(),
    }
}

pub fn at(tick: i64) -> At {
    At {
        axis: None,
        tick: Some(tick),
        as_of_version: None,
    }
}

pub fn record_id(record: &Record) -> &str {
    match record {
        Record::Fact { common, .. }
        | Record::Holding { common, .. }
        | Record::Awareness { common, .. }
        | Record::Disposition { common, .. } => &common.id,
    }
}

pub fn expect_code<T: std::fmt::Debug>(
    result: Result<T, UmweltError>,
    code: UmweltErrorCode,
) -> UmweltError {
    let error = result.expect_err("operation must fail");
    assert_eq!(error.code, code, "{}", error.message);
    error
}
