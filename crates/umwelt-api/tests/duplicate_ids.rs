// SPDX-License-Identifier: AGPL-3.0-only
//! Creating an object whose id is already taken fails with `CONFLICT`, never
//! with a storage error, and leaves the stored object unchanged. Defining an
//! entity type or a predicate again with the same definition is a no-op.

mod common;

use common::*;
use umwelt_core::UmweltErrorCode;
use umwelt_core::domain::*;

fn project_input(name: &str) -> CreateProjectInput {
    CreateProjectInput {
        id: Some("p".to_owned()),
        name: name.to_owned(),
        description: None,
        timescale: "day".to_owned(),
        vocabulary_mode: VocabularyMode::Strict,
    }
}

fn entity_input(id: &str, external_ref: Option<ExternalRef>) -> CreateEntityInput {
    CreateEntityInput {
        project_id: "p".to_owned(),
        id: Some(id.to_owned()),
        type_id: "agent".to_owned(),
        name: "Someone else".to_owned(),
        aliases: Vec::new(),
        fields: serde_json::Map::new(),
        external_ref,
        created_at: Position { tick: 0 },
    }
}

fn external(id: &str) -> ExternalRef {
    ExternalRef {
        system: "game".to_owned(),
        id: id.to_owned(),
    }
}

fn evidence(title: &str) -> EvidenceSource {
    EvidenceSource {
        id: "letter".to_owned(),
        project_id: "p".to_owned(),
        source_type: "document".to_owned(),
        title: title.to_owned(),
        description: None,
        locator: None,
        content_hash: None,
        metadata: None,
        created_at: "2026-10-08T12:00:00Z".to_owned(),
    }
}

fn location(label: &str, cardinality: Cardinality) -> DefinePredicateInput {
    DefinePredicateInput {
        project_id: "p".to_owned(),
        predicate: PredicateDef {
            id: "location".to_owned(),
            label: label.to_owned(),
            object: ObjectKind::Entity,
            cardinality,
            subject_types: None,
        },
    }
}

#[test]
fn project_id_conflict() {
    let mut engine = engine();
    expect_code(
        engine.create_project(project_input("Again")),
        UmweltErrorCode::Conflict,
    );
    assert_eq!(engine.project("p").expect("project").name, "Test");
}

fn agent_type(name: &str, kind: EntityKind) -> CreateEntityTypeInput {
    CreateEntityTypeInput {
        project_id: "p".to_owned(),
        id: Some("agent".to_owned()),
        name: name.to_owned(),
        kind,
        fields: Vec::new(),
    }
}

#[test]
fn identical_entity_type_definition_is_a_no_op() {
    let mut engine = engine();
    let defined = engine
        .create_entity_type(agent_type("agent", EntityKind::Agent))
        .expect("same definition");
    assert_eq!(defined.kind, EntityKind::Agent);
    let count = engine
        .entity_types("p")
        .expect("entity types")
        .iter()
        .filter(|value| value.id == "agent")
        .count();
    assert_eq!(count, 1);
}

#[test]
fn entity_type_id_conflict() {
    let mut engine = engine();
    let error = expect_code(
        engine.create_entity_type(agent_type("Robot", EntityKind::Other)),
        UmweltErrorCode::Conflict,
    );
    let details = error.details.expect("details");
    assert_eq!(details["entityType"]["name"], "agent");
    assert_eq!(details["entityType"]["kind"], "agent");
    let types = engine.entity_types("p").expect("entity types");
    let agent = types
        .iter()
        .find(|value| value.id == "agent")
        .expect("agent");
    assert_eq!(agent.kind, EntityKind::Agent);
}

#[test]
fn entity_id_conflict() {
    let mut engine = engine();
    expect_code(
        engine.create_entity(entity_input("alice", None)),
        UmweltErrorCode::Conflict,
    );
    assert_eq!(engine.entity("p", "alice").expect("alice").name, "alice");
}

#[test]
fn external_reference_conflict_on_create_and_update() {
    let mut engine = engine();
    engine
        .create_entity(entity_input("dave", Some(external("npc-1"))))
        .expect("dave");
    expect_code(
        engine.create_entity(entity_input("erin", Some(external("npc-1")))),
        UmweltErrorCode::Conflict,
    );
    expect_code(engine.entity("p", "erin"), UmweltErrorCode::NotFound);

    let error = expect_code(
        engine.update_entity(UpdateEntityInput {
            project_id: "p".to_owned(),
            entity_id: "alice".to_owned(),
            name: None,
            aliases: None,
            fields: None,
            external_ref: Some(external("npc-1")),
        }),
        UmweltErrorCode::Conflict,
    );
    assert_eq!(
        error.message,
        "Another entity in project 'p' already has the external reference 'game:npc-1'."
    );
    assert_eq!(
        engine.entity("p", "alice").expect("alice").external_ref,
        None
    );
}

#[test]
fn evidence_id_conflict() {
    let mut engine = engine();
    engine.put_evidence(evidence("Letter")).expect("register");
    let error = expect_code(
        engine.put_evidence(evidence("Forgery")),
        UmweltErrorCode::Conflict,
    );
    assert_eq!(
        error.message,
        "Evidence source 'letter' already exists in project 'p'."
    );
    assert_eq!(
        engine.evidence("p", "letter").expect("letter").title,
        "Letter"
    );
}

#[test]
fn identical_predicate_definition_is_a_no_op() {
    let mut engine = engine();
    let defined = engine
        .define_predicate(location("location", Cardinality::One))
        .expect("same definition");
    assert_eq!(defined.label, "location");
    let count = engine
        .predicates("p")
        .expect("predicates")
        .iter()
        .filter(|value| value.id == "location")
        .count();
    assert_eq!(count, 1);
}

#[test]
fn different_predicate_definition_conflicts() {
    let mut engine = engine();
    let error = expect_code(
        engine.define_predicate(location("Where", Cardinality::Many)),
        UmweltErrorCode::Conflict,
    );
    let details = error.details.expect("details");
    assert_eq!(details["predicate"]["label"], "location");
    assert_eq!(details["predicate"]["cardinality"], "one");
    let stored = engine.predicates("p").expect("predicates");
    let location = stored
        .iter()
        .find(|value| value.id == "location")
        .expect("location");
    assert_eq!(location.cardinality, Cardinality::One);
}
