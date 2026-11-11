// SPDX-License-Identifier: AGPL-3.0-only
// Generated from the versioned JSON schemas by scripts/generate-csharp.mjs. Do not edit.
#nullable enable
using System;
using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Kallamo.Umwelt;

public enum Accessibility
{
    Available,
    Suppressed,
    Inaccessible,
}

internal sealed class AccessibilityConverter : JsonConverter<Accessibility>
{
    public override Accessibility Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "available" => Accessibility.Available,
        "suppressed" => Accessibility.Suppressed,
        "inaccessible" => Accessibility.Inaccessible,
        var other => throw new JsonException($"Unknown Accessibility '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, Accessibility value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        Accessibility.Available => "available",
        Accessibility.Suppressed => "suppressed",
        Accessibility.Inaccessible => "inaccessible",
        _ => throw new JsonException($"Unknown Accessibility {value}."),
    });
}

public abstract class Acquisition
{
    /// <summary>The variant tag.</summary>
    [JsonPropertyName("kind")]
    public abstract string Kind { get; }

    public sealed class Witnessed : Acquisition
    {
        [JsonPropertyName("kind")]
        public override string Kind => "witnessed";

        [JsonPropertyName("eventId")]
        public string EventId { get; set; } = default!;
    }

    public sealed class ToldBy : Acquisition
    {
        [JsonPropertyName("kind")]
        public override string Kind => "told-by";

        [JsonPropertyName("intent")]
        public Intent Intent { get; set; }

        [JsonPropertyName("sourceHolderId")]
        public string SourceHolderId { get; set; } = default!;

        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("sourceRecordId")]
        public string? SourceRecordId { get; set; }
    }

    public sealed class Evidence : Acquisition
    {
        [JsonPropertyName("kind")]
        public override string Kind => "evidence";

        [JsonPropertyName("evidenceId")]
        public string EvidenceId { get; set; } = default!;
    }

    public sealed class Inferred : Acquisition
    {
        [JsonPropertyName("kind")]
        public override string Kind => "inferred";

        [JsonPropertyName("fromRecordIds")]
        public List<string> FromRecordIds { get; set; } = new();

        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("rule")]
        public string? Rule { get; set; }
    }

    public sealed class Authored : Acquisition
    {
        [JsonPropertyName("kind")]
        public override string Kind => "authored";

    }
}

internal sealed class AcquisitionConverter : JsonConverter<Acquisition>
{
    public override Acquisition Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var document = JsonDocument.ParseValue(ref reader);
        var kind = document.RootElement.TryGetProperty("kind", out var tag) ? tag.GetString() : null;
        return kind switch
        {
            "witnessed" => document.RootElement.Deserialize<Acquisition.Witnessed>(options)!,
            "told-by" => document.RootElement.Deserialize<Acquisition.ToldBy>(options)!,
            "evidence" => document.RootElement.Deserialize<Acquisition.Evidence>(options)!,
            "inferred" => document.RootElement.Deserialize<Acquisition.Inferred>(options)!,
            "authored" => document.RootElement.Deserialize<Acquisition.Authored>(options)!,
            _ => throw new JsonException($"Unknown Acquisition kind '{kind}'."),
        };
    }

    public override void Write(Utf8JsonWriter writer, Acquisition value, JsonSerializerOptions options) => JsonSerializer.Serialize(writer, value, value.GetType(), options);
}

/// <summary>
/// Adjusts many Holdings in one commit, e.g. a night of memory decay. How much
/// anything decays is the client's policy.
/// </summary>
public sealed partial class AdjustHoldingsInput
{
    [JsonPropertyName("adjustments")]
    public List<HoldingAdjustment> Adjustments { get; set; } = new();

    [JsonPropertyName("attemptId")]
    public string AttemptId { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("expectedVersion")]
    public ulong? ExpectedVersion { get; set; }

    /// <summary>
    /// Positions of the generated event on additional axes.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("on")]
    public Dictionary<string, long>? On { get; set; }

    [JsonPropertyName("position")]
    public Position Position { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("provenance")]
    public Provenance? Provenance { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

public sealed partial class ApplyPayload
{
    [JsonPropertyName("attemptId")]
    public string AttemptId { get; set; } = default!;

    [JsonPropertyName("proposalId")]
    public string ProposalId { get; set; } = default!;
}

/// <summary>
/// A read position. `tick` is on the main axis unless `axis` names an
/// additional axis (`axis.define`); then it is a position on that axis.
/// </summary>
public sealed partial class At
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("asOfVersion")]
    public ulong? AsOfVersion { get; set; }

    /// <summary>
    /// An additional axis to read on. Absent: the main axis.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("axis")]
    public string? Axis { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("tick")]
    public long? Tick { get; set; }
}

public enum Attitude
{
    Knows,
    Believes,
    Suspects,
    Remembers,
}

internal sealed class AttitudeConverter : JsonConverter<Attitude>
{
    public override Attitude Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "knows" => Attitude.Knows,
        "believes" => Attitude.Believes,
        "suspects" => Attitude.Suspects,
        "remembers" => Attitude.Remembers,
        var other => throw new JsonException($"Unknown Attitude '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, Attitude value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        Attitude.Knows => "knows",
        Attitude.Believes => "believes",
        Attitude.Suspects => "suspects",
        Attitude.Remembers => "remembers",
        _ => throw new JsonException($"Unknown Attitude {value}."),
    });
}

/// <summary>
/// An additional order of time, such as reading order next to story order.
/// The main axis (`Position.tick`) is implicit and has no definition.
/// </summary>
public sealed partial class AxisDef
{
    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("label")]
    public string Label { get; set; } = default!;
}

/// <summary>
/// The statuses `transition.batch` applies. `superseded` is not one of them:
/// it needs a replacement record, which `holding.adjustBatch` writes.
/// </summary>
public enum BatchTransitionStatus
{
    Retracted,
    Revoked,
    Forgotten,
    Abandoned,
}

internal sealed class BatchTransitionStatusConverter : JsonConverter<BatchTransitionStatus>
{
    public override BatchTransitionStatus Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "retracted" => BatchTransitionStatus.Retracted,
        "revoked" => BatchTransitionStatus.Revoked,
        "forgotten" => BatchTransitionStatus.Forgotten,
        "abandoned" => BatchTransitionStatus.Abandoned,
        var other => throw new JsonException($"Unknown BatchTransitionStatus '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, BatchTransitionStatus value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        BatchTransitionStatus.Retracted => "retracted",
        BatchTransitionStatus.Revoked => "revoked",
        BatchTransitionStatus.Forgotten => "forgotten",
        BatchTransitionStatus.Abandoned => "abandoned",
        _ => throw new JsonException($"Unknown BatchTransitionStatus {value}."),
    });
}

/// <summary>
/// What a holder thinks another holder knows (`umwelt:holds` Holdings), next
/// to what the other holder actually holds at the same position.
/// </summary>
public sealed partial class BeliefAboutItem
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("actualRecordId")]
    public string? ActualRecordId { get; set; }

    /// <summary>
    /// Whether the other holder really holds the claim (active, available).
    /// </summary>
    [JsonPropertyName("actuallyHolds")]
    public bool ActuallyHolds { get; set; }

    [JsonPropertyName("attitude")]
    public Attitude Attitude { get; set; }

    [JsonPropertyName("claimKey")]
    public string ClaimKey { get; set; } = default!;

    [JsonPropertyName("confidence")]
    public double Confidence { get; set; }

    /// <summary>
    /// True: the holder thinks the other holds the claim. False: thinks not.
    /// </summary>
    [JsonPropertyName("polarity")]
    public bool Polarity { get; set; }

    /// <summary>
    /// The holder's `umwelt:holds` Holding.
    /// </summary>
    [JsonPropertyName("recordId")]
    public string RecordId { get; set; } = default!;
}

public sealed partial class BeliefsAboutPayload
{
    /// <summary>
    /// The other holder whose knowledge `holder` has beliefs about.
    /// </summary>
    [JsonPropertyName("about")]
    public string About { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    [JsonPropertyName("holder")]
    public string Holder { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

/// <summary>
/// A secret told to someone outside its circle.
/// </summary>
public sealed partial class Breach
{
    [JsonPropertyName("eventId")]
    public string EventId { get; set; } = default!;

    /// <summary>
    /// Who keeps the secret.
    /// </summary>
    [JsonPropertyName("owner")]
    public string Owner { get; set; } = default!;

    [JsonPropertyName("position")]
    public Position Position { get; set; } = default!;

    [JsonPropertyName("recipient")]
    public string Recipient { get; set; } = default!;

    /// <summary>
    /// The recipient's Holding acquired by being told.
    /// </summary>
    [JsonPropertyName("recordId")]
    public string RecordId { get; set; } = default!;

    /// <summary>
    /// The secret Holding.
    /// </summary>
    [JsonPropertyName("secretRecordId")]
    public string SecretRecordId { get; set; } = default!;

    [JsonPropertyName("toldBy")]
    public string ToldBy { get; set; } = default!;
}

public sealed partial class BreachesPayload
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    /// <summary>
    /// Only the secrets kept by this holder.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("owner")]
    public string? Owner { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

public sealed partial class CanonFilter
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("object")]
    public string? Object { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("predicate")]
    public string? Predicate { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("subject")]
    public string? Subject { get; set; }
}

public sealed partial class CanonPayload
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("filter")]
    public CanonFilter? Filter { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("page")]
    public Page? Page { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

/// <summary>
/// How a claim relates to canon at the read position.
/// </summary>
public enum CanonStatus
{
    Agrees,
    Contradicts,
    NoCanon,
}

internal sealed class CanonStatusConverter : JsonConverter<CanonStatus>
{
    public override CanonStatus Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "agrees" => CanonStatus.Agrees,
        "contradicts" => CanonStatus.Contradicts,
        "no-canon" => CanonStatus.NoCanon,
        var other => throw new JsonException($"Unknown CanonStatus '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, CanonStatus value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        CanonStatus.Agrees => "agrees",
        CanonStatus.Contradicts => "contradicts",
        CanonStatus.NoCanon => "no-canon",
        _ => throw new JsonException($"Unknown CanonStatus {value}."),
    });
}

public enum Cardinality
{
    One,
    Many,
}

internal sealed class CardinalityConverter : JsonConverter<Cardinality>
{
    public override Cardinality Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "one" => Cardinality.One,
        "many" => Cardinality.Many,
        var other => throw new JsonException($"Unknown Cardinality '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, Cardinality value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        Cardinality.One => "one",
        Cardinality.Many => "many",
        _ => throw new JsonException($"Unknown Cardinality {value}."),
    });
}

public sealed partial class ChainLink
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("acquisition")]
    public Acquisition? Acquisition { get; set; }

    [JsonPropertyName("record")]
    public Record Record { get; set; } = default!;
}

public sealed partial class ChangeItem
{
    [JsonPropertyName("payload")]
    public JsonElement Payload { get; set; }

    [JsonPropertyName("seq")]
    public ulong Seq { get; set; }

    [JsonPropertyName("timelineId")]
    public string TimelineId { get; set; } = default!;

    [JsonPropertyName("type")]
    public string Type { get; set; } = default!;

