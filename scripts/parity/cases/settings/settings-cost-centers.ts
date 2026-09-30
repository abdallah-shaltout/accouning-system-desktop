/**
 * L1 platform lane. Cost centers (01-settings.md §3 "Cost centers"): create, duplicate-code refusal,
 * update (no validation, Q-5), delete refusal (branch cost center can't be deleted; `cc-main`'s own
 * rent split, seeded by v2 phase 9's `branches9.ts`, doubles as the "used by a posted line" refusal),
 * then a clean delete. Cost-center CRUD lives in `branchesService.ts` (thin wrapper over
 * `mocks/backend/branches.ts`), not `settingsService.ts`. `demo-sa` already seeds `CC-MKT` as a plain
 * department cost center with no journal lines against it, so this case uses a fresh code instead.
 */
import { defineCase } from '../../case';
import * as branchesService from '../../../../src/modules/settings/services/branchesService';

export default defineCase({
  name: 'settings/settings-cost-centers',
  source: '03-domains/01-settings.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('list-before', () => branchesService.getCostCenters());

    const created = await s.step('create', () =>
      branchesService.createCostCenter({ code: 'CC-OPS', name: 'العمليات', type: 'department', active: true }),
    );

    await s.expectError('duplicate-code', () => branchesService.createCostCenter({ code: 'CC-OPS', name: 'أخرى', type: 'department', active: true }));
    await s.expectError('blank-name', () => branchesService.createCostCenter({ code: 'CC-X', name: '  ', type: 'department', active: true }));
    await s.expectError('blank-code', () => branchesService.createCostCenter({ code: '  ', name: 'اسم', type: 'department', active: true }));

    await s.step('update', () => branchesService.updateCostCenter(created.id, { name: 'العمليات والدعم' }));
    await s.expectError('update-missing', () => branchesService.updateCostCenter('cc-does-not-exist', { name: 'x' }));

    // The auto-created main-branch cost center can never be deleted, and it also carries the
    // seeded rent-split journal line (branches9.ts §3), so this one refusal covers both guards.
    await s.expectError('delete-branch-cost-center', () => branchesService.deleteCostCenter(s.baseId('cc-main')));

    await s.step('delete', () => branchesService.deleteCostCenter(created.id));
    await s.expectError('delete-missing', () => branchesService.deleteCostCenter('cc-does-not-exist'));
    await s.step('list-final', () => branchesService.getCostCenters());
  },
});
