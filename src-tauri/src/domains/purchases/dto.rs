//! `purchases` DTOs (03-domains/07-purchases.md §2) — mirrors `src/modules/purchases/types/index.ts`
//! field-for-field. Conventions per `core/dto.rs`/`products/dto/catalog.rs` header (P2-33):
//! `camelCase`, `Option` fields `skip_serializing_none` + `#[ts(optional)]` (absent, never `null`),
//! `Decimal` via `utils::money::serde_number` + `#[ts(type = "number")]`, `Id` as
//! `#[ts(type = "string")]`, dates as `DocDate` key strings.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::domains::parties::dto::Supplier;
use crate::domains::payments::dto::Payment;
use crate::domains::products::dto::catalog::ProductType;
use crate::utils::id::Id;
use crate::utils::money::serde_number;

// --- Status / small enums ---------------------------------------------------------------------

/// `PurchaseStatus` (`types/index.ts:4`) — `SCREAMING_SNAKE_CASE`, maps the entity's own enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "purchases/types/gen/")]
pub enum PurchaseStatus {
    Draft,
    Ordered,
    Received,
    Canceled,
}

/// `LandedCostSpread` (`types/index.ts:34`) — `lowercase`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "purchases/types/gen/")]
pub enum LandedCostSpread {
    Value,
    Qty,
}

/// `PaymentStatus` (`invoices/types/index.ts`, reused by `PurchaseOrder.paymentStatus` per
/// `types/index.ts:56` — same 3-way shape the entity's own `purchase_orders.payment_status` column
/// stores). Defined here (not imported from `payments::dto`, which has no `PaymentStatus` DTO of
/// its own — its domain only deals with `Payment`/`PaymentAllocation`) so `purchases` doesn't take
/// a dependency on `invoices` (08, built after 07 per the wave order) just for this enum.
/// `SCREAMING_SNAKE_CASE` matches the entity's stored strings exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "purchases/types/gen/")]
pub enum PaymentStatus {
    Unpaid,
    PartiallyPaid,
    Paid,
}

/// `RefundMethod` (`types/index.ts:158`) — `snake_case` (`bank_transfer`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "purchases/types/gen/")]
pub enum RefundMethod {
    Cash,
    BankTransfer,
    Credit,
}

// --- Lines / landed costs / invoice discount ---------------------------------------------------

/// `PurchaseLine` (`types/index.ts:9-31`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchaseLine {
    #[ts(type = "string")]
    pub product_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cost_price: Decimal,
    #[ts(optional, type = "string")]
    pub unit_id: Option<Id>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub unit_factor: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub discount: Option<Decimal>,
    #[ts(optional)]
    pub discount_is_pct: Option<bool>,
    #[ts(optional, type = "string")]
    pub tax_id: Option<Id>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub received_qty: Option<Decimal>,
    #[ts(optional)]
    pub batch_no: Option<String>,
    #[ts(optional, type = "string")]
    pub expiry_date: Option<chrono::NaiveDate>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub landed_cost_share: Option<Decimal>,
}

/// `LandedCostLine` (`types/index.ts:36-43`). `id` is a mock-only synthetic id (text), never
/// referenced elsewhere — kept as plain `String`, not `Id`, so a fresh one is trivial to mint.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct LandedCostLine {
    pub id: String,
    pub label: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    #[ts(optional, type = "string")]
    pub supplier_id: Option<Id>,
    pub spread_by: LandedCostSpread,
}

/// `LandedCostLineInput` (`types/index.ts:100-105`) — same shape without `id` (assigned fresh on
/// save/receive).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct LandedCostLineInput {
    pub label: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    #[ts(optional, type = "string")]
    pub supplier_id: Option<Id>,
    pub spread_by: LandedCostSpread,
}

/// `PurchaseOrder.invoiceDiscount` inline shape (`types/index.ts:65`) — both `Option<Decimal>`,
/// mutually exclusive by convention (the totals engine picks whichever is set), not enforced.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct InvoiceDiscount {
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub pct: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub amount: Option<Decimal>,
}

