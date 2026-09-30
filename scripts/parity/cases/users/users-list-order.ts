/**
 * L1 platform lane. `getUsers` list order (03-users.md §3: mock array order = `created_at, id`):
 * the seeded order, then two new users appended at the end in creation order.
 */
import { defineCase } from '../../case';
import * as userService from '../../../../src/modules/users/services/userService';

export default defineCase({
  name: 'users/users-list-order',
  source: '03-domains/03-users.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('list-seeded', () => userService.getUsers());
    await s.step('create-1', () => userService.createUser({ username: 'zeta', name: 'زيتا', role: 'cashier', maxDiscount: 0, active: true, password: 'x1234567' }));
    await s.step('create-2', () => userService.createUser({ username: 'alpha', name: 'ألفا', role: 'cashier', maxDiscount: 0, active: true, password: 'x1234567' }));
    await s.step('list-after', () => userService.getUsers());
  },
});
