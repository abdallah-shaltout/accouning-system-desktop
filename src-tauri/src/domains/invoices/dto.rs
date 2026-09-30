//! `invoices` DTOs (03-domains/08-invoices.md §2, 08b-pos-shifts.md §2) — mirrors
//! `src/modules/invoices/types/index.ts` field-for-field. Conventions per `core/dto.rs` header
//! (P2-33): `camelCase`; `Option` fields `skip_serializing_none` + `#[ts(optional)]` (absent, never
//! `null`); `Decimal` via `utils::money::serde_number` + `#[ts(type = "number")]`; `Id` as
//! `#[ts(type = "string")]`; document dates are `String` = `DocDate::key()`.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::domains::parties::dto::Customer;
use crate::domains::payments::dto::Payment;
use crate::domains::settings::dto::{StoreSettings, TaxCategory};
use crate::utils::id::Id;
use crate::utils::money::serde_number;

// --- Small enums (invoices, 08 §2) --------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "invoices/types/gen/")]
pub enum InvoiceStatus {
    Draft,
    Completed,
    Refunded,
}

/// Reused by both `Invoice.paymentStatus` and `Shift`'s nothing — the invoices-owned copy of the
/// 3-way payment-status enum (purchases has its own identical copy, same reasoning as that domain's
/// doc comment: avoids a cross-domain dependency just for an enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "invoices/types/gen/")]
pub enum PaymentStatus {
    Unpaid,
    PartiallyPaid,
    Paid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "invoices/types/gen/")]
pub enum SalePaymentMethod {
    Cash,
    Card,
    BankTransfer,
    Credit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
#[ts(export_to = "invoices/types/gen/")]
pub enum InvoiceSource {
    Pos,
    Desk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
#[ts(export_to = "invoices/types/gen/")]
pub enum InvoiceType {
    Standard,
    Simplified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "invoices/types/gen/")]
pub enum RefundMethod {
    Cash,
    Card,
    BankTransfer,
    CustomerCredit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
#[ts(export_to = "invoices/types/gen/")]
pub enum QuotationStatus {
    Draft,
    Sent,
    Accepted,
    Rejected,
    Expired,
}

// --- Lines / tenders -----------------------------------------------------------------------------

/// `InvoiceLine` (`types/index.ts:5-41`). `product_id` is the UUID text, or `freetext-{position}`
/// when the row's `product_id` is `NULL` and `is_free_text` (D-I2).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoiceLine {
    pub id: String,
    pub product_id: String,
    pub name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub price: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cost_price: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub discount: Decimal,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_category: Option<TaxCategory>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_rate: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat: Option<Decimal>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_factor: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_price: Option<Decimal>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_override_reason: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_no: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_free_text: Option<bool>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revenue_account_id: Option<Id>,
}

/// `Tender` (`:49-54`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct Tender {
    #[ts(type = "string")]
    pub payment_method_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

// --- Invoice ---------------------------------------------------------------------------------

/// `Invoice` (`:56-114`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct Invoice {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    /// `DocDate` key string.
    pub date: String,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<Id>,
    #[ts(type = "string")]
    pub cashier_id: Id,
    pub status: InvoiceStatus,
    pub payment_status: PaymentStatus,
    pub lines: Vec<InvoiceLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub sub_total: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub discount_rate: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub discount_amount: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub tax_rate: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub tax_amount: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub grand_total: Decimal,
    pub payment_method: SalePaymentMethod,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub paid_amount: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenders: Option<Vec<Tender>>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub refunded_amount: Decimal,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tendered_amount: Option<Decimal>,
    /// `DocDate` key string of the due date, when one is set.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<InvoiceSource>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shift_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_type: Option<InvoiceType>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub po_reference: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_ids: Option<Vec<String>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_rate: Option<Decimal>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_center_id: Option<Id>,
}

/// `InvoiceRow` (`invoiceService.ts:35`) — flatten `Invoice` + `customerName?`, `cashierName`,
/// `outstanding`.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoiceRow {
    #[serde(flatten)]
    #[ts(flatten)]
    pub invoice: Invoice,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_name: Option<String>,
    pub cashier_name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub outstanding: Decimal,
}

/// `JournalRef` (`invoiceService.ts:41` shape, reused across invoice/purchase detail DTOs).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct JournalRef {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    pub description: String,
}

