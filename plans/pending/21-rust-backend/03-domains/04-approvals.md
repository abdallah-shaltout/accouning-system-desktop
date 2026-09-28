# 21 · 03.04 — `approvals` (async manager-approval queue)

> **Status:** planned 2026-09-28, not implemented. Wave **W2** (entry file §4). Depends on: 03-users
> (session, the requester/decider name), Part 02 `shared::activity`, `core/lock.rs`, entity
> `platform/approval_requests.rs`, and Part 02 gaps G-6/G-8/G-9 (§7).

**Goal.** Port the 5 functions of `approvalService.ts` (logic in `src/mocks/backend/approvals.ts`)
to `domains/approvals/`: submit a request, list/count, approve/reject under a row lock so two
managers on two terminals can't both decide the same request. No ledger, stock or numbering reach.

**Read first.** [`../01-frontend-analysis/approvals.md`](../01-frontend-analysis/approvals.md) (§1–§6,
§8 `ledger:changed` note) · mock `src/mocks/backend/approvals.ts:13-73`, service
`src/modules/approvals/services/approvalService.ts:8-36`, types `src/modules/approvals/types/index.ts:20-56`
· callers `DiscountDialog.vue:85`, `CustomPriceDialog.vue:94`, `ApprovalPinDialog.vue:60`,
`ApprovalsPage.vue:25,57-58` · Rust `entities/platform/approval_requests.rs` (+ `m0013_platform.rs:95-120`),
`core/lock.rs::for_update_by_id`, `shared/activity/record.rs::log`.

## 1. Commands

| Mock fn | Disposition | Rust command | Args → Return | Area / Access | Tx | Events |
|---|---|---|---|---|---|---|
| `submitApprovalRequest` | port | `approvals_submit_approval_request` | `ApprovalsSubmitApprovalRequestArgs { input: ApprovalRequestInput }` → `ApprovalRequest` | by kind (D-2): `discount`/`below_cost` → Pos:Write **or** Sales:Write; `write_off` → Inventory:Write | `with_tx` | `Ledger` (mock `emit('ledger:changed')`, `:34`) |
| `getApprovalRequests` | port | `approvals_get_approval_requests` | `ApprovalsGetApprovalRequestsArgs { filter: Option<ApprovalListFilter> }` → `Vec<ApprovalRequest>` | Approvals:Read | `with_read` | — |
| `getPendingApprovalCount` | port | `approvals_get_pending_approval_count` | `()` → `u32` | Approvals:Read | `with_read` | — |
| `approveRequest` | port | `approvals_approve_request` | `ApprovalsApproveRequestArgs { id, input: Option<ApprovalDecisionInput> }` → `ApprovalRequest` | Approvals:Write | `with_tx` | `Ledger` (`:63`) |
| `rejectRequest` | port | `approvals_reject_request` | `ApprovalsRejectRequestArgs { id, input: ApprovalDecisionInput }` → `ApprovalRequest` | Approvals:Write | `with_tx` | `Ledger` |

5 commands. Approve and reject share one `decide(conn, cx, registry, id, status, input)` (approvals.md §1 note).

## 2. DTOs (`domains/approvals/dto.rs`, `#[ts(export_to = "approvals/types/gen/")]`)

| Rust DTO | TS type (file:line) | Notes |
|---|---|---|
| `ApprovalKind` | `ApprovalKind` (`:20`) | `#[serde(rename_all = "snake_case")]` → `discount`/`write_off`/`below_cost`; `From`/`Into` the SeaORM `approval_requests::ApprovalKind`. |
| `ApprovalStatus` | `ApprovalStatus` (`:22`) | lowercase; `From`/`Into` the entity enum. |
| `ApprovalRequest` | `ApprovalRequest` (`:24-44`) | `id`, `requested_by`, `decided_by` → `#[ts(type="string")]`; `value: Decimal` `serde_number` + `#[ts(type="number")]`; `requested_at: String` = `model.requested_at().key()`, `decided_at: Option<String>` = `decided_at().map(key)`; `request_note`, `decided_by*`, `decision_comment`, `link` optional (`skip_serializing_none` + `#[ts(optional)]`); `link: Option<RouteRef>` with `#[ts(optional, type = "import('@/modules/core/types/route').AppRoute")]` (alias path — see G-8c). |
| `ApprovalRequestInput` | `ApprovalRequestInput` (`:46-52`) | request-only; `value: Decimal` (`serde_number` deserialize); `link: Option<RouteRef>`. |
| `ApprovalDecisionInput` | `ApprovalDecisionInput` (`:54-56`) | `comment: Option<String>`. |
| `ApprovalListFilter` | inline `{ status?: 'pending' \| 'approved' \| 'rejected' }` (`approvalService.ts:18`) | `status: Option<ApprovalStatus>`. |