    [JsonPropertyName("version")]
    public ulong Version { get; set; }
}

public sealed partial class ChangePageResult
{
    [JsonPropertyName("items")]
    public List<ChangeItem> Items { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("nextCursor")]
    public string? NextCursor { get; set; }
}

public sealed partial class ChangesPayload
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("limit")]
    public uint? Limit { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("sinceSeq")]
    public ulong? SinceSeq { get; set; }

    /// <summary>
    /// Narrows the feed to one timeline. The feed is global by default.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

/// <summary>
/// Claims a holder says or thinks, to check against what the holder can know.
/// The claims come from the client (or a model it runs): the engine never
/// reads free text.
/// </summary>
public sealed partial class CheckUtteranceInput
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    [JsonPropertyName("claims")]
    public List<Claim> Claims { get; set; } = new();

    [JsonPropertyName("holder")]
    public string Holder { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

public sealed partial class CheckUtteranceResult
{
    /// <summary>
    /// Every claim has the verdict `holds` with the same polarity.
    /// </summary>
    [JsonPropertyName("consistent")]
    public bool Consistent { get; set; }

    [JsonPropertyName("items")]
    public List<UtteranceCheck> Items { get; set; } = new();
}

public sealed partial class Claim
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("about")]
    public string? About { get; set; }

    [JsonPropertyName("object")]
    public ClaimObject Object { get; set; } = default!;

    [JsonPropertyName("polarity")]
    public bool Polarity { get; set; }

    [JsonPropertyName("predicate")]
    public string Predicate { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("qualifiers")]
    public Dictionary<string, JsonElement>? Qualifiers { get; set; }

    [JsonPropertyName("subject")]
    public string Subject { get; set; } = default!;
}

public sealed partial class ClaimKeyPayload
{
    [JsonPropertyName("claim")]
    public Claim Claim { get; set; } = default!;
}

/// <summary>
/// The keys the engine derives from a claim (`spec/claim-key.md`).
/// </summary>
public sealed partial class ClaimKeyResult
{
    /// <summary>
    /// Identifies the claim whatever its polarity.
    /// </summary>
    [JsonPropertyName("claimKey")]
    public string ClaimKey { get; set; } = default!;

    /// <summary>
    /// Identifies the subject and predicate.
    /// </summary>
    [JsonPropertyName("slotKey")]
    public string SlotKey { get; set; } = default!;
}

public abstract class ClaimObject
{
    /// <summary>The variant tag.</summary>
    [JsonPropertyName("kind")]
    public abstract string Kind { get; }

    public sealed class Entity : ClaimObject
    {
        [JsonPropertyName("kind")]
        public override string Kind => "entity";

        [JsonPropertyName("id")]
        public string Id { get; set; } = default!;
    }

    public sealed class ValueClaimObject : ClaimObject
    {
        [JsonPropertyName("kind")]
        public override string Kind => "value";

        [JsonPropertyName("value")]
        public JsonElement Value { get; set; }
    }

    public sealed class None : ClaimObject
    {
        [JsonPropertyName("kind")]
        public override string Kind => "none";

    }
}

internal sealed class ClaimObjectConverter : JsonConverter<ClaimObject>
{
    public override ClaimObject Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var document = JsonDocument.ParseValue(ref reader);
        var kind = document.RootElement.TryGetProperty("kind", out var tag) ? tag.GetString() : null;
        return kind switch
        {
            "entity" => document.RootElement.Deserialize<ClaimObject.Entity>(options)!,
            "value" => document.RootElement.Deserialize<ClaimObject.ValueClaimObject>(options)!,
            "none" => document.RootElement.Deserialize<ClaimObject.None>(options)!,
            _ => throw new JsonException($"Unknown ClaimObject kind '{kind}'."),
        };
    }

    public override void Write(Utf8JsonWriter writer, ClaimObject value, JsonSerializerOptions options) => JsonSerializer.Serialize(writer, value, value.GetType(), options);
}

/// <summary>
/// Exactly one of the properties is set.
/// </summary>
public sealed class ClaimRef
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("claim")]
    public Claim? Claim { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("claimKey")]
    public string? ClaimKey { get; set; }
}

public sealed partial class CommitBatchPayload
{
    [JsonPropertyName("commits")]
    public List<CommitInput> Commits { get; set; } = new();
}

public sealed partial class CommitInput
{
    [JsonPropertyName("attemptId")]
    public string AttemptId { get; set; } = default!;

    [JsonPropertyName("event")]
    public EventInput Event { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("expectedVersion")]
    public ulong? ExpectedVersion { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("records")]
    public List<RecordInput>? Records { get; set; }

    /// <summary>
    /// Timeline to write to; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("transitions")]
    public List<TransitionInput>? Transitions { get; set; }
}

public sealed partial class CommitOutcome
{
    [JsonPropertyName("eventId")]
    public string EventId { get; set; } = default!;

    [JsonPropertyName("recordIds")]
    public List<string> RecordIds { get; set; } = new();

    [JsonPropertyName("transitionIds")]
    public List<string> TransitionIds { get; set; } = new();

    [JsonPropertyName("version")]
    public ulong Version { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("warnings")]
    public List<Warning>? Warnings { get; set; }
}

public sealed partial class CompareCounts
{
    [JsonPropertyName("conflicting")]
    public ulong Conflicting { get; set; }

    [JsonPropertyName("onlyA")]
    public ulong OnlyA { get; set; }

    [JsonPropertyName("onlyB")]
    public ulong OnlyB { get; set; }

    [JsonPropertyName("shared")]
    public ulong Shared { get; set; }
}

public sealed partial class CompareEntry
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("a")]
    public Record? A { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("b")]
    public Record? B { get; set; }

    [JsonPropertyName("claimKey")]
    public string ClaimKey { get; set; } = default!;
}

public sealed partial class ComparePayload
{
    [JsonPropertyName("a")]
    public string A { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    /// <summary>
    /// Where `b` is read; defaults to `at`. It may be on another axis.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("atB")]
    public At? AtB { get; set; }

    [JsonPropertyName("b")]
    public string B { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

public sealed partial class CompareResult
{
    [JsonPropertyName("conflicting")]
    public List<CompareEntry> Conflicting { get; set; } = new();

    [JsonPropertyName("counts")]
    public CompareCounts Counts { get; set; } = default!;

    [JsonPropertyName("onlyA")]
    public List<CompareEntry> OnlyA { get; set; } = new();

    [JsonPropertyName("onlyB")]
    public List<CompareEntry> OnlyB { get; set; } = new();

    [JsonPropertyName("shared")]
    public List<CompareEntry> Shared { get; set; } = new();
}

/// <summary>
/// Decides which candidates may enter a context seen from one perspective.
/// Without `holder` the perspective is the narrator's, who sees everything
/// visible at the position.
/// </summary>
public sealed partial class ContextAdmitInput
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("authorizedNarrator")]
    public bool? AuthorizedNarrator { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("budget")]
    public ulong? Budget { get; set; }

    [JsonPropertyName("candidates")]
    public List<ContextCandidate> Candidates { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("excludeClassifications")]
    public List<string>? ExcludeClassifications { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("holder")]
    public string? Holder { get; set; }

    /// <summary>
    /// Admit suppressed and inaccessible Holdings. Requires
    /// `authorizedNarrator`.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("includeWithheld")]
    public bool? IncludeWithheld { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

public sealed partial class ContextAdmitResult
{
    /// <summary>
    /// Admitted candidates, highest priority first (ties keep input order).
    /// </summary>
    [JsonPropertyName("admitted")]
    public List<ContextAdmitted> Admitted { get; set; } = new();

    /// <summary>
    /// Refused candidates, in input order.
    /// </summary>
    [JsonPropertyName("rejected")]
    public List<ContextRejected> Rejected { get; set; } = new();

    [JsonPropertyName("usedBudget")]
    public ulong UsedBudget { get; set; }
}

public sealed partial class ContextAdmitted
{
    [JsonPropertyName("cost")]
    public uint Cost { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("eventId")]
    public string? EventId { get; set; }

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("priority")]
    public int Priority { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("recordId")]
    public string? RecordId { get; set; }
}

/// <summary>
/// Something a client wants to put in a context (a prompt, a scene brief).
/// Exactly one of `recordId` and `eventId` is given.
/// </summary>
public sealed partial class ContextCandidate
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("classification")]
    public string? Classification { get; set; }

    /// <summary>
    /// Budget units the candidate costs (tokens, lines...). Defaults to 1.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("cost")]
    public uint? Cost { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("eventId")]
    public string? EventId { get; set; }

    /// <summary>
    /// The client's own id for the candidate.
    /// </summary>
    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    /// <summary>
    /// Higher priorities are admitted first when the budget is short.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("priority")]
    public int? Priority { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("recordId")]
    public string? RecordId { get; set; }
}

public sealed partial class ContextRejected
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("eventId")]
    public string? EventId { get; set; }

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("reason")]
    public ContextRejection Reason { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("recordId")]
    public string? RecordId { get; set; }
}

/// <summary>
/// Why a candidate was refused. Checks run in this order and a candidate
/// reports the first one it fails.
/// </summary>
public enum ContextRejection
{
    /// <summary>
    /// Neither or both of `recordId` and `eventId`.
    /// </summary>
    InvalidCandidate,
    /// <summary>
    /// No such record or event in this project and timeline.
    /// </summary>
    NotInProject,
    /// <summary>
    /// Recorded after the read position.
    /// </summary>
    Future,
    /// <summary>
    /// Read on an additional axis, and the candidate's event has no position
    /// on it: it is visible to nobody there.
    /// </summary>
    NotOnAxis,
    /// <summary>
    /// Retracted, revoked, forgotten, abandoned or superseded.
    /// </summary>
    Inactive,
    /// <summary>
    /// Not part of the holder's perspective.
    /// </summary>
    NotHeld,
    /// <summary>
    /// Suppressed or inaccessible for the holder.
    /// </summary>
    Withheld,
    /// <summary>
    /// The candidate's classification is excluded.
    /// </summary>
    ExcludedClassification,
    /// <summary>
    /// The same record or event was already admitted.
    /// </summary>
    Duplicate,
    /// <summary>
    /// Higher-priority candidates used the budget first.
    /// </summary>
    OverBudget,
}

internal sealed class ContextRejectionConverter : JsonConverter<ContextRejection>
{
    public override ContextRejection Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "invalid-candidate" => ContextRejection.InvalidCandidate,
        "not-in-project" => ContextRejection.NotInProject,
        "future" => ContextRejection.Future,
        "not-on-axis" => ContextRejection.NotOnAxis,
        "inactive" => ContextRejection.Inactive,
        "not-held" => ContextRejection.NotHeld,
        "withheld" => ContextRejection.Withheld,
        "excluded-classification" => ContextRejection.ExcludedClassification,
        "duplicate" => ContextRejection.Duplicate,
        "over-budget" => ContextRejection.OverBudget,
        var other => throw new JsonException($"Unknown ContextRejection '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, ContextRejection value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        ContextRejection.InvalidCandidate => "invalid-candidate",
        ContextRejection.NotInProject => "not-in-project",
        ContextRejection.Future => "future",
        ContextRejection.NotOnAxis => "not-on-axis",
        ContextRejection.Inactive => "inactive",
        ContextRejection.NotHeld => "not-held",
        ContextRejection.Withheld => "withheld",
        ContextRejection.ExcludedClassification => "excluded-classification",
        ContextRejection.Duplicate => "duplicate",
        ContextRejection.OverBudget => "over-budget",
        _ => throw new JsonException($"Unknown ContextRejection {value}."),
    });
}

