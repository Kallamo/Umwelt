// SPDX-License-Identifier: AGPL-3.0-only
//! Batch witnesses and group queries.

mod common;

use common::*;
use umwelt_core::UmweltErrorCode;
use umwelt_core::domain::*;

fn witness(holder: &str) -> Witness {
    Witness {
        holder: holder.to_owned(),
        attitude: Attitude::Knows,
        confidence: 1.0,
        accessibility: Accessibility::Available,
        record_ids: Vec::new(),
    }
}

fn scene(attempt: &str, witnesses: Vec<Witness>, canon: bool) -> WitnessInput {
    WitnessInput {
        project_id: "p".to_owned(),
        attempt_id: attempt.to_owned(),
        timeline_id: None,
        expected_version: None,
        event: EventInput {
            id: None,
            ..event("unused", 3)
        },
        claims: vec![
            WitnessedClaim {
                claim: claim("castle", "related", "item"),
                canon,
                fact_id: canon.then(|| "fire".to_owned()),
            },
            WitnessedClaim {
                claim: claim("carol", "location", "castle"),
                canon: false,
                fact_id: None,
            },
        ],
        witnesses,
    }
}

fn holds(engine: &TestEngine, holder: &str, value: Claim) -> HoldsResult {
    engine
        .holds("p", holder, ClaimRef::Claim { claim: value }, At::default())
        .expect("holds")
}

#[test]
fn every_witness_gets_a_witnessed_holding_for_every_claim() {
    let mut engine = engine();
    let mut dim = witness("bob");
    dim.confidence = 0.4;
    dim.attitude = Attitude::Suspects;
    let outcome = engine
        .witness(scene("fire", vec![witness("alice"), dim], true))
        .expect("witness");
    assert_eq!(
        outcome.record_ids.len(),
        5,
        "one fact plus 2 witnesses x 2 claims"
    );
    assert_eq!(outcome.record_ids[0], "fire");
    for holder in ["alice", "bob"] {
        assert!(holds(&engine, holder, claim("castle", "related", "item")).holds);
        assert!(holds(&engine, holder, claim("carol", "location", "castle")).holds);
    }
    assert!(!holds(&engine, "carol", claim("castle", "related", "item")).holds);
    let bob = holds(&engine, "bob", claim("castle", "related", "item"));
    assert_eq!(bob.confidence, Some(0.4));
    assert_eq!(bob.attitude, Some(Attitude::Suspects));
    let record = engine
        .record("p", bob.record_id.as_deref().expect("record id"))
        .expect("record");
    assert!(matches!(
        record,
        Record::Holding { acquisition: Acquisition::Witnessed { event_id }, .. }
            if event_id == outcome.event_id
    ));
    let canon = engine
        .canon("p", CanonFilter::default(), At::default(), Page::default())
        .expect("canon");
    assert_eq!(canon.items.len(), 1);
}

#[test]
fn replaying_a_witness_request_is_idempotent_even_with_generated_ids() {
    let mut engine = engine();
    let first = engine
        .witness(scene("fire", vec![witness("alice"), witness("bob")], false))
        .expect("first");
    let counts = engine.counts("p").expect("counts");
    let replay = engine
        .witness(scene("fire", vec![witness("alice"), witness("bob")], false))
        .expect("replay");
    assert_eq!(first, replay);
    assert_eq!(engine.counts("p").expect("counts after"), counts);
    expect_code(
        engine.witness(scene("fire", vec![witness("alice")], false)),
        UmweltErrorCode::Conflict,
    );
}

#[test]
fn withheld_witnesses_hold_nothing_visible() {
    let mut engine = engine();
    let mut blurred = witness("bob");
    blurred.accessibility = Accessibility::Suppressed;
    engine
        .witness(scene("fire", vec![blurred], false))
        .expect("witness");
    assert!(!holds(&engine, "bob", claim("castle", "related", "item")).holds);
    let holdings = engine
        .holdings(
            "p",
            "bob",
            HoldingFilter::default(),
            At::default(),
            Page::default(),
        )
        .expect("holdings");
    assert_eq!(holdings.withheld_count, 2);
}

