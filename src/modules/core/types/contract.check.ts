/**
 * Drift check (21.02-F, F-4): proves every ts-rs-generated cross-cutting DTO
 * (`core/types/gen/*`, written by `bun run bindings` from `src-tauri/src/core/{dto,events}.rs`)
 * has *exactly* the same shape as the hand-written TS type it mirrors. A field-type or
 * optionality drift on either side fails `vue-tsc` here, not silently at runtime.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`.
 */
import type { Equals, Expect, Simplify } from './contract';
import type { ApiErrorPayload, BackendChangedPayload, BackendServerStatus, BackendStatus, ChangeCategory } from './backend';
import type { PagedQuery, PagedResult, PageSort } from './paging';
import type { ActivityEntry, DashboardSummary } from './index';
import type { HomeKpi, HomeKpis, TopCustomerRow, TopProductRow, getRecentActivity, getRecentInvoices } from '../services/dashboardService';
import type { Insight, InsightIconKey } from '../services/insightTypes';
import type { AttachmentKind, AttachmentMeta } from '@/mocks/attachments';
import type { Role } from '@/modules/users/types';

import type { ApiErrorPayload as GenApiErrorPayload } from './gen/ApiErrorPayload';
import type { BackendChangedPayload as GenBackendChangedPayload } from './gen/BackendChangedPayload';
import type { BackendServerStatus as GenBackendServerStatus } from './gen/BackendServerStatus';
import type { BackendStatus as GenBackendStatus } from './gen/BackendStatus';
import type { ChangeCategory as GenChangeCategory } from './gen/ChangeCategory';
import type { PageSort as GenPageSort } from './gen/PageSort';
import type { PagedQuery as GenPagedQuery } from './gen/PagedQuery';
import type { PagedResult as GenPagedResult } from './gen/PagedResult';
import type { ActivityEntry as GenActivityEntry } from './gen/ActivityEntry';

import type { DashboardSummary as GenDashboardSummary } from './gen/DashboardSummary';
import type { HomeKpi as GenHomeKpi } from './gen/HomeKpi';
import type { HomeKpis as GenHomeKpis } from './gen/HomeKpis';
import type { GrossProfitKpi as GenGrossProfitKpi } from './gen/GrossProfitKpi';
import type { ReceivablesKpi as GenReceivablesKpi } from './gen/ReceivablesKpi';
import type { TopProductRow as GenTopProductRow } from './gen/TopProductRow';
import type { TopCustomerRow as GenTopCustomerRow } from './gen/TopCustomerRow';
import type { RecentInvoice as GenRecentInvoice } from './gen/RecentInvoice';
import type { RecentActivityEntry as GenRecentActivityEntry } from './gen/RecentActivityEntry';
import type { InsightDto as GenInsightDto } from './gen/InsightDto';
import type { AttachmentKind as GenAttachmentKind } from './gen/AttachmentKind';
import type { AttachmentMeta as GenAttachmentMeta } from './gen/AttachmentMeta';

export type _ApiErrorPayload = Expect<Equals<GenApiErrorPayload, ApiErrorPayload>>;
export type _BackendChangedPayload = Expect<Equals<GenBackendChangedPayload, BackendChangedPayload>>;
export type _BackendServerStatus = Expect<Equals<GenBackendServerStatus, BackendServerStatus>>;
export type _BackendStatus = Expect<Equals<GenBackendStatus, BackendStatus>>;
export type _ChangeCategory = Expect<Equals<GenChangeCategory, ChangeCategory>>;
export type _PageSort = Expect<Equals<GenPageSort, PageSort>>;

// Generics checked at a concrete instantiation (F-4) — the Rust side is exported with a `String`
// filler type argument (`core/ipc.rs`'s `export_bindings`), so the TS comparison uses the same
// concrete instance rather than the bare generic alias.
export type _PagedQuery = Expect<Equals<GenPagedQuery<Record<string, unknown>>, PagedQuery<Record<string, unknown>>>>;
export type _PagedResult = Expect<Equals<GenPagedResult<string>, PagedResult<string>>>;

export type _ActivityEntry = Expect<Equals<GenActivityEntry, ActivityEntry>>;

