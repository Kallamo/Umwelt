// SPDX-License-Identifier: AGPL-3.0-only
// Runs the language-neutral cases in conformance/cases through the .NET
// binding. Every request payload and every result also makes a round trip
// through the generated types, which must reproduce equivalent JSON.
using System.Reflection;
using System.Text.Json;
using System.Text.Json.Nodes;
using Kallamo.Umwelt;

var casesDirectory = args.Length > 0
    ? args[0]
    : Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "../../../../../../conformance/cases"));
var files = Directory.GetFiles(casesDirectory, "*.json").OrderBy(path => path, StringComparer.Ordinal).ToList();
if (files.Count == 0)
{
    Console.Error.WriteLine($"No cases in {casesDirectory}.");
    return 2;
}

var failures = 0;
foreach (var file in files)
{
    var name = Path.GetFileNameWithoutExtension(file);
    try
    {
        Conformance.Run(JsonNode.Parse(File.ReadAllText(file))!);
        Console.WriteLine($"ok   {name}");
    }
    catch (Exception error)
    {
        failures++;
        Console.WriteLine($"FAIL {name}: {error.Message}");
    }
}
try
{
    await EmbeddingHelpers.Run();
    Console.WriteLine("ok   sdk-embedding-helpers");
}
catch (Exception error)
{
    failures++;
    Console.WriteLine($"FAIL sdk-embedding-helpers: {error.Message}");
}
try
{
    Builders.Run(Path.Combine(casesDirectory, "..", "builders.json"));
    Console.WriteLine("ok   sdk-builders");
}
catch (Exception error)
{
    failures++;
    Console.WriteLine($"FAIL sdk-builders: {error.Message}");
}
Console.WriteLine($"{files.Count + 2 - failures} passed, {failures} failed");
return failures == 0 ? 0 : 1;

internal static class Conformance
{
    internal static readonly JsonSerializerOptions Options = (JsonSerializerOptions)typeof(UmweltEngine).Assembly
        .GetType("Kallamo.Umwelt.UmweltJson")!
        .GetField("Options", BindingFlags.NonPublic | BindingFlags.Static)!
        .GetValue(null)!;

    public static void Run(JsonNode testCase)
    {
        var directory = Directory.CreateDirectory(Path.Combine(Path.GetTempPath(), $"umwelt-conformance-{Guid.NewGuid():N}"));
        try
        {
            using var engine = UmweltEngine.OpenWithoutHandshake(Path.Combine(directory.FullName, "test.umw"), create: true);
            var captures = new Dictionary<string, JsonNode?>();
            var index = 0;
            foreach (var step in testCase["steps"]!.AsArray())
            {
                index++;
                var op = step!["request"]!["op"]!.GetValue<string>();
                var payload = Resolve(step["request"]!["payload"], captures);
                var expected = step["expect"]!;
                var expectOk = expected["ok"]!.GetValue<bool>();
                var method = typeof(UmweltEngine).GetMethod(Pascal(op), BindingFlags.Public | BindingFlags.Instance);
                var payloadElement = JsonSerializer.SerializeToElement(payload);
                if (method is not null && expectOk)
                {
                    AssertRoundTrip(payloadElement, method.GetParameters()[0].ParameterType, $"step {index} {op} payload");
                }
                var response = JsonNode.Parse(engine.SendEnvelope(op, payloadElement).GetRawText())!;
                if (method is not null && response["ok"]!.GetValue<bool>())
                {
                    AssertRoundTrip(JsonSerializer.SerializeToElement(response["result"]), method.ReturnType, $"step {index} {op} result");
                }
                Subset(expected, response, captures, $"step {index} {op}");
            }
        }
        finally
        {
            directory.Delete(recursive: true);
        }
    }

    // `timeline.openDraft` is the method TimelineOpenDraft.
    private static string Pascal(string op) => string.Concat(
        op.Split('.', '-').Where(part => part.Length > 0).Select(part => char.ToUpperInvariant(part[0]) + part[1..]));

    private static void AssertRoundTrip(JsonElement original, Type type, string where)
    {
        var typed = original.Deserialize(type, Options);
        var again = JsonSerializer.SerializeToElement(typed, type, Options);
        if (!Equivalent(original, again))
        {
            throw new InvalidOperationException($"{where}: {type.Name} does not round-trip.\n  original: {original.GetRawText()}\n  typed:    {again.GetRawText()}");
        }
    }

