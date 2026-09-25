import { clone, delay, session } from '@/mocks';
import { decideApproval, listApprovalRequests, pendingApprovalCount, requestApproval } from '@/mocks/backend/approvals';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import type { ApprovalDecisionInput, ApprovalRequest, ApprovalRequestInput } from '../types';

import { wrap } from '@/modules/diagnostics/services/defineService';

function currentUserName(): string {
  return useAuthStore().user?.name ?? 'مستخدم';
}

/** Submits a request to the async queue (docs/v2/14-platform.md §6) — used by the "لا يوجد مدير حالياً" escape hatch on the discount/price/write-off dialogs. */
export const submitApprovalRequest = wrap('approvals.submitApprovalRequest', async function submitApprovalRequest(input: ApprovalRequestInput): Promise<ApprovalRequest> {
  await delay(200);
  return clone(requestApproval(input, session.userId, currentUserName()));
});

export const getApprovalRequests = wrap('approvals.getApprovalRequests', async function getApprovalRequests(filter: { status?: 'pending' | 'approved' | 'rejected' } = {}): Promise<ApprovalRequest[]> {
  await delay();
  return clone(listApprovalRequests(filter));
});

export const getPendingApprovalCount = wrap('approvals.getPendingApprovalCount', async function getPendingApprovalCount(): Promise<number> {
  await delay(0);
  return pendingApprovalCount();
});

export const approveRequest = wrap('approvals.approveRequest', async function approveRequest(id: string, input: ApprovalDecisionInput = {}): Promise<ApprovalRequest> {
  await delay(250);
  return clone(decideApproval(id, 'approved', input, session.userId, currentUserName()));
});

export const rejectRequest = wrap('approvals.rejectRequest', async function rejectRequest(id: string, input: ApprovalDecisionInput): Promise<ApprovalRequest> {
  await delay(250);
  return clone(decideApproval(id, 'rejected', input, session.userId, currentUserName()));
});
