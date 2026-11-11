// SPDX-License-Identifier: AGPL-3.0-only
//! Context admission, proposal rebase and draft rebase.

mod common;

use common::*;
use umwelt_core::UmweltErrorCode;
use umwelt_core::domain::*;

fn record(id: &str, record_id: &str) -> ContextCandidate {
    ContextCandidate {
        id: id.to_owned(),
        record_id: Some(record_id.to_owned()),
        event_id: None,
        cost: 1,
        priority: 0,
        classification: None,
    }
}

fn event_candidate(id: &str, event_id: &str) -> ContextCandidate {
    ContextCandidate {
        id: id.to_owned(),
        record_id: None,
        event_id: Some(event_id.to_owned()),
        cost: 1,
        priority: 0,
        classification: None,
    }
}

fn admit_input(
    holder: Option<&str>,
    tick: i64,
    candidates: Vec<ContextCandidate>,
) -> ContextAdmitInput {
    ContextAdmitInput {
        project_id: "p".to_owned(),
        timeline_id: None,
        holder: holder.map(str::to_owned),
        at: at(tick),
        candidates,
        budget: None,
        exclude_classifications: Vec::new(),
        include_withheld: false,
        authorized_narrator: false,
    }
}

fn reasons(result: &ContextAdmitResult) -> Vec<(&str, ContextRejection)> {
    result
        .rejected
        .iter()
        .map(|item| (item.id.as_str(), item.reason))
        .collect()
}

fn admitted(result: &ContextAdmitResult) -> Vec<&str> {
    result
        .admitted
        .iter()
        .map(|item| item.id.as_str())
        .collect()
}

fn world() -> TestEngine {
    let mut engine = engine();
    let mut buried = holding("h-buried", "alice", 1, claim("bob", "related", "item"));
    if let RecordInput::Holding { accessibility, .. } = &mut buried {
        *accessibility = Accessibility::Inaccessible;
    }
    engine
        .commit(commit(
            "day1",
            1,
            vec![
                fact("f1", 1, claim("castle", "related", "item")),
                holding("h-alice", "alice", 1, claim("castle", "related", "item")),
                holding("h-bob", "bob", 1, claim("castle", "related", "item")),
                holding("h-gone", "alice", 1, claim("carol", "related", "item")),
                buried,
            ],
        ))
        .expect("day 1");
    engine
        .witness(WitnessInput {
            project_id: "p".to_owned(),
            attempt_id: "fire".to_owned(),
            timeline_id: None,
            expected_version: None,
            event: event("e-fire", 2),
            claims: vec![WitnessedClaim {
                claim: claim("castle", "related", "town"),
                canon: false,
                fact_id: None,
            }],
            witnesses: vec![Witness {
                holder: "alice".to_owned(),
                attitude: Attitude::Knows,
                confidence: 1.0,
                accessibility: Accessibility::Available,
                record_ids: Vec::new(),
            }],
        })
        .expect("alice sees the fire");
    engine
        .commit(commit(
            "other",
            2,
            vec![holding(
                "h-other",
                "bob",
                2,
                claim("bob", "related", "town"),
            )],
        ))
        .expect("another event");
    let mut forget = commit("forget", 3, vec![]);
    forget.transitions = vec![TransitionInput {
        id: Some("t-gone".to_owned()),
        target_record_id: "h-gone".to_owned(),
        status: TransitionStatus::Forgotten,
        superseded_by: None,
        position: Some(Position { tick: 3 }),
        provenance: Some(provenance()),
    }];
    engine.commit(forget).expect("forget");
    engine
        .commit(commit(
            "later",
            9,
            vec![holding(
                "h-later",
                "alice",
                9,
                claim("bob", "related", "castle"),
            )],
        ))
        .expect("later");
    engine
}

#[test]
fn the_narrator_sees_everything_visible_at_the_position() {
    let engine = world();
    let result = engine
        .context_admit(&admit_input(
            None,
            4,
            vec![
                record("canon", "f1"),
                record("bob", "h-bob"),
                event_candidate("fire", "e-fire"),
                record("missing", "nope"),
                record("future", "h-later"),
                record("forgotten", "h-gone"),
                ContextCandidate {
                    event_id: Some("e-fire".to_owned()),
                    ..record("both", "f1")
                },
            ],
        ))
        .expect("admit");
    assert_eq!(admitted(&result), ["canon", "bob", "fire"]);
    assert_eq!(
        reasons(&result),
        [
            ("missing", ContextRejection::NotInProject),
            ("future", ContextRejection::Future),
            ("forgotten", ContextRejection::Inactive),
            ("both", ContextRejection::InvalidCandidate),
        ]
    );
}

