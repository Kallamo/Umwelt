// SPDX-License-Identifier: AGPL-3.0-only

use std::fs;
use std::io;
use std::path::PathBuf;

use schemars::{JsonSchema, schema_for};
use serde_json::{Map, Value, json};
use umwelt_api::*;
use umwelt_protocol::*;

fn insert<T: JsonSchema>(
    defs: &mut Map<String, Value>,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut schema = serde_json::to_value(schema_for!(T))?;
    let object = schema
        .as_object_mut()
        .ok_or_else(|| io::Error::other("Generated schema root is not an object."))?;
    object.remove("$schema");
    if let Some(Value::Object(nested)) = object.remove("$defs") {
        defs.extend(nested);
    }
    defs.insert(name.to_owned(), schema);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../spec/schema/umwelt-protocol-v1.schema.json");
    let mut defs = Map::new();
    macro_rules! define { ($($name:ident),* $(,)?) => { $(insert::<$name>(&mut defs, stringify!($name))?;)* }; }
    define!(
        RequestEnvelope,
        SuccessEnvelope,
        FailureEnvelope,
        HelloPayload,
        HelloResult,
        PublicError,
        ErrorCode,
        ProtocolRange,
        Capability,
        CapabilityKind,
        EmptyPayload,
        CommitBatchPayload,
        TransmitBatchPayload,
        TransmitShared,
        TransmitBatchItem,
        ClaimKeyPayload,
        ClaimKeyResult,
        DispositionsPayload,
        EnsureWorldInput,
        EnsureWorldResult,
        ProjectId,
        PagedProject,
        EntityId,
        EvidenceId,
        RecordId,
        ProposalId,
        ReviewPayload,
        ApplyPayload,
        RejectPayload,
        HoldsPayload,
        HoldingsPayload,
        WhoHoldsPayload,
        CanonPayload,
        PerspectivePayload,
        ComparePayload,
        HolderAt,
        TimelinePayload,
        ChangesPayload,
        SearchPayload,
        TimelineRef,
        DiscardPayload,
        TimelineListPayload,
        OpenDraftInput,
        Timeline,
        PurgeResult,
        ResolveEntityPayload,
        ResolveEntitiesPayload,
        EnsureEntitiesInput,
        GroupCoveragePayload,
        WitnessInput,
        GroupCoverageResult,
        AdjustHoldingsInput,
        TransitionBatchInput,
        SetDispositionsInput,
        EntityAtPayload,
        EntityKnownPayload,
        EntityAtResult,
        EntityKnownResult,
        BeliefsAboutPayload,
        BreachesPayload,
        CheckUtteranceInput,
        CheckUtteranceResult,
        ContextAdmitInput,
        ContextAdmitResult,
        ProposalRebaseResult,
        CreateProjectInput,
        CreateEntityTypeInput,
        CreateEntityInput,
        UpdateEntityInput,
        DefinePredicateInput,
        DefineAxisInput,
        AxisDef,
        DefineEmbeddingSpaceInput,
        EmbeddingSpace,
        EmbeddingSpaceRef,
        DropEmbeddingSpaceResult,
        PendingEmbeddingsInput,
        PendingEmbeddingsResult,
        PutEmbeddingsInput,
        PutEmbeddingsResult,
        DeleteEmbeddingsInput,
        DeleteEmbeddingsResult,
        SearchSemantic,
        Project,
        EntityType,
        Entity,
        PredicateDef,
        EvidenceSource,
        CommitInput,
        CommitOutcome,
        ProposeInput,
        Proposal,
        ReviewInput,
        TransmitInput,
        HoldsResult,
        HoldingsResult,
        PerspectiveResult,
        CompareResult,
        TraceResult,
        Record,
        At,
        Page,
        HoldingFilter,
        WhoHoldsFilter,
        CanonFilter,
        SearchFilter,
        ChangeItem,
        SearchItem,
        Event,
    );
    insert::<ProjectListResult>(&mut defs, "ProjectListResult")?;
    insert::<EntityTypeListResult>(&mut defs, "EntityTypeListResult")?;
    insert::<PredicateListResult>(&mut defs, "PredicateListResult")?;
    insert::<AxisListResult>(&mut defs, "AxisListResult")?;
    insert::<EmbeddingSpaceListResult>(&mut defs, "EmbeddingSpaceListResult")?;
    insert::<ProposalListResult>(&mut defs, "ProposalListResult")?;
    insert::<CommitBatchResult>(&mut defs, "CommitBatchResult")?;
    insert::<TransmitBatchResult>(&mut defs, "TransmitBatchResult")?;
    insert::<DispositionsResult>(&mut defs, "DispositionsResult")?;
    insert::<WhoHoldsResult>(&mut defs, "WhoHoldsResult")?;
    insert::<ContradictionsResult>(&mut defs, "ContradictionsResult")?;
    insert::<BreachesResult>(&mut defs, "BreachesResult")?;
    insert::<BeliefsAboutResult>(&mut defs, "BeliefsAboutResult")?;
    insert::<ResolveEntitiesResult>(&mut defs, "ResolveEntitiesResult")?;
    insert::<EnsureEntitiesResult>(&mut defs, "EnsureEntitiesResult")?;
    insert::<TimelineListResult>(&mut defs, "TimelineListResult")?;
    insert::<PageResult<Record>>(&mut defs, "RecordPageResult")?;
    insert::<PageResult<Event>>(&mut defs, "EventPageResult")?;
    insert::<PageResult<Entity>>(&mut defs, "EntityPageResult")?;
    insert::<PageResult<ChangeItem>>(&mut defs, "ChangePageResult")?;
    insert::<PageResult<SearchItem>>(&mut defs, "SearchPageResult")?;

    let x_ops = OP_SCHEMAS
        .iter()
        .map(|operation| {
            (
                operation.name.to_owned(),
                json!({
                    "kind": operation.kind,
                    "payload": format!("#/$defs/{}", operation.payload),
                    "result": format!("#/$defs/{}", operation.result),
                }),
            )
        })
        .collect::<Map<_, _>>();
    let schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://umwelt.dev/spec/schema/umwelt-protocol-v1.schema.json",
        "title": "Umwelt JSON Protocol v1",
        "protocolVersion": 1,
        "$defs": defs,
        "x-ops": x_ops,
    });
    let generated = format!("{}\n", serde_json::to_string_pretty(&schema)?);
    if std::env::args().any(|argument| argument == "--check") {
        let current = match fs::read_to_string(&output) {
            Ok(value) => value,
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
        fs::write(output, generated)?;
    }
    Ok(())
}