    // Equal JSON, where an absent property equals null and numbers compare by value.
    private static bool Equivalent(JsonElement left, JsonElement right)
    {
        if (left.ValueKind == JsonValueKind.Number && right.ValueKind == JsonValueKind.Number)
        {
            return left.GetDouble() == right.GetDouble();
        }
        if (left.ValueKind != right.ValueKind)
        {
            return false;
        }
        switch (left.ValueKind)
        {
            case JsonValueKind.Object:
                var leftProperties = left.EnumerateObject().Where(property => property.Value.ValueKind != JsonValueKind.Null).ToDictionary(property => property.Name, property => property.Value);
                var rightProperties = right.EnumerateObject().Where(property => property.Value.ValueKind != JsonValueKind.Null).ToDictionary(property => property.Name, property => property.Value);
                return leftProperties.Count == rightProperties.Count
                    && leftProperties.All(pair => rightProperties.TryGetValue(pair.Key, out var other) && Equivalent(pair.Value, other));
            case JsonValueKind.Array:
                var leftItems = left.EnumerateArray().ToList();
                var rightItems = right.EnumerateArray().ToList();
                return leftItems.Count == rightItems.Count && leftItems.Zip(rightItems).All(pair => Equivalent(pair.First, pair.Second));
            case JsonValueKind.String:
                return left.GetString() == right.GetString();
            default:
                return true;
        }
    }

    private static JsonNode? Resolve(JsonNode? value, Dictionary<string, JsonNode?> captures)
    {
        switch (value)
        {
            case JsonValue text when text.TryGetValue<string>(out var s) && s.StartsWith("$ref:", StringComparison.Ordinal):
                return captures.TryGetValue(s[5..], out var captured) ? Clone(captured) : throw new InvalidOperationException($"Unknown capture {s}");
            case JsonArray array:
                return new JsonArray(array.Select(item => Resolve(item, captures)).ToArray());
            case JsonObject obj:
                var copy = new JsonObject();
                foreach (var (key, item) in obj)
                {
                    copy[key] = Resolve(item, captures);
                }
                return copy;
            default:
                return Clone(value);
        }
    }

    private static void Subset(JsonNode? expected, JsonNode? actual, Dictionary<string, JsonNode?> captures, string where)
    {
        if (expected is JsonValue marker && marker.TryGetValue<string>(out var text))
        {
            if (text == "$any")
            {
                return;
            }
            if (text.StartsWith("$capture:", StringComparison.Ordinal))
            {
                captures[text[9..]] = Clone(actual);
                return;
            }
            if (text.StartsWith("$ref:", StringComparison.Ordinal))
            {
                var reference = captures[text[5..]];
                if (!NodesEqual(reference, actual))
                {
                    throw new InvalidOperationException($"{where}: expected {reference?.ToJsonString()} got {actual?.ToJsonString()}");
                }
                return;
            }
        }
        switch (expected)
        {
            case JsonObject fields:
                if (actual is not JsonObject actualObject)
                {
                    throw new InvalidOperationException($"{where}: expected an object, got {actual?.ToJsonString()}");
                }
                foreach (var (key, value) in fields)
                {
                    if (!actualObject.ContainsKey(key))
                    {
                        throw new InvalidOperationException($"{where}: missing '{key}' in {actualObject.ToJsonString()}");
                    }
                    Subset(value, actualObject[key], captures, $"{where}.{key}");
                }
                break;
            case JsonArray items:
                if (actual is not JsonArray actualArray || actualArray.Count != items.Count)
                {
                    throw new InvalidOperationException($"{where}: expected {items.Count} items, got {actual?.ToJsonString()}");
                }
                for (var i = 0; i < items.Count; i++)
                {
                    Subset(items[i], actualArray[i], captures, $"{where}[{i}]");
                }
                break;
            default:
                if (!NodesEqual(expected, actual))
                {
                    throw new InvalidOperationException($"{where}: expected {expected?.ToJsonString()} got {actual?.ToJsonString()}");
                }
                break;
        }
    }

    // JsonNode.DeepClone is .NET 8; this runner targets .NET 6.
    private static JsonNode? Clone(JsonNode? node) => node is null ? null : JsonNode.Parse(node.ToJsonString());

    internal static bool NodesEqual(JsonNode? left, JsonNode? right) =>
        Equivalent(JsonSerializer.SerializeToElement(left), JsonSerializer.SerializeToElement(right));
}

