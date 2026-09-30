/**
 * 08 §8(b) refund-partial-then-final — a two-line cash sale refunded in two steps: a partial,
 * non-final refund (restocked, cash back, a REFUND_CASH movement on the open shift) and the final
 * refund of the rest, which ends on the exact remainders and flips the invoice to REFUNDED. Every
 * §3.5 refusal is recorded along the way (unknown invoice, empty selection, unknown line, over the
 * remaining qty, customer credit on a walk-in invoice, refunding a REFUNDED invoice), and
 * `getRefund` (plus its NOT_FOUND) reads a credit note back.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

/** An unknown id in the only shape a real id has (a UUID — Rust decodes ids strictly, so a free-form
 * `'x-does-not-exist'` would test id parsing, not the lookup's §3 NOT_FOUND/VALIDATION refusal). */
const MISSING_ID = '00000000-0000-7000-8000-000000000001';

export default defineCase({
  name: 'invoices/refund-partial-then-final',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const shift = await s.step('open-shift', () => invoiceService.openPosShift({ terminalId: 'pos-1', openingFloat: 300 }));
    const sale = await s.step('sale', () =>
      invoiceService.createSale({
        lines: [
          { productId: s.baseId('prd-10'), qty: 3, price: 119 },
          { productId: s.baseId('prd-13'), qty: 2, price: 99, discount: 4 },
        ],
        discountRate: 0,
        paymentMethod: 'cash',
        paidAmount: 1000,
        shiftId: shift.id,
      }),
    );
    const [l1, l2] = sale.lines;

    await s.expectError('unknown-invoice', () => invoiceService.createRefund({ invoiceId: MISSING_ID, lines: [{ invoiceLineId: l1.id, qty: 1 }] }));
    await s.expectError('nothing-selected', () => invoiceService.createRefund({ invoiceId: sale.id, lines: [{ invoiceLineId: l1.id, qty: 0 }] }));
    await s.expectError('customer-credit-walk-in', () =>
      invoiceService.createRefund({ invoiceId: sale.id, lines: [{ invoiceLineId: l1.id, qty: 1 }], refundMethod: 'customer_credit' }),
    );
    await s.expectError('unknown-line', () => invoiceService.createRefund({ invoiceId: sale.id, lines: [{ invoiceLineId: 'line-does-not-exist', qty: 1 }] }));
    await s.expectError('over-remaining', () => invoiceService.createRefund({ invoiceId: sale.id, lines: [{ invoiceLineId: l1.id, qty: 4 }] }));

    await s.setClock('2026-06-30T09:05:00.000Z');
    const partial = await s.step('refund-partial', () =>
      invoiceService.createRefund({ invoiceId: sale.id, reason: 'مقاس غير مناسب', lines: [{ invoiceLineId: l1.id, qty: 1, restock: true }] }),
    );
    await s.step('get-refund', () => invoiceService.getRefund(partial.id));
    await s.expectError('get-refund-unknown', () => invoiceService.getRefund(MISSING_ID));
    await s.step('detail-after-partial', () => invoiceService.getInvoice(sale.id));
    await s.expectError('over-remaining-after-partial', () =>
      invoiceService.createRefund({ invoiceId: sale.id, lines: [{ invoiceLineId: l1.id, qty: 3 }] }),
    );

    await s.setClock('2026-06-30T09:10:00.000Z');
    const final = await s.step('refund-final', () =>
      invoiceService.createRefund({
        invoiceId: sale.id,
        lines: [
          { invoiceLineId: l1.id, qty: 2 },
          { invoiceLineId: l2.id, qty: 2 },
        ],
      }),
    );
    await s.step('detail-after-final', () => invoiceService.getInvoice(sale.id));
    for (const [tag, id] of [['partial', partial.id], ['final', final.id]] as const) {
      const refs = await s.step(`journal-refs-${tag}`, () => accountingService.getJournalEntriesForSource('refund', id));
      for (let i = 0; i < refs.length; i++) await s.step(`journal-${tag}-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    }
    await s.expectError('refund-refunded-invoice', () => invoiceService.createRefund({ invoiceId: sale.id, lines: [{ invoiceLineId: l1.id, qty: 1 }] }));
    await s.step('x-report', () => invoiceService.getXReport(shift.id));
  },
});
