<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# C# minimal example

The scene of [the getting-started guide](../../docs/getting-started.md) with the
official .NET binding (`bindings/csharp/Kallamo.Umwelt`): it creates a `.umw`
file, records that Ana saw Bob steal a necklace, asks who knows it, and shows a
typed error.

Build the native library from the repository root, then run the example:

```bash
cargo build -p umwelt-ffi --release
```

```bash
dotnet run --project examples/csharp-minimal/UmweltMinimal.csproj
```

The project references the binding's source and copies the native library from
`target/release` next to the program. A published app references the `Kallamo.Umwelt`
NuGet package instead, which carries the native library for Windows, Linux and
macOS. The program prints `Ana knows: True` and `Bob knows: False`, and targets
.NET 6.

## Durability

`UmweltEngine.Create` and `UmweltEngine.Open` default to `Durability.Normal`:
committed data survives an application crash, and a power loss may drop the
most recent commits without corrupting the file. Pass `Durability.Full` to
flush every commit before it returns.
