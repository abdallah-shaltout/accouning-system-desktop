/**
 * 05-parties §8(b): link sets both sides; linked net balance; unlink from the supplier side clears
 * both; unlinking an unlinked party is a silent no-op; link with a missing party → الطرف غير موجود.
 */
import { defineCase } from '../../case';
import * as partyService from '../../../../src/modules/parties/services/partyService';

export default defineCase({
  name: 'parties/parties-link-unlink',
  source: '03-domains/05-parties.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const cus = s.baseId('cus-4');
    const sup = s.baseId('sup-4');
    // A customer id where a supplier is expected (and vice versa) is "missing" — kinds must match.
    await s.expectError('link-missing-supplier', () => partyService.linkPartyRecords(cus, s.baseId('cus-5')));
    await s.expectError('link-missing-customer', () => partyService.linkPartyRecords(s.baseId('sup-3'), sup));
    await s.step('link', () => partyService.linkPartyRecords(cus, sup));
    await s.step('customer-linked', () => partyService.getCustomer(cus));
    await s.step('supplier-linked', () => partyService.getSupplier(sup));
    await s.step('net-balance', () => partyService.getLinkedNetBalance(cus, sup));
    await s.step('unlink-from-supplier', () => partyService.unlinkPartyRecord(sup, 'supplier'));
    await s.step('customer-unlinked', () => partyService.getCustomer(cus));
    await s.step('supplier-unlinked', () => partyService.getSupplier(sup));
    await s.step('unlink-again-noop', () => partyService.unlinkPartyRecord(cus, 'customer'));
    await s.step('relink', () => partyService.linkPartyRecords(cus, sup));
    await s.step('unlink-from-customer', () => partyService.unlinkPartyRecord(cus, 'customer'));
    await s.step('supplier-final', () => partyService.getSupplier(sup));
  },
});
