import type { PaymentStatus } from '@/modules/invoices/types';

/** v2 (docs/v2/09-purchases-payments-expenses.md §1): DRAFT → (optional) ORDERED → RECEIVED (posted), or CANCELED. */
export type PurchaseStatus = 'DRAFT' | 'ORDERED' | 'RECEIVED' | 'CANCELED';

/** Kept for callers that still read `PurchaseStatus` from before v2 (RECEIVED replaces the old CONFIRMED). Back-compat alias. */
export const LEGACY_CONFIRMED_STATUS: PurchaseStatus = 'RECEIVED';

export interface PurchaseLine {
  productId: string;
  /** Ordered/invoiced qty, in the line's `unitId` (base unit when `unitId` is omitted — legacy rows). */
  qty: number;
  /** Unit cost in the line's `unitId`. */
  costPrice: number;
  /** v2 §1 "units" — the product's `ProductUnit.id`. Omitted = base unit (legacy rows, 1:1 with stock). */
  unitId?: string;
  /** How many base (stock) units one of `unitId` contains. 1 when `unitId` is omitted. Snapshotted at save time so a later unit-factor edit never changes a posted document. */
  unitFactor?: number;
  /** v2 §1 line discount: % of qty×price when `discountIsPct`, else a flat amount. */
  discount?: number;
  discountIsPct?: boolean;
  /** v2 §1 per-line tax id (`db.taxes`) — falls back to the store's default purchase tax when omitted. */
  taxId?: string;
  /** Received qty so far, in base (stock) units — filled by the receiving flow. Undefined until first received. */
  receivedQty?: number;
  /** v2 §1 batch/expiry capture for tracked products, filled at receiving time (one batch per line — see `ReceiveLineInput` for multi-batch). */
  batchNo?: string;
  expiryDate?: string;
  /** v2 landed costs: this line's share of freight/customs/clearing, in the base currency — added to the line's posted inventory cost. Filled at receiving/posting time. */
  landedCostShare?: number;
}

/** v2 §1 landed costs — freight/customs/clearing rows, spread over stock lines by value or qty. */
export type LandedCostSpread = 'value' | 'qty';

export interface LandedCostLine {
  id: string;
  label: string;
  amount: number;
  /** Optional different supplier (e.g. the shipping company) — omitted = same supplier as the purchase, added to its AP total. */
  supplierId?: string;
  spreadBy: LandedCostSpread;
}

export interface PurchaseOrder {
  id: string;
  number: string;
  supplierId: string;
  date: string;
  status: PurchaseStatus;
  lines: PurchaseLine[];
  subTotal: number;
  taxRate: number;
  taxAmount: number;
  grandTotal: number;
  paymentStatus: PaymentStatus;
  /** Extensions mirroring Invoice, needed to compute the supplier balance. */
  paidAmount: number;
  returnedAmount: number;
  note?: string;

  // --- v2 phase 8 additions (docs/v2/09-purchases-payments-expenses.md §1) ---------------------

  /** v2 §1 invoice-level discount, applied on top of line discounts (same math as sales — totals.ts). */
  invoiceDiscount?: { pct?: number; amount?: number };
  /** v2 §1 landed costs (freight/customs/clearing). */
  landedCosts?: LandedCostLine[];
  /** v2 §1 "بعد الاستلام… رقم فاتورة المورد وتاريخها" — a warning banner shows while these are missing. */
  supplierInvoiceNo?: string;
  supplierInvoiceDate?: string;
  /** v2 §1 non-VAT supplier handling: true when the supplier had no VAT number at receiving time — input VAT was NOT claimed and was instead added to the stock/expense cost (review E3). */
  vatNotRecoverable?: boolean;
  /** Set once "إرسال للمورد" has printed/exported the PO. */
  sentAt?: string;
  /** v2 §1 short delivery → "إنشاء أمر متبقٍ": this order is the backorder draft for the given original order's shortfall. */
  backorderOfId?: string;
  /** The date goods were actually received/posted (RECEIVED status), separate from the order `date`. */
  receivedDate?: string;
  attachmentIds?: string[];
  /** docs/v2/09 §6 — inert until phase 9 (branch's default cost center), just the field for now. */
  costCenterId?: string;
}

