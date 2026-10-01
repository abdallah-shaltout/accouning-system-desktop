/**
 * IndexedDB persistence for the mock DB. Services wrap their mutating entry points in `mutate()`,
 * which schedules a debounced (500ms) snapshot of every `MockDb` table to IndexedDB. Attachment
 * blobs are NOT handled here — that's a separate Phase 0 track's object store.
 *
 * Boot sequence (see `bootMockDb()` below, called once from `main.ts`):
 *   - snapshot found  -> load it (through `migrations` if its stored version is older than current)
 *   - no snapshot     -> leave `db` empty; the welcome screen decides (demo seed / empty company)
 *
 * Rust mode (plan 21 Part 04, E-3, P4-9 — `usesRustEverywhere()`): the snapshot is no longer the
 * app's data, it is the user's **legacy** data waiting for its one-time import. `main.ts` skips
 * `bootMockDb()`, `mutate()`/`flushSnapshot()` never write it and `clearSnapshot()` refuses, so the
 * app can neither overwrite nor delete it. `readPersistedSnapshot()` still reads it for the importer,
 * and the import marker (`LEGACY_IMPORT_MARKER_KEY`) is a separate record in the same store.
 */
import { usesRustEverywhere } from '@/modules/core/services/backend';
import { db, type MockDb } from './db';
import { ApiError, bumpIdCounter, clone } from './utils';

const DB_NAME = 'mock-db';
const STORE_NAME = 'snapshot';
const SNAPSHOT_KEY = 'current';
/** P4-9 / E-4: the "legacy data already imported" marker — its own key next to `current`, which it
 * never touches (the app never deletes the legacy snapshot). */
const LEGACY_IMPORT_MARKER_KEY = 'legacyImportedAt';

/** Bump this whenever `MockDb`'s shape changes in a way old snapshots can't be loaded as-is. */
export const SCHEMA_VERSION = 1;

export interface Snapshot {
  version: number;
  savedAt: string;
  data: MockDb;
}

/**
 * Upgrades a snapshot's `data` from its stored version up to `SCHEMA_VERSION`, running each
 * migration in order. Empty for now (version 1 is the only version) — later phases add
 * `2: (old) => ({ ...old, newField: ... })`, etc.
 */
export const migrations: Record<number, (old: any) => any> = {};

function runMigrations(data: any, fromVersion: number): MockDb {
  let result = data;
  for (let v = fromVersion; v < SCHEMA_VERSION; v++) {
    const migrate = migrations[v];
    if (migrate) result = migrate(result);
  }
  return result as MockDb;
}

// --- IndexedDB plumbing ---------------------------------------------------------------------

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    // No explicit version: opens at whatever version the database is currently at (1 for a brand
    // new database — `onupgradeneeded` still fires and creates `STORE_NAME` in that case). This
    // module used to hardcode version 1, which threw `VersionError` as soon as `attachments.ts`
    // (opened at version 2) had touched the database first in the session — IndexedDB refuses
    // `open(name, v)` whenever `v` is lower than the database's current on-disk version, regardless
    // of call order. Omitting the version avoids the two modules needing to agree on one at all.
    const req = indexedDB.open(DB_NAME);
    req.onupgradeneeded = () => {
      if (!req.result.objectStoreNames.contains(STORE_NAME)) req.result.createObjectStore(STORE_NAME);
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

async function idbGet<T>(key: string): Promise<T | undefined> {
  const conn = await openDb();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(STORE_NAME, 'readonly');
    const req = tx.objectStore(STORE_NAME).get(key);
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
    tx.oncomplete = () => conn.close();
  });
}

async function idbSet(key: string, value: unknown): Promise<void> {
  const conn = await openDb();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(STORE_NAME, 'readwrite');
    tx.objectStore(STORE_NAME).put(value, key);
    tx.oncomplete = () => {
      conn.close();
      resolve();
    };
    tx.onerror = () => reject(tx.error);
  });
}

async function idbClear(): Promise<void> {
  return idbDelete(SNAPSHOT_KEY);
}

