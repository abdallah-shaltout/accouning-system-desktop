//! `domains::accounting::commands::period` — thin IPC layer (12b-period-close.md §1). 12b-owned.

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::error::AppError;
use crate::core::ipc::IpcSig;
use crate::core::state::AppState;
use crate::core::tx::{with_read, with_tx, BoxFuture, TxOpts, TxResult};
use crate::ipc_sig;

use super::super::dto::{
    AccountingCloseYearArgs, AccountingGetCloseYearPreChecksArgs, AccountingGetVatPeriodTotalsArgs, AccountingPayVatSettlementNowArgs,
    AccountingReopenYearArgs, AccountingSaveFiscalYearArgs, AccountingSaveLockDateArgs, AccountingSubmitVatSettlementArgs, CloseYearPreCheck, CloseYearResult,
    FiscalYear, JournalEntry, VatPeriodTotals,
};
use super::super::service;

type CmdResult<T> = Result<T, ApiErrorPayload>;

/// Session only (`BudgetVsActualPage.vue:15`).
#[tauri::command]
pub async fn accounting_get_fiscal_years(state: State<'_, AppState>) -> CmdResult<Vec<FiscalYear>> {
    if state.session.read().unwrap().is_none() {
        return Err(ApiErrorPayload::from(AppError::unauthorized("سجّل الدخول أولاً")));
    }
    with_read(&state, |tx| Box::pin(async move { service::period::get_fiscal_years(tx).await }) as BoxFuture<'_, TxResult<Vec<FiscalYear>>>)
        .await
        .map_err(ApiErrorPayload::from)
}

/// Session only (`useReportRange.ts:3`); needs `cx.clock.today()`, so `with_tx` rather than
/// `with_read` even though it writes nothing (mirrors 11-expenses' `getDueRecurringExpenses`).
#[tauri::command]
pub async fn accounting_get_current_fiscal_year(state: State<'_, AppState>) -> CmdResult<Option<FiscalYear>> {
    with_tx(&state, TxOpts { require_user: true }, move |tx, cx| {
        Box::pin(async move {
            let today = cx.clock.today();
            service::period::get_current_fiscal_year(tx, today).await
        }) as BoxFuture<'_, TxResult<Option<FiscalYear>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_save_fiscal_year(state: State<'_, AppState>, args: AccountingSaveFiscalYearArgs) -> CmdResult<FiscalYear> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let id = args.id;
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            service::period::save_fiscal_year(tx, cx, input, id).await
        }) as BoxFuture<'_, TxResult<FiscalYear>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_get_lock_date(state: State<'_, AppState>) -> CmdResult<Option<String>> {
    crate::core::tx::with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Accounting, Access::Read).await?;
            service::period::get_lock_date(tx).await
        }) as BoxFuture<'_, TxResult<Option<String>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_save_lock_date(state: State<'_, AppState>, args: AccountingSaveLockDateArgs) -> CmdResult<()> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let lock_date = args.lock_date.clone();
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            service::period::save_lock_date(tx, cx, lock_date).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_get_close_year_pre_checks(state: State<'_, AppState>, args: AccountingGetCloseYearPreChecksArgs) -> CmdResult<Vec<CloseYearPreCheck>> {
    crate::core::tx::with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Accounting, Access::Read).await?;
            service::period::get_close_year_pre_checks(tx, args.fiscal_year_id).await
        }) as BoxFuture<'_, TxResult<Vec<CloseYearPreCheck>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_close_year(state: State<'_, AppState>, args: AccountingCloseYearArgs) -> CmdResult<CloseYearResult> {
    let fiscal_year_id = args.fiscal_year_id;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            service::period::close_year(tx, cx, &undo, fiscal_year_id).await
        }) as BoxFuture<'_, TxResult<CloseYearResult>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

/// Area::Accounting Write **and** role admin — the admin check is re-verified inside
/// `service::period::reopen_year` itself (never trust the hidden button, analysis §1): it refuses
/// a non-admin with `إعادة فتح السنة المالية للمدير فقط` before touching anything else.
#[tauri::command]
pub async fn accounting_reopen_year(state: State<'_, AppState>, args: AccountingReopenYearArgs) -> CmdResult<FiscalYear> {
    let fiscal_year_id = args.fiscal_year_id;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            let (fy, _audit_id) = service::period::reopen_year(tx, cx, &undo, fiscal_year_id).await?;
            Ok(fy)
        }) as BoxFuture<'_, TxResult<FiscalYear>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_get_vat_period_totals(state: State<'_, AppState>, args: AccountingGetVatPeriodTotalsArgs) -> CmdResult<VatPeriodTotals> {
    crate::core::tx::with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Accounting, Access::Read).await?;
            service::vat::vat_totals(tx, &args.from, &args.to).await
        }) as BoxFuture<'_, TxResult<VatPeriodTotals>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_submit_vat_settlement(state: State<'_, AppState>, args: AccountingSubmitVatSettlementArgs) -> CmdResult<JournalEntry> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let from = args.from.clone();
        let to = args.to.clone();
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            service::vat::submit_vat_settlement(tx, cx, &from, &to).await
        }) as BoxFuture<'_, TxResult<JournalEntry>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_pay_vat_settlement_now(state: State<'_, AppState>, args: AccountingPayVatSettlementNowArgs) -> CmdResult<JournalEntry> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        let amount = args.amount;
        let payment_method_id = args.payment_method_id;
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            service::vat::pay_vat_settlement_now(tx, cx, &undo, amount, payment_method_id).await
        }) as BoxFuture<'_, TxResult<JournalEntry>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

/// 12b's 11 `ipc_sig!` lines — appended by 12's `mod.rs::ipc_signatures()`.
pub fn ipc_signatures() -> Vec<IpcSig> {
    vec![
        ipc_sig!(accounting_get_fiscal_years, (), Vec<FiscalYear>),
        ipc_sig!(accounting_get_current_fiscal_year, (), Option<FiscalYear>),
        ipc_sig!(accounting_save_fiscal_year, super::super::dto::AccountingSaveFiscalYearArgs, FiscalYear),
        ipc_sig!(accounting_get_lock_date, (), Option<String>),
        ipc_sig!(accounting_save_lock_date, super::super::dto::AccountingSaveLockDateArgs, ()),
        ipc_sig!(accounting_get_close_year_pre_checks, super::super::dto::AccountingGetCloseYearPreChecksArgs, Vec<CloseYearPreCheck>),
        ipc_sig!(accounting_close_year, super::super::dto::AccountingCloseYearArgs, CloseYearResult),
        ipc_sig!(accounting_reopen_year, super::super::dto::AccountingReopenYearArgs, FiscalYear),
        ipc_sig!(accounting_get_vat_period_totals, super::super::dto::AccountingGetVatPeriodTotalsArgs, VatPeriodTotals),
        ipc_sig!(accounting_submit_vat_settlement, super::super::dto::AccountingSubmitVatSettlementArgs, JournalEntry),
        ipc_sig!(accounting_pay_vat_settlement_now, super::super::dto::AccountingPayVatSettlementNowArgs, JournalEntry),
    ]
}
