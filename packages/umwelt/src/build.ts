// SPDX-License-Identifier: AGPL-3.0-only
// Factories for the write contract. Each one is pure and returns a plain
// contract object: no state, no generated ids, no call to the engine. What is
// not passed stays omitted and takes the engine's documented write default
// (a record's position and provenance come from the commit's event; a Holding
// defaults to confidence 1 and `available`).
//
// Every factory has a line in `conformance/builders.json`, which the .NET
// binding checks too, so both languages build the same JSON.
import type {
  Accessibility,
  Acquisition,
  Attitude,
  Claim,
  ClaimObject,
  Intent,
  Position,
  Provenance,
  RecordInput,
  Secrecy,
} from './generated/contracts.js';

/** The Fact variant of `RecordInput`. */
export type FactInput = Extract<RecordInput, { kind: 'fact' }>;
/** The Holding variant of `RecordInput`. */
export type HoldingInput = Extract<RecordInput, { kind: 'holding' }>;

export interface ClaimOptions {
  /** Defaults to `true`. */
  polarity?: boolean;
  qualifiers?: { [key: string]: unknown };
  /** The record this claim is about. */
  about?: string;
}

export interface RecordOptions {
  id?: string;
  /** Omitted: the position of the commit's event. */
  position?: Position;
  /** Omitted: the provenance of the commit's event. */
  provenance?: Provenance;
}

export interface HoldingOptions extends RecordOptions {
  /** Omitted: 1. */
  confidence?: number;
  /** Omitted: `available`. */
  accessibility?: Accessibility;
  text?: string;
  secrecy?: Secrecy;
}

/** Copies only the keys that were given, so nothing is sent as `undefined`. */
function given<T extends object>(options: T | undefined, keys: ReadonlyArray<keyof T>): Partial<T> {
  const result: Partial<T> = {};
  if (options === undefined) return result;
  for (const key of keys) {
    if (options[key] !== undefined) result[key] = options[key];
  }
  return result;
}

function makeClaim(subject: string, predicate: string, object: ClaimObject, options?: ClaimOptions): Claim {
  return {
    subject,
    predicate,
    object,
    polarity: options?.polarity ?? true,
    ...given(options, ['qualifiers', 'about']),
  };
}

/** Claims, by the kind of their object. */
export const claim = {
  /** `subject predicate entity`, e.g. `claim.entity('bob', 'stole', 'necklace')`. */
  entity: (subject: string, predicate: string, objectId: string, options?: ClaimOptions): Claim =>
    makeClaim(subject, predicate, { kind: 'entity', id: objectId }, options),
  /** `subject predicate value`, e.g. `claim.value('bob', 'age', 42)`. */
  value: (subject: string, predicate: string, value: unknown, options?: ClaimOptions): Claim =>
    makeClaim(subject, predicate, { kind: 'value', value }, options),
  /** `subject predicate`, with no object, e.g. `claim.unary('bob', 'asleep')`. */
  unary: (subject: string, predicate: string, options?: ClaimOptions): Claim =>
    makeClaim(subject, predicate, { kind: 'none' }, options),
  /**
   * A claim about what a holder holds: second-order knowledge. The key of the
   * claim comes from the engine (`client.claimKey`), never from a hash made here.
   */
  holds: (holder: string, claimKey: string, options?: ClaimOptions): Claim =>
    makeClaim(holder, 'umwelt:holds', { kind: 'value', value: { claimKey } }, options),
};

/** The same claim with the opposite polarity. */
export function negate(value: Claim): Claim {
  return { ...value, polarity: !value.polarity };
}

/** A position on the main axis. */
export function at(tick: number): Position {
  return { tick };
}

/** Provenance, by who or what wrote the data. */
export const by = {
  human: (actorId?: string): Provenance => ({
    origin: { kind: 'human', ...(actorId === undefined ? {} : { actorId }) },
  }),
  model: (model: string, promptHash?: string): Provenance => ({
    origin: { kind: 'model', model, ...(promptHash === undefined ? {} : { promptHash }) },
  }),
  system: (name: string): Provenance => ({ origin: { kind: 'system', name } }),
  imported: (): Provenance => ({ origin: { kind: 'import' } }),
};

/** The holder was there: acquired at the event `eventId`. */
export function witnessed(eventId: string): Acquisition {
  return { kind: 'witnessed', eventId };
}

/** Someone told the holder, optionally from one of their own Holdings. */
export function toldBy(sourceHolderId: string, intent: Intent, sourceRecordId?: string): Acquisition {
  return {
    kind: 'told-by',
    sourceHolderId,
    intent,
    ...(sourceRecordId === undefined ? {} : { sourceRecordId }),
  };
}

/** The holder read or saw a piece of evidence. */
export function evidence(evidenceId: string): Acquisition {
  return { kind: 'evidence', evidenceId };
}

/** The holder concluded it from other records. */
export function inferred(fromRecordIds: string[], rule?: string): Acquisition {
  return { kind: 'inferred', fromRecordIds, ...(rule === undefined ? {} : { rule }) };
}

/** The author says so: no in-world source. */
export function authored(): Acquisition {
  return { kind: 'authored' };
}

/** Canon: what is true in the world, whoever knows it. */
export function fact(value: Claim, options?: RecordOptions): FactInput {
  return { kind: 'fact', claim: value, ...given(options, ['id', 'position', 'provenance']) };
}

function holding(attitude: Attitude) {
  return (holder: string, value: Claim, acquisition: Acquisition, options?: HoldingOptions): HoldingInput => ({
    kind: 'holding',
    holder,
    attitude,
    claim: value,
    acquisition,
    ...given(options, ['id', 'position', 'provenance', 'confidence', 'accessibility', 'text', 'secrecy']),
  });
}

/** A Holding with attitude `knows`. How the holder came to know is always explicit. */
export const knows = holding('knows');
/** A Holding with attitude `believes`. */
export const believes = holding('believes');
/** A Holding with attitude `suspects`. */
export const suspects = holding('suspects');
/** A Holding with attitude `remembers`. */
export const remembers = holding('remembers');