#[test]
fn invalid_witness_requests_write_nothing() {
    let mut engine = engine();
    let before = engine.counts("p").expect("counts");
    expect_code(
        engine.witness(scene("a", vec![witness("alice"), witness("alice")], false)),
        UmweltErrorCode::ValidationFailed,
    );
    let mut short = witness("alice");
    short.record_ids = vec!["only-one".to_owned()];
    expect_code(
        engine.witness(scene("b", vec![short], false)),
        UmweltErrorCode::ValidationFailed,
    );
    expect_code(
        engine.witness(scene("c", vec![witness("alice"), witness("item")], false)),
        UmweltErrorCode::ValidationFailed,
    );
    assert_eq!(engine.counts("p").expect("counts"), before);
}

fn membership(id: &str, member: &str, tick: i64) -> RecordInput {
    fact(id, tick, claim(member, MEMBER_OF_PREDICATE_ID, "guild"))
}

fn guild_with_rumor(engine: &mut TestEngine) {
    let rumor = claim("castle", "related", "item");
    engine
        .commit(commit(
            "members",
            1,
            vec![
                membership("m-alice", "alice", 1),
                membership("m-bob", "bob", 1),
            ],
        ))
        .expect("members");
    engine
        .commit(commit(
            "rumor",
            2,
            vec![
                holding("h-alice", "alice", 2, rumor.clone()),
                holding("h-carol", "carol", 2, rumor),
            ],
        ))
        .expect("rumor");
}

#[test]
fn who_holds_can_be_narrowed_to_current_group_members() {
    let mut engine = engine();
    guild_with_rumor(&mut engine);
    let members_only = engine
        .who_holds(
            "p",
            ClaimRef::Claim {
                claim: claim("castle", "related", "item"),
            },
            At::default(),
            WhoHoldsFilter {
                member_of: Some("guild".to_owned()),
                ..WhoHoldsFilter::default()
            },
        )
        .expect("who holds");
    assert_eq!(
        members_only
            .iter()
            .map(|item| item.holder.as_str())
            .collect::<Vec<_>>(),
        ["alice"]
    );
}

#[test]
fn group_coverage_reports_members_with_and_without_the_claim() {
    let mut engine = engine();
    guild_with_rumor(&mut engine);
    let rumor = ClaimRef::Claim {
        claim: claim("castle", "related", "item"),
    };
    let coverage = engine
        .group_coverage("p", "guild", rumor.clone(), At::default())
        .expect("coverage");
    assert_eq!(coverage.members, ["alice", "bob"]);
    assert_eq!(coverage.holding.len(), 1);
    assert_eq!(coverage.missing, ["bob"]);
    assert!(
        coverage.group_holding.is_none(),
        "members knowing is not the group knowing"
    );

    let mut leave = commit("leave", 5, vec![]);
    leave.transitions = vec![TransitionInput {
        id: Some("t-leave".to_owned()),
        target_record_id: "m-bob".to_owned(),
        status: TransitionStatus::Retracted,
        superseded_by: None,
        position: Some(Position { tick: 5 }),
        provenance: Some(provenance()),
    }];
    engine.commit(leave).expect("bob leaves");
    engine
        .commit(commit(
            "guild-knows",
            5,
            vec![holding(
                "h-guild",
                "guild",
                5,
                claim("castle", "related", "item"),
            )],
        ))
        .expect("guild record");
    let later = engine
        .group_coverage("p", "guild", rumor.clone(), at(6))
        .expect("later coverage");
    assert_eq!(later.members, ["alice"]);
    assert!(later.missing.is_empty());
    assert_eq!(
        later.group_holding.expect("guild holds").record_id,
        "h-guild"
    );
    let earlier = engine
        .group_coverage("p", "guild", rumor, at(2))
        .expect("earlier coverage");
    assert_eq!(earlier.members, ["alice", "bob"]);
}
