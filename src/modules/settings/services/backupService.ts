/**
 * Backup & restore orchestration (docs/v2/14-platform.md §4). UI-only against the mock backend:
 * everything here reads/writes `src/mocks/persist.ts`'s snapshot and `src/mocks/attachments.ts`'s
 * blob store, plus (Tauri mode) real files via `@tauri-apps/plugin-fs`/`@tauri-apps/plugin-dialog`,
 * or (browser mode) a small IndexedDB store of the last 5 snapshots.
 *
 * Tauri vs browser detection matches the rest of the app (`isTauri()` from `@tauri-apps/api/core`,
 * same as `modules/reports/helpers/export.ts`).
 */
import { isTauri } from '@tauri-apps/api/core';
import { db, session } from '@/mocks/db';
import { mutate, migrations, SCHEMA_VERSION, flushSnapshot } from '@/mocks/persist';
import { clone } from '@/mocks/utils';
import { replaceAllAttachments } from '@/mocks/attachments';
import { logActivity } from '@/mocks/backend/core';
import {
  buildBackupArchive,
  backupFileName,
  parseArchive,
  decryptArchive,
  readManifest,
  archiveAttachmentToRecord,
  type BackupData,
} from '../helpers/backupArchive';
import { sha256Hex } from '../helpers/backupCrypto';
import { updateSettings } from './settingsService';
import type { BackupHistoryEntry, BackupKind, BackupManifest, BackupSettings, RestorePreview } from '../types/backup';
import { DEFAULT_BACKUP_SETTINGS } from '../types/backup';

export function backupSettings(): BackupSettings {
  return { ...DEFAULT_BACKUP_SETTINGS, ...(db.settings.backup ?? {}) };
}

export async function saveBackupSettings(patch: Partial<BackupSettings>): Promise<BackupSettings> {
  const next = { ...backupSettings(), ...patch };
  await updateSettings({ backup: next });
  return next;
}

// --- Tauri detection -------------------------------------------------------------------------

export function isTauriMode(): boolean {
  return isTauri();
}

// --- browser-mode history (IndexedDB, last N snapshots) --------------------------------------

// Deliberately a SEPARATE IndexedDB database from `mock-db` (not just a new store inside it).
// `persist.ts` hardcodes `indexedDB.open('mock-db', 1)` forever (its comment explains this is
// intentional: `attachments.ts` opens the same database at version 2 and is a strict superset, so
// persist.ts's v1 open still succeeds against a v2-upgraded database). Once a database's on-disk
// version is bumped, any later `open(name, olderVersion)` call throws `VersionError` — so a v3 open
// here would have permanently broken persist.ts's v1 open. A separate database sidesteps that
// coupling entirely instead of extending a version chain a different Phase 0 file depends on.
const BROWSER_DB_NAME = 'mock-db-backup-history';
const BROWSER_DB_VERSION = 1;
const BROWSER_STORE = 'backup-history';
const BROWSER_KEEP = 5;

function openBrowserStore(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(BROWSER_DB_NAME, BROWSER_DB_VERSION);
    req.onupgradeneeded = () => {
      if (!req.result.objectStoreNames.contains(BROWSER_STORE)) req.result.createObjectStore(BROWSER_STORE, { keyPath: 'id' });
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

interface BrowserBackupRecord {
  id: string;
  manifest: BackupManifest;
  bytes: Uint8Array;
  sizeBytes: number;
  savedAt: string;
}

async function browserPut(record: BrowserBackupRecord): Promise<void> {
  const conn = await openBrowserStore();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(BROWSER_STORE, 'readwrite');
    tx.objectStore(BROWSER_STORE).put(record);
    tx.oncomplete = () => {
      conn.close();
      resolve();
    };
    tx.onerror = () => reject(tx.error);
  });
}

async function browserGetAll(): Promise<BrowserBackupRecord[]> {
  const conn = await openBrowserStore();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(BROWSER_STORE, 'readonly');
    const req = tx.objectStore(BROWSER_STORE).getAll();
    req.onsuccess = () => resolve((req.result as BrowserBackupRecord[]).sort((a, b) => b.savedAt.localeCompare(a.savedAt)));
    req.onerror = () => reject(req.error);
    tx.oncomplete = () => conn.close();
  });
}

async function browserDelete(id: string): Promise<void> {
  const conn = await openBrowserStore();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(BROWSER_STORE, 'readwrite');
    tx.objectStore(BROWSER_STORE).delete(id);
    tx.oncomplete = () => {
      conn.close();
      resolve();
    };
    tx.onerror = () => reject(tx.error);
  });
}

