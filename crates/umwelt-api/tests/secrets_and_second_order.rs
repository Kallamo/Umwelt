// SPDX-License-Identifier: AGPL-3.0-only
//! Second-order knowledge ("she believes he knows") and secrets.

mod common;

use std::collections::BTreeMap;

use common::*;
use serde_json::json;
use umwelt_api::claim_key;
use umwelt_core::UmweltErrorCode;
use umwelt_core::domain::*;

fn affair() -> Claim {
    claim("alice", "related", "castle")
}

fn thinks_holds(other: &str, key: &str, polarity: bool) -> Claim {
    Claim {
        subject: other.to_owned(),
        predicate: HOLDS_PREDICATE_ID.to_owned(),
        object: ClaimObject::Value {
            value: json!({ "claimKey": key }),
        },
        qualifiers: BTreeMap::new(),
        polarity,
        about: None,
    }
}

fn secret_holding(id: &str, holder: &str, shared_with: &[&str]) -> RecordInput {
    let RecordInput::Holding {
        common,
        holder,
        attitude,
        claim,
        acquisition,
        confidence,
        accessibility,
        text,
        ..
    } = holding(id, holder, 1, affair())
    else {
        unreachable!("holding() builds a Holding");
    };
    RecordInput::Holding {
        common,
        holder,
        attitude,
        claim,
        acquisition,
        confidence,
        accessibility,
        text,
        secrecy: Some(Secrecy {
            shared_with: shared_with
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        }),
    }
}

fn tell(attempt: &str, from: &str, to: &str, source: &str, tick: i64) -> TransmitInput {
    TransmitInput {
        on: Default::default(),
        project_id: "p".to_owned(),
        attempt_id: attempt.to_owned(),
        timeline_id: None,
        position: Position { tick },
        from: from.to_owned(),
        to: to.to_owned(),
        source_record_id: source.to_owned(),
        record_id: Some(format!("{to}-heard")),
        attitude: Attitude::Believes,
        confidence: 1.0,
        intent: Intent::Honest,
        distorted_claim: None,
        text: None,
    }
}

#[test]
fn second_order_beliefs_are_compared_with_what_the_other_really_holds() {
    let mut engine = engine();
    let key = claim_key(&affair());
    engine
        .commit(commit(
            "setup",
            1,
            vec![
                holding("bob-knows", "bob", 1, affair()),
                holding("alice-thinks", "alice", 1, thinks_holds("bob", &key, true)),
                holding(
                    "alice-thinks-carol",
                    "alice",
                    1,
                    thinks_holds("carol", &key, true),
                ),
            ],
        ))
        .expect("setup");
    let about_bob = engine
        .beliefs_about("p", "alice", "bob", At::default())
        .expect("about bob");
    assert_eq!(about_bob.len(), 1);
    assert_eq!(about_bob[0].claim_key, key);
    assert!(about_bob[0].actually_holds);
    assert_eq!(about_bob[0].actual_record_id.as_deref(), Some("bob-knows"));
    let about_carol = engine
        .beliefs_about("p", "alice", "carol", At::default())
        .expect("about carol");
    assert!(!about_carol[0].actually_holds, "alice is wrong about carol");
    assert!(
        engine
            .beliefs_about("p", "bob", "alice", At::default())
            .expect("bob about alice")
            .is_empty()
    );
}

#[test]
fn second_order_claims_need_a_holder_subject_and_a_claim_key() {
    let mut engine = engine();
    expect_code(
        engine.commit(commit(
            "bad-key",
            1,
            vec![holding(
                "x",
                "alice",
                1,
                thinks_holds("bob", "not-a-key", true),
            )],
        )),
        UmweltErrorCode::ValidationFailed,
    );
    expect_code(
        engine.commit(commit(
            "bad-subject",
            1,
            vec![holding(
                "y",
                "alice",
                1,
                thinks_holds("castle", &claim_key(&affair()), true),
            )],
        )),
        UmweltErrorCode::ValidationFailed,
    );
}

#[test]
fn telling_a_secret_outside_its_circle_is_allowed_warned_and_reported() {
    let mut engine = engine();
    engine
        .commit(commit(
            "secret",
            1,
            vec![secret_holding("s", "alice", &["bob"])],
        ))
        .expect("secret");
    let allowed = engine
        .transmit(tell("to-bob", "alice", "bob", "s", 2))
        .expect("bob is in the circle");
    assert!(allowed.warnings.is_empty());
    let breach = engine
        .transmit(tell("to-carol", "alice", "carol", "s", 3))
        .expect("telling is never blocked");
    assert_eq!(breach.warnings.len(), 1);
    assert_eq!(breach.warnings[0].code, "SECRET_BREACH");
    assert!(
        engine
            .holds(
                "p",
                "carol",
                ClaimRef::Claim { claim: affair() },
                At::default()
            )
            .expect("holds")
            .holds
    );

    let breaches = engine.breaches("p", None, At::default()).expect("breaches");
    assert_eq!(breaches.len(), 1);
    let reported = &breaches[0];
    assert_eq!(reported.secret_record_id, "s");
    assert_eq!(reported.owner, "alice");
    assert_eq!(reported.told_by, "alice");
    assert_eq!(reported.recipient, "carol");
    assert_eq!(reported.record_id, "carol-heard");
    assert_eq!(reported.position.tick, 3);
    assert!(
        engine
            .breaches("p", None, at(2))
            .expect("earlier")
            .is_empty()
    );
    assert!(
        engine
            .breaches("p", Some("bob"), At::default())
            .expect("bob")
            .is_empty()
    );
}

#[test]
fn a_secret_told_by_someone_in_the_circle_still_counts_as_a_breach() {
    let mut engine = engine();
    engine
        .commit(commit(
            "secret",
            1,
            vec![secret_holding("s", "alice", &["bob"])],
        ))
        .expect("secret");
    let manual = CommitInput {
        records: vec![RecordInput::Holding {
            common: common("carol-heard", 2),
            holder: "carol".to_owned(),
            attitude: Attitude::Believes,
            claim: affair(),
            acquisition: Acquisition::ToldBy {
                source_holder_id: "bob".to_owned(),
                source_record_id: Some("s".to_owned()),
                intent: Intent::Honest,
            },
            confidence: 1.0,
            accessibility: Accessibility::Available,
            text: None,
            secrecy: None,
        }],
        ..commit("gossip", 2, vec![])
    };
    let outcome = engine.commit(manual).expect("bob gossips");
    assert_eq!(outcome.warnings[0].code, "SECRET_BREACH");
    let breaches = engine
        .breaches("p", Some("alice"), At::default())
        .expect("breaches");
    assert_eq!(breaches[0].told_by, "bob");
    assert_eq!(breaches[0].recipient, "carol");
}
