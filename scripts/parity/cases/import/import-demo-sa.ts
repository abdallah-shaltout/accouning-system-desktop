/**
 * L1 platform lane. Import parity for `demo-sa` (00-import.md §8(b)): the base itself IS the
 * importer's target state (the parity harness's `demo-sa` base is `seedDatabase(..., 'SA')`, the
 * exact snapshot the Rust importer consumes, `scripts/parity/bases.ts`) — so this case's job is to
 * call every list-returning service across the domains the importer's 46-table map touches, and let
 * `--mock-only`'s run1-vs-run2 diff prove every one of them is deterministic on top of that seed
 * (list order, computed fields). The books snapshot the runner appends automatically (trial balance,
 * customers, suppliers, inventory) covers the accounting side; this case covers the reads books.ts
 * doesn't: settings/dimensions, users, taxes/payment methods, parties detail lists, products.
 */
import { defineCase } from '../../case';
import * as settingsService from '../../../../src/modules/settings/services/settingsService';
import * as branchesService from '../../../../src/modules/settings/services/branchesService';
import * as userService from '../../../../src/modules/users/services/userService';
import * as partyService from '../../../../src/modules/parties/services/partyService';
import * as productService from '../../../../src/modules/products/services/productService';

export default defineCase({
  name: 'import/import-demo-sa',
  source: '03-domains/00-import.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  // Seeded service/medicine rows (`prd-svc-1`, `prd-svc-2`, `prd-panadol`) have no `prices` key; Rust
  // always returns the (empty) array (06-products Q-7: `[]` ≡ absent for this field).
  allow: [{ path: 'steps.products.value[].prices', reason: '06-products Q-7: Rust returns prices: [] where a seeded mock product has no prices key' }],
  async run(s) {
    await s.step('settings', () => settingsService.getSettings());
    await s.step('taxes', () => settingsService.getTaxes());
    await s.step('payment-methods', () => settingsService.getPaymentMethods());
    await s.step('branches', () => branchesService.getBranches());
    await s.step('cost-centers', () => branchesService.getCostCenters());
    await s.step('currencies', () => branchesService.getCurrencies());
    await s.step('exchange-rates', () => branchesService.getExchangeRates());
    await s.step('users', () => userService.getUsers());
    await s.step('customer-groups', () => partyService.getPartyGroups('customer'));
    await s.step('supplier-groups', () => partyService.getPartyGroups('supplier'));
    await s.step('customers', () => partyService.getCustomers());
    await s.step('suppliers', () => partyService.getSuppliers());
    await s.step('products', () => productService.getProducts());
  },
});
