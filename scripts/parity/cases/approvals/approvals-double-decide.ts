/**
 * L1 platform lane. Deciding an already-decided request, and deciding an unknown id
 * (04-approvals.md §3 `decide` steps 1-2).
 */
import { defineCase } from '../../case';
import * as approvalService from '../../../../src/modules/approvals/services/approvalService';

export default defineCase({
  name: 'approvals/approvals-double-decide',
  source: '03-domains/04-approvals.md §8(b)',
  base: 'demo-sa',
  user: 'cashier',
  async run(s) {
    const req = await s.step('submit', () => approvalService.submitApprovalRequest({ kind: 'discount', summary: 'خصم كبير', value: 40 }));

    await s.login('admin');
    await s.step('approve', () => approvalService.approveRequest(req.id));
    await s.expectError('approve-again', () => approvalService.approveRequest(req.id));
    await s.expectError('reject-after-approved', () => approvalService.rejectRequest(req.id, { comment: 'متأخر' }));
    await s.expectError('decide-unknown-id', () => approvalService.approveRequest('apr-does-not-exist'));
  },
});
