// SPDX-License-Identifier: AGPL-3.0-only
// The bridge between the engine and the client's embedding model. The engine
// never runs a model: it lists the text to embed, stores the vectors and
// searches them. This layer calls the model the application configured.
import type * as T from './generated/contracts.js';
import { UmweltClient } from './generated/client.js';
import { StdioTransport } from './transport.js';
import type { StdioOptions } from './transport.js';

/** The application's embedding model, bound to one embedding space. */
export interface Embedder {
  /** The space the vectors belong to (`embedding.defineSpace`). */
  spaceId: string;
  /** One vector per text, in order, of the space's dimensions. */
  embed(texts: string[]): Promise<number[][]> | number[][];
  /** Texts per `embed` call when filling pending documents. Default 64. */
  batchSize?: number;
  /**
   * Embed new documents after every write that adds searchable text. Off by
   * default: it calls the model on every commit.
   */
  autoEmbed?: boolean;
}

export interface EmbedPendingOptions {
  projectId: string;
  /** Timeline to read; defaults to "main". */
  timelineId?: string;
  /**
   * Start after this position (a `resumeCursor`). Without it every document
   * is checked again.
   */
  cursor?: string;
}

export interface EmbedPendingResult {
  /** Vectors stored. */
  embedded: number;
  /** Pass it as `cursor` later to embed only documents written since. */
  resumeCursor: string;
}

/** `search` with a text that the embedder turns into the query vector. */
export type SemanticSearchPayload = Omit<T.SearchPayload, 'text' | 'semantic'> & {
  text: string;
  /** `hybrid` (default) also matches the words; `semantic` uses meaning only. */
  mode?: 'hybrid' | 'semantic';
};

export type ClientOptions = StdioOptions & { embedder?: Embedder };

/** Writes that can add searchable documents to the timeline they name. */
const WRITES = new Set([
  'commit', 'commitBatch', 'transmit', 'transmitBatch', 'witness', 'proposal.apply',
  'holding.adjustBatch', 'transition.batch', 'disposition.setBatch',
]);

/** Runs `afterWrite` once a write succeeded; used for `autoEmbed`. */
class EmbeddingTransport extends StdioTransport {
  afterWrite?: (op: string, payload: unknown) => Promise<void>;
  override async request<R>(op: string, payload: unknown): Promise<R> {
    const result = await super.request<R>(op, payload);
    if (this.afterWrite && WRITES.has(op)) await this.afterWrite(op, payload);
    return result;
  }
}

/** The generated client plus the embedding helpers. */
export class EmbeddingClient extends UmweltClient {
  private readonly resume = new Map<string, string>();

  constructor(transport: StdioTransport, readonly embedder?: Embedder) {
    super(transport);
    if (embedder?.autoEmbed && transport instanceof EmbeddingTransport) {
      transport.afterWrite = (op, payload) => this.embedAfterWrite(op, payload);
    }
  }

  /**
   * Embeds every searchable document of a timeline that has no vector (or a
   * stale one) in the embedder's space, in batches, until none is left.
   */
  async embedPending(options: EmbedPendingOptions): Promise<EmbedPendingResult> {
    const embedder = this.requireEmbedder();
    const key = `${options.projectId}\n${options.timelineId ?? 'main'}`;
    let cursor = options.cursor;
    let embedded = 0;
    for (;;) {
      const page = await this.embeddingPending({
        projectId: options.projectId,
        spaceId: embedder.spaceId,
        ...(options.timelineId ? { timelineId: options.timelineId } : {}),
        page: { limit: embedder.batchSize ?? 64, ...(cursor ? { cursor } : {}) },
      });
      if (page.items.length > 0) {
        const vectors = await embedder.embed(page.items.map(item => item.text));
        if (vectors.length !== page.items.length) {
          throw new Error(`The embedder returned ${vectors.length} vectors for ${page.items.length} texts.`);
        }
        const stored = await this.embeddingPut({
          projectId: options.projectId,
          spaceId: embedder.spaceId,
          items: page.items.map((item, index) => ({
            docType: item.docType,
            docId: item.docId,
            textHash: item.textHash,
            vector: vectors[index] ?? [],
          })),
        });
        embedded += stored.stored;
      }
      if (!page.nextCursor) {
        this.resume.set(key, page.resumeCursor);
        return { embedded, resumeCursor: page.resumeCursor };
      }
      cursor = page.nextCursor;
    }
  }

  /** Searches by meaning (and by words, in `hybrid` mode) with one call. */
  async semanticSearch(payload: SemanticSearchPayload): Promise<T.SearchPageResult> {
    const embedder = this.requireEmbedder();
    const { text, mode, ...rest } = payload;
    const [vector] = await embedder.embed([text]);
    if (!vector) throw new Error('The embedder returned no vector for the query.');
    return this.search({
      ...rest,
      ...(mode === 'semantic' ? {} : { text }),
      semantic: { spaceId: embedder.spaceId, vector },
    });
  }

  private async embedAfterWrite(op: string, payload: unknown): Promise<void> {
    const target = writeTarget(op, payload);
    if (!target) return;
    const key = `${target.projectId}\n${target.timelineId ?? 'main'}`;
    const cursor = this.resume.get(key);
    await this.embedPending({ ...target, ...(cursor ? { cursor } : {}) });
  }

  private requireEmbedder(): Embedder {
    if (!this.embedder) throw new Error('No embedder is configured: pass `embedder` to createClient.');
    return this.embedder;
  }
}

/** The project and timeline a write added documents to. */
function writeTarget(op: string, payload: unknown): { projectId: string; timelineId?: string } | undefined {
  const value = payload as Record<string, unknown> | undefined;
  const first = op === 'commitBatch'
    ? (value?.commits as Record<string, unknown>[] | undefined)?.[0]
    : op === 'transmitBatch'
      ? (value?.transmits as Record<string, unknown>[] | undefined)?.[0]
      : value;
  const projectId = first?.projectId;
  if (typeof projectId !== 'string') return undefined;
  const timelineId = first?.timelineId;
  return typeof timelineId === 'string' ? { projectId, timelineId } : { projectId };
}

export function createEmbeddingClient(options: ClientOptions): EmbeddingClient {
  const { embedder, ...stdio } = options;
  return new EmbeddingClient(new EmbeddingTransport(stdio), embedder);
}
