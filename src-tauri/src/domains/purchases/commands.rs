//! `purchases` IPC commands (07-purchases.md §1) — thin layer: parse args, authorize (area
//! `purchases` for every command — reads `Purchases:Read`, writes `Purchases:Write`), open a
//! transaction, call the service, map the error. Reads use `with_read_ctx`; writes use `with_tx`
//! (posting commands move `state.undo.clone()` into the closure).

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_read_ctx, with_tx, BoxFuture, TxOpts, TxResult};
use crate::domains::products::dto::inventory::{DebitNoteDraft, ProductBatch};
use crate::utils::id::Id;

use super::dto::*;
use super::service::{orders, read, receive, returns};

type CmdResult<T> = Result<T, ApiErrorPayload>;

fn parse_id(raw: &str) -> Result<Id, ApiErrorPayload> {
    raw.parse::<Id>().map_err(|_| crate::core::error::AppError::validation("معرّف غير صالح").into())
}

// --- Reads -----------------------------------------------------------------------------------

#[tauri::command]
pub async fn purchases_get_purchase_orders(state: State<'_, AppState>, args: PurchasesGetPurchaseOrdersArgs) -> CmdResult<Vec<PurchaseRow>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Purchases, Access::Read).await?;
            read::get_purchase_orders(tx, args.filter).await
        }) as BoxFuture<'_, TxResult<Vec<PurchaseRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn purchases_get_purchase_order(state: State<'_, AppState>, args: PurchasesGetPurchaseOrderArgs) -> CmdResult<PurchaseDetail> {
    let id = parse_id(&args.id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Purchases, Access::Read).await?;
            read::get_purchase_order(tx, id).await
        }) as BoxFuture<'_, TxResult<PurchaseDetail>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn purchases_get_purchase_return(state: State<'_, AppState>, args: PurchasesGetPurchaseReturnArgs) -> CmdResult<PurchaseReturn> {
    let id = parse_id(&args.id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Purchases, Access::Read).await?;
            read::get_purchase_return(tx, id).await
        }) as BoxFuture<'_, TxResult<PurchaseReturn>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn purchases_get_active_batches(state: State<'_, AppState>, args: PurchasesGetActiveBatchesArgs) -> CmdResult<Vec<ProductBatch>> {
    let product_id = parse_id(&args.product_id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Purchases, Access::Read).await?;
            read::get_active_batches(tx, product_id).await
        }) as BoxFuture<'_, TxResult<Vec<ProductBatch>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn purchases_get_debit_note_drafts(state: State<'_, AppState>) -> CmdResult<Vec<DebitNoteDraft>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Purchases, Access::Read).await?;
            read::get_debit_note_drafts(tx).await
        }) as BoxFuture<'_, TxResult<Vec<DebitNoteDraft>>>
    })
    .await
    .map_err(Into::into)
}

// --- Writes ----------------------------------------------------------------------------------

#[tauri::command]
pub async fn purchases_save_purchase_order(state: State<'_, AppState>, args: PurchasesSavePurchaseOrderArgs) -> CmdResult<PurchaseOrder> {
    let id = match &args.id {
        Some(s) => Some(parse_id(s)?),
        None => None,
    };
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Purchases, Access::Write).await?;
            orders::save_purchase_order(tx, cx, &undo, input, id).await
        }) as BoxFuture<'_, TxResult<PurchaseOrder>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn purchases_send_purchase_order_to_supplier(state: State<'_, AppState>, args: PurchasesSendPurchaseOrderToSupplierArgs) -> CmdResult<PurchaseOrder> {
    let id = parse_id(&args.id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Purchases, Access::Write).await?;
            orders::send_purchase_order_to_supplier(tx, cx, &undo, id).await
        }) as BoxFuture<'_, TxResult<PurchaseOrder>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn purchases_receive_purchase_order(state: State<'_, AppState>, args: PurchasesReceivePurchaseOrderArgs) -> CmdResult<PurchaseOrder> {
    let id = parse_id(&args.id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Purchases, Access::Write).await?;
            receive::receive_purchase(tx, cx, &undo, id, input).await
        }) as BoxFuture<'_, TxResult<PurchaseOrder>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn purchases_confirm_purchase_order(state: State<'_, AppState>, args: PurchasesConfirmPurchaseOrderArgs) -> CmdResult<PurchaseOrder> {
    let id = parse_id(&args.id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Purchases, Access::Write).await?;
            orders::confirm_purchase_order(tx, cx, &undo, id).await
        }) as BoxFuture<'_, TxResult<PurchaseOrder>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn purchases_cancel_purchase_order(state: State<'_, AppState>, args: PurchasesCancelPurchaseOrderArgs) -> CmdResult<PurchaseOrder> {
    let id = parse_id(&args.id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Purchases, Access::Write).await?;
            orders::cancel_purchase_order(tx, cx, &undo, id).await
        }) as BoxFuture<'_, TxResult<PurchaseOrder>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn purchases_create_purchase_return(state: State<'_, AppState>, args: PurchasesCreatePurchaseReturnArgs) -> CmdResult<PurchaseReturn> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Purchases, Access::Write).await?;
            returns::create_purchase_return(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<PurchaseReturn>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn purchases_post_debit_note_draft(state: State<'_, AppState>, args: PurchasesPostDebitNoteDraftArgs) -> CmdResult<PurchaseReturn> {
    let draft_id = parse_id(&args.draft_id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let refund_method = args.refund_method;
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Purchases, Access::Write).await?;
            returns::post_debit_note_draft(tx, cx, &undo, draft_id, refund_method).await
        }) as BoxFuture<'_, TxResult<PurchaseReturn>>
    })
    .await
    .map_err(Into::into)
}