// --- PurchaseOrder / inputs ----------------------------------------------------------------------

/// `PurchaseOrder` (`types/index.ts:45-87`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchaseOrder {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    #[ts(type = "string")]
    pub supplier_id: Id,
    /// `DocDate` key string.
    pub date: String,
    pub status: PurchaseStatus,
    pub lines: Vec<PurchaseLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub sub_total: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub tax_rate: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub tax_amount: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub grand_total: Decimal,
    pub payment_status: PaymentStatus,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub paid_amount: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub returned_amount: Decimal,
    #[ts(optional)]
    pub note: Option<String>,
    #[ts(optional)]
    pub invoice_discount: Option<InvoiceDiscount>,
    #[ts(optional)]
    pub landed_costs: Option<Vec<LandedCostLine>>,
    #[ts(optional)]
    pub supplier_invoice_no: Option<String>,
    #[ts(optional, type = "string")]
    pub supplier_invoice_date: Option<chrono::NaiveDate>,
    #[ts(optional)]
    pub vat_not_recoverable: Option<bool>,
    #[ts(optional)]
    pub sent_at: Option<String>,
    #[ts(optional, type = "string")]
    pub backorder_of_id: Option<Id>,
    /// `DocDate` key string — set once received.
    #[ts(optional)]
    pub received_date: Option<String>,
    #[ts(optional)]
    pub attachment_ids: Option<Vec<String>>,
    #[ts(optional, type = "string")]
    pub cost_center_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub branch_id: Option<Id>,
    #[ts(optional)]
    pub currency: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub exchange_rate: Option<Decimal>,
}

/// `PurchaseLineInput` (`types/index.ts:89-98`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchaseLineInput {
    #[ts(type = "string")]
    pub product_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cost_price: Decimal,
    #[ts(optional, type = "string")]
    pub unit_id: Option<Id>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub unit_factor: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub discount: Option<Decimal>,
    #[ts(optional)]
    pub discount_is_pct: Option<bool>,
    #[ts(optional, type = "string")]
    pub tax_id: Option<Id>,
}

/// `PurchaseOrderInput` (`types/index.ts:107-123`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchaseOrderInput {
    #[ts(type = "string")]
    pub supplier_id: Id,
    /// The raw wire date (bare day or ISO instant) — resolved with `RawDocDate`.
    pub date: String,
    pub lines: Vec<PurchaseLineInput>,
    #[ts(optional)]
    pub note: Option<String>,
    #[ts(optional)]
    pub invoice_discount: Option<InvoiceDiscount>,
    #[ts(optional)]
    pub landed_costs: Option<Vec<LandedCostLineInput>>,
    #[ts(optional)]
    pub supplier_invoice_no: Option<String>,
    #[ts(optional, type = "string")]
    pub supplier_invoice_date: Option<chrono::NaiveDate>,
    #[ts(optional)]
    pub attachment_ids: Option<Vec<String>>,
    #[ts(optional, type = "string")]
    pub cost_center_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub branch_id: Option<Id>,
    #[ts(optional)]
    pub currency: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub exchange_rate: Option<Decimal>,
    pub confirm: bool,
}

// --- Receiving -------------------------------------------------------------------------------

/// `ReceiveBatchInput` (`types/index.ts:135`, one batch per requested receiving-line split).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct ReceiveBatchInput {
    pub batch_no: String,
    #[ts(optional, type = "string")]
    pub expiry_date: Option<chrono::NaiveDate>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
}

/// `ReceiveLineInput` (`types/index.ts:131-136`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct ReceiveLineInput {
    #[ts(type = "string")]
    pub product_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub received_qty: Decimal,
    #[ts(optional)]
    pub batches: Option<Vec<ReceiveBatchInput>>,
}

