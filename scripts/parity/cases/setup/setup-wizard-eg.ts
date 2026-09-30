/**
 * 02-setup §8(b): the full setup wizard on an empty install, Egypt — shell, business type, country
 * and tax (+ an extra currency), fiscal year from the go-live date, branches (rename main + one
 * more, with the duplicate-code and empty-list refusals), chart of accounts, payment methods, then
 * the opening step exactly as `StepOpening.vue` runs it: customers/suppliers/product created in the
 * wizard, `postOpeningBalances` (cash, customer, supplier and other lines; the 3900 difference
 * closed to capital), `postOpeningStock` for the main branch, `recloseOpeningBalanceEquity`; then
 * finish, log in, and read the GL/progress.
 *
 * Signed out until the wizard's own bootstrap (03 H-1): the mock's wizard writes carry user `''`,
 * Rust's the bootstrap admin (quirk Q-1). Steps record no user fields except the projected journal,
 * which leaves them out.
 *
 * Known red on the mock (reported by lane L4, 2026-09-29): `customer-allocation` /
 * `supplier-allocation` (§4.6) ignore opening balances, and `opening-balance-equity` (§4.9) is
 * checked before onboarding is complete, so the opening-stock step (3900 credited until the
 * reclose that follows) and every party opening line trip them. `setup/setup-wizard-sa` runs the
 * same wizard without party lines and stock and is green.
 */
import { defineCase } from '../../case';
import * as setupService from '../../../../src/modules/setup/services/setupService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as partyService from '../../../../src/modules/parties/services/partyService';
import * as productService from '../../../../src/modules/products/services/productService';
import * as catalogService from '../../../../src/modules/products/services/catalogService';
import * as settingsService from '../../../../src/modules/settings/services/settingsService';
import * as branchesService from '../../../../src/modules/settings/services/branchesService';

