/**
 * 12b-period-close §8(b): paying the VAT due now (§3.4) — a payment voucher Dr VAT payable /
 * Cr the payment method's account, `VCH-` number, returned as the voucher's journal entry.
 * Refusals: zero and negative amounts. No cap at the payable balance (D-A7 kept, quirk Q6), so a
 * payment above it is accepted.
 *
 * Base: the SA demo plus May's settlement posted through the mock's `postVatSettlement` (so the
 * payable balance exists); the case itself never settles, because a successful settlement trips
 * the mock's §4.5 VAT invariant (see `accounting/vat-settlement-payable`).
 */
import { defineCase } from '../../case';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import { db, session, type MockDb } from '../../../../src/mocks/db';
import { seedDatabase } from '../../../../src/mocks/seed';
import { clone, resetIdCounters } from '../../../../src/mocks/utils';
import { postVatSettlement } from '../../../../src/mocks/backend/journal';
import { pinClock } from '../../clock';
import { SEED_NOW } from '../../bases';

function baseWithMaySettled(): MockDb {
  pinClock(SEED_NOW, 'Asia/Riyadh', 'base:vat-pay');
  resetIdCounters();
  seedDatabase(new Date(SEED_NOW), 'SA');
  db.settings.country ??= 'SA';
  session.userId = 'usr-3';
  postVatSettlement('2026-05-01', '2026-05-31', 'usr-3');
  session.userId = '';
  return clone(db);
}

export default defineCase({
  name: 'accounting/vat-pay',
  source: '03-domains/12b-period-close.md §8(b)',
  base: baseWithMaySettled,
  user: 'accountant',
  async run(s) {
    const payable = async () => (await accountingService.getAccounts()).filter((a) => a.code === '2155');
    await s.step('payable-before', payable);
    await s.expectError('zero', () => accountingService.payVatSettlementNow(0, s.baseId('pm-bank-transfer')));
    await s.expectError('negative', () => accountingService.payVatSettlementNow(-10, s.baseId('pm-bank-transfer')));
    const paid = await s.step('pay-bank', () => accountingService.payVatSettlementNow(7021.5, s.baseId('pm-bank-transfer')));
    await s.step('detail', () => accountingService.getJournalEntry(paid.id));
    await s.step('payable-after', payable);
    await s.step('pay-over-balance-cash', () => accountingService.payVatSettlementNow(100, s.baseId('pm-cash')));
    await s.step('payable-negative', payable);
  },
});
