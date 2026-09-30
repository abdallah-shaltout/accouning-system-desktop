/**
 * L1 platform lane. Import parity for `demo-eg` (00-import.md §8(b)): the EG country profile
 * (14% VAT, EGP, `Africa/Cairo`) exercised the same way as `import/import-demo-sa` — see that
 * case's doc comment for why the base itself stands in for "after import" here.
 */
import { defineCase } from '../../case';
import * as settingsService from '../../../../src/modules/settings/services/settingsService';
import * as branchesService from '../../../../src/modules/settings/services/branchesService';
import * as userService from '../../../../src/modules/users/services/userService';
import * as partyService from '../../../../src/modules/parties/services/partyService';
import * as productService from '../../../../src/modules/products/services/productService';

export default defineCase({
  name: 'import/import-demo-eg',
  source: '03-domains/00-import.md §8(b)',
  base: 'demo-eg',
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
