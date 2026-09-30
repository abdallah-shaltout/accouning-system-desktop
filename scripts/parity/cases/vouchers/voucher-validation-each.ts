/**
 * 10 §8(b) voucher-validation-each — every §3 refusal byte for byte: amount ≤ 0, unknown method,
 * unknown account, a non-manual account (both wordings: "إلى" on a receipt, "من" on a payment),
 * the same account on both sides of a transfer (checked before any lookup), a fee without a fee
 * account, an unknown transfer / owner cash account, `getVoucher` NOT_FOUND, and — last, after
 * deactivating the drawings account through its own service — the missing drawings-role message.
 */
import { defineCase } from '../../case';
import * as voucherService from '../../../../src/modules/vouchers/services/voucherService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

/** An unknown id in the only shape a real id has (a UUID — Rust decodes ids strictly, so a free-form
 * `'x-does-not-exist'` would test id parsing, not the lookup's §3 NOT_FOUND/VALIDATION refusal). */
const MISSING_ID = '00000000-0000-7000-8000-000000000001';

export default defineCase({
  name: 'vouchers/voucher-validation-each',
  source: '03-domains/10-vouchers.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const date = '2026-06-30T09:00:00.000Z';
    const cash = s.baseId('pm-cash');
    const receipt = { date, amount: 100, description: 'اختبار', paymentMethodId: cash, creditAccountId: s.baseId('acc-4300') };
    const payment = { date, amount: 100, description: 'اختبار', paymentMethodId: cash, debitAccountId: s.baseId('acc-6280') };
    const transfer = { date, amount: 100, description: 'اختبار', sourceAccountId: s.baseId('acc-1110'), destinationAccountId: s.baseId('acc-1120') };
    const owner = { date, amount: 100, description: 'اختبار', direction: 'drawings' as const, cashAccountId: s.baseId('acc-1110') };

    await s.expectError('receipt-amount-zero', () => voucherService.createReceiptVoucher({ ...receipt, amount: 0 }));
    await s.expectError('receipt-unknown-method', () => voucherService.createReceiptVoucher({ ...receipt, paymentMethodId: MISSING_ID }));
    await s.expectError('receipt-unknown-account', () => voucherService.createReceiptVoucher({ ...receipt, creditAccountId: MISSING_ID }));
    await s.expectError('receipt-non-manual', () => voucherService.createReceiptVoucher({ ...receipt, creditAccountId: s.baseId('acc-2150') }));
    await s.expectError('payment-amount-negative', () => voucherService.createPaymentVoucher({ ...payment, amount: -5 }));
    await s.expectError('payment-unknown-method', () => voucherService.createPaymentVoucher({ ...payment, paymentMethodId: MISSING_ID }));
    await s.expectError('payment-non-manual', () => voucherService.createPaymentVoucher({ ...payment, debitAccountId: s.baseId('acc-5100') }));
    await s.expectError('transfer-amount-zero', () => voucherService.createTransferVoucher({ ...transfer, amount: 0 }));
    await s.expectError('transfer-same-accounts', () =>
      voucherService.createTransferVoucher({ ...transfer, sourceAccountId: MISSING_ID, destinationAccountId: MISSING_ID }),
    );
    await s.expectError('transfer-unknown-source', () => voucherService.createTransferVoucher({ ...transfer, sourceAccountId: MISSING_ID }));
    await s.expectError('transfer-unknown-destination', () => voucherService.createTransferVoucher({ ...transfer, destinationAccountId: MISSING_ID }));
    await s.expectError('transfer-fee-no-account', () => voucherService.createTransferVoucher({ ...transfer, feeAmount: 3 }));
    await s.expectError('owner-amount-zero', () => voucherService.createOwnerVoucher({ ...owner, amount: 0 }));
    await s.expectError('owner-unknown-cash-account', () => voucherService.createOwnerVoucher({ ...owner, cashAccountId: MISSING_ID }));
    await s.expectError('get-unknown', () => voucherService.getVoucher(MISSING_ID));
    await s.step('list-unchanged', () => voucherService.getVouchers());

    // Missing drawings role: deactivate the only drawings account (accounting's own service).
    const accounts = await s.step('accounts', () => accountingService.getAccounts());
    const drawings = accounts.find((a) => a.id === s.baseId('acc-3400'))!;
    await s.step('deactivate-drawings', () =>
      accountingService.saveAccount(
        {
          code: drawings.code,
          name: drawings.name,
          nameEn: drawings.nameEn,
          parentId: drawings.parentId ?? undefined,
          isGroup: drawings.isGroup,
          kind: drawings.kind,
          subtype: drawings.subtype,
          normalSide: drawings.normalSide,
          requiresParty: drawings.requiresParty,
          allowManual: drawings.allowManual,
          active: false,
        },
        drawings.id,
      ),
    );
    await s.expectError('owner-no-drawings-role', () => voucherService.createOwnerVoucher(owner));
  },
});
