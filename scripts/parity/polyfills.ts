/**
 * Browser-storage polyfills for the parity harness (plan 21 Part 04, B-8/B-9 follow-up — L1 gaps).
 *
 * `bun run parity` runs the real TS services under plain Bun, which has neither `localStorage`/
 * `sessionStorage`, `indexedDB`, nor a DOM. Three mock-side/shared services read those directly:
 * - `templateService.ts` (`localStorage`, key `pdf_templates_v1`) — every call threw inside its own
 *   try/catch and silently re-seeded two fresh default templates, so no write ever persisted across
 *   calls (found by L1 while authoring `templates/templates-lifecycle`).
 * - `src/mocks/attachments.ts` (`indexedDB`) — every call threw uncaught (no try/catch there), so
 *   `backupService.ts`'s `previewBackupCounts`/`backupNow`/`restoreFromArchive` (all three read
 *   `getAllAttachmentRecords()` first) could never complete (found by L1 while authoring the backup
 *   cases).
 * - `core/services/saveFile.ts`'s `browserDownload()` (`backupNow`/`restoreFromArchive`'s
 *   pre-restore backup both end here once `isTauri()` is false, which it always is under Bun) —
 *   `document.createElement('a')`/`document.body.appendChild(...)` throw, `document` being
 *   undefined. A **harness-only** `document` stub below (never touches `saveFile.ts` itself) gives it
 *   just enough of an `<a>`/`body` to complete a no-op "download" — no production behaviour change,
 *   since the real app always runs inside a real webview with a real `document`.
 * - `backupService.ts`'s `initAutoBackup`/`stopAutoBackup` (non-Tauri branch) — `window` itself is
 *   undefined under Bun even though Bun already provides `addEventListener`/`removeEventListener` as
 *   globals, so aliasing `window` to `globalThis` (exactly what it is in a real browser/webview) is
 *   the whole fix (found while restoring `backup/backup-auto-run`).
 *
 * `installPolyfills()` installs all three once per process, before any case module is imported (so a
 * service's top-level `localStorage.getItem(...)` — none currently exist, but nothing should assume
 * otherwise — and every call inside `run()` see a real Storage/IDBFactory/document). `resetPolyfills()`
 * clears the storages; `run.ts`/`pass.ts` call it once per pass (mock and rust alike — the rust pass
 * never touches these but clearing is free) so every pass starts from the same empty storage, exactly
 * like a fresh browser profile, and `--mock-only`'s run-1-vs-run-2 determinism check stays meaningful.
 *
 * This never runs in the real app: `main.ts` never imports this file, and installing it a second
 * time (e.g. a stray import) is a guarded no-op — every install checks the global is genuinely absent
 * first, so it can never shadow a real browser/webview's `document`/`Storage`/`indexedDB`.
 */
import 'fake-indexeddb/auto';

// --- localStorage / sessionStorage ---------------------------------------------------------------

/** A minimal `Storage` — every method the app's `localStorage`/`sessionStorage` call sites use
 * (`getItem`, `setItem`, `removeItem`, `clear`, `key`, `.length`). Not `Map`-backed to keep the
 * exact `Storage` shape (`instanceof Storage` is never checked anywhere in this codebase). */
class MemoryStorage implements Storage {
  private store = new Map<string, string>();

  get length(): number {
    return this.store.size;
  }
  clear(): void {
    this.store.clear();
  }
  getItem(key: string): string | null {
    return this.store.has(key) ? this.store.get(key)! : null;
  }
  key(index: number): string | null {
    return [...this.store.keys()][index] ?? null;
  }
  removeItem(key: string): void {
    this.store.delete(key);
  }
  setItem(key: string, value: string): void {
    this.store.set(key, String(value));
  }
}

let localStoragePolyfill: MemoryStorage | null = null;
let sessionStoragePolyfill: MemoryStorage | null = null;

function installStorage(): void {
  // Only install when the global is genuinely absent — never shadow a real browser/jsdom Storage.
  if (typeof globalThis.localStorage === 'undefined') {
    localStoragePolyfill = new MemoryStorage();
    Object.defineProperty(globalThis, 'localStorage', { value: localStoragePolyfill, configurable: true, writable: false });
  }
  if (typeof globalThis.sessionStorage === 'undefined') {
    sessionStoragePolyfill = new MemoryStorage();
    Object.defineProperty(globalThis, 'sessionStorage', { value: sessionStoragePolyfill, configurable: true, writable: false });
  }
}

