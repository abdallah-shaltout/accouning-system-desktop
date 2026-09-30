---
id: ACC-0006
kind: accounting
status: verified
area: products
first_seen: 2026-09-28
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0006-draft-adjustment-approval.json
---

## اعتماد مسودة تسوية مخزون يتجاوز حد الاعتماد دون تأكيد المدير (G-31c)

A stock-in or write-off at or above `settings.inventoryApprovalThreshold` needs a manager's approval
before it posts. A direct adjustment checked this, but the rule could be bypassed by saving a draft
(drafts post nothing, so they are not checked) and then completing it: `completeStockAdjustment`
(`src/mocks/backend/inventory.ts`) posted the draft without calling `assertApproval`. The Rust port
(`complete_adjustment`, plan 21 part 03, 06b Q-I1/Q-I2) had copied this.

**Rule now:** completing a draft posts it, so it passes the same approval check as a direct
adjustment. The check runs on the lines rebuilt at completion. STOCKTAKE is exempt, the same as a
direct adjustment. Above the threshold, a missing approver is refused with the same `FORBIDDEN`
message. On the Rust side, the approver is also refused if its manager-PIN grant is not live
(`AppState.approval_grants`) or it is not an active admin or manager, the same as the direct path.
`completeAdjustment(id, approvedBy?)` / `products_complete_adjustment { id, approvedBy? }` take the
approver. `StockAdjustmentDetailPage.vue` opens the shared `ApprovalPinDialog` when a draft is above
the threshold, and also when the server refuses with `FORBIDDEN`. The approver and time are stored
on the adjustment.

### خطوات إعادة الإنتاج

1. Set `inventoryApprovalThreshold = 1000`, and add a product with cost 10.
2. Save a STOCK_IN draft (reason `found`) of 200 units (value 2000) with no approver.
3. Complete it without `approvedBy`. Old code: it posts. New code: `قيمة هذه الحركة (2000.00) تتجاوز حد
   الاعتماد (1000.00) — يلزم تأكيد المدير`. Completing it again with an approver posts it.

`bun run verify:replay` replays this as `scripts/verify/cases/ACC-0006-draft-adjustment-approval.json`
(step 0: complete without an approver → refused, recorded `ok: false`; step 1: complete with an
approver → posts). On the old code, step 0 succeeds, so the case breaks there.

### ما جُرِّب ولم ينجح

- Reusing the `approvedBy` saved on the draft: drafts never store an approver (the form sends none
  for a draft), and a stored id would skip the time-limited PIN grant on the Rust side. So the
  approver is supplied when the draft is completed.

### الملفات ذات الصلة

- `src/mocks/backend/inventory.ts` — `completeStockAdjustment`, `assertApproval`.
- `src/modules/products/services/inventoryService.ts` — `completeAdjustment`.
- `src/modules/products/pages/StockAdjustmentDetailPage.vue` — the PIN dialog on completion.
- `src-tauri/src/domains/products/service/adjustments.rs` — `complete_adjustment`, `assert_approval`.
- `src-tauri/src/domains/products/commands/inventory.rs` — `products_complete_adjustment`.
- `src-tauri/src/domains/products/dto/inventory.rs` — `ProductsCompleteAdjustmentArgs.approved_by`.
- `src-tauri/tests/domain_products.rs` — `completing_draft_above_threshold_requires_granted_manager`.
