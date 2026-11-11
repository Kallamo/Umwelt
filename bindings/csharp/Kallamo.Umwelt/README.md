<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Umwelt for .NET

The official .NET binding of [Umwelt](https://github.com/Kallamo/Umwelt), an
epistemic state engine: it records not only what happened in a story or a
game, but who knows it, how they came to know it, and since when. The engine
runs in-process through its native library; no server or extra process is
involved.

```bash
dotnet add package Kallamo.Umwelt
```

Targets .NET 6 and later. Native libraries are included for Windows, Linux and
macOS. The [getting-started guide](https://github.com/Kallamo/Umwelt/blob/main/docs/getting-started.md#net)
records a complete scene and asks who knows what happened in it.

```csharp
using Kallamo.Umwelt;

using var engine = UmweltEngine.Create("save.umw");
engine.ProjectCreate(new CreateProjectInput { Id = "world", Name = "World", Timescale = "day" });

// ... entity types, entities and predicates ...

var result = engine.Holds(new HoldsPayload
{
    ProjectId = "world",
    Holder = "mira",
    ClaimKey = claimKey,
});
if (result.Holds)
{
    // Mira knows it.
}
```

Records are plain objects, and small factories build the repetitive ones:

```csharp
var theft = Claim.Entity("bob", "stole", "necklace");
engine.Commit(new CommitInput
{
    ProjectId = "ball",
    AttemptId = "scene-1",
    Event = new EventInput { Id = "scene-1", Type = "scene", Title = "The ball", Position = Position.At(1), Provenance = Provenance.Human() },
    Records = new()
    {
        Records.Fact(theft),
        Records.Knows("ana", theft, Acquired.Witnessed("scene-1")),
        Records.Suspects("duchess", Claim.Entity("ana", "stole", "necklace"), Acquired.Authored(), confidence: 0.6),
    },
});
```

`Claim.Entity`, `Claim.Value`, `Claim.Unary`, `Claim.Holds` and
`claim.Negated()` build claims; `Position.At` a position; `Provenance.Human`, `.Model`, `.System` and
`.Imported` a provenance; `Acquired.Witnessed`, `.ToldBy`, `.Evidence`,
`.Inferred` and `.Authored` an acquisition; `Records.Fact` and one factory per
attitude (`Records.Knows`, `.Believes`, `.Suspects`, `.Remembers`) a record.
Each is pure and returns the contract type, so it mixes with object
initializers. A record that gives no position or provenance takes those of
its event, and a Holding defaults to confidence 1 and `Available`; optional
named arguments (`id`, `position`, `provenance`, `confidence`,
`accessibility`, `text`, `secrecy`) set them. When everyone at an event saw
the same thing, `engine.Witness` records it in one call.

Semantic search uses your own embedding model: implement `IEmbedder`
(`SpaceId` and `EmbedAsync`), then `EmbedPendingAsync` fills the vectors of
every searchable document and `SemanticSearchAsync` searches by meaning,
hybrid by default. Umwelt never runs a model itself.

Every protocol operation is a typed method of `UmweltEngine`, named after the
operation (`timeline.openDraft` is `TimelineOpenDraft`). Errors are exceptions
carrying the stable protocol code: `VersionConflictException`,
`NotFoundException`, `ValidationFailedException`, `ConflictException`,
`InvalidStateException`, and so on. `Call<T>` and `CallRaw` remain available
for generic tooling.

Calls on one engine are serialized: use it from one thread at a time or let
the engine's lock order them. Disposing the engine checkpoints and closes the
file.

## Native library lookup

The binding loads the native library (`umwelt_ffi.dll`, `libumwelt_ffi.so` or
`libumwelt_ffi.dylib`) from, in order:

1. `UmweltNative.LibraryPath`, when set: a file, or a directory to search;
2. the directory of `Kallamo.Umwelt.dll`;
3. `runtimes/<rid>/native/` inside that directory (the NuGet layout);
4. the default .NET lookup.

Plugin hosts that load assemblies from their own folders, such as game mod
loaders, need no configuration when the native library ships next to
`Kallamo.Umwelt.dll`. If the host loads assemblies from memory, set
`UmweltNative.LibraryPath` to the plugin folder before opening an engine.
`UmweltNative.LoadedFrom` tells which file was loaded.

## License

AGPL-3.0-only. Free, non-monetized games, mods, interactive fiction and
tabletop play tools may use Umwelt without releasing their own code under the
[Umwelt Free Play Exception](https://github.com/Kallamo/Umwelt/blob/main/LICENSE-EXCEPTION.md);
other uses that cannot comply with the AGPL can obtain a commercial license at
contact@kallamo.com. Built by [Kallamo](https://kallamo.com);
see the [repository](https://github.com/Kallamo/Umwelt) for the documentation
and how to contribute.
