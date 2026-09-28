//! `payments` IPC commands (09-payments.md §1) — 7 commands. Every read `with_read_ctx` +
//! `Payments∨Parties:Read`; every write `with_tx` + `Payments:Write`.

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::{ApiErrorPayload, PagedResult};
use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::core::tx::{with_read_ctx, with_tx, BoxFuture, TxOpts, TxResult};

use super::dto::{
    OpenDocument, Payment, PaymentRow, PaymentsAllocateExistingPaymentArgs, PaymentsCreatePaymentArgs, PaymentsGetOpenDocumentsArgs, PaymentsGetPaymentArgs,
    PaymentsGetPaymentsArgs, PaymentsGetPaymentsPagedArgs, PaymentsRemoveAllocationArgs,
};
use super::service;

fn parse_id(raw: &str) -> Result<crate::utils::id::Id, ApiErrorPayload> {
    raw.parse().map_err(|_| AppError::validation("معرّف غير صالح").into())
}

#[tauri::command]
pub async fn payments_get_payments(state: State<'_, AppState>, args: PaymentsGetPaymentsArgs) -> Result<Vec<PaymentRow>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let filter = args.filter.clone().unwrap_or_default();
        Box::pin(async move {
            ctx.require_any(tx, &[(Area::Payments, Access::Read), (Area::Parties, Access::Read)]).await?;
            service::read::get_payments(tx, filter).await
        }) as BoxFuture<'_, TxResult<Vec<PaymentRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn payments_get_payments_paged(state: State<'_, AppState>, args: PaymentsGetPaymentsPagedArgs) -> Result<PagedResult<PaymentRow>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let query = args.query.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Payments, Access::Read).await?;
            service::read::get_payments_paged(tx, query).await
        }) as BoxFuture<'_, TxResult<PagedResult<PaymentRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn payments_get_payment(state: State<'_, AppState>, args: PaymentsGetPaymentArgs) -> Result<PaymentRow, ApiErrorPayload> {
    let id = parse_id(&args.id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Payments, Access::Read).await?;
            service::read::get_payment(tx, id).await
        }) as BoxFuture<'_, TxResult<PaymentRow>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn payments_create_payment(state: State<'_, AppState>, args: PaymentsCreatePaymentArgs) -> Result<Payment, ApiErrorPayload> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Payments, Access::Write).await?;
            service::create::create_payment(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<Payment>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn payments_allocate_existing_payment(state: State<'_, AppState>, args: PaymentsAllocateExistingPaymentArgs) -> Result<Payment, ApiErrorPayload> {
    let payment_id = parse_id(&args.payment_id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let allocations = args.allocations.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Payments, Access::Write).await?;
            service::allocate::allocate_existing_payment(tx, cx, &undo, payment_id, allocations).await
        }) as BoxFuture<'_, TxResult<Payment>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn payments_remove_allocation(state: State<'_, AppState>, args: PaymentsRemoveAllocationArgs) -> Result<Payment, ApiErrorPayload> {
    let payment_id = parse_id(&args.payment_id)?;
    let allocation_id = parse_id(&args.allocation_id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Payments, Access::Write).await?;
            service::allocate::remove_allocation(tx, cx, &undo, payment_id, allocation_id).await
        }) as BoxFuture<'_, TxResult<Payment>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn payments_get_open_documents(state: State<'_, AppState>, args: PaymentsGetOpenDocumentsArgs) -> Result<Vec<OpenDocument>, ApiErrorPayload> {
    let target_id = parse_id(&args.target_id)?;
    with_read_ctx(&state, move |tx, ctx| {
        let target_type = args.target_type;
        Box::pin(async move {
            ctx.require_any(tx, &[(Area::Payments, Access::Read), (Area::Parties, Access::Read)]).await?;
            service::read::get_open_documents(tx, target_type, target_id).await
        }) as BoxFuture<'_, TxResult<Vec<OpenDocument>>>
    })
    .await
    .map_err(Into::into)
}

/// §1's 7 commands, in table order.
pub fn ipc_signatures() -> Vec<crate::core::ipc::IpcSig> {
    use super::dto::*;
    vec![
        crate::ipc_sig!(payments_get_payments, PaymentsGetPaymentsArgs, Vec<PaymentRow>),
        crate::ipc_sig!(payments_get_payments_paged, PaymentsGetPaymentsPagedArgs, crate::core::dto::PagedResult<PaymentRow>),
        crate::ipc_sig!(payments_get_payment, PaymentsGetPaymentArgs, PaymentRow),
        crate::ipc_sig!(payments_create_payment, PaymentsCreatePaymentArgs, Payment),
        crate::ipc_sig!(payments_allocate_existing_payment, PaymentsAllocateExistingPaymentArgs, Payment),
        crate::ipc_sig!(payments_remove_allocation, PaymentsRemoveAllocationArgs, Payment),
        crate::ipc_sig!(payments_get_open_documents, PaymentsGetOpenDocumentsArgs, Vec<OpenDocument>),
    ]
}
