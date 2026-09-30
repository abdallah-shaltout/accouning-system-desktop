/**
 * 05-parties §8(b): a party with a positive balance can't be deactivated (customer and supplier
 * messages); one with zero balance can, and then drops out of the default list.
 */
import { defineCase } from '../../case';
import * as partyService from '../../../../src/modules/parties/services/partyService';

export default defineCase({
  name: 'parties/parties-deactivate-guard',
  source: '03-domains/05-parties.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: '**.phones', emptyArrayOnly: true, reason: '05-parties D-6: phones is always an array in Rust; the mock leaves it absent on a party saved without phones (absent ≡ [])' },
  ],
  async run(s) {
    // cus-6 has an unpaid invoice in the seed → balance > 0.
    const owing = await s.step('owing-customer', () => partyService.getCustomer(s.baseId('cus-6')));
    await s.expectError('deactivate-owing-customer', () =>
      partyService.saveCustomer({ type: owing.type, name: owing.name, active: false }, owing.id),
    );
    // sup-1 has an unpaid received PO in the seed → balance > 0.
    const owed = await s.step('owed-supplier', () => partyService.getSupplier(s.baseId('sup-1')));
    await s.expectError('deactivate-owed-supplier', () =>
      partyService.saveSupplier({ type: owed.type, name: owed.name, active: false }, owed.id),
    );
    // A fresh customer/supplier has zero balance → deactivation is allowed.
    const fresh = await s.step('create-customer', () => partyService.saveCustomer({ type: 'individual', name: 'عميل مؤقت', active: true }));
    const freshSup = await s.step('create-supplier', () => partyService.saveSupplier({ type: 'company', name: 'مورد مؤقت', active: true }));
    // The deactivations happen a minute later, as they would at a real counter: Rust shows
    // `updatedAt` only when `updated_at ≠ created_at` (05 §DTO — the mock sets it on every update),
    // which a create + update inside the same pinned millisecond can't express.
    await s.setClock('2026-06-30T09:01:00.000Z');
    await s.step('deactivate-customer', () => partyService.saveCustomer({ type: 'individual', name: 'عميل مؤقت', active: false }, fresh.id));
    await s.step('deactivate-supplier', () => partyService.saveSupplier({ type: 'company', name: 'مورد مؤقت', active: false }, freshSup.id));
    await s.step('customers-active', () => partyService.getCustomers());
    await s.step('customers-all', () => partyService.getCustomers({ includeInactive: true }));
    await s.step('suppliers-active', () => partyService.getSuppliers());
    await s.step('suppliers-all', () => partyService.getSuppliers({ includeInactive: true }));
  },
});
