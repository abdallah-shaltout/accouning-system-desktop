/**
 * 08b §8(b) shift-open-close-exact — a negative opening float is refused; open (SH-000005, float
 * rounded), the current shift for the terminal, a second open on the same terminal is a CONFLICT;
 * one cash sale; close counted = expected → no variance entry, the shift reads CLOSED with its
 * summary; a second close is refused; unknown shift ids are NOT_FOUND; the CLOSED list.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

/** An unknown id in the only shape a real id has (a UUID — Rust decodes ids strictly, so a free-form
 * `'x-does-not-exist'` would test id parsing, not the lookup's §3 NOT_FOUND/VALIDATION refusal). */
const MISSING_ID = '00000000-0000-7000-8000-000000000001';

export default defineCase({
  name: 'shifts/shift-open-close-exact',
  source: '03-domains/08b-pos-shifts.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    await s.step('current-none', () => invoiceService.getCurrentShift('pos-1'));
    await s.expectError('negative-float', () => invoiceService.openPosShift({ terminalId: 'pos-1', openingFloat: -1 }));
    const shift = await s.step('open', () => invoiceService.openPosShift({ terminalId: 'pos-1', openingFloat: 250.456 }));
    await s.step('current', () => invoiceService.getCurrentShift('pos-1'));
    await s.expectError('open-twice', () => invoiceService.openPosShift({ terminalId: 'pos-1', openingFloat: 100 }));

    await s.setClock('2026-06-30T09:05:00.000Z');
    await s.step('cash-sale', () =>
      invoiceService.createSale({
        lines: [{ productId: s.baseId('prd-1'), qty: 1, price: 129 }],
        discountRate: 0,
        paymentMethod: 'cash',
        paidAmount: 129,
        tenderedAmount: 200,
        shiftId: shift.id,
      }),
    );
    const x = await s.step('x-report', () => invoiceService.getXReport(shift.id));

    await s.setClock('2026-06-30T17:00:00.000Z');
    await s.step('close', () => invoiceService.closePosShift(shift.id, { countedCash: x.expectedCash, note: 'مطابق' }));
    await s.step('variance-entries', () => accountingService.getJournalEntriesForSource('shift', shift.id));
    await s.step('get-closed', () => invoiceService.getShift(shift.id));
    await s.step('current-after-close', () => invoiceService.getCurrentShift('pos-1'));
    await s.expectError('close-twice', () => invoiceService.closePosShift(shift.id, { countedCash: x.expectedCash }));
    await s.expectError('close-unknown', () => invoiceService.closePosShift(MISSING_ID, { countedCash: 0 }));
    await s.expectError('get-unknown', () => invoiceService.getShift(MISSING_ID));
    await s.step('list-closed', () => invoiceService.getShifts({ status: 'CLOSED' }));
  },
});
