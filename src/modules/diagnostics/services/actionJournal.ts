/**
 * Action journal + repro bundles (18.F4). In debug mode (`equal.debug` includes `accounting`, the
 * same switch F1's posting trace uses), every wrapped service call (`defineService.ts`'s `wrap()`
 * — already the choke point for every module service function) is recorded here: name, args
 * (redacted the same way `logService` redacts secrets), and the id(s) it returned, on top of a
 * snapshot taken when recording started. Exporting bundles that JSON via `saveFile` gives a
 * headless replay (`bun run verify:replay <bundle>`, see `scripts/verify/replay.ts`) something
 * deterministic to re-run against a fresh mock backend, checking invariants after every step —
 * turning "it broke while I was clicking around" into a permanent regression case.
 *
 * Recording is opt-in and OFF by default: this module does nothing unless a caller starts it
 * explicitly (`startRecording()`), so normal app usage never pays for it. Seam-safe: this file
 * never imports `src/mocks/*` — `wrap()` calls `recordServiceCall` with only plain data, and the
 * snapshot is captured by whoever starts recording (a dev menu action, which is allowed to import
 * mocks, wires the actual `db` snapshot in — see `scripts/verify/replay.ts` for the headless side,
 * which reads its own `db` directly since it isn't page code).
 */
import { debugEnabled } from './logService';

export interface ActionJournalEntry {
  seq: number;
  at: string;
  /** The `wrap()` source name, e.g. "invoices.createSale". */
  source: string;
  /** Redacted args — same "N arg(s)" opacity `defineService.ts` already logs with, plus a shallow
   * JSON-safe copy when args are small and plain (so replay actually has something to call with). */
  args: unknown[];
  correlationId?: string;
  ok: boolean;
  /** Returned value's `id` field when the result looks like `{ id: string }` — the common shape
   * every create/post service returns; kept so a bundle can show what a step produced. */
  resultId?: string;
  error?: string;
}

export interface ReproBundle {
  v: 1;
  startedAt: string;
  endedAt: string;
  /** Whatever `startRecording(snapshot)` was given — the whole `MockDb` when called for real. */
  startSnapshot: unknown;
  actions: ActionJournalEntry[];
}

let recording = false;
let startedAt = '';
let startSnapshot: unknown;
let seq = 0;
const actions: ActionJournalEntry[] = [];

export function isRecording(): boolean {
  return recording;
}

/** Starts (or restarts) recording. `snapshot` should be a deep clone of the current `MockDb` —
 * the caller (dev menu action, which may import mocks) is responsible for cloning it; this module
 * only stores whatever it's given. */
export function startRecording(snapshot: unknown): void {
  recording = true;
  startedAt = new Date().toISOString();
  startSnapshot = snapshot;
  seq = 0;
  actions.length = 0;
}

export function stopRecording(): void {
  recording = false;
}

function safeArgsCopy(args: unknown[]): unknown[] {
  try {
    // Small/plain args only — mirrors logService's redact() depth guard so a bundle never grows
    // unbounded from a caller passing something huge (a File/Blob, a whole product list, …).
    const json = JSON.stringify(args);
    if (json.length > 20_000) return [`<${args.length} arg(s), too large to record>`];
    return JSON.parse(json);
  } catch {
    return [`<${args.length} arg(s), not JSON-safe>`];
  }
}

function extractResultId(result: unknown): string | undefined {
  if (result && typeof result === 'object' && 'id' in result && typeof (result as { id: unknown }).id === 'string') {
    return (result as { id: string }).id;
  }
  return undefined;
}

/** Called from `defineService.ts`'s `wrap()` after every service call resolves (success or
 * failure) — a no-op unless `startRecording()` was called first, so `wrap()` doesn't need its own
 * debug-mode check. */
export function recordServiceCall(opts: { source: string; args: unknown[]; correlationId?: string; ok: boolean; result?: unknown; error?: string }): void {
  if (!recording) return;
  actions.push({
    seq: seq++,
    at: new Date().toISOString(),
    source: opts.source,
    args: safeArgsCopy(opts.args),
    correlationId: opts.correlationId,
    ok: opts.ok,
    resultId: opts.ok ? extractResultId(opts.result) : undefined,
    error: opts.ok ? undefined : opts.error,
  });
}

export function currentJournal(): ActionJournalEntry[] {
  return [...actions];
}

/** Builds the exportable bundle — "تصدير حالة لإعادة الإنتاج" calls this then hands the JSON to
 * `saveFile`. Returns `undefined` if nothing was ever recorded. */
export function buildReproBundle(): ReproBundle | undefined {
  if (!startedAt) return undefined;
  return {
    v: 1,
    startedAt,
    endedAt: new Date().toISOString(),
    startSnapshot,
    actions: currentJournal(),
  };
}

/** Debug-mode gate for whether recording should even be offered in the UI — same `accounting`
 * namespace as the posting trace, so one toggle turns on both. */
export function debugRecordingAvailable(): boolean {
  return debugEnabled('accounting');
}
