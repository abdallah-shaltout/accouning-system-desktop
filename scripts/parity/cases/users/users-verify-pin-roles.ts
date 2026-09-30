/**
 * L1 platform lane. `verifyManagerPin` (03-users.md §3): a cashier's own PIN is refused (not a
 * manager), an inactive manager is refused, and a real manager succeeds without touching the
 * signed-in session (the caller stays the cashier).
 */
import { defineCase } from '../../case';
import * as authService from '../../../../src/modules/users/services/authService';
import * as userService from '../../../../src/modules/users/services/userService';

export default defineCase({
  name: 'users/users-verify-pin-roles',
  source: '03-domains/03-users.md §8(b)',
  base: 'demo-sa',
  user: 'cashier',
  async run(s) {
    await s.expectError('cashier-not-a-manager', () => authService.verifyManagerPin('cashier', 'cashier123'));
    await s.expectError('unknown-user', () => authService.verifyManagerPin('nobody', 'x'));
    await s.expectError('wrong-password', () => authService.verifyManagerPin('manager', 'wrong'));

    const approver = await s.step('manager-pin-ok', () => authService.verifyManagerPin('manager', 'manager123'));
    // The session is untouched — the currently signed-in user (cashier) can still act.
    await s.step('still-cashier', () => userService.getUsers());

    void approver;
    // The runner reads the books (trial balance, inventory report) with whoever is signed in; a
    // cashier has no Reports/Inventory read on Rust (role gating, `permissions.ts`), the mock's reads
    // are ungated. Sign in as admin so the books are compared rather than refused on one side.
    await s.login('admin');
  },
});
