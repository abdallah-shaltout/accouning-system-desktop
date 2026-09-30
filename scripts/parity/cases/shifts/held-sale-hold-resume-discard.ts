/**
 * 08b §8(b) held-sale-hold-resume-discard — POS F6 held carts: hold two carts at distinct times,
 * list (newest first, this terminal only), resume the first (it leaves the list), resuming it again
 * is NOT_FOUND, discard the second, discarding a missing id is a no-op, and the list ends empty.
 * Held sales never post or touch stock.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';

/** An unknown id in the only shape a real id has (a UUID — Rust decodes ids strictly, so a free-form
 * `'x-does-not-exist'` would test id parsing, not the lookup's §3 NOT_FOUND/VALIDATION refusal). */
const MISSING_ID = '00000000-0000-7000-8000-000000000001';

export default defineCase({
  name: 'shifts/held-sale-hold-resume-discard',
  source: '03-domains/08b-pos-shifts.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    await s.step('list-empty', () => invoiceService.getHeldSales('pos-1'));
    await s.setClock('2026-06-30T09:10:00.000Z');
    const first = await s.step('hold-1', () =>
      invoiceService.holdSale({
        label: 'عميل الانتظار',
        terminalId: 'pos-1',
        customerId: s.baseId('cus-8'),
        discountRate: 5,
        discountIsPct: true,
        note: 'سيعود بعد قليل',
        lines: [
          { productId: s.baseId('prd-1'), qty: 2, price: 129 },
          { productId: s.baseId('prd-3'), qty: 1, price: 79 },
        ],
      }),
    );
    await s.setClock('2026-06-30T09:20:00.000Z');
    const second = await s.step('hold-2', () =>
      invoiceService.holdSale({
        terminalId: 'pos-1',
        discountRate: 0,
        discountIsPct: false,
        lines: [{ productId: s.baseId('prd-14'), qty: 4, price: 39 }],
      }),
    );
    await s.step('list-two', () => invoiceService.getHeldSales('pos-1'));
    await s.step('resume-1', () => invoiceService.resumeHeldSale(first.id));
    await s.step('list-after-resume', () => invoiceService.getHeldSales('pos-1'));
    await s.expectError('resume-again', () => invoiceService.resumeHeldSale(first.id));
    await s.step('discard-2', () => invoiceService.discardHeldSale(second.id));
    await s.step('discard-missing', () => invoiceService.discardHeldSale(MISSING_ID));
    await s.step('list-end', () => invoiceService.getHeldSales('pos-1'));
  },
});
