// SPDX-License-Identifier: AGPL-3.0-only

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

fn exchange(input: &mut impl Write, output: &mut impl BufRead, request: &Value) -> Value {
    writeln!(input, "{request}").expect("request should be written");
    input.flush().expect("request should be flushed");
    let mut line = String::new();
    output
        .read_line(&mut line)
        .expect("response should be read");
    serde_json::from_str(&line).expect("response should be JSON")
}

#[test]
fn serve_runs_the_v1_protocol_and_survives_bad_input() {
    let directory = tempfile::tempdir().expect("temporary directory should be created");
    let path = directory.path().join("protocol.umw");
    let mut child = Command::new(env!("CARGO_BIN_EXE_umwelt"))
        .args([
            "serve",
            path.to_str().expect("path should be UTF-8"),
            "--create",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("server should start");
    let mut input = child.stdin.take().expect("stdin should be piped");
    let mut output = BufReader::new(child.stdout.take().expect("stdout should be piped"));

    let before = exchange(
        &mut input,
        &mut output,
        &json!({"v":1,"id":"before","op":"project.list","payload":{}}),
    );
    assert_eq!(before["error"]["code"], "HANDSHAKE_REQUIRED");
    let hello = exchange(
        &mut input,
        &mut output,
        &json!({"v":1,"id":"hello","op":"hello","payload":{"protocol":{"min":1,"max":1},"client":"integration-test/1"}}),
    );
    assert_eq!(hello["result"]["protocol"], 1);
    assert_eq!(hello["result"]["contract"], env!("CARGO_PKG_VERSION"));

    let controlled = exchange(
        &mut input,
        &mut output,
        &json!({"v":1,"id":"controlled","op":"project.create","payload":{"name":"Wrong","timescale":"turn","version":57}}),
    );
    assert_eq!(controlled["error"]["code"], "VALIDATION_FAILED");
    let generated = exchange(
        &mut input,
        &mut output,
        &json!({"v":1,"id":"generated","op":"project.create","payload":{"name":"Generated","timescale":"turn"}}),
    );
    assert_eq!(generated["result"]["version"], 0);
    assert!(
        generated["result"]["id"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert!(
        generated["result"]["createdAt"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert_eq!(generated["result"]["vocabularyMode"], "strict");

    let project = json!({"id":"p","name":"Protocol","timescale":"turn","vocabularyMode":"open"});
    assert_eq!(
        exchange(
            &mut input,
            &mut output,
            &json!({"v":1,"id":"project","op":"project.create","payload":project})
        )["ok"],
        true
    );
    let entity_type =
        json!({"id":"person","projectId":"p","name":"Person","kind":"agent","fields":[]});
    assert_eq!(
        exchange(
            &mut input,
            &mut output,
            &json!({"v":1,"id":"type","op":"entityType.create","payload":entity_type})
        )["ok"],
        true
    );
    let entity = json!({"id":"player","projectId":"p","typeId":"person","name":"Player"});
    assert_eq!(
        exchange(
            &mut input,
            &mut output,
            &json!({"v":1,"id":"entity","op":"entity.create","payload":entity})
        )["ok"],
        true
    );
    let forbidden_update = exchange(
        &mut input,
        &mut output,
        &json!({"v":1,"id":"update-forbidden","op":"entity.update","payload":{"projectId":"p","entityId":"player","typeId":"other","archived":true}}),
    );
    assert_eq!(forbidden_update["error"]["code"], "VALIDATION_FAILED");
    let invalid_fields = exchange(
        &mut input,
        &mut output,
        &json!({"v":1,"id":"update-fields","op":"entity.update","payload":{"projectId":"p","entityId":"player","fields":{"unknown":true}}}),
    );
    assert_eq!(invalid_fields["error"]["code"], "VALIDATION_FAILED");
    let updated = exchange(
        &mut input,
        &mut output,
        &json!({"v":1,"id":"update","op":"entity.update","payload":{"projectId":"p","entityId":"player","name":"Renamed","aliases":["Hero"]}}),
    );
    assert_eq!(updated["result"]["name"], "Renamed");
    assert_eq!(updated["result"]["typeId"], "person");
    assert_eq!(updated["result"]["archived"], false);

    let reserved = exchange(
        &mut input,
        &mut output,
        &json!({"v":1,"id":"reserved","op":"predicate.define","payload":{"projectId":"p","predicate":{"id":"umwelt:x","label":"Reserved","object":"none","cardinality":"many"}}}),
    );
    assert_eq!(reserved["error"]["code"], "VALIDATION_FAILED");
    let predicate = json!({"projectId":"p","predicate":{"id":"arrived","label":"Arrived","object":"none","cardinality":"many"}});
    assert_eq!(
        exchange(
            &mut input,
            &mut output,
            &json!({"v":1,"id":"predicate","op":"predicate.define","payload":predicate})
        )["ok"],
        true
    );

    let provenance = json!({"origin":{"kind":"system","name":"test"}});
    let commit = json!({
        "projectId":"p","attemptId":"attempt-1",
        "event":{"id":"event-1","type":"test","title":"Arrival","position":{"tick":1},"participants":[],"evidence":[],"provenance":provenance},
        "records":[{"kind":"fact","id":"record-1","position":{"tick":1},"provenance":provenance,"claim":{"subject":"player","predicate":"arrived","object":{"kind":"none"},"qualifiers":{},"polarity":true}}],
        "transitions":[]
    });
    let first = exchange(
        &mut input,
        &mut output,
        &json!({"v":1,"id":"commit-1","op":"commit","payload":commit}),
    );
    let replay = exchange(
        &mut input,
        &mut output,
        &json!({"v":1,"id":"commit-2","op":"commit","payload":commit}),
    );
    assert_eq!(first["result"], replay["result"]);

    let proposal = exchange(
        &mut input,
        &mut output,
        &json!({"v":1,"id":"propose","op":"propose","payload":{
            "projectId":"p",
            "event":{"id":"proposal-event","type":"test","title":"Proposal","position":{"tick":2},"participants":[],"evidence":[],"provenance":provenance},
            "records":[{"kind":"fact","id":"proposal-record","position":{"tick":2},"provenance":provenance,"claim":{"subject":"player","predicate":"arrived","object":{"kind":"none"},"qualifiers":{},"polarity":true}}],
            "transitions":[]
        }}),
    );
    let proposal_id = proposal["result"]["id"]
        .as_str()
        .expect("proposal id should exist");
    let unapplied = exchange(
        &mut input,
        &mut output,
        &json!({"v":1,"id":"apply-empty","op":"proposal.apply","payload":{"proposalId":proposal_id,"attemptId":"empty-apply"}}),
    );
    assert_eq!(unapplied["error"]["code"], "INVALID_STATE");

    writeln!(input, "{{\"v\":1,\"id\":\"broken\",\"op\":")
        .expect("invalid request should be written");
    input.flush().expect("invalid request should be flushed");
    let mut line = String::new();
    output
        .read_line(&mut line)
        .expect("invalid response should be read");
    let invalid: Value = serde_json::from_str(&line).expect("invalid response should be JSON");
    assert_eq!(invalid["id"], "broken");
    assert_eq!(invalid["error"]["code"], "VALIDATION_FAILED");

    let giant = "x".repeat(16 * 1024 * 1024 + 1);
    writeln!(input, "{giant}").expect("giant request should be written");
    input.flush().expect("giant request should be flushed");
    line.clear();
    output
        .read_line(&mut line)
        .expect("giant response should be read");
    let oversized: Value = serde_json::from_str(&line).expect("giant response should be JSON");
    assert_eq!(oversized["error"]["code"], "PAYLOAD_TOO_LARGE");
    let next = exchange(
        &mut input,
        &mut output,
        &json!({"v":1,"id":"next","op":"project.get","payload":{"projectId":"p"}}),
    );
    assert_eq!(next["result"]["id"], "p");

    let shutdown = exchange(
        &mut input,
        &mut output,
        &json!({"v":1,"id":"shutdown","op":"shutdown","payload":{}}),
    );
    assert_eq!(shutdown["ok"], true);
    drop(input);
    let status = child.wait().expect("server should exit");
    assert_eq!(status.code(), Some(0));

    let call = Command::new(env!("CARGO_BIN_EXE_umwelt"))
        .args([
            "call",
            path.to_str().expect("path should be UTF-8"),
            "invalid.op",
            "{}",
        ])
        .output()
        .expect("call should run");
    assert_eq!(call.status.code(), Some(1));
    let response: Value =
        serde_json::from_slice(&call.stdout).expect("call response should be JSON");
    assert_eq!(response["error"]["code"], "UNKNOWN_OP");
}
