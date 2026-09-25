/**
 * The one logger every part of the app calls (18.B1/B2). Five channels, one `LogEntry` schema.
 * Seam-safe: this file never imports `src/mocks/*` — callers (services, stores, main.ts) pass
 * whatever business context they have (`setContext()`), and the mock/real backend forwards its
 * own posting traces in rather than this module reaching into the backend.
 *
 * Storage is pluggable per platform (desktop Tauri files vs. browser IndexedDB vs. dev-server
 * `/__diag` POST) — see `services/diagnosticsService.ts` for the actual persistence, which this
 * file only hands batched entries to. Writes are batched and must never throw into the caller.
 */
import { isTauri } from '@tauri-apps/api/core';
import { FLUSH_BATCH_SIZE, FLUSH_INTERVAL_MS, REDACTED_KEYS } from '../config';
import type { LogChannel, LogContext, LogEntry, LogLevel } from '../types';
const REDACTED_VALUE = '[محجوب]';
const REDACTED_KEY_SET = new Set(REDACTED_KEYS.map((k) => k.toLowerCase()));

function redact(value: unknown, depth = 0): unknown {
  if (depth > 6 || value == null) return value;
  if (Array.isArray(value)) return value.map((v) => redact(v, depth + 1));
  if (typeof value === 'object') {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      out[k] = REDACTED_KEY_SET.has(k.toLowerCase()) ? REDACTED_VALUE : redact(v, depth + 1);
    }
    return out;
  }
  return value;
}

/** Strips digits and ids from a message so "invoice inv-482 not found" and "invoice inv-501 not
 * found" produce the same fingerprint (same bug, different data) — 18.B1's whole point of
 * counting occurrences instead of flooding the log with near-duplicates. */
function normalizeForFingerprint(s: string): string {
  return s.replace(/[0-9a-f]{6,}/gi, '#').replace(/\d+/g, '#').trim().toLowerCase();
}

/** A small stable hash (not cryptographic — just needs to be deterministic and short). */
function hash(s: string): string {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return (h >>> 0).toString(16).padStart(8, '0');
}

export function fingerprintOf(name: string, message: string, firstAppFrame?: string): string {
  const basis = `${name}|${normalizeForFingerprint(message)}|${firstAppFrame ?? ''}`;
  return hash(basis);
}

/** First stack frame that points into this app's own source (not node_modules/vue-internals). */
function firstAppFrame(stack?: string): string | undefined {
  if (!stack) return undefined;
  const line = stack.split('\n').slice(1).find((l) => l.includes('/src/') && !l.includes('node_modules'));
  return line?.trim();
}

let sessionId = '';
try {
  sessionId = crypto.randomUUID();
} catch {
  sessionId = `${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

let appVersion = '0.0.0';
if (isTauri()) {
  import('@tauri-apps/api/app')
    .then(({ getVersion }) => getVersion())
    .then((v) => (appVersion = v))
    .catch(() => {});
}

const context: Omit<LogContext, 'sessionId' | 'appVersion'> = {};

/** Updates the ambient context (current user/branch/route) attached to every entry from now on.
 * Called from outside this module (e.g. after login, on route change) — never reads it itself. */
export function setLogContext(patch: Partial<Omit<LogContext, 'sessionId' | 'appVersion'>>): void {
  Object.assign(context, patch);
}

let currentCorrelationId: string | undefined;

/** Starts a new correlation id for one user action ("حفظ الفاتورة") and returns it — pass the
 * same id to `withCorrelation` for anything that action triggers (service call, posting trace,
 * any error), so 18.F's inspector can pull everything one action caused back together. */
export function newCorrelationId(): string {
  try {
    return crypto.randomUUID();
  } catch {
    return `${Date.now()}-${Math.random().toString(36).slice(2)}`;
  }
}

export function withCorrelation<T>(id: string, fn: () => T): T {
  const prev = currentCorrelationId;
  currentCorrelationId = id;
  try {
    return fn();
  } finally {
    currentCorrelationId = prev;
  }
}

type Sink = (entries: LogEntry[]) => void;
const sinks: Sink[] = [];
/** Registered by `diagnosticsService.ts` on startup — kept decoupled so `logService` has no
 * platform-specific storage code and no import cycle with the service that reads it back. */
export function registerSink(sink: Sink): void {
  sinks.push(sink);
}

const queue: LogEntry[] = [];
let flushTimer: ReturnType<typeof setTimeout> | undefined;

function scheduleFlush(): void {
  if (flushTimer) return;
  flushTimer = setTimeout(flush, FLUSH_INTERVAL_MS);
}

function flush(): void {
  flushTimer = undefined;
  if (!queue.length) return;
  const batch = queue.splice(0, queue.length);
  for (const sink of sinks) {
    try {
      sink(batch);
    } catch {
      /* a broken sink must never take the app down over logging */
    }
  }
}

function enqueue(entry: LogEntry): void {
  queue.push(entry);
  if (queue.length >= FLUSH_BATCH_SIZE) flush();
  else scheduleFlush();
}

/** Namespaces enabled for the `debug` channel (18.B5) — off by default, per device. */
function debugEnabled(namespace: string): boolean {
  let raw: string | null = null;
  try {
    raw = localStorage.getItem('equal.debug');
  } catch {
    return false;
  }
  if (!raw) return false;
  const patterns = raw.split(',').map((s) => s.trim()).filter(Boolean);
  return patterns.some((p) => (p.endsWith('.*') ? namespace.startsWith(p.slice(0, -2)) : p === namespace));
}

function write(channel: LogChannel, level: LogLevel, source: string, msg: string, data?: unknown, err?: Error): void {
  try {
    const entry: LogEntry = {
      v: 1,
      ts: new Date().toISOString(),
      channel,
      level,
      source,
      msg,
      data: data !== undefined ? redact(data) : undefined,
      err: err
        ? {
            name: err.name,
            message: err.message,
            stack: err.stack,
            fingerprint: fingerprintOf(err.name, err.message, firstAppFrame(err.stack)),
          }
        : undefined,
      ctx: { sessionId, appVersion, correlationId: currentCorrelationId, ...context },
    };
    enqueue(entry);
  } catch {
    /* logging must never throw into the caller */
  }
}

export const log = {
  /** Always on. `err` becomes the entry's fingerprinted error block when given. */
  error(source: string, msg: string, err?: Error, data?: unknown): void {
    write('error', 'error', source, msg, data, err);
  },
  /** Always on (cheap): service durations, route timing, long tasks, startup, budget breaches. */
  perf(source: string, msg: string, data?: unknown): void {
    write('perf', 'info', source, msg, data);
  },
  /** Off unless `namespace` is enabled via `localStorage['equal.debug']` or the dev overlay. */
  debug(namespace: string, msg: string, data?: unknown): void {
    if (!debugEnabled(namespace)) return;
    write('debug', 'debug', namespace, msg, data);
  },
  /** Written by the backend adapter (18.B4), not called directly from pages. */
  audit(source: string, msg: string, data?: unknown): void {
    write('audit', 'info', source, msg, data);
  },
  /** Posting traces (18.F) — debug-mode only for traces; invariant failures always log, via level 'error'. */
  accounting(source: string, msg: string, data?: unknown, level: LogLevel = 'info'): void {
    if (level !== 'error' && !debugEnabled('accounting')) return;
    write('accounting', level, source, msg, data);
  },
};

/** Flushes on tab/app close so the last batch isn't lost. */
if (typeof window !== 'undefined') {
  window.addEventListener('beforeunload', flush);
}
