/**
 * 08b §8(b) shift-force-close — a manager force-closes a shift left open by a cashier: with no
 * count, counted defaults to expected (no variance entry), `forceClosedBy` is stamped and the note
 * is the fixed Arabic text; a second shift force-closed with a count posts its variance; a
 * force-close never drops; closed and unknown shifts are refused.
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';

/** An unknown id in the only shape a real id has (a UUID — Rust decodes ids strictly, so a free-form
 * `'x-does-not-exist'` would test id parsing, not the lookup's §3 NOT_FOUND/VALIDATION refusal). */
const MISSING_ID = '00000000-0000-7000-8000-000000000001';

export default defineCase({
  name: 'shifts/shift-force-close',
  source: '03-domains/08b-pos-shifts.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    await s.login('cashier');
    const first = await s.step('cashier-open', () => invoiceService.openPosShift({ terminalId: 'pos-1', openingFloat: 200 }));
    await s.setClock('2026-06-30T09:05:00.000Z');
    await s.step('cashier-sale', () =>
      invoiceService.createSale({
        lines: [{ productId: s.baseId('prd-18'), qty: 2, price: 39 }],
        discountRate: 0,
        paymentMethod: 'cash',
        paidAmount: 78,
        shiftId: first.id,
      }),
    );

    await s.login('manager');
    await s.setClock('2026-06-30T18:00:00.000Z');
    await s.step('force-close-default', () => invoiceService.forceClosePosShift(first.id));
    await s.step('first-entries', () => accountingService.getJournalEntriesForSource('shift', first.id));
    await s.step('first-closed', () => invoiceService.getShift(first.id));
    await s.expectError('force-close-closed', () => invoiceService.forceClosePosShift(first.id));
    await s.expectError('force-close-unknown', () => invoiceService.forceClosePosShift(MISSING_ID));

    await s.setClock('2026-06-30T18:05:00.000Z');
    const second = await s.step('manager-open', () => invoiceService.openPosShift({ terminalId: 'pos-1', openingFloat: 150 }));
    await s.setClock('2026-06-30T18:30:00.000Z');
    await s.step('force-close-counted', () => invoiceService.forceClosePosShift(second.id, 140));
    const refs = await s.step('second-refs', () => accountingService.getJournalEntriesForSource('shift', second.id));
    for (let i = 0; i < refs.length; i++) await s.step(`second-journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    await s.step('second-closed', () => invoiceService.getShift(second.id));
  },
});