#[test]
fn a_holder_perspective_admits_only_what_the_holder_holds_or_witnessed() {
    let engine = world();
    let result = engine
        .context_admit(&admit_input(
            Some("alice"),
            4,
            vec![
                record("own", "h-alice"),
                record("canon", "f1"),
                record("other-mind", "h-bob"),
                record("buried", "h-buried"),
                event_candidate("seen", "e-fire"),
                event_candidate("unseen", "e-other"),
            ],
        ))
        .expect("admit");
    assert_eq!(admitted(&result), ["own", "seen"]);
    assert_eq!(
        reasons(&result),
        [
            ("canon", ContextRejection::NotHeld),
            ("other-mind", ContextRejection::NotHeld),
            ("buried", ContextRejection::Withheld),
            ("unseen", ContextRejection::NotHeld),
        ]
    );
}

#[test]
fn withheld_content_needs_an_authorized_narrator() {
    let engine = world();
    let mut input = admit_input(Some("alice"), 4, vec![record("buried", "h-buried")]);
    input.include_withheld = true;
    expect_code(
        engine.context_admit(&input),
        UmweltErrorCode::ValidationFailed,
    );
    input.authorized_narrator = true;
    let result = engine.context_admit(&input).expect("authorized");
    assert_eq!(admitted(&result), ["buried"]);
}

#[test]
fn classification_duplicates_and_budget_are_checked_last_in_order() {
    let engine = world();
    let mut input = admit_input(
        Some("alice"),
        4,
        vec![
            ContextCandidate {
                cost: 2,
                priority: 1,
                ..record("a", "h-alice")
            },
            ContextCandidate {
                cost: 2,
                priority: 5,
                ..event_candidate("b", "e-fire")
            },
            ContextCandidate {
                classification: Some("spoiler".to_owned()),
                ..record("secret", "h-alice")
            },
            record("again", "h-alice"),
            ContextCandidate {
                cost: 1,
                priority: 0,
                ..record("c", "h-buried")
            },
        ],
    );
    input.exclude_classifications = vec!["spoiler".to_owned()];
    input.budget = Some(3);
    input.include_withheld = true;
    input.authorized_narrator = true;
    let result = engine.context_admit(&input).expect("admit");
    assert_eq!(
        admitted(&result),
        ["b", "c"],
        "priority first, then whatever still fits"
    );
    assert_eq!(result.used_budget, 3);
    assert_eq!(
        reasons(&result),
        [
            ("a", ContextRejection::OverBudget),
            ("secret", ContextRejection::ExcludedClassification),
            ("again", ContextRejection::Duplicate),
        ]
    );
}

#[test]
fn context_respects_timelines() {
    let mut engine = world();
    engine
        .open_draft(OpenDraftInput {
            project_id: "p".to_owned(),
            id: Some("d".to_owned()),
            label: None,
            base_timeline_id: None,
        })
        .expect("draft");
    let mut in_draft = commit(
        "draft",
        10,
        vec![holding(
            "h-draft",
            "alice",
            10,
            claim("carol", "related", "town"),
        )],
    );
    in_draft.timeline_id = Some("d".to_owned());
    engine.commit(in_draft).expect("draft commit");
    let mut input = admit_input(Some("alice"), 10, vec![record("draft", "h-draft")]);
    let main = engine.context_admit(&input).expect("main");
    assert_eq!(reasons(&main), [("draft", ContextRejection::NotInProject)]);
    input.timeline_id = Some("d".to_owned());
    let draft = engine.context_admit(&input).expect("draft");
    assert_eq!(admitted(&draft), ["draft"]);
}

fn propose(engine: &mut TestEngine) -> Proposal {
    engine
        .propose(ProposeInput {
            project_id: "p".to_owned(),
            timeline_id: None,
            expected_version: Some(engine.project("p").expect("project").version),
            event: event("e-proposal", 4),
            records: vec![
                holding("p-new", "alice", 4, claim("carol", "related", "castle")),
                holding("taken", "bob", 4, claim("carol", "related", "castle")),
            ],
            transitions: vec![TransitionInput {
                id: Some("p-forget".to_owned()),
                target_record_id: "p-new".to_owned(),
                status: TransitionStatus::Forgotten,
                superseded_by: None,
                position: Some(Position { tick: 5 }),
                provenance: Some(provenance()),
            }],
        })
        .expect("propose")
}

#[test]
fn proposal_rebase_reports_which_operations_still_fit_the_new_head() {
    let mut engine = world();
    let proposal = propose(&mut engine);
    engine
        .commit(commit(
            "meanwhile",
            4,
            vec![holding(
                "taken",
                "carol",
                4,
                claim("bob", "related", "town"),
            )],
        ))
        .expect("main moves and takes the id");
    let head = engine.project("p").expect("project").version;
    let before = engine.counts("p").expect("counts");
    let result = engine.rebase_proposal(&proposal.id).expect("rebase");
    assert_eq!(result.head_version, head);
    assert_eq!(result.proposal.input.expected_version, Some(head));
    let checks = result
        .operations
        .iter()
        .map(|check| (check.operation_index, check.valid))
        .collect::<Vec<_>>();
    assert_eq!(checks, [(0, true), (1, false), (2, true)]);
    assert_eq!(
        result.operations[1].error.as_ref().expect("error").code,
        "CONFLICT"
    );
    assert_eq!(
        engine.counts("p").expect("counts"),
        before,
        "nothing was applied"
    );

    engine
        .review(
            &proposal.id,
            vec![0, 2]
                .into_iter()
                .map(|operation_index| ReviewInput {
                    operation_index,
                    decision: ReviewDecision::Accepted,
                    rationale: "still fine".to_owned(),
                })
                .collect(),
        )
        .expect("review");
    engine
        .apply(&proposal.id, "apply".to_owned())
        .expect("the rebased proposal applies at the new head");
}

