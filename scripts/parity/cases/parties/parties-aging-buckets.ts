/**
 * 05-parties §8(b): aging buckets by dueDate else date against the business "today"; the clock is
 * moved forward so the seed's open documents walk through every bucket (Q-4: `90+` starts at day 61),
 * bucket totals round2'd stepwise.
 */
import { defineCase } from '../../case';
import * as partyService from '../../../../src/modules/parties/services/partyService';

export default defineCase({
  name: 'parties/parties-aging-buckets',
  source: '03-domains/05-parties.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const cus7 = s.baseId('cus-7');
    const cus4 = s.baseId('cus-4');
    await s.step('cus-7-today', () => partyService.getPartyAging('customer', cus7));
    await s.step('cus-4-today', () => partyService.getPartyAging('customer', cus4));
    await s.step('sup-1-today', () => partyService.getPartyAging('supplier', s.baseId('sup-1')));
    await s.setClock('2026-07-25T09:00:00.000Z');
    await s.step('cus-7-plus-25', () => partyService.getPartyAging('customer', cus7));
    await s.setClock('2026-08-15T09:00:00.000Z');
    await s.step('cus-7-plus-46', () => partyService.getPartyAging('customer', cus7));
    await s.setClock('2026-09-10T09:00:00.000Z');
    await s.step('cus-7-plus-72', () => partyService.getPartyAging('customer', cus7));
    await s.step('cus-4-plus-72', () => partyService.getPartyAging('customer', cus4));
    await s.step('sup-2-plus-72', () => partyService.getPartyAging('supplier', s.baseId('sup-2')));
    await s.step('none-open', () => partyService.getPartyAging('customer', s.baseId('cus-3')));
  },
});
