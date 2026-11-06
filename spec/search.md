<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Search

`search` finds documents by their words, by their meaning, or both. Every mode
applies the same visibility as every other read: the timeline, the position
(on any axis), the status of each record and, with `filter.holder`, the
holder's own perspective. A document the read cannot see is never returned,
however well it matches.

## Searchable documents

| `docType` | Document | Text |
| --- | --- | --- |
| `record` | an active Fact | its subject, predicate, object and string qualifiers |
| `record` | a Holding with `text` | the `text` |
| `event` | an event whose type does not begin with `umwelt:` | its title and description |

Without `filter.holder`, a search returns Facts and events (the narrator's
view). With it, only that holder's active, available Holdings.

## Modes

| Payload | Mode | Order |
| --- | --- | --- |
| `text` | full-text (FTS5) | relevance (`bm25`), then `docType` and `docId` |
| `semantic` | semantic | cosine similarity, highest first, then `docType` and `docId` |
| `text` and `semantic` | hybrid | Reciprocal Rank Fusion, then `docType` and `docId` |

At least one of `text` and `semantic` is required. Each item reports `match`
(`text`, `semantic` or `both`) and, in the semantic and hybrid modes, the
`score` it was ranked by.

The text is turned into a safe FTS5 query: whitespace-separated terms are
quoted individually, embedded double quotes are doubled, and terms combine
with `AND`. Empty text in text mode returns an empty page.

Hybrid search fuses the two rankings with `score = Σ 1 / (60 + position)`,
positions counted from 1 in each ranking a document appears in. It does not
depend on the scale of either ranking, and a document found by both ranks
above one found by either alone.

Cosine similarity is computed in `f64`, in index order, over vectors
normalized when they are stored, so a search gives the same order and the
same scores on every platform. Search is exact: every visible document with a
vector is compared, so its cost grows with the number of candidates. With
`filter.holder` the candidates are that holder's documents; without it, every
Fact and event with a vector in the space. See
[../benchmarks/README.md](../benchmarks/README.md) for measured costs.

## Pages

The cursor is an offset. If the ledger changes between pages, results can
shift, so a page may skip or repeat items; keep the file unchanged while
paging, or start again after a write. A semantic or hybrid page reads at most
1,000 results (`offset + limit <= 1000`).

## Vectors come from the client

The engine never runs a model and never calls one. The client's model turns
text into vectors; the engine stores them, lists what still needs one, and
searches them.

### Embedding spaces

A vector is only comparable with vectors of the same model, so a project
declares **embedding spaces**:

- `embedding.defineSpace`: `{ projectId, space: { id, dimensions, model } }`.
  `dimensions` is between 1 and 4096; `model` documents the model and is never
  read by the engine. Ids beginning with `umwelt:` are reserved. The same
  definition again is a no-op; a different one fails with `CONFLICT`.
- `embedding.listSpaces`: `{ projectId }`.
- `embedding.dropSpace`: `{ projectId, spaceId }` → `{ removed }`. Removes the
  space and every vector in it.

A project may hold several spaces, for example while it moves to a new model.

### Filling vectors

- `embedding.pending`: `{ projectId, spaceId, timelineId?, docTypes?, page? }`
  → `{ items, nextCursor?, resumeCursor }`. Lists the searchable documents
  visible in the timeline whose vector in the space is missing or stale, in
  the order they were indexed, each with the exact `text` to embed and its
  `textHash` (SHA-256, 64 hex characters). `nextCursor` continues the listing;
  `resumeCursor` lets a later call see only documents indexed since. Without a
  cursor every document is checked again.
- `embedding.put`: `{ projectId, spaceId, items: [{ docType, docId, textHash,
  vector }] }` → `{ stored, stale }`. Stores or replaces vectors in one
  transaction. A vector must have the space's dimensions, finite values and
  not be all zeros; it is normalized before it is stored. An unknown document
  fails with `NOT_FOUND`; a document that is not searchable (an Awareness, a
  Holding without text) with `VALIDATION_FAILED`.
- `embedding.delete`: `{ projectId, spaceId, items: [{ docType, docId }] }` →
  `{ removed }`.

A vector stored with a `textHash` that is not its document's is **stale**: it
is kept, counted in `stale`, and listed again by `embedding.pending` until it
is replaced.

### Vectors are derived data

A vector is not part of history. It can be replaced or removed at any time,
it grants no knowledge and it never changes what a read can see; it only
orders documents that the read would see anyway. Vectors have no timeline of
their own: a draft's documents keep their vectors through a rebase (the ids do
not change), move with the draft on promote, and lose them on purge.

### Changing models

1. Define a space for the new model.
2. Fill it (`embedding.pending` and `embedding.put`), at any pace.
3. Search the new space.
4. Drop the old space.

No history is touched.

## Client libraries

The TypeScript client (`@kallamo/umwelt`) and the .NET binding
(`Kallamo.Umwelt`) fill and search vectors with the model the
application provides:

```ts
const client = createClient({
  file: 'story.umw',
  embedder: { spaceId: 'e5-small', embed: texts => model.embed(texts) },
});
await client.embedPending({ projectId: 'saga' });
await client.semanticSearch({ projectId: 'saga', text: 'who took the necklace?', filter: { holder: 'ana' } });
```

`embedPending` lists pending documents, calls the model in batches and stores
the vectors. `semanticSearch` embeds the query text and searches, hybrid by
default (`mode: 'semantic'` for meaning only). With `autoEmbed: true` the npm
client embeds new documents after every write that can add them. In .NET,
`IEmbedder` with `EmbedPendingAsync` and `SemanticSearchAsync` do the same.

Any model works. Models with a permissive license (MIT or Apache-2.0) that
cover many languages include `intfloat/multilingual-e5-small`,
`paraphrase-multilingual-MiniLM-L12-v2` and
`Alibaba-NLP/gte-multilingual-base`; check the license of the version you use.