fn open(engine: &mut TestEngine, id: &str) {
    engine
        .open_draft(OpenDraftInput {
            project_id: "p".to_owned(),
            id: Some(id.to_owned()),
            label: None,
            base_timeline_id: None,
        })
        .expect("open draft");
}

fn gossip(attempt: &str) -> TransmitInput {
    TransmitInput {
        on: Default::default(),
        project_id: "p".to_owned(),
        attempt_id: attempt.to_owned(),
        timeline_id: Some("d".to_owned()),
        position: Position { tick: 10 },
        from: "alice".to_owned(),
        to: "carol".to_owned(),
        source_record_id: "h-alice".to_owned(),
        record_id: Some("carol-heard".to_owned()),
        attitude: Attitude::Believes,
        confidence: 1.0,
        intent: Intent::Honest,
        distorted_claim: None,
        text: None,
    }
}

#[test]
fn draft_rebase_replays_the_draft_on_the_new_main() {
    let mut engine = world();
    open(&mut engine, "d");
    let first = engine.transmit(gossip("tell")).expect("draft gossip");
    engine
        .commit(commit(
            "main-moves",
            10,
            vec![fact("f-new", 10, claim("town", "related", "item"))],
        ))
        .expect("main moves");
    expect_code(
        engine.promote_draft("p", "d"),
        UmweltErrorCode::VersionConflict,
    );

    let rebased = engine.rebase_draft("p", "d").expect("rebase");
    let head = engine.project("p").expect("project").version;
    assert_eq!(rebased.base_version, Some(head));
    assert!(rebased.head_version > head);
    let heard = engine
        .in_timeline("d")
        .record("p", "carol-heard")
        .expect("same ids after rebase");
    assert!(matches!(heard, Record::Holding { holder, .. } if holder == "carol"));
    assert!(
        engine.in_timeline("d").record("p", "f-new").is_ok(),
        "the draft now sees main's commit"
    );

    let counts = engine.counts("p").expect("counts");
    let replay = engine
        .transmit(gossip("tell"))
        .expect("replay after rebase");
    assert_eq!(replay.record_ids, first.record_ids);
    assert_eq!(engine.counts("p").expect("counts"), counts);

    engine
        .promote_draft("p", "d")
        .expect("promote after rebase");
    assert!(
        engine
            .holds(
                "p",
                "carol",
                ClaimRef::Claim {
                    claim: claim("castle", "related", "item")
                },
                At::default()
            )
            .expect("holds")
            .holds
    );
    let feed = engine.changes("p", 0, 1000).expect("changes").items;
    assert!(
        feed.iter()
            .any(|item| item.change_type == "timeline.rebased")
    );
}

#[test]
fn a_draft_that_no_longer_fits_is_left_unchanged() {
    let mut engine = world();
    open(&mut engine, "d");
    engine.transmit(gossip("tell")).expect("draft gossip");
    let before = engine.get_timeline("p", "d").expect("draft");
    let mut forget_source = commit("forget-source", 6, vec![]);
    forget_source.transitions = vec![TransitionInput {
        id: Some("t-source".to_owned()),
        target_record_id: "h-alice".to_owned(),
        status: TransitionStatus::Forgotten,
        superseded_by: None,
        position: Some(Position { tick: 6 }),
        provenance: Some(provenance()),
    }];
    engine
        .commit(forget_source)
        .expect("main: alice forgot before telling");

    let error = expect_code(
        engine.rebase_draft("p", "d"),
        UmweltErrorCode::ValidationFailed,
    );
    let details = error.details.expect("details");
    assert_eq!(details["failedAttemptId"], "tell");
    assert_eq!(details["timelineId"], "d");
    assert_eq!(engine.get_timeline("p", "d").expect("draft"), before);
    assert!(engine.in_timeline("d").record("p", "carol-heard").is_ok());
}

#[test]
fn rebasing_an_up_to_date_or_closed_draft() {
    let mut engine = world();
    open(&mut engine, "d");
    let current = engine.get_timeline("p", "d").expect("draft");
    assert_eq!(engine.rebase_draft("p", "d").expect("no-op"), current);
    engine.discard_draft("p", "d", false).expect("discard");
    expect_code(engine.rebase_draft("p", "d"), UmweltErrorCode::InvalidState);
    expect_code(
        engine.rebase_draft("p", "main"),
        UmweltErrorCode::InvalidState,
    );
}