public sealed partial class Contradiction
{
    [JsonPropertyName("holdingId")]
    public string HoldingId { get; set; } = default!;

    [JsonPropertyName("kind")]
    public ContradictionKind Kind { get; set; }

    [JsonPropertyName("otherRecordId")]
    public string OtherRecordId { get; set; } = default!;

    [JsonPropertyName("reason")]
    public ContradictionReason Reason { get; set; }
}

public enum ContradictionKind
{
    VsCanon,
    VsSelf,
}

internal sealed class ContradictionKindConverter : JsonConverter<ContradictionKind>
{
    public override ContradictionKind Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "vs-canon" => ContradictionKind.VsCanon,
        "vs-self" => ContradictionKind.VsSelf,
        var other => throw new JsonException($"Unknown ContradictionKind '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, ContradictionKind value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        ContradictionKind.VsCanon => "vs-canon",
        ContradictionKind.VsSelf => "vs-self",
        _ => throw new JsonException($"Unknown ContradictionKind {value}."),
    });
}

public enum ContradictionReason
{
    OppositePolarity,
    DifferentValueInFunctionalSlot,
}

internal sealed class ContradictionReasonConverter : JsonConverter<ContradictionReason>
{
    public override ContradictionReason Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "opposite-polarity" => ContradictionReason.OppositePolarity,
        "different-value-in-functional-slot" => ContradictionReason.DifferentValueInFunctionalSlot,
        var other => throw new JsonException($"Unknown ContradictionReason '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, ContradictionReason value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        ContradictionReason.OppositePolarity => "opposite-polarity",
        ContradictionReason.DifferentValueInFunctionalSlot => "different-value-in-functional-slot",
        _ => throw new JsonException($"Unknown ContradictionReason {value}."),
    });
}

public sealed partial class CreateEntityInput
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("aliases")]
    public List<string>? Aliases { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("createdAt")]
    public Position? CreatedAt { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("externalRef")]
    public ExternalRef? ExternalRef { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("fields")]
    public Dictionary<string, JsonElement>? Fields { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("id")]
    public string? Id { get; set; }

    [JsonPropertyName("name")]
    public string Name { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonPropertyName("typeId")]
    public string TypeId { get; set; } = default!;
}

public sealed partial class CreateEntityTypeInput
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("fields")]
    public List<FieldDef>? Fields { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("id")]
    public string? Id { get; set; }

    [JsonPropertyName("kind")]
    public EntityKind Kind { get; set; }

    [JsonPropertyName("name")]
    public string Name { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;
}

public sealed partial class CreateProjectInput
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("id")]
    public string? Id { get; set; }

    [JsonPropertyName("name")]
    public string Name { get; set; } = default!;

    [JsonPropertyName("timescale")]
    public string Timescale { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("vocabularyMode")]
    public VocabularyMode? VocabularyMode { get; set; }
}

public sealed partial class DefineAxisInput
{
    [JsonPropertyName("axis")]
    public AxisDef Axis { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;
}

public sealed partial class DefineEmbeddingSpaceInput
{
    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonPropertyName("space")]
    public EmbeddingSpace Space { get; set; } = default!;
}

public sealed partial class DefinePredicateInput
{
    [JsonPropertyName("predicate")]
    public PredicateDef Predicate { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;
}

public sealed partial class DeleteEmbeddingsInput
{
    [JsonPropertyName("items")]
    public List<EmbeddingDocRef> Items { get; set; } = new();

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonPropertyName("spaceId")]
    public string SpaceId { get; set; } = default!;
}

public sealed partial class DeleteEmbeddingsResult
{
    [JsonPropertyName("removed")]
    public ulong Removed { get; set; }
}

public sealed partial class DiscardPayload
{
    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Also remove the draft's rows (`timeline.purge`) in the same call.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("purge")]
    public bool? Purge { get; set; }

    [JsonPropertyName("timelineId")]
    public string TimelineId { get; set; } = default!;
}

public sealed partial class DispositionSetting
{
    [JsonPropertyName("dimension")]
    public string Dimension { get; set; } = default!;

    [JsonPropertyName("holder")]
    public string Holder { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("id")]
    public string? Id { get; set; }

    [JsonPropertyName("target")]
    public string Target { get; set; } = default!;

    [JsonPropertyName("value")]
    public JsonElement Value { get; set; }
}

public sealed partial class DispositionsPayload
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    /// <summary>
    /// Only this dimension.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("dimension")]
    public string? Dimension { get; set; }

    [JsonPropertyName("holder")]
    public string Holder { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Only the Dispositions toward this entity.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("target")]
    public string? Target { get; set; }

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

public sealed partial class DropEmbeddingSpaceResult
{
    /// <summary>
    /// Vectors removed with the space.
    /// </summary>
    [JsonPropertyName("removed")]
    public ulong Removed { get; set; }
}

/// <summary>
/// A searchable document: a record (a Fact, or a Holding with text) or an
/// event whose type does not begin with `umwelt:`.
/// </summary>
public sealed partial class EmbeddingDocRef
{
    [JsonPropertyName("docId")]
    public string DocId { get; set; } = default!;

    /// <summary>
    /// `record` or `event`.
    /// </summary>
    [JsonPropertyName("docType")]
    public string DocType { get; set; } = default!;
}

public sealed partial class EmbeddingItem
{
    [JsonPropertyName("docId")]
    public string DocId { get; set; } = default!;

    /// <summary>
    /// `record` or `event`.
    /// </summary>
    [JsonPropertyName("docType")]
    public string DocType { get; set; } = default!;

    /// <summary>
    /// The `textHash` the vector was made from (see `embedding.pending`).
    /// </summary>
    [JsonPropertyName("textHash")]
    public string TextHash { get; set; } = default!;

    [JsonPropertyName("vector")]
    public List<double> Vector { get; set; } = new();
}

/// <summary>
/// The vectors of one embedding model. A vector is only compared with
/// vectors of the same space. `model` documents the model; the engine never
/// reads it and never runs a model.
/// </summary>
public sealed partial class EmbeddingSpace
{
    [JsonPropertyName("dimensions")]
    public uint Dimensions { get; set; }

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("model")]
    public string Model { get; set; } = default!;
}

public sealed partial class EmbeddingSpaceRef
{
    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonPropertyName("spaceId")]
    public string SpaceId { get; set; } = default!;
}

public sealed partial class EmptyPayload
{
}

public sealed partial class EnsureEntitiesInput
{
    [JsonPropertyName("entities")]
    public List<EnsureEntityInput> Entities { get; set; } = new();

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;
}

/// <summary>
/// One entity owned by another system (a game, a wiki, a host app), keyed by
/// its `externalRef`. Ensuring it twice is a no-op; changed fields are updated.
/// </summary>
public sealed partial class EnsureEntityInput
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("aliases")]
    public List<string>? Aliases { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("createdAt")]
    public Position? CreatedAt { get; set; }

    [JsonPropertyName("externalRef")]
    public ExternalRef ExternalRef { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("fields")]
    public Dictionary<string, JsonElement>? Fields { get; set; }

    /// <summary>
    /// Id used only when the entity is created; an existing entity keeps its id.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("id")]
    public string? Id { get; set; }

    [JsonPropertyName("name")]
    public string Name { get; set; } = default!;

    [JsonPropertyName("typeId")]
    public string TypeId { get; set; } = default!;
}

public sealed partial class EnsureEntityResult
{
    [JsonPropertyName("created")]
    public bool Created { get; set; }

    [JsonPropertyName("entity")]
    public Entity Entity { get; set; } = default!;
}

/// <summary>
/// A manifest of what must exist: the project and its entity types, entities,
/// predicates and axes. Ensuring it creates what is missing, in one
/// transaction, and leaves what is already there untouched; anything that
/// exists with another definition refuses the whole manifest.
/// </summary>
public sealed partial class EnsureWorldInput
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("axes")]
    public List<AxisDef>? Axes { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("entities")]
    public List<WorldEntity>? Entities { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("entityTypes")]
    public List<WorldEntityType>? EntityTypes { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("predicates")]
    public List<PredicateDef>? Predicates { get; set; }

    [JsonPropertyName("project")]
    public WorldProject Project { get; set; } = default!;
}

public sealed partial class EnsureWorldResult
{
    [JsonPropertyName("created")]
    public WorldCreated Created { get; set; } = default!;

    [JsonPropertyName("project")]
    public Project Project { get; set; } = default!;
}

public sealed partial class Entity
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("aliases")]
    public List<string>? Aliases { get; set; }

    [JsonPropertyName("archived")]
    public bool Archived { get; set; }

    [JsonPropertyName("createdAt")]
    public Position CreatedAt { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("externalRef")]
    public ExternalRef? ExternalRef { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("fields")]
    public Dictionary<string, JsonElement>? Fields { get; set; }

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("name")]
    public string Name { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonPropertyName("typeId")]
    public string TypeId { get; set; } = default!;
}

public sealed partial class EntityAtPayload
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    [JsonPropertyName("entityId")]
    public string EntityId { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

/// <summary>
/// An entity as canon describes it at one position: its current name, title
/// and aliases from `umwelt:name`, `umwelt:title` and `umwelt:alias` Facts,
/// falling back to the entity's own name and aliases.
/// </summary>
public sealed partial class EntityAtResult
{
    [JsonPropertyName("aliases")]
    public List<string> Aliases { get; set; } = new();

    [JsonPropertyName("entity")]
    public Entity Entity { get; set; } = default!;

    [JsonPropertyName("name")]
    public string Name { get; set; } = default!;

    /// <summary>
    /// The Facts the name, title and aliases come from.
    /// </summary>
    [JsonPropertyName("recordIds")]
    public List<string> RecordIds { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("title")]
    public string? Title { get; set; }
}

public sealed partial class EntityId
{
    [JsonPropertyName("entityId")]
    public string EntityIdValue { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;
}

public enum EntityKind
{
    Agent,
    Group,
    Other,
}

internal sealed class EntityKindConverter : JsonConverter<EntityKind>
{
    public override EntityKind Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "agent" => EntityKind.Agent,
        "group" => EntityKind.Group,
        "other" => EntityKind.Other,
        var other => throw new JsonException($"Unknown EntityKind '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, EntityKind value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        EntityKind.Agent => "agent",
        EntityKind.Group => "group",
        EntityKind.Other => "other",
        _ => throw new JsonException($"Unknown EntityKind {value}."),
    });
}

public sealed partial class EntityKnownPayload
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    [JsonPropertyName("entityId")]
    public string EntityId { get; set; } = default!;

    [JsonPropertyName("holder")]
    public string Holder { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

/// <summary>
/// An entity as one holder knows it: only what the holder's own active,
/// available Holdings say. A holder that knows no name gets none.
/// </summary>
public sealed partial class EntityKnownResult
{
    [JsonPropertyName("aliases")]
    public List<string> Aliases { get; set; } = new();

    /// <summary>
    /// The holder has an Awareness record of the entity.
    /// </summary>
    [JsonPropertyName("aware")]
    public bool Aware { get; set; }

    [JsonPropertyName("entityId")]
    public string EntityId { get; set; } = default!;

    [JsonPropertyName("holder")]
    public string Holder { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("name")]
    public string? Name { get; set; }

    /// <summary>
    /// The Holdings the name, title and aliases come from.
    /// </summary>
    [JsonPropertyName("recordIds")]
    public List<string> RecordIds { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("title")]
    public string? Title { get; set; }
}

public sealed partial class EntityPageResult
{
    [JsonPropertyName("items")]
    public List<Entity> Items { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("nextCursor")]
    public string? NextCursor { get; set; }
}

public sealed partial class EntityType
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("fields")]
    public List<FieldDef>? Fields { get; set; }

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("kind")]
    public EntityKind Kind { get; set; }

    [JsonPropertyName("name")]
    public string Name { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;
}

