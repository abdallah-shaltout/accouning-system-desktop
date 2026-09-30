/**
 * L5 (03-domains/13-reports.md §8(b)): "a multi-currency invoice and a partial refund, then the
 * VAT report and detail". Uses the seed's own FX worked example (`cus-usd-1`'s USD invoice,
 * `seed/branches9.ts`, docs/v2/10 §2) — already present and fully settled in `demo-sa` — then adds
 * a partial refund on top of it, and reads the VAT report/detail over the range that covers both.
 *
 * Deliberately does **not** create a new FC invoice/payment in this case: doing so (tried while
 * writing this case) surfaces a real pre-existing mock bug — `checkAllocationsWithinTotal`
 * (`src/mocks/backend/invariants.ts:382`) compares a payment allocation's base-currency `amount`
 * directly against an FC invoice's FC-denominated `grandTotal` with no currency conversion, so
 * *any* FX invoice/payment (including the seed's own, already present before this case's `run()`
 * starts) fails `allocations-within-total`. Confirmed independently of this case: seeding
 * `demo-sa` alone and running `checkAllocationsWithinTotal` on the fresh snapshot already reports
 * "1 over-allocated: pay-135->inv-634 (48500 > 1000)". Reported here, not fixed (out of this
 * lane's scope) — the parity harness's `baseline` (`scripts/parity/pass.ts`) already absorbs this
 * as a pre-existing failure before any step in this case, so reusing the seed's own invoice keeps
 * this case green while still reading the VAT report over one.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as reportService from '../../../../src/modules/reports/services/reportService';

export default defineCase({
  name: 'reports/vat-multicurrency-refund',
  source: '03-domains/13-reports.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    const customer = s.baseId('cus-usd-1');
    const rows = await s.step('find-usd-invoice', () => invoiceService.getInvoices({ customerId: customer }));
    const target = rows[0];
    const invoice = await s.step('usd-invoice-detail', () => invoiceService.getInvoice(target.id));

    await s.step('refund-partial', () =>
      invoiceService.createRefund({
        invoiceId: invoice.id,
        reason: 'استرجاع جزئي',
        lines: [{ invoiceLineId: invoice.lines[0].id, qty: invoice.lines[0].qty / 2 }],
      }),
    );

    const range = { from: '2026-01-01', to: '2026-06-30' };
    await s.step('vat-report', () => reportService.getVatReport(range));
    await s.step('vat-detail', () => reportService.getVatDetail(range));
  },
});
