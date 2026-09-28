//! `products` inventory DTOs (06b-inventory.md §2) — mirrors `src/modules/products/types/index.ts:
//! 187-374`. Every `date`-like field is a `DocDate` key string; `startedAt`/`approvedAt`/… instants
//! serialize with `utils::dates::format_iso_ms`.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::utils::id::Id;
use crate::utils::money::serde_number;
use crate::utils::route::RouteRef;

// --- Stock adjustments --------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "products/types/gen/")]
pub enum StockAdjustmentType {
    StockIn,
    Loss,
    Stocktake,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "products/types/gen/")]
pub enum StockInReason {
    Opening,
    OwnerContribution,
    Gift,
    Found,
    Other,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct StockAdjustmentLine {
    #[ts(type = "string")]
    pub product_id: Id,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub system_qty: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub counted_qty: Option<Decimal>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty_change: Decimal,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub unit_cost: Option<Decimal>,
    #[ts(optional)]
    pub batch_no: Option<String>,
    #[ts(optional, type = "string")]
    pub expiry_date: Option<chrono::NaiveDate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "products/types/gen/")]
pub enum StockAdjustmentStatus {
    Draft,
    Completed,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct StockAdjustment {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    pub r#type: StockAdjustmentType,
    pub date: String,
    pub status: StockAdjustmentStatus,
    pub lines: Vec<StockAdjustmentLine>,
    #[ts(optional)]
    pub note: Option<String>,
    #[ts(optional)]
    pub reason: Option<StockInReason>,
    #[ts(optional, type = "string")]
    pub offset_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub approved_by: Option<Id>,
    #[ts(optional)]
    pub approved_at: Option<String>,
}

/// `StockAdjustmentDetail` (`StockAdjustment & { journalEntryId? }`) — flat struct (codebase
/// convention over `#[serde(flatten)]`, see `CategoryWithCount`'s doc comment in `dto/catalog.rs`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct StockAdjustmentDetail {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    pub r#type: StockAdjustmentType,
    pub date: String,
    pub status: StockAdjustmentStatus,
    pub lines: Vec<StockAdjustmentLine>,
    #[ts(optional)]
    pub note: Option<String>,
    #[ts(optional)]
    pub reason: Option<StockInReason>,
    #[ts(optional, type = "string")]
    pub offset_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub approved_by: Option<Id>,
    #[ts(optional)]
    pub approved_at: Option<String>,
    #[ts(optional, type = "string")]
    pub journal_entry_id: Option<Id>,
}

