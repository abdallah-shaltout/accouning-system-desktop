/**
 * Dev-only tooling seam (docs/v2/17-ui-system-rtl-themes.md Phase F "DevMenu" controls, now hosted
 * inside `NavUser`'s dev section): reset-to-empty, reload-demo-data, latency toggle. These wrap
 * `src/mocks/seed`/`persist` so `NavUser.vue` (and any future `/dev/*` page) never imports the mock
 * backend directly — the seam rule applies to dev tooling too, even though it will never be backed
 * by a real IPC command.
 */
import { db } from '@/mocks';
import { seedDatabase } from '@/mocks/seed';
import { clearSnapshot, flushSnapshot, SCHEMA_VERSION } from '@/mocks/persist';
import { backendCall, usesRust } from '@/modules/core/services/backend';

import { wrap } from '@/modules/diagnostics/services/defineService';

/** Wipes the persisted snapshot — caller is expected to reload the app afterward. Refuses in Rust
 * mode (`clearSnapshot`, Part 04 E-3): that snapshot is the user's legacy data (P4-9). */
export const resetToEmpty = wrap('core.resetToEmpty', async function resetToEmpty(): Promise<void> {
  await clearSnapshot();
});

/**
 * Whether the welcome page may offer "explore with demo data" (Part 04 E-2 (3)). Always on the mock
 * (the browser build). On the real backend only in dev builds: `setup_import_snapshot` refuses
 * `mode: 'demo'` in a release build, because demo books imported into a real company's empty
 * database could never be removed again (a release build has no wipe, 00-import step 1).
 */
export function canLoadDemoData(): boolean {
  return import.meta.env.DEV || !usesRust('setup');
}

/**
 * Reseeds fresh demo data. In a Tauri build (`usesRust('setup')`), the mock seed only builds the
 * snapshot in memory — the D10 importer (`setup_import_snapshot`, `mode: 'demo'`) is what actually
 * writes it into the real MariaDB, since the mock is not the backend in that mode
 * (`03-domains/00-import.md` §6). `replaceExisting` is only ever true in dev builds — a release
 * build's demo-reseed button (if any) must never be able to wipe a real company's data.
 */
export const reloadDemoData = wrap('core.reloadDemoData', async function reloadDemoData(): Promise<void> {
  if (usesRust('setup')) {
    seedDatabase();
    await backendCall('setup_import_snapshot', {
      snapshotJson: JSON.stringify({ version: SCHEMA_VERSION, savedAt: new Date().toISOString(), data: db }),
      mode: 'demo',
      replaceExisting: import.meta.env.DEV,
    });
    return;
  }
  seedDatabase();
  await flushSnapshot();
});
