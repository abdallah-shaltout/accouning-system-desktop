/**
 * 10 §8(b) settlement-mixed-card-wallet — today's sales tendered by mada and STC Pay (card clearing
 * and wallet clearing), then one settlement of both groups: one credit line per clearing role in
 * first-appearance order, the fee to card fees, the deposit to the bank.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as voucherService from '../../../../src/modules/vouchers/services/voucherService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import { round2 } from '../../../../src/modules/core/helpers/numbers';

export default defineCase({
  name: 'vouchers/settlement-mixed-card-wallet',
  source: '03-domains/10-vouchers.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const mada = s.baseId('pm-mada');
    const stc = s.baseId('pm-stc-pay');
    await s.step('sale-mada-stc', () =>
      invoiceService.createSale({
        lines: [{ productId: s.baseId('prd-19'), qty: 1, price: 329 }],
        discountRate: 0,
        paymentMethod: 'card',
        paidAmount: 329,
        tenders: [
          { paymentMethodId: mada, amount: 200 },
          { paymentMethodId: stc, amount: 129 },
        ],
      }),
    );
    await s.setClock('2026-06-30T09:05:00.000Z');
    await s.step('sale-stc', () =>
      invoiceService.createSale({
        lines: [{ productId: s.baseId('prd-22'), qty: 1, price: 115 }],
        discountRate: 0,
        paymentMethod: 'card',
        paidAmount: 115,
        tenders: [{ paymentMethodId: stc, amount: 115 }],
      }),
    );
    const groups = await s.step('unsettled', () => voucherService.getUnsettledTenderGroups());
    const picked = groups.filter((g) => g.date === '2026-06-30' && (g.paymentMethodId === mada || g.paymentMethodId === stc));
    const estimate = await s.step('estimate', () => voucherService.estimateSettlementFee(picked));
    const gross = round2(picked.reduce((a, g) => a + g.total, 0));
    await s.setClock('2026-06-30T20:00:00.000Z');
    const stl = await s.step('settle', () =>
      voucherService.createCardSettlement({
        date: '2026-06-30T20:00:00.000Z',
        groups: picked.map((g) => ({ date: g.date, paymentMethodId: g.paymentMethodId })),
        depositAmount: round2(gross - estimate),
      }),
    );
    await s.step('get', () => voucherService.getCardSettlement(stl.id));
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('settlement', stl.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    await s.step('unsettled-after', () => voucherService.getUnsettledTenderGroups());
  },
});
