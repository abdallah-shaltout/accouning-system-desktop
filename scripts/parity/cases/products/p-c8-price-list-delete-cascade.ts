/**
 * 06-products §8(b) P-C8: deleting a price list strips its entries from every product's `prices`
 * (Q-3: `unitPrices` untouched); a list assigned to a user can't be deleted until unassigned.
 */
import { defineCase } from '../../case';
import * as catalogService from '../../../../src/modules/products/services/catalogService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as userService from '../../../../src/modules/users/services/userService';
import type { User, UserInput } from '../../../../src/modules/users/types';

function userInput(u: User, priceListId: string | undefined): UserInput {
  return {
    username: u.username,
    name: u.name,
    phone: u.phone,
    role: u.role,
    maxDiscount: u.maxDiscount,
    priceListId,
    active: u.active,
    allowedBranches: u.allowedBranches,
    homeBranch: u.homeBranch,
  };
}

export default defineCase({
  name: 'products/p-c8-price-list-delete-cascade',
  source: '03-domains/06-products.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const prd1 = s.baseId('prd-1');
    const prd2 = s.baseId('prd-2');
    const pl = await s.step('create-list', () => catalogService.savePriceList({ name: 'تخفيضات الصيف', active: true }));
    await s.step('set-values', () => catalogService.setPriceListValues(pl.id, { [prd1]: 100, [prd2]: 150 }));
    await s.step('prd-1-with-list', () => productService.getProduct(prd1));

    const cashier = await s.step('cashier', () => userService.getUser(s.baseId('usr-4')));
    await s.step('assign-to-cashier', () => userService.updateUser(cashier.id, userInput(cashier, pl.id)));
    await s.expectError('delete-assigned', () => catalogService.deletePriceList(pl.id));
    await s.step('unassign', () => userService.updateUser(cashier.id, userInput(cashier, undefined)));
    await s.step('delete-new-list', () => catalogService.deletePriceList(pl.id));
    await s.step('prd-1-after-new-list-delete', () => productService.getProduct(prd1));
    await s.step('prd-2-after-new-list-delete', () => productService.getProduct(prd2));

    // A seeded list with seeded entries (prd-1 has a pl-vip price).
    await s.step('delete-seed-vip', () => catalogService.deletePriceList(s.baseId('pl-vip')));
    await s.step('prd-1-after-vip-delete', () => productService.getProduct(prd1));
    await s.step('price-lists', () => catalogService.getPriceLists());
    // Deleting a missing id is a silent no-op.
    await s.step('delete-missing', () => catalogService.deletePriceList(s.baseId('cus-1')));
  },
});
