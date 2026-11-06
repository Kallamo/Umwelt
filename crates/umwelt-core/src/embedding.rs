// SPDX-License-Identifier: AGPL-3.0-only
//! Searchable text and the pure arithmetic of semantic search: vector
//! validation, similarity and rank fusion. The engine never runs a model;
//! vectors come from the client.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::domain::{Claim, ClaimObject, EmbeddingSpace, Event, Record, SearchMatch};
use crate::{Result, UmweltError};

/// Largest number of dimensions a space may declare.
pub const MAX_EMBEDDING_DIMENSIONS: u32 = 4096;
/// `offset + limit` of a semantic or hybrid search page cannot exceed this.
pub const MAX_SEMANTIC_WINDOW: u64 = 1000;
/// The `k` of Reciprocal Rank Fusion.
pub const RRF_K: f64 = 60.0;
pub const RECORD_DOC: &str = "record";
pub const EVENT_DOC: &str = "event";

/// SHA-256 of a document's searchable text, as 64 hex characters.
pub fn text_hash(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The text full-text and semantic search index for a record: a Fact's
/// subject, predicate, object and string qualifiers, or a Holding's `text`.
/// Other records are not searchable.
pub fn record_search_text(record: &Record) -> Option<String> {
    let text = match record {
        Record::Fact { claim, .. } => fact_search_text(claim),
        Record::Holding {
            text: Some(text), ..
        } => text.clone(),
        _ => return None,
    };
    (!text.trim().is_empty()).then_some(text)
}

/// The text search indexes for an event: its title and description. Events
/// the engine writes itself (`umwelt:*` types) are not searchable.
pub fn event_search_text(event: &Event) -> Option<String> {
    if event.event_type.starts_with("umwelt:") {
        return None;
    }
    let text = event.description.as_ref().map_or_else(
        || event.title.clone(),
        |value| format!("{} {value}", event.title),
    );
    (!text.trim().is_empty()).then_some(text)
}

/// Facts have no free text: their claim is indexed as plain words. Indexing
/// the claim's JSON stored every key name and brace in the full-text index.
fn fact_search_text(claim: &Claim) -> String {
    let mut parts = vec![
        claim.subject.clone(),
        claim.predicate.replace(['-', ':'], " "),
    ];
    match &claim.object {
        ClaimObject::Entity { id } => parts.push(id.clone()),
        ClaimObject::Value { value } => parts.push(match value {
            serde_json::Value::String(text) => text.clone(),
            other => other.to_string(),
        }),
        ClaimObject::None => {}
    }
    parts.extend(
        claim
            .qualifiers
            .values()
            .filter_map(|value| value.as_str().map(str::to_owned)),
    );
    parts.join(" ")
}

/// Checks a space definition.
pub fn validate_embedding_space(space: &EmbeddingSpace) -> Result<()> {
    if space.id.trim().is_empty() {
        return Err(UmweltError::validation(
            "Embedding space ids cannot be empty.",
        ));
    }
    if space.id.starts_with("umwelt:") {
        return Err(UmweltError::validation(format!(
            "Embedding space id '{}' is reserved.",
            space.id
        )));
    }
    if space.dimensions == 0 || space.dimensions > MAX_EMBEDDING_DIMENSIONS {
        return Err(UmweltError::validation(format!(
            "Embedding spaces have between 1 and {MAX_EMBEDDING_DIMENSIONS} dimensions."
        )));
    }
    if space.model.trim().is_empty() {
        return Err(UmweltError::validation(
            "An embedding space must name its model.",
        ));
    }
    Ok(())
}

/// Checks a document type a vector may belong to.
pub fn validate_doc_type(doc_type: &str) -> Result<()> {
    if doc_type == RECORD_DOC || doc_type == EVENT_DOC {
        Ok(())
    } else {
        Err(UmweltError::validation(format!(
            "Document type '{doc_type}' has no vectors; use 'record' or 'event'."
        )))
    }
}

/// Checks a `textHash`: 64 hex characters.
pub fn validate_text_hash(hash: &str) -> Result<()> {
    if hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(UmweltError::validation(
            "textHash must be 64 hex characters.",
        ))
    }
}

/// A vector of the space's size, finite and not zero, scaled to length 1 so
/// that the cosine similarity of two stored vectors is their dot product.
pub fn normalize_vector(vector: &[f32], dimensions: u32) -> Result<Vec<f32>> {
    if vector.len() != dimensions as usize {
        return Err(UmweltError::validation(format!(
            "The vector has {} dimensions; the space has {dimensions}.",
            vector.len()
        )));
    }
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(UmweltError::validation(
            "Vector values must be finite numbers.",
        ));
    }
    let norm = vector
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>()
        .sqrt();
    if norm == 0.0 || !norm.is_finite() {
        return Err(UmweltError::validation("A vector cannot be all zeros."));
    }
    Ok(vector
        .iter()
        .map(|value| (f64::from(*value) / norm) as f32)
        .collect())
}

