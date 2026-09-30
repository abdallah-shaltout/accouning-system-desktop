/**
 * 02-setup §8(b): the setup wizard switched to Saudi Arabia — business type `pharmacy` (units),
 * country/tax SA (base currency EGP → SAR, 15 %, `Asia/Riyadh`), a July-start fiscal year from the
 * go-live date, one branch, the `detailed` chart (SA + pharmacy add-ons), payment methods, an
 * opening entry with cash/bank/other lines whose difference is closed to the owner's current
 * account, then finish and log in. No party lines or opening stock here (see
 * `setup/setup-wizard-eg` for the full opening step and why it is red on the mock).
 *
 * Base: the empty-company shell (`seedEmptyCompany('EG')`, what `ensureEmptyCompanyShell` builds
 * on a fresh install). The `empty` base itself cannot run on the mock pass: the mock's
 * `runAllInvariants` throws on a database with no chart of accounts (`glBalance` →
 * `accountFor`), although its contract says it never throws — reported by lane L4, 2026-09-29.
 * On Rust the shell is imported (no `__reset_empty`), so Rust's own shell seeding is exercised only
 * by `setup/setup-wizard-eg`. Wizard writes carry user `''` on the mock and the bootstrap admin on
 * Rust (quirk Q-1); only the two journal-entry reads record one, allowlisted citing Q-1.
 */
import { defineCase } from '../../case';
import * as setupService from '../../../../src/modules/setup/services/setupService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as catalogService from '../../../../src/modules/products/services/catalogService';
import * as settingsService from '../../../../src/modules/settings/services/settingsService';
import * as branchesService from '../../../../src/modules/settings/services/branchesService';
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
  name: 'setup/setup-wizard-sa',
  source: '03-domains/02-setup.md §8(b)',
  base: shell,
  user: null,
  allow: ['opening-entry', 'closing-entry'].flatMap((step) =>
    ['createdBy', 'postedBy', 'createdByName'].map((field) => ({
      path: `steps.${step}.value.${field}`,
      reason: "02-setup Q-1: wizard writes carry user '' on the mock ('—' as its name) and the bootstrap admin on Rust",
    })),
  ),
  async run(s) {
    await s.step('progress-start', () => setupService.getOnboardingProgress());
    await s.step('business-type', () => setupService.applyBusinessTypeDefaults('pharmacy'));
    await s.step('units', () => catalogService.getUnits());
    // A second call is a no-op once units exist.
    await s.step('business-type-again', () => setupService.applyBusinessTypeDefaults('supermarket'));
    await s.step('business-type-progress', () => setupService.saveOnboardingProgress({ businessType: 'pharmacy', completedStep: 0 }));
    await s.step('done-0', () => setupService.markStepDone('businessType', 0));
    await s.step('done-0-again', () => setupService.markStepDone('businessType'));

    await s.step('country-tax', () => setupService.applyCountryTax({ country: 'SA', currency: 'SAR', vatRegistered: true, pricesIncludeTax: true, extraCurrencies: [] }));
    await s.step('settings-after-country', () => settingsService.getSettings());
    await s.step('taxes-after-country', () => settingsService.getTaxes());
    await s.step('done-1', () => setupService.markStepDone('countryTax', 1));
    await s.step('skip-2', () => setupService.markStepSkipped('company'));

    await s.step('fiscal-year', () => setupService.applyFiscalYear(7, 1, '2026-04-01'));
    await s.step('fiscal-years', () => accountingService.getFiscalYears());
    await s.step('done-3', () => setupService.markStepDone('fiscalYear', 3));

    await s.step('branches', () => setupService.applyBranches([{ name: 'صيدلية الرياض', code: 'ryd' }]));
    await s.step('done-4', () => setupService.markStepDone('branches', 4));
    await s.step('coa', () => setupService.applyCoaTemplate('detailed', 'SA', 'pharmacy'));
    await s.step('done-5', () => setupService.markStepDone('coa', 5));
    await s.step('payment-methods', () =>
      setupService.applyPaymentMethods([
        { name: 'نقداً', type: 'cash', accountRole: 'cash', active: true },
        { name: 'مدى', type: 'card', accountRole: 'cardClearing', active: true },
        { name: 'تحويل بنكي', type: 'bank_transfer', accountRole: 'bank', active: true },
      ]),
    );
    await s.step('methods-after', () => settingsService.getPaymentMethods());
    await s.step('done-6', () => setupService.markStepDone('paymentMethods', 6));

    const accounts = await s.step('accounts', () => accountingService.getAccounts());
    const code = (c: string) => accounts.find((a) => a.code === c)!.id;
    const posted = await s.step('opening-balances', () =>
      setupService.postOpeningBalances(
        {
          date: '2026-04-01',
          cash: [{ accountId: code('1110'), amount: 18000 }, { accountId: code('1120'), amount: 95000.25 }],
          customers: [],
          suppliers: [],
          other: [
            { accountId: code('1220'), side: 'debit', amount: 42000, description: 'ثلاجات أدوية' },
            { accountId: code('2210'), side: 'credit', amount: 60000 },
          ],
        },
        'ownerCurrent',
      ),
    );
    await s.step('opening-entry', () => accountingService.getJournalEntry(posted.openingEntryId));
    await s.step('closing-entry', () => accountingService.getJournalEntry(posted.closingEntryId!));
    await s.step('equity-net', () => setupService.getOpeningBalanceEquityNet());
    await s.step('reclose-noop', () => setupService.recloseOpeningBalanceEquity('2026-04-01', 'ownerCurrent'));
    await s.step('first-use-posted', () => setupService.isFirstUsePosted());
    await s.step('done-7', () => setupService.markStepDone('opening', 7));
    await s.step('finish', () => setupService.finishOnboarding());
    await s.login('admin', 'admin123');
    await s.step('progress-end', () => setupService.getOnboardingProgress());
    await s.step('branches-end', () => branchesService.getBranches());
  },
});