public sealed partial class Event
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("evidence")]
    public List<EvidenceRef>? Evidence { get; set; }

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("location")]
    public string? Location { get; set; }

    /// <summary>
    /// Positions on additional axes. Records and transitions of the event
    /// inherit them.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("on")]
    public Dictionary<string, long>? On { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("participants")]
    public List<string>? Participants { get; set; }

    [JsonPropertyName("position")]
    public Position Position { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonPropertyName("provenance")]
    public Provenance Provenance { get; set; } = default!;

    [JsonPropertyName("recordedAt")]
    public ulong RecordedAt { get; set; }

    [JsonPropertyName("timelineId")]
    public string TimelineId { get; set; } = default!;

    [JsonPropertyName("title")]
    public string Title { get; set; } = default!;

    [JsonPropertyName("type")]
    public string Type { get; set; } = default!;
}

public sealed partial class EventInput
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("evidence")]
    public List<EvidenceRef>? Evidence { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("id")]
    public string? Id { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("location")]
    public string? Location { get; set; }

    /// <summary>
    /// Positions on additional axes (`axis.define`), each `&gt;= 0`.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("on")]
    public Dictionary<string, long>? On { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("participants")]
    public List<string>? Participants { get; set; }

    [JsonPropertyName("position")]
    public Position Position { get; set; } = default!;

    [JsonPropertyName("provenance")]
    public Provenance Provenance { get; set; } = default!;

    [JsonPropertyName("title")]
    public string Title { get; set; } = default!;

    [JsonPropertyName("type")]
    public string Type { get; set; } = default!;
}

public sealed partial class EventPageResult
{
    [JsonPropertyName("items")]
    public List<Event> Items { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("nextCursor")]
    public string? NextCursor { get; set; }
}

public sealed partial class EvidenceId
{
    [JsonPropertyName("evidenceId")]
    public string EvidenceIdValue { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;
}

public sealed partial class EvidenceRef
{
    [JsonPropertyName("evidenceSourceId")]
    public string EvidenceSourceId { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("excerpt")]
    public string? Excerpt { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("locator")]
    public string? Locator { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("metadata")]
    public Dictionary<string, JsonElement>? Metadata { get; set; }
}

public sealed partial class EvidenceSource
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("contentHash")]
    public string? ContentHash { get; set; }

    [JsonPropertyName("createdAt")]
    public string CreatedAt { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("locator")]
    public string? Locator { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("metadata")]
    public Dictionary<string, JsonElement>? Metadata { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonPropertyName("title")]
    public string Title { get; set; } = default!;

    [JsonPropertyName("type")]
    public string Type { get; set; } = default!;
}

public sealed partial class ExternalRef
{
    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("system")]
    public string System { get; set; } = default!;
}

public sealed partial class FieldConstraints
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("allowedEntityTypeIds")]
    public List<string>? AllowedEntityTypeIds { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("maxLength")]
    public ulong? MaxLength { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("maximum")]
    public double? Maximum { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("minLength")]
    public ulong? MinLength { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("minimum")]
    public double? Minimum { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("options")]
    public List<FieldOption>? Options { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("required")]
    public bool? Required { get; set; }
}

public sealed partial class FieldDef
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("constraints")]
    public FieldConstraints? Constraints { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("key")]
    public string Key { get; set; } = default!;

    [JsonPropertyName("label")]
    public string Label { get; set; } = default!;

    [JsonPropertyName("type")]
    public FieldType Type { get; set; }
}

public sealed partial class FieldOption
{
    [JsonPropertyName("label")]
    public string Label { get; set; } = default!;

    [JsonPropertyName("value")]
    public string Value { get; set; } = default!;
}

public enum FieldType
{
    Text,
    Number,
    Boolean,
    Date,
    Enum,
    EntityReference,
    EntityReferenceList,
    Json,
}

internal sealed class FieldTypeConverter : JsonConverter<FieldType>
{
    public override FieldType Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "text" => FieldType.Text,
        "number" => FieldType.Number,
        "boolean" => FieldType.Boolean,
        "date" => FieldType.Date,
        "enum" => FieldType.Enum,
        "entity-reference" => FieldType.EntityReference,
        "entity-reference-list" => FieldType.EntityReferenceList,
        "json" => FieldType.Json,
        var other => throw new JsonException($"Unknown FieldType '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, FieldType value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        FieldType.Text => "text",
        FieldType.Number => "number",
        FieldType.Boolean => "boolean",
        FieldType.Date => "date",
        FieldType.Enum => "enum",
        FieldType.EntityReference => "entity-reference",
        FieldType.EntityReferenceList => "entity-reference-list",
        FieldType.Json => "json",
        _ => throw new JsonException($"Unknown FieldType {value}."),
    });
}

public sealed partial class GroupCoveragePayload
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("claim")]
    public Claim? Claim { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("claimKey")]
    public string? ClaimKey { get; set; }

    [JsonPropertyName("group")]
    public string Group { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

public sealed partial class GroupCoverageResult
{
    [JsonPropertyName("group")]
    public string Group { get; set; } = default!;

    /// <summary>
    /// The group's own Holding, if the group itself holds the claim.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("groupHolding")]
    public WhoHoldsItem? GroupHolding { get; set; }

    /// <summary>
    /// Members holding the claim.
    /// </summary>
    [JsonPropertyName("holding")]
    public List<WhoHoldsItem> Holding { get; set; } = new();

    [JsonPropertyName("members")]
    public List<string> Members { get; set; } = new();

    /// <summary>
    /// Members holding nothing about the claim.
    /// </summary>
    [JsonPropertyName("missing")]
    public List<string> Missing { get; set; } = new();
}

public sealed partial class HolderAt
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    [JsonPropertyName("holder")]
    public string Holder { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

/// <summary>
/// A new value for an active Holding. The old Holding is superseded by a copy
/// carrying the changed fields; its history stays readable.
/// </summary>
public sealed partial class HoldingAdjustment
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("accessibility")]
    public Accessibility? Accessibility { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("attitude")]
    public Attitude? Attitude { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("confidence")]
    public double? Confidence { get; set; }

    /// <summary>
    /// Id of the replacing Holding. Generated when absent.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("newRecordId")]
    public string? NewRecordId { get; set; }

    [JsonPropertyName("recordId")]
    public string RecordId { get; set; } = default!;
}

public sealed partial class HoldingFilter
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("about")]
    public string? About { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("attitude")]
    public List<Attitude>? Attitude { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("includeWithheld")]
    public bool? IncludeWithheld { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("minConfidence")]
    public double? MinConfidence { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("predicate")]
    public string? Predicate { get; set; }
}

public sealed partial class HoldingItem
{
    [JsonPropertyName("acquisition")]
    public Acquisition Acquisition { get; set; } = default!;

    [JsonPropertyName("record")]
    public Record Record { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("status")]
    public TransitionStatus? Status { get; set; }
}

public sealed partial class HoldingsPayload
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("filter")]
    public HoldingFilter? Filter { get; set; }

    [JsonPropertyName("holder")]
    public string Holder { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("page")]
    public Page? Page { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

public sealed partial class HoldingsResult
{
    [JsonPropertyName("items")]
    public List<HoldingItem> Items { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("nextCursor")]
    public string? NextCursor { get; set; }

    [JsonPropertyName("withheldCount")]
    public ulong WithheldCount { get; set; }
}

/// <summary>
/// Whether the Holding behind a `holds` answer has the polarity that was
/// asked about. A holder who believes the opposite still holds the claim.
/// </summary>
public enum HoldsAs
{
    /// <summary>
    /// The Holding has the polarity of the claim asked about.
    /// </summary>
    Same,
    /// <summary>
    /// The Holding has the opposite polarity: the holder holds the negation.
    /// </summary>
    Opposite,
}

internal sealed class HoldsAsConverter : JsonConverter<HoldsAs>
{
    public override HoldsAs Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "same" => HoldsAs.Same,
        "opposite" => HoldsAs.Opposite,
        var other => throw new JsonException($"Unknown HoldsAs '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, HoldsAs value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        HoldsAs.Same => "same",
        HoldsAs.Opposite => "opposite",
        _ => throw new JsonException($"Unknown HoldsAs {value}."),
    });
}

public sealed partial class HoldsPayload
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("claim")]
    public Claim? Claim { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("claimKey")]
    public string? ClaimKey { get; set; }

    [JsonPropertyName("holder")]
    public string Holder { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

public sealed partial class HoldsResult
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("attitude")]
    public Attitude? Attitude { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("confidence")]
    public double? Confidence { get; set; }

    /// <summary>
    /// The holder has a Holding about the claim, with either polarity.
    /// </summary>
    [JsonPropertyName("holds")]
    public bool Holds { get; set; }

    /// <summary>
    /// Whether that Holding affirms what was asked (`same`) or its negation
    /// (`opposite`). Absent when `holds` is false or the question gave only
    /// a `claimKey`, which has no polarity.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("holdsAs")]
    public HoldsAs? HoldsAs { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("polarity")]
    public bool? Polarity { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("recordId")]
    public string? RecordId { get; set; }
}

public enum Intent
{
    Honest,
    Deceptive,
    Unknown,
}

internal sealed class IntentConverter : JsonConverter<Intent>
{
    public override Intent Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "honest" => Intent.Honest,
        "deceptive" => Intent.Deceptive,
        "unknown" => Intent.Unknown,
        var other => throw new JsonException($"Unknown Intent '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, Intent value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        Intent.Honest => "honest",
        Intent.Deceptive => "deceptive",
        Intent.Unknown => "unknown",
        _ => throw new JsonException($"Unknown Intent {value}."),
    });
}

public enum ObjectKind
{
    Entity,
    Value,
    None,
}

internal sealed class ObjectKindConverter : JsonConverter<ObjectKind>
{
    public override ObjectKind Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "entity" => ObjectKind.Entity,
        "value" => ObjectKind.Value,
        "none" => ObjectKind.None,
        var other => throw new JsonException($"Unknown ObjectKind '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, ObjectKind value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        ObjectKind.Entity => "entity",
        ObjectKind.Value => "value",
        ObjectKind.None => "none",
        _ => throw new JsonException($"Unknown ObjectKind {value}."),
    });
}

