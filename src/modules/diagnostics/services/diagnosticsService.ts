/**
 * Diagnostics storage (18.B3) — registers a sink on `logService` and persists batches to whatever
 * this platform has:
 *   - Tauri desktop  -> Rust `diag_*` commands (JSONL files under the app's log dir).
 *   - browser/e2e    -> an IndexedDB ring buffer (last N per channel) + `window.__equal.diag.export()`.
 *   - `bun run dev`  -> also POSTs each batch to the Vite `/__diag` middleware (vite.config.ts),
 *                       which appends to `.diagnostics/logs/<channel>.jsonl` in the repo so agents
 *                       and developers can read runtime logs from disk while working.
 * This is the only file that talks to `src-tauri`'s `diag_*` commands or IndexedDB for logs —
 * everything else goes through `logService`.
 */
import { isTauri, invoke } from '@tauri-apps/api/core';
import { ERROR_RETENTION_DAYS, RETENTION_DAYS, RING_BUFFER_SIZE } from '../config';
import { registerSink } from './logService';
import type { LogChannel, LogEntry } from '../types';
const CHANNELS: LogChannel[] = ['error', 'perf', 'debug', 'audit', 'accounting'];

// --- IndexedDB ring buffer (browser / e2e) ----------------------------------------------------

const IDB_NAME = 'equal-diag';
const IDB_VERSION = 1;

function openIdb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(IDB_NAME, IDB_VERSION);
    req.onupgradeneeded = () => {
      for (const channel of CHANNELS) {
        if (!req.result.objectStoreNames.contains(channel)) {
          req.result.createObjectStore(channel, { keyPath: 'seq', autoIncrement: true });
        }
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

async function idbAppend(channel: LogChannel, entries: LogEntry[]): Promise<void> {
  const conn = await openIdb();
  await new Promise<void>((resolve, reject) => {
    const tx = conn.transaction(channel, 'readwrite');
    const store = tx.objectStore(channel);
    for (const entry of entries) store.add(entry);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
  await idbTrim(channel);
  conn.close();
}

/** Keeps only the last `RING_BUFFER_SIZE` entries per channel. */
async function idbTrim(channel: LogChannel): Promise<void> {
  const conn = await openIdb();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(channel, 'readwrite');
    const store = tx.objectStore(channel);
    const countReq = store.count();
    countReq.onsuccess = () => {
      const excess = countReq.result - RING_BUFFER_SIZE;
      if (excess <= 0) return;
      const cursorReq = store.openCursor();
      let deleted = 0;
      cursorReq.onsuccess = () => {
        const cursor = cursorReq.result;
        if (cursor && deleted < excess) {
          cursor.delete();
          deleted++;
          cursor.continue();
        }
      };
    };
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

async function idbReadAll(channel: LogChannel): Promise<LogEntry[]> {
  const conn = await openIdb();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(channel, 'readonly');
    const req = tx.objectStore(channel).getAll();
    req.onsuccess = () => resolve(req.result as LogEntry[]);
    req.onerror = () => reject(req.error);
    tx.oncomplete = () => conn.close();
  });
}

async function idbClear(channel?: LogChannel): Promise<void> {
  const conn = await openIdb();
  const targets = channel ? [channel] : CHANNELS;
  await new Promise<void>((resolve, reject) => {
    const tx = conn.transaction(targets, 'readwrite');
    for (const c of targets) tx.objectStore(c).clear();
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
  conn.close();
}

// --- Dev-server bridge (`bun run dev` only) ---------------------------------------------------

function postToDevMiddleware(channel: LogChannel, entries: LogEntry[]): void {
  if (!import.meta.env.DEV) return;
  fetch('/__diag', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ channel, entries }),
    keepalive: true,
  }).catch(() => {
    /* dev convenience only — never surfaces to the user */
  });
}

// --- Desktop (Tauri) ----------------------------------------------------------------------------

async function diagAppendDesktop(channel: LogChannel, entries: LogEntry[]): Promise<void> {
  const lines = entries.map((e) => JSON.stringify(e));
  await invoke('diag_append', { channel, lines });
}

export async function readLogs(channel: LogChannel, from: string, to: string): Promise<LogEntry[]> {
  if (isTauri()) {
    const lines = await invoke<string[]>('diag_read', { channel, from, to });
    return lines.map((l) => JSON.parse(l) as LogEntry).filter(Boolean);
  }
  const all = await idbReadAll(channel);
  return all.filter((e) => e.ts.slice(0, 10) >= from && e.ts.slice(0, 10) <= to);
}

export async function clearLogs(channel?: LogChannel): Promise<void> {
  if (isTauri()) {
    await invoke('diag_clear', { channel });
    return;
  }
  await idbClear(channel);
}

export async function openLogFolder(): Promise<void> {
  if (!isTauri()) return;
  await invoke('diag_open_folder');
}

/** Runs once on startup — deletes files/rows past retention. No-op cost on the UI thread. */
export async function rotateLogs(): Promise<void> {
  if (isTauri()) {
    await invoke('diag_rotate', { retentionDays: RETENTION_DAYS, errorRetentionDays: ERROR_RETENTION_DAYS });
  }
  // The IndexedDB path is already bounded by the ring buffer (idbTrim runs on every append), so
  // there is nothing extra to rotate there.
}

/** `window.__equal.diag.export()` — read by `scripts/e2e/run.py` after each flow (18.G) and by
 * the "تصدير ملف التشخيص" support-bundle action (18.B6). */
export async function exportAll(): Promise<Record<LogChannel, LogEntry[]>> {
  const today = new Date().toISOString().slice(0, 10);
  const from = '2000-01-01';
  const out = {} as Record<LogChannel, LogEntry[]>;
  for (const channel of CHANNELS) {
    out[channel] = await readLogs(channel, from, today);
  }
  return out;
}

let initialized = false;

/** Called once from `main.ts`. Wires `logService`'s sink to this platform's storage and exposes
 * the `window.__equal.diag` bridge for e2e/support-bundle export. */
export function initDiagnostics(): void {
  if (initialized) return;
  initialized = true;

  registerSink((entries) => {
    const byChannel = new Map<LogChannel, LogEntry[]>();
    for (const entry of entries) {
      const list = byChannel.get(entry.channel) ?? [];
      list.push(entry);
      byChannel.set(entry.channel, list);
    }
    for (const [channel, list] of byChannel) {
      if (isTauri()) {
        diagAppendDesktop(channel, list).catch(() => {});
      } else {
        idbAppend(channel, list).catch(() => {});
      }
      postToDevMiddleware(channel, list);
    }
  });

  void rotateLogs();

  if (typeof window !== 'undefined') {
    (window as any).__equal = (window as any).__equal ?? {};
    (window as any).__equal.diag = { export: exportAll, clear: clearLogs };
  }
}