/// `ReceivePurchaseInput` (`types/index.ts:138-148`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct ReceivePurchaseInput {
    pub date: String,
    pub lines: Vec<ReceiveLineInput>,
    #[ts(optional)]
    pub supplier_invoice_no: Option<String>,
    #[ts(optional, type = "string")]
    pub supplier_invoice_date: Option<chrono::NaiveDate>,
    #[ts(optional)]
    pub vat_not_recoverable: Option<bool>,
    #[ts(optional)]
    pub landed_costs: Option<Vec<LandedCostLineInput>>,
    #[ts(optional)]
    pub create_backorder: Option<bool>,
}

// --- Filter ------------------------------------------------------------------------------------

/// `PurchaseFilter & { from?; to? }` (`types/index.ts:150-155`, `purchaseService.ts:59`) — one flat
/// struct, all optional.
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchaseListFilter {
    #[ts(optional)]
    pub search: Option<String>,
    #[ts(optional)]
    pub status: Option<PurchaseStatus>,
    #[ts(optional)]
    pub payment_status: Option<PaymentStatus>,
    #[ts(optional, type = "string")]
    pub supplier_id: Option<Id>,
    #[ts(optional)]
    pub from: Option<String>,
    #[ts(optional)]
    pub to: Option<String>,
}

// --- Returns / debit notes ---------------------------------------------------------------------

/// `DebitNoteLine` (`types/index.ts:160-166`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct DebitNoteLine {
    #[ts(type = "string")]
    pub product_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cost_price: Decimal,
    #[ts(optional, type = "string")]
    pub batch_id: Option<Id>,
}

/// `PurchaseReturn` (`types/index.ts:168-185`). The mock's runtime object also carries an extra
/// `taxRate` field not in the TS type or here (Q-U8) — parity compares typed fields only.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchaseReturn {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    #[ts(type = "string")]
    pub purchase_order_id: Id,
    #[ts(type = "string")]
    pub supplier_id: Id,
    /// `DocDate` key string.
    pub date: String,
    #[ts(optional)]
    pub reason: Option<String>,
    pub lines: Vec<DebitNoteLine>,
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
    pub settled_to_payable: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cash_back: Decimal,
    pub refund_method: RefundMethod,
    #[ts(optional, type = "string")]
    pub from_draft_id: Option<Id>,
}

/// `PurchaseReturnInput` (`types/index.ts:187-194`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchaseReturnInput {
    #[ts(type = "string")]
    pub purchase_order_id: Id,
    #[ts(optional)]
    pub reason: Option<String>,
    #[ts(optional)]
    pub refund_method: Option<RefundMethod>,
    pub lines: Vec<PurchaseReturnInputLine>,
    #[ts(optional, type = "string")]
    pub from_draft_id: Option<Id>,
}

/// `PurchaseReturnInput.lines[]` element (`types/index.ts:192`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchaseReturnInputLine {
    #[ts(type = "string")]
    pub product_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[ts(optional, type = "string")]
    pub batch_id: Option<Id>,
}

// --- List row / detail --------------------------------------------------------------------------

/// `PurchaseRow` (`purchaseService.ts:36`) — `PurchaseOrder & { supplierName, outstanding,
/// missingSupplierInvoice }`. Flat struct (codebase convention over `serde(flatten)`, per
/// `products::dto::catalog::CategoryWithCount`'s doc comment).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchaseRow {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    #[ts(type = "string")]
    pub supplier_id: Id,
    pub date: String,
    pub status: PurchaseStatus,
    pub lines: Vec<PurchaseLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub sub_total: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub tax_rate: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub tax_amount: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub grand_total: Decimal,
    pub payment_status: PaymentStatus,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub paid_amount: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub returned_amount: Decimal,
    #[ts(optional)]
    pub note: Option<String>,
    #[ts(optional)]
    pub invoice_discount: Option<InvoiceDiscount>,
    #[ts(optional)]
    pub landed_costs: Option<Vec<LandedCostLine>>,
    #[ts(optional)]
    pub supplier_invoice_no: Option<String>,
    #[ts(optional, type = "string")]
    pub supplier_invoice_date: Option<chrono::NaiveDate>,
    #[ts(optional)]
    pub vat_not_recoverable: Option<bool>,
    #[ts(optional)]
    pub sent_at: Option<String>,
    #[ts(optional, type = "string")]
    pub backorder_of_id: Option<Id>,
    #[ts(optional)]
    pub received_date: Option<String>,
    #[ts(optional)]
    pub attachment_ids: Option<Vec<String>>,
    #[ts(optional, type = "string")]
    pub cost_center_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub branch_id: Option<Id>,
    #[ts(optional)]
    pub currency: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub exchange_rate: Option<Decimal>,
    pub supplier_name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub outstanding: Decimal,
    pub missing_supplier_invoice: bool,
}

