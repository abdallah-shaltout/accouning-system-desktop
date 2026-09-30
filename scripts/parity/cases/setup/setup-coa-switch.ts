/**
 * 02-setup §8(b): switching the chart-of-accounts template during the wizard (§3.5) — detailed →
 * detailed + pharmacy add-on → standard + pharmacy, account identity kept by code (decision D-7: shared codes keep their id,
 * codes missing from the new template are removed, order = template order), then the wholesale
 * replace guards once anything is posted: chart, fiscal year, payment methods and country/currency
 * are all refused (FORBIDDEN) after the first journal entry.
 *
 * Base: the empty-company shell (see `setup/setup-wizard-sa` for why not `empty`). The `basic`
 * template is not applied: it has no card-clearing account, and the mock's `runAllInvariants`
 * throws on a missing role account (`checkClearingAccounts` → `accountFor('cardClearing')`), which
 * aborts the pass — reported by lane L4, 2026-09-29. The refusal after posting still sends `basic`.
 */
import { defineCase } from '../../case';
import * as setupService from '../../../../src/modules/setup/services/setupService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import { db, type MockDb } from '../../../../src/mocks/db';
import { seedEmptyCompany } from '../../../../src/mocks/seed';
import { clone, resetIdCounters } from '../../../../src/mocks/utils';
import { pinClock } from '../../clock';
import { SEED_NOW } from '../../bases';

function shell(): MockDb {
  pinClock(SEED_NOW, 'Africa/Cairo', 'base:setup-shell');
  resetIdCounters();
  seedEmptyCompany('EG');
  return clone(db);
}

export default defineCase({
  name: 'setup/setup-coa-switch',
  source: '03-domains/02-setup.md §8(b)',
  base: shell,
  user: null,
  async run(s) {
    await s.step('progress', () => setupService.getOnboardingProgress());
    await s.step('accounts-shell', () => accountingService.getAccounts());
    await s.step('apply-detailed', () => setupService.applyCoaTemplate('detailed', 'EG', 'retail'));
    await s.step('apply-detailed-pharmacy', () => setupService.applyCoaTemplate('detailed', 'EG', 'pharmacy'));
    await s.step('apply-standard-pharmacy', () => setupService.applyCoaTemplate('standard', 'EG', 'pharmacy'));
    await s.step('accounts-after', () => accountingService.getAccounts());
    await s.step('progress-after', () => setupService.getOnboardingProgress());

    // First posting: an opening entry (cash vs capital through 3900).
    const cash = (await s.step('cash-account', async () => (await accountingService.getAccounts()).filter((a) => a.code === '1110')))[0];
    await s.step('opening', () => setupService.postOpeningBalances({ date: '2026-01-01', cash: [{ accountId: cash.id, amount: 5000 }], customers: [], suppliers: [], other: [] }, 'capital'));
    await s.expectError('coa-locked', () => setupService.applyCoaTemplate('basic', 'EG', 'retail'));
    await s.expectError('fiscal-year-locked', () => setupService.applyFiscalYear(1, 1, '2026-01-01'));
    await s.expectError('payment-methods-locked', () => setupService.applyPaymentMethods([{ name: 'نقداً', type: 'cash', accountRole: 'cash', active: true }]));
    await s.step('currency-locked', () => setupService.isBaseCurrencyLocked());
    await s.expectError('country-locked', () => setupService.applyCountryTax({ country: 'SA', currency: 'SAR', vatRegistered: true, pricesIncludeTax: true, extraCurrencies: [] }));
    await s.step('accounts-final', () => accountingService.getAccounts({ from: '2026-01-01', to: '2026-01-01' }));
  },
});
