// SPDX-License-Identifier: AGPL-3.0-only
// Factories for the write contract. Each one is pure and returns a plain
// contract object: no state, no generated ids, no call to the engine. What is
// not passed stays null, is not sent, and takes the engine's documented write
// default (a record's position and provenance come from the commit's event; a
// Holding defaults to confidence 1 and Available).
//
// Every factory has a line in conformance/builders.json, which the npm
// package checks too, so both languages build the same JSON.
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace Kallamo.Umwelt;

public sealed partial class Claim
{
    /// <summary><c>subject predicate entity</c>, e.g. <c>Claim.Entity("bob", "stole", "necklace")</c>.</summary>
    public static Claim Entity(string subject, string predicate, string objectId, bool polarity = true, IReadOnlyDictionary<string, object?>? qualifiers = null, string? about = null) =>
        Create(subject, predicate, new ClaimObject.Entity { Id = objectId }, polarity, qualifiers, about);

    /// <summary><c>subject predicate value</c>, e.g. <c>Claim.Value("bob", "age", 42)</c>. The value is any JSON-serializable object.</summary>
    public static Claim Value(string subject, string predicate, object? value, bool polarity = true, IReadOnlyDictionary<string, object?>? qualifiers = null, string? about = null) =>
        Create(subject, predicate, new ClaimObject.ValueClaimObject { Value = ToElement(value) }, polarity, qualifiers, about);

    /// <summary><c>subject predicate</c>, with no object, e.g. <c>Claim.Unary("bob", "asleep")</c>.</summary>
    public static Claim Unary(string subject, string predicate, bool polarity = true, IReadOnlyDictionary<string, object?>? qualifiers = null, string? about = null) =>
        Create(subject, predicate, new ClaimObject.None(), polarity, qualifiers, about);

    /// <summary>
    /// A claim about what a holder holds: second-order knowledge. The key of the claim comes from
    /// the engine (<c>engine.ClaimKey</c>), never from a hash made in the client.
    /// </summary>
    public static Claim Holds(string holder, string claimKey, bool polarity = true, IReadOnlyDictionary<string, object?>? qualifiers = null, string? about = null) =>
        Create(holder, "umwelt:holds", new ClaimObject.ValueClaimObject { Value = ToElement(new Dictionary<string, string> { ["claimKey"] = claimKey }) }, polarity, qualifiers, about);

    /// <summary>The same claim with the opposite polarity. This claim is not changed.</summary>
    public Claim Negated() => new()
    {
        Subject = Subject,
        Predicate = Predicate,
        Object = Object,
        Polarity = !Polarity,
        Qualifiers = Qualifiers is null ? null : new Dictionary<string, JsonElement>(Qualifiers),
        About = About,
    };

    private static Claim Create(string subject, string predicate, ClaimObject claimObject, bool polarity, IReadOnlyDictionary<string, object?>? qualifiers, string? about) => new()
    {
        Subject = subject,
        Predicate = predicate,
        Object = claimObject,
        Polarity = polarity,
        Qualifiers = qualifiers?.ToDictionary(pair => pair.Key, pair => ToElement(pair.Value)),
        About = about,
    };

    private static JsonElement ToElement(object? value) =>
        value is JsonElement element ? element.Clone() : JsonSerializer.SerializeToElement(value, UmweltJson.Options);
}

public sealed partial class Position
{
    /// <summary>A position on the main axis.</summary>
    public static Position At(long tick) => new() { Tick = tick };
}

public sealed partial class Provenance
{
    /// <summary>Written by a person.</summary>
    public static Provenance Human(string? actorId = null) => new() { Origin = new Origin.Human { ActorId = actorId } };

    /// <summary>Written by a model.</summary>
    public static Provenance Model(string model, string? promptHash = null) => new() { Origin = new Origin.ModelOrigin { Model = model, PromptHash = promptHash } };

    /// <summary>Written by an automated part of the application.</summary>
    public static Provenance System(string name) => new() { Origin = new Origin.System { Name = name } };

    /// <summary>Imported from another source.</summary>
    public static Provenance Imported() => new() { Origin = new Origin.Import() };
}