public sealed partial class OpenDraftInput
{
    /// <summary>
    /// Timeline to branch from: "main" (the default) or an active draft.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("baseTimelineId")]
    public string? BaseTimelineId { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("id")]
    public string? Id { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("label")]
    public string? Label { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;
}

public sealed partial class OperationCheck
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("error")]
    public OperationError? Error { get; set; }

    /// <summary>
    /// Index in the proposal: records first, then transitions.
    /// </summary>
    [JsonPropertyName("operationIndex")]
    public ulong OperationIndex { get; set; }

    [JsonPropertyName("valid")]
    public bool Valid { get; set; }
}

public sealed partial class OperationError
{
    [JsonPropertyName("code")]
    public string Code { get; set; } = default!;

    [JsonPropertyName("message")]
    public string Message { get; set; } = default!;
}

public abstract class Origin
{
    /// <summary>The variant tag.</summary>
    [JsonPropertyName("kind")]
    public abstract string Kind { get; }

    public sealed class Human : Origin
    {
        [JsonPropertyName("kind")]
        public override string Kind => "human";

        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("actorId")]
        public string? ActorId { get; set; }
    }

    public sealed class ModelOrigin : Origin
    {
        [JsonPropertyName("kind")]
        public override string Kind => "model";

        [JsonPropertyName("model")]
        public string Model { get; set; } = default!;

        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("promptHash")]
        public string? PromptHash { get; set; }
    }

    public sealed class System : Origin
    {
        [JsonPropertyName("kind")]
        public override string Kind => "system";

        [JsonPropertyName("name")]
        public string Name { get; set; } = default!;
    }

    public sealed class Import : Origin
    {
        [JsonPropertyName("kind")]
        public override string Kind => "import";

    }
}

internal sealed class OriginConverter : JsonConverter<Origin>
{
    public override Origin Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var document = JsonDocument.ParseValue(ref reader);
        var kind = document.RootElement.TryGetProperty("kind", out var tag) ? tag.GetString() : null;
        return kind switch
        {
            "human" => document.RootElement.Deserialize<Origin.Human>(options)!,
            "model" => document.RootElement.Deserialize<Origin.ModelOrigin>(options)!,
            "system" => document.RootElement.Deserialize<Origin.System>(options)!,
            "import" => document.RootElement.Deserialize<Origin.Import>(options)!,
            _ => throw new JsonException($"Unknown Origin kind '{kind}'."),
        };
    }

    public override void Write(Utf8JsonWriter writer, Origin value, JsonSerializerOptions options) => JsonSerializer.Serialize(writer, value, value.GetType(), options);
}

public sealed partial class Page
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("cursor")]
    public string? Cursor { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("limit")]
    public uint? Limit { get; set; }
}

public sealed partial class PagedProject
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("page")]
    public Page? Page { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;
}

public sealed partial class PendingEmbedding
{
    [JsonPropertyName("docId")]
    public string DocId { get; set; } = default!;

    [JsonPropertyName("docType")]
    public string DocType { get; set; } = default!;

    /// <summary>
    /// A vector exists but was made from another text.
    /// </summary>
    [JsonPropertyName("stale")]
    public bool Stale { get; set; }

    /// <summary>
    /// The text the search indexes for this document: embed exactly this.
    /// </summary>
    [JsonPropertyName("text")]
    public string Text { get; set; } = default!;

    /// <summary>
    /// SHA-256 of `text`, to send back with the vector.
    /// </summary>
    [JsonPropertyName("textHash")]
    public string TextHash { get; set; } = default!;
}

/// <summary>
/// Documents visible in a timeline whose vector in a space is missing or
/// stale, with the text to embed.
/// </summary>
public sealed partial class PendingEmbeddingsInput
{
    /// <summary>
    /// `record` and/or `event`; empty for both.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("docTypes")]
    public List<string>? DocTypes { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("page")]
    public Page? Page { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonPropertyName("spaceId")]
    public string SpaceId { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

public sealed partial class PendingEmbeddingsResult
{
    [JsonPropertyName("items")]
    public List<PendingEmbedding> Items { get; set; } = new();

    /// <summary>
    /// More pending documents follow; pass it as `page.cursor`.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("nextCursor")]
    public string? NextCursor { get; set; }

    /// <summary>
    /// Where a later call can start to see only documents written after this
    /// one. Starting without a cursor checks every document again.
    /// </summary>
    [JsonPropertyName("resumeCursor")]
    public string ResumeCursor { get; set; } = default!;
}

public sealed partial class PerspectivePayload
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    [JsonPropertyName("holder")]
    public string Holder { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("page")]
    public Page? Page { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

public sealed partial class PerspectiveResult
{
    [JsonPropertyName("awareness")]
    public List<Record> Awareness { get; set; } = new();

    [JsonPropertyName("dispositions")]
    public List<Record> Dispositions { get; set; } = new();

    [JsonPropertyName("holder")]
    public Entity Holder { get; set; } = default!;

    [JsonPropertyName("holdings")]
    public List<HoldingItem> Holdings { get; set; } = new();

    [JsonPropertyName("memberships")]
    public List<Record> Memberships { get; set; } = new();

    [JsonPropertyName("withheldCount")]
    public ulong WithheldCount { get; set; }
}

public sealed partial class Position
{
    [JsonPropertyName("tick")]
    public long Tick { get; set; }
}

public sealed partial class PredicateDef
{
    [JsonPropertyName("cardinality")]
    public Cardinality Cardinality { get; set; }

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("label")]
    public string Label { get; set; } = default!;

    [JsonPropertyName("object")]
    public ObjectKind Object { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("subjectTypes")]
    public List<string>? SubjectTypes { get; set; }
}

public sealed partial class Project
{
    [JsonPropertyName("createdAt")]
    public string CreatedAt { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("name")]
    public string Name { get; set; } = default!;

    [JsonPropertyName("timescale")]
    public string Timescale { get; set; } = default!;

    [JsonPropertyName("version")]
    public ulong Version { get; set; }

    [JsonPropertyName("vocabularyMode")]
    public VocabularyMode VocabularyMode { get; set; }
}

public sealed partial class ProjectId
{
    [JsonPropertyName("projectId")]
    public string ProjectIdValue { get; set; } = default!;
}

public sealed partial class Proposal
{
    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("input")]
    public ProposeInput Input { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("rejectionRationale")]
    public string? RejectionRationale { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("reviews")]
    public List<ProposalReview>? Reviews { get; set; }

    [JsonPropertyName("status")]
    public ProposalStatus Status { get; set; }
}

public sealed partial class ProposalId
{
    [JsonPropertyName("proposalId")]
    public string ProposalIdValue { get; set; } = default!;
}

/// <summary>
/// A pending proposal checked again against the current head of its timeline.
/// </summary>
public sealed partial class ProposalRebaseResult
{
    [JsonPropertyName("headVersion")]
    public ulong HeadVersion { get; set; }

    [JsonPropertyName("operations")]
    public List<OperationCheck> Operations { get; set; } = new();

    [JsonPropertyName("proposal")]
    public Proposal Proposal { get; set; } = default!;
}

public sealed partial class ProposalReview
{
    [JsonPropertyName("createdAt")]
    public string CreatedAt { get; set; } = default!;

    [JsonPropertyName("decision")]
    public ReviewDecision Decision { get; set; }

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("operationIndex")]
    public ulong OperationIndex { get; set; }

    [JsonPropertyName("rationale")]
    public string Rationale { get; set; } = default!;
}

public enum ProposalStatus
{
    Pending,
    Applied,
    PartiallyApplied,
    Rejected,
}

internal sealed class ProposalStatusConverter : JsonConverter<ProposalStatus>
{
    public override ProposalStatus Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "pending" => ProposalStatus.Pending,
        "applied" => ProposalStatus.Applied,
        "partially-applied" => ProposalStatus.PartiallyApplied,
        "rejected" => ProposalStatus.Rejected,
        var other => throw new JsonException($"Unknown ProposalStatus '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, ProposalStatus value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        ProposalStatus.Pending => "pending",
        ProposalStatus.Applied => "applied",
        ProposalStatus.PartiallyApplied => "partially-applied",
        ProposalStatus.Rejected => "rejected",
        _ => throw new JsonException($"Unknown ProposalStatus {value}."),
    });
}

public sealed partial class ProposeInput
{
    [JsonPropertyName("event")]
    public EventInput Event { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("expectedVersion")]
    public ulong? ExpectedVersion { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("records")]
    public List<RecordInput>? Records { get; set; }

    /// <summary>
    /// Timeline to write to; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("transitions")]
    public List<TransitionInput>? Transitions { get; set; }
}

public sealed partial class Provenance
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("metadata")]
    public Dictionary<string, JsonElement>? Metadata { get; set; }

    [JsonPropertyName("origin")]
    public Origin Origin { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("reason")]
    public string? Reason { get; set; }
}

public sealed partial class PurgeCounts
{
    [JsonPropertyName("attempts")]
    public ulong Attempts { get; set; }

    [JsonPropertyName("events")]
    public ulong Events { get; set; }

    [JsonPropertyName("records")]
    public ulong Records { get; set; }

    [JsonPropertyName("transitions")]
    public ulong Transitions { get; set; }
}

public sealed partial class PurgeResult
{
    [JsonPropertyName("purged")]
    public bool Purged { get; set; }

    [JsonPropertyName("removed")]
    public PurgeCounts Removed { get; set; } = default!;
}

/// <summary>
/// Stores or replaces vectors in one transaction.
/// </summary>
public sealed partial class PutEmbeddingsInput
{
    [JsonPropertyName("items")]
    public List<EmbeddingItem> Items { get; set; } = new();

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonPropertyName("spaceId")]
    public string SpaceId { get; set; } = default!;
}

public sealed partial class PutEmbeddingsResult
{
    /// <summary>
    /// Stored vectors whose `textHash` is not the document's: they stay
    /// pending until replaced.
    /// </summary>
    [JsonPropertyName("stale")]
    public ulong Stale { get; set; }

    [JsonPropertyName("stored")]
    public ulong Stored { get; set; }
}

public abstract class Record
{
    /// <summary>The variant tag.</summary>
    [JsonPropertyName("kind")]
    public abstract string Kind { get; }

    public sealed class Fact : Record
    {
        [JsonPropertyName("kind")]
        public override string Kind => "fact";

        [JsonPropertyName("claim")]
        public Claim Claim { get; set; } = default!;

        [JsonPropertyName("eventId")]
        public string EventId { get; set; } = default!;

        [JsonPropertyName("id")]
        public string Id { get; set; } = default!;

        [JsonPropertyName("position")]
        public Position Position { get; set; } = default!;

        [JsonPropertyName("projectId")]
        public string ProjectId { get; set; } = default!;

        [JsonPropertyName("provenance")]
        public Provenance Provenance { get; set; } = default!;

        [JsonPropertyName("recordedAt")]
        public ulong RecordedAt { get; set; }

        [JsonPropertyName("timelineId")]
        public string TimelineId { get; set; } = default!;
    }

    public sealed class Holding : Record
    {
        [JsonPropertyName("kind")]
        public override string Kind => "holding";