// --- window (harness-only alias for backupService.ts's non-Tauri close-backup listener) -----------

/** `backupService.ts`'s `initAutoBackup`/`stopAutoBackup` call `window.addEventListener`/
 * `removeEventListener('beforeunload', ...)` in the non-Tauri branch (found while restoring
 * `backup/backup-auto-run` to the real end-to-end entry point). Bun already provides
 * `addEventListener`/`removeEventListener`/`dispatchEvent` as globals (it implements the same
 * `EventTarget`-based DOM event surface Node/browsers do for things like `unhandledrejection`); the
 * only thing missing is the `window` name itself. Aliasing it to `globalThis` (exactly what `window`
 * *is* in a real browser/webview — the global object under another name) is the minimal fix, and it
 * only ever installs when `window` is genuinely absent. */
function installWindowAlias(): void {
  if (typeof globalThis.window !== 'undefined') return;
  Object.defineProperty(globalThis, 'window', { value: globalThis, configurable: true, writable: false });
}

// --- document (harness-only stub for saveFile.ts's browserDownload()) -----------------------------

/** Just enough of `HTMLAnchorElement` for `browserDownload()`: settable `href`/`download`, and
 * `click()`/`remove()` as no-ops (there's no real download to trigger headlessly — the point is
 * only that `backupNow`/`restoreFromArchive` complete instead of throwing on a missing DOM). */
class StubAnchorElement {
  href = '';
  download = '';
  click(): void {}
  remove(): void {}
}

/** Just enough of `document` for `saveFile.ts`'s `browserDownload()`: `createElement('a')` and
 * `body.appendChild(...)`. Never a general jsdom-style document — anything else the app might touch
 * stays genuinely undefined, so a service that reaches further into the DOM still fails loudly
 * instead of silently half-working. */
function installDocumentStub(): void {
  if (typeof globalThis.document !== 'undefined') return;
  const stubDocument = {
    createElement: (tag: string) => (tag.toLowerCase() === 'a' ? new StubAnchorElement() : { click() {}, remove() {} }),
    body: { appendChild: (_node: unknown) => _node },
  };
  Object.defineProperty(globalThis, 'document', { value: stubDocument, configurable: true, writable: false });
}

// --- indexedDB ------------------------------------------------------------------------------------

// Installed as a module-evaluation side effect (not behind a function call) so ESM's "evaluate
// imports before the importer's own top-level code" ordering is what makes this run first, instead
// of depending on `run.ts` remembering to call an install function before its other imports.
// `fake-indexeddb/auto` (imported above) already defines `globalThis.indexedDB`/`IDBKeyRange` when
// they're absent, and no-ops if a real indexedDB is already global.
installStorage();
installWindowAlias();
installDocumentStub();

/** No-op kept for callers that want an explicit, self-documenting call at the top of an entry point
 * (`run.ts`) — importing this module has already done the work by the time this runs. */
export function installPolyfills(): void {
  installStorage();
  installWindowAlias();
  installDocumentStub();
}

/** Clears both storages and gives every fake-indexeddb-backed database a clean slate, so each pass
 * (mock or rust) starts from the same empty-browser-profile state. Rust passes never touch these,
 * but resetting is cheap and keeps the call unconditional in `pass.ts`. */
export async function resetPolyfills(): Promise<void> {
  localStoragePolyfill?.clear();
  sessionStoragePolyfill?.clear();
  await resetFakeIndexedDb();
}

async function resetFakeIndexedDb(): Promise<void> {
  const idb = globalThis.indexedDB as (IDBFactory & { databases?: () => Promise<{ name?: string }[]> }) | undefined;
  if (!idb) return;
  // `IDBFactory.databases()` is part of the modern spec and fake-indexeddb implements it — used
  // instead of hardcoding db names ('mock-db', 'mock-db-backup-history') so a new store added later
  // is cleared automatically too.
  const dbs = (await idb.databases?.()) ?? [];
  await Promise.all(
    dbs
      .map((d) => d.name)
      .filter((name): name is string => !!name)
      .map(
        (name) =>
          new Promise<void>((resolve) => {
            const req = idb.deleteDatabase(name);
            req.onsuccess = () => resolve();
            req.onerror = () => resolve(); // best-effort — a stuck delete shouldn't fail the pass
            req.onblocked = () => resolve();
          }),
      ),
  );
}
