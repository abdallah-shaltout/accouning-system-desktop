//! `domains::accounting::commands::journal` — thin IPC layer (12-accounting.md §1).

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::{ApiErrorPayload, PagedResult};
use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::core::tx::{with_read, with_tx, BoxFuture, TxOpts, TxResult};
use crate::utils::dates::RawDocDate;

use super::super::dto::{
    AccountingCreateJournalEntryArgs, AccountingDeleteJournalDraftArgs, AccountingGetJournalEntriesArgs, AccountingGetJournalEntriesForSourceArgs,
    AccountingGetJournalEntriesPagedArgs, AccountingGetJournalEntryArgs, AccountingPostJournalDraftArgs, AccountingReverseJournalEntryArgs,
    AccountingUpdateJournalDraftArgs, JournalEntry, JournalEntryDetail, JournalRow, LinkedJournalEntry,
};
use super::super::service;

type CmdResult<T> = Result<T, ApiErrorPayload>;

#[tauri::command]
pub async fn accounting_get_journal_entries(state: State<'_, AppState>, args: AccountingGetJournalEntriesArgs) -> CmdResult<Vec<JournalRow>> {
    crate::core::tx::with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Accounting, Access::Read).await?;
            service::journal_reads::get_journal_entries(tx, args.filter).await
        }) as BoxFuture<'_, TxResult<Vec<JournalRow>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

/// Session only (`ExpenseDetailPage.vue:12`, `VoucherDetailPage.vue:14`).
#[tauri::command]
pub async fn accounting_get_journal_entries_for_source(state: State<'_, AppState>, args: AccountingGetJournalEntriesForSourceArgs) -> CmdResult<Vec<LinkedJournalEntry>> {
    if state.session.read().unwrap().is_none() {
        return Err(ApiErrorPayload::from(AppError::unauthorized("سجّل الدخول أولاً")));
    }
    with_read(&state, move |tx| {
        Box::pin(async move { service::journal_reads::get_journal_entries_for_source(tx, &args.source_kind, &args.source_id).await }) as BoxFuture<'_, TxResult<Vec<LinkedJournalEntry>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_get_journal_entries_paged(state: State<'_, AppState>, args: AccountingGetJournalEntriesPagedArgs) -> CmdResult<PagedResult<JournalRow>> {
    crate::core::tx::with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Accounting, Access::Read).await?;
            service::journal_reads::get_journal_entries_paged(tx, args.query).await
        }) as BoxFuture<'_, TxResult<PagedResult<JournalRow>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_get_journal_entry(state: State<'_, AppState>, args: AccountingGetJournalEntryArgs) -> CmdResult<JournalEntryDetail> {
    crate::core::tx::with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Accounting, Access::Read).await?;
            service::journal_reads::get_journal_entry(tx, args.id).await
        }) as BoxFuture<'_, TxResult<JournalEntryDetail>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_create_journal_entry(state: State<'_, AppState>, args: AccountingCreateJournalEntryArgs) -> CmdResult<JournalEntry> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            service::journal::create_journal_entry(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<JournalEntry>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_update_journal_draft(state: State<'_, AppState>, args: AccountingUpdateJournalDraftArgs) -> CmdResult<JournalEntry> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let id = args.id;
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            service::journal::update_journal_draft(tx, cx, id, input).await
        }) as BoxFuture<'_, TxResult<JournalEntry>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_post_journal_draft(state: State<'_, AppState>, args: AccountingPostJournalDraftArgs) -> CmdResult<JournalEntry> {
    let id = args.id;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            service::journal::post_journal_draft(tx, cx, &undo, id).await
        }) as BoxFuture<'_, TxResult<JournalEntry>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_delete_journal_draft(state: State<'_, AppState>, args: AccountingDeleteJournalDraftArgs) -> CmdResult<()> {
    let id = args.id;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            service::journal::delete_journal_draft(tx, id).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_reverse_journal_entry(state: State<'_, AppState>, args: AccountingReverseJournalEntryArgs) -> CmdResult<JournalEntry> {
    let id = args.id;
    let date_raw = args.date;
    let reason = args.reason;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        let date_raw = date_raw.clone();
        let reason = reason.clone();
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            let parsed = RawDocDate::parse(&date_raw).map_err(|_| AppError::validation("التاريخ غير صالح"))?;
            let date = parsed.resolve(&cx.clock);
            let (entry, _audit_id) = service::journal::reverse_journal_entry(tx, cx, &undo, id, date, &reason).await?;
            Ok(entry)
        }) as BoxFuture<'_, TxResult<JournalEntry>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}
