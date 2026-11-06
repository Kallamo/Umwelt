# @kallamo/umwelt

The TypeScript client of [Umwelt](https://github.com/Kallamo/Umwelt), an
epistemic state engine: it records what happened in a story, a game or a
simulation and, kept apart from it, who knows it, how they came to know it and
since when.

```bash
npm install @kallamo/umwelt
```

Requires Node.js 22 or newer. The package includes the Umwelt engine for
Windows, Linux and macOS; the client runs it as a child process and talks to it
over the [JSON protocol](https://github.com/Kallamo/Umwelt/blob/main/spec/protocol.md).
No native Node addon is used.

```ts
import { createClient, UmweltError } from '@kallamo/umwelt';

const client = createClient({ file: 'world.umw', create: true });
try {
  const project = await client.projectCreate({ name: 'World', timescale: 'day' });
  console.log(await client.projectGet({ projectId: project.id }));
} catch (error) {
  if (error instanceof UmweltError) console.error(error.code, error.message);
  throw error;
} finally {
  await client.close();
}
```

The [getting-started guide](https://github.com/Kallamo/Umwelt/blob/main/docs/getting-started.md#typescript)
records a complete scene and asks who knows what happened in it.

## The client

Every protocol operation is a typed method, named in camelCase:
`project.create` is `projectCreate`, `timeline.openDraft` is
`timelineOpenDraft`. Payload and result types are generated from the engine's
JSON Schemas and exported by the package.

`createClient` options:

| Option | Meaning |
| --- | --- |
| `file` | The `.umw` file to open. |
| `create` | Create the file if it does not exist. |
| `binary` | Path of an `umwelt` executable to run instead of the bundled one. |
| `client` | The name the client reports in the handshake. |
| `onStderr` | Receives the engine's diagnostics. |
| `embedder` | Your embedding model, for semantic search (below). |

The client performs the handshake, sends requests concurrently and matches each
response to its request. A failed request rejects with an `UmweltError` that
carries the protocol `code`, a `message` and optional `details`.
`TRANSPORT_CLOSED` is reported by the client itself, not by the engine: if the
engine process ends, every pending request rejects with it. `close()` asks the
engine to shut down and waits for it to exit.

## Semantic search with your own model

Umwelt never runs a model. Pass an `embedder` and the client uses yours:

```ts
const client = createClient({
  file: 'world.umw',
  embedder: { spaceId: 'e5-small', embed: texts => model.embed(texts), autoEmbed: true },
});
await client.embeddingDefineSpace({
  projectId: 'world',
  space: { id: 'e5-small', dimensions: 384, model: 'intfloat/multilingual-e5-small' },
});
await client.embedPending({ projectId: 'world' });
const found = await client.semanticSearch({
  projectId: 'world',
  text: 'who took the necklace?',
  filter: { holder: 'ana' },
});
```

`embedPending` embeds every searchable document that has no vector yet, in
batches (`batchSize`, default 64). `semanticSearch` embeds the query and
searches, hybrid by default (`mode: 'semantic'` for meaning only). `autoEmbed`
embeds new documents after each write; it is off by default because it calls
the model on every commit. See
[search](https://github.com/Kallamo/Umwelt/blob/main/spec/search.md).

## Lower level

`StdioTransport` exposes `request(op, payload)` for generic protocol tooling;
applications should prefer the typed methods. `automaticHello: false` is
reserved for protocol test harnesses that send their own handshake.

## License

AGPL-3.0-only. Free, non-monetized games, mods, interactive fiction and
tabletop play tools may use Umwelt without releasing their own code under the
[Umwelt Free Play Exception](https://github.com/Kallamo/Umwelt/blob/main/LICENSE-EXCEPTION.md);
other uses that cannot comply with the AGPL can obtain a commercial license at
contact@kallamo.com. Built by [Kallamo](https://kallamo.com); see the
[repository](https://github.com/Kallamo/Umwelt) for the license, the
documentation and how to contribute.
