/**
 * General vouchers (docs/v2/09-purchases-payments-expenses.md §5) — money movements that aren't
 * invoices. Each posts a normal system journal entry with a source link and prints as a PDF
 * (generic voucher print route — pdfService only has the invoice template until Phase 11b).
 */
export type VoucherKind = 'RECEIPT' | 'PAYMENT' | 'TRANSFER' | 'OWNER';

/** OWNER voucher direction: owner takes money out (drawings) or puts money in (contribution). */
export type OwnerDirection = 'drawings' | 'contribution';

export interface VoucherBase {
  id: string;
  number: string;
  kind: VoucherKind;
  date: string;
  amount: number;
  description: string;
  note?: string;
  attachmentIds?: string[];
  costCenterId?: string;
  createdBy: string;
}

/** سند قبض عام — Dr method account / Cr chosen account (e.g. scrap sale, a refund from a government fee). */
export interface ReceiptVoucher extends VoucherBase {
  kind: 'RECEIPT';
  paymentMethodId: string;
  /** The account credited — any non-control account the user picks. */
  creditAccountId: string;
}

/** سند صرف عام — Dr chosen account / Cr method account (e.g. paying a one-off fee). */
export interface PaymentVoucher extends VoucherBase {
  kind: 'PAYMENT';
  paymentMethodId: string;
  /** The account debited — any non-control account the user picks. */
  debitAccountId: string;
}

/** تحويل بين الحسابات — Dr destination / Cr source, with an optional fee line (drawer → bank, bank → bank). */
export interface TransferVoucher extends VoucherBase {
  kind: 'TRANSFER';
  sourceAccountId: string;
  destinationAccountId: string;
  feeAmount?: number;
  /** The account the fee is expensed to (required when `feeAmount > 0`). */
  feeAccountId?: string;
}

/** مسحوبات / إضافة رأس مال — Dr drawings/Cr cash (drawings), or the reverse (contribution). */
export interface OwnerVoucher extends VoucherBase {
  kind: 'OWNER';
  direction: OwnerDirection;
  /** The cash/bank account touched. */
  cashAccountId: string;
}

export type Voucher = ReceiptVoucher | PaymentVoucher | TransferVoucher | OwnerVoucher;

export type ReceiptVoucherInput = Omit<ReceiptVoucher, 'id' | 'number' | 'createdBy' | 'kind'>;
export type PaymentVoucherInput = Omit<PaymentVoucher, 'id' | 'number' | 'createdBy' | 'kind'>;
export type TransferVoucherInput = Omit<TransferVoucher, 'id' | 'number' | 'createdBy' | 'kind'>;
export type OwnerVoucherInput = Omit<OwnerVoucher, 'id' | 'number' | 'createdBy' | 'kind'>;

export interface VoucherFilter {
  kind?: VoucherKind;
  from?: string;
  to?: string;
  search?: string;
}

// ---------------------------------------------------------------------------------------------
// Card/wallet settlement (docs/v2/09 §2 "Card settlement") — completes the follow-up Phase 3 left
// in src/mocks/backend/sales.ts.
// ---------------------------------------------------------------------------------------------

/** One day×method group of unsettled tenders, offered for selection in the settlement screen. */
export interface UnsettledTenderGroup {
  date: string;
  paymentMethodId: string;
  paymentMethodName: string;
  accountRole: 'cardClearing' | 'walletClearing';
  total: number;
  tenderCount: number;
}

export interface CardSettlementInput {
  date: string;
  /** `date`+`paymentMethodId` pairs being settled — one bank deposit can cover several days/methods of the same clearing account. */
  groups: { date: string; paymentMethodId: string }[];
  /** The amount the bank actually received. Fee = Σ selected tenders − depositAmount. */
  depositAmount: number;
  note?: string;
}

export interface CardSettlement {
  id: string;
  number: string;
  date: string;
  groups: { date: string; paymentMethodId: string; amount: number }[];
  grossAmount: number;
  depositAmount: number;
  feeAmount: number;
  note?: string;
  createdBy: string;
}