impl PurchaseRow {
    pub fn from_order(po: PurchaseOrder, supplier_name: String, outstanding: Decimal, missing_supplier_invoice: bool) -> Self {
        Self {
            id: po.id,
            number: po.number,
            supplier_id: po.supplier_id,
            date: po.date,
            status: po.status,
            lines: po.lines,
            sub_total: po.sub_total,
            tax_rate: po.tax_rate,
            tax_amount: po.tax_amount,
            grand_total: po.grand_total,
            payment_status: po.payment_status,
            paid_amount: po.paid_amount,
            returned_amount: po.returned_amount,
            note: po.note,
            invoice_discount: po.invoice_discount,
            landed_costs: po.landed_costs,
            supplier_invoice_no: po.supplier_invoice_no,
            supplier_invoice_date: po.supplier_invoice_date,
            vat_not_recoverable: po.vat_not_recoverable,
            sent_at: po.sent_at,
            backorder_of_id: po.backorder_of_id,
            received_date: po.received_date,
            attachment_ids: po.attachment_ids,
            cost_center_id: po.cost_center_id,
            branch_id: po.branch_id,
            currency: po.currency,
            exchange_rate: po.exchange_rate,
            supplier_name,
            outstanding,
            missing_supplier_invoice,
        }
    }
}

/// `PurchaseDetail.products` element (`purchaseService.ts:41`) — always the product's *current*
/// display values, never snapshotted on the PO line.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchaseProductInfo {
    pub name: String,
    pub sku: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub stock_qty: Decimal,
    pub r#type: ProductType,
    #[ts(optional)]
    pub track_batches: Option<bool>,
}

/// `PurchaseDetail.journalEntries[]` element (`purchaseService.ts:44`).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchaseJournalRef {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    pub description: String,
}

/// `PurchaseDetail` (`purchaseService.ts:38-48`) — flat `PurchaseRow` fields + the extra detail
/// fields, flat struct per the same codebase convention as `PurchaseRow`.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchaseDetail {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    #[ts(type = "string")]
    pub supplier_id: Id,
    pub date: String,
    pub status: PurchaseStatus,
    pub lines: Vec<PurchaseLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub sub_total: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub tax_rate: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub tax_amount: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub grand_total: Decimal,
    pub payment_status: PaymentStatus,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub paid_amount: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub returned_amount: Decimal,
    #[ts(optional)]
    pub note: Option<String>,
    #[ts(optional)]
    pub invoice_discount: Option<InvoiceDiscount>,
    #[ts(optional)]
    pub landed_costs: Option<Vec<LandedCostLine>>,
    #[ts(optional)]
    pub supplier_invoice_no: Option<String>,
    #[ts(optional, type = "string")]
    pub supplier_invoice_date: Option<chrono::NaiveDate>,
    #[ts(optional)]
    pub vat_not_recoverable: Option<bool>,
    #[ts(optional)]
    pub sent_at: Option<String>,
    #[ts(optional, type = "string")]
    pub backorder_of_id: Option<Id>,
    #[ts(optional)]
    pub received_date: Option<String>,
    #[ts(optional)]
    pub attachment_ids: Option<Vec<String>>,
    #[ts(optional, type = "string")]
    pub cost_center_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub branch_id: Option<Id>,
    #[ts(optional)]
    pub currency: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub exchange_rate: Option<Decimal>,
    pub supplier_name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub outstanding: Decimal,
    pub missing_supplier_invoice: bool,
    #[ts(optional)]
    pub supplier: Option<Supplier>,
    #[ts(type = "Record<string, import('./PurchaseProductInfo').PurchaseProductInfo>")]
    pub products: BTreeMap<String, PurchaseProductInfo>,
    pub returns: Vec<PurchaseReturn>,
    pub payments: Vec<Payment>,
    pub journal_entries: Vec<PurchaseJournalRef>,
    #[ts(type = "Record<string, number>")]
    pub returned_qty: BTreeMap<String, Decimal>,
    #[ts(optional)]
    pub duplicate_invoice_warning: Option<String>,
}

