/**
 * L1 platform lane. Submit → approve happy path (04-approvals.md §3 `submit`/`decide`): the request
 * row, the activity row, and approving without a comment vs. with a trimmed comment.
 */
import { defineCase } from '../../case';
import * as approvalService from '../../../../src/modules/approvals/services/approvalService';

export default defineCase({
  name: 'approvals/approvals-submit-approve',
  source: '03-domains/04-approvals.md §8(b)',
  base: 'demo-sa',
  user: 'cashier',
  async run(s) {
    const req = await s.step('submit-discount', () =>
      approvalService.submitApprovalRequest({ kind: 'discount', summary: 'خصم 25% على فاتورة عميل نقدي', value: 25, requestNote: '  عميل دائم  ' }),
    );
    // The queue and its bell count need Approvals:Read, which a cashier doesn't have
    // (`permissions.ts`, 04-approvals.md D-2 — the mock's reads have no gate, Rust's do).
    await s.login('manager');
    await s.step('list-pending', () => approvalService.getApprovalRequests({ status: 'pending' }));
    await s.step('pending-count', () => approvalService.getPendingApprovalCount());

    const approved = await s.step('approve-no-comment', () => approvalService.approveRequest(req.id));
    await s.step('pending-count-after', () => approvalService.getPendingApprovalCount());

    const req2 = await s.step('submit-below-cost', () =>
      approvalService.submitApprovalRequest({ kind: 'below_cost', summary: 'بيع بسعر أقل من التكلفة', value: 12.5 }),
    );
    await s.step('approve-with-comment', () => approvalService.approveRequest(req2.id, { comment: '  ok  ' }));

    void approved;
    await s.step('list-approved', () => approvalService.getApprovalRequests({ status: 'approved' }));
  },
});
