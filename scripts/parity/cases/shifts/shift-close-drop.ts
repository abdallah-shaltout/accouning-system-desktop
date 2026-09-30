/**
 * 08b §8(b) shift-close-drop — closing with the DROP handover and a counted 500.00 that is also
 * 2.00 over: the variance entry posts first, then a TRANSFER voucher (Dr bank / Cr cash 500.00)
 * with its own number, journal entry and activity row (D-S2: identical numbers on both sides).
 */
import { defineCase } from '../../case';
import * as invoiceService from '../../../../src/modules/invoices/services/invoiceService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import * as voucherService from '../../../../src/modules/vouchers/services/voucherService';

export default defineCase({
  name: 'shifts/shift-close-drop',
  source: '03-domains/08b-pos-shifts.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const shift = await s.step('open', () => invoiceService.openPosShift({ terminalId: 'pos-1', openingFloat: 498 }));
    await s.setClock('2026-06-30T17:00:00.000Z');
    await s.step('close', () => invoiceService.closePosShift(shift.id, { countedCash: 500, handoverMode: 'DROP', note: 'إيداع نهاية اليوم' }));
    const refs = await s.step('variance-refs', () => accountingService.getJournalEntriesForSource('shift', shift.id));
    for (let i = 0; i < refs.length; i++) await s.step(`variance-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    const drops = await s.step('drop-vouchers', () => voucherService.getVouchers({ kind: 'TRANSFER', from: '2026-06-30', to: '2026-06-30' }));
    for (let i = 0; i < drops.length; i++) {
      const vRefs = await s.step(`drop-${i}-refs`, () => accountingService.getJournalEntriesForSource('voucher', drops[i].id));
      for (let j = 0; j < vRefs.length; j++) await s.step(`drop-${i}-journal-${j}`, () => accountingService.getJournalEntry(vRefs[j].id));
    }
    await s.step('get-closed', () => invoiceService.getShift(shift.id));
  },
});
