/**
 * L5 (03-domains/14b-insights.md §8(b)): `dashboard_compute_insights` with Arabic-Indic numerals —
 * same 21-rule catalogue and role filtering as `insights-latin.ts`, proving the numeral system
 * (G-36) only changes the formatted strings inside messages/metrics, never which rules fire.
 */
import { defineCase } from '../../case';
import * as insightEngine from '../../../../src/modules/core/services/insightEngine';
import { setNumerals } from '../../../../src/modules/core/helpers/format';
import { mirrorsSettled } from '../../../../src/modules/core/services/backendMirror';
import { useBackupStore } from '../../../../src/modules/settings/controllers/useBackupStore';
import type { Role } from '../../../../src/modules/users/types';

const ROLES: Role[] = ['admin', 'manager', 'accountant', 'cashier', 'storekeeper'];

export default defineCase({
  name: 'dashboard/insights-arab',
  source: '03-domains/14b-insights.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  // See insights-latin.ts: discount-leak's `value` is a raw average (14b §8(b) epsilon field).
  epsilon: ROLES.map((role) => `steps.insights-${role}.value[].value`),
  async run(s) {
    setNumerals('arab');
    // The mock engine's module-level insight cache outlives a pass's `db` swap (no change event
    // fires), so start every pass from a fresh computation.
    insightEngine.forceRefresh();
    await s.step('load-backup-store', () => useBackupStore().load());
    for (const role of ROLES) {
      // `getInsights` is synchronous over a `backendMirror` (G-37) on Rust: the first read starts the
      // IPC load and returns the fallback `[]`, so read, wait for the load, and read again.
      const read = () => insightEngine.getInsights({ role, includeHidden: true });
      await s.step(`insights-${role}`, async () => {
        read();
        await mirrorsSettled();
        return read();
      });
    }
  },
});
