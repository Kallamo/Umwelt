// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::BTreeMap;

use serde_json::json;
use umwelt_core::claim_key;
use umwelt_core::domain::{Claim, ClaimObject};

const EXPECTED_CLAIM_KEY: &str = "a0a8d5482c1eb719682e2cf8ef931adb41673a7d6ce1c509a625e93cfa1ed391";

fn example_claim() -> Claim {
    Claim {
        subject: "player".to_owned(),
        predicate: "gave-gift".to_owned(),
        object: ClaimObject::Entity {
            id: "mira".to_owned(),
        },
        qualifiers: BTreeMap::from([("item".to_owned(), json!("lantern"))]),
        polarity: true,
        about: None,
    }
}

#[test]
fn claim_key_matches_the_fixed_contract_vector() {
    assert_eq!(claim_key(&example_claim()), EXPECTED_CLAIM_KEY);
}

#[test]
fn claim_key_ignores_polarity_and_qualifier_insertion_order() {
    let original = Claim {
        qualifiers: BTreeMap::from([
            ("season".to_owned(), json!("summer")),
            ("item".to_owned(), json!("lantern")),
        ]),
        ..example_claim()
    };
    let reordered = Claim {
        qualifiers: [
            ("item".to_owned(), json!("lantern")),
            ("season".to_owned(), json!("summer")),
        ]
        .into_iter()
        .collect(),
        polarity: false,
        ..example_claim()
    };
    assert_eq!(claim_key(&original), claim_key(&reordered));
}