        [JsonPropertyName("accessibility")]
        public Accessibility Accessibility { get; set; }

        [JsonPropertyName("acquisition")]
        public Acquisition Acquisition { get; set; } = default!;

        [JsonPropertyName("attitude")]
        public Attitude Attitude { get; set; }

        [JsonPropertyName("claim")]
        public Claim Claim { get; set; } = default!;

        [JsonPropertyName("confidence")]
        public double Confidence { get; set; }

        [JsonPropertyName("eventId")]
        public string EventId { get; set; } = default!;

        [JsonPropertyName("holder")]
        public string Holder { get; set; } = default!;

        [JsonPropertyName("id")]
        public string Id { get; set; } = default!;

        [JsonPropertyName("position")]
        public Position Position { get; set; } = default!;

        [JsonPropertyName("projectId")]
        public string ProjectId { get; set; } = default!;

        [JsonPropertyName("provenance")]
        public Provenance Provenance { get; set; } = default!;

        [JsonPropertyName("recordedAt")]
        public ulong RecordedAt { get; set; }

        /// <summary>
        /// The holder keeps this secret; only `sharedWith` may be told.
        /// </summary>
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("secrecy")]
        public Secrecy? Secrecy { get; set; }

        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("text")]
        public string? Text { get; set; }

        [JsonPropertyName("timelineId")]
        public string TimelineId { get; set; } = default!;
    }

    public sealed class Awareness : Record
    {
        [JsonPropertyName("kind")]
        public override string Kind => "awareness";

        [JsonPropertyName("eventId")]
        public string EventId { get; set; } = default!;

        [JsonPropertyName("holder")]
        public string Holder { get; set; } = default!;

        [JsonPropertyName("id")]
        public string Id { get; set; } = default!;

        [JsonPropertyName("position")]
        public Position Position { get; set; } = default!;

        [JsonPropertyName("projectId")]
        public string ProjectId { get; set; } = default!;

        [JsonPropertyName("provenance")]
        public Provenance Provenance { get; set; } = default!;

        [JsonPropertyName("recordedAt")]
        public ulong RecordedAt { get; set; }

        [JsonPropertyName("target")]
        public string Target { get; set; } = default!;

        [JsonPropertyName("timelineId")]
        public string TimelineId { get; set; } = default!;
    }

    public sealed class Disposition : Record
    {
        [JsonPropertyName("kind")]
        public override string Kind => "disposition";

        [JsonPropertyName("dimension")]
        public string Dimension { get; set; } = default!;

        [JsonPropertyName("eventId")]
        public string EventId { get; set; } = default!;

        [JsonPropertyName("holder")]
        public string Holder { get; set; } = default!;

        [JsonPropertyName("id")]
        public string Id { get; set; } = default!;

        [JsonPropertyName("position")]
        public Position Position { get; set; } = default!;

        [JsonPropertyName("projectId")]
        public string ProjectId { get; set; } = default!;

        [JsonPropertyName("provenance")]
        public Provenance Provenance { get; set; } = default!;

        [JsonPropertyName("recordedAt")]
        public ulong RecordedAt { get; set; }

        [JsonPropertyName("target")]
        public string Target { get; set; } = default!;

        [JsonPropertyName("timelineId")]
        public string TimelineId { get; set; } = default!;

        [JsonPropertyName("value")]
        public JsonElement Value { get; set; }
    }
}

internal sealed class RecordConverter : JsonConverter<Record>
{
    public override Record Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var document = JsonDocument.ParseValue(ref reader);
        var kind = document.RootElement.TryGetProperty("kind", out var tag) ? tag.GetString() : null;
        return kind switch
        {
            "fact" => document.RootElement.Deserialize<Record.Fact>(options)!,
            "holding" => document.RootElement.Deserialize<Record.Holding>(options)!,
            "awareness" => document.RootElement.Deserialize<Record.Awareness>(options)!,
            "disposition" => document.RootElement.Deserialize<Record.Disposition>(options)!,
            _ => throw new JsonException($"Unknown Record kind '{kind}'."),
        };
    }

    public override void Write(Utf8JsonWriter writer, Record value, JsonSerializerOptions options) => JsonSerializer.Serialize(writer, value, value.GetType(), options);
}

public sealed partial class RecordCommon
{
    [JsonPropertyName("eventId")]
    public string EventId { get; set; } = default!;

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("position")]
    public Position Position { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonPropertyName("provenance")]
    public Provenance Provenance { get; set; } = default!;

    [JsonPropertyName("recordedAt")]
    public ulong RecordedAt { get; set; }

    [JsonPropertyName("timelineId")]
    public string TimelineId { get; set; } = default!;
}

public sealed partial class RecordId
{
    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonPropertyName("recordId")]
    public string RecordIdValue { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

public abstract class RecordInput
{
    /// <summary>The variant tag.</summary>
    [JsonPropertyName("kind")]
    public abstract string Kind { get; }

    public sealed class Fact : RecordInput
    {
        [JsonPropertyName("kind")]
        public override string Kind => "fact";

        [JsonPropertyName("claim")]
        public Claim Claim { get; set; } = default!;

        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("id")]
        public string? Id { get; set; }

        /// <summary>
        /// Omitted: the position of the commit's event.
        /// </summary>
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("position")]
        public Position? Position { get; set; }

        /// <summary>
        /// Omitted: the provenance of the commit's event.
        /// </summary>
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("provenance")]
        public Provenance? Provenance { get; set; }
    }

    public sealed class Holding : RecordInput
    {
        [JsonPropertyName("kind")]
        public override string Kind => "holding";

        /// <summary>
        /// Omitted: `available`.
        /// </summary>
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("accessibility")]
        public Accessibility? Accessibility { get; set; }

        [JsonPropertyName("acquisition")]
        public Acquisition Acquisition { get; set; } = default!;

        [JsonPropertyName("attitude")]
        public Attitude Attitude { get; set; }

        [JsonPropertyName("claim")]
        public Claim Claim { get; set; } = default!;

        /// <summary>
        /// Omitted: 1.
        /// </summary>
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("confidence")]
        public double? Confidence { get; set; }

        [JsonPropertyName("holder")]
        public string Holder { get; set; } = default!;

        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("id")]
        public string? Id { get; set; }

        /// <summary>
        /// Omitted: the position of the commit's event.
        /// </summary>
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("position")]
        public Position? Position { get; set; }

        /// <summary>
        /// Omitted: the provenance of the commit's event.
        /// </summary>
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("provenance")]
        public Provenance? Provenance { get; set; }

        /// <summary>
        /// The holder keeps this secret; only `sharedWith` may be told.
        /// </summary>
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("secrecy")]
        public Secrecy? Secrecy { get; set; }

        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("text")]
        public string? Text { get; set; }
    }

    public sealed class Awareness : RecordInput
    {
        [JsonPropertyName("kind")]
        public override string Kind => "awareness";

        [JsonPropertyName("holder")]
        public string Holder { get; set; } = default!;

        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("id")]
        public string? Id { get; set; }

        /// <summary>
        /// Omitted: the position of the commit's event.
        /// </summary>
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("position")]
        public Position? Position { get; set; }

        /// <summary>
        /// Omitted: the provenance of the commit's event.
        /// </summary>
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("provenance")]
        public Provenance? Provenance { get; set; }

        [JsonPropertyName("target")]
        public string Target { get; set; } = default!;
    }

    public sealed class Disposition : RecordInput
    {
        [JsonPropertyName("kind")]
        public override string Kind => "disposition";

        [JsonPropertyName("dimension")]
        public string Dimension { get; set; } = default!;

        [JsonPropertyName("holder")]
        public string Holder { get; set; } = default!;

        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("id")]
        public string? Id { get; set; }

        /// <summary>
        /// Omitted: the position of the commit's event.
        /// </summary>
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("position")]
        public Position? Position { get; set; }

        /// <summary>
        /// Omitted: the provenance of the commit's event.
        /// </summary>
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        [JsonPropertyName("provenance")]
        public Provenance? Provenance { get; set; }

        [JsonPropertyName("target")]
        public string Target { get; set; } = default!;

        [JsonPropertyName("value")]
        public JsonElement Value { get; set; }
    }
}

internal sealed class RecordInputConverter : JsonConverter<RecordInput>
{
    public override RecordInput Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var document = JsonDocument.ParseValue(ref reader);
        var kind = document.RootElement.TryGetProperty("kind", out var tag) ? tag.GetString() : null;
        return kind switch
        {
            "fact" => document.RootElement.Deserialize<RecordInput.Fact>(options)!,
            "holding" => document.RootElement.Deserialize<RecordInput.Holding>(options)!,
            "awareness" => document.RootElement.Deserialize<RecordInput.Awareness>(options)!,
            "disposition" => document.RootElement.Deserialize<RecordInput.Disposition>(options)!,
            _ => throw new JsonException($"Unknown RecordInput kind '{kind}'."),
        };
    }

    public override void Write(Utf8JsonWriter writer, RecordInput value, JsonSerializerOptions options) => JsonSerializer.Serialize(writer, value, value.GetType(), options);
}

public sealed partial class RecordInputCommon
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("id")]
    public string? Id { get; set; }

    /// <summary>
    /// Omitted: the position of the commit's event.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("position")]
    public Position? Position { get; set; }

    /// <summary>
    /// Omitted: the provenance of the commit's event.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("provenance")]
    public Provenance? Provenance { get; set; }
}

public sealed partial class RecordPageResult
{
    [JsonPropertyName("items")]
    public List<Record> Items { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("nextCursor")]
    public string? NextCursor { get; set; }
}

public sealed partial class RecordState
{
    [JsonPropertyName("record")]
    public Record Record { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("transition")]
    public Transition? Transition { get; set; }
}

public sealed partial class RejectPayload
{
    [JsonPropertyName("proposalId")]
    public string ProposalId { get; set; } = default!;

    [JsonPropertyName("rationale")]
    public string Rationale { get; set; } = default!;
}

public sealed partial class ResolveEntitiesPayload
{
    [JsonPropertyName("externalRefs")]
    public List<ExternalRef> ExternalRefs { get; set; } = new();

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;
}

public sealed partial class ResolveEntityPayload
{
    [JsonPropertyName("externalRef")]
    public ExternalRef ExternalRef { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;
}

public sealed partial class ResolvedEntity
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("entityId")]
    public string? EntityId { get; set; }

    [JsonPropertyName("externalRef")]
    public ExternalRef ExternalRef { get; set; } = default!;
}

public enum ReviewDecision
{
    Accepted,
    Rejected,
}

internal sealed class ReviewDecisionConverter : JsonConverter<ReviewDecision>
{
    public override ReviewDecision Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "accepted" => ReviewDecision.Accepted,
        "rejected" => ReviewDecision.Rejected,
        var other => throw new JsonException($"Unknown ReviewDecision '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, ReviewDecision value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        ReviewDecision.Accepted => "accepted",
        ReviewDecision.Rejected => "rejected",
        _ => throw new JsonException($"Unknown ReviewDecision {value}."),
    });
}

public sealed partial class ReviewInput
{
    [JsonPropertyName("decision")]
    public ReviewDecision Decision { get; set; }

    [JsonPropertyName("operationIndex")]
    public ulong OperationIndex { get; set; }

