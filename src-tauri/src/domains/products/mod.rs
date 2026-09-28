//! `products` domain (03-domains/06-products.md + 06b-inventory.md) — catalog (products, categories,
//! units, price lists, custom fields, 22 commands) + inventory (stock adjustments, movements,
//! batches/expiry, debit-note drafts, stock counts, branch transfers, 25 commands). 47 commands
//! total. No undo compensators (both files' §5 — not undoable via the registry).
//!
//! **Needs from manager** (blocking `cargo check` for this module — see this wave's final report):
//! `pub mod products;` + `domains::products::ipc_signatures()`/`export_bindings()` hooks in
//! `domains/mod.rs`, the matching `domains::products::commands::*` lines in `lib.rs`'s
//! `generate_handler!`, `AppState.approval_grants` (G-P3), `SequenceLock::ProductCodes` +
//! `document_counters` seed row `productCodesLock` (G-P4c), `shared::defaults::branch_prefix`
//! (present — G-P2 done), migration m0016 (`categories/units/price_lists.name_live` -> utf8mb4_bin,
//! `uq_products_barcode_live`, G-P4).

pub mod commands;
pub mod dto;
pub mod service;

use crate::core::ipc::IpcSig;
use crate::ipc_sig;

/// 06 §1 (22) + 06b §1 (25) = 47 commands, in each file's table order.
pub fn ipc_signatures() -> Vec<IpcSig> {
    use dto::catalog as c;
    use dto::inventory as i;

    vec![
        // --- 06-products.md catalog (22) ---
        ipc_sig!(products_get_products, c::ProductsGetProductsArgs, Vec<c::Product>),
        ipc_sig!(products_get_product, c::ProductsGetProductArgs, c::Product),
        ipc_sig!(products_find_by_code, c::ProductsFindByCodeArgs, Option<c::Product>),
        ipc_sig!(products_generate_ean13, (), String),
        ipc_sig!(products_create_product, c::ProductsCreateProductArgs, c::Product),
        ipc_sig!(products_update_product, c::ProductsUpdateProductArgs, c::Product),
        ipc_sig!(products_suggest_sku, c::ProductsSuggestSkuArgs, String),
        ipc_sig!(products_get_categories, (), Vec<c::CategoryWithCount>),
        ipc_sig!(products_save_category, c::ProductsSaveCategoryArgs, c::Category),
        ipc_sig!(products_delete_category, c::ProductsDeleteCategoryArgs, ()),
        ipc_sig!(products_get_units, (), Vec<c::UnitWithCount>),
        ipc_sig!(products_save_unit, c::ProductsSaveUnitArgs, c::Unit),
        ipc_sig!(products_apply_unit_preset, c::ProductsApplyUnitPresetArgs, Vec<c::Unit>),
        ipc_sig!(products_delete_unit, c::ProductsDeleteUnitArgs, ()),
        ipc_sig!(products_get_price_lists, (), Vec<c::PriceList>),
        ipc_sig!(products_save_price_list, c::ProductsSavePriceListArgs, c::PriceList),
        ipc_sig!(products_delete_price_list, c::ProductsDeletePriceListArgs, ()),
        ipc_sig!(products_set_price_list_values, c::ProductsSetPriceListValuesArgs, ()),
        ipc_sig!(products_get_custom_field_defs, (), Vec<c::CustomFieldDef>),
        ipc_sig!(products_save_custom_field_def, c::ProductsSaveCustomFieldDefArgs, c::CustomFieldDef),
        ipc_sig!(products_delete_custom_field_def, c::ProductsDeleteCustomFieldDefArgs, ()),
        ipc_sig!(products_reorder_custom_field_defs, c::ProductsReorderCustomFieldDefsArgs, ()),
        // --- 06b-inventory.md inventory (25) ---
        ipc_sig!(products_get_stock_adjustments, i::ProductsGetStockAdjustmentsArgs, Vec<i::StockAdjustment>),
        ipc_sig!(products_get_stock_adjustment, i::ProductsGetStockAdjustmentArgs, i::StockAdjustmentDetail),
        ipc_sig!(products_create_stock_adjustment, i::ProductsCreateStockAdjustmentArgs, i::StockAdjustment),
        ipc_sig!(products_complete_adjustment, i::ProductsCompleteAdjustmentArgs, i::StockAdjustment),
        ipc_sig!(products_delete_draft_adjustment, i::ProductsDeleteDraftAdjustmentArgs, ()),
        ipc_sig!(products_get_stock_movements, i::ProductsGetStockMovementsArgs, Vec<i::StockMovementRow>),
        ipc_sig!(
            products_get_stock_movements_paged,
            i::ProductsGetStockMovementsPagedArgs,
            crate::core::dto::PagedResult<i::StockMovementRow>
        ),
        ipc_sig!(products_get_batches, i::ProductsGetBatchesArgs, Vec<i::ProductBatch>),
        ipc_sig!(products_get_expiry_report, (), Vec<i::ExpiryRow>),
        ipc_sig!(products_write_off_expired_batches, i::ProductsWriteOffExpiredBatchesArgs, i::StockAdjustment),
        ipc_sig!(products_return_batches_to_supplier, i::ProductsReturnBatchesToSupplierArgs, i::DebitNoteDraft),
        ipc_sig!(products_get_debit_note_drafts, (), Vec<i::DebitNoteDraft>),
        ipc_sig!(products_get_stock_counts, (), Vec<i::StockCount>),
        ipc_sig!(products_get_stock_count, i::ProductsGetStockCountArgs, i::StockCount),
        ipc_sig!(products_create_stock_count, i::ProductsCreateStockCountArgs, i::StockCount),
        ipc_sig!(products_update_stock_count_line, i::ProductsUpdateStockCountLineArgs, i::StockCount),
        ipc_sig!(products_submit_count_for_review, i::ProductsCountIdArgs, i::StockCount),
        ipc_sig!(products_resume_counting, i::ProductsCountIdArgs, i::StockCount),
        ipc_sig!(products_complete_stock_count, i::ProductsCountIdArgs, i::StockAdjustment),
        ipc_sig!(products_get_transfers, (), Vec<i::StockTransfer>),
        ipc_sig!(products_get_transfer, i::ProductsGetTransferArgs, i::StockTransfer),
        ipc_sig!(products_create_transfer, i::ProductsCreateTransferArgs, i::StockTransfer),
        ipc_sig!(products_send_transfer, i::ProductsSendTransferArgs, i::StockTransfer),
        ipc_sig!(products_receive_transfer, i::ProductsReceiveTransferArgs, i::StockTransfer),
        ipc_sig!(products_reject_transfer, i::ProductsRejectTransferArgs, i::StockTransfer),
    ]
}

