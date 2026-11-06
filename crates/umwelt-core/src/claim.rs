// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::BTreeMap;

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::domain::{Claim, CommitInput};

pub fn claim_key(claim: &Claim) -> String {
    let mut value = Map::new();
    value.insert("subject".to_owned(), Value::String(claim.subject.clone()));
    value.insert(
        "predicate".to_owned(),
        Value::String(claim.predicate.clone()),
    );
    value.insert(
        "object".to_owned(),
        serde_json::to_value(&claim.object).unwrap_or(Value::Null),
    );
    value.insert(
        "qualifiers".to_owned(),
        Value::Object(
            claim
                .qualifiers
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        ),
    );
    value.insert(
        "about".to_owned(),
        claim.about.clone().map_or(Value::Null, Value::String),
    );
    hash_canonical_json(&Value::Object(value))
}

pub fn slot_key(claim: &Claim) -> String {
    let value = Value::Object(
        [
            ("subject".to_owned(), Value::String(claim.subject.clone())),
            (
                "predicate".to_owned(),
                Value::String(claim.predicate.clone()),
            ),
        ]
        .into_iter()
        .collect(),
    );
    hash_canonical_json(&value)
}

/// Fingerprint of a commit request. `timelineId` is left out: an attempt is the
/// same request in whichever visible timeline it is replayed.
pub fn commit_fingerprint(input: &CommitInput) -> crate::Result<String> {
    let mut value = serde_json::to_value(input)?;
    if let Value::Object(fields) = &mut value {
        fields.remove("timelineId");
    }
    Ok(hash_canonical_json(&value))
}

/// Fingerprint of a request that the engine turns into a commit (`witness`,
/// batch maintenance). It hashes the request itself, not the derived commit,
/// whose generated ids would differ between retries. `timelineId` is left out
/// for the same reason as in [`commit_fingerprint`].
pub fn request_fingerprint<T: serde::Serialize>(
    operation: &str,
    request: &T,
) -> crate::Result<String> {
    let mut value = serde_json::to_value(request)?;
    if let Value::Object(fields) = &mut value {
        fields.remove("timelineId");
    }
    let mut wrapper = Map::new();
    wrapper.insert("operation".to_owned(), Value::String(operation.to_owned()));
    wrapper.insert("request".to_owned(), value);
    Ok(hash_canonical_json(&Value::Object(wrapper)))
}

pub fn canonical_json(value: &Value) -> String {
    let mut output = String::new();
    write_canonical(value, &mut output);
    output
}

fn hash_canonical_json(value: &Value) -> String {
    let digest = Sha256::digest(canonical_json(value).as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn write_canonical(value: &Value, output: &mut String) {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) => output.push_str(&value.to_string()),
        Value::String(value) => write_string(value, output),
        Value::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_canonical(value, output);
            }
            output.push(']');
        }
        Value::Object(values) => {
            output.push('{');
            let ordered: BTreeMap<&str, &Value> = values
                .iter()
                .map(|(key, value)| (key.as_str(), value))
                .collect();
            for (index, (key, value)) in ordered.into_iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_string(key, output);
                output.push(':');
                write_canonical(value, output);
            }
            output.push('}');
        }
    }
}

fn write_string(value: &str, output: &mut String) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character <= '\u{1f}' => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
}
