/**
 * L1 platform lane. `getAuditEntities` (16-diagnostics.md §3 `get_audit_entities`): distinct entity
 * kinds seen so far, sorted by code point (not locale-folded) — seeded history already writes
 * several kinds; a fresh one (`user`) is added so the list changes deterministically.
 */
import { defineCase } from '../../case';
import * as auditService from '../../../../src/modules/diagnostics/services/auditService';
import * as userService from '../../../../src/modules/users/services/userService';

export default defineCase({
  name: 'diagnostics/diagnostics-audit-entities',
  source: '03-domains/16-diagnostics.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('before', () => auditService.getAuditEntities());
    await s.step('create-user', () => userService.createUser({ username: 'entitycheck', name: 'فحص الكيانات', role: 'cashier', maxDiscount: 0, active: true, password: 'ent12345' }));
    await s.step('after', () => auditService.getAuditEntities());
  },
});
