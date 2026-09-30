/**
 * L1 platform lane. Reject requires a non-blank comment (04-approvals.md §3 `decide` step 3): no
 * comment, a whitespace-only comment, then a real rejection with a trimmed comment.
 */
import { defineCase } from '../../case';
import * as approvalService from '../../../../src/modules/approvals/services/approvalService';

export default defineCase({
  name: 'approvals/approvals-reject-requires-comment',
  source: '03-domains/04-approvals.md §8(b)',
  base: 'demo-sa',
  user: 'storekeeper',
  async run(s) {
    const req = await s.step('submit', () => approvalService.submitApprovalRequest({ kind: 'write_off', summary: 'إتلاف بضاعة تالفة', value: 340 }));

    await s.login('manager');
    await s.expectError('reject-no-comment', () => approvalService.rejectRequest(req.id, {}));
    await s.expectError('reject-blank-comment', () => approvalService.rejectRequest(req.id, { comment: '   ' }));

    await s.step('reject-with-comment', () => approvalService.rejectRequest(req.id, { comment: '  لا حاجة له  ' }));
    await s.step('list-rejected', () => approvalService.getApprovalRequests({ status: 'rejected' }));
  },
});