async function idbDelete(key: string): Promise<void> {
  const conn = await openDb();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(STORE_NAME, 'readwrite');
    tx.objectStore(STORE_NAME).delete(key);
    tx.oncomplete = () => {
      conn.close();
      resolve();
    };
    tx.onerror = () => reject(tx.error);
  });
}

// --- Save side --------------------------------------------------------------------------------

let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let saving: Promise<void> | null = null;
/** Resolved once the most recent scheduled snapshot has actually been written — dev-menu/tests can await it. */
let pendingSaveResolvers: (() => void)[] = [];

function writeSnapshotNow(): Promise<void> {
  // E-3: in Rust mode the stored snapshot is the user's legacy data (P4-9) — never overwrite it.
  if (usesRustEverywhere()) {
    const resolvers = pendingSaveResolvers;
    pendingSaveResolvers = [];
    resolvers.forEach((r) => r());
    return Promise.resolve();
  }
  const snapshot: Snapshot = { version: SCHEMA_VERSION, savedAt: new Date().toISOString(), data: clone(db) };
  saving = idbSet(SNAPSHOT_KEY, snapshot)
    .catch((err) => {
      // Persistence is best-effort: a mock UI shouldn't crash because a browser blocked storage.
      console.error('[mocks/persist] failed to save snapshot', err);
    })
    .finally(() => {
      saving = null;
      const resolvers = pendingSaveResolvers;
      pendingSaveResolvers = [];
      resolvers.forEach((r) => r());
    });
  return saving;
}

function scheduleSnapshot(): void {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    debounceTimer = null;
    void writeSnapshotNow();
  }, 500);
}

/** Wait for any in-flight or scheduled snapshot save to finish. Useful for tests / the dev menu. */
export function flushSnapshot(): Promise<void> {
  return new Promise((resolve) => {
    if (debounceTimer) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
      void writeSnapshotNow().then(resolve);
    } else if (saving) {
      pendingSaveResolvers.push(resolve);
    } else {
      resolve();
    }
  });
}

/**
 * Thin wrapper services use around writes: runs `fn`, then schedules a debounced snapshot.
 * `fn` runs synchronously (the mock backend is all synchronous), so this returns its result.
 * In Rust mode (E-3) nothing is scheduled: the in-memory `db` is not the app's data there, and
 * writing it would overwrite the legacy snapshot before it is imported.
 */
export function mutate<T>(fn: () => T): T {
  const result = fn();
  if (!usesRustEverywhere()) scheduleSnapshot();
  return result;
}

// --- Boot side ----------------------------------------------------------------------------------

/**
 * Looks for a persisted snapshot and loads it into `db` if found (migrating first if needed).
 * Returns true if a snapshot was loaded, false if none existed (db is left untouched/empty).
 */
export async function loadSnapshot(): Promise<boolean> {
  try {
    const snapshot = await idbGet<Snapshot>(SNAPSHOT_KEY);
    if (!snapshot) return false;
    const data = snapshot.version < SCHEMA_VERSION ? runMigrations(snapshot.data, snapshot.version) : snapshot.data;
    Object.assign(db, clone(data));
    resyncIdCounters(db);
    return true;
  } catch (err) {
    console.error('[mocks/persist] failed to load snapshot', err);
    return false;
  }
}

/**
 * `uid()`'s counters live in a module variable, not in the persisted snapshot, so right after
 * loading one they'd start back at 1 for every prefix while the restored `db` already has ids
 * like `je-884` — the next `uid('je')` would collide with an existing record (see `bumpIdCounter`'s
 * doc comment in utils.ts). This walks the whole restored `db` once, finds every `"prefix-123"`
 * shaped id, and bumps each prefix's counter to at least the highest number found — generic over
 * every table so no module needs its own resync call.
 */
