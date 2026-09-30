/**
 * 05-parties §8(b): duplicate warnings by phone (legacy `phone` and `phones[]`) and VAT number,
 * customers listed before suppliers, phone warning before VAT warning, `excludeId` skips self.
 */
import { defineCase } from '../../case';
import * as partyService from '../../../../src/modules/parties/services/partyService';

export default defineCase({
  name: 'parties/parties-duplicates',
  source: '03-domains/05-parties.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    await s.step('phone-customer', () => partyService.checkDuplicates({ phone: '+966501112233' }));
    await s.step('vat-supplier', () => partyService.checkDuplicates({ vatNumber: '300111222300003' }));
    await s.step('exclude-self', () => partyService.checkDuplicates({ phone: '+966501112233' }, s.baseId('cus-1')));
    await s.step('none', () => partyService.checkDuplicates({ phone: '+966599999999', vatNumber: '300999999900003' }));
    await s.step('empty-input', () => partyService.checkDuplicates({}));
    // A supplier whose phones[] carries an existing customer's number and whose VAT equals that
    // customer's: the check sees the customer first (phone, then VAT), then the supplier.
    await s.step('create-supplier', () =>
      partyService.saveSupplier({
        type: 'company',
        name: 'مورد برقم مكرر',
        vatNumber: '300456789100003',
        phones: [{ id: 'phone-1', label: 'mobile', number: '+966112223344' }],
        active: true,
      }),
    );
    await s.step('phone-and-vat', () => partyService.checkDuplicates({ phone: '+966112223344', vatNumber: '300456789100003' }));
    await s.step('phone-and-vat-exclude-customer', () =>
      partyService.checkDuplicates({ phone: '+966112223344', vatNumber: '300456789100003' }, s.baseId('cus-2')),
    );
  },
});
