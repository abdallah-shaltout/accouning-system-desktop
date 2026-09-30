/**
 * 08b §8(b) shift-close-over — the drawer counts 5.00 more than expected: the close posts
 * Dr cash / Cr cash-over 5.00 (source = the shift), and the closed shift records expected, counted
 * and variance (invariant 11: expected − counted = the posted over/short amount).
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import { round2 } from '../../../../src/modules/core/helpers/numbers';

export default defineCase({
  name: 'shifts/shift-close-over',
  source: '03-domains/08b-pos-shifts.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const shift = await s.step('open', () => invoiceService.openPosShift({ terminalId: 'pos-1', openingFloat: 500 }));
    await s.setClock('2026-06-30T09:05:00.000Z');
    await s.step('cash-sale', () =>
      invoiceService.createSale({
        lines: [{ productId: s.baseId('prd-5'), qty: 2, price: 145 }],
        discountRate: 0,
        paymentMethod: 'cash',
        paidAmount: 290,
        shiftId: shift.id,
      }),
    );
    const x = await s.step('x-report', () => invoiceService.getXReport(shift.id));
    await s.setClock('2026-06-30T17:00:00.000Z');
    await s.step('close', () => invoiceService.closePosShift(shift.id, { countedCash: round2(x.expectedCash + 5) }));
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('shift', shift.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    await s.step('get-closed', () => invoiceService.getShift(shift.id));
  },
});
