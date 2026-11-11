// SPDX-License-Identifier: AGPL-3.0-only
// The getting-started scene with the .NET binding: Ana sees Bob steal the
// necklace. Canon records the theft; Ana and Bob know it, and the duchess,
// though present, does not. The next morning she suspects the wrong person.
using System.Globalization;
using Kallamo.Umwelt;

var path = Path.Combine(Path.GetTempPath(), $"umwelt-example-{Guid.NewGuid():N}.umw");

using (var engine = UmweltEngine.Create(path))
{
    const string projectId = "ball";
    // The world, declared once. It creates what is missing and changes nothing
    // that is already there, so it can run every time the file is opened.
    engine.WorldEnsure(new EnsureWorldInput
    {
        Project = new WorldProject { Id = projectId, Name = "The Ball", Timescale = "scene" },
        EntityTypes = new()
        {
            new WorldEntityType { Id = "person", Name = "Person", Kind = EntityKind.Agent },
            new WorldEntityType { Id = "thing", Name = "Thing", Kind = EntityKind.Other },
        },
        Entities = new()
        {
            new WorldEntity { Id = "ana", TypeId = "person", Name = "Ana" },
            new WorldEntity { Id = "bob", TypeId = "person", Name = "Bob" },
            new WorldEntity { Id = "duchess", TypeId = "person", Name = "The duchess" },
            new WorldEntity { Id = "necklace", TypeId = "thing", Name = "The duchess's necklace" },
        },
        Predicates = new() { new PredicateDef { Id = "stole", Label = "stole", Object = ObjectKind.Entity, Cardinality = Cardinality.Many } },
    });

    // The scene: one event, the Fact (canon) and what each witness saw.
    var theft = Claim.Entity("bob", "stole", "necklace");
    engine.Witness(new WitnessInput
    {
        ProjectId = projectId,
        AttemptId = "scene-1",
        Event = new EventInput
        {
            Id = "scene-1",
            Type = "scene",
            Title = "The ball",
            Position = Position.At(1),
            Participants = new() { "ana", "bob", "duchess" },
            Provenance = Provenance.Human(),
        },
        Claims = new() { new WitnessedClaim { Claim = theft, Canon = true } },
        Witnesses = new() { new Witness { Holder = "ana" }, new Witness { Holder = "bob" } },
    });

    var ana = engine.Holds(new HoldsPayload { ProjectId = projectId, Holder = "ana", Claim = theft });
    var bob = engine.Holds(new HoldsPayload { ProjectId = projectId, Holder = "bob", Claim = theft });
    var duchess = engine.Holds(new HoldsPayload { ProjectId = projectId, Holder = "duchess", Claim = theft });
    // Holds is also true for someone who believes the opposite; HoldsAs tells them apart.
    Console.WriteLine($"Ana knows: {ana.HoldsAs == HoldsAs.Same}"); // Ana knows: True
    Console.WriteLine($"Bob knows: {bob.HoldsAs == HoldsAs.Same}"); // Bob knows: True
    Console.WriteLine($"The duchess knows: {duchess.HoldsAs == HoldsAs.Same}"); // The duchess knows: False

    // The morning after: nobody saw anything and nobody told her, so this is a
    // record written directly. It takes its position and provenance from the event.
    var accusation = Claim.Entity("ana", "stole", "necklace");
    engine.Commit(new CommitInput
    {
        ProjectId = projectId,
        AttemptId = "morning",
        Event = new EventInput { Type = "scene", Title = "The morning after", Position = Position.At(2), Provenance = Provenance.Human() },
        Records = new() { Records.Suspects("duchess", accusation, Acquired.Authored(), confidence: 0.6) },
    });

    var suspicion = engine.Holds(new HoldsPayload { ProjectId = projectId, Holder = "duchess", Claim = accusation });
    var confidence = suspicion.Confidence?.ToString(CultureInfo.InvariantCulture);
    Console.WriteLine($"The duchess {suspicion.Attitude.ToString().ToLowerInvariant()} Ana: {confidence}"); // The duchess suspects Ana: 0.6

    // Errors are exceptions that carry the protocol code.
    try
    {
        engine.Holds(new HoldsPayload { ProjectId = "missing", Holder = "ana", Claim = theft });
    }
    catch (NotFoundException error)
    {
        Console.WriteLine($"Error: {error.Code}"); // Error: NOT_FOUND
    }

    if (ana.HoldsAs != HoldsAs.Same || bob.HoldsAs != HoldsAs.Same || duchess.Holds || suspicion.Attitude != Attitude.Suspects)
    {
        return 1;
    }
}

File.Delete(path);
return 0;
