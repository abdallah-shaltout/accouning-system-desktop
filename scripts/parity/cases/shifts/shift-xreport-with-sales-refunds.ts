/**
 * 08b §8(b) shift-xreport-with-sales-refunds — the X-report mid-shift: a cash sale, a cash + mada
 * split sale and a card sale feed `salesByMethod` (first-appearance order), `salesTotal` and
 * `invoiceCount`; a cash refund of the first sale adds a REFUND_CASH movement; expected cash =
 * float + cash sales − cash refunds. Then the Z (close on expected) and the shift list.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';

export default defineCase({
  name: 'shifts/shift-xreport-with-sales-refunds',
  source: '03-domains/08b-pos-shifts.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const shift = await s.step('open', () => invoiceService.openPosShift({ terminalId: 'pos-1', openingFloat: 500 }));
    await s.setClock('2026-06-30T09:10:00.000Z');
    const cash = await s.step('sale-cash', () =>
      invoiceService.createSale({
        lines: [{ productId: s.baseId('prd-10'), qty: 2, price: 119 }],
        discountRate: 0,
        paymentMethod: 'cash',
        paidAmount: 238,
        shiftId: shift.id,
      }),
    );
    await s.setClock('2026-06-30T09:20:00.000Z');
    await s.step('sale-split', () =>
      invoiceService.createSale({
        lines: [{ productId: s.baseId('prd-12'), qty: 1, price: 229 }],
        discountRate: 0,
        paymentMethod: 'cash',
        paidAmount: 229,
        tenders: [
          { paymentMethodId: s.baseId('pm-mada'), amount: 129 },
          { paymentMethodId: s.baseId('pm-cash'), amount: 100 },
        ],
        shiftId: shift.id,
      }),
    );
    await s.setClock('2026-06-30T09:30:00.000Z');
    await s.step('sale-card', () =>
      invoiceService.createSale({
        lines: [{ productId: s.baseId('prd-20'), qty: 1, price: 269 }],
        discountRate: 0,
        paymentMethod: 'card',
        paidAmount: 0,
        shiftId: shift.id,
      }),
    );
    await s.step('x-before-refund', () => invoiceService.getXReport(shift.id));
    await s.setClock('2026-06-30T09:40:00.000Z');
    await s.step('refund-cash', () => invoiceService.createRefund({ invoiceId: cash.id, lines: [{ invoiceLineId: cash.lines[0].id, qty: 1 }], refundMethod: 'cash' }));
    const x = await s.step('x-after-refund', () => invoiceService.getXReport(shift.id));
    await s.setClock('2026-06-30T17:00:00.000Z');
    await s.step('close', () => invoiceService.closePosShift(shift.id, { countedCash: x.expectedCash }));
    await s.step('list-all', () => invoiceService.getShifts());
  },
});
