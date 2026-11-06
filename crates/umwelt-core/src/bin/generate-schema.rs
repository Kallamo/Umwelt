// SPDX-License-Identifier: AGPL-3.0-only
//! Generates the Umwelt v1 JSON Schema from the Rust contract.

use std::fs;
use std::io;
use std::path::PathBuf;

use schemars::schema_for;
use serde_json::{Map, Value, json};
use umwelt_core::domain::*;

fn insert_schema<T: schemars::JsonSchema>(
    definitions: &mut Map<String, Value>,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut schema = serde_json::to_value(schema_for!(T))?;
    let object = schema
        .as_object_mut()
        .ok_or_else(|| io::Error::other("Generated schema root is not an object."))?;
    object.remove("$schema");
    if let Some(Value::Object(nested)) = object.remove("$defs") {
        definitions.extend(nested);
    }
    definitions.insert(name.to_owned(), schema);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec/schema/umwelt-v1.schema.json");
    let mut definitions = Map::new();
    macro_rules! define {
        ($($name:ident),* $(,)?) => {
            $(
                insert_schema::<$name>(&mut definitions, stringify!($name))?;
            )*
        };
    }
    define!(
        Position,
        At,
        Page,
        VocabularyMode,
        Project,
        CreateProjectInput,
        EntityKind,
        FieldType,
        FieldOption,
        FieldConstraints,
        FieldDef,
        EntityType,
        CreateEntityTypeInput,
        ExternalRef,
        Entity,
        CreateEntityInput,
        UpdateEntityInput,
        ObjectKind,
        Cardinality,
        PredicateDef,
        DefinePredicateInput,
        AxisDef,
        DefineAxisInput,
        EmbeddingSpace,
        DefineEmbeddingSpaceInput,
        DropEmbeddingSpaceResult,
        EmbeddingDocRef,
        PendingEmbeddingsInput,
        PendingEmbedding,
        PendingEmbeddingsResult,
        EmbeddingItem,
        PutEmbeddingsInput,
        PutEmbeddingsResult,
        DeleteEmbeddingsInput,
        DeleteEmbeddingsResult,
        SearchSemantic,
        SearchMatch,
        ClaimObject,
        Claim,
        ClaimRef,
        Attitude,
        Accessibility,
        Intent,
        Acquisition,
        Origin,
        Provenance,
        RecordCommon,
        Record,
        TransitionStatus,
        Transition,
        EvidenceRef,
        EvidenceSource,
        Event,
        EventInput,
        RecordInputCommon,
        RecordInput,
        TransitionInput,
        CommitInput,
        ProposeInput,
        Warning,
        CommitOutcome,
        ProposalStatus,
        ReviewDecision,
        ReviewInput,
        ProposalReview,
        Proposal,
        HoldsResult,
        HoldingFilter,
        HoldingItem,
        HoldingsResult,
        WhoHoldsFilter,
        WhoHoldsItem,
        CanonFilter,
        PerspectiveResult,
        CompareEntry,
        CompareCounts,
        CompareResult,
        ContradictionKind,
        ContradictionReason,
        Contradiction,
        ChainLink,
        TraceResult,
        ChangeItem,
        SearchFilter,
        SearchItem,
        TransmitInput,
        RecordState,
        ProposalRebaseResult,
        OperationError,
        OperationCheck,
        ContextAdmitResult,
        ContextRejected,
        ContextAdmitted,
        ContextRejection,
        ContextAdmitInput,
        ContextCandidate,
        CheckUtteranceResult,
        UtteranceCheck,
        CanonStatus,
        UtteranceVerdict,
        CheckUtteranceInput,
        Breach,
        BeliefAboutItem,
        EntityKnownResult,
        EntityAtResult,
        Secrecy,
        SetDispositionsInput,
        DispositionSetting,
        TransitionBatchInput,
        AdjustHoldingsInput,
        HoldingAdjustment,
        GroupCoverageResult,
        WitnessInput,
        Witness,
        WitnessedClaim,
        ResolvedEntity,
        EnsureEntityResult,
        EnsureEntitiesInput,
        EnsureEntityInput,
        TimelineKind,
        TimelineStatus,
        Timeline,
        OpenDraftInput,
        PurgeCounts,
        PurgeResult,
    );
    insert_schema::<PageResult<Record>>(&mut definitions, "RecordPageResult")?;
    insert_schema::<PageResult<Event>>(&mut definitions, "EventPageResult")?;
    insert_schema::<PageResult<Entity>>(&mut definitions, "EntityPageResult")?;
    insert_schema::<PageResult<ChangeItem>>(&mut definitions, "ChangePageResult")?;
    insert_schema::<PageResult<SearchItem>>(&mut definitions, "SearchPageResult")?;
    let schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://umwelt.dev/spec/schema/umwelt-v1.schema.json",
        "title": "Umwelt v1 Contract",
        "contractVersion": UMWELT_CONTRACT_VERSION,
        "$defs": definitions,
    });
    let generated = format!("{}\n", serde_json::to_string_pretty(&schema)?);
    if std::env::args().any(|argument| argument == "--check") {
        let current = match fs::read_to_string(&output) {
            Ok(current) => current,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error.into()),
        };
        if current != generated {
            return Err(io::Error::other(format!(
                "Generated schema differs from {}",
                output.display()
            ))
            .into());
        }
    } else {
        let parent = output
            .parent()
            .ok_or_else(|| io::Error::other("Schema output has no parent directory."))?;
        fs::create_dir_all(parent)?;
        fs::write(&output, generated)?;
    }
    Ok(())
}