impl StockAdjustmentDetail {
    pub fn new(base: StockAdjustment, journal_entry_id: Option<Id>) -> Self {
        Self {
            id: base.id,
            number: base.number,
            r#type: base.r#type,
            date: base.date,
            status: base.status,
            lines: base.lines,
            note: base.note,
            reason: base.reason,
            offset_account_id: base.offset_account_id,
            approved_by: base.approved_by,
            approved_at: base.approved_at,
            journal_entry_id,
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct StockAdjustmentLineInput {
    #[ts(type = "string")]
    pub product_id: Id,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub qty_change: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub counted_qty: Option<Decimal>,
    #[ts(optional)]
    pub batch_no: Option<String>,
    #[ts(optional, type = "string")]
    pub expiry_date: Option<chrono::NaiveDate>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct StockAdjustmentInput {
    pub r#type: StockAdjustmentType,
    pub date: String,
    #[ts(optional)]
    pub note: Option<String>,
    #[ts(optional)]
    pub reason: Option<StockInReason>,
    #[ts(optional, type = "string")]
    pub offset_account_id: Option<Id>,
    pub lines: Vec<StockAdjustmentLineInput>,
    #[ts(optional, type = "string")]
    pub approved_by: Option<Id>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct AdjustmentFilter {
    #[ts(optional)]
    pub r#type: Option<StockAdjustmentType>,
    #[ts(optional)]
    pub status: Option<StockAdjustmentStatus>,
    #[ts(optional)]
    pub from: Option<String>,
    #[ts(optional)]
    pub to: Option<String>,
}

// --- Stock movements -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "products/types/gen/")]
pub enum StockMovementReason {
    Sale,
    Purchase,
    StockIn,
    Loss,
    Stocktake,
    Refund,
    PurchaseReturn,
    TransferOut,
    TransferIn,
}

impl StockMovementReason {
    pub fn as_str(self) -> &'static str {
        match self {
            StockMovementReason::Sale => "sale",
            StockMovementReason::Purchase => "purchase",
            StockMovementReason::StockIn => "stock_in",
            StockMovementReason::Loss => "loss",
            StockMovementReason::Stocktake => "stocktake",
            StockMovementReason::Refund => "refund",
            StockMovementReason::PurchaseReturn => "purchase_return",
            StockMovementReason::TransferOut => "transfer_out",
            StockMovementReason::TransferIn => "transfer_in",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "sale" => Some(Self::Sale),
            "purchase" => Some(Self::Purchase),
            "stock_in" => Some(Self::StockIn),
            "loss" => Some(Self::Loss),
            "stocktake" => Some(Self::Stocktake),
            "refund" => Some(Self::Refund),
            "purchase_return" => Some(Self::PurchaseReturn),
            "transfer_out" => Some(Self::TransferOut),
            "transfer_in" => Some(Self::TransferIn),
            _ => None,
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct MovementFilter {
    #[ts(optional, type = "string")]
    pub product_id: Option<Id>,
    #[ts(optional)]
    pub reason: Option<StockMovementReason>,
    #[ts(optional)]
    pub from: Option<String>,
    #[ts(optional)]
    pub to: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct StockMovement {
    #[ts(type = "string")]
    pub id: Id,
    pub date: String,
    #[ts(type = "string")]
    pub product_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty_change: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub value_change: Decimal,
    pub reason: StockMovementReason,
    #[ts(type = "string")]
    pub ref_id: Id,
    #[ts(optional)]
    pub ref_number: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub balance_after: Option<Decimal>,
    #[ts(optional, type = "string")]
    pub batch_id: Option<Id>,
}

/// `StockMovementRow` (`StockMovement & { productName, refLink? }`) — flat struct.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct StockMovementRow {
    #[ts(type = "string")]
    pub id: Id,
    pub date: String,
    #[ts(type = "string")]
    pub product_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty_change: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub value_change: Decimal,
    pub reason: StockMovementReason,
    #[ts(type = "string")]
    pub ref_id: Id,
    #[ts(optional)]
    pub ref_number: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub balance_after: Option<Decimal>,
    #[ts(optional, type = "string")]
    pub batch_id: Option<Id>,
    pub product_name: String,
    #[ts(optional, type = "import('@/modules/core/types/route').AppRoute")]
    pub ref_link: Option<RouteRef>,
}

impl StockMovementRow {
    pub fn new(base: StockMovement, product_name: String, ref_link: Option<RouteRef>) -> Self {
        Self {
            id: base.id,
            date: base.date,
            product_id: base.product_id,
            qty_change: base.qty_change,
            value_change: base.value_change,
            reason: base.reason,
            ref_id: base.ref_id,
            ref_number: base.ref_number,
            balance_after: base.balance_after,
            batch_id: base.batch_id,
            product_name,
            ref_link,
        }
    }
}

// --- Batches / expiry ------------------------------------------------------------------------

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductBatch {
    #[ts(type = "string")]
    pub id: Id,
    #[ts(type = "string")]
    pub product_id: Id,
    pub batch_no: String,
    #[ts(optional, type = "string")]
    pub expiry_date: Option<chrono::NaiveDate>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub unit_cost: Decimal,
    #[ts(optional, type = "string")]
    pub supplier_id: Option<Id>,
    pub received_date: String,
    #[ts(optional, type = "string")]
    pub source_ref_id: Option<Id>,
    #[ts(optional)]
    pub source_ref_number: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub enum ExpiryBucket {
    Expired,
    Within30,
    Within60,
    Within90,
    Ok,
}

/// `ExpiryRow` (`ProductBatch & { productName, productSku, supplierName?, bucket, daysLeft }`) —
/// flat struct; `days_left` always present (`number | null`, never absent — `#[serialize_always]`
/// via plain `Option<i64>` without `skip_serializing_none` on this one field is handled by not
/// putting it under that attribute's scope; see field-level override below).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ExpiryRow {
    #[ts(type = "string")]
    pub id: Id,
    #[ts(type = "string")]
    pub product_id: Id,
    pub batch_no: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "string")]
    pub expiry_date: Option<chrono::NaiveDate>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub unit_cost: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "string")]
    pub supplier_id: Option<Id>,
    pub received_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "string")]
    pub source_ref_id: Option<Id>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source_ref_number: Option<String>,
    pub product_name: String,
    pub product_sku: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub supplier_name: Option<String>,
    pub bucket: ExpiryBucket,
    /// Always present on the wire (`number | null`), never omitted.
    #[ts(type = "number | null")]
    pub days_left: Option<i64>,
}

// --- Debit note drafts --------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "products/types/gen/")]
pub enum DebitNoteDraftStatus {
    Draft,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct DebitNoteDraftLine {
    #[ts(type = "string")]
    pub product_id: Id,
    #[ts(type = "string")]
    pub batch_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub unit_cost: Decimal,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct DebitNoteDraft {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    #[ts(type = "string")]
    pub supplier_id: Id,
    pub date: String,
    pub status: DebitNoteDraftStatus,
    pub lines: Vec<DebitNoteDraftLine>,
    #[ts(optional)]
    pub note: Option<String>,
}

// --- Stock counts --------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "products/types/gen/")]
pub enum StockCountScope {
    All,
    Category,
    Location,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "products/types/gen/")]
pub enum StockCountStatus {
    Open,
    Review,
    Completed,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct StockCountLine {
    #[ts(type = "string")]
    pub product_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub system_qty: Decimal,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub counted_qty: Option<Decimal>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub unit_cost: Decimal,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct StockCount {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    pub status: StockCountStatus,
    pub scope: StockCountScope,
    #[ts(optional, type = "string")]
    pub category_id: Option<Id>,
    #[ts(optional)]
    pub location: Option<String>,
    pub blind: bool,
    pub started_at: String,
    #[ts(type = "string")]
    pub started_by: Id,
    pub lines: Vec<StockCountLine>,
    #[ts(optional)]
    pub note: Option<String>,
    #[ts(optional, type = "string")]
    pub adjustment_id: Option<Id>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct StockCountInput {
    pub scope: StockCountScope,
    #[ts(optional, type = "string")]
    pub category_id: Option<Id>,
    #[ts(optional)]
    pub location: Option<String>,
    pub blind: bool,
    #[ts(optional)]
    pub note: Option<String>,
}

// --- Transfers -------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[ts(export_to = "products/types/gen/")]
pub enum StockTransferStatus {
    Draft,
    Sent,
    Received,
    Rejected,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct StockTransferLine {
    #[ts(type = "string")]
    pub product_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[ts(optional, type = "string")]
    pub unit_id: Option<Id>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub unit_factor: Option<Decimal>,
    #[ts(optional, type = "string")]
    pub batch_id: Option<Id>,
    #[ts(optional)]
    pub batch_no: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub received_qty: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub unit_cost: Option<Decimal>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct StockTransfer {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    #[ts(type = "string")]
    pub from_branch_id: Id,
    #[ts(type = "string")]
    pub to_branch_id: Id,
    pub status: StockTransferStatus,
    pub date: String,
    pub lines: Vec<StockTransferLine>,
    #[ts(optional)]
    pub note: Option<String>,
    #[ts(optional)]
    pub sent_at: Option<String>,
    #[ts(optional, type = "string")]
    pub sent_by: Option<Id>,
    #[ts(optional)]
    pub received_at: Option<String>,
    #[ts(optional, type = "string")]
    pub received_by: Option<Id>,
    #[ts(optional)]
    pub rejected_at: Option<String>,
    #[ts(optional, type = "string")]
    pub rejected_by: Option<Id>,
    #[ts(optional)]
    pub reject_reason: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub shortage_value: Option<Decimal>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct StockTransferLineInput {
    #[ts(type = "string")]
    pub product_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[ts(optional, type = "string")]
    pub unit_id: Option<Id>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub unit_factor: Option<Decimal>,
    #[ts(optional, type = "string")]
    pub batch_id: Option<Id>,
    #[ts(optional)]
    pub batch_no: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct StockTransferInput {
    #[ts(type = "string")]
    pub from_branch_id: Id,
    #[ts(type = "string")]
    pub to_branch_id: Id,
    pub date: String,
    #[ts(optional)]
    pub note: Option<String>,
    pub lines: Vec<StockTransferLineInput>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ReceiveTransferLineInput {
    #[ts(type = "string")]
    pub product_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub received_qty: Decimal,
    #[ts(optional, type = "string")]
    pub batch_id: Option<Id>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ReceiveTransferInput {
    pub lines: Vec<ReceiveTransferLineInput>,
}

// --- Command args -----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsGetStockAdjustmentsArgs {
    #[ts(optional)]
    #[serde(default)]
    pub filter: Option<AdjustmentFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsGetStockAdjustmentArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsCreateStockAdjustmentArgs {
    pub input: StockAdjustmentInput,
    #[ts(optional)]
    #[serde(default)]
    pub as_draft: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsCompleteAdjustmentArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsDeleteDraftAdjustmentArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsGetStockMovementsArgs {
    #[ts(optional)]
    #[serde(default)]
    pub filter: Option<MovementFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsGetStockMovementsPagedArgs {
    pub query: crate::core::dto::PagedQuery<MovementFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsGetBatchesArgs {
    pub product_id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsWriteOffExpiredBatchesArgs {
    pub batch_ids: Vec<String>,
    #[ts(optional)]
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsReturnBatchesToSupplierArgs {
    pub supplier_id: String,
    pub lines: Vec<DebitNoteDraftLineInput>,
    #[ts(optional)]
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct DebitNoteDraftLineInput {
    #[ts(type = "string")]
    pub product_id: Id,
    #[ts(type = "string")]
    pub batch_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub unit_cost: Decimal,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsGetStockCountArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsCreateStockCountArgs {
    pub input: StockCountInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsUpdateStockCountLineArgs {
    pub count_id: String,
    pub product_id: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[ts(optional)]
    #[serde(default)]
    pub delta: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsCountIdArgs {
    pub count_id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsGetTransferArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsCreateTransferArgs {
    pub input: StockTransferInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsSendTransferArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsReceiveTransferArgs {
    pub id: String,
    pub input: ReceiveTransferInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsRejectTransferArgs {
    pub id: String,
    pub reason: String,
}