/// <summary>
/// The hand-written write factories against conformance/builders.json, which
/// the npm package checks too: both languages must build the same JSON.
/// </summary>
internal static class Builders
{
    private static readonly Dictionary<string, Func<JsonObject, object>> Factories = new()
    {
        ["claim.entity"] = a => Claim.Entity(Text(a, "subject"), Text(a, "predicate"), Text(a, "objectId"), Polarity(a), Qualifiers(a), Optional(a, "about")),
        ["claim.value"] = a => Claim.Value(Text(a, "subject"), Text(a, "predicate"), JsonSerializer.SerializeToElement(a["value"]), Polarity(a), Qualifiers(a), Optional(a, "about")),
        ["claim.unary"] = a => Claim.Unary(Text(a, "subject"), Text(a, "predicate"), Polarity(a), Qualifiers(a), Optional(a, "about")),
        ["claim.holds"] = a => Claim.Holds(Text(a, "holder"), Text(a, "claimKey"), Polarity(a), Qualifiers(a), Optional(a, "about")),
        ["negate"] = a => Typed<Claim>(a, "claim")!.Negated(),
        ["at"] = a => Position.At(a["tick"]!.GetValue<long>()),
        ["by.human"] = a => Provenance.Human(Optional(a, "actorId")),
        ["by.model"] = a => Provenance.Model(Text(a, "model"), Optional(a, "promptHash")),
        ["by.system"] = a => Provenance.System(Text(a, "name")),
        ["by.imported"] = _ => Provenance.Imported(),
        ["witnessed"] = a => Acquired.Witnessed(Text(a, "eventId")),
        ["toldBy"] = a => Acquired.ToldBy(Text(a, "sourceHolderId"), Typed<Intent>(a, "intent"), Optional(a, "sourceRecordId")),
        ["evidence"] = a => Acquired.Evidence(Text(a, "evidenceId")),
        ["inferred"] = a => Acquired.Inferred(Typed<List<string>>(a, "fromRecordIds")!, Optional(a, "rule")),
        ["authored"] = _ => Acquired.Authored(),
        ["fact"] = a => Records.Fact(Typed<Claim>(a, "claim")!, Optional(a, "id"), Typed<Position>(a, "position"), Typed<Provenance>(a, "provenance")),
        ["knows"] = a => Holding(Records.Knows, a),
        ["believes"] = a => Holding(Records.Believes, a),
        ["suspects"] = a => Holding(Records.Suspects, a),
        ["remembers"] = a => Holding(Records.Remembers, a),
    };

    private delegate RecordInput.Holding HoldingFactory(string holder, Claim claim, Acquisition acquisition, string? id, Position? position, Provenance? provenance, double? confidence, Accessibility? accessibility, string? text, Secrecy? secrecy);

    public static void Run(string path)
    {
        var lines = JsonNode.Parse(File.ReadAllText(path))!.AsArray();
        var called = new HashSet<string>();
        var index = 0;
        foreach (var line in lines)
        {
            index++;
            var call = line!["call"]!.GetValue<string>();
            called.Add(call);
            if (!Factories.TryGetValue(call, out var factory))
            {
                throw new InvalidOperationException($"line {index}: no .NET factory for '{call}'.");
            }
            var built = factory(line["args"]!.AsObject());
            var actual = JsonSerializer.SerializeToNode(built, built.GetType().BaseType is { IsAbstract: true } parent ? parent : built.GetType(), Conformance.Options);
            if (!Conformance.NodesEqual(line["expect"], actual))
            {
                throw new InvalidOperationException($"line {index} {call}: expected {line["expect"]!.ToJsonString()} got {actual?.ToJsonString()}");
            }
        }
        var unused = Factories.Keys.Where(name => !called.Contains(name)).ToList();
        if (unused.Count > 0)
        {
            throw new InvalidOperationException($"no line in builders.json for {string.Join(", ", unused)}.");
        }
    }

    private static RecordInput.Holding Holding(HoldingFactory factory, JsonObject a) => factory(
        Text(a, "holder"), Typed<Claim>(a, "claim")!, Typed<Acquisition>(a, "acquisition")!, Optional(a, "id"),
        Typed<Position>(a, "position"), Typed<Provenance>(a, "provenance"), a["confidence"]?.GetValue<double>(),
        a.ContainsKey("accessibility") ? Typed<Accessibility>(a, "accessibility") : null, Optional(a, "text"), Typed<Secrecy>(a, "secrecy"));

    private static string Text(JsonObject args, string name) => args[name]!.GetValue<string>();

    private static string? Optional(JsonObject args, string name) => args[name]?.GetValue<string>();

