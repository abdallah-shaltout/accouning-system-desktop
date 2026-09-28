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
 */
import { ApiError } from '@/mocks';
import { readPersistedSnapshot } from '@/mocks/persist';
import { backendCall, usesRust } from '@/modules/core/services/backend';
import { wrap } from '@/modules/diagnostics/services/defineService';
import type { LegacySnapshotSummary } from '../types';

const LEGACY_IMPORTED_AT_KEY = 'equal.legacyImportedAt';
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

/** Frontend-only (no Rust round trip): a legacy snapshot exists **and** it hasn't already been
 * imported on this machine (`equal.legacyImportedAt` unset) — drives whether the one-time import
 * card shows at all. */
export const hasLegacySnapshot = wrap('setup.hasLegacySnapshot', async function hasLegacySnapshot(): Promise<boolean> {
  if (!usesRust('setup')) return false;
  let alreadyImported = false;
  try {
    alreadyImported = localStorage.getItem(LEGACY_IMPORTED_AT_KEY) != null;
  } catch {
    alreadyImported = false;
  }
  if (alreadyImported) return false;
  const snapshot = await readPersistedSnapshot();
  return snapshot != null;
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
 * then). On success, marks this machine as already-imported so the offer disappears. */
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
    localStorage.setItem(LEGACY_IMPORTED_AT_KEY, new Date().toISOString());
  } catch {
    /* private mode — the offer may reappear next launch, not a correctness issue */
  }
});