/// `InvoiceDetail` (`invoiceService.ts:37-44`) — flatten `InvoiceRow` + detail fields.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoiceDetail {
    #[serde(flatten)]
    #[ts(flatten)]
    pub row: InvoiceRow,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<Customer>,
    pub refunds: Vec<Refund>,
    pub payments: Vec<Payment>,
    pub journal_entries: Vec<JournalRef>,
    #[ts(type = "Record<string, number>")]
    #[serde(with = "crate::core::dto::totals_map::required")]
    pub returned_qty: BTreeMap<String, Decimal>,
}

/// `JournalPreviewLine` (`types/index.ts:349-354`).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct JournalPreviewLine {
    pub account_code: String,
    pub account_name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub debit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub credit: Decimal,
}

// --- Refund ----------------------------------------------------------------------------------

/// `Refund.lines[]` element (`:325-339`). `invoice_line_id` is the invoice line's DTO id
/// (`InvoiceLine.id` = `{invoiceId}-l{position+1}`, `common::line_display_id`) — the id the UI reads
/// off the invoice and sends back — never the `invoice_lines.id` row key, which no DTO exposes.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct RefundLine {
    #[ts(type = "string")]
    pub invoice_line_id: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restock: Option<bool>,
}

/// `Refund` (`:321-339`). `credited_to_account` present only when `> 0` (`sales.ts:467`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct Refund {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    #[ts(type = "string")]
    pub invoice_id: Id,
    /// `DocDate` key string.
    pub date: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub lines: Vec<RefundLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub sub_total: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub tax_amount: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub grand_total: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub settled_to_receivable: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cash_back: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refund_method: Option<RefundMethod>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credited_to_account: Option<Decimal>,
}

// --- Request-only inputs -----------------------------------------------------------------------

/// `SaleInput.lines[]` element (`:266-285`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct SaleInputLine {
    /// The UUID text of a catalog product, or `"freetext"` for a free-text line
    /// (`InvoiceFormPage.vue:135`).
    pub product_id: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub price: Decimal,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount: Option<Decimal>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_is_pct: Option<bool>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_factor: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_price: Option<Decimal>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_override_reason: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_no: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_free_text: Option<bool>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revenue_account_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// `SaleInput` (`:258-316`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct SaleInput {
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<Id>,
    pub lines: Vec<SaleInputLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub discount_rate: Decimal,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_amount: Option<Decimal>,
    pub payment_method: SalePaymentMethod,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub paid_amount: Decimal,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tendered_amount: Option<Decimal>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenders: Option<Vec<Tender>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<InvoiceSource>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shift_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_type: Option<InvoiceType>,
    /// The raw wire date (bare day or ISO instant) — resolved with `RawDocDate`.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date_override: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub po_reference: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_ids: Option<Vec<String>>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manager_approved_by: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_center_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_rate: Option<Decimal>,
}

/// `RefundInput` (`:341-346`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct RefundInput {
    #[ts(type = "string")]
    pub invoice_id: Id,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub lines: Vec<RefundLine>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refund_method: Option<RefundMethod>,
}

/// `InvoiceFilter & { openOnly?: boolean }` (`:242-256`, `invoiceService.ts:78`).
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoiceListFilter {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<InvoiceStatus>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_status: Option<PaymentStatus>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<InvoiceSource>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_type: Option<InvoiceType>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cashier_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overdue_only: Option<bool>,
    /// `& { openOnly?: boolean }` (`invoiceService.ts:78`) — not part of `InvoiceFilter` itself.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_only: Option<bool>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_amount: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_amount: Option<Decimal>,
}

// --- Quotations --------------------------------------------------------------------------------

/// `Quotation` (`:121-143`). `expiry_date` = DocDate key.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct Quotation {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    pub date: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<Id>,
    #[ts(type = "string")]
    pub salesperson_id: Id,
    pub status: QuotationStatus,
    pub lines: Vec<InvoiceLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub discount_rate: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub discount_amount: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub tax_amount: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub sub_total: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub grand_total: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub po_reference: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_ids: Option<Vec<String>>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub converted_invoice_id: Option<Id>,
}

/// `QuotationRow` (`invoiceService.ts:361` — `Quotation & { customerName? }`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct QuotationRow {
    #[serde(flatten)]
    #[ts(flatten)]
    pub quotation: Quotation,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_name: Option<String>,
}

