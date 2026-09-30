/**
 * L5 (03-domains/14b-insights.md §8(b)): `dashboard_compute_insights` (the mock's `computeAll()`,
 * reached here through `getInsights`) with Latin numerals — the 21-rule catalogue over the seeded
 * demo-sa data, filtered by role the same way on both backends (I-5). Decision I-4: the backup
 * store must be loaded before the mock run so `backup-overdue` sees the same
 * `last_backup_at`/`last_backup_failed_at` the Rust side reads from the DB.
 */
import { defineCase } from '../../case';
import * as insightEngine from '../../../../src/modules/core/services/insightEngine';
import { setNumerals } from '../../../../src/modules/core/helpers/format';
import { mirrorsSettled } from '../../../../src/modules/core/services/backendMirror';
import { useBackupStore } from '../../../../src/modules/settings/controllers/useBackupStore';
import type { Role } from '../../../../src/modules/users/types';

const ROLES: Role[] = ['admin', 'manager', 'accountant', 'cashier', 'storekeeper'];

export default defineCase({
  name: 'dashboard/insights-latin',
  source: '03-domains/14b-insights.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  // discount-leak's `value` is a raw (unrounded) average of discount rates (14b §8(b)); the path
  // syntax can't discriminate by ruleKey within the array, so this covers `value` on every
  // insight row — harmless on `--mock-only` (both mock runs compute the same floats byte-for-byte)
  // and narrower once Part 04's Rust pass exists is the manager's call (P4-6).
  epsilon: ROLES.map((role) => `steps.insights-${role}.value[].value`),
  async run(s) {
    setNumerals('latn');
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
