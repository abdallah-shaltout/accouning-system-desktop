/**
 * 08 §8(b) sale-validation-each — every §3.2 `prepare_sale` refusal, byte for byte and in the
 * mock's order (a qty ≤ 0 on line 1 beats a missing product on line 2; a missing product beats the
 * stock check; tenders are resolved before the paid-amount checks).
 * Ends with a valid sale so the books and its number show nothing leaked.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as productService from '../../../../src/modules/products/services/productService';

/** An unknown id in the only shape a real id has (a UUID — Rust decodes ids strictly, so a free-form
 * `'x-does-not-exist'` would test id parsing, not the lookup's §3 NOT_FOUND/VALIDATION refusal). */
const MISSING_ID = '00000000-0000-7000-8000-000000000001';

export default defineCase({
  name: 'invoices/sale-validation-each',
  source: '03-domains/08-invoices.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const prd = s.baseId('prd-1');
    const ok = { discountRate: 0, paymentMethod: 'cash' as const, paidAmount: 129 };
    const line = { productId: prd, qty: 1, price: 129 };

    await s.expectError('empty-cart', () => invoiceService.createSale({ ...ok, lines: [] }));
    // admin's maxDiscount is 100: 101 % trips the per-user limit before the 0..100 range check.
    await s.expectError('discount-over-user-max', () => invoiceService.createSale({ ...ok, lines: [line], discountRate: 101 }));
    await s.expectError('discount-negative', () => invoiceService.createSale({ ...ok, lines: [line], discountRate: -5 }));
    await s.expectError('qty-zero', () => invoiceService.createSale({ ...ok, lines: [{ ...line, qty: 0 }] }));
    await s.expectError('qty-first-beats-missing-product', () =>
      invoiceService.createSale({ ...ok, lines: [{ ...line, qty: -1 }, { productId: MISSING_ID, qty: 1, price: 10 }] }),
    );
    await s.expectError('price-negative', () => invoiceService.createSale({ ...ok, lines: [{ ...line, price: -1 }] }));
    await s.expectError('free-text-no-account', () =>
      invoiceService.createSale({ ...ok, lines: [{ productId: 'freetext', qty: 1, price: 50, isFreeText: true, name: 'خدمة' }] }),
    );
    await s.expectError('unknown-product', () =>
      invoiceService.createSale({ ...ok, lines: [line, { productId: MISSING_ID, qty: 1, price: 10 }] }),
    );
    await s.expectError('missing-product-beats-stock', () =>
      invoiceService.createSale({ ...ok, lines: [{ ...line, qty: 100000 }, { productId: MISSING_ID, qty: 1, price: 10 }] }),
    );

    // Inactive product: deactivate one catalog item through its own service (cross-domain, allowed).
    const belt = await s.step('belt', () => productService.getProduct(s.baseId('prd-23')));
    const { id: beltId, stockQty: _q, stockValue: _v, ...beltInput } = belt;
    await s.step('deactivate-belt', () => productService.updateProduct(beltId, { ...beltInput, active: false }));
    await s.expectError('inactive-product', () => invoiceService.createSale({ ...ok, lines: [{ productId: beltId, qty: 1, price: 79 }] }));

    await s.expectError('insufficient-stock', () => invoiceService.createSale({ ...ok, lines: [{ ...line, qty: 100000 }] }));
    await s.expectError('unknown-tender-method', () =>
      invoiceService.createSale({ ...ok, lines: [line], tenders: [{ paymentMethodId: MISSING_ID, amount: 129 }] }),
    );
    await s.expectError('paid-negative', () =>
      invoiceService.createSale({ ...ok, lines: [line], tenders: [{ paymentMethodId: s.baseId('pm-cash'), amount: -10 }] }),
    );
    await s.expectError('paid-over-total', () =>
      invoiceService.createSale({
        ...ok,
        lines: [line],
        tenders: [
          { paymentMethodId: s.baseId('pm-cash'), amount: 100 },
          { paymentMethodId: s.baseId('pm-mada'), amount: 100 },
        ],
      }),
    );
    await s.expectError('partial-needs-customer', () => invoiceService.createSale({ ...ok, lines: [line], paymentMethod: 'credit', paidAmount: 0 }));
    await s.expectError('inactive-customer', () =>
      invoiceService.createSale({ ...ok, lines: [line], paymentMethod: 'credit', paidAmount: 0, customerId: s.baseId('cus-9') }),
    );

    // A valid sale after all the refusals: its number proves no refusal consumed a counter.
    // (The §3.4 step 6 period-lock refusal is not exercised here: on the mock, `recordSale` writes the
    // invoice, counter and stock before `postJournal` throws — reported to the manager as a mock bug —
    // and any lock date covering today also trips the `lock-date` invariant on the seeded history.)
    await s.step('valid-sale-after', () => invoiceService.createSale({ ...ok, lines: [line] }));
    await s.step('product-after', () => productService.getProduct(prd));
  },
});
