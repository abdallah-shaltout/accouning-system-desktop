/**
 * L1 platform lane. Self-protection guard (03-users.md §3 `update_user` step 3): the signed-in
 * admin cannot deactivate themself or change their own role, but may change other fields freely.
 */
import { defineCase } from '../../case';
import * as userService from '../../../../src/modules/users/services/userService';

export default defineCase({
  name: 'users/users-self-protection',
  source: '03-domains/03-users.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const self = await s.step('get-self', () => userService.getUser(s.baseId('usr-1')));

    await s.expectError('self-deactivate', () =>
      userService.updateUser(self.id, { username: self.username, name: self.name, role: self.role, maxDiscount: self.maxDiscount, active: false }),
    );
    await s.expectError('self-role-change', () =>
      userService.updateUser(self.id, { username: self.username, name: self.name, role: 'manager', maxDiscount: self.maxDiscount, active: true }),
    );

    // Changing an unrelated field on self is fine.
    await s.step('self-rename-ok', () =>
      userService.updateUser(self.id, { username: self.username, name: 'المدير المعدّل', role: self.role, maxDiscount: self.maxDiscount, active: true }),
    );

    // A non-self user's role/active flags are not protected.
    const cashier = await s.step('get-cashier', () => userService.getUser(s.baseId('usr-4')));
    await s.step('deactivate-other', () =>
      userService.updateUser(cashier.id, { username: cashier.username, name: cashier.name, role: cashier.role, maxDiscount: cashier.maxDiscount, active: false }),
    );
  },
});
