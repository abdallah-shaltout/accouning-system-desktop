/**
 * Approvals (docs/v2/14-platform.md §6 "Approvals page (`/approvals`) for managers"). Two paths
 * exist for a manager approval, and this module is only the second one:
 *
 *   1. **Synchronous PIN approval** — already fully built (Phase 6/7): `DiscountDialog.vue`
 *      (discount over `pos.maxDiscountPct`), `CustomPriceDialog.vue` (price below cost, when
 *      `pos.sellBelowCost` allows asking at all) and `ApprovalPinDialog.vue` (stock-in/write-off
 *      above `settings.inventoryApprovalThreshold`) all block the action right there until a
 *      manager types their own username+password. This is the common case and stays exactly as
 *      it is — it needs no queue, because a manager is standing next to the cashier/storekeeper.
 *   2. **Async queue (this module)** — for when the synchronous path can't complete because no
 *      manager is physically present to type a PIN. Each of those three dialogs' "no manager
 *      here" escape hatch creates an `ApprovalRequest` instead of blocking the sale/adjustment;
 *      the request is queued here for any manager/admin to decide later, from `/approvals` or the
 *      notifications drawer. This is the gap the doc's "for managers" page actually fills — not a
 *      redundant parallel system, since it only ever fires when the synchronous path is unusable.
 */
import type { AppRoute } from '@/modules/core/types/route';

export type ApprovalKind = 'discount' | 'write_off' | 'below_cost';

export type ApprovalStatus = 'pending' | 'approved' | 'rejected';

export interface ApprovalRequest {
  id: string;
  kind: ApprovalKind;
  /** Arabic summary shown in the list, e.g. "خصم 25% على فاتورة عميل نقدي". */
  summary: string;
  /** The value at stake — discount %, write-off value, or the shortfall below cost — for sorting/display. */
  value: number;
  /** Free-form context the requester leaves (why they're asking). */
  requestNote?: string;
  requestedBy: string;
  requestedByName: string;
  requestedAt: string;
  status: ApprovalStatus;
  decidedBy?: string;
  decidedByName?: string;
  decidedAt?: string;
  /** Manager's comment on approve/reject — required on reject, optional on approve. */
  decisionComment?: string;
  /** Deep-link back to the document/screen this request is about (POS held sale, adjustment draft…). */
  link?: AppRoute;
}

export interface ApprovalRequestInput {
  kind: ApprovalKind;
  summary: string;
  value: number;
  requestNote?: string;
  link?: AppRoute;
}

export interface ApprovalDecisionInput {
  comment?: string;
}