`src/modules/approvals/types/contract.check.ts` (new): `Equals` for `ApprovalKind`, `ApprovalStatus`,
`ApprovalRequest`, `ApprovalRequestInput`, `ApprovalDecisionInput`; the filter checked as
`Equals<GenApprovalListFilter, NonNullable<Parameters<typeof getApprovalRequests>[0]>>`.
`getPendingApprovalCount`: `u32` → `number`, no DTO.

## 3. Service logic (`domains/approvals/service.rs`)

`KIND_LABEL` (`approvals.ts:13-17`), copied byte for byte: `discount` → `خصم يتجاوز الحد المسموح`,
`write_off` → `إتلاف/تسوية مخزون تتجاوز حد الاعتماد`, `below_cost` → `بيع بسعر أقل من التكلفة`.
`fn actor_name(conn, cx)`: the `users.name` of `cx.actor.id`, or `مستخدم` when the row is missing
(`approvalService.ts:8-10` reads the auth store's user name with the same fallback) — the name is
**never** taken from the client (approvals.md §1).

**`submit(conn, cx, registry, input)`** (`approvals.ts:19-36`):
1. Access by kind (D-2) — done in the command before the service call, using `require_any` (G-6).
2. Insert `approval_requests`: `id = Id::new()`, `kind`, `summary` as given (no trim, no
   validation — the mock has none), `value = round4(input.value)` (DECIMAL(19,4), D-3),
   `request_note = input.request_note.trim()` or `None` if empty (`:25`), `requested_by = actor.id`,
   `requested_by_name = actor_name`, `requested_at = DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) }`
   via `doc_date::write`, `status = pending`, `link`, `created_at = updated_at = now`, `sync_status = local`.
3. `log(conn, cx, registry, ActivityKind::Approval, format!("طلب اعتماد جديد: {} — {}", KIND_LABEL[kind], summary),
   Some(requested_at), Some(RouteRef::list("approvals")))` (`:33`).
4. `cx.touch(ChangeCategory::Ledger)` (`:34`, D-1).
5. Return the DTO of the inserted model.

**`list(conn, filter)`** (`:67-69`): `SELECT … [WHERE status = ?] ORDER BY created_at, id`, then a
**stable** Rust sort by `requested_at` key descending (`b.requestedAt.localeCompare(a.requestedAt)`;
ties keep insertion order; a string sort, not SQL, so day-only and instant keys compare exactly as
in the mock).

**`pending_count(conn)`** (`:71-73`): `COUNT(*) WHERE status = 'pending'` → `u32`.

**`decide(conn, cx, registry, id, status, input)`** (`:44-65`):
1. `lock::for_update_by_id(conn, "approval_requests", id)` then `find_by_id` → none → `NOT_FOUND`
   `طلب الاعتماد غير موجود` (`:38-41`).
2. `status != pending` → `VALIDATION` `تم اتخاذ قرار بشأن هذا الطلب مسبقاً` (`:46`) — read after the lock (§4).
3. Reject only: `input.comment` trimmed empty or absent → `VALIDATION` `أدخل سبب الرفض` (`:47`).
4. Update: `status`, `decided_by = actor.id`, `decided_by_name = actor_name`, `decided_at = DocDate { today, Some(now) }`,
   `decision_comment = comment.trim()` or `None` if empty (`:54`, never stored as `""`), `updated_at = now`.
5. `log(ActivityKind::Approval, format!("{} طلب: {} — {}", if approved {"اعتماد"} else {"رفض"}, KIND_LABEL[kind], summary),
   Some(decided_at), Some(RouteRef::list("approvals")))` (`:56-62`).
6. `cx.touch(ChangeCategory::Ledger)` (`:63`). Return the DTO.

`approveRequest(id, input = {})` → the command passes `input.unwrap_or_default()`.

## 4. Concurrency (D8)

- Two managers deciding the same request: `SELECT … FOR UPDATE` on the request row before reading
  `status` (approvals.md §5). The second transaction waits, then sees the committed status and gets
  the "already decided" `VALIDATION`. With READ COMMITTED (P2-06) the post-lock `find_by_id` reads
  the committed row.
- Submissions are independent inserts (UUIDv7 ids) — no contention.
- The `Ledger` bump happens in `with_tx` at commit (last lock, P2-10).

## 5. Undo

Not undoable via the registry (approvals.md §4 — decisions are terminal; phase-e E-5 lists none).

## 6. Frontend switch lines (`src/modules/approvals/services/approvalService.ts`)

- `submitApprovalRequest`: `if (usesRust('approvals')) return backendCall('approvals_submit_approval_request', { input });`
- `getApprovalRequests`: `… return backendCall('approvals_get_approval_requests', { filter });`
- `getPendingApprovalCount`: `… return backendCall('approvals_get_pending_approval_count');`
- `approveRequest`: `… return backendCall('approvals_approve_request', { id, input });`
- `rejectRequest`: `… return backendCall('approvals_reject_request', { id, input });`

`currentUserName()` stays for the mock path only.

## 7. Known mock quirks (kept) and decisions

**Quirks kept:**
- Q-1 The audit row's `entity_id` is a fresh id, not the request id (`{ name: 'approvals' }` is a list route, so `entityFromLink` falls back — `core.ts:335-343`).
- Q-2 `summary`/`value` are not validated (empty summary, negative value accepted).
- Q-3 A requester may approve their own request if they hold Approvals:Write (the mock doesn't forbid it).
- Q-4 `requestedByName` is a snapshot; a later rename doesn't change old requests.

**Decisions:**
- D-1 Keep touching `ChangeCategory::Ledger` (mock parity; the frontend's `onLedgerChanged` refreshes the bell/insights). approvals.md §8 said either choice preserves behaviour; parity wins.
- D-2 Submit is authorised by the area of the action being approved: the three callers are the POS/sales discount and price dialogs (cashier: Pos/Sales write) and the inventory PIN dialog (storekeeper: Inventory write). Approvals:Write can't be used — cashiers have Approvals:None (`permissions.ts`).
- D-3 `value` is `round4`'d at write (DECIMAL(19,4) storage scale, approvals.md §2); callers already send `round2` values.
- D-4 `link` is stored as `RouteRef` (`name` + `params`); an `AppRoute` `query`/`hash` would be dropped — no caller sends a link today.
- D-5 `requested_at`/`decided_at` DTO strings depend on G-9 below (instant columns must be `DATETIME(3)`), otherwise milliseconds are lost and the sort ties differ from the mock.

**Part 02 gaps this file needs** (ids as defined in [`03-users.md`](03-users.md) §7): G-6
`require_any` (submit), G-8 bindings.
Plus **G-9**: every DocDate `*_instant` column in m0008–m0013 (here `approval_requests.requested_at_instant`/
`decided_at_instant`, `m0013_platform.rs:108,113`) is `.timestamp()` = `TIMESTAMP(0)`, not P2-09's
`DATETIME(3)`: milliseconds are lost, so `DocDate::key()` stops matching the mock's ISO string and
same-second rows sort differently. Fix in place (Part 02's tests have not passed yet, entry §3.7).

## 8. Tests

**(a) `src-tauri/tests/domain_approvals.rs`:**
- submit as cashier (`discount`) → row `pending`, `requested_by` = session user, `requested_by_name` = DB name (not client-sent), `request_note` `"  "` → absent; one audit + one activity (`kind=approval`, message text exact); `change_versions.ledger` +1.
- submit `write_off` as cashier → `FORBIDDEN`; as storekeeper → OK; accountant `discount` → OK (Sales write).
- list: newest first; `status` filter; ties keep insertion order.
- count equals pending rows.
- approve without input → `decision_comment` absent; approve with `"  ok "` → `"ok"`.
- reject without comment / whitespace comment → `VALIDATION` `أدخل سبب الرفض`.
- decide unknown id → `NOT_FOUND`; decide twice → `VALIDATION` `تم اتخاذ قرار بشأن هذا الطلب مسبقاً`.
- cashier approve → `FORBIDDEN`.
- concurrency: approve and reject the same request in parallel → exactly one succeeds, the other gets the "already decided" `VALIDATION`; final `decided_by` = the winner.

**(b) Parity cases:** `approvals-submit-approve`, `approvals-reject-requires-comment`, `approvals-double-decide`, `approvals-list-order`.

## 9. Checklist

- [ ] `domains/approvals/{mod,dto,service,commands}.rs` (mod: `ipc_signatures()` with 5 lines + `export_bindings(cfg)`).
- [ ] `dto.rs` per §2 with entity-enum conversions.
- [ ] `service.rs`: `KIND_LABEL`, `actor_name`, `submit`, `list`, `pending_count`, `decide` (§3 order, mock line comments).
- [ ] `commands.rs`: 5 commands; kind-based `require_any` in submit; Approvals:Read/Write elsewhere.
- [ ] Manager: register the 5 commands, `pub mod approvals;`, hooks in `domains/mod.rs`.
- [ ] Switch lines (§6); `src/modules/approvals/types/contract.check.ts` (§2).
- [ ] `tests/domain_approvals.rs` (§8a); parity list to Part 04.
- [ ] Status note at the top of this file.

## Gate

`cargo check` clean (manager run) · tests written · switch lines · `contract.check.ts` compiles ·
`memory:check` 0 contract gaps for the 5 commands. DB tests/parity in the deferred time-boxed pass.
