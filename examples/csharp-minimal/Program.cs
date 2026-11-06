// SPDX-License-Identifier: AGPL-3.0-only
// The getting-started scene with the .NET binding: Ana sees Bob steal the
// necklace. Canon records the theft; only Ana holds it.
using Kallamo.Umwelt;

var path = Path.Combine(Path.GetTempPath(), $"umwelt-example-{Guid.NewGuid():N}.umw");

using (var engine = UmweltEngine.Create(path))
{
    const string projectId = "ball";
    engine.ProjectCreate(new CreateProjectInput { Id = projectId, Name = "The Ball", Timescale = "scene" });
    engine.EntityTypeCreate(new CreateEntityTypeInput { ProjectId = projectId, Id = "person", Name = "Person", Kind = EntityKind.Agent });
    engine.EntityTypeCreate(new CreateEntityTypeInput { ProjectId = projectId, Id = "thing", Name = "Thing", Kind = EntityKind.Other });
    engine.EntityCreate(new CreateEntityInput { ProjectId = projectId, Id = "ana", TypeId = "person", Name = "Ana" });
    engine.EntityCreate(new CreateEntityInput { ProjectId = projectId, Id = "bob", TypeId = "person", Name = "Bob" });
    engine.EntityCreate(new CreateEntityInput { ProjectId = projectId, Id = "necklace", TypeId = "thing", Name = "The duchess's necklace" });
    engine.PredicateDefine(new DefinePredicateInput
    {
        ProjectId = projectId,
        Predicate = new PredicateDef { Id = "stole", Label = "stole", Object = ObjectKind.Entity, Cardinality = Cardinality.Many },
    });

    var theft = new Claim
    {
        Subject = "bob",
        Predicate = "stole",
        Object = new ClaimObject.Entity { Id = "necklace" },
        Polarity = true,
    };
    var provenance = new Provenance { Origin = new Origin.Human() };
    engine.Commit(new CommitInput
    {
        ProjectId = projectId,
        AttemptId = "scene-1",
        Event = new EventInput
        {
            Id = "scene-1",
            Type = "scene",
            Title = "The ball",
            Position = new Position { Tick = 1 },
            Participants = new() { "ana", "bob" },
            Provenance = provenance,
        },
        Records = new()
        {
            new RecordInput.Fact { Id = "theft", Position = new Position { Tick = 1 }, Provenance = provenance, Claim = theft },
            new RecordInput.Holding
            {
                Id = "ana-saw",
                Position = new Position { Tick = 1 },
                Provenance = provenance,
                Holder = "ana",
                Attitude = Attitude.Knows,
                Claim = theft,
                Acquisition = new Acquisition.Witnessed { EventId = "scene-1" },
                Confidence = 1.0,
                Accessibility = Accessibility.Available,
            },
        },
    });

    var ana = engine.Holds(new HoldsPayload { ProjectId = projectId, Holder = "ana", Claim = theft });
    var bob = engine.Holds(new HoldsPayload { ProjectId = projectId, Holder = "bob", Claim = theft });
    Console.WriteLine($"Ana knows: {ana.Holds}"); // Ana knows: True
    Console.WriteLine($"Bob knows: {bob.Holds}"); // Bob knows: False

    // Errors are exceptions that carry the protocol code.
    try
    {
        engine.Holds(new HoldsPayload { ProjectId = "missing", Holder = "ana", Claim = theft });
    }
    catch (NotFoundException error)
    {
        Console.WriteLine($"Error: {error.Code}"); // Error: NOT_FOUND
    }

    if (!ana.Holds || bob.Holds)
    {
        return 1;
    }
}

File.Delete(path);
return 0;
