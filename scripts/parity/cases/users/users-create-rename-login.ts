/**
 * L1 platform lane. `createUser`/`updateUser` (03-users.md §3): create, duplicate username
 * (case-insensitive), missing password, rename + password change, then log in with the new
 * username/password to prove the credential moved with the rename.
 */
import { defineCase } from '../../case';
import * as userService from '../../../../src/modules/users/services/userService';
import * as authService from '../../../../src/modules/users/services/authService';

export default defineCase({
  name: 'users/users-create-rename-login',
  source: '03-domains/03-users.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const created = await s.step('create', () =>
      userService.createUser({ username: 'newcashier', name: 'كاشير جديد', role: 'cashier', maxDiscount: 5, active: true, password: 'newpass1' }),
    );

    await s.expectError('duplicate-username-ci', () =>
      userService.createUser({ username: 'NEWCASHIER', name: 'تكرار', role: 'cashier', maxDiscount: 0, active: true, password: 'x' }),
    );
    await s.expectError('no-password', () => userService.createUser({ username: 'nopass', name: 'بلا كلمة مرور', role: 'cashier', maxDiscount: 0, active: true }));

    await s.step('get', () => userService.getUser(created.id));
    await s.expectError('get-missing', () => userService.getUser('usr-does-not-exist'));

    await s.step('rename-and-repassword', () =>
      userService.updateUser(created.id, { username: 'renamedcashier', name: 'كاشير معاد تسميته', role: 'cashier', maxDiscount: 5, active: true, password: 'newpass2' }),
    );
    await s.expectError('update-missing', () =>
      userService.updateUser('usr-does-not-exist', { username: 'x', name: 'x', role: 'cashier', maxDiscount: 0, active: true }),
    );
    await s.expectError('update-duplicate-username', () =>
      userService.updateUser(created.id, { username: 'admin', name: 'x', role: 'cashier', maxDiscount: 5, active: true }),
    );

    await s.step('list', () => userService.getUsers());

    await s.logout();
    // Old username no longer works; the credential moved with the rename.
    await s.expectError('old-username-gone', () => authService.login('newcashier', 'newpass2'));
    await s.step('login-renamed', () => authService.login('renamedcashier', 'newpass2'));
  },
});
