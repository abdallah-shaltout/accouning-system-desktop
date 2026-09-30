/**
 * 10 §8(b) settlement-already-settled-conflict — settling a group that is already settled is a
 * CONFLICT (before and after a successful settlement of it); the §3.6 refusals (no group, missing /
 * negative deposit, deposit over gross by 1.00); a deposit over gross by 0.003 is still accepted
 * (fee rounds to 0); an unknown settlement id is NOT_FOUND.
 */
import { defineCase } from '../../case';
import * as voucherService from '../../../../src/modules/vouchers/services/voucherService';
import { round2 } from '../../../../src/modules/core/helpers/numbers';

/** An unknown id in the only shape a real id has (a UUID — Rust decodes ids strictly, so a free-form
 * `'x-does-not-exist'` would test id parsing, not the lookup's §3 NOT_FOUND/VALIDATION refusal). */
const MISSING_ID = '00000000-0000-7000-8000-000000000001';

export default defineCase({
  name: 'vouchers/settlement-already-settled-conflict',
  source: '03-domains/10-vouchers.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const mada = s.baseId('pm-mada');
    const date = '2026-06-30T09:00:00.000Z';
    const groups = await s.step('unsettled', () => voucherService.getUnsettledTenderGroups());
    const g = groups.find((x) => x.paymentMethodId === mada && x.date === '2026-06-29')!;
    const key = [{ date: g.date, paymentMethodId: g.paymentMethodId }];

    await s.expectError('no-groups', () => voucherService.createCardSettlement({ date, groups: [], depositAmount: 10 }));
    await s.expectError('negative-deposit', () => voucherService.createCardSettlement({ date, groups: key, depositAmount: -1 }));
    await s.expectError('missing-deposit', () => voucherService.createCardSettlement({ date, groups: key, depositAmount: Number.NaN }));
    await s.expectError('unknown-group', () =>
      voucherService.createCardSettlement({ date, groups: [{ date: '2020-01-01', paymentMethodId: mada }], depositAmount: 10 }),
    );
    await s.expectError('deposit-over-gross', () => voucherService.createCardSettlement({ date, groups: key, depositAmount: round2(g.total + 1) }));

    const stl = await s.step('settle-over-by-0.003', () => voucherService.createCardSettlement({ date, groups: key, depositAmount: g.total + 0.003 }));
    await s.step('get', () => voucherService.getCardSettlement(stl.id));
    await s.expectError('settle-again', () => voucherService.createCardSettlement({ date, groups: key, depositAmount: g.total }));
    await s.expectError('get-unknown', () => voucherService.getCardSettlement(MISSING_ID));
    await s.step('unsettled-after', () => voucherService.getUnsettledTenderGroups());
  },
});
