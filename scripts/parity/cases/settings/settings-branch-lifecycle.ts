/**
 * L1 platform lane. Branch lifecycle (01-settings.md §3 "Branches"): create (cash account + cost
 * center auto-created) → duplicate-code / blank-field refusals → update → deactivate (no stock, no
 * open shift on the fresh branch) → reactivate. `demo-sa` already seeds a second branch (`JED`,
 * v2 phase 9) with stock transferred into it, so the "only active branch" refusal isn't reachable
 * here without tearing that down — parity for that specific guard is covered on the mock side by
 * `tests/domain_settings.rs` (03-domains/01-settings.md §8(a)); this case sticks to what a fresh
 * branch's own lifecycle can prove deterministically.
 */
import { defineCase } from '../../case';
import * as branchesService from '../../../../src/modules/settings/services/branchesService';

export default defineCase({
  name: 'settings/settings-branch-lifecycle',
  source: '03-domains/01-settings.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('list-before', () => branchesService.getBranches());

    const created = await s.step('create', () =>
      branchesService.createBranch({ name: 'فرع الدمام', code: 'dmm', address: 'شارع الخليج', phone: '+966512345678', active: true }),
    );
    await s.step('list-after-create', () => branchesService.getBranches());

    await s.expectError('duplicate-code-ci', () => branchesService.createBranch({ name: 'فرع آخر', code: 'DMM', active: true }));
    await s.expectError('blank-name', () => branchesService.createBranch({ name: '  ', code: 'ABC', active: true }));
    await s.expectError('blank-code', () => branchesService.createBranch({ name: 'فرع', code: '  ', active: true }));

    await s.step('update', () => branchesService.updateBranch(created.id, { name: 'فرع الدمام الكبير', phone: '+966598765432' }));
    await s.expectError('update-missing', () => branchesService.updateBranch('branch-does-not-exist', { name: 'x' }));

    // The fresh branch has no stock and no open shift, so it deactivates cleanly.
    await s.step('deactivate', () => branchesService.deactivateBranch(created.id));
    await s.expectError('deactivate-missing', () => branchesService.deactivateBranch('branch-does-not-exist'));

    await s.step('reactivate', () => branchesService.reactivateBranch(created.id));
    // Reactivating an already-active branch has no guard (Q-4) — still succeeds, still logs.
    await s.step('reactivate-again', () => branchesService.reactivateBranch(created.id));

    await s.step('list-final', () => branchesService.getBranches());
  },
});