async function browserGet(id: string): Promise<BrowserBackupRecord | undefined> {
  const conn = await openBrowserStore();
  return new Promise((resolve, reject) => {
    const tx = conn.transaction(BROWSER_STORE, 'readonly');
    const req = tx.objectStore(BROWSER_STORE).get(id);
    req.onsuccess = () => resolve(req.result as BrowserBackupRecord | undefined);
    req.onerror = () => reject(req.error);
    tx.oncomplete = () => conn.close();
  });
}

async function pruneBrowserHistory(keep: number): Promise<void> {
  const all = await browserGetAll();
  const toDelete = all.slice(keep);
  for (const rec of toDelete) await browserDelete(rec.id);
}

// --- back up now -----------------------------------------------------------------------------

export interface BackupNowResult {
  manifest: BackupManifest;
  sizeBytes: number;
  path?: string;
  cancelled?: boolean;
}

/**
 * Runs the whole "نسخة الآن" flow: builds the archive, then either shows a Tauri save dialog and
 * writes the file, or triggers a browser download (and always keeps a browser-mode history copy
 * in IndexedDB, capped at the last 5, regardless of Tauri/browser — cheap and lets "السجل" work
 * even before a folder is chosen in Tauri mode... but Tauri mode instead lists the real folder,
 * see `listHistory()`).
 */
export async function backupNow(kind: BackupKind, password?: string, opts?: { suggestedPath?: string }): Promise<BackupNowResult> {
  const { bytes, manifest } = await buildBackupArchive(kind, password);
  const filename = backupFileName(manifest.company);

  if (isTauriMode()) {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const { writeFile, mkdir, exists } = await import('@tauri-apps/plugin-fs');
    let targetPath: string | null;
    if (opts?.suggestedPath) {
      // Automatic backups reuse the configured folder without prompting.
      targetPath = opts.suggestedPath;
      const folder = targetPath.slice(0, Math.max(targetPath.lastIndexOf('/'), targetPath.lastIndexOf('\\')));
      if (folder && !(await exists(folder))) await mkdir(folder, { recursive: true });
    } else {
      targetPath = await save({ defaultPath: filename, filters: [{ name: 'Backup', extensions: ['zip'] }] });
    }
    if (!targetPath) return { manifest, sizeBytes: bytes.length, cancelled: true };
    await writeFile(targetPath, bytes);
    await afterBackupSaved(manifest, kind);
    return { manifest, sizeBytes: bytes.length, path: targetPath };
  }

  // Browser mode: download + keep a rolling history in IndexedDB.
  const blob = new Blob([bytes.slice().buffer], { type: 'application/zip' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);

  await browserPut({ id: `bak-${Date.now()}`, manifest, bytes, sizeBytes: bytes.length, savedAt: manifest.createdAt });
  await pruneBrowserHistory(BROWSER_KEEP);
  await afterBackupSaved(manifest, kind);
  return { manifest, sizeBytes: bytes.length };
}

async function afterBackupSaved(manifest: BackupManifest, kind: BackupKind): Promise<void> {
  await saveBackupSettings({ lastBackupAt: manifest.createdAt, lastBackupKind: kind });
  logActivity('settings', kind === 'manual' ? 'إنشاء نسخة احتياطية يدوية' : kind === 'auto' ? 'نسخة احتياطية تلقائية' : 'نسخة احتياطية قبل الاستعادة', session.userId, manifest.createdAt);
}

// --- history -----------------------------------------------------------------------------------

export async function listHistory(): Promise<BackupHistoryEntry[]> {
  if (isTauriMode()) {
    const folder = backupSettings().folder;
    if (!folder) return [];
    const { readDir, stat } = await import('@tauri-apps/plugin-fs');
    try {
      const entries = await readDir(folder);
      const zipEntries = entries.filter((e) => e.isFile && e.name?.endsWith('.zip'));
      const out: BackupHistoryEntry[] = [];
      for (const e of zipEntries) {
        const path = `${folder}/${e.name}`;
        try {
          const info = await stat(path);
          const { readFile } = await import('@tauri-apps/plugin-fs');
          const bytes = await readFile(path);
          const manifest = readManifest(bytes);
          out.push({ id: path, manifest, path, sizeBytes: info.size, location: 'file' });
        } catch {
          // Skip files that aren't readable/valid backups (e.g. leftover partial writes).
        }
      }
      return out.sort((a, b) => b.manifest.createdAt.localeCompare(a.manifest.createdAt));
    } catch {
      return [];
    }
  }

  const all = await browserGetAll();
  return all.map((r) => ({ id: r.id, manifest: r.manifest, path: r.id, sizeBytes: r.sizeBytes, location: 'browser' as const }));
}

export async function deleteHistoryEntry(entry: BackupHistoryEntry): Promise<void> {
  if (entry.location === 'file') {
    const { remove } = await import('@tauri-apps/plugin-fs');
    await remove(entry.path);
  } else {
    await browserDelete(entry.path);
  }
}

/** Re-checksums the archive and compares it to what's recorded in its own manifest. */
export async function verifyHistoryEntry(entry: BackupHistoryEntry): Promise<{ ok: boolean; detail: string }> {
  let bytes: Uint8Array;
  if (entry.location === 'file') {
    const { readFile } = await import('@tauri-apps/plugin-fs');
    bytes = await readFile(entry.path);
  } else {
    const rec = await browserGet(entry.path);
    if (!rec) return { ok: false, detail: 'النسخة غير موجودة بعد الآن' };
    bytes = rec.bytes;
  }
  try {
    const parsed = parseArchive(bytes);
    const actual = await recomputeChecksum(bytes, parsed.manifest);
    const ok = actual === parsed.manifest.checksum;
    return { ok, detail: ok ? 'المجموع الاختباري مطابق — الملف سليم' : 'المجموع الاختباري غير مطابق — الملف قد يكون تالفاً' };
  } catch (err) {
    return { ok: false, detail: err instanceof Error ? err.message : 'تعذّرت قراءة الملف' };
  }
}

async function recomputeChecksum(bytes: Uint8Array, manifest: BackupManifest): Promise<string> {
  const { unzipSync } = await import('fflate');
  const files = unzipSync(bytes);
  if (manifest.encrypted) {
    return sha256Hex(files['payload.enc']);
  }
  // Same stable-order concatenation as buildBackupArchive's concatFiles().
  const keys = Object.keys(files).filter((k) => k !== 'manifest.json').sort();
  const total = keys.reduce((acc, k) => acc + files[k].length, 0);
  const out = new Uint8Array(total);
  let offset = 0;
  for (const k of keys) {
    out.set(files[k], offset);
    offset += files[k].length;
  }
  return sha256Hex(out);
}

// --- restore -------------------------------------------------------------------------------------

export async function pickRestoreFile(): Promise<Uint8Array | undefined> {
  if (isTauriMode()) {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const { readFile } = await import('@tauri-apps/plugin-fs');
    const path = await open({ multiple: false, filters: [{ name: 'Backup', extensions: ['zip'] }] });
    if (!path || Array.isArray(path)) return undefined;
    return readFile(path);
  }
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.zip,application/zip';
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return resolve(undefined);
      resolve(new Uint8Array(await file.arrayBuffer()));
    };
    input.click();
  });
}

