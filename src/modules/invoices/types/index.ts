export type InvoiceStatus = 'DRAFT' | 'COMPLETED' | 'REFUNDED';
export type PaymentStatus = 'UNPAID' | 'PARTIALLY_PAID' | 'PAID';
export type SalePaymentMethod = 'cash' | 'card' | 'bank_transfer' | 'credit';

export interface InvoiceLine {
  id: string;
  productId: string;
  /** Extension: name snapshot so reprints survive product renames. */
  name: string;
  qty: number;
  price: number;
  costPrice: number;
  discount: number;
  /**
   * v2 (docs/v2/06-sales-and-pos.md §3): the tax charged on this line, snapshotted at sale time so
   * historical invoices never change if the tax's rate/category is edited later. Undefined = no tax
   * assigned to the line (falls back to out-of-scope, 0 VAT) — the product catalog doesn't carry a
   * per-product tax yet (that's Phase 6's "product form v2 tabs").
   */
  taxId?: string;
  taxCategory?: import('@/modules/settings/types').TaxCategory;
  taxRate?: number;
  /** v2: this line's VAT, computed by `computeInvoiceTotals` (net + vat = the line's gross). */
  net?: number;
  vat?: number;
  /** v2 phase 7 (docs/v2/06-sales-and-pos.md §1 "Unit picker"): the unit this line was sold in (Phase 6's `ProductUnit.id`), when the product has more than one unit. Undefined = the product's base unit. */
  unitId?: string;
  /** v2 phase 7: how many base units one of `unitId` contains, snapshotted at sale time (audit/receipt display; `qty` stays in the sold unit). */
  unitFactor?: number;
  /** v2 phase 7 (§1 "Custom price"): the catalog/list price before any override, kept for audit. Present only when `price` was hand-edited below it. */
  listPrice?: number;
  /** v2 phase 7: reason typed by the cashier when `price < listPrice` (required by the doc). */
  priceOverrideReason?: string;
  /** v2 phase 7 (§1 "Batch: auto-picked FEFO... can be changed"): the batch this line drew from, for batch-tracked products. */
  batchId?: string;
  batchNo?: string;
  /** Free-text service line not tied to a catalog product (desk invoice form) — `productId` is a synthetic id in this case. */
  isFreeText?: boolean;
  /** Free-text line's revenue account (required when `isFreeText`). */
  revenueAccountId?: string;
}

/**
 * v2 split payment (docs/v2/06-sales-and-pos.md §1 "Pay (F12)… Split payment"): one line per
 * payment method used to settle the sale. `paymentMethod`/`paidAmount` on `Invoice`/`SaleInput`
 * stay as the legacy single-tender snapshot (still used by receipts/reports outside this phase's
 * surface) — `tenders`, when present, is the source of truth for posting (src/mocks/backend/sales.ts).
 */
export interface Tender {
  paymentMethodId: string;
  amount: number;
  /** Card last 4 / approval code / transfer reference. */
  reference?: string;
}

export interface Invoice {
  id: string;
  number: string;
  date: string;
  customerId?: string;
  cashierId: string;
  status: InvoiceStatus;
  paymentStatus: PaymentStatus;
  lines: InvoiceLine[];
  subTotal: number;
  discountRate: number;
  /** Extension: the money value of `discountRate` (plus line discounts). */
  discountAmount: number;
  /** Extension: VAT rate snapshot used for this invoice. */
  taxRate: number;
  taxAmount: number;
  grandTotal: number;
  paymentMethod: SalePaymentMethod;
  paidAmount: number;
  /**
   * v2 split payment (docs/v2/06-sales-and-pos.md §1): the actual tenders posted, one per payment
   * method. Absent on invoices recorded through the legacy single-method path (`paymentMethod` +
   * `paidAmount` only) — `sales.ts` falls back to a single implied tender in that case.
   */
  tenders?: Tender[];
  /** Extension: sum of refunds issued against this invoice. */
  refundedAmount: number;
  /** Extension: cash handed over by the customer (cash sales), for the receipt. */
  tenderedAmount?: number;
  /**
   * v2 (docs/v2/02-accounting-review.md D3): for a credit/partial sale, `date` + the customer's
   * `paymentTermsDays` at the time of sale — drives aging and "overdue". Set by
   * `modules/invoices/services/invoiceService.ts::createSale`, not the posting rule itself.
   */
  dueDate?: string;
  note?: string;
  /** v2 phase 7: 'POS' (till) or 'DESK' (accountant's full invoice form) — drives the invoice-list source filter. */
  source?: 'POS' | 'DESK';
  /** v2 phase 7 (§5 shifts): the shift this sale's cash/tenders were recorded against, when sold from the till. */
  shiftId?: string;
  /** v2 phase 7 (real branch since phase 9 — see docs/v2/10 §1); branch-prefixed numbering follows `number`. */
  branchId?: string;
  /** v2 phase 7 (§2 desk form "Invoice type is automatic"). */
  invoiceType?: 'STANDARD' | 'SIMPLIFIED';
  /** v2 phase 7 (§2): buyer PO reference, notes/terms text. */
  poReference?: string;
  terms?: string;
  attachmentIds?: string[];
  /**
   * v2 phase 9 currency (docs/v2/10-branches-currencies-cost-centers.md §2): set only when the
   * customer's currency differs from the base currency. `lines[]`/totals stay in this currency;
   * `exchangeRate` (base per 1 unit, snapshotted at sale time) converts them for posting. Undefined
   * = base currency (the overwhelmingly common case, and the only one before this phase).
   */
  currency?: string;
  exchangeRate?: number;
  /** v2 phase 9: cost center for the sale's revenue/COGS lines (defaults to the branch's cost center). */
  costCenterId?: string;
}

