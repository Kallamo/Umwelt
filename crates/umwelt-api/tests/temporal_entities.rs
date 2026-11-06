// SPDX-License-Identifier: AGPL-3.0-only
//! Names, titles and aliases that change over time, objectively and as known.

mod common;

use std::collections::BTreeMap;

use common::*;
use serde_json::json;
use umwelt_core::UmweltErrorCode;
use umwelt_core::domain::*;

fn named(subject: &str, predicate: &str, text: &str) -> Claim {
    Claim {
        subject: subject.to_owned(),
        predicate: predicate.to_owned(),
        object: ClaimObject::Value { value: json!(text) },
        qualifiers: BTreeMap::new(),
        polarity: true,
        about: None,
    }
}

fn story() -> TestEngine {
    let mut engine = engine();
    engine
        .commit(commit(
            "intro",
            1,
            vec![
                fact("n1", 1, named("bob", NAME_PREDICATE_ID, "The Stranger")),
                fact("t1", 1, named("bob", TITLE_PREDICATE_ID, "Prince")),
                holding(
                    "k-alice",
                    "alice",
                    1,
                    named("bob", NAME_PREDICATE_ID, "The Stranger"),
                ),
            ],
        ))
        .expect("intro");
    engine
        .commit(commit(
            "reveal",
            5,
            vec![
                fact("n2", 5, named("bob", NAME_PREDICATE_ID, "Kael")),
                fact("t2", 5, named("bob", TITLE_PREDICATE_ID, "King")),
                fact("a1", 5, named("bob", ALIAS_PREDICATE_ID, "The Stranger")),
                holding(
                    "k-carol",
                    "carol",
                    5,
                    named("bob", NAME_PREDICATE_ID, "Kael"),
                ),
            ],
        ))
        .expect("reveal");
    engine
}

#[test]
fn canon_names_follow_the_read_position() {
    let engine = story();
    let early = engine.entity_at("p", "bob", at(2)).expect("early");
    assert_eq!(early.name, "The Stranger");
    assert_eq!(early.title.as_deref(), Some("Prince"));
    assert!(early.aliases.is_empty());
    let late = engine.entity_at("p", "bob", at(6)).expect("late");
    assert_eq!(late.name, "Kael");
    assert_eq!(late.title.as_deref(), Some("King"));
    assert_eq!(late.aliases, ["The Stranger"]);
    assert!(late.record_ids.contains(&"n2".to_owned()));
    let unnamed = engine.entity_at("p", "carol", at(6)).expect("fallback");
    assert_eq!(
        unnamed.name, "carol",
        "no naming fact: the entity's own name"
    );
}

#[test]
fn each_holder_knows_the_names_it_was_told() {
    let engine = story();
    let alice = engine
        .entity_known_by("p", "bob", "alice", at(6))
        .expect("alice");
    assert_eq!(
        alice.name.as_deref(),
        Some("The Stranger"),
        "the reveal did not reach alice"
    );
    assert_eq!(alice.title, None);
    let carol = engine
        .entity_known_by("p", "bob", "carol", at(6))
        .expect("carol");
    assert_eq!(carol.name.as_deref(), Some("Kael"));
    let alice_unknown = engine
        .entity_known_by("p", "alice", "carol", at(6))
        .expect("carol about alice");
    assert_eq!(alice_unknown.name, None);
    assert!(!alice_unknown.aware);
}

#[test]
fn awareness_is_reported_without_granting_a_name() {
    let mut engine = story();
    engine
        .commit(commit(
            "seen",
            6,
            vec![RecordInput::Awareness {
                common: common("aw", 6),
                holder: "carol".to_owned(),
                target: "alice".to_owned(),
            }],
        ))
        .expect("awareness");
    let known = engine
        .entity_known_by("p", "alice", "carol", at(6))
        .expect("known");
    assert!(known.aware);
    assert_eq!(known.name, None);
}

#[test]
fn naming_predicates_need_string_values() {
    let mut engine = engine();
    let mut not_text = named("bob", NAME_PREDICATE_ID, "x");
    not_text.object = ClaimObject::Value { value: json!(42) };
    expect_code(
        engine.commit(commit("bad", 1, vec![fact("f", 1, not_text)])),
        UmweltErrorCode::ValidationFailed,
    );
    expect_code(
        engine.commit(commit(
            "blank",
            1,
            vec![fact("f", 1, named("bob", TITLE_PREDICATE_ID, " "))],
        )),
        UmweltErrorCode::ValidationFailed,
    );
}
