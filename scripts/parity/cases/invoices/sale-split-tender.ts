/**
 * 08 §8(b) sale-split-tender — one sale paid part cash, part mada: the mada tender posts to the
 * card-clearing account, and only the cash part becomes a SALE_CASH movement on the open shift.
 * A second split sale (cash + STC Pay) without a shift records no movement at all.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

export default defineCase({
  name: 'invoices/sale-split-tender',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const shift = await s.step('open-shift', () => invoiceService.openPosShift({ terminalId: 'pos-1', openingFloat: 500 }));
    await s.setClock('2026-06-30T09:05:00.000Z');
    const withShift = await s.step('sale-with-shift', () =>
      invoiceService.createSale({
        lines: [{ productId: s.baseId('prd-3'), qty: 2, price: 79 }],
        discountRate: 0,
        paymentMethod: 'cash',
        paidAmount: 158,
        tenders: [
          { paymentMethodId: s.baseId('pm-cash'), amount: 58 },
          { paymentMethodId: s.baseId('pm-mada'), amount: 100, reference: 'RRN-4411' },
        ],
        shiftId: shift.id,
      }),
    );
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('invoice', withShift.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    await s.step('x-report-1', () => invoiceService.getXReport(shift.id));

    await s.setClock('2026-06-30T09:10:00.000Z');
    const noShift = await s.step('sale-no-shift', () =>
      invoiceService.createSale({
        lines: [{ productId: s.baseId('prd-17'), qty: 1, price: 59 }],
        discountRate: 0,
        paymentMethod: 'cash',
        paidAmount: 59,
        tenders: [
          { paymentMethodId: s.baseId('pm-cash'), amount: 20 },
          { paymentMethodId: s.baseId('pm-stc-pay'), amount: 39 },
        ],
      }),
    );
    await s.step('detail-no-shift', () => invoiceService.getInvoice(noShift.id));
    await s.step('x-report-2', () => invoiceService.getXReport(shift.id));
  },
});
