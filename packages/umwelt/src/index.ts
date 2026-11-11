// SPDX-License-Identifier: AGPL-3.0-only

export type * from './generated/contracts.js';
export { UmweltClient } from './generated/client.js';
export { StdioTransport, UmweltError } from './transport.js';
export type { StdioOptions } from './transport.js';
export { BUNDLED_PLATFORMS, resolveEngineBinary } from './engine-binary.js';
export {
  at,
  authored,
  believes,
  by,
  claim,
  evidence,
  fact,
  inferred,
  knows,
  negate,
  remembers,
  suspects,
  toldBy,
  witnessed,
} from './build.js';
export type { ClaimOptions, FactInput, HoldingInput, HoldingOptions, RecordOptions } from './build.js';
export { EmbeddingClient } from './embedding.js';
export type {
  ClientOptions,
  EmbedPendingOptions,
  EmbedPendingResult,
  Embedder,
  SemanticSearchPayload,
} from './embedding.js';
import { createEmbeddingClient } from './embedding.js';
import type { ClientOptions, EmbeddingClient } from './embedding.js';

/**
 * A client over `umwelt serve`. With `embedder`, it also embeds documents
 * with the application's model (`embedPending`) and searches by meaning
 * (`semanticSearch`).
 */
export function createClient(options: ClientOptions): EmbeddingClient {
  return createEmbeddingClient(options);
}
