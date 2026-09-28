//! `domains::vouchers::commands` — thin IPC layer (10-vouchers.md §1). Every route is area
//! `payments`; reads use `with_read` + `cx.require(tx, Payments, Read)` is not available on the
//! plain `with_read` transaction, so reads that need auth use `with_tx` with a read-shaped body,
//! same convention as `expenses::expenses_get_due_recurring_expenses` — except here every read is a
//! genuine "no business clock needed" query, so `with_read` alone would work for auth-less reads;
//! the spec calls for `core::settings::require` explicitly, so every read command threads through
//! `with_tx`/`cx.require` for consistency with the writes' auth path. 11 commands total.

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_tx, BoxFuture, TxOpts, TxResult};

use super::dto::{
    CardSettlement, FeeEstimate, UnsettledTenderGroup, Voucher, VouchersCreateCardSettlementArgs, VouchersCreateOwnerVoucherArgs,
    VouchersCreatePaymentVoucherArgs, VouchersCreateReceiptVoucherArgs, VouchersCreateTransferVoucherArgs, VouchersEstimateSettlementFeeArgs,
    VouchersGetCardSettlementArgs, VouchersGetVoucherArgs, VouchersGetVouchersArgs,
};
use super::service;

type CmdResult<T> = Result<T, ApiErrorPayload>;

#[tauri::command]
pub async fn vouchers_create_receipt_voucher(state: State<'_, AppState>, args: VouchersCreateReceiptVoucherArgs) -> CmdResult<Voucher> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Payments, Access::Write).await?;
            service::general::create_receipt_voucher(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<Voucher>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn vouchers_create_payment_voucher(state: State<'_, AppState>, args: VouchersCreatePaymentVoucherArgs) -> CmdResult<Voucher> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Payments, Access::Write).await?;
            service::general::create_payment_voucher(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<Voucher>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn vouchers_create_transfer_voucher(state: State<'_, AppState>, args: VouchersCreateTransferVoucherArgs) -> CmdResult<Voucher> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Payments, Access::Write).await?;
            service::general::create_transfer_voucher(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<Voucher>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn vouchers_create_owner_voucher(state: State<'_, AppState>, args: VouchersCreateOwnerVoucherArgs) -> CmdResult<Voucher> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Payments, Access::Write).await?;
            service::general::create_owner_voucher(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<Voucher>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn vouchers_get_vouchers(state: State<'_, AppState>, args: VouchersGetVouchersArgs) -> CmdResult<Vec<Voucher>> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let filter = args.filter.clone();
        Box::pin(async move {
            cx.require(tx, Area::Payments, Access::Read).await?;
            service::read::get_vouchers(tx, filter).await
        }) as BoxFuture<'_, TxResult<Vec<Voucher>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn vouchers_get_voucher(state: State<'_, AppState>, args: VouchersGetVoucherArgs) -> CmdResult<Voucher> {
    let id = args.id;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Payments, Access::Read).await?;
            service::read::get_voucher(tx, id).await
        }) as BoxFuture<'_, TxResult<Voucher>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn vouchers_get_unsettled_tender_groups(state: State<'_, AppState>) -> CmdResult<Vec<UnsettledTenderGroup>> {
    with_tx(&state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Payments, Access::Read).await?;
            service::settlements::unsettled_tender_groups(tx).await
        }) as BoxFuture<'_, TxResult<Vec<UnsettledTenderGroup>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn vouchers_estimate_settlement_fee(state: State<'_, AppState>, args: VouchersEstimateSettlementFeeArgs) -> CmdResult<FeeEstimate> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let groups = args.groups.clone();
        Box::pin(async move {
            cx.require(tx, Area::Payments, Access::Read).await?;
            service::settlements::estimate_settlement_fee(tx, groups).await
        }) as BoxFuture<'_, TxResult<FeeEstimate>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn vouchers_create_card_settlement(state: State<'_, AppState>, args: VouchersCreateCardSettlementArgs) -> CmdResult<CardSettlement> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Payments, Access::Write).await?;
            service::settlements::create_card_settlement(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<CardSettlement>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn vouchers_get_card_settlements(state: State<'_, AppState>) -> CmdResult<Vec<CardSettlement>> {
    with_tx(&state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Payments, Access::Read).await?;
            service::read::get_card_settlements(tx).await
        }) as BoxFuture<'_, TxResult<Vec<CardSettlement>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn vouchers_get_card_settlement(state: State<'_, AppState>, args: VouchersGetCardSettlementArgs) -> CmdResult<CardSettlement> {
    let id = args.id;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Payments, Access::Read).await?;
            service::read::get_card_settlement(tx, id).await
        }) as BoxFuture<'_, TxResult<CardSettlement>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}