/// The cosine similarity of two normalized vectors: their dot product, summed
/// in `f64` in index order so every platform gets the same value.
pub fn similarity(left: &[f32], right: &[f32]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(a, b)| f64::from(*a) * f64::from(*b))
        .sum()
}

/// A document in a ranked list.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocRef {
    pub doc_type: String,
    pub doc_id: String,
}

/// A ranked search result before its document is loaded.
#[derive(Debug, Clone, PartialEq)]
pub struct RankedDoc {
    pub doc: DocRef,
    pub matched: SearchMatch,
    pub score: Option<f64>,
}

/// Orders by score, highest first, then by document type and id.
pub fn rank_order(left: &(DocRef, f64), right: &(DocRef, f64)) -> Ordering {
    right
        .1
        .total_cmp(&left.1)
        .then_with(|| left.0.cmp(&right.0))
}

/// Reciprocal Rank Fusion of a text ranking and a semantic ranking (both best
/// first): each document scores `1 / (k + position)` per list it appears in,
/// positions counted from 1. Independent of the scale of either ranking.
pub fn fuse_rankings(text: &[DocRef], semantic: &[(DocRef, f64)]) -> Vec<RankedDoc> {
    let mut fused: BTreeMap<DocRef, (f64, bool, bool)> = BTreeMap::new();
    for (position, doc) in text.iter().enumerate() {
        let entry = fused.entry(doc.clone()).or_insert((0.0, false, false));
        entry.0 += 1.0 / (RRF_K + (position + 1) as f64);
        entry.1 = true;
    }
    for (position, (doc, _)) in semantic.iter().enumerate() {
        let entry = fused.entry(doc.clone()).or_insert((0.0, false, false));
        entry.0 += 1.0 / (RRF_K + (position + 1) as f64);
        entry.2 = true;
    }
    let mut ranked = fused
        .into_iter()
        .map(|(doc, (score, in_text, in_semantic))| {
            let matched = match (in_text, in_semantic) {
                (true, true) => SearchMatch::Both,
                (true, false) => SearchMatch::Text,
                _ => SearchMatch::Semantic,
            };
            (doc, score, matched)
        })
        .collect::<Vec<_>>();
    ranked
        .sort_by(|left, right| rank_order(&(left.0.clone(), left.1), &(right.0.clone(), right.1)));
    ranked
        .into_iter()
        .map(|(doc, score, matched)| RankedDoc {
            doc,
            matched,
            score: Some(score),
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn doc(id: &str) -> DocRef {
        DocRef {
            doc_type: RECORD_DOC.to_owned(),
            doc_id: id.to_owned(),
        }
    }

    #[test]
    fn normalized_vectors_have_unit_length_and_cosine_is_their_dot_product() {
        let left = normalize_vector(&[3.0, 4.0], 2).expect("normalize");
        assert!((similarity(&left, &left) - 1.0).abs() < 1e-6);
        let right = normalize_vector(&[4.0, -3.0], 2).expect("normalize");
        assert!(similarity(&left, &right).abs() < 1e-6);
        assert!(normalize_vector(&[0.0, 0.0], 2).is_err());
        assert!(normalize_vector(&[1.0], 2).is_err());
        assert!(normalize_vector(&[f32::NAN, 1.0], 2).is_err());
    }

    #[test]
    fn fusion_rewards_documents_found_by_both_rankings() {
        let fused = fuse_rankings(&[doc("a"), doc("b")], &[(doc("b"), 0.9), (doc("c"), 0.8)]);
        let order = fused
            .iter()
            .map(|item| (item.doc.doc_id.as_str(), item.matched))
            .collect::<Vec<_>>();
        assert_eq!(
            order,
            vec![
                ("b", SearchMatch::Both),
                ("a", SearchMatch::Text),
                ("c", SearchMatch::Semantic)
            ]
        );
    }

    #[test]
    fn ties_are_broken_by_document_type_and_id() {
        let mut ranked = [(doc("b"), 0.5), (doc("a"), 0.5), (doc("c"), 0.9)];
        ranked.sort_by(rank_order);
        let ids = ranked
            .iter()
            .map(|(doc, _)| doc.doc_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["c", "a", "b"]);
    }
}