/// This domain's DTO exports (06 §2 + 06b §2) — one line the manager appends to
/// `domains::export_bindings` (`domains/mod.rs`) in the same commit that adds `pub mod products;`
/// there, mirroring `users::export_bindings`'s doc comment.
pub fn export_bindings(cfg: &ts_rs::Config) {
    use ts_rs::TS;
    use dto::catalog as c;
    use dto::inventory as i;

    // --- Catalog (06 §2) ---
    c::Product::export_all(cfg).expect("export Product");
    c::ProductUnit::export_all(cfg).expect("export ProductUnit");
    c::ProductUnitPrice::export_all(cfg).expect("export ProductUnitPrice");
    c::ProductPrice::export_all(cfg).expect("export ProductPrice");
    c::ProductPriceInput::export_all(cfg).expect("export ProductPriceInput");
    c::BranchStock::export_all(cfg).expect("export BranchStock");
    c::ProductInput::export_all(cfg).expect("export ProductInput");
    c::ProductFilter::export_all(cfg).expect("export ProductFilter");
    c::Category::export_all(cfg).expect("export Category");
    c::CategoryDefaults::export_all(cfg).expect("export CategoryDefaults");
    c::CategoryWithCount::export_all(cfg).expect("export CategoryWithCount");
    c::Unit::export_all(cfg).expect("export Unit");
    c::UnitExtra::export_all(cfg).expect("export UnitExtra");
    c::UnitWithCount::export_all(cfg).expect("export UnitWithCount");
    c::UnitPresetKind::export_all(cfg).expect("export UnitPresetKind");
    c::PriceList::export_all(cfg).expect("export PriceList");
    c::PriceListInput::export_all(cfg).expect("export PriceListInput");
    c::CustomFieldType::export_all(cfg).expect("export CustomFieldType");
    c::CustomFieldDef::export_all(cfg).expect("export CustomFieldDef");
    c::CustomFieldDefInput::export_all(cfg).expect("export CustomFieldDefInput");
    c::ProductsGetProductsArgs::export_all(cfg).expect("export ProductsGetProductsArgs");
    c::ProductsGetProductArgs::export_all(cfg).expect("export ProductsGetProductArgs");
    c::ProductsFindByCodeArgs::export_all(cfg).expect("export ProductsFindByCodeArgs");
    c::ProductsCreateProductArgs::export_all(cfg).expect("export ProductsCreateProductArgs");
    c::ProductsUpdateProductArgs::export_all(cfg).expect("export ProductsUpdateProductArgs");
    c::ProductsSuggestSkuArgs::export_all(cfg).expect("export ProductsSuggestSkuArgs");
    c::ProductsSaveCategoryArgs::export_all(cfg).expect("export ProductsSaveCategoryArgs");
    c::ProductsDeleteCategoryArgs::export_all(cfg).expect("export ProductsDeleteCategoryArgs");
    c::ProductsSaveUnitArgs::export_all(cfg).expect("export ProductsSaveUnitArgs");
    c::ProductsApplyUnitPresetArgs::export_all(cfg).expect("export ProductsApplyUnitPresetArgs");
    c::ProductsDeleteUnitArgs::export_all(cfg).expect("export ProductsDeleteUnitArgs");
    c::ProductsSavePriceListArgs::export_all(cfg).expect("export ProductsSavePriceListArgs");
    c::ProductsDeletePriceListArgs::export_all(cfg).expect("export ProductsDeletePriceListArgs");
    c::ProductsSetPriceListValuesArgs::export_all(cfg).expect("export ProductsSetPriceListValuesArgs");
    c::ProductsSaveCustomFieldDefArgs::export_all(cfg).expect("export ProductsSaveCustomFieldDefArgs");
    c::ProductsDeleteCustomFieldDefArgs::export_all(cfg).expect("export ProductsDeleteCustomFieldDefArgs");
    c::ProductsReorderCustomFieldDefsArgs::export_all(cfg).expect("export ProductsReorderCustomFieldDefsArgs");

    // --- Inventory (06b §2) ---
    i::StockAdjustmentType::export_all(cfg).expect("export StockAdjustmentType");
    i::StockInReason::export_all(cfg).expect("export StockInReason");
    i::StockAdjustmentLine::export_all(cfg).expect("export StockAdjustmentLine");
    i::StockAdjustmentStatus::export_all(cfg).expect("export StockAdjustmentStatus");
    i::StockAdjustment::export_all(cfg).expect("export StockAdjustment");
    i::StockAdjustmentDetail::export_all(cfg).expect("export StockAdjustmentDetail");
    i::StockAdjustmentLineInput::export_all(cfg).expect("export StockAdjustmentLineInput");
    i::StockAdjustmentInput::export_all(cfg).expect("export StockAdjustmentInput");
    i::AdjustmentFilter::export_all(cfg).expect("export AdjustmentFilter");
    i::StockMovementReason::export_all(cfg).expect("export StockMovementReason");
    i::MovementFilter::export_all(cfg).expect("export MovementFilter");
    i::StockMovement::export_all(cfg).expect("export StockMovement");
    i::StockMovementRow::export_all(cfg).expect("export StockMovementRow");
    i::ProductBatch::export_all(cfg).expect("export ProductBatch");
    i::ExpiryBucket::export_all(cfg).expect("export ExpiryBucket");
    i::ExpiryRow::export_all(cfg).expect("export ExpiryRow");
    i::DebitNoteDraftStatus::export_all(cfg).expect("export DebitNoteDraftStatus");
    i::DebitNoteDraftLine::export_all(cfg).expect("export DebitNoteDraftLine");
    i::DebitNoteDraft::export_all(cfg).expect("export DebitNoteDraft");
    i::StockCountScope::export_all(cfg).expect("export StockCountScope");
    i::StockCountStatus::export_all(cfg).expect("export StockCountStatus");
    i::StockCountLine::export_all(cfg).expect("export StockCountLine");
    i::StockCount::export_all(cfg).expect("export StockCount");
    i::StockCountInput::export_all(cfg).expect("export StockCountInput");
    i::StockTransferStatus::export_all(cfg).expect("export StockTransferStatus");
    i::StockTransferLine::export_all(cfg).expect("export StockTransferLine");
    i::StockTransfer::export_all(cfg).expect("export StockTransfer");
    i::StockTransferLineInput::export_all(cfg).expect("export StockTransferLineInput");
    i::StockTransferInput::export_all(cfg).expect("export StockTransferInput");
    i::ReceiveTransferLineInput::export_all(cfg).expect("export ReceiveTransferLineInput");
    i::ReceiveTransferInput::export_all(cfg).expect("export ReceiveTransferInput");
    i::ProductsGetStockAdjustmentsArgs::export_all(cfg).expect("export ProductsGetStockAdjustmentsArgs");
    i::ProductsGetStockAdjustmentArgs::export_all(cfg).expect("export ProductsGetStockAdjustmentArgs");
    i::ProductsCreateStockAdjustmentArgs::export_all(cfg).expect("export ProductsCreateStockAdjustmentArgs");
    i::ProductsCompleteAdjustmentArgs::export_all(cfg).expect("export ProductsCompleteAdjustmentArgs");
    i::ProductsDeleteDraftAdjustmentArgs::export_all(cfg).expect("export ProductsDeleteDraftAdjustmentArgs");
    i::ProductsGetStockMovementsArgs::export_all(cfg).expect("export ProductsGetStockMovementsArgs");
    i::ProductsGetStockMovementsPagedArgs::export_all(cfg).expect("export ProductsGetStockMovementsPagedArgs");
    i::ProductsGetBatchesArgs::export_all(cfg).expect("export ProductsGetBatchesArgs");
    i::ProductsWriteOffExpiredBatchesArgs::export_all(cfg).expect("export ProductsWriteOffExpiredBatchesArgs");
    i::ProductsReturnBatchesToSupplierArgs::export_all(cfg).expect("export ProductsReturnBatchesToSupplierArgs");
    i::DebitNoteDraftLineInput::export_all(cfg).expect("export DebitNoteDraftLineInput");
    i::ProductsGetStockCountArgs::export_all(cfg).expect("export ProductsGetStockCountArgs");
    i::ProductsCreateStockCountArgs::export_all(cfg).expect("export ProductsCreateStockCountArgs");
    i::ProductsUpdateStockCountLineArgs::export_all(cfg).expect("export ProductsUpdateStockCountLineArgs");
    i::ProductsCountIdArgs::export_all(cfg).expect("export ProductsCountIdArgs");
    i::ProductsGetTransferArgs::export_all(cfg).expect("export ProductsGetTransferArgs");
    i::ProductsCreateTransferArgs::export_all(cfg).expect("export ProductsCreateTransferArgs");
    i::ProductsSendTransferArgs::export_all(cfg).expect("export ProductsSendTransferArgs");
    i::ProductsReceiveTransferArgs::export_all(cfg).expect("export ProductsReceiveTransferArgs");
    i::ProductsRejectTransferArgs::export_all(cfg).expect("export ProductsRejectTransferArgs");
}
