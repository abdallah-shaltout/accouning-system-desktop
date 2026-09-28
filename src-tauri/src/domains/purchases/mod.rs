//! `purchases` domain (03-domains/07-purchases.md) — purchase orders (draft/order/receive at cost
//! with landed costs folded in, short-delivery backorders), purchase returns/debit notes (three
//! refund methods). 12 commands. No undo compensators (§5 — not undoable via the registry; a
//! receipt is corrected by a purchase return, a return by a new purchase).
//!
//! **Needs from manager** (blocking `cargo check` for this module — see this wave's final report):
//! `pub mod purchases;` + `domains::purchases::ipc_signatures()`/`export_bindings()` hooks in
//! `domains/mod.rs`, the matching `domains::purchases::commands::*` lines in `lib.rs`'s
//! `generate_handler!`. Depends on `domains::payments` (G-30's `Payment`/`PaymentAllocation` DTOs
//! and `service::common::payment_dtos`) and `domains::parties` (`Supplier`,
//! `service::read::get_supplier`) both being wired in too, since this module's `get_purchase_order`
//! calls into both.

pub mod commands;
pub mod dto;
pub mod service;

use crate::core::ipc::IpcSig;
use crate::ipc_sig;

/// 12 commands, §1 table order.
pub fn ipc_signatures() -> Vec<IpcSig> {
    use dto as d;

    vec![
        ipc_sig!(purchases_get_purchase_orders, d::PurchasesGetPurchaseOrdersArgs, Vec<d::PurchaseRow>),
        ipc_sig!(purchases_get_purchase_order, d::PurchasesGetPurchaseOrderArgs, d::PurchaseDetail),
        ipc_sig!(purchases_save_purchase_order, d::PurchasesSavePurchaseOrderArgs, d::PurchaseOrder),
        ipc_sig!(purchases_send_purchase_order_to_supplier, d::PurchasesSendPurchaseOrderToSupplierArgs, d::PurchaseOrder),
        ipc_sig!(purchases_receive_purchase_order, d::PurchasesReceivePurchaseOrderArgs, d::PurchaseOrder),
        ipc_sig!(purchases_confirm_purchase_order, d::PurchasesConfirmPurchaseOrderArgs, d::PurchaseOrder),
        ipc_sig!(purchases_cancel_purchase_order, d::PurchasesCancelPurchaseOrderArgs, d::PurchaseOrder),
        ipc_sig!(purchases_create_purchase_return, d::PurchasesCreatePurchaseReturnArgs, d::PurchaseReturn),
        ipc_sig!(purchases_get_purchase_return, d::PurchasesGetPurchaseReturnArgs, d::PurchaseReturn),
        ipc_sig!(purchases_get_active_batches, d::PurchasesGetActiveBatchesArgs, Vec<crate::domains::products::dto::inventory::ProductBatch>),
        ipc_sig!(purchases_get_debit_note_drafts, (), Vec<crate::domains::products::dto::inventory::DebitNoteDraft>),
        ipc_sig!(purchases_post_debit_note_draft, d::PurchasesPostDebitNoteDraftArgs, d::PurchaseReturn),
    ]
}

/// This domain's DTO exports (§2) — one line the manager appends to `domains::export_bindings`
/// (`domains/mod.rs`) in the same commit that adds `pub mod purchases;` there, mirroring
/// `products::export_bindings`'s doc comment. `ProductBatch`/`DebitNoteDraft` (06b) and
/// `Payment`/`Supplier` (payments/parties) are exported by their own owning domains, not
/// re-exported here.
pub fn export_bindings(cfg: &ts_rs::Config) {
    use ts_rs::TS;
    use dto as d;

    d::PurchaseStatus::export_all(cfg).expect("export PurchaseStatus");
    d::LandedCostSpread::export_all(cfg).expect("export LandedCostSpread");
    d::RefundMethod::export_all(cfg).expect("export RefundMethod");
    d::PaymentStatus::export_all(cfg).expect("export purchases::PaymentStatus");
    d::PurchaseLine::export_all(cfg).expect("export PurchaseLine");
    d::LandedCostLine::export_all(cfg).expect("export LandedCostLine");
    d::LandedCostLineInput::export_all(cfg).expect("export LandedCostLineInput");
    d::InvoiceDiscount::export_all(cfg).expect("export purchases::InvoiceDiscount");
    d::PurchaseOrder::export_all(cfg).expect("export PurchaseOrder");
    d::PurchaseLineInput::export_all(cfg).expect("export PurchaseLineInput");
    d::PurchaseOrderInput::export_all(cfg).expect("export PurchaseOrderInput");
    d::ReceiveBatchInput::export_all(cfg).expect("export ReceiveBatchInput");
    d::ReceiveLineInput::export_all(cfg).expect("export ReceiveLineInput");
    d::ReceivePurchaseInput::export_all(cfg).expect("export ReceivePurchaseInput");
    d::PurchaseListFilter::export_all(cfg).expect("export PurchaseListFilter");
    d::DebitNoteLine::export_all(cfg).expect("export DebitNoteLine");
    d::PurchaseReturn::export_all(cfg).expect("export PurchaseReturn");
    d::PurchaseReturnInput::export_all(cfg).expect("export PurchaseReturnInput");
    d::PurchaseReturnInputLine::export_all(cfg).expect("export PurchaseReturnInputLine");
    d::PurchaseRow::export_all(cfg).expect("export PurchaseRow");
    d::PurchaseProductInfo::export_all(cfg).expect("export PurchaseProductInfo");
    d::PurchaseJournalRef::export_all(cfg).expect("export PurchaseJournalRef");
    d::PurchaseDetail::export_all(cfg).expect("export PurchaseDetail");
    d::PurchasesGetPurchaseOrdersArgs::export_all(cfg).expect("export PurchasesGetPurchaseOrdersArgs");
    d::PurchasesGetPurchaseOrderArgs::export_all(cfg).expect("export PurchasesGetPurchaseOrderArgs");
    d::PurchasesSavePurchaseOrderArgs::export_all(cfg).expect("export PurchasesSavePurchaseOrderArgs");
    d::PurchasesSendPurchaseOrderToSupplierArgs::export_all(cfg).expect("export PurchasesSendPurchaseOrderToSupplierArgs");
    d::PurchasesReceivePurchaseOrderArgs::export_all(cfg).expect("export PurchasesReceivePurchaseOrderArgs");
    d::PurchasesConfirmPurchaseOrderArgs::export_all(cfg).expect("export PurchasesConfirmPurchaseOrderArgs");
    d::PurchasesCancelPurchaseOrderArgs::export_all(cfg).expect("export PurchasesCancelPurchaseOrderArgs");
    d::PurchasesCreatePurchaseReturnArgs::export_all(cfg).expect("export PurchasesCreatePurchaseReturnArgs");
    d::PurchasesGetPurchaseReturnArgs::export_all(cfg).expect("export PurchasesGetPurchaseReturnArgs");
    d::PurchasesGetActiveBatchesArgs::export_all(cfg).expect("export PurchasesGetActiveBatchesArgs");
    d::PurchasesPostDebitNoteDraftArgs::export_all(cfg).expect("export PurchasesPostDebitNoteDraftArgs");
}
