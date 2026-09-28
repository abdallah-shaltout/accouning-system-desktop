# 21 · 01.B — `approvals` contract

> **Status:** done (2026-09-27) · **Inventory:** `docs/backend/contract/approvals.md`
> (regenerate with `bun run contract`) · **Mock spec:** `src/mocks/backend/approvals.ts` ·
> **Services:** `src/modules/approvals/services/approvalService.ts` · **Types:**
> `src/modules/approvals/types/index.ts`
>
> Small, self-contained module — an async approval queue (docs/v2/14-platform.md §6) for when the
> synchronous manager-PIN dialogs (discount/write-off/below-cost, built elsewhere and out of scope
> here) can't complete because no manager is physically present. It has one table (`approvalRequests`)
> and no ledger/stock/numbering/period reach at all — the smallest module reviewed so far.

## 1. Endpoints

| Function | Disposition (confirmed / changed + why) | Rust command | Request DTO | Response DTO | Writes (confirmed) | Shared | Undo | Notes |
|---|---|---|---|---|---|---|---|---|
| `submitApprovalRequest` | port (confirmed) | `approvals_submit_approval_request` | `ApprovalRequestInput` | `ApprovalRequest` | `approvalRequests`, activity | activity | not undoable | Called from the "لا يوجد مدير حالياً" escape hatch on the discount/price/write-off dialogs (those dialogs themselves are out of scope for this module). Resolves `requestedBy`/`requestedByName` from the **current session**, not from client-supplied input — Rust must do the same (never trust a client-sent user id/name for this field). Emits `ledger:changed` purely as a cache-invalidation signal for the insight engine / notification bell badge — not an actual ledger change (see §6). |
| `getApprovalRequests` | port (confirmed) | `approvals_get_approval_requests` | `{ status?: 'pending' \| 'approved' \| 'rejected' }` | `ApprovalRequest[]` | — | — | n/a (read) | Sorted by `requestedAt` descending (newest first) — the Rust `ORDER BY` must match. |
| `getPendingApprovalCount` | port (confirmed) | `approvals_get_pending_approval_count` | — | `number` | — | — | n/a (read) | Simple `COUNT(*) WHERE status = 'pending'` — feeds the sidebar/notification badge. |
| `approveRequest` | port (confirmed) | `approvals_approve_request` | `{ id: string, input?: ApprovalDecisionInput }` | `ApprovalRequest` | `approvalRequests`, activity | activity | not undoable — see §4 | `input` defaults to `{}` at the service layer (comment is optional on approve). Resolves `decidedBy`/`decidedByName` from the current session, same trust boundary as above. |
| `rejectRequest` | port (confirmed) | `approvals_reject_request` | `{ id: string, input: ApprovalDecisionInput }` | `ApprovalRequest` | `approvalRequests`, activity | activity | not undoable — see §4 | `input.comment` is **required** here (validated), unlike `approveRequest`'s optional comment — the two functions share one underlying `decideApproval(id, status, ...)` in the mock, and Rust's command layer should likewise share one internal function with `status` as the only real difference, not duplicate the whole decision-writing logic per command. |

## 2. DTOs → Rust

| Type | Field | Rust type | Column | Why |
|---|---|---|---|---|
| `ApprovalRequest` | `id` | `Uuid` | `UUID` | UUIDv7 (D2). |
| `ApprovalRequest` | `kind` | `enum ApprovalKind { Discount, WriteOff, BelowCost }` | `ENUM('discount','write_off','below_cost')` | `#[serde(rename_all = "snake_case")]` — literals already snake_case. |
| `ApprovalRequest` | `status` | `enum ApprovalStatus { Pending, Approved, Rejected }` | `ENUM('pending','approved','rejected')` | `#[serde(rename_all = "lowercase")]`. |
| `ApprovalRequest` | `value` | `Decimal` | `DECIMAL(19,4)` | "Discount %, write-off value, or the shortfall below cost" per the doc comment — a mixed-meaning field depending on `kind` (percentage for `discount`, money for `write_off`/`below_cost`). Recommend scale 4 to safely hold either without truncating a percentage's precision; no `round2`/`round4` call exists in the mock for this field at all (it's stored as given, purely for sorting/display) — so this is a storage-scale choice, not a rounding-rule one. |
| `ApprovalRequest` | `requestedBy` / `decidedBy` | `Uuid` / `Option<Uuid>` | `UUID` FK → `users.id` | — |
| `ApprovalRequest` | `requestedAt` / `decidedAt` | `DateTime<Utc>` | `DATETIME(3)` | Instants, not business-local dates — these are "when was this clicked," not an accounting date. |
| `ApprovalRequest` | `link` | `Option<RouteRef>` | JSON column, or nullable `(route_name, route_params)` pair | **Already typed as `AppRoute` in the frontend** (`link?: AppRoute` — the doc comment even calls out CLAUDE.md rule 25 implicitly by using the proper type). This is the **one module reviewed so far with zero path-string links** — nothing to fix in §8 for F7. |
| `ApprovalRequestInput` | (mirrors `ApprovalRequest` minus server-set fields) | — | — | No new mapping needed beyond the above — same field types, request-side. |
| `ApprovalDecisionInput` | `comment` | `Option<String>` | `TEXT`/`VARCHAR` on `decisionComment` | Trimmed server-side (`input.comment?.trim() || undefined`) — Rust must trim too, and treat an empty-after-trim string as absent, matching the mock exactly (an empty string is NOT stored as `""`). |

## 3. Validation and errors

