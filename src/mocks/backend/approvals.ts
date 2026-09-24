import type { ApprovalDecisionInput, ApprovalRequest, ApprovalRequestInput } from '@/modules/approvals/types';
import { db } from '../db';
import { emit } from '../events';
import { mutate } from '../persist';
import { ApiError, uid } from '../utils';
import { logActivity } from './core';

/**
 * Async approval queue (docs/v2/14-platform.md §6) — see `modules/approvals/types`'s doc comment
 * for why this exists alongside (not instead of) the synchronous PIN dialogs.
 */

const KIND_LABEL: Record<ApprovalRequest['kind'], string> = {
  discount: 'خصم يتجاوز الحد المسموح',
  write_off: 'إتلاف/تسوية مخزون تتجاوز حد الاعتماد',
  below_cost: 'بيع بسعر أقل من التكلفة',
};

export function requestApproval(input: ApprovalRequestInput, userId: string, userName: string): ApprovalRequest {
  const request: ApprovalRequest = {
    id: uid('apr'),
    kind: input.kind,
    summary: input.summary,
    value: input.value,
    requestNote: input.requestNote?.trim() || undefined,
    requestedBy: userId,
    requestedByName: userName,
    requestedAt: new Date().toISOString(),
    status: 'pending',
    link: input.link,
  };
  mutate(() => db.approvalRequests.push(request));
  logActivity('approval', `طلب اعتماد جديد: ${KIND_LABEL[input.kind]} — ${input.summary}`, userId, request.requestedAt, '/approvals');
  emit('ledger:changed'); // cheapest existing event to invalidate the insight-engine cache / bell badge
  return request;
}

function findRequest(id: string): ApprovalRequest {
  const request = db.approvalRequests.find((r) => r.id === id);
  if (!request) throw new ApiError('طلب الاعتماد غير موجود', 'NOT_FOUND');
  return request;
}

export function decideApproval(id: string, status: 'approved' | 'rejected', input: ApprovalDecisionInput, userId: string, userName: string): ApprovalRequest {
  const request = findRequest(id);
  if (request.status !== 'pending') throw new ApiError('تم اتخاذ قرار بشأن هذا الطلب مسبقاً', 'VALIDATION');
  if (status === 'rejected' && !input.comment?.trim()) throw new ApiError('أدخل سبب الرفض', 'VALIDATION');

  mutate(() => {
    request.status = status;
    request.decidedBy = userId;
    request.decidedByName = userName;
    request.decidedAt = new Date().toISOString();
    request.decisionComment = input.comment?.trim() || undefined;
  });
  logActivity(
    'approval',
    `${status === 'approved' ? 'اعتماد' : 'رفض'} طلب: ${KIND_LABEL[request.kind]} — ${request.summary}`,
    userId,
    request.decidedAt!,
    '/approvals',
  );
  emit('ledger:changed');
  return request;
}

export function listApprovalRequests(filter: { status?: 'pending' | 'approved' | 'rejected' } = {}): ApprovalRequest[] {
  return db.approvalRequests.filter((r) => !filter.status || r.status === filter.status).sort((a, b) => b.requestedAt.localeCompare(a.requestedAt));
}

export function pendingApprovalCount(): number {
  return db.approvalRequests.filter((r) => r.status === 'pending').length;
}
