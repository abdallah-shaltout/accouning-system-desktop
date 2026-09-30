/**
 * L1 platform lane. `getAuditEntries` filters (16-diagnostics.md §3 `get_audit_entries`): unfiltered
 * (newest first), then by `entity`, `action`, a `from`/`to` date window and a `search` term — each
 * against the audit rows the seeded history/tax/user actions already produced.
 */
import { defineCase } from '../../case';
import * as auditService from '../../../../src/modules/diagnostics/services/auditService';
import * as settingsService from '../../../../src/modules/settings/services/settingsService';
import * as userService from '../../../../src/modules/users/services/userService';

export default defineCase({
  name: 'diagnostics/diagnostics-audit-filter',
  source: '03-domains/16-diagnostics.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    // Produce a few fresh audit rows deterministically: a tax delete and a user create.
    const tax = await s.step('create-tax', () =>
      settingsService.saveTax({ name: 'ضريبة للتدقيق', rate: 3, type: 'INPUT', isDefault: false, active: true, category: 'S', direction: 'purchase', accountRole: 'vatInput' }),
    );
    await s.step('delete-tax', () => settingsService.deleteTax(tax.id));
    await s.step('create-user', () => userService.createUser({ username: 'audituser', name: 'مستخدم تدقيق', role: 'cashier', maxDiscount: 0, active: true, password: 'aud12345' }));

    await s.step('all', () => auditService.getAuditEntries());
    await s.step('by-entity-tax', () => auditService.getAuditEntries({ entity: 'tax' }));
    await s.step('by-entity-user', () => auditService.getAuditEntries({ entity: 'user' }));
    await s.step('by-action-delete', () => auditService.getAuditEntries({ action: 'delete' }));
    await s.step('by-date-window', () => auditService.getAuditEntries({ from: '2026-06-30', to: '2026-06-30' }));
    await s.step('by-date-window-none', () => auditService.getAuditEntries({ from: '2020-01-01', to: '2020-01-02' }));
    await s.step('by-search', () => auditService.getAuditEntries({ search: 'تدقيق' }));
    await s.step('by-search-none', () => auditService.getAuditEntries({ search: 'لا-يوجد-تطابق-ابدا' }));
  },
});