impl PurchaseDetail {
    #[allow(clippy::too_many_arguments)]
    pub fn from_row(
        row: PurchaseRow,
        supplier: Option<Supplier>,
        products: BTreeMap<String, PurchaseProductInfo>,
        returns: Vec<PurchaseReturn>,
        payments: Vec<Payment>,
        journal_entries: Vec<PurchaseJournalRef>,
        returned_qty: BTreeMap<String, Decimal>,
        duplicate_invoice_warning: Option<String>,
    ) -> Self {
        Self {
            id: row.id,
            number: row.number,
            supplier_id: row.supplier_id,
            date: row.date,
            status: row.status,
            lines: row.lines,
            sub_total: row.sub_total,
            tax_rate: row.tax_rate,
            tax_amount: row.tax_amount,
            grand_total: row.grand_total,
            payment_status: row.payment_status,
            paid_amount: row.paid_amount,
            returned_amount: row.returned_amount,
            note: row.note,
            invoice_discount: row.invoice_discount,
            landed_costs: row.landed_costs,
            supplier_invoice_no: row.supplier_invoice_no,
            supplier_invoice_date: row.supplier_invoice_date,
            vat_not_recoverable: row.vat_not_recoverable,
            sent_at: row.sent_at,
            backorder_of_id: row.backorder_of_id,
            received_date: row.received_date,
            attachment_ids: row.attachment_ids,
            cost_center_id: row.cost_center_id,
            branch_id: row.branch_id,
            currency: row.currency,
            exchange_rate: row.exchange_rate,
            supplier_name: row.supplier_name,
            outstanding: row.outstanding,
            missing_supplier_invoice: row.missing_supplier_invoice,
            supplier,
            products,
            returns,
            payments,
            journal_entries,
            returned_qty,
            duplicate_invoice_warning,
        }
    }
}

// --- Command args (§1, one struct per command, camelCase) --------------------------------------

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchasesGetPurchaseOrdersArgs {
    #[ts(optional)]
    pub filter: Option<PurchaseListFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchasesGetPurchaseOrderArgs {
    pub id: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchasesSavePurchaseOrderArgs {
    pub input: PurchaseOrderInput,
    #[ts(optional)]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchasesSendPurchaseOrderToSupplierArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchasesReceivePurchaseOrderArgs {
    pub id: String,
    pub input: ReceivePurchaseInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchasesConfirmPurchaseOrderArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchasesCancelPurchaseOrderArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchasesCreatePurchaseReturnArgs {
    pub input: PurchaseReturnInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchasesGetPurchaseReturnArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchasesGetActiveBatchesArgs {
    pub product_id: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "purchases/types/gen/")]
pub struct PurchasesPostDebitNoteDraftArgs {
    pub draft_id: String,
    #[ts(optional)]
    pub refund_method: Option<RefundMethod>,
}

// `ProductBatch`/`DebitNoteDraft` (06b `dto/inventory.rs`) are reused directly by `service::read`/
// `commands.rs` via `crate::domains::products::dto::inventory::{ProductBatch, DebitNoteDraft}` —
// not redeclared here (spec §2 "reused" row).