function resyncIdCounters(data: MockDb): void {
  const idPattern = /^([a-z]+)-(\d+)$/;
  const visited = new Set<unknown>();
  function walk(value: unknown): void {
    if (!value || typeof value !== 'object') return;
    if (visited.has(value)) return;
    visited.add(value);
    if (Array.isArray(value)) {
      for (const item of value) walk(item);
      return;
    }
    // `entityId` too: an audit row whose link names no entity gets a placeholder `uid('unk')`
    // (`core.ts` `entityFromLink`) that exists only there — without it the counter restarted at
    // `unk-1` after a reload and re-issued placeholders already used by older rows (Part 04 Wave 2).
    for (const field of ['id', 'entityId'] as const) {
      const id = (value as Record<string, unknown>)[field];
      if (typeof id === 'string') {
        const match = idPattern.exec(id);
        if (match) bumpIdCounter(match[1], Number(match[2]));
      }
    }
    for (const key of Object.keys(value)) walk((value as Record<string, unknown>)[key]);
  }
  walk(data);
}

/**
 * D10 (21.03 §00-import): reads the persisted IndexedDB snapshot **without** loading it into `db` —
 * used only by `setup/services/legacyImportService.ts` to hand the raw snapshot to the Rust importer
 * (`setup_inspect_legacy_snapshot`/`setup_import_snapshot`). The seam rule allows a **service** to
 * import mocks directly; this export exists so that service never has to reach into `persist.ts`'s
 * private IndexedDB plumbing (`idbGet`/`SNAPSHOT_KEY`) itself.
 */
export async function readPersistedSnapshot(): Promise<Snapshot | undefined> {
  return idbGet<Snapshot>(SNAPSHOT_KEY);
}

/** The P4-9 import marker's stored shape. */
export interface LegacyImportMarker {
  importedAt: string;
}

/** E-4 (P4-9): when this PC's legacy snapshot was imported into the real database, or `undefined`
 * if it never was. Read by `setup/services/legacyImportService.ts` to hide the import card. */
export async function readLegacyImportMarker(): Promise<LegacyImportMarker | undefined> {
  return idbGet<LegacyImportMarker>(LEGACY_IMPORT_MARKER_KEY);
}

/** E-4 (P4-9): records a successful legacy import under its own key. Never touches `current`: the
 * legacy snapshot stays on this PC, untouched, after the import. */
export async function writeLegacyImportMarker(importedAt: string): Promise<void> {
  const marker: LegacyImportMarker = { importedAt };
  await idbSet(LEGACY_IMPORT_MARKER_KEY, marker);
}

/** Dev-menu "reset data": clears the persisted snapshot. Caller is responsible for reloading the app.
 * Refuses in Rust mode (E-3): there the snapshot is legacy data that may not be imported yet, and
 * deleting it would lose it for good (zero data loss, P4-9). */
export async function clearSnapshot(): Promise<void> {
  if (usesRustEverywhere()) {
    throw new ApiError('إعادة التعيين غير متاحة مع قاعدة البيانات الحقيقية — بيانات الإصدار السابق على هذا الجهاز لا تُحذف', 'FORBIDDEN');
  }
  if (debounceTimer) {
    clearTimeout(debounceTimer);
    debounceTimer = null;
  }
  await idbClear();
}

/** ACC-0035: the explicit, dev-only escape hatch `clearSnapshot()` deliberately doesn't have — a
 * developer's own test machine can accumulate an old legacy snapshot (from an earlier mock session,
 * before this device ever had a real database) that the welcome page keeps offering to import. Unlike
 * `clearSnapshot()`, this also deletes the `legacyImportedAt` marker, so a stale "already imported"
 * state can't linger either. Dev builds only, by design — a release build must never be able to make
 * a real customer's not-yet-imported legacy data unrecoverable. */
export async function forgetLegacySnapshot(): Promise<void> {
  if (!import.meta.env.DEV) {
    throw new ApiError('متاح فقط في وضع التطوير', 'FORBIDDEN');
  }
  await idbDelete(SNAPSHOT_KEY);
  await idbDelete(LEGACY_IMPORT_MARKER_KEY);
}