    [JsonPropertyName("rationale")]
    public string Rationale { get; set; } = default!;
}

public sealed partial class ReviewPayload
{
    [JsonPropertyName("proposalId")]
    public string ProposalId { get; set; } = default!;

    [JsonPropertyName("reviews")]
    public List<ReviewInput> Reviews { get; set; } = new();
}

public sealed partial class SearchFilter
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("docType")]
    public string? DocType { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("holder")]
    public string? Holder { get; set; }
}

public sealed partial class SearchItem
{
    [JsonPropertyName("docId")]
    public string DocId { get; set; } = default!;

    [JsonPropertyName("docType")]
    public string DocType { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("event")]
    public Event? Event { get; set; }

    /// <summary>
    /// Which search found the item.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("match")]
    public SearchMatch? Match { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("record")]
    public Record? Record { get; set; }

    /// <summary>
    /// The value the item was ranked by: the cosine similarity in a semantic
    /// search, the fused rank score in a hybrid one. Absent in a text search.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("score")]
    public double? Score { get; set; }
}

public enum SearchMatch
{
    Text,
    Semantic,
    Both,
}

internal sealed class SearchMatchConverter : JsonConverter<SearchMatch>
{
    public override SearchMatch Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "text" => SearchMatch.Text,
        "semantic" => SearchMatch.Semantic,
        "both" => SearchMatch.Both,
        var other => throw new JsonException($"Unknown SearchMatch '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, SearchMatch value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        SearchMatch.Text => "text",
        SearchMatch.Semantic => "semantic",
        SearchMatch.Both => "both",
        _ => throw new JsonException($"Unknown SearchMatch {value}."),
    });
}

public sealed partial class SearchPageResult
{
    [JsonPropertyName("items")]
    public List<SearchItem> Items { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("nextCursor")]
    public string? NextCursor { get; set; }
}

public sealed partial class SearchPayload
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("filter")]
    public SearchFilter? Filter { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("page")]
    public Page? Page { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// A query vector from the client's model.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("semantic")]
    public SearchSemantic? Semantic { get; set; }

    /// <summary>
    /// Full-text query. At least one of `text` and `semantic` is required;
    /// with both, the search is hybrid.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("text")]
    public string? Text { get; set; }

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

/// <summary>
/// The semantic part of a search: a query vector made by the client's model
/// in one of the project's embedding spaces.
/// </summary>
public sealed partial class SearchSemantic
{
    [JsonPropertyName("spaceId")]
    public string SpaceId { get; set; } = default!;

    [JsonPropertyName("vector")]
    public List<double> Vector { get; set; } = new();
}

/// <summary>
/// Marks a Holding as a secret. The engine never blocks telling it: telling
/// anyone outside `sharedWith` (besides the holder) is recorded and reported
/// as a breach.
/// </summary>
public sealed partial class Secrecy
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("sharedWith")]
    public List<string>? SharedWith { get; set; }
}

/// <summary>
/// Records many Disposition values in one commit (trust, affection...). The
/// newest value per holder, target and dimension is the current one.
/// </summary>
public sealed partial class SetDispositionsInput
{
    [JsonPropertyName("attemptId")]
    public string AttemptId { get; set; } = default!;

    [JsonPropertyName("dispositions")]
    public List<DispositionSetting> Dispositions { get; set; } = new();

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("expectedVersion")]
    public ulong? ExpectedVersion { get; set; }

    /// <summary>
    /// Positions of the generated event on additional axes.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("on")]
    public Dictionary<string, long>? On { get; set; }

    [JsonPropertyName("position")]
    public Position Position { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("provenance")]
    public Provenance? Provenance { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

/// <summary>
/// A line of history. "main" is the confirmed history of a project; a draft
/// branches from its parent (main or another draft) at `baseVersion` and is
/// later promoted into that parent or discarded.
/// </summary>
public sealed partial class Timeline
{
    /// <summary>
    /// Head version of the parent when the draft was opened. Absent for main.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("baseVersion")]
    public ulong? BaseVersion { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("closedAt")]
    public string? ClosedAt { get; set; }

    [JsonPropertyName("createdAt")]
    public string CreatedAt { get; set; } = default!;

    /// <summary>
    /// Drafts from main down to this timeline: 0 for main, 1 for a draft of
    /// main, at most `MAX_DRAFT_DEPTH`.
    /// </summary>
    [JsonPropertyName("depth")]
    public uint Depth { get; set; }

    [JsonPropertyName("headVersion")]
    public ulong HeadVersion { get; set; }

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("kind")]
    public TimelineKind Kind { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("label")]
    public string? Label { get; set; }

    [JsonPropertyName("maxTick")]
    public long MaxTick { get; set; }

    /// <summary>
    /// Timeline this draft branched from: "main" or another draft. Absent for
    /// main.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("parentTimelineId")]
    public string? ParentTimelineId { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("purgedAt")]
    public string? PurgedAt { get; set; }

    [JsonPropertyName("status")]
    public TimelineStatus Status { get; set; }
}

public enum TimelineKind
{
    Main,
    Draft,
}

internal sealed class TimelineKindConverter : JsonConverter<TimelineKind>
{
    public override TimelineKind Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "main" => TimelineKind.Main,
        "draft" => TimelineKind.Draft,
        var other => throw new JsonException($"Unknown TimelineKind '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, TimelineKind value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        TimelineKind.Main => "main",
        TimelineKind.Draft => "draft",
        _ => throw new JsonException($"Unknown TimelineKind {value}."),
    });
}

public sealed partial class TimelineListPayload
{
    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("status")]
    public TimelineStatus? Status { get; set; }
}

public sealed partial class TimelinePayload
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("page")]
    public Page? Page { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

public sealed partial class TimelineRef
{
    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonPropertyName("timelineId")]
    public string TimelineId { get; set; } = default!;
}

public enum TimelineStatus
{
    Active,
    Promoted,
    Discarded,
}

internal sealed class TimelineStatusConverter : JsonConverter<TimelineStatus>
{
    public override TimelineStatus Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "active" => TimelineStatus.Active,
        "promoted" => TimelineStatus.Promoted,
        "discarded" => TimelineStatus.Discarded,
        var other => throw new JsonException($"Unknown TimelineStatus '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, TimelineStatus value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        TimelineStatus.Active => "active",
        TimelineStatus.Promoted => "promoted",
        TimelineStatus.Discarded => "discarded",
        _ => throw new JsonException($"Unknown TimelineStatus {value}."),
    });
}

public sealed partial class TraceResult
{
    [JsonPropertyName("chain")]
    public List<ChainLink> Chain { get; set; } = new();

    [JsonPropertyName("event")]
    public Event Event { get; set; } = default!;

    [JsonPropertyName("record")]
    public Record Record { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("status")]
    public TransitionStatus? Status { get; set; }

    [JsonPropertyName("transitions")]
    public List<Transition> Transitions { get; set; } = new();

    [JsonPropertyName("truncated")]
    public bool Truncated { get; set; }
}

public sealed partial class Transition
{
    [JsonPropertyName("eventId")]
    public string EventId { get; set; } = default!;

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("ordinal")]
    public uint Ordinal { get; set; }

    [JsonPropertyName("position")]
    public Position Position { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonPropertyName("provenance")]
    public Provenance Provenance { get; set; } = default!;

    [JsonPropertyName("recordedAt")]
    public ulong RecordedAt { get; set; }

    [JsonPropertyName("status")]
    public TransitionStatus Status { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("supersededBy")]
    public string? SupersededBy { get; set; }

    [JsonPropertyName("targetRecordId")]
    public string TargetRecordId { get; set; } = default!;

    [JsonPropertyName("timelineId")]
    public string TimelineId { get; set; } = default!;
}

/// <summary>
/// One status change applied to many records in one commit, e.g. everything a
/// character forgets overnight. `superseded` needs a replacement and is not
/// accepted here; use `holding.adjustBatch`.
/// </summary>
public sealed partial class TransitionBatchInput
{
    [JsonPropertyName("attemptId")]
    public string AttemptId { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("expectedVersion")]
    public ulong? ExpectedVersion { get; set; }

    /// <summary>
    /// Positions of the generated event on additional axes.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("on")]
    public Dictionary<string, long>? On { get; set; }

    [JsonPropertyName("position")]
    public Position Position { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("provenance")]
    public Provenance? Provenance { get; set; }

    [JsonPropertyName("recordIds")]
    public List<string> RecordIds { get; set; } = new();

    [JsonPropertyName("status")]
    public BatchTransitionStatus Status { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

public sealed partial class TransitionInput
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("id")]
    public string? Id { get; set; }

    /// <summary>
    /// Omitted: the position of the commit's event.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("position")]
    public Position? Position { get; set; }

    /// <summary>
    /// Omitted: the provenance of the commit's event.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("provenance")]
    public Provenance? Provenance { get; set; }

    [JsonPropertyName("status")]
    public TransitionStatus Status { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("supersededBy")]
    public string? SupersededBy { get; set; }

    [JsonPropertyName("targetRecordId")]
    public string TargetRecordId { get; set; } = default!;
}

public enum TransitionStatus
{
    Retracted,
    Revoked,
    Forgotten,
    Abandoned,
    Superseded,
}

internal sealed class TransitionStatusConverter : JsonConverter<TransitionStatus>
{
    public override TransitionStatus Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "retracted" => TransitionStatus.Retracted,
        "revoked" => TransitionStatus.Revoked,
        "forgotten" => TransitionStatus.Forgotten,
        "abandoned" => TransitionStatus.Abandoned,
        "superseded" => TransitionStatus.Superseded,
        var other => throw new JsonException($"Unknown TransitionStatus '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, TransitionStatus value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        TransitionStatus.Retracted => "retracted",
        TransitionStatus.Revoked => "revoked",
        TransitionStatus.Forgotten => "forgotten",
        TransitionStatus.Abandoned => "abandoned",
        TransitionStatus.Superseded => "superseded",
        _ => throw new JsonException($"Unknown TransitionStatus {value}."),
    });
}

/// <summary>
/// One transmit of a batch: a `TransmitInput` whose shareable fields may be
/// left to the batch's `shared`.
/// </summary>
public sealed partial class TransmitBatchItem
{
    [JsonPropertyName("attemptId")]
    public string AttemptId { get; set; } = default!;

    /// <summary>
    /// Neither here nor in `shared`: `believes`.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("attitude")]
    public Attitude? Attitude { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("confidence")]
    public double? Confidence { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("distortedClaim")]
    public Claim? DistortedClaim { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("from")]
    public string? From { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("intent")]
    public Intent? Intent { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("on")]
    public Dictionary<string, long>? On { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("position")]
    public Position? Position { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("projectId")]
    public string? ProjectId { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("recordId")]
    public string? RecordId { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("sourceRecordId")]
    public string? SourceRecordId { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("text")]
    public string? Text { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("to")]
    public string? To { get; set; }
}

public sealed partial class TransmitBatchPayload
{
    /// <summary>
    /// What the transmits have in common, written once.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("shared")]
    public TransmitShared? Shared { get; set; }

    [JsonPropertyName("transmits")]
    public List<TransmitBatchItem> Transmits { get; set; } = new();
}