| Function | Rule (source: mock check line) | Code | Exact Arabic message |
|---|---|---|---|
| `approveRequest` / `rejectRequest` | the request must exist (`findRequest`) | `NOT_FOUND` | `طلب الاعتماد غير موجود` |
| `approveRequest` / `rejectRequest` | the request must still be `pending` — a decision can't be made twice | `VALIDATION` | `تم اتخاذ قرار بشأن هذا الطلب مسبقاً` |
| `rejectRequest` | `input.comment` (trimmed) must be non-empty | `VALIDATION` | `أدخل سبب الرفض` |

No dedicated Zod schema for this module (confirmed no `validators/` folder under `src/modules/approvals`) — the three rules above are the entirety of server-side validation, all inline in `decideApproval`.

## 4. Undo matrix (every function that writes)

| Function | Undoable? | Compensation (existing fn) | Refused when | Period rule (D7) |
|---|---|---|---|---|
| `submitApprovalRequest` | No — but note this isn't really a gap: a pending request can simply be approved-then-nothing or rejected, which are its own natural terminal states, not an "undo" of the submission itself | — | n/a | n/a — not period-scoped, no ledger/stock touch at all |
| `approveRequest` | **No, and this is worth flagging explicitly**: once approved, there is no way to reverse the decision back to `pending` or to `rejected` (the `status !== 'pending'` guard blocks re-deciding in either direction). Whatever action the approval unblocked (a discount, a write-off, a below-cost sale) has **already happened by the time this queue enters the picture at all** — this module only records the *authorization* decision, never re-triggers or blocks the underlying document. So "undoing an approval" would mean undoing the discount/write-off/sale itself, which is entirely outside this module's writes and belongs to whichever module posted that document. **Not a gap in this module** — correctly out of scope, noted for clarity rather than left ambiguous. | none applicable | n/a | n/a |
| `rejectRequest` | Same reasoning as above — a rejection is terminal by design (the requester's dialog is expected to show "rejected" and the requester tries again through the normal synchronous path next time a manager is available). | none applicable | n/a | n/a |

This module has **no ledger/stock/numbering reach at all** (confirmed by the contract inventory's blank "Shared" column beyond `activity`) — it is purely a record-keeping queue with no accounting consequence of its own, which is why "undo" doesn't apply in the usual master-plan §3 rule 7 sense.

## 5. Concurrency under D8 (several terminals on one DB)

| Race | Rows | Settled by |
|---|---|---|
| Two managers on different terminals both try to decide the same pending request at once (e.g. one approves while the other rejects) | `approvalRequests` row | The `status !== 'pending'` check must run against a **locked read** (`SELECT ... FOR UPDATE`) inside the deciding transaction, so the second decision sees the first one's committed `status` and gets the "already decided" `VALIDATION` error, rather than both succeeding and leaving an inconsistent state (e.g. `decidedBy` overwritten by whichever commits last). This is the one real concurrency concern in an otherwise trivial module. |
| Two terminals submit approval requests concurrently | `approvalRequests` (insert-only) | No conflict possible — every submission is an independent new row (`uid('apr')` → UUIDv7 in Rust), never touching another row. |

## 6. Events and side effects

- **Activity/audit rows** written for: `submitApprovalRequest` (`activityKind: 'approval'`, message includes the Arabic `KIND_LABEL` for the approval kind), `approveRequest`/`rejectRequest` (same `activityKind`, message reflects the decision).
- **Events emitted:** `submitApprovalRequest` and `decideApproval` (both `approveRequest`/`rejectRequest`) emit `ledger:changed` — but this is **purely a cache-invalidation signal**, not an actual ledger write (confirmed: the mock's own comment says "cheapest existing event to invalidate the insight-engine cache / bell badge"). Under Rust, this should NOT be conflated with a real ledger-changed event from `shared::ledger` — it needs its own semantics (or the same event name reused deliberately, if the frontend's `onLedgerChanged` listener is meant to double as a generic "something dashboard-relevant changed" signal, which is what the mock currently exploits). **Flagged in §8** as worth a naming/design note before Part 02-F's event bridge is built, so the reused event name doesn't confuse a future reader into thinking approvals post to the GL.
- **Attachments/printing:** none.

## 7. Aggregations (reports / analytics / dashboard / insights only)

Not applicable — `getPendingApprovalCount` is a plain count, not an aggregation with grouping/filters/rounding worth a table here.

## 8. Contract fixes needed in the mock (01.C — reviewed 2026-09-27, nothing needed)

- [ ] **`ledger:changed` reused as a generic "dashboard cache" signal, not an actual ledger event** (see §6) — not a mock *bug*, but worth a design note for Part 02-F: when the Rust event bridge is built, decide whether `approvals_submit_approval_request`/`approvals_approve_request`/`approvals_reject_request` should emit the real `ledger:changed` (reusing the mock's shortcut) or a more accurately-named signal the frontend's cache-invalidation listener also subscribes to. **Carried forward as a Part 02-F design note, not a decision needed now** — either choice preserves current frontend behavior since `onLedgerChanged` already fires on this event; it's purely about naming clarity for future readers, not a functional gap.
- No validation, audit, error-code (F5), or path-string-link (F7) issues found in this module — it already uses `AppRoute` for its one link field, all three error paths use proper `ApiError` with correct codes, and every write is audited. **This module needed no mock fixes.**

## 9. Open questions (→ decisions in `00-MASTER-PLAN.md`)

None. This module is small and clean enough that nothing rises to the level of a decision — the one note in §8 is a Part 02-F implementation detail, not a fork in the road.

## Gate

- [x] Every inventory function is in §1 with a confirmed disposition (5/5, no overrides needed).
- [x] Every write function is in §4.
- [x] Every DTO field needing a non-default mapping is in §2.
- [x] `bun run contract:check` — no override applied for this module, contract already up to date (last verified as part of the `users.md` regeneration in this same session; no code or config change touches `approvals` since then).
