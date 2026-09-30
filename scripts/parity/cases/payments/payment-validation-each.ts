/**
 * 09 §8(b) payment-validation-each — every §3.1/§3.2/§3.3 refusal byte for byte, in the mock's
 * order: amount ≤ 0, unknown customer / supplier (chosen by `type`, Q-P4), a document that is not
 * open for this party (checked before over-allocation), allocations over the payment amount, over a
 * base document's outstanding, a partial base-currency amount against an FC document, an FC amount
 * over the FC outstanding, "allocate later" on an unknown payment / with nothing to allocate, and
 * `getPayment` NOT_FOUND. Nothing is written by any of them.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as paymentService from '../../../../src/modules/payments/services/paymentService';

/** An unknown id in the only shape a real id has (a UUID — Rust decodes ids strictly, so a free-form
 * `'x-does-not-exist'` would test id parsing, not the lookup's §3 NOT_FOUND/VALIDATION refusal). */
const MISSING_ID = '00000000-0000-7000-8000-000000000001';

export default defineCase({
  name: 'payments/payment-validation-each',
  source: '03-domains/09-payments.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const cus = s.baseId('cus-4');
    const receipt = { date: '2026-06-30T09:00:00.000Z', type: 'RECEIVED' as const, targetType: 'customer' as const, targetId: cus, method: 'cash' as const };
    const inv = await s.step('sale', () =>
      invoiceService.createSale({
        customerId: cus,
        lines: [{ productId: s.baseId('prd-13'), qty: 1, price: 99 }],
        discountRate: 0,
        paymentMethod: 'credit',
        paidAmount: 0,
        source: 'DESK',
      }),
    );
    await s.setClock('2026-06-30T09:05:00.000Z');
    const usdInv = await s.step('sale-usd', () =>
      invoiceService.createSale({
        customerId: s.baseId('cus-usd-1'),
        lines: [{ productId: 'freetext', qty: 1, price: 10, isFreeText: true, revenueAccountId: s.baseId('acc-4110'), name: 'استشارة' }],
        discountRate: 0,
        paymentMethod: 'credit',
        paidAmount: 0,
        source: 'DESK',
        currency: 'USD',
        exchangeRate: 48.5,
      }),
    );

    await s.expectError('amount-zero', () => paymentService.createPayment({ ...receipt, amount: 0 }));
    await s.expectError('amount-rounds-to-zero', () => paymentService.createPayment({ ...receipt, amount: 0.004 }));
    await s.expectError('unknown-customer', () => paymentService.createPayment({ ...receipt, targetId: MISSING_ID, amount: 10 }));
    await s.expectError('unknown-supplier', () =>
      paymentService.createPayment({ ...receipt, type: 'PAID', targetType: 'supplier', targetId: MISSING_ID, amount: 10 }),
    );
    await s.expectError('supplier-type-with-customer-id', () => paymentService.createPayment({ ...receipt, type: 'PAID', amount: 10 }));
    await s.expectError('doc-not-open-for-party', () =>
      paymentService.createPayment({ ...receipt, amount: 50, allocations: [{ targetKind: 'invoice', targetId: usdInv.id, amount: 50 }] }),
    );
    await s.expectError('unknown-doc-before-over-allocation', () =>
      paymentService.createPayment({
        ...receipt,
        amount: 50,
        allocations: [
          { targetKind: 'invoice', targetId: MISSING_ID, amount: 10 },
          { targetKind: 'invoice', targetId: inv.id, amount: 999 },
        ],
      }),
    );
    await s.expectError('over-payment-amount', () =>
      paymentService.createPayment({ ...receipt, amount: 50, allocations: [{ targetKind: 'invoice', targetId: inv.id, amount: 60 }] }),
    );
    await s.expectError('over-outstanding', () =>
      paymentService.createPayment({ ...receipt, amount: 500, allocations: [{ targetKind: 'invoice', targetId: inv.id, amount: 100 }] }),
    );
    const usdReceipt = { ...receipt, targetId: s.baseId('cus-usd-1') };
    await s.expectError('fc-partial-base', () =>
      paymentService.createPayment({ ...usdReceipt, amount: 100, allocations: [{ targetKind: 'invoice', targetId: usdInv.id, amount: 100 }] }),
    );
    await s.expectError('fc-over-fc-outstanding', () =>
      paymentService.createPayment({
        ...usdReceipt,
        method: 'bank_transfer',
        amount: 984,
        currency: 'USD',
        amountFc: 20,
        rate: 49.2,
        allocations: [{ targetKind: 'invoice', targetId: usdInv.id, amount: 984 }],
      }),
    );
    await s.expectError('allocate-unknown-payment', () =>
      paymentService.allocateExistingPayment(MISSING_ID, [{ targetKind: 'invoice', targetId: inv.id, amount: 10 }]),
    );
    await s.setClock('2026-06-30T09:10:00.000Z');
    const open = await s.step('receipt-unallocated', () => paymentService.createPayment({ ...receipt, date: '2026-06-30T09:10:00.000Z', amount: 40 }));
    await s.expectError('allocate-nothing', () => paymentService.allocateExistingPayment(open.id, [{ targetKind: 'invoice', targetId: inv.id, amount: 0 }]));
    await s.expectError('allocate-over-remaining', () => paymentService.allocateExistingPayment(open.id, [{ targetKind: 'invoice', targetId: inv.id, amount: 41 }]));
    await s.expectError('get-unknown', () => paymentService.getPayment(MISSING_ID));
    await s.step('open-docs-unchanged', () => paymentService.getOpenDocuments('customer', cus));
  },
});
