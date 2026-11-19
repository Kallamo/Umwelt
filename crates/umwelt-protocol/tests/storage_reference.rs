// SPDX-License-Identifier: AGPL-3.0-only
//! The reference file of every published storage version
//! (`fixtures/storage/v<N>.umw`) opens with this engine, migrated if it is
//! older, and answers the reads recorded by the release that wrote it.
use std::{fs, path::Path, sync::Arc};

use serde_json::{Value, json};
use umwelt_api::{Engine, FixedClock, SeqIdGen, SqliteStore};
use umwelt_protocol::Dispatcher;

/// Every recorded field is present and equal. Arrays keep their length and
/// order; a release may add optional fields to a result, never remove one.
fn same(expected: &Value, actual: &Value, path: &str) {
    match expected {
        Value::Object(fields) => {
            let object = actual
                .as_object()
                .unwrap_or_else(|| panic!("{path}: expected an object, got {actual}"));
            for (key, value) in fields {
                let inner = object
                    .get(key)
                    .unwrap_or_else(|| panic!("{path}.{key} is missing"));
                same(value, inner, &format!("{path}.{key}"));
            }
        }
        Value::Array(items) => {
            let array = actual
                .as_array()
                .unwrap_or_else(|| panic!("{path}: expected an array, got {actual}"));
            assert_eq!(items.len(), array.len(), "{path}: array length");
            for (index, (expected, actual)) in items.iter().zip(array).enumerate() {
                same(expected, actual, &format!("{path}[{index}]"));
            }
        }
        // The recorded file was written by another JSON library, and serde_json
        // may parse a float one unit in the last place away from it.
        Value::Number(number) => {
            let (expected, actual) = (
                number.as_f64().expect("number"),
                actual
                    .as_f64()
                    .unwrap_or_else(|| panic!("{path}: expected a number, got {actual}")),
            );
            assert!(
                (expected - actual).abs() <= 1e-12 * expected.abs().max(1.0),
                "{path}: {expected} != {actual}"
            );
        }
        _ => assert_eq!(expected, actual, "{path}"),
    }
}

fn references() -> Vec<(String, std::path::PathBuf)> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/storage");
    let mut files = fs::read_dir(&directory)
        .expect("fixtures/storage")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "umw"))
        .map(|path| {
            let stem = path
                .file_stem()
                .expect("stem")
                .to_string_lossy()
                .into_owned();
            (stem, path)
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

/// Sends every recorded request and compares the answer with the recorded one.
fn answers(dispatcher: &mut Dispatcher<SqliteStore>, stem: &str, kind: &str, recorded: &Value) {
    for (index, item) in recorded.as_array().expect(kind).iter().enumerate() {
        let label = format!("{stem} {kind} {index} ({})", item["op"]);
        let request =
            json!({"v":1,"id":format!("{kind}-{index}"),"op":item["op"],"payload":item["payload"]});
        let response =
            serde_json::to_value(dispatcher.handle_line(&request.to_string())).expect("response");
        assert_eq!(response["ok"], true, "{label}: {response}");
        same(&item["result"], &response["result"], &label);
    }
}

#[test]
fn every_reference_file_answers_its_recorded_reads() {
    let files = references();
    assert!(!files.is_empty(), "no reference files in fixtures/storage");
    for (stem, path) in files {
        let expected: Value = serde_json::from_str(
            &fs::read_to_string(path.with_file_name(format!("{stem}.expected.json")))
                .expect("expected answers"),
        )
        .expect("expected JSON");
        let directory = tempfile::tempdir().expect("temporary directory");
        let copy = directory.path().join("reference.umw");
        fs::copy(&path, &copy).expect("copy the reference file");
        let engine = Engine::new(
            SqliteStore::open(&copy).unwrap_or_else(|error| panic!("{stem}: {error}")),
            Arc::new(FixedClock("2026-01-01T00:00:00Z".to_owned())),
            Arc::new(SeqIdGen::new()),
        );
        let mut dispatcher = Dispatcher::new(engine);
        let hello = json!({"v":1,"id":"hello","op":"hello","payload":{"protocol":{"min":1,"max":1},"client":"storage-reference"}});
        let response =
            serde_json::to_value(dispatcher.handle_line(&hello.to_string())).expect("hello");
        assert_eq!(response["ok"], true, "{stem}: {response}");
        let storage = expected["storage"].as_u64().expect("storage version");
        if storage < u64::from(umwelt_api::STORAGE_VERSION) {
            let migration = &response["result"]["migration"];
            assert_eq!(migration["from"], storage, "{stem}: {response}");
            assert_eq!(migration["to"], umwelt_api::STORAGE_VERSION);
            let backup = migration["backup"].as_str().expect("backup path");
            assert_eq!(
                fs::read(backup).expect("backup"),
                fs::read(&path).expect("reference")
            );
        } else {
            assert!(
                response["result"].get("migration").is_none(),
                "{stem}: {response}"
            );
        }
        answers(&mut dispatcher, &stem, "read", &expected["reads"]);
        // A retry of a write stored by that release replays its outcome: this
        // engine computes the same fingerprint for it.
        answers(&mut dispatcher, &stem, "retry", &expected["retries"]);
    }
}
