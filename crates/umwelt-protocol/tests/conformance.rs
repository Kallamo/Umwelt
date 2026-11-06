// SPDX-License-Identifier: AGPL-3.0-only
use std::{collections::BTreeMap, fs, path::Path, sync::Arc};

use serde_json::{Value, json};
use umwelt_api::{Engine, FixedClock, SeqIdGen, SqliteStore};
use umwelt_protocol::Dispatcher;

fn resolve(value: &Value, captures: &BTreeMap<String, Value>) -> Value {
    match value {
        Value::String(text) if text.starts_with("$ref:") => captures
            .get(&text[5..])
            .expect("captured reference")
            .clone(),
        Value::Array(items) => Value::Array(items.iter().map(|v| resolve(v, captures)).collect()),
        Value::Object(items) => Value::Object(
            items
                .iter()
                .map(|(k, v)| (k.clone(), resolve(v, captures)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn subset(expected: &Value, actual: &Value, captures: &mut BTreeMap<String, Value>) {
    if let Some(text) = expected.as_str() {
        if text == "$any" {
            return;
        }
        if let Some(name) = text.strip_prefix("$capture:") {
            captures.insert(name.to_owned(), actual.clone());
            return;
        }
        if let Some(name) = text.strip_prefix("$ref:") {
            assert_eq!(captures.get(name).expect("captured value"), actual);
            return;
        }
    }
    match expected {
        Value::Object(fields) => {
            let object = actual.as_object().expect("expected object");
            for (key, value) in fields {
                subset(
                    value,
                    object
                        .get(key)
                        .unwrap_or_else(|| panic!("Missing {key} in {actual}")),
                    captures,
                );
            }
        }
        Value::Array(items) => {
            let array = actual.as_array().expect("expected array");
            assert_eq!(items.len(), array.len(), "array length: {actual}");
            for (expected, actual) in items.iter().zip(array) {
                subset(expected, actual, captures);
            }
        }
        _ => assert_eq!(expected, actual),
    }
}

#[test]
fn shared_language_agnostic_cases() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases");
    let mut paths = fs::read_dir(directory)
        .expect("case directory")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();
    assert!(paths.len() >= 30);
    for path in paths {
        let case: Value = serde_json::from_str(&fs::read_to_string(&path).expect("case file"))
            .expect("case JSON");
        let engine = Engine::new(
            SqliteStore::in_memory().expect("store"),
            Arc::new(FixedClock("2026-01-01T00:00:00Z".to_owned())),
            Arc::new(SeqIdGen::new()),
        );
        let mut dispatcher = Dispatcher::new(engine);
        let mut captures = BTreeMap::new();
        for (index, step) in case["steps"].as_array().expect("steps").iter().enumerate() {
            let request = resolve(&step["request"], &captures);
            let response = serde_json::to_value(dispatcher.handle_line(&json!({"v":1,"id":format!("case-{index}"),"op":request["op"],"payload":request["payload"]}).to_string())).expect("response");
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                subset(&step["expect"], &response, &mut captures)
            }));
            assert!(
                result.is_ok(),
                "{} step {}: {}",
                path.display(),
                index,
                response
            );
        }
    }
}