export function previewRestore(bytes: Uint8Array): RestorePreview {
  const manifest = readManifest(bytes);
  const compatible = manifest.schemaVersion <= SCHEMA_VERSION;
  const compatibilityNote =
    manifest.schemaVersion > SCHEMA_VERSION
      ? 'هذه النسخة أُنشئت بإصدار أحدث من التطبيق الحالي — يلزم تحديث التطبيق قبل الاستعادة'
      : manifest.schemaVersion < SCHEMA_VERSION
        ? 'سيتم ترقية بيانات هذه النسخة تلقائياً إلى الإصدار الحالي عند الاستعادة'
        : undefined;
  return { manifest, compatible, compatibilityNote };
}

function migrateDb(data: unknown, fromVersion: number): typeof db {
  let result = data;
  for (let v = fromVersion; v < SCHEMA_VERSION; v++) {
    const migrate = migrations[v];
    if (migrate) result = migrate(result);
  }
  return result as typeof db;
}

/**
 * Runs the full restore flow (docs/v2/14-platform.md §4): decrypt if needed, run an automatic
 * pre-restore backup of the CURRENT data first (so restoring is never a one-way door), then
 * replace the persisted DB + attachments and reload the app. Caller is responsible for the typed
 * "استعادة" confirmation UI before calling this.
 */
export async function restoreFromArchive(bytes: Uint8Array, password?: string): Promise<void> {
  const parsed = parseArchive(bytes);
  let data: BackupData;
  if (parsed.manifest.encrypted) {
    if (!password) throw new Error('هذه النسخة مشفّرة — أدخل كلمة المرور');
    data = await decryptArchive(parsed, password);
  } else {
    data = parsed.data!;
  }

  // Automatic pre-restore backup of what's currently on disk/IndexedDB, before anything is overwritten.
  await backupNow('pre-restore');

  const migrated = migrateDb(clone(data.db), parsed.manifest.schemaVersion);
  mutate(() => Object.assign(db, migrated));
  await flushSnapshot();
  await replaceAllAttachments(data.attachments.map(archiveAttachmentToRecord));

  logActivity('settings', 'استعادة من نسخة احتياطية', session.userId, new Date().toISOString());
  await flushSnapshot();
}

