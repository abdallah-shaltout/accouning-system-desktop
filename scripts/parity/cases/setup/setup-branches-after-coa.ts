/**
 * 02-setup §8(b): branches applied before the chart template, then the template switched (§3.5):
 * `applyBranches` renames the main branch (name/code untrimmed, code upper-cased — quirk Q-2) and
 * creates the others with their own cash accounts (1111, 1112) and `features.branches`; a later
 * `applyCoaTemplate` rebuilds the chart by code, so the branch cash accounts not in the template
 * are removed and the branches keep pointing at them (quirk Q-8). Refusals: empty list, duplicate
 * code (case-insensitive, including the renamed main branch).
 *
 * Base: the empty-company shell (see `setup/setup-wizard-sa` for why not `empty`). The switch is
 * to `detailed` (not `basic`, whose missing card-clearing account makes the mock's invariants
 * throw — see `setup/setup-coa-switch`).
 */
import { defineCase } from '../../case';
import * as setupService from '../../../../src/modules/setup/services/setupService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as branchesService from '../../../../src/modules/settings/services/branchesService';
import * as settingsService from '../../../../src/modules/settings/services/settingsService';
import { db, type MockDb } from '../../../../src/mocks/db';
import { seedEmptyCompany } from '../../../../src/mocks/seed';
import { clone, resetIdCounters } from '../../../../src/mocks/utils';
import { pinClock } from '../../clock';
import { SEED_NOW } from '../../bases';

function shell(): MockDb {
  pinClock(SEED_NOW, 'Africa/Cairo', 'base:setup-shell');
  resetIdCounters();
  seedEmptyCompany('EG');
  return clone(db);
}

export default defineCase({
  name: 'setup/setup-branches-after-coa',
  source: '03-domains/02-setup.md §8(b)',
  base: shell,
  user: null,
  async run(s) {
    await s.step('progress', () => setupService.getOnboardingProgress());
    await s.expectError('empty-list', () => setupService.applyBranches([]));
    await s.expectError('duplicate-of-main', () =>
      setupService.applyBranches([
        { name: 'الفرع الرئيسي', code: 'main' },
        { name: 'فرع ثان', code: 'MAIN' },
      ]),
    );
    await s.step('apply-branches', () =>
      setupService.applyBranches([
        { name: ' فرع وسط البلد ', code: ' dt' },
        { name: 'فرع مدينة نصر', code: 'NSR', address: { country: 'EG', regionName: 'القاهرة', cityName: 'مدينة نصر', street: 'عباس العقاد', buildingNo: '12', postalCode: '11765' } },
        { name: 'فرع المعادي', code: 'MAD' },
      ]),
    );
    await s.step('branches', () => branchesService.getBranches());
    await s.step('cash-accounts', async () => (await accountingService.getAccounts()).filter((a) => a.code.startsWith('111')));
    await s.step('features', async () => (await settingsService.getSettings()).features);
    await s.step('apply-coa', () => setupService.applyCoaTemplate('detailed', 'EG', 'retail'));
    await s.step('branches-after-coa', () => branchesService.getBranches());
    await s.step('cash-accounts-after-coa', async () => (await accountingService.getAccounts()).filter((a) => a.code.startsWith('111')));
  },
});
