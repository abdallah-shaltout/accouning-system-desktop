/**
 * 01-settings §8(b) (lane L4): FX revaluation — the default rates (latest rate per active
 * currency, future-dated included, quirk Q-8), the preview of open FC balances at a chosen rate,
 * posting it (revaluation entry dated `date` + its mirror dated the 1st of the next month,
 * source `fxReval`, `FXR-<date>`; the mirror is a fresh posting, not linked by `reversalOfId`,
 * quirk Q-10), and the GL after (the runner's trial balance + the FX accounts). Refusal: rates that
 * leave no balance to revalue.
 *
 * The demo's only FC invoice is fully paid, so the story first makes an open FC balance: a USD
 * credit sale to the USD customer at 49.20.
 */
import { defineCase } from '../../case';
import * as branchesService from '../../../../src/modules/settings/services/branchesService';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'settings/settings-revaluation-post',
  source: '03-domains/01-settings.md §8(b)',
  lane: 'L4',
  base: 'demo-sa',
  // Admin: revaluation is a `settings.*` write (01-settings §2: Settings:Write) and the accountant
  // has no Settings access (`permissions.ts`); the mock service has no role check of its own.
  user: 'admin',
  async run(s) {
    await s.step('default-rates', () => branchesService.getDefaultRevaluationRates());
    await s.step('preview-before-sale', () => branchesService.getRevaluationPreview({ USD: 50 }));
    await s.step('fc-sale', () =>
      invoiceService.createSale({
        customerId: s.baseId('cus-usd-1'),
        lines: [{ productId: s.baseId('prd-1'), qty: 3, price: 40 }],
        discountRate: 0,
        paymentMethod: 'credit',
        paidAmount: 0,
        source: 'DESK',
        currency: 'USD',
        exchangeRate: 49.2,
      }),
    );
    await s.step('preview-50', () => branchesService.getRevaluationPreview({ USD: 50 }));
    await s.step('preview-no-rate', () => branchesService.getRevaluationPreview({}));
    await s.expectError('nothing-to-revalue', () => branchesService.postRevaluation('2026-06-30', { USD: 49.2 }));
    const result = await s.step('post', () => branchesService.postRevaluation('2026-06-30', { USD: 50.1234 }));
    await s.step('entry', () => accountingService.getJournalEntry(result.entryId));
    await s.step('mirror', () => accountingService.getJournalEntry(result.reversalEntryId));
    await s.step('fx-accounts', async () => (await accountingService.getAccounts()).filter((a) => a.systemRole === 'fxGain' || a.systemRole === 'fxLoss' || a.systemRole === 'receivable'));
    await s.step('fx-accounts-june', async () => (await accountingService.getAccounts({ from: '2026-06-01', to: '2026-06-30' })).filter((a) => a.systemRole === 'fxGain' || a.systemRole === 'receivable'));
  },
});
