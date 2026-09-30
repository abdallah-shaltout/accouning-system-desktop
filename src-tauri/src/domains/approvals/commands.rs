//! `approvals` IPC commands (03-domains/04-approvals.md §1) — thin layer: parse args, authorize,
//! open a transaction, call the service, map the error.

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_read_ctx, with_tx, BoxFuture, TxOpts, TxResult};
use crate::utils::id::Id;

use super::dto::{
    ApprovalRequest, ApprovalsApproveRequestArgs, ApprovalsGetApprovalRequestsArgs, ApprovalsRejectRequestArgs, ApprovalsSubmitApprovalRequestArgs, ApprovalStatus,
};
use super::service;

/// A non-UUID id (`'usr-does-not-exist'`) maps to a never-stored id (`Id::unknown_from_text`, plan 21
/// Part 04 Wave 2) so the service answers it with its own `NOT_FOUND` refusal, as the mock does,
/// instead of a generic `VALIDATION` "معرّف غير صالح".
fn parse_id(raw: &str) -> Result<Id, ApiErrorPayload> {
    Ok(raw.parse::<Id>().unwrap_or_else(|_| Id::unknown_from_text(raw)))
}

/// D-2: authorised by the area of the action being approved, not `Approvals:Write` — cashiers
/// (discount/below-cost) have `Approvals:None`, so the check must key off `input.kind`.
#[tauri::command]
pub async fn approvals_submit_approval_request(state: State<'_, AppState>, args: ApprovalsSubmitApprovalRequestArgs) -> Result<ApprovalRequest, ApiErrorPayload> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            use super::dto::ApprovalKind;
            match input.kind {
                ApprovalKind::Discount | ApprovalKind::BelowCost => {
                    cx.require_any(tx, &[(Area::Pos, Access::Write), (Area::Sales, Access::Write)]).await?;
                }
                ApprovalKind::WriteOff => {
                    cx.require(tx, Area::Inventory, Access::Write).await?;
                }
            }
            service::submit(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<ApprovalRequest>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn approvals_get_approval_requests(state: State<'_, AppState>, args: ApprovalsGetApprovalRequestsArgs) -> Result<Vec<ApprovalRequest>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Approvals, Access::Read).await?;
            service::list(tx, args.filter).await
        }) as BoxFuture<'_, TxResult<Vec<ApprovalRequest>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn approvals_get_pending_approval_count(state: State<'_, AppState>) -> Result<u32, ApiErrorPayload> {
    with_read_ctx(&state, |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Approvals, Access::Read).await?;
            service::pending_count(tx).await
        }) as BoxFuture<'_, TxResult<u32>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn approvals_approve_request(state: State<'_, AppState>, args: ApprovalsApproveRequestArgs) -> Result<ApprovalRequest, ApiErrorPayload> {
    let id = parse_id(&args.id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone().unwrap_or_default();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Approvals, Access::Write).await?;
            service::decide(tx, cx, &undo, id, ApprovalStatus::Approved, input).await
        }) as BoxFuture<'_, TxResult<ApprovalRequest>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn approvals_reject_request(state: State<'_, AppState>, args: ApprovalsRejectRequestArgs) -> Result<ApprovalRequest, ApiErrorPayload> {
    let id = parse_id(&args.id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Approvals, Access::Write).await?;
            service::decide(tx, cx, &undo, id, ApprovalStatus::Rejected, input).await
        }) as BoxFuture<'_, TxResult<ApprovalRequest>>
    })
    .await
    .map_err(Into::into)
}
