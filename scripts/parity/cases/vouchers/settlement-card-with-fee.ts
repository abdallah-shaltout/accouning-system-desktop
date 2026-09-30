/**
 * 10 §8(b) settlement-card-with-fee — the unsettled mada tender groups (by day × method), the fee
 * estimate from the method's fee %, then a settlement of two days whose bank deposit is less than
 * the gross: Dr bank (deposit) + Dr card fees (gross − deposit) / Cr card clearing (gross). The
 * settled groups leave the unsettled list; the settlement reads back and appears in the list.
 */
import { defineCase } from '../../case';
import * as voucherService from '../../../../src/modules/vouchers/services/voucherService';
import * as accountingService from '../../../../src/modules/accounting/services/accountingService';
import { round2 } from '../../../../src/modules/core/helpers/numbers';

export default defineCase({
  name: 'vouchers/settlement-card-with-fee',
  source: '03-domains/10-vouchers.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const mada = s.baseId('pm-mada');
    const groups = await s.step('unsettled-before', () => voucherService.getUnsettledTenderGroups());
    const picked = groups.filter((g) => g.paymentMethodId === mada && (g.date === '2026-06-28' || g.date === '2026-06-27'));
    await s.step('estimate', () => voucherService.estimateSettlementFee(picked));
    const gross = round2(picked.reduce((a, g) => a + g.total, 0));
    const stl = await s.step('settle', () =>
      voucherService.createCardSettlement({
        date: '2026-06-30T09:00:00.000Z',
        groups: picked.map((g) => ({ date: g.date, paymentMethodId: g.paymentMethodId })),
        depositAmount: round2(gross - 16.53),
        note: 'إيداع مدى',
      }),
    );
    await s.step('get', () => voucherService.getCardSettlement(stl.id));
    const refs = await s.step('journal-refs', () => accountingService.getJournalEntriesForSource('settlement', stl.id));
    for (let i = 0; i < refs.length; i++) await s.step(`journal-${i}`, () => accountingService.getJournalEntry(refs[i].id));
    await s.step('unsettled-after', () => voucherService.getUnsettledTenderGroups());
    await s.step('list', () => voucherService.getCardSettlements());
  },
});
