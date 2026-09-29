/**
 * Drift check (21.02-F, F-4): proves the ts-rs-generated audit DTOs (`diagnostics/types/gen/*`,
 * written by `bun run bindings` from `src-tauri/src/core/dto.rs`) have exactly the same shape as
 * the hand-written TS types in `./index.ts`.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`.
 */
import type { Equals, Expect, Simplify } from '@/modules/core/types/contract';
import type { AuditAction, AuditEntry, AuditFieldDiff, ServerDiagnostics, SupportSnapshot } from './index';
import type { AuditFilter } from '../services/auditService';
import type {
  AccountingDocSummary,
  DriftRow,
  ExplainLine,
  getBalancesAround,
  getInvariantResults,
  getPostingTrace,
} from '../services/accountingDebugService';

import type { AuditAction as GenAuditAction } from './gen/AuditAction';
import type { AuditFieldDiff as GenAuditFieldDiff } from './gen/AuditFieldDiff';
import type { AuditEntry as GenAuditEntry } from './gen/AuditEntry';
import type { AuditFilter as GenAuditFilter } from './gen/AuditFilter';
import type { ServerDiagnosticsDto as GenServerDiagnostics } from './gen/ServerDiagnosticsDto';
import type { SupportSnapshot as GenSupportSnapshot } from './gen/SupportSnapshot';
import type { AccountingDocSummary as GenAccountingDocSummary } from './gen/AccountingDocSummary';
import type { BalanceAround as GenBalanceAround } from './gen/BalanceAround';
import type { DriftRow as GenDriftRow } from './gen/DriftRow';
import type { ExplainLine as GenExplainLine } from './gen/ExplainLine';
import type { InvariantResult as GenInvariantResult } from './gen/InvariantResult';
import type { PostingTrace as GenPostingTrace } from './gen/PostingTrace';

export type _AuditAction = Expect<Equals<GenAuditAction, AuditAction>>;
export type _AuditFieldDiff = Expect<Equals<GenAuditFieldDiff, AuditFieldDiff>>;
export type _AuditEntry = Expect<Equals<GenAuditEntry, AuditEntry>>;
// contract-ok: the Rust `AuditFilter.userId` binds as `string` (the `Id` newtype's ts-rs override)
// against the same `string` the hand-written frontend type already used for a raw user id — no
// widening/narrowing beyond that, so a plain Equals check is used rather than a Simplify wrapper.
export type _AuditFilter = Expect<Equals<GenAuditFilter, AuditFilter>>;
export type _ServerDiagnostics = Expect<Equals<GenServerDiagnostics, ServerDiagnostics>>;
export type _SupportSnapshot = Expect<Equals<GenSupportSnapshot, SupportSnapshot>>;

// Slice B (21 Part 03 §16 spec §2) — the 7 debug-build-only accounting-debugger reads. `BalanceAround`
// and `InvariantResult` have no standalone exported interface on the frontend (they're only ever the
// element type of a service's return array), so they're pulled off the service function's own return
// type rather than a named export; `PostingTrace` is `NonNullable<...>` since the frontend type is
// `T | undefined` where the Rust `Option<T>` binds as `T | null` (handled at the switch line instead).
export type _AccountingDocSummary = Expect<Equals<GenAccountingDocSummary, AccountingDocSummary>>;
export type _BalanceAround = Expect<Equals<GenBalanceAround, Awaited<ReturnType<typeof getBalancesAround>>[number]>>;
export type _DriftRow = Expect<Equals<GenDriftRow, DriftRow>>;
export type _ExplainLine = Expect<Equals<GenExplainLine, ExplainLine>>;
export type _InvariantResult = Expect<Equals<GenInvariantResult, Awaited<ReturnType<typeof getInvariantResults>>[number]>>;
// contract-ok: `PostingTrace.lines` is typed `JournalLine[]` on the frontend (the persisted shape),
// but a trace is recorded BEFORE the journal entry/lines are persisted (`shared::ledger::trace`'s
// own doc comment) — no line id exists yet, and `branchId`/`currency` are always populated (never
// optional) at trace time since the ledger always resolves them before posting. Rust's `TraceLine`
// mirrors what's actually available at that point, not `JournalLine`'s full persisted shape, so the
// `lines` element type is compared against that narrower shape instead of `JournalLine` itself.
// `Simplify` here too (not just on the outer `_PostingTrace` object below) — `Omit<> & {...}` stays
// a distinct intersection type until flattened, and `Equals` can tell an unflattened intersection
// apart from ts-rs's flat object even when every individual member matches (proven: every field
// compared pairwise passes `Equals`, only the whole, un-simplified type does not).
type ExpectedTraceLine = Simplify<Omit<import('@/modules/accounting/types').JournalLine, 'id' | 'branchId' | 'currency'> & { branchId: string; currency: string }>;
export type _PostingTrace = Expect<
  Equals<GenPostingTrace, Simplify<Omit<NonNullable<Awaited<ReturnType<typeof getPostingTrace>>>, 'lines'> & { lines: ExpectedTraceLine[] }>>
>;