/**
 * v2 phase 7 (§2 "Quotations"): same shape as `Invoice` but never posts to the ledger or touches
 * stock until converted. Kept as its own entity (not an `Invoice` with a DRAFT-like status) so the
 * invoice list's invariants (every posted document has one journal entry) never have to special-case it.
 */
export type QuotationStatus = 'DRAFT' | 'SENT' | 'ACCEPTED' | 'REJECTED' | 'EXPIRED';

export interface Quotation {
  id: string;
  number: string;
  date: string;
  expiryDate?: string;
  customerId?: string;
  salespersonId: string;
  status: QuotationStatus;
  lines: InvoiceLine[];
  discountRate: number;
  discountAmount: number;
  taxAmount: number;
  subTotal: number;
  grandTotal: number;
  note?: string;
  terms?: string;
  poReference?: string;
  attachmentIds?: string[];
  /** Set once "Convert → invoice" runs. */
  convertedInvoiceId?: string;
}

export interface QuotationInput {
  customerId?: string;
  expiryDate?: string;
  lines: SaleInput['lines'];
  discountRate: number;
  note?: string;
  terms?: string;
  poReference?: string;
}

// =================================================================================================
// v2 phase 7 §1/§5 — Held sales (POS "F6") and shifts (docs/v2/06-sales-and-pos.md §5)
// =================================================================================================

/** A parked POS cart, per terminal, surviving a reload (persist.ts snapshots `db.heldSales` like any other table). */
export interface HeldSale {
  id: string;
  label?: string;
  terminalId: string;
  heldAt: string;
  heldBy: string;
  customerId?: string;
  discountRate: number;
  discountIsPct: boolean;
  note?: string;
  lines: {
    productId: string;
    unitId?: string;
    qty: number;
    price: number;
    listPrice?: number;
    priceOverrideReason?: string;
    discount?: number;
    discountIsPct?: boolean;
    batchId?: string;
    taxId?: string;
  }[];
}

export type ShiftStatus = 'OPEN' | 'CLOSED';

export interface DenominationCount {
  value: number;
  count: number;
}

/** One cash/tender event during a shift (docs/v2/06 §5 "Every cash tender, cash refund, pay-in, pay-out and bank drop"). */
export type ShiftMovementKind = 'SALE_CASH' | 'REFUND_CASH' | 'PAY_IN' | 'PAY_OUT' | 'BANK_DROP';

export interface ShiftMovement {
  id: string;
  kind: ShiftMovementKind;
  amount: number;
  note?: string;
  refId?: string;
  refNumber?: string;
  at: string;
  by: string;
}

export interface Shift {
  id: string;
  number: string;
  terminalId: string;
  branchId?: string;
  status: ShiftStatus;
  openedBy: string;
  openedAt: string;
  openingFloat: number;
  openingDenominations?: DenominationCount[];
  movements: ShiftMovement[];
  closedBy?: string;
  closedAt?: string;
  countedCash?: number;
  closingDenominations?: DenominationCount[];
  expectedCash?: number;
  variance?: number;
  /** 'HANDOVER' (default, no posting) or 'DROP' (posts a cash-drawer → safe/bank transfer). */
  handoverMode?: 'HANDOVER' | 'DROP';
  forceClosedBy?: string;
  note?: string;
}

export interface OpenShiftInput {
  terminalId: string;
  branchId?: string;
  openingFloat: number;
  openingDenominations?: DenominationCount[];
}

export interface CloseShiftInput {
  countedCash: number;
  closingDenominations?: DenominationCount[];
  handoverMode?: 'HANDOVER' | 'DROP';
  note?: string;
}

