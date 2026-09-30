/**
 * stdio JSON-lines transport to the Rust `parity_host` (plan 21 Part 04, B-8). The wire contract is
 * `host-protocol.ts`. One host process serves a whole run (one database); a call that takes longer
 * than 60 s kills it, rejects every pending call, and the next case starts a fresh host
 * (memory `feedback_dont-block-on-long-runs`: record it and move on).
 */
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createWriteStream, existsSync, readdirSync, statSync, type WriteStream } from 'node:fs';
import { join } from 'node:path';
import type { ParityTransport } from '../../src/modules/core/services/backend';
import type { ClockArgs, HostInvariant, HostReply, ResetArgs, ResetReply } from './host-protocol';

export const CALL_TIMEOUT_MS = 60_000;

export const HOST_BIN_DIR = join(import.meta.dirname, '../../.diagnostics/parity/bin');

/** The newest `.diagnostics/parity/bin/parity_host-*.exe` (the manager's copy of each host build,
 * entry file §5), or null when none exists yet. */
export function newestHostExe(): string | null {
  if (!existsSync(HOST_BIN_DIR)) return null;
  const candidates = readdirSync(HOST_BIN_DIR)
    .filter((f) => /^parity_host.*\.exe$/i.test(f) || /^parity_host(-[\w.-]+)?$/.test(f))
    .map((f) => join(HOST_BIN_DIR, f))
    .sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs);
  return candidates[0] ?? null;
}

/** Thrown for harness-level failures (timeout, host exit, meta-command error) — never a parity diff. */
export class HostError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'HostError';
  }
}

