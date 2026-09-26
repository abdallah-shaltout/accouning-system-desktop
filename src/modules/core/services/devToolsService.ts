/**
 * Dev-only tooling seam (docs/v2/17-ui-system-rtl-themes.md Phase F "DevMenu" controls, now hosted
 * inside `NavUser`'s dev section): reset-to-empty, reload-demo-data, latency toggle. These wrap
 * `src/mocks/seed`/`persist` so `NavUser.vue` (and any future `/dev/*` page) never imports the mock
 * backend directly — the seam rule applies to dev tooling too, even though it will never be backed
 * by a real IPC command.
 */
import { seedDatabase } from '@/mocks/seed';
import { clearSnapshot, flushSnapshot } from '@/mocks/persist';

import { wrap } from '@/modules/diagnostics/services/defineService';

/** Wipes the persisted snapshot — caller is expected to reload the app afterward. */
export const resetToEmpty = wrap('core.resetToEmpty', async function resetToEmpty(): Promise<void> {
  await clearSnapshot();
});

/** Reseeds fresh demo data and persists it — caller is expected to reload the app afterward. */
export const reloadDemoData = wrap('core.reloadDemoData', async function reloadDemoData(): Promise<void> {
  seedDatabase();
  await flushSnapshot();
});
