/**
 * 05-parties §8(b): update is Object.assign — absent keys keep their stored value (Q-2), cleaned
 * strings `''` clear (`nameEn`), `phones` are replaced in order, `updatedAt` appears, history grows.
 */
import { defineCase } from '../../case';
import * as partyService from '../../../../src/modules/parties/services/partyService';

export default defineCase({
  name: 'parties/parties-update-absent-keys',
  source: '03-domains/05-parties.md §8(b)',
  base: 'demo-sa',
  async run(s) {
    const id = s.baseId('cus-3');
    const before = await s.step('before', () => partyService.getCustomer(id));
    // First give it an English name, phones, notes and a credit limit (round2'd, D-4).
    await s.step('update-1', () =>
      partyService.saveCustomer(
        {
          type: before.type,
          name: before.name,
          nameEn: 'Reem',
          phone: before.phone,
          phones: [
            { id: 'phone-1', label: 'mobile', number: '+966553334455' },
            { id: 'phone-2', label: 'whatsapp', number: '+966553334400' },
          ],
          creditLimit: 2500.5,
          notes: '  عميلة مميزة  ',
          active: true,
        },
        id,
      ),
    );
    // Second update: no groupId / creditLimit keys at all (kept, Q-2), nameEn '' (cleared), phones replaced.
    await s.step('update-2', () =>
      partyService.saveCustomer(
        {
          type: before.type,
          name: 'ريم الحربي',
          nameEn: '',
          phones: [{ id: 'phone-9', label: 'work', number: '+966112220000' }],
          active: true,
        },
        id,
      ),
    );
    await s.step('after', () => partyService.getCustomer(id));
    await s.step('history', () => partyService.getPartyHistory(id));

    const supId = s.baseId('sup-5');
    const sup = await s.step('supplier-before', () => partyService.getSupplier(supId));
    await s.step('supplier-update', () =>
      partyService.saveSupplier({ type: sup.type, name: sup.name, contactPerson: '  منى  ', nameEn: ' Happy Kids ', active: true }, supId),
    );
    await s.step('supplier-after', () => partyService.getSupplier(supId));
    await s.step('supplier-history', () => partyService.getPartyHistory(supId));
    await s.expectError('update-customer-by-supplier-id', () => partyService.saveCustomer({ type: 'individual', name: 'س', active: true }, s.baseId('sup-1')));
    await s.expectError('update-supplier-by-customer-id', () => partyService.saveSupplier({ type: 'company', name: 'س', active: true }, s.baseId('cus-1')));
  },
});