/// `QuotationInput` (`:145-153`, `invoiceService.ts:384-391`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct QuotationInput {
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<String>,
    pub lines: Vec<SaleInputLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub discount_rate: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub po_reference: Option<String>,
}

/// `QuotationFilter` (`invoiceService.ts:367`).
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct QuotationFilter {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<QuotationStatus>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
}

/// `ConvertPayment` (`invoiceService.ts:455`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct ConvertPayment {
    pub payment_method: SalePaymentMethod,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub paid_amount: Decimal,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tendered_amount: Option<Decimal>,
}

// --- Print data --------------------------------------------------------------------------------

/// `PrintData` (`invoiceService.ts:193-200`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct PrintData {
    pub invoice: Invoice,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<Customer>,
    pub cashier_name: String,
    pub settings: StoreSettings,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample: Option<bool>,
}

// =================================================================================================
// 08b — POS shifts, cash in/out, X/Z report, held sales
// =================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
#[ts(export_to = "invoices/types/gen/")]
pub enum ShiftStatus {
    Open,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
#[ts(export_to = "invoices/types/gen/")]
pub enum HandoverMode {
    Handover,
    Drop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "invoices/types/gen/")]
pub enum ShiftMovementKind {
    SaleCash,
    RefundCash,
    PayIn,
    PayOut,
    BankDrop,
}

/// `CashMovementKind` (`invoiceService.ts:319`) — the 3 kinds `recordCashInOut` accepts (a strict
/// subset of `ShiftMovementKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "invoices/types/gen/")]
pub enum CashMovementKind {
    PayIn,
    PayOut,
    BankDrop,
}

impl From<CashMovementKind> for ShiftMovementKind {
    fn from(k: CashMovementKind) -> Self {
        match k {
            CashMovementKind::PayIn => ShiftMovementKind::PayIn,
            CashMovementKind::PayOut => ShiftMovementKind::PayOut,
            CashMovementKind::BankDrop => ShiftMovementKind::BankDrop,
        }
    }
}

/// `DenominationCount` (`types/index.ts:186-189`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct DenominationCount {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub value: Decimal,
    pub count: i32,
}

/// `ShiftMovement` (`:194-203`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct ShiftMovement {
    #[ts(type = "string")]
    pub id: Id,
    pub kind: ShiftMovementKind,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_number: Option<String>,
    /// `DocDate` key string.
    pub at: String,
    #[ts(type = "string")]
    pub by: Id,
}

/// The fields of `Shift` other than `expectedCash`, shared with `ShiftRow` via flatten.
/// `ShiftRow` flattens this instead of `Shift` itself because it also flattens `ShiftSummary`,
/// which has its own (always-defined) `expectedCash` — the mock's `toShiftRow` spreads
/// `shiftSummary(s)` after `s`, so the summary's value always wins and `Shift`'s own optional copy
/// never surfaces on `ShiftRow`. `ts-rs`'s `#[ts(flatten)]` has no per-field exclusion, so
/// flattening `Shift` as-is would emit `expectedCash` twice (a TS2300 duplicate-identifier error);
/// splitting the field out keeps both `Shift`'s own contract (still has `expectedCash?: number`)
/// and `ShiftRow`'s (one `expectedCash: number`, from the summary) exactly as the mock returns
/// them, with no change to either's wire shape.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct ShiftBase {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    /// UUID text of the owning terminal.
    pub terminal_id: String,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<Id>,
    pub status: ShiftStatus,
    #[ts(type = "string")]
    pub opened_by: Id,
    /// `DocDate` key string.
    pub opened_at: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub opening_float: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening_denominations: Option<Vec<DenominationCount>>,
    pub movements: Vec<ShiftMovement>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_by: Option<Id>,
    /// `DocDate` key string.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counted_cash: Option<Decimal>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closing_denominations: Option<Vec<DenominationCount>>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variance: Option<Decimal>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handover_mode: Option<HandoverMode>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_closed_by: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// `Shift` (`:205-226`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct Shift {
    #[serde(flatten)]
    #[ts(flatten)]
    pub base: ShiftBase,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_cash: Option<Decimal>,
}