/// <summary>How a holder came to hold a claim. Always explicit: the engine never assumes it.</summary>
public static class Acquired
{
    /// <summary>The holder was there: acquired at the event <paramref name="eventId"/>.</summary>
    public static Acquisition Witnessed(string eventId) => new Acquisition.Witnessed { EventId = eventId };

    /// <summary>Someone told the holder, optionally from one of their own Holdings.</summary>
    public static Acquisition ToldBy(string sourceHolderId, Intent intent, string? sourceRecordId = null) =>
        new Acquisition.ToldBy { SourceHolderId = sourceHolderId, Intent = intent, SourceRecordId = sourceRecordId };

    /// <summary>The holder read or saw a piece of evidence.</summary>
    public static Acquisition Evidence(string evidenceId) => new Acquisition.Evidence { EvidenceId = evidenceId };

    /// <summary>The holder concluded it from other records.</summary>
    public static Acquisition Inferred(IEnumerable<string> fromRecordIds, string? rule = null) =>
        new Acquisition.Inferred { FromRecordIds = fromRecordIds.ToList(), Rule = rule };

    /// <summary>The author says so: no in-world source.</summary>
    public static Acquisition Authored() => new Acquisition.Authored();
}

/// <summary>Records for a commit. A Holding's attitude is the name of its factory.</summary>
public static class Records
{
    /// <summary>Canon: what is true in the world, whoever knows it.</summary>
    public static RecordInput.Fact Fact(Claim claim, string? id = null, Position? position = null, Provenance? provenance = null) =>
        new() { Claim = claim, Id = id, Position = position, Provenance = provenance };

    /// <summary>A Holding with attitude <see cref="Attitude.Knows"/>.</summary>
    public static RecordInput.Holding Knows(string holder, Claim claim, Acquisition acquisition, string? id = null, Position? position = null, Provenance? provenance = null, double? confidence = null, Accessibility? accessibility = null, string? text = null, Secrecy? secrecy = null) =>
        Holding(Attitude.Knows, holder, claim, acquisition, id, position, provenance, confidence, accessibility, text, secrecy);

    /// <summary>A Holding with attitude <see cref="Attitude.Believes"/>.</summary>
    public static RecordInput.Holding Believes(string holder, Claim claim, Acquisition acquisition, string? id = null, Position? position = null, Provenance? provenance = null, double? confidence = null, Accessibility? accessibility = null, string? text = null, Secrecy? secrecy = null) =>
        Holding(Attitude.Believes, holder, claim, acquisition, id, position, provenance, confidence, accessibility, text, secrecy);

    /// <summary>A Holding with attitude <see cref="Attitude.Suspects"/>.</summary>
    public static RecordInput.Holding Suspects(string holder, Claim claim, Acquisition acquisition, string? id = null, Position? position = null, Provenance? provenance = null, double? confidence = null, Accessibility? accessibility = null, string? text = null, Secrecy? secrecy = null) =>
        Holding(Attitude.Suspects, holder, claim, acquisition, id, position, provenance, confidence, accessibility, text, secrecy);

    /// <summary>A Holding with attitude <see cref="Attitude.Remembers"/>.</summary>
    public static RecordInput.Holding Remembers(string holder, Claim claim, Acquisition acquisition, string? id = null, Position? position = null, Provenance? provenance = null, double? confidence = null, Accessibility? accessibility = null, string? text = null, Secrecy? secrecy = null) =>
        Holding(Attitude.Remembers, holder, claim, acquisition, id, position, provenance, confidence, accessibility, text, secrecy);

    private static RecordInput.Holding Holding(Attitude attitude, string holder, Claim claim, Acquisition acquisition, string? id, Position? position, Provenance? provenance, double? confidence, Accessibility? accessibility, string? text, Secrecy? secrecy) => new()
    {
        Id = id,
        Position = position,
        Provenance = provenance,
        Holder = holder,
        Attitude = attitude,
        Claim = claim,
        Acquisition = acquisition,
        Confidence = confidence,
        Accessibility = accessibility,
        Text = text,
        Secrecy = secrecy,
    };
}