interface Pending {
  cmd: string;
  resolve: (reply: HostReply) => void;
  reject: (err: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

export class HostTransport {
  private proc: ChildProcessWithoutNullStreams | null = null;
  private nextId = 1;
  private pending = new Map<number, Pending>();
  private buffer = '';
  private stderr: WriteStream | null = null;
  /** The last harness failure (timeout, crash, protocol error) since `clearFailure()` — `backendCall`
   * turns a transport rejection into an `INTERNAL` ApiError, so the runner checks this after a pass
   * to tell "Rust said INTERNAL" from "the host died". */
  lastFailure: string | null = null;

  constructor(
    private readonly exe: string,
    private readonly dbName: string,
    private readonly stderrLogPath: string,
  ) {}

  private start(): void {
    if (this.proc) return;
    this.stderr ??= createWriteStream(this.stderrLogPath, { flags: 'a' });
    // A `.ts`/`.js` host (a protocol stub for testing the runner) runs under this same Bun.
    const script = /\.[cm]?[jt]s$/i.test(this.exe);
    const [cmd, args] = script ? [process.execPath, [this.exe, '--db', this.dbName]] : [this.exe, ['--db', this.dbName]];
    const proc = spawn(cmd, args, { stdio: ['pipe', 'pipe', 'pipe'], env: process.env });
    this.proc = proc;
    this.buffer = '';
    proc.stdout.setEncoding('utf8');
    proc.stdout.on('data', (chunk: string) => this.onData(chunk));
    proc.stderr.on('data', (chunk: Buffer) => this.stderr?.write(chunk));
    proc.on('error', (err) => this.fail(`parity_host failed to start (${this.exe}): ${err.message}`));
    proc.on('exit', (code, signal) => {
      if (this.proc === proc) {
        this.proc = null;
        if (this.pending.size > 0) this.fail(`parity_host exited (code ${code}, signal ${signal}) with ${this.pending.size} call(s) pending`);
      }
    });
  }

  private onData(chunk: string): void {
    this.buffer += chunk;
    let nl: number;
    while ((nl = this.buffer.indexOf('\n')) >= 0) {
      const line = this.buffer.slice(0, nl).trim();
      this.buffer = this.buffer.slice(nl + 1);
      if (!line) continue;
      let reply: HostReply;
      try {
        reply = JSON.parse(line) as HostReply;
      } catch {
        this.fail(`parity_host wrote a non-JSON line to stdout (protocol: stdout carries replies only): ${line.slice(0, 200)}`);
        return;
      }
      const p = this.pending.get(reply.id);
      if (!p) {
        this.lastFailure = `parity_host replied to unknown request id ${String(reply.id)}`;
        continue;
      }
      clearTimeout(p.timer);
      this.pending.delete(reply.id);
      p.resolve(reply);
    }
  }

  /** Kills the host and rejects every pending call. The next call starts a fresh host. */
  private fail(message: string): void {
    this.lastFailure = message;
    const pending = [...this.pending.values()];
    this.pending.clear();
    for (const p of pending) {
      clearTimeout(p.timer);
      p.reject(new HostError(message));
    }
    this.kill();
  }

  kill(): void {
    const proc = this.proc;
    this.proc = null;
    if (proc && proc.exitCode === null) proc.kill();
  }

  /** Closes stdin (the host exits cleanly) and the stderr log. */
  async close(): Promise<void> {
    const proc = this.proc;
    this.proc = null;
    if (proc && proc.exitCode === null) {
      await new Promise<void>((resolve) => {
        const t = setTimeout(() => {
          proc.kill();
          resolve();
        }, 5_000);
        proc.once('exit', () => {
          clearTimeout(t);
          resolve();
        });
        proc.stdin.end();
      });
    }
    this.stderr?.end();
    this.stderr = null;
  }

  clearFailure(): void {
    this.lastFailure = null;
  }

  /** Sends one request and resolves with the raw reply (ok or not). Rejects only on a harness failure. */
  send(cmd: string, args: unknown): Promise<HostReply> {
    this.start();
    const proc = this.proc;
    if (!proc) return Promise.reject(new HostError(this.lastFailure ?? 'parity_host is not running'));
    const id = this.nextId++;
    return new Promise<HostReply>((resolve, reject) => {
      const timer = setTimeout(() => this.fail(`parity_host call "${cmd}" timed out after ${CALL_TIMEOUT_MS / 1000}s`), CALL_TIMEOUT_MS);
      this.pending.set(id, { cmd, resolve, reject, timer });
      proc.stdin.write(`${JSON.stringify({ id, cmd, args: args ?? null })}\n`);
    });
  }

  /** The `ParityTransport` handed to `setParityTransport`: resolves a command's value, rejects with
   * the host's `error` payload (`{code, message}` or a string) so `backendCall` converts it exactly
   * like a Tauri rejection. */
  readonly invoke: ParityTransport = async (command, payload) => {
    const reply = await this.send(command, payload?.args ?? null);
    if (reply.ok) return reply.value;
    throw reply.error;
  };

  private async meta<T>(cmd: string, args: unknown): Promise<T> {
    const reply = await this.send(cmd, args);
    if (!reply.ok) {
      const e = reply.error;
      throw new HostError(`parity_host ${cmd} failed: ${typeof e === 'string' ? e : `${e.code}: ${e.message}`}`);
    }
    return reply.value as T;
  }

  reset(args: ResetArgs): Promise<ResetReply> {
    return this.meta<ResetReply>('__reset', args);
  }

  /** `__reset_empty` → `{ terminalId }` (an older host replied `null`). */
  resetEmpty(): Promise<{ terminalId?: string } | null> {
    return this.meta<{ terminalId?: string } | null>('__reset_empty', null);
  }

  clock(iso: string | null): Promise<void> {
    const args: ClockArgs = { iso };
    return this.meta<void>('__clock', args);
  }

  invariants(): Promise<HostInvariant[]> {
    return this.meta<HostInvariant[]>('__invariants', null);
  }
}

/** `ResetReply.idPairs` → a plain map. Also accepts `{ mockId: uuid }` or `[{ old, new }]` so a small
 * serialization choice on the host side can't break the runner. */
export function pairsToMap(idPairs: unknown): Map<string, string> {
  const map = new Map<string, string>();
  if (Array.isArray(idPairs)) {
    for (const p of idPairs) {
      if (Array.isArray(p) && typeof p[0] === 'string' && typeof p[1] === 'string') map.set(p[0], p[1]);
      else if (p && typeof p === 'object') {
        const o = p as Record<string, unknown>;
        const from = o.old ?? o.mock ?? o.mockId ?? o.from;
        const to = o.new ?? o.id ?? o.rust ?? o.to;
        if (typeof from === 'string' && typeof to === 'string') map.set(from, to);
      }
    }
  } else if (idPairs && typeof idPairs === 'object') {
    for (const [k, v] of Object.entries(idPairs as Record<string, unknown>)) if (typeof v === 'string') map.set(k, v);
  }
  return map;
}

/**
 * The mock's terminal identity: the POS store's `terminalId = ref('pos-1')`
 * (`src/modules/invoices/controllers/usePosStore.ts`) and the seed's `TERMINAL = 'pos-1'`
 * (`src/mocks/seed/shifts.ts`). On Rust the terminal is server-owned (D-S1): `parity_host`'s own
 * `TerminalIdentity`, which `__reset` also hands the importer as `adopt_terminal` (00-import D-5 —
 * the single legacy terminal string adopts this machine's id), and reports back as `terminalId`.
 */
export const MOCK_TERMINAL_ID = 'pos-1';

/** Adds `MOCK_TERMINAL_ID ↔ terminalId` to a pass's id pairs — unless the importer already paired
 * `pos-1` with something else (a snapshot with several terminal strings is not adopted, D-5), in
 * which case the mock's terminal genuinely is not this machine and the diff must say so. */
export function addTerminalPair(pairs: Map<string, string>, terminalId: unknown): void {
  if (typeof terminalId !== 'string' || pairs.has(MOCK_TERMINAL_ID)) return;
  for (const r of pairs.values()) if (r === terminalId) return;
  pairs.set(MOCK_TERMINAL_ID, terminalId);
}
