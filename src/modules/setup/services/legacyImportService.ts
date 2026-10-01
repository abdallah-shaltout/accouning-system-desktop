/**
 * D10 (21.03 §00-import): the one-time "import your data from the previous version" flow. Every
 * function here is Tauri-only — outside a desktop build (`usesRust('setup')` never true in a
 * browser/e2e context, since `usesRust` itself gates on `isTauri()`) they throw `ApiError('الاستيراد
 * متاح في نسخة سطح المكتب فقط', 'FORBIDDEN')` rather than silently no-op, so a stray call from a
 * browser context fails loudly instead of pretending to import nothing.
 *
 * Reads the persisted IndexedDB snapshot directly (`readPersistedSnapshot`, seam rule: a service may
 * import mocks) and ships it to the Rust importer as a JSON string — the whole `MockDb` never needs
 * its own ts-rs type (`00-import.md` §3's "the snapshot travels as a JSON string so ts-rs never has
 * to type the whole `MockDb`").
 *
 * Plan 21 Part 04 (P4-9, E-4): the app never deletes that snapshot. A successful import records an
 * `importedAt` marker under its own key in the same IndexedDB store (`writeLegacyImportMarker`), and
 * the card hides once it exists. The importer's own empty-database refusal (00-import step 1) stays
 * the real guard against a second import.
 */
import { ApiError } from '@/mocks';
import { forgetLegacySnapshot, readLegacyImportMarker, readPersistedSnapshot, writeLegacyImportMarker } from '@/mocks/persist';
import { backendCall, usesRust } from '@/modules/core/services/backend';
import { log } from '@/modules/diagnostics/services/logService';
import { wrap } from '@/modules/diagnostics/services/defineService';
import { ensureDeviceSetupState } from './deviceService';
import type { LegacySnapshotSummary } from '../types';

const TEMPLATES_STORAGE_KEY = 'pdf_templates_v1';

function desktopOnly(): never {
  throw new ApiError('الاستيراد متاح في نسخة سطح المكتب فقط', 'FORBIDDEN');
}

function readTemplatesJson(): string | undefined {
  try {
    return localStorage.getItem(TEMPLATES_STORAGE_KEY) ?? undefined;
  } catch {
    return undefined;
  }
}

/** Whether the one-time import card shows at all: this is the Main PC (00-import D-3 — a terminal's
 * own old snapshot is never merged and stays on that PC untouched, P4-9), this PC's snapshot was not
 * imported yet (no E-4 marker), and the snapshot holds a company (at least one user). Only the device
 * state is a Rust round trip, and it is the cached one the router guard already fetched. */
export const hasLegacySnapshot = wrap('setup.hasLegacySnapshot', async function hasLegacySnapshot(): Promise<boolean> {
  if (!usesRust('setup')) return false;
  const device = await ensureDeviceSetupState();
  if (device.role !== 'main') return false;
  if (await readLegacyImportMarker()) return false;
  const snapshot = await readPersistedSnapshot();
  return (snapshot?.data?.users?.length ?? 0) > 0;
});

/** `setup_inspect_legacy_snapshot`: summarizes the persisted snapshot (row counts, branches,
 * whether print templates exist) for the one-time import screen, before committing to anything. */
export const inspectLegacySnapshot = wrap('setup.inspectLegacySnapshot', async function inspectLegacySnapshot(): Promise<LegacySnapshotSummary> {
  if (!usesRust('setup')) desktopOnly();
  const snapshot = await readPersistedSnapshot();
  const snapshotJson = JSON.stringify(snapshot);
  const templatesJson = readTemplatesJson();
  return backendCall('setup_inspect_legacy_snapshot', { snapshotJson, templatesJson });
});

/** `setup_import_snapshot` in `mode: 'legacy'`: the actual one-time import. `templateBranchId` is
 * only needed when the snapshot has more than one branch and print templates exist
 * (`LegacySnapshotSummary.branches.length > 1 && hasTemplates` — the card only shows the picker
 * then). On success, writes the E-4 marker so the offer disappears. (The import created the company's
 * users; the router guard re-reads the device state before it would bounce `/login` to `/welcome`.) */
export const importLegacySnapshot = wrap('setup.importLegacySnapshot', async function importLegacySnapshot(templateBranchId?: string): Promise<void> {
  if (!usesRust('setup')) desktopOnly();
  const snapshot = await readPersistedSnapshot();
  const snapshotJson = JSON.stringify(snapshot);
  const templatesJson = readTemplatesJson();
  await backendCall('setup_import_snapshot', {
    snapshotJson,
    templatesJson,
    templateBranchId,
    mode: 'legacy',
    replaceExisting: false,
  });
  try {
    await writeLegacyImportMarker(new Date().toISOString());
  } catch (err) {
    // Not a correctness issue: the offer may reappear, and the importer refuses a non-empty database.
    log.error('setup.importLegacySnapshot', 'failed to write the legacy-import marker', err instanceof Error ? err : new Error(String(err)));
  }
});

/** ACC-0035: whether the settings "Danger Zone" tab may offer "wipe all data and start fresh".
 * Debug desktop builds only — `setup_wipe_business_data` itself refuses in a release build too
 * (defense in depth), and the browser/mock build has no real database to wipe. */
export function canWipeBusinessData(): boolean {
  return import.meta.env.DEV && usesRust('setup');
}

/** ACC-0035: deletes this PC's old legacy snapshot outright (and its import marker), so the welcome
 * page stops offering to import it. Distinct from the main "wipe the live database" action: this one
 * targets the separate pre-migration IndexedDB snapshot that `hasLegacySnapshot` reads. Dev builds
 * only (`forgetLegacySnapshot` itself refuses otherwise) — a release build must never be able to make
 * a real customer's not-yet-imported data unrecoverable. */
export const clearLegacySnapshot = wrap('setup.clearLegacySnapshot', async function clearLegacySnapshot(): Promise<void> {
  await forgetLegacySnapshot();
});

/** `setup_wipe_business_data`: deletes every business row (same primitive a replace-existing import
 * uses) and leaves the database empty but still configured, so the welcome page's "ابدأ شركتك" /
 * "استكشف ببيانات تجريبية" cards work again without reinstalling. Caller must re-navigate to
 * `/welcome` afterward — every cached store/session is now stale. */
export const wipeBusinessData = wrap('setup.wipeBusinessData', async function wipeBusinessData(): Promise<void> {
  if (!canWipeBusinessData()) desktopOnly();
  await backendCall('setup_wipe_business_data');
});
