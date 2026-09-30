/**
 * L1 platform lane. `getApprovalRequests` order (04-approvals.md §3 `list`): newest `requestedAt`
 * first, ties keeping insertion order — three submissions under the pinned clock (same instant),
 * then a decided one moving only its own row.
 */
import { defineCase } from '../../case';
import * as approvalService from '../../../../src/modules/approvals/services/approvalService';

export default defineCase({
  name: 'approvals/approvals-list-order',
  source: '03-domains/04-approvals.md §8(b)',
  base: 'demo-sa',
  user: 'cashier',
  async run(s) {
    const a = await s.step('submit-a', () => approvalService.submitApprovalRequest({ kind: 'discount', summary: 'طلب أ', value: 10 }));
    const b = await s.step('submit-b', () => approvalService.submitApprovalRequest({ kind: 'discount', summary: 'طلب ب', value: 20 }));
    await s.setClock('2026-07-01T10:00:00.000Z');
    const c = await s.step('submit-c', () => approvalService.submitApprovalRequest({ kind: 'below_cost', summary: 'طلب ج', value: 5 }));

    // Reading the queue needs Approvals:Read, which a cashier doesn't have (`permissions.ts`,
    // 04-approvals.md D-2 — the mock's list has no gate, Rust's does); the approver reads it.
    await s.login('manager');
    await s.step('list-all', () => approvalService.getApprovalRequests());

    await s.step('approve-b', () => approvalService.approveRequest(b.id));
    await s.step('list-after-decide', () => approvalService.getApprovalRequests());
    await s.step('list-pending-only', () => approvalService.getApprovalRequests({ status: 'pending' }));

    void a;
    void c;
  },
});
