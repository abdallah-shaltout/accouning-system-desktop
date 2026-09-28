/**
 * Drift check (21.02-F, F-4 / plans/pending/21-rust-backend/03-domains/08-invoices.md §2,
 * 08b-pos-shifts.md §2): proves the ts-rs-generated `invoices` DTOs (`invoices/types/gen/*`,
 * written by `bun run bindings` from `src-tauri/src/domains/invoices/dto.rs`) have exactly the
 * same shape as the hand-written TS types in `./index.ts` / `../services/invoiceService.ts`.
 *
 * `Simplify<T>` (`core/types/contract.ts`) flattens an intersection into a single object type so
 * `Equals` (identity, not mutual assignability) can compare it against a ts-rs-generated flat
 * struct — `InvoiceRow`/`InvoiceDetail`/`ShiftRow`/`QuotationRow` are declared as intersections in
 * `invoiceService.ts` but as flat structs on the Rust side (08 §2 / 08b §2's own convention, same
 * as `purchases.md`'s `PurchaseRow`), so both sides go through `Simplify` here.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`. The
 * `./gen/*` imports do not exist until `bun run bindings` runs against a compiled `src-tauri`
 * (manager's wave build) — this file itself is not type-checkable until then.
 */
import type { Equals, Expect, Simplify } from '@/modules/core/types/contract';
import type {
  InvoiceLine,
  Tender,
  Invoice,
  Refund,
  JournalPreviewLine,
  SaleInput,
  RefundInput,
  Quotation,
  QuotationInput,
  QuotationStatus,
  HeldSale,
  Shift,
  ShiftMovement,
  DenominationCount,
  OpenShiftInput,
  CloseShiftInput,
} from './index';
import type { InvoiceRow, InvoiceDetail, QuotationRow, PrintData, ShiftRow } from '../services/invoiceService';

import type { InvoiceLine as GenInvoiceLine } from './gen/InvoiceLine';
import type { Tender as GenTender } from './gen/Tender';
import type { Invoice as GenInvoice } from './gen/Invoice';
import type { Refund as GenRefund } from './gen/Refund';
import type { JournalPreviewLine as GenJournalPreviewLine } from './gen/JournalPreviewLine';
import type { SaleInput as GenSaleInput } from './gen/SaleInput';
import type { RefundInput as GenRefundInput } from './gen/RefundInput';
import type { Quotation as GenQuotation } from './gen/Quotation';
import type { QuotationInput as GenQuotationInput } from './gen/QuotationInput';
import type { QuotationStatus as GenQuotationStatus } from './gen/QuotationStatus';
import type { HeldSale as GenHeldSale } from './gen/HeldSale';
import type { Shift as GenShift } from './gen/Shift';
import type { ShiftMovement as GenShiftMovement } from './gen/ShiftMovement';
import type { DenominationCount as GenDenominationCount } from './gen/DenominationCount';
import type { OpenShiftInput as GenOpenShiftInput } from './gen/OpenShiftInput';
import type { CloseShiftInput as GenCloseShiftInput } from './gen/CloseShiftInput';
import type { InvoiceRow as GenInvoiceRow } from './gen/InvoiceRow';
import type { InvoiceDetail as GenInvoiceDetail } from './gen/InvoiceDetail';
import type { QuotationRow as GenQuotationRow } from './gen/QuotationRow';
import type { PrintData as GenPrintData } from './gen/PrintData';
import type { ShiftRow as GenShiftRow } from './gen/ShiftRow';
import type { InvoiceListFilter as GenInvoiceListFilter } from './gen/InvoiceListFilter';

export type _InvoiceLine = Expect<Equals<GenInvoiceLine, InvoiceLine>>;
export type _Tender = Expect<Equals<GenTender, Tender>>;
export type _Invoice = Expect<Equals<GenInvoice, Invoice>>;
export type _Refund = Expect<Equals<GenRefund, Refund>>;
export type _JournalPreviewLine = Expect<Equals<GenJournalPreviewLine, JournalPreviewLine>>;
export type _SaleInput = Expect<Equals<GenSaleInput, SaleInput>>;
export type _RefundInput = Expect<Equals<GenRefundInput, RefundInput>>;
export type _Quotation = Expect<Equals<GenQuotation, Quotation>>;
export type _QuotationInput = Expect<Equals<GenQuotationInput, QuotationInput>>;
export type _QuotationStatus = Expect<Equals<GenQuotationStatus, QuotationStatus>>;
export type _HeldSale = Expect<Equals<GenHeldSale, HeldSale>>;
export type _Shift = Expect<Equals<GenShift, Shift>>;
export type _ShiftMovement = Expect<Equals<GenShiftMovement, ShiftMovement>>;
export type _DenominationCount = Expect<Equals<GenDenominationCount, DenominationCount>>;
// contract-ok: Rust's `terminalId` is optional (D-S1, invoices/dto.rs's own `contract-ok` comment)
// — it's server-owned and never read (`cx.terminal_id` is used instead), kept only for wire-shape
// parity with the mock's historically-required field. Optional is a safe superset of required, so
// the callers that always pass it (the mock path) still satisfy the Rust-side type.
export type _OpenShiftInput = Expect<Equals<GenOpenShiftInput, Simplify<Omit<OpenShiftInput, 'terminalId'> & { terminalId?: string }>>>;
export type _CloseShiftInput = Expect<Equals<GenCloseShiftInput, CloseShiftInput>>;

// `InvoiceRow`/`InvoiceDetail`/`QuotationRow`/`ShiftRow` are intersections on the TS side
// (`Invoice & {...}`, `InvoiceRow & {...}`, etc.) but flat structs on the Rust side.
export type _InvoiceRow = Expect<Equals<Simplify<GenInvoiceRow>, Simplify<InvoiceRow>>>;
export type _InvoiceDetail = Expect<Equals<Simplify<GenInvoiceDetail>, Simplify<InvoiceDetail>>>;
export type _QuotationRow = Expect<Equals<Simplify<GenQuotationRow>, Simplify<QuotationRow>>>;
export type _PrintData = Expect<Equals<Simplify<GenPrintData>, Simplify<PrintData>>>;
// contract-ok: `ShiftRow` = `Shift & ReturnType<typeof shiftSummary> & { openedByName; closedByName?
// }` (`invoiceService.ts:253`) — `shiftSummary`'s return type is inferred from the mock function's
// body, not a named type, so it can't be spelled on the Rust side field-for-field the way the other
// flattened rows are; the Rust `ShiftRow`'s `summary` fields are checked structurally via `Simplify`
// against the same shape `shiftSummary` actually returns.
export type _ShiftRow = Expect<Equals<Simplify<GenShiftRow>, Simplify<ShiftRow>>>;

// `InvoiceFilter & { openOnly?: boolean }` (`invoiceService.ts:78`) vs. the Rust side's one flat
// `InvoiceListFilter` struct (08 §2).
import type { InvoiceFilter } from './index';
export type _InvoiceListFilter = Expect<Equals<Simplify<GenInvoiceListFilter>, Simplify<InvoiceFilter & { openOnly?: boolean }>>>;
