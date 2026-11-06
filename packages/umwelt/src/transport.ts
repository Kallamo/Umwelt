// SPDX-License-Identifier: AGPL-3.0-only
import { spawn } from 'node:child_process';
import type { ChildProcessWithoutNullStreams } from 'node:child_process';
import { createInterface } from 'node:readline';
import type { ErrorCode, HelloResult } from './generated/contracts.js';
import { resolveEngineBinary } from './engine-binary.js';

export class UmweltError extends Error {
  constructor(readonly code: ErrorCode | 'TRANSPORT_CLOSED', message: string, readonly details?: unknown) {
    super(message);
    this.name = 'UmweltError';
  }
}
export interface StdioOptions {
  file: string;
  create?: boolean;
  /**
   * The `umwelt` executable to run. Defaults to `UMWELT_BINARY`, then the
   * engine bundled for this platform, then `umwelt` from the PATH.
   */
  binary?: string;
  client?: string;
  onStderr?: (text: string) => void;
  /** Protocol conformance harnesses may send their own explicit hello. */
  automaticHello?: boolean;
}
type Pending = { resolve(value: unknown): void; reject(error: Error): void };
export class StdioTransport {
  private readonly process: ChildProcessWithoutNullStreams;
  private readonly pending = new Map<string, Pending>();
  private sequence = 0;
  private closed = false;
  private closing = false;
  private readonly exited: Promise<void>;
  readonly ready: Promise<HelloResult | undefined>;

  constructor(options: StdioOptions) {
    this.process = spawn(options.binary ?? resolveEngineBinary(), ['serve', options.file, ...(options.create ? ['--create'] : [])], { stdio: 'pipe' });
    this.exited = new Promise(resolve => this.process.once('close', () => { this.failAll('Engine process closed.'); resolve(); }));
    this.process.on('error', error => this.failAll(error.message));
    this.process.stdin.on('error', error => this.failAll(error.message));
    this.process.stderr.on('data', chunk => options.onStderr?.(String(chunk)));
    const lines = createInterface({ input: this.process.stdout });
    lines.on('line', line => {
      try {
        const envelope = JSON.parse(line) as { id: string; ok: boolean; result?: unknown; error?: { code: ErrorCode; message: string; details?: unknown } };
        const pending = this.pending.get(envelope.id);
        if (!pending) return;
        this.pending.delete(envelope.id);
        if (envelope.ok) pending.resolve(envelope.result);
        else if (envelope.error) pending.reject(new UmweltError(envelope.error.code, envelope.error.message, envelope.error.details));
        else pending.reject(new UmweltError('TRANSPORT_CLOSED', 'Malformed engine response.'));
      } catch { this.failAll('Invalid JSON from engine.'); this.process.kill(); }
    });
    this.ready = options.automaticHello === false ? Promise.resolve(undefined) : this.send<HelloResult>('hello', { protocol: { min: 1, max: 1 }, client: options.client ?? '@kallamo/umwelt/0.1.0' });
    // A failed startup is also delivered to subsequent request/close callers.
    void this.ready.catch(() => {});
  }
  private failAll(message: string): void {
    this.closed = true;
    for (const pending of this.pending.values()) pending.reject(new UmweltError('TRANSPORT_CLOSED', message));
    this.pending.clear();
  }
  private send<T>(op: string, payload: unknown): Promise<T> {
    if (this.closed) return Promise.reject(new UmweltError('TRANSPORT_CLOSED', 'Engine process is closed.'));
    const id = `sdk-${++this.sequence}`;
    return new Promise<T>((resolve, reject) => {
      this.pending.set(id, { resolve: value => resolve(value as T), reject });
      try { this.process.stdin.write(JSON.stringify({ v: 1, id, op, payload }) + '\n'); }
      catch (error) { this.pending.delete(id); reject(error); }
    });
  }
  async request<T>(op: string, payload: unknown): Promise<T> {
    await this.ready;
    if (this.closing) throw new UmweltError('TRANSPORT_CLOSED', 'Engine process is closing.');
    return this.send<T>(op, payload);
  }
  async close(): Promise<void> {
    let failure: unknown;
    if (!this.closing && !this.closed) {
      this.closing = true;
      try { await this.ready; await this.send('shutdown', {}); }
      catch (error) { failure = error; }
      finally { this.process.stdin.end(); }
    }
    await this.exited;
    if (failure) throw failure;
  }
}
