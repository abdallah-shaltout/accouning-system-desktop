//! `products` branch-transfer IPC commands (06b-inventory.md §1, `transferService.ts`). Area
//! `inventory` for every command, same thin-layer pattern as `commands/inventory.rs`.

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_read_ctx, with_tx, BoxFuture, TxOpts, TxResult};
use crate::utils::id::Id;

use super::super::dto::inventory::*;
use super::super::service::transfers;

type CmdResult<T> = Result<T, ApiErrorPayload>;

fn parse_id(raw: &str) -> Result<Id, ApiErrorPayload> {
    raw.parse::<Id>().map_err(|_| crate::core::error::AppError::validation("معرّف غير صالح").into())
}

#[tauri::command]
pub async fn products_get_transfers(state: State<'_, AppState>) -> CmdResult<Vec<StockTransfer>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            transfers::get_transfers(tx).await
        }) as BoxFuture<'_, TxResult<Vec<StockTransfer>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_get_transfer(state: State<'_, AppState>, args: ProductsGetTransferArgs) -> CmdResult<StockTransfer> {
    let id = parse_id(&args.id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            transfers::get_transfer(tx, id).await
        }) as BoxFuture<'_, TxResult<StockTransfer>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_create_transfer(state: State<'_, AppState>, args: ProductsCreateTransferArgs) -> CmdResult<StockTransfer> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            transfers::create_transfer(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<StockTransfer>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_send_transfer(state: State<'_, AppState>, args: ProductsSendTransferArgs) -> CmdResult<StockTransfer> {
    let id = parse_id(&args.id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            transfers::send_transfer(tx, cx, &undo, id).await
        }) as BoxFuture<'_, TxResult<StockTransfer>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_receive_transfer(state: State<'_, AppState>, args: ProductsReceiveTransferArgs) -> CmdResult<StockTransfer> {
    let id = parse_id(&args.id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            transfers::receive_transfer(tx, cx, &undo, id, input).await
        }) as BoxFuture<'_, TxResult<StockTransfer>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_reject_transfer(state: State<'_, AppState>, args: ProductsRejectTransferArgs) -> CmdResult<StockTransfer> {
    let id = parse_id(&args.id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let reason = args.reason.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            transfers::reject_transfer(tx, cx, &undo, id, reason).await
        }) as BoxFuture<'_, TxResult<StockTransfer>>
    })
    .await
    .map_err(Into::into)
}