export interface InvoiceFilter {
  search?: string;
  status?: InvoiceStatus;
  paymentStatus?: PaymentStatus;
  customerId?: string;
  from?: string;
  to?: string;
  /** v2 phase 7 (§6 "Invoice list v2" filters). */
  source?: 'POS' | 'DESK';
  invoiceType?: 'STANDARD' | 'SIMPLIFIED';
  cashierId?: string;
  overdueOnly?: boolean;
  minAmount?: number;
  maxAmount?: number;
}

export interface SaleInput {
  customerId?: string;
  /**
   * `taxId`: v2 per-line tax (docs/v2/06-sales-and-pos.md §3) — omitted falls back to the store's
   * default sales tax (`db.settings.defaultTaxId`), matching the pre-Phase-3 single-rate behavior.
   * `discountIsPct`: line discount is a % of the line when true, a flat amount otherwise (defaults
   * to flat amount, matching the pre-existing `discount` field's meaning).
   */
  lines: {
    productId: string;
    qty: number;
    price: number;
    discount?: number;
    discountIsPct?: boolean;
    taxId?: string;
    /** v2 phase 7: unit sold in + its base-unit factor (Phase 6's `ProductUnit`). */
    unitId?: string;
    unitFactor?: number;
    /** v2 phase 7: catalog/list price before an override, and the required reason when priced below it. */
    listPrice?: number;
    priceOverrideReason?: string;
    /** v2 phase 7: batch drawn from (FEFO auto-pick or manual override). */
    batchId?: string;
    batchNo?: string;
    isFreeText?: boolean;
    revenueAccountId?: string;
    name?: string;
  }[];
  discountRate: number;
  /** v2 phase 7: flat invoice-discount amount, used instead of `discountRate` when set (POS Shift+F8 "amount"). */
  discountAmount?: number;
  paymentMethod: SalePaymentMethod;
  /** Cash tendered / amount paid now. Ignored for card & bank transfer (paid in full). */
  paidAmount: number;
  tenderedAmount?: number;
  /**
   * v2 split payment (docs/v2/06-sales-and-pos.md §1 "Split payment"): part cash + part card in one
   * sale. When present, `sales.ts` posts one line per tender (each to its method's account) instead
   * of the single `paymentMethod`/`paidAmount` pair; the sum of `tenders[].amount` must equal
   * `paidAmount`. Optional so every existing caller (POS cart, CheckoutModal) keeps working unchanged.
   */
  tenders?: import('.').Tender[];
  note?: string;
  /** v2 phase 7: 'POS' vs 'DESK' — see `Invoice.source`. Defaults to 'POS' in `sales.ts` when omitted. */
  source?: 'POS' | 'DESK';
  shiftId?: string;
  invoiceType?: 'STANDARD' | 'SIMPLIFIED';
  dueDateOverride?: string;
  poReference?: string;
  terms?: string;
  attachmentIds?: string[];
  managerApprovedBy?: string;
  /** v2 phase 9: real branch (docs/v2/10 §1) — omitted falls back to the caller's default/home branch. */
  branchId?: string;
  costCenterId?: string;
  /** v2 phase 9 currency: FC sale — `lines[].price`/totals are all in this currency; `exchangeRate` converts to base for posting. */
  currency?: string;
  exchangeRate?: number;
}

/** v2 phase 7 (§4): refund method after the outstanding balance is settled first. */
export type RefundMethod = 'cash' | 'card' | 'bank_transfer' | 'customer_credit';

export interface Refund {
  id: string;
  number: string;
  invoiceId: string;
  date: string;
  reason?: string;
  lines: { invoiceLineId: string; qty: number; restock?: boolean }[];
  /** Extension: net (before VAT) and VAT portions of the refund. */
  subTotal: number;
  taxAmount: number;
  grandTotal: number;
  /** Extension: how much reduced the customer's receivable vs. was paid back. */
  settledToReceivable: number;
  cashBack: number;
  /** v2 phase 7: how `cashBack` was paid out — defaults to the original invoice's method when omitted (legacy callers). */
  refundMethod?: RefundMethod;
  /** v2 phase 7: portion of `cashBack` that went to customer_credit (unallocated, usable on a future invoice) instead of a cash/card/bank payout. */
  creditedToAccount?: number;
}

export interface RefundInput {
  invoiceId: string;
  reason?: string;
  lines: { invoiceLineId: string; qty: number; restock?: boolean }[];
  refundMethod?: RefundMethod;
}

/** Preview of the double-entry posting a sale will produce. */
export interface JournalPreviewLine {
  accountCode: string;
  accountName: string;
  debit: number;
  credit: number;
}
