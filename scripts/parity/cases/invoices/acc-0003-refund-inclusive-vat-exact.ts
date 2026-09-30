/**
 * L3-0 / ACC-0003 (G-25, P4-7): a tax-inclusive refund returns exactly the proportional VAT the
 * sale booked — never re-taxes an inclusive price. A tax-inclusive cash sale (SA 15 %) with a line
 * discount and an invoice discount (order line → invoice → VAT, docs/v2/02-accounting-review.md),
 * refunded partly and then finally; the final refund ends on the exact remainders, so Σ refunded
 * net / VAT / gross = the invoice's net / VAT / gross to the cent. A one-unit 115.00 sale refunds
 * 115.00, not 130.00 (the issue's own example). `check-*` steps fail the case on any cent of drift.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import { round2 } from '../../../../src/modules/core/helpers/numbers';

interface Detail {
  grandTotal: number;
  taxAmount: number;
  refundedAmount: number;
  status: string;
  lines: { net?: number; vat?: number }[];
  refunds: { subTotal: number; taxAmount: number; grandTotal: number }[];
}

function exactness(d: Detail) {
  const sum = (xs: number[]) => round2(xs.reduce((a, x) => a + x, 0));
  const refunded = {
    net: sum(d.refunds.map((r) => r.subTotal)),
    vat: sum(d.refunds.map((r) => r.taxAmount)),
    gross: sum(d.refunds.map((r) => r.grandTotal)),
  };
  const invoice = { net: sum(d.lines.map((l) => l.net ?? 0)), vat: round2(d.taxAmount), gross: round2(d.grandTotal) };
  const drift = [
    refunded.net !== invoice.net && `net ${refunded.net} ≠ ${invoice.net}`,
    refunded.vat !== invoice.vat && `VAT ${refunded.vat} ≠ ${invoice.vat}`,
    refunded.gross !== invoice.gross && `gross ${refunded.gross} ≠ ${invoice.gross}`,
    round2(d.refundedAmount) !== invoice.gross && `refundedAmount ${d.refundedAmount} ≠ ${invoice.gross}`,
    d.status !== 'REFUNDED' && `status ${d.status}`,
  ].filter(Boolean);
  if (drift.length) throw new Error(`ACC-0003 regression: ${drift.join('; ')}`);
  return { refunded, invoice };
}

export default defineCase({
  name: 'invoices/acc-0003-refund-inclusive-vat-exact',
  source: 'docs/diagnostics/issues/ACC-0003-refund-over-refunds-vat.md + phase-b2-parity-cases.md L3-0',
  base: 'demo-sa',
  async run(s) {
    const sale = await s.step('sale', () =>
      invoiceService.createSale({
        lines: [
          { productId: s.baseId('prd-1'), qty: 3, price: 129, discount: 10 },
          { productId: s.baseId('prd-2'), qty: 2, price: 179 },
          { productId: s.baseId('prd-14'), qty: 7, price: 39, discount: 10, discountIsPct: true },
        ],
        discountRate: 5,
        paymentMethod: 'cash',
        paidAmount: 100000,
        tenderedAmount: 1000,
      }),
    );
    const [l1, l2, l3] = sale.lines;

    await s.setClock('2026-06-30T09:05:00.000Z');
    const partial = await s.step('refund-partial', () =>
      invoiceService.createRefund({
        invoiceId: sale.id,
        lines: [
          { invoiceLineId: l1.id, qty: 1 },
          { invoiceLineId: l2.id, qty: 1 },
          { invoiceLineId: l3.id, qty: 3 },
        ],
      }),
    );
    await s.setClock('2026-06-30T09:10:00.000Z');
    const final = await s.step('refund-final', () =>
      invoiceService.createRefund({
        invoiceId: sale.id,
        lines: [
          { invoiceLineId: l1.id, qty: 2 },
          { invoiceLineId: l2.id, qty: 1 },
          { invoiceLineId: l3.id, qty: 4 },
        ],
      }),
    );
    const detail = await s.step('detail', () => invoiceService.getInvoice(sale.id));
    await s.step('check-exact', () => exactness(detail as unknown as Detail));
    for (const [tag, id] of [['partial', partial.id], ['final', final.id]] as const) {
      const refs = await s.step(`journal-refs-${tag}`, () => accountingService.getJournalEntriesForSource('refund', id));
      for (let i = 0; i < refs.length; i++) await s.step(`journal-${tag}-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    }

    // The issue's canonical example: 1 × 115.00 inclusive at 15 % refunds 115.00 (net 100, VAT 15).
    await s.setClock('2026-06-30T09:15:00.000Z');
    const one = await s.step('sale-115', () =>
      invoiceService.createSale({ lines: [{ productId: s.baseId('prd-3'), qty: 1, price: 115 }], discountRate: 0, paymentMethod: 'cash', paidAmount: 115 }),
    );
    await s.setClock('2026-06-30T09:20:00.000Z');
    const r115 = await s.step('refund-115', () => invoiceService.createRefund({ invoiceId: one.id, lines: [{ invoiceLineId: one.lines[0].id, qty: 1 }] }));
    await s.step('check-115', () => {
      if (r115.grandTotal !== 115 || r115.subTotal !== 100 || r115.taxAmount !== 15) {
        throw new Error(`ACC-0003 regression: 1 × 115 refunded ${r115.subTotal} + ${r115.taxAmount} = ${r115.grandTotal}, expected 100 + 15 = 115`);
      }
      return { subTotal: r115.subTotal, taxAmount: r115.taxAmount, grandTotal: r115.grandTotal };
    });
    await s.step('detail-115', () => invoiceService.getInvoice(one.id));
  },
});
