/**
 * 05-parties §8(b): codes are MAX+1 per kind (`C-0900` in the seed → `C-0901`, `S-0005` → `S-0006`),
 * the create path's validation order (name before VAT, SA VAT rule), history + audit rows.
 */
import { defineCase } from '../../case';
import * as partyService from '../../../../src/modules/parties/services/partyService';

export default defineCase({
  name: 'parties/parties-create-codes',
  source: '03-domains/05-parties.md §8(b)',
  base: 'demo-sa',
  allow: [
    { path: '**.phones', emptyArrayOnly: true, reason: '05-parties D-6: phones is always an array in Rust; the mock leaves it absent on a party saved without phones (absent ≡ [])' },
  ],
  async run(s) {
    await s.expectError('blank-name', () => partyService.saveCustomer({ type: 'individual', name: '   ', active: true }));
    // Name is checked before the VAT rule (validate_common order).
    await s.expectError('blank-name-bad-vat', () => partyService.saveCustomer({ type: 'company', name: ' ', vatNumber: '123', active: true }));
    await s.expectError('bad-vat-sa', () => partyService.saveCustomer({ type: 'company', name: 'شركة اختبار', vatNumber: '123456789', active: true }));
    await s.expectError('supplier-blank-name', () => partyService.saveSupplier({ type: 'company', name: '', active: true }));
    await s.expectError('supplier-bad-vat-sa', () => partyService.saveSupplier({ type: 'company', name: 'مورد اختبار', vatNumber: '400000000000003', active: true }));

    const c1 = await s.step('create-customer-1', () =>
      partyService.saveCustomer({ type: 'individual', name: '  سالم العتيبي  ', nameEn: '  ', phone: ' +966500000001 ', email: '', active: true }),
    );
    const c2 = await s.step('create-customer-2', () =>
      partyService.saveCustomer({ type: 'company', name: 'مؤسسة الأفق', vatNumber: '300000000000003', creditLimit: 1000, groupId: s.baseId('pg-corporate'), active: true }),
    );
    const sup = await s.step('create-supplier', () =>
      partyService.saveSupplier({ type: 'company', name: 'مورد الأقمشة الجديد', contactPerson: '  خالد  ', active: true }),
    );
    await s.step('get-customer-1', () => partyService.getCustomer(c1.id));
    await s.step('get-customer-2', () => partyService.getCustomer(c2.id));
    await s.step('get-supplier', () => partyService.getSupplier(sup.id));
    await s.step('history-customer-1', () => partyService.getPartyHistory(c1.id));
    await s.step('history-supplier', () => partyService.getPartyHistory(sup.id));
    // A supplier id through the customer reader (and vice versa) is "not found" — kind must match.
    await s.expectError('get-customer-by-supplier-id', () => partyService.getCustomer(s.baseId('sup-1')));
    await s.expectError('get-supplier-by-customer-id', () => partyService.getSupplier(s.baseId('cus-1')));
    await s.step('customers-after', () => partyService.getCustomers({ includeInactive: true }));
    await s.step('suppliers-after', () => partyService.getSuppliers({ includeInactive: true }));
  },
});