    private static bool Polarity(JsonObject args) => args["polarity"]?.GetValue<bool>() ?? true;

    private static T? Typed<T>(JsonObject args, string name) =>
        args[name] is { } node ? node.Deserialize<T>(Conformance.Options) : default;

    private static Dictionary<string, object?>? Qualifiers(JsonObject args) =>
        args["qualifiers"]?.AsObject().ToDictionary(pair => pair.Key, pair => (object?)JsonSerializer.SerializeToElement(pair.Value));
}

/// <summary>
/// The hand-written embedding helpers, with a deterministic stand-in for a
/// model: one dimension per topic.
/// </summary>
internal static class EmbeddingHelpers
{
    private sealed class TopicEmbedder : IEmbedder
    {
        public List<int> Batches { get; } = new();
        public string SpaceId => "topics";
        public int BatchSize => 2;

        public Task<IReadOnlyList<IReadOnlyList<double>>> EmbedAsync(IReadOnlyList<string> texts, CancellationToken cancellationToken = default)
        {
            Batches.Add(texts.Count);
            IReadOnlyList<IReadOnlyList<double>> vectors = texts
                .Select(text => (IReadOnlyList<double>)new[]
                {
                    text.Contains("jewel") || text.Contains("stole") ? 1.0 : 0.01,
                    text.Contains("storm") ? 1.0 : 0.01,
                    text.Contains("bread") ? 1.0 : 0.01,
                })
                .ToList();
            return Task.FromResult(vectors);
        }
    }

    public static async Task Run()
    {
        var path = Path.Combine(Path.GetTempPath(), $"umwelt-embeddings-{Guid.NewGuid():N}.umw");
        try
        {
            using var engine = UmweltEngine.Create(path, client: "conformance-embeddings/1");
            engine.ProjectCreate(new CreateProjectInput { Id = "p", Name = "Town", Timescale = "day", VocabularyMode = VocabularyMode.Open });
            engine.EntityTypeCreate(new CreateEntityTypeInput { ProjectId = "p", Id = "agent", Name = "Agent", Kind = EntityKind.Agent });
            engine.EntityCreate(new CreateEntityInput { ProjectId = "p", Id = "ana", TypeId = "agent", Name = "Ana" });
            engine.EmbeddingDefineSpace(new DefineEmbeddingSpaceInput
            {
                ProjectId = "p",
                Space = new EmbeddingSpace { Id = "topics", Dimensions = 3, Model = "fake" },
            });
            var provenance = new Provenance { Origin = new Origin.Human() };
            RecordInput Note(string id, string text) => new RecordInput.Holding
            {
                Id = id,
                Position = new Position { Tick = 1 },
                Provenance = provenance,
                Holder = "ana",
                Attitude = Attitude.Believes,
                Claim = new Claim { Subject = "ana", Predicate = "notes", Object = new ClaimObject.None(), Polarity = true },
                Acquisition = new Acquisition.Authored(),
                Confidence = 1.0,
                Accessibility = Accessibility.Available,
                Text = text,
            };
            engine.Commit(new CommitInput
            {
                ProjectId = "p",
                AttemptId = "day-1",
                Event = new EventInput { Id = "e-day-1", Type = "umwelt:test-day", Title = "Day", Position = new Position { Tick = 1 }, Provenance = provenance },
                Records = new() { Note("necklace", "someone took the jewel"), Note("weather", "a storm all night"), Note("market", "bread is expensive") },
            });
            var embedder = new TopicEmbedder();
            var filled = await engine.EmbedPendingAsync(embedder, "p");
            Expect(filled.Embedded == 3, $"embedded {filled.Embedded}");
            Expect(embedder.Batches.SequenceEqual(new[] { 2, 1 }), "batches of two");
            Expect((await engine.EmbedPendingAsync(embedder, "p")).Embedded == 0, "nothing left");
            var found = await engine.SemanticSearchAsync(embedder, new SearchPayload { ProjectId = "p", Filter = new SearchFilter { Holder = "ana" } }, "who stole it?", hybrid: false);
            Expect(found.Items.FirstOrDefault()?.DocId == "necklace", "semantic search finds the jewel");
            Expect(found.Items[0].Match == SearchMatch.Semantic, "semantic match");
        }
        finally
        {
            File.Delete(path);
        }
    }

    private static void Expect(bool condition, string what)
    {
        if (!condition)
        {
            throw new InvalidOperationException($"Embedding helpers: {what}.");
        }
    }
}