export interface PurchaseLineInput {
  productId: string;
  qty: number;
  costPrice: number;
  unitId?: string;
  unitFactor?: number;
  discount?: number;
  discountIsPct?: boolean;
  taxId?: string;
}

export interface LandedCostLineInput {
  label: string;
  amount: number;
  supplierId?: string;
  spreadBy: LandedCostSpread;
}

export interface PurchaseOrderInput {
  supplierId: string;
  date: string;
  lines: PurchaseLineInput[];
  note?: string;
  invoiceDiscount?: { pct?: number; amount?: number };
  landedCosts?: LandedCostLineInput[];
  supplierInvoiceNo?: string;
  supplierInvoiceDate?: string;
  attachmentIds?: string[];
  costCenterId?: string;
  /** true = post immediately (stock + journal, status RECEIVED); false = save as DRAFT. Superseded by `order`/`receive` flows below for the v2 order→receive split, kept for the "confirm directly" quick path. */
  confirm: boolean;
}

/** v2 §1 "إرسال للمورد" — moves a DRAFT to ORDERED and stamps `sentAt`. */
export interface SendToSupplierResult {
  po: PurchaseOrder;
}

/** v2 §2 receiving screen — one line's received qty (base units) + batch capture. Several batches per line are allowed (docs/v2/07 §3). */
export interface ReceiveLineInput {
  productId: string;
  /** Total qty received for this line, in base (stock) units. */
  receivedQty: number;
  batches?: { batchNo: string; expiryDate?: string; qty: number }[];
}

export interface ReceivePurchaseInput {
  date: string;
  lines: ReceiveLineInput[];
  supplierInvoiceNo?: string;
  supplierInvoiceDate?: string;
  /** true when the supplier has no VAT number — input VAT isn't claimed; it's added to cost instead (review E3). */
  vatNotRecoverable?: boolean;
  landedCosts?: LandedCostLineInput[];
  /** v2 §1 "إنشاء أمر متبقٍ" — when true and the receipt is short, a DRAFT backorder PO is created for the shortfall. */
  createBackorder?: boolean;
}

export interface PurchaseFilter {
  search?: string;
  status?: PurchaseStatus;
  paymentStatus?: PaymentStatus;
  supplierId?: string;
}

/** v2 (E1): how the supplier gives the money back — cash/bank hit their own account; credit stays on AP. */
export type RefundMethod = 'cash' | 'bank_transfer' | 'credit';

export interface DebitNoteLine {
  productId: string;
  qty: number;
  costPrice: number;
  /** v2 §4 batch picking for tracked products — which batch(es) this return draws down. */
  batchId?: string;
}

export interface PurchaseReturn {
  id: string;
  number: string;
  purchaseOrderId: string;
  supplierId: string;
  date: string;
  reason?: string;
  lines: DebitNoteLine[];
  subTotal: number;
  taxAmount: number;
  grandTotal: number;
  settledToPayable: number;
  cashBack: number;
  /** v2 (E1): the method `cashBack` was actually refunded through (never hard-coded to cash any more). */
  refundMethod: RefundMethod;
  /** v2 §4 — set when this debit note was created from Phase 6's "return expiring batch" expiry-report shortcut / `debitNoteDrafts`. */
  fromDraftId?: string;
}

export interface PurchaseReturnInput {
  purchaseOrderId: string;
  reason?: string;
  /** v2 (E1): defaults to 'credit' (stays on the supplier's account) when omitted. */
  refundMethod?: RefundMethod;
  lines: { productId: string; qty: number; batchId?: string }[];
  fromDraftId?: string;
}
