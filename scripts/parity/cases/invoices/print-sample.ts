/**
 * 08 §8(b) print-sample — print data on demo-eg: the settings "test print" sample uses the store's
 * own default sales-tax rate (14 % in Egypt, never a hard-coded 15 %), a real invoice's print data
 * carries the invoice, customer, cashier name and settings, and an unknown id is NOT_FOUND.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';

/** An unknown id in the only shape a real id has (a UUID — Rust decodes ids strictly, so a free-form
 * `'x-does-not-exist'` would test id parsing, not the lookup's §3 NOT_FOUND/VALIDATION refusal). */
const MISSING_ID = '00000000-0000-7000-8000-000000000001';

export default defineCase({
  name: 'invoices/print-sample',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-eg',
  allow: [
    {
      path: 'steps.sample.value.sample',
      reason:
        "08-invoices D-I11: the unsaved test-print sample's invoice.id is a fresh UUID on Rust vs the mock's literal 'sample'; the pair makes the identical PrintData.sample key read as a mapped id used literally",
    },
  ],
  async run(s) {
    await s.step('sample', () => invoiceService.getInvoicePrintData('sample'));
    const sale = await s.step('sale', () =>
      invoiceService.createSale({
        customerId: s.baseId('cus-1'),
        lines: [{ productId: s.baseId('prd-12'), qty: 1, price: 229 }],
        discountRate: 0,
        paymentMethod: 'cash',
        paidAmount: 229,
        tenderedAmount: 250,
      }),
    );
    await s.step('real', () => invoiceService.getInvoicePrintData(sale.id));
    await s.expectError('unknown', () => invoiceService.getInvoicePrintData(MISSING_ID));
  },
});