public sealed partial class TransmitInput
{
    [JsonPropertyName("attemptId")]
    public string AttemptId { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("attitude")]
    public Attitude? Attitude { get; set; }

    [JsonPropertyName("confidence")]
    public double Confidence { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("distortedClaim")]
    public Claim? DistortedClaim { get; set; }

    [JsonPropertyName("from")]
    public string From { get; set; } = default!;

    [JsonPropertyName("intent")]
    public Intent Intent { get; set; }

    /// <summary>
    /// Positions of the generated event on additional axes.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("on")]
    public Dictionary<string, long>? On { get; set; }

    [JsonPropertyName("position")]
    public Position Position { get; set; } = default!;

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("recordId")]
    public string? RecordId { get; set; }

    [JsonPropertyName("sourceRecordId")]
    public string SourceRecordId { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("text")]
    public string? Text { get; set; }

    /// <summary>
    /// Timeline to write to; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }

    [JsonPropertyName("to")]
    public string To { get; set; } = default!;
}

/// <summary>
/// What every transmit of a batch has in common, written once. A field given
/// by an item wins over the shared one.
/// </summary>
public sealed partial class TransmitShared
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("attitude")]
    public Attitude? Attitude { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("confidence")]
    public double? Confidence { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("from")]
    public string? From { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("intent")]
    public Intent? Intent { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("on")]
    public Dictionary<string, long>? On { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("position")]
    public Position? Position { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("projectId")]
    public string? ProjectId { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("sourceRecordId")]
    public string? SourceRecordId { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("to")]
    public string? To { get; set; }
}

public sealed partial class UpdateEntityInput
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("aliases")]
    public List<string>? Aliases { get; set; }

    [JsonPropertyName("entityId")]
    public string EntityId { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("externalRef")]
    public ExternalRef? ExternalRef { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("fields")]
    public Dictionary<string, JsonElement>? Fields { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("name")]
    public string? Name { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;
}

public sealed partial class UtteranceCheck
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("attitude")]
    public Attitude? Attitude { get; set; }

    [JsonPropertyName("canon")]
    public CanonStatus Canon { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("canonRecordId")]
    public string? CanonRecordId { get; set; }

    [JsonPropertyName("claimKey")]
    public string ClaimKey { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("confidence")]
    public double? Confidence { get; set; }

    /// <summary>
    /// The Holding that decides the verdict.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("recordId")]
    public string? RecordId { get; set; }

    [JsonPropertyName("verdict")]
    public UtteranceVerdict Verdict { get; set; }
}

/// <summary>
/// How a claim relates to what the holder holds at the read position.
/// </summary>
public enum UtteranceVerdict
{
    /// <summary>
    /// The holder holds the claim (see `attitude`).
    /// </summary>
    Holds,
    /// <summary>
    /// The holder holds the claim with the opposite polarity.
    /// </summary>
    HoldsOpposite,
    /// <summary>
    /// The holder holds another value in the same functional slot.
    /// </summary>
    HoldsOtherValue,
    /// <summary>
    /// The holder holds it, but suppressed or inaccessible.
    /// </summary>
    Withheld,
    /// <summary>
    /// The holder only comes to hold it at a later tick (on the read's axis):
    /// an anachronism.
    /// </summary>
    Future,
    /// <summary>
    /// Nothing the holder holds covers the claim.
    /// </summary>
    Unknown,
}

internal sealed class UtteranceVerdictConverter : JsonConverter<UtteranceVerdict>
{
    public override UtteranceVerdict Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "holds" => UtteranceVerdict.Holds,
        "holds-opposite" => UtteranceVerdict.HoldsOpposite,
        "holds-other-value" => UtteranceVerdict.HoldsOtherValue,
        "withheld" => UtteranceVerdict.Withheld,
        "future" => UtteranceVerdict.Future,
        "unknown" => UtteranceVerdict.Unknown,
        var other => throw new JsonException($"Unknown UtteranceVerdict '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, UtteranceVerdict value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        UtteranceVerdict.Holds => "holds",
        UtteranceVerdict.HoldsOpposite => "holds-opposite",
        UtteranceVerdict.HoldsOtherValue => "holds-other-value",
        UtteranceVerdict.Withheld => "withheld",
        UtteranceVerdict.Future => "future",
        UtteranceVerdict.Unknown => "unknown",
        _ => throw new JsonException($"Unknown UtteranceVerdict {value}."),
    });
}

public enum VocabularyMode
{
    Strict,
    Open,
}

internal sealed class VocabularyModeConverter : JsonConverter<VocabularyMode>
{
    public override VocabularyMode Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString() switch
    {
        "strict" => VocabularyMode.Strict,
        "open" => VocabularyMode.Open,
        var other => throw new JsonException($"Unknown VocabularyMode '{other}'."),
    };

    public override void Write(Utf8JsonWriter writer, VocabularyMode value, JsonSerializerOptions options) => writer.WriteStringValue(value switch
    {
        VocabularyMode.Strict => "strict",
        VocabularyMode.Open => "open",
        _ => throw new JsonException($"Unknown VocabularyMode {value}."),
    });
}

public sealed partial class Warning
{
    [JsonPropertyName("code")]
    public string Code { get; set; } = default!;

    [JsonPropertyName("message")]
    public string Message { get; set; } = default!;
}

public sealed partial class WhoHoldsFilter
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("attitude")]
    public List<Attitude>? Attitude { get; set; }

    /// <summary>
    /// Only holders that are members of this group at the read position
    /// (an active `umwelt:member-of` Fact).
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("memberOf")]
    public string? MemberOf { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("polarity")]
    public bool? Polarity { get; set; }
}

public sealed partial class WhoHoldsItem
{
    [JsonPropertyName("attitude")]
    public Attitude Attitude { get; set; }

    [JsonPropertyName("confidence")]
    public double Confidence { get; set; }

    [JsonPropertyName("holder")]
    public string Holder { get; set; } = default!;

    [JsonPropertyName("polarity")]
    public bool Polarity { get; set; }

    [JsonPropertyName("recordId")]
    public string RecordId { get; set; } = default!;
}

public sealed partial class WhoHoldsPayload
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("at")]
    public At? At { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("claim")]
    public Claim? Claim { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("claimKey")]
    public string? ClaimKey { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("filter")]
    public WhoHoldsFilter? Filter { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    /// <summary>
    /// Timeline to read; defaults to "main".
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }
}

public sealed partial class Witness
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("accessibility")]
    public Accessibility? Accessibility { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("attitude")]
    public Attitude? Attitude { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("confidence")]
    public double? Confidence { get; set; }

    [JsonPropertyName("holder")]
    public string Holder { get; set; } = default!;

    /// <summary>
    /// One Holding id per claim, in claim order. Empty: generated.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("recordIds")]
    public List<string>? RecordIds { get; set; }
}

/// <summary>
/// One event seen by several holders: every witness gets a `witnessed`
/// Holding for every claim, in one atomic commit. Who saw what, and how
/// clearly, is the client's decision.
/// </summary>
public sealed partial class WitnessInput
{
    [JsonPropertyName("attemptId")]
    public string AttemptId { get; set; } = default!;

    [JsonPropertyName("claims")]
    public List<WitnessedClaim> Claims { get; set; } = new();

    [JsonPropertyName("event")]
    public EventInput Event { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("expectedVersion")]
    public ulong? ExpectedVersion { get; set; }

    [JsonPropertyName("projectId")]
    public string ProjectId { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("timelineId")]
    public string? TimelineId { get; set; }

    [JsonPropertyName("witnesses")]
    public List<Witness> Witnesses { get; set; } = new();
}

/// <summary>
/// One claim seen at a witnessed event.
/// </summary>
public sealed partial class WitnessedClaim
{
    /// <summary>
    /// Also record the claim as canon (a Fact) in the same commit.
    /// </summary>
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("canon")]
    public bool? Canon { get; set; }

    [JsonPropertyName("claim")]
    public Claim Claim { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("factId")]
    public string? FactId { get; set; }
}

/// <summary>
/// What a manifest created; everything else was already there.
/// </summary>
public sealed partial class WorldCreated
{
    [JsonPropertyName("axes")]
    public List<string> Axes { get; set; } = new();

    [JsonPropertyName("entities")]
    public List<string> Entities { get; set; } = new();

    [JsonPropertyName("entityTypes")]
    public List<string> EntityTypes { get; set; } = new();

    [JsonPropertyName("predicates")]
    public List<string> Predicates { get; set; } = new();

    [JsonPropertyName("project")]
    public bool Project { get; set; }
}

/// <summary>
/// One thing a manifest asks for that already exists with another definition.
/// </summary>
public sealed partial class WorldDivergence
{
    /// <summary>
    /// The field whose stored value differs.
    /// </summary>
    [JsonPropertyName("field")]
    public string Field { get; set; } = default!;

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    /// <summary>
    /// `project`, `entityType`, `entity`, `predicate` or `axis`.
    /// </summary>
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = default!;
}

public sealed partial class WorldEntity
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("aliases")]
    public List<string>? Aliases { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("createdAt")]
    public Position? CreatedAt { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("externalRef")]
    public ExternalRef? ExternalRef { get; set; }

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("fields")]
    public Dictionary<string, JsonElement>? Fields { get; set; }

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("name")]
    public string Name { get; set; } = default!;

    [JsonPropertyName("typeId")]
    public string TypeId { get; set; } = default!;
}

public sealed partial class WorldEntityType
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("fields")]
    public List<FieldDef>? Fields { get; set; }

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("kind")]
    public EntityKind Kind { get; set; }

    [JsonPropertyName("name")]
    public string Name { get; set; } = default!;
}

/// <summary>
/// The project of a world manifest. Unlike `project.create`, the id is
/// required: it is what makes the manifest repeatable.
/// </summary>
public sealed partial class WorldProject
{
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonPropertyName("id")]
    public string Id { get; set; } = default!;

    [JsonPropertyName("name")]
    public string Name { get; set; } = default!;

    [JsonPropertyName("timescale")]
    public string Timescale { get; set; } = default!;

    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    [JsonPropertyName("vocabularyMode")]
    public VocabularyMode? VocabularyMode { get; set; }
}

internal static partial class UmweltJson
{
    private static IEnumerable<JsonConverter> GeneratedConverters() => new JsonConverter[]
    {
        new AccessibilityConverter(),
        new AcquisitionConverter(),
        new AttitudeConverter(),
        new BatchTransitionStatusConverter(),
        new CanonStatusConverter(),
        new CardinalityConverter(),
        new ClaimObjectConverter(),
        new ContextRejectionConverter(),
        new ContradictionKindConverter(),
        new ContradictionReasonConverter(),
        new EntityKindConverter(),
        new FieldTypeConverter(),
        new HoldsAsConverter(),
        new IntentConverter(),
        new ObjectKindConverter(),
        new OriginConverter(),
        new ProposalStatusConverter(),
        new RecordConverter(),
        new RecordInputConverter(),
        new ReviewDecisionConverter(),
        new SearchMatchConverter(),
        new TimelineKindConverter(),
        new TimelineStatusConverter(),
        new TransitionStatusConverter(),
        new UtteranceVerdictConverter(),
        new VocabularyModeConverter(),
    };
}
