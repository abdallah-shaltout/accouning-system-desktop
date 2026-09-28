//! `domains::accounting::commands::accounts` — thin IPC layer (12-accounting.md §1).

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_read, with_tx, BoxFuture, TxOpts, TxResult};

use super::super::dto::{Account, AccountWithBalance, AccountingDeleteAccountArgs, AccountingGetAccountsArgs, AccountingReparentAccountArgs, AccountingSaveAccountArgs};
use super::super::service;

type CmdResult<T> = Result<T, ApiErrorPayload>;

/// Session only — read by other modules' pages (`pdfService.ts:30`, invoice/product/voucher/
/// setup/expense pages) with no area check, matching the mock's unguarded `getAccounts`.
#[tauri::command]
pub async fn accounting_get_accounts(state: State<'_, AppState>, args: AccountingGetAccountsArgs) -> CmdResult<Vec<AccountWithBalance>> {
    if state.session.read().unwrap().is_none() {
        return Err(ApiErrorPayload::from(crate::core::error::AppError::unauthorized("سجّل الدخول أولاً")));
    }
    with_read(&state, move |tx| Box::pin(async move { service::accounts::get_accounts(tx, args.range).await }) as BoxFuture<'_, TxResult<Vec<AccountWithBalance>>>)
        .await
        .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_save_account(state: State<'_, AppState>, args: AccountingSaveAccountArgs) -> CmdResult<Account> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let id = args.id;
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            service::accounts::save_account(tx, cx, input, id).await
        }) as BoxFuture<'_, TxResult<Account>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_delete_account(state: State<'_, AppState>, args: AccountingDeleteAccountArgs) -> CmdResult<()> {
    let id = args.id;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            service::accounts::delete_account(tx, cx, id).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_reparent_account(state: State<'_, AppState>, args: AccountingReparentAccountArgs) -> CmdResult<Account> {
    let id = args.id;
    let new_parent_id = args.new_parent_id;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            service::accounts::reparent_account(tx, cx, id, new_parent_id).await
        }) as BoxFuture<'_, TxResult<Account>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}