/// `ShiftSummary` (`shifts.ts:69-91`).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct SalesByMethod {
    pub label: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct ShiftSummary {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cash_sales: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cash_refunds: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub pay_ins: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub pay_outs: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub bank_drops: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub expected_cash: Decimal,
    pub sales_by_method: Vec<SalesByMethod>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub sales_total: Decimal,
    pub invoice_count: i32,
}

/// `ShiftRow` (`invoiceService.ts:253`, `shifts.ts:69-91`) — flatten `Shift` + flatten
/// `ShiftSummary` + `openedByName`/`closedByName?`.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct ShiftRow {
    #[serde(flatten)]
    #[ts(flatten)]
    pub shift: ShiftBase,
    #[serde(flatten)]
    #[ts(flatten)]
    pub summary: ShiftSummary,
    pub opened_by_name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_by_name: Option<String>,
}

/// `OpenShiftInput` (`:228-233`). `terminal_id` accepted and ignored (server-owned, D-S1).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct OpenShiftInput {
    // contract-ok: terminalId is server-owned (cross-cutting §2) — accepted for wire-shape parity
    // with the mock's `OpenShiftInput`, but never read; `cx.terminal_id` is used instead (D-S1).
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_id: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<Id>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub opening_float: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening_denominations: Option<Vec<DenominationCount>>,
}

/// `CloseShiftInput` (`:235-240`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct CloseShiftInput {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub counted_cash: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closing_denominations: Option<Vec<DenominationCount>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handover_mode: Option<HandoverMode>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// `ShiftFilter` (`invoiceService.ts:271`).
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct ShiftFilter {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ShiftStatus>,
}

/// `HeldSale.lines[]` element (`types/index.ts:170-181`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct HeldSaleLine {
    pub product_id: String,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<String>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub price: Decimal,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_price: Option<Decimal>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_override_reason: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount: Option<Decimal>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_is_pct: Option<bool>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<Id>,
}

/// `HeldSale` (`:160-182`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct HeldSale {
    #[ts(type = "string")]
    pub id: Id,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub terminal_id: String,
    /// `DocDate` key string (always an instant, `invoiceService.ts:339`).
    pub held_at: String,
    #[ts(type = "string")]
    pub held_by: Id,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<Id>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub discount_rate: Decimal,
    pub discount_is_pct: bool,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub lines: Vec<HeldSaleLine>,
}

/// `HeldSaleInput` (`Omit<HeldSale, 'id' | 'heldAt' | 'heldBy'>`, `invoiceService.ts:337`).
/// `terminal_id` accepted and ignored (D-S1).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct HeldSaleInput {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    // contract-ok: terminalId is server-owned (cross-cutting §2) — see `OpenShiftInput`.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_id: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<Id>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub discount_rate: Decimal,
    pub discount_is_pct: bool,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub lines: Vec<HeldSaleLine>,
}

// --- Command args (§1, one struct per command, camelCase) --------------------------------------

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesGetInvoicesArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<InvoiceListFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesGetInvoicesPagedArgs {
    pub query: crate::core::dto::PagedQuery<InvoiceListFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesGetInvoiceArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesPreviewSaleArgs {
    pub input: SaleInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesCreateSaleArgs {
    pub input: SaleInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesCreateRefundArgs {
    pub input: RefundInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesGetRefundArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesGetInvoicePrintDataArgs {
    pub id: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesGetQuotationsArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<QuotationFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesGetQuotationArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesSaveQuotationArgs {
    pub input: QuotationInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesSetQuotationStatusArgs {
    pub id: String,
    pub status: QuotationStatus,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesConvertQuotationToInvoiceArgs {
    pub id: String,
    pub payment: ConvertPayment,
}

// 08b command args --------------------------------------------------------------------------------

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesGetShiftsArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<ShiftFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesGetShiftArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesOpenPosShiftArgs {
    pub input: OpenShiftInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesGetXReportArgs {
    pub shift_id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesClosePosShiftArgs {
    pub shift_id: String,
    pub input: CloseShiftInput,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesForceClosePosShiftArgs {
    pub shift_id: String,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counted_cash: Option<Decimal>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesRecordCashInOutArgs {
    pub kind: CashMovementKind,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesHoldSaleArgs {
    pub input: HeldSaleInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesResumeHeldSaleArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "invoices/types/gen/")]
pub struct InvoicesDiscardHeldSaleArgs {
    pub id: String,
}