// --- 21.03 14-analytics.md / 14b-insights.md (dashboard) ------------------------------------------

export type _DashboardSummary = Expect<Equals<GenDashboardSummary, DashboardSummary>>;
export type _HomeKpi = Expect<Equals<GenHomeKpi, HomeKpi>>;
// `HomeKpis.grossProfit`/`.receivables` are themselves intersections (`HomeKpi & { marginPct }`
// etc.) — a nested intersection field defeats `Equals` even under an outer `Simplify` (it only
// flattens the outer object's own members, not a member's own type), so those two fields are
// pre-simplified before the top-level comparison (same G-38 reasoning as `_GrossProfitKpi` below).
export type _HomeKpis = Expect<
  Equals<GenHomeKpis, Simplify<Omit<HomeKpis, 'grossProfit' | 'receivables'> & { grossProfit: Simplify<HomeKpis['grossProfit']>; receivables: Simplify<HomeKpis['receivables']> }>>
>;
export type _TopProductRow = Expect<Equals<GenTopProductRow, TopProductRow>>;
export type _TopCustomerRow = Expect<Equals<GenTopCustomerRow, TopCustomerRow>>;

// G-38: `HomeKpi & { marginPct: number }` / `HomeKpi & { overdue: number }` are intersections on the
// TS side but flat structs on the Rust side — compared via `Simplify` (contract.ts's own doc
// comment gives this exact pair as the motivating example).
export type _GrossProfitKpi = Expect<Equals<Simplify<GenGrossProfitKpi>, Simplify<HomeKpis['grossProfit']>>>;
export type _ReceivablesKpi = Expect<Equals<Simplify<GenReceivablesKpi>, Simplify<HomeKpis['receivables']>>>;

// `Invoice & { customerName?: string }` / `ActivityEntry & { userName?: string }` — same reason;
// the hand-written shape is inline in each function's return type, so it's pulled out via
// `Awaited<ReturnType<...>>[number]` rather than duplicated as a second named type.
export type _RecentInvoice = Expect<Equals<Simplify<GenRecentInvoice>, Simplify<Awaited<ReturnType<typeof getRecentInvoices>>[number]>>>;
export type _RecentActivityEntry = Expect<Equals<Simplify<GenRecentActivityEntry>, Simplify<Awaited<ReturnType<typeof getRecentActivity>>[number]>>>;

// `Insight` minus the Vue `icon: Component`, plus the wire-facing `icon: InsightIconKey` — see
// 14b-insights.md §2's `contract-ok` note: `icon` is a Vue component client-side (the service maps
// `InsightIconKey` → `Component`), and `roles` is `readonly Role[]` in TS vs a plain array in Rust,
// neither of which `Equals`/`Simplify` needs reconciled beyond the shape below.
// contract-ok: icon is a Vue component; the service maps InsightIconKey → Component; roles is readonly in TS
export type _InsightDto = Expect<Equals<Simplify<GenInsightDto>, Simplify<Omit<Insight, 'icon' | 'roles'> & { icon: InsightIconKey; roles: Role[] }>>>;

// --- C-16 attachments (src-tauri/src/domains/attachments/dto.rs) ----------------------------------

export type _AttachmentKind = Expect<Equals<GenAttachmentKind, AttachmentKind>>;
// `AttachmentMeta`'s wire shape is identical to the mock's own `AttachmentMeta` (`src/mocks/
// attachments.ts`) — `attachmentService.ts`'s `metaFromDto` maps the Rust DTO onto it 1:1.
// `AttachmentRecord` isn't checked here: Rust carries the blob as `blobBase64`/`thumbnailBase64`
// (base64 strings, the `pdf_base64` precedent) where the mock's own type has `blob`/`thumbnail:
// Blob` — `attachmentService.ts`'s `recordFromDto` is the one place that reconciles the two shapes.
// contract-ok: AttachmentRecord's blob/thumbnail are Blob in the mock vs base64 strings on the wire (attachmentService.ts's recordFromDto)
export type _AttachmentMeta = Expect<Equals<GenAttachmentMeta, AttachmentMeta>>;
