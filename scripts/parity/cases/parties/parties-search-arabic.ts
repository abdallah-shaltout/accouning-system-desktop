/**
 * 05-parties §8(b): list filters — Arabic-normalized search (`احمد` finds `أحمد`, Arabic-Indic digits
 * find a phone), groupId, withBalanceOnly, overLimitOnly, includeInactive, and the supplier haystack.
 */
import { defineCase } from '../../case';
import * as partyService from '../../../../src/modules/parties/services/partyService';

export default defineCase({
  name: 'parties/parties-search-arabic',
  source: '03-domains/05-parties.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    await s.step('hamza', () => partyService.getCustomers({ search: 'أحمد' }));
    await s.step('no-hamza', () => partyService.getCustomers({ search: 'احمد' }));
    await s.step('indic-digits', () => partyService.getCustomers({ search: '٥٠١١١٢' }));
    await s.step('latin-digits', () => partyService.getCustomers({ search: '501112' }));
    await s.step('code', () => partyService.getCustomers({ search: 'c-0005' }));
    await s.step('english', () => partyService.getCustomers({ search: 'global' }));
    await s.step('inactive-hidden', () => partyService.getCustomers({ search: 'لطيفة' }));
    await s.step('inactive-shown', () => partyService.getCustomers({ search: 'لطيفة', includeInactive: true }));
    await s.step('group', () => partyService.getCustomers({ groupId: s.baseId('pg-corporate') }));
    await s.step('with-balance', () => partyService.getCustomers({ withBalanceOnly: true }));
    await s.step('over-limit', () => partyService.getCustomers({ overLimitOnly: true }));
    await s.step('whitespace-search', () => partyService.getCustomers({ search: '   ' }));
    await s.step('groups-customer', () => partyService.getPartyGroups('customer'));
    await s.step('groups-supplier', () => partyService.getPartyGroups('supplier'));
    await s.step('suppliers-taa-marbuta', () => partyService.getSuppliers({ search: 'مؤسسه' }));
    await s.step('suppliers-with-balance', () => partyService.getSuppliers({ withBalanceOnly: true }));
    await s.step('suppliers-vat', () => partyService.getSuppliers({ search: '300555666700003' }));
  },
});
