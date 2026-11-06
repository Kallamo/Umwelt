// SPDX-License-Identifier: AGPL-3.0-only
//! Entities owned by another system, keyed by `externalRef`.

mod common;

use common::*;
use umwelt_core::UmweltErrorCode;
use umwelt_core::domain::*;

fn external(id: &str) -> ExternalRef {
    ExternalRef {
        system: "game".to_owned(),
        id: id.to_owned(),
    }
}

fn character(id: &str, name: &str) -> EnsureEntityInput {
    EnsureEntityInput {
        external_ref: external(id),
        id: None,
        type_id: "agent".to_owned(),
        name: name.to_owned(),
        aliases: None,
        fields: None,
        created_at: None,
    }
}

fn ensure(engine: &mut TestEngine, entities: Vec<EnsureEntityInput>) -> Vec<EnsureEntityResult> {
    engine
        .ensure_entities(EnsureEntitiesInput {
            project_id: "p".to_owned(),
            entities,
        })
        .expect("ensure entities")
}

#[test]
fn ensuring_twice_creates_once_and_keeps_ids() {
    let mut engine = engine();
    let mut with_id = character("Mira", "Mira");
    with_id.id = Some("npc-mira".to_owned());
    let first = ensure(&mut engine, vec![with_id, character("Theo", "Theo")]);
    assert!(first.iter().all(|result| result.created));
    assert_eq!(first[0].entity.id, "npc-mira");
    let second = ensure(
        &mut engine,
        vec![character("Mira", "Mira"), character("Theo", "Theo")],
    );
    assert!(second.iter().all(|result| !result.created));
    assert_eq!(second[0].entity.id, "npc-mira");
    assert_eq!(second[1].entity.id, first[1].entity.id);
}

#[test]
fn ensuring_updates_changed_fields_only() {
    let mut engine = engine();
    ensure(&mut engine, vec![character("Mira", "Mira")]);
    let mut renamed = character("Mira", "Mira Castell");
    renamed.aliases = Some(vec!["Pen".to_owned()]);
    let result = ensure(&mut engine, vec![renamed]);
    assert!(!result[0].created);
    let stored = engine
        .resolve_entity("p", &external("Mira"))
        .expect("resolve");
    assert_eq!(stored.name, "Mira Castell");
    assert_eq!(stored.aliases, ["Pen"]);
}

#[test]
fn a_failing_item_rolls_back_the_whole_batch() {
    let mut engine = engine();
    let mut unknown_type = character("Bad", "Bad");
    unknown_type.type_id = "missing".to_owned();
    expect_code(
        engine.ensure_entities(EnsureEntitiesInput {
            project_id: "p".to_owned(),
            entities: vec![character("Mira", "Mira"), unknown_type],
        }),
        UmweltErrorCode::NotFound,
    );
    expect_code(
        engine.resolve_entity("p", &external("Mira")),
        UmweltErrorCode::NotFound,
    );
}

#[test]
fn conflicting_inputs_are_rejected() {
    let mut engine = engine();
    expect_code(
        engine.ensure_entities(EnsureEntitiesInput {
            project_id: "p".to_owned(),
            entities: vec![character("Mira", "A"), character("Mira", "B")],
        }),
        UmweltErrorCode::ValidationFailed,
    );
    ensure(&mut engine, vec![character("Mira", "Mira")]);
    let mut retyped = character("Mira", "Mira");
    retyped.type_id = "other".to_owned();
    expect_code(
        engine.ensure_entities(EnsureEntitiesInput {
            project_id: "p".to_owned(),
            entities: vec![retyped],
        }),
        UmweltErrorCode::ValidationFailed,
    );
    let mut other_id = character("Mira", "Mira");
    other_id.id = Some("someone-else".to_owned());
    expect_code(
        engine.ensure_entities(EnsureEntitiesInput {
            project_id: "p".to_owned(),
            entities: vec![other_id],
        }),
        UmweltErrorCode::Conflict,
    );
}

#[test]
fn resolve_batch_maps_known_and_unknown_references() {
    let mut engine = engine();
    let created = ensure(&mut engine, vec![character("Mira", "Mira")]);
    let resolved = engine
        .resolve_entities("p", vec![external("Mira"), external("Nobody")])
        .expect("resolve batch");
    assert_eq!(
        resolved[0].entity_id.as_deref(),
        Some(created[0].entity.id.as_str())
    );
    assert_eq!(resolved[1].entity_id, None);
}
