/**
 * Drift check (21.02-F, F-4 / plans/pending/21-rust-backend/03-domains/04-approvals.md §2): proves
 * the ts-rs-generated `approvals` DTOs (`approvals/types/gen/*`, written by `bun run bindings` from
 * `src-tauri/src/domains/approvals/dto.rs`) have exactly the same shape as the hand-written TS
 * types in `./index.ts`.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`.
 */
import type { Equals, Expect } from '@/modules/core/types/contract';
import type { ApprovalDecisionInput, ApprovalKind, ApprovalRequest, ApprovalRequestInput, ApprovalStatus } from './index';
import { getApprovalRequests } from '../services/approvalService';

import type { ApprovalKind as GenApprovalKind } from './gen/ApprovalKind';
import type { ApprovalStatus as GenApprovalStatus } from './gen/ApprovalStatus';
import type { ApprovalRequest as GenApprovalRequest } from './gen/ApprovalRequest';
import type { ApprovalRequestInput as GenApprovalRequestInput } from './gen/ApprovalRequestInput';
import type { ApprovalDecisionInput as GenApprovalDecisionInput } from './gen/ApprovalDecisionInput';
import type { ApprovalListFilter as GenApprovalListFilter } from './gen/ApprovalListFilter';

export type _ApprovalKind = Expect<Equals<GenApprovalKind, ApprovalKind>>;
export type _ApprovalStatus = Expect<Equals<GenApprovalStatus, ApprovalStatus>>;
export type _ApprovalRequest = Expect<Equals<GenApprovalRequest, ApprovalRequest>>;
export type _ApprovalRequestInput = Expect<Equals<GenApprovalRequestInput, ApprovalRequestInput>>;
export type _ApprovalDecisionInput = Expect<Equals<GenApprovalDecisionInput, ApprovalDecisionInput>>;

// The inline `{ status?: 'pending' | 'approved' | 'rejected' }` filter (`approvalService.ts:18`)
// has no hand-written named type — checked directly against `getApprovalRequests`'s own parameter
// (04-approvals.md §2).
export type _ApprovalListFilter = Expect<Equals<GenApprovalListFilter, NonNullable<Parameters<typeof getApprovalRequests>[0]>>>;
