/**
 * 08b §8(b) shift-cash-in-out — pay-in / pay-out / bank-drop drawer movements: refused with no open
 * shift (CONFLICT) and for a zero amount; with a shift open, each movement lands on the log and in
 * the X-report's expected cash (no journal entry, Q-S1/Q-S2); closing on the expected amount.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';

export default defineCase({
  name: 'shifts/shift-cash-in-out',
  source: '03-domains/08b-pos-shifts.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    await s.expectError('no-open-shift', () => invoiceService.recordCashInOut('pos-1', 'PAY_IN', 100));
    const shift = await s.step('open', () => invoiceService.openPosShift({ terminalId: 'pos-1', openingFloat: 300 }));
    await s.expectError('zero-amount', () => invoiceService.recordCashInOut('pos-1', 'PAY_OUT', 0));
    await s.setClock('2026-06-30T10:00:00.000Z');
    await s.step('pay-in', () => invoiceService.recordCashInOut('pos-1', 'PAY_IN', 200, 'فكة من الخزينة'));
    await s.setClock('2026-06-30T11:00:00.000Z');
    await s.step('pay-out', () => invoiceService.recordCashInOut('pos-1', 'PAY_OUT', 45.5, 'مياه وضيافة'));
    await s.setClock('2026-06-30T12:00:00.000Z');
    await s.step('bank-drop', () => invoiceService.recordCashInOut('pos-1', 'BANK_DROP', 100));
    const x = await s.step('x-report', () => invoiceService.getXReport(shift.id));
    await s.setClock('2026-06-30T17:00:00.000Z');
    await s.step('close', () => invoiceService.closePosShift(shift.id, { countedCash: x.expectedCash }));
    await s.step('get-closed', () => invoiceService.getShift(shift.id));
  },
});
