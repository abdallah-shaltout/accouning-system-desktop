/**
 * IndexedDB persistence for the mock DB. Services wrap their mutating entry points in `mutate()`,
 * which schedules a debounced (500ms) snapshot of every `MockDb` table to IndexedDB. Attachment
 * blobs are NOT handled here — that's a separate Phase 0 track's object store.
 *
 * Boot sequence (see `bootMockDb()` below, called once from `main.ts`):
 *   - snapshot found  -> load it (through `migrations` if its stored version is older than current)
 *   - no snapshot     -> leave `db` empty; the welcome screen decides (demo seed / empty company)
 */
import { db, type MockDb } from './db';
import { clone } from './utils';

const DB_NAME = 'mock-db';
const STORE_NAME = 'snapshot';
const SNAPSHOT_KEY = 'current';

/** Bump this whenever `MockDb`'s shape changes in a way old snapshots can't be loaded as-is. */
export const SCHEMA_VERSION = 1;

interface Snapshot {
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

async function idbGet(key: string): Promise<Snapshot | undefined> {
  const conn = await openDb();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(STORE_NAME, 'readonly');
    const req = tx.objectStore(STORE_NAME).get(key);
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
    tx.oncomplete = () => conn.close();
  });
}

async function idbSet(key: string, value: Snapshot): Promise<void> {
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
  const conn = await openDb();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(STORE_NAME, 'readwrite');
    tx.objectStore(STORE_NAME).delete(SNAPSHOT_KEY);
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
 */
export function mutate<T>(fn: () => T): T {
  const result = fn();
  scheduleSnapshot();
  return result;
}

// --- Boot side ----------------------------------------------------------------------------------

/**
 * Looks for a persisted snapshot and loads it into `db` if found (migrating first if needed).
 * Returns true if a snapshot was loaded, false if none existed (db is left untouched/empty).
 */
export async function loadSnapshot(): Promise<boolean> {
  try {
    const snapshot = await idbGet(SNAPSHOT_KEY);
    if (!snapshot) return false;
    const data = snapshot.version < SCHEMA_VERSION ? runMigrations(snapshot.data, snapshot.version) : snapshot.data;
    Object.assign(db, clone(data));
    return true;
  } catch (err) {
    console.error('[mocks/persist] failed to load snapshot', err);
    return false;
  }
}

/** Dev-menu "reset data": clears the persisted snapshot. Caller is responsible for reloading the app. */
export async function clearSnapshot(): Promise<void> {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
    debounceTimer = null;
  }
  await idbClear();
}