export default defineCase({
  name: 'setup/setup-wizard-eg',
  source: '03-domains/02-setup.md §8(b)',
  base: 'empty',
  user: null,
  allow: [{ path: '**.phones', emptyArrayOnly: true, reason: '05-parties D-6: phones is always an array in Rust; the mock leaves it absent on a party saved without phones (absent ≡ [])' }],
  async run(s) {
    await s.step('shell', () => setupService.ensureEmptyCompanyShell());
    await s.step('progress-start', () => setupService.getOnboardingProgress());

    await s.step('business-type', () => setupService.applyBusinessTypeDefaults('retail'));
    await s.step('business-type-progress', () => setupService.saveOnboardingProgress({ businessType: 'retail' }));
    await s.step('done-0', () => setupService.markStepDone('businessType', 0));

    await s.step('locked-before-posting', () => setupService.isBaseCurrencyLocked());
    await s.step('country-tax', () =>
      setupService.applyCountryTax({ country: 'EG', currency: 'EGP', vatRegistered: true, pricesIncludeTax: true, extraCurrencies: [{ code: 'USD', rate: 50.5 }, { code: '', rate: 1 }] }),
    );
    await s.step('done-1', () => setupService.markStepDone('countryTax', 1));
    await s.step('done-2', () => setupService.markStepDone('company', 2));

    await s.step('fiscal-year', () => setupService.applyFiscalYear(1, 1, '2026-03-15'));
    await s.step('done-3', () => setupService.markStepDone('fiscalYear', 3));

    await s.expectError('branches-empty', () => setupService.applyBranches([]));
    await s.expectError('branches-duplicate-code', () =>
      setupService.applyBranches([
        { name: 'الفرع الرئيسي', code: 'CAI' },
        { name: 'فرع مكرر', code: 'cai' },
      ]),
    );
    await s.step('branches', () =>
      setupService.applyBranches([
        { name: 'فرع القاهرة', code: 'cai' },
        { name: 'فرع الإسكندرية', code: 'ALX' },
      ]),
    );
    await s.step('done-4', () => setupService.markStepDone('branches', 4));

    await s.step('coa', () => setupService.applyCoaTemplate('standard', 'EG', 'retail'));
    await s.step('done-5', () => setupService.markStepDone('coa', 5));

    await s.step('payment-methods', () =>
      setupService.applyPaymentMethods([
        { name: 'نقداً', type: 'cash', accountRole: 'cash', active: true },
        { name: 'فيزا', type: 'card', accountRole: 'cardClearing', active: true },
        { name: 'فودافون كاش', type: 'wallet', accountRole: 'walletClearing', active: true },
        { name: 'آجل', type: 'credit', accountRole: 'receivable', active: true },
      ]),
    );
    await s.step('methods-after', () => settingsService.getPaymentMethods());
    await s.step('done-6', () => setupService.markStepDone('paymentMethods', 6));

    // --- Opening (StepOpening.vue) ---
    const customer = await s.step('customer', () => partyService.saveCustomer({ type: 'company', name: 'شركة النيل للتجارة', active: true }));
    const supplier = await s.step('supplier', () => partyService.saveSupplier({ type: 'company', name: 'مصنع الدلتا', active: true }));
    const units = await s.step('units', () => catalogService.getUnits());
    const product = await s.step('product', () =>
      productService.createProduct({ name: 'قميص قطن', sku: 'EG-001', unitId: units[0]?.id, type: 'product', costPrice: 120, price: 199, active: true } as Parameters<typeof productService.createProduct>[0]),
    );
    const accounts = await s.step('accounts-before-opening', () => accountingService.getAccounts());
    const code = (c: string) => accounts.find((a) => a.code === c)!.id;
    await s.step('equity-net-before', () => setupService.getOpeningBalanceEquityNet());
    await s.step('opening-balances', () =>
      setupService.postOpeningBalances(
        {
          date: '2026-03-15',
          cash: [
            { accountId: code('1110'), amount: 25000 },
            { accountId: code('1120'), amount: 140000.5 },
          ],
          customers: [{ partyKind: 'customer', partyId: customer.id, amount: 8000, side: 'debit' }],
          suppliers: [{ partyKind: 'supplier', partyId: supplier.id, amount: 12500, side: 'credit' }],
          other: [
            { accountId: code('1220'), side: 'debit', amount: 30000, description: 'أجهزة' },
            { accountId: code('2210'), side: 'credit', amount: 50000 },
            { accountId: code('1210'), side: 'debit', amount: 0 },
          ],
        },
        'capital',
      ),
    );
    const branches = await s.step('branches-for-stock', () => branchesService.getBranches());
    await s.step('opening-stock', () => setupService.postOpeningStock(branches[0].id, '2026-03-15', [{ productId: product.id, qty: 40, unitCost: 118.5 }, { productId: product.id, qty: 0, unitCost: 1 }]));
    await s.step('reclose', () => setupService.recloseOpeningBalanceEquity('2026-03-15', 'capital'));
    await s.step('equity-net-after', () => setupService.getOpeningBalanceEquityNet());
    await s.step('first-use-posted', () => setupService.isFirstUsePosted());
    await s.step('locked-after-posting', () => setupService.isBaseCurrencyLocked());
    await s.step('done-7', () => setupService.markStepDone('opening', 7));
    await s.step('skip-8', () => setupService.markStepSkipped('users'));
    await s.step('skip-9', () => setupService.markStepSkipped('printing'));

    // SetupWizardPage records the `ready` step before finishing (finishing ends the bootstrap
    // session on Rust, 02-setup D-1).
    await s.step('done-10', () => setupService.markStepDone('ready', 10));
    await s.step('finish', () => setupService.finishOnboarding());
    await s.login('admin', 'admin123');

    await s.step('progress-end', () => setupService.getOnboardingProgress());
    await s.step('settings', () => settingsService.getSettings());
    await s.step('taxes', () => settingsService.getTaxes());
    await s.step('currencies', () => branchesService.getCurrencies());
    await s.step('fiscal-years', () => accountingService.getFiscalYears());
    await s.step('journal', async () =>
      (await accountingService.getJournalEntries()).map((e) => ({
        number: e.number,
        date: e.date,
        type: e.type,
        description: e.description,
        sourceRef: e.sourceRef,
        lines: e.lines.map((l) => ({ accountId: l.accountId, debit: l.debit, credit: l.credit, partyId: l.partyId, branchId: l.branchId })),
      })),
    );
    await s.step('product-after', () => productService.getProduct(product.id));
  },
});