// --- automatic backup: daily schedule + on-close --------------------------------------------

let dailyTimer: ReturnType<typeof setInterval> | null = null;
let closeUnlisten: (() => void) | null = null;
let beforeUnloadHandler: ((e: BeforeUnloadEvent) => void) | null = null;

async function pruneFileHistory(): Promise<void> {
  const settings = backupSettings();
  if (!settings.folder) return;
  const history = await listHistory();
  const sorted = history.sort((a, b) => b.manifest.createdAt.localeCompare(a.manifest.createdAt));
  const toDelete = sorted.slice(settings.retention);
  for (const entry of toDelete) {
    try {
      await deleteHistoryEntry(entry);
    } catch {
      /* best-effort retention cleanup */
    }
  }
}

async function runAutoBackupIfDue(): Promise<void> {
  const settings = backupSettings();
  if (!settings.autoEnabled) return;
  const now = new Date();
  const todayKey = now.toISOString().slice(0, 10);
  if (settings.lastAutoRunDate === todayKey) return;
  const [h, m] = settings.autoTime.split(':').map(Number);
  const due = now.getHours() > h || (now.getHours() === h && now.getMinutes() >= m);
  if (!due) return;
  if (isTauriMode() && !settings.folder) return; // no folder chosen yet — nothing to write to
  try {
    const suggestedPath = isTauriMode() && settings.folder ? `${settings.folder}/${backupFileName(db.settings.storeName)}` : undefined;
    await backupNow('auto', undefined, { suggestedPath });
    await saveBackupSettings({ lastAutoRunDate: todayKey });
    if (isTauriMode()) await pruneFileHistory();
  } catch (err) {
    console.error('[backup] scheduled auto backup failed', err);
  }
}

/** Runs an auto backup once on app close (best-effort — see notes below on Tauri vs browser). */
async function runCloseBackup(): Promise<void> {
  const settings = backupSettings();
  if (!settings.autoEnabled) return;
  if (isTauriMode() && !settings.folder) return;
  try {
    const suggestedPath = isTauriMode() && settings.folder ? `${settings.folder}/${backupFileName(db.settings.storeName)}` : undefined;
    await backupNow('auto', undefined, { suggestedPath });
    if (isTauriMode()) await pruneFileHistory();
  } catch (err) {
    console.error('[backup] close-time auto backup failed', err);
  }
}

/**
 * Wires the daily schedule check (polled every minute — simplest reliable approach for a page-
 * lifetime timer, no OS-level cron) and the on-close hook. Call once from `main.ts` after boot.
 *
 * On-close: Tauri exposes `tauri://close-requested` on the current window, which we listen for
 * and `await` the backup inside (blocking the actual close briefly) since Tauri lets an event
 * handler delay the window's destruction as long as it's still executing. In the browser,
 * `beforeunload` cannot reliably await an async IndexedDB+zip operation before the tab closes, so
 * that path is a best-effort approximation only: it fires the backup without blocking navigation.
 */
export async function initAutoBackup(): Promise<void> {
  if (dailyTimer) return;
  await runAutoBackupIfDue();
  dailyTimer = setInterval(() => void runAutoBackupIfDue(), 60_000);

  if (isTauriMode()) {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      const win = getCurrentWindow();
      closeUnlisten = await win.listen('tauri://close-requested', async () => {
        await runCloseBackup();
        await win.destroy();
      });
    } catch (err) {
      console.error('[backup] could not hook window close event', err);
    }
  } else {
    beforeUnloadHandler = () => {
      // Best-effort only (see doc comment) — fire-and-forget, browsers do not guarantee this
      // async work completes before the tab actually unloads.
      void runCloseBackup();
    };
    window.addEventListener('beforeunload', beforeUnloadHandler);
  }
}

export function stopAutoBackup(): void {
  if (dailyTimer) {
    clearInterval(dailyTimer);
    dailyTimer = null;
  }
  if (closeUnlisten) {
    closeUnlisten();
    closeUnlisten = null;
  }
  if (beforeUnloadHandler) {
    window.removeEventListener('beforeunload', beforeUnloadHandler);
    beforeUnloadHandler = null;
  }
}

// --- folder picker (Tauri only) ---------------------------------------------------------------

export async function pickBackupFolder(): Promise<string | undefined> {
  const { open } = await import('@tauri-apps/plugin-dialog');
  const path = await open({ directory: true, multiple: false });
  if (!path || Array.isArray(path)) return undefined;
  return path;
}
