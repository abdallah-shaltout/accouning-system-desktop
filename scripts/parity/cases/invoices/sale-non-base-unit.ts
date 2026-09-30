/**
 * 08 §8(b) sale-non-base-unit — a POS sale in a non-base unit: Panadol sold by the box
 * (`ProductUnit` `pu-panadol-box`, factor 3) next to a strip line in its base unit. The line's
 * `unitId` is the product's own `ProductUnit.id` (what `usePosStore.toSaleLines` sends as
 * `l.unit?.id`), not a `units` row id — it must be accepted and read back verbatim on the invoice
 * and a held cart, while stock, batches (FEFO) and COGS move in base units (`qty × unitFactor`).
 * Regression for the `fk_*_unit_id` FK that refused every non-base-unit line on Rust (m0020).
 * Then the box line is refunded (one box restocked, one written off — ACC-0032: 3 strips each, at
 * 3 strips' cost) and a one-box quotation is converted (ACC-0033: the unit survives, 3 strips sold).
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as inventoryService from '../../../../src/modules/products/services/inventoryService';
import * as productService from '../../../../src/modules/products/services/productService';

export default defineCase({
  name: 'invoices/sale-non-base-unit',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: 'steps.*.value.prices', emptyArrayOnly: true, reason: '06-products Q-7: Product.prices is [] in Rust for a product that has none; the mock leaves it absent on seed rows never saved through the form ([] ≡ absent for this field)' },
  ],
  async run(s) {
    const productId = s.baseId('prd-panadol');
    await s.step('product-before', () => productService.getProduct(productId));
    await s.step('batches-before', () => inventoryService.getBatches(productId));

    // Held cart first: the unit id must survive the held-sale JSON round-trip too.
    const held = await s.step('hold', () =>
      invoiceService.holdSale({
        terminalId: 'pos-1',
        discountRate: 0,
        discountIsPct: true,
        lines: [
          { productId, unitId: 'pu-panadol-box', qty: 2, price: 24 },
          { productId, unitId: 'pu-panadol-strip', qty: 1, price: 8.5 },
        ],
      }),
    );
    await s.step('held-list', () => invoiceService.getHeldSales('pos-1'));
    await s.step('resume', () => invoiceService.resumeHeldSale(held.id));

    await s.setClock('2026-06-30T09:05:00.000Z');
    const input = {
      lines: [
        { productId, qty: 2, price: 24, unitId: 'pu-panadol-box', unitFactor: 3 },
        { productId, qty: 1, price: 8.5, unitId: 'pu-panadol-strip', unitFactor: 1 },
      ],
      discountRate: 0,
      paymentMethod: 'cash' as const,
      paidAmount: 100,
      tenderedAmount: 100,
    };
    await s.step('preview', () => invoiceService.previewSale(input));
    const sale = await s.step('create', () => invoiceService.createSale(input));
    await s.step('detail', () => invoiceService.getInvoice(sale.id));
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('invoice', sale.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    await s.step('batches-after', () => inventoryService.getBatches(productId));
    await s.step('product-after', () => productService.getProduct(productId));

    // ACC-0032: refunding 1 box restocks 3 strips at 3 × the line's cost (Dr inventory / Cr COGS),
    // and writing the other box off books 3 strips' cost to 5120 — never 1 strip per box.
    await s.setClock('2026-06-30T09:10:00.000Z');
    const refund = await s.step('refund-box', () =>
      invoiceService.createRefund({
        invoiceId: sale.id,
        reason: 'مرتجع علبة',
        lines: [{ invoiceLineId: sale.lines[0].id, qty: 1, restock: true }],
      }),
    );
    const refundRefs = await s.step('refund-journal-refs', () => accountingService.getJournalEntriesForSource('refund', refund.id));
    for (let i = 0; i < refundRefs.length; i++) await s.step(`refund-journal-${i}`, () => accountingService.getJournalEntry(refundRefs[i].id));
    await s.step('product-after-refund', () => productService.getProduct(productId));
    const writeOff = await s.step('write-off-box', () =>
      invoiceService.createRefund({
        invoiceId: sale.id,
        reason: 'علبة تالفة',
        lines: [{ invoiceLineId: sale.lines[0].id, qty: 1, restock: false }],
      }),
    );
    const writeOffRefs = await s.step('write-off-journal-refs', () => accountingService.getJournalEntriesForSource('refund', writeOff.id));
    for (let i = 0; i < writeOffRefs.length; i++) await s.step(`write-off-journal-${i}`, () => accountingService.getJournalEntry(writeOffRefs[i].id));
    await s.step('product-after-write-off', () => productService.getProduct(productId));

    // ACC-0033: a quotation keeps its line's unit, and converting it sells 1 box = 3 strips.
    await s.setClock('2026-06-30T09:15:00.000Z');
    const quotation = await s.step('quotation-save', () =>
      invoiceService.saveQuotation({ lines: [{ productId, qty: 1, price: 24, unitId: 'pu-panadol-box', unitFactor: 3 }], discountRate: 0 }),
    );
    const converted = await s.step('quotation-convert', () => invoiceService.convertQuotationToInvoice(quotation.id, { paymentMethod: 'cash', paidAmount: 24 }));
    const convertedRefs = await s.step('converted-journal-refs', () => accountingService.getJournalEntriesForSource('invoice', converted.id));
    for (let i = 0; i < convertedRefs.length; i++) await s.step(`converted-journal-${i}`, () => accountingService.getJournalEntry(convertedRefs[i].id));
    await s.step('batches-after-quotation', () => inventoryService.getBatches(productId));
    await s.step('product-after-quotation', () => productService.getProduct(productId));
  },
});
