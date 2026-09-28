//! `domains::accounting::commands::templates` — thin IPC layer (12-accounting.md §1).

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_tx, BoxFuture, TxOpts, TxResult};

use super::super::dto::{
    AccountingCreateOrUpdateJournalTemplateArgs, AccountingGetJournalTemplateArgs, AccountingLoadTemplateIntoEntryArgs, AccountingPostRecurringTemplateArgs,
    AccountingRemoveJournalTemplateArgs, JournalEntry, JournalTemplate,
};
use super::super::service;

type CmdResult<T> = Result<T, ApiErrorPayload>;

#[tauri::command]
pub async fn accounting_get_journal_templates(state: State<'_, AppState>) -> CmdResult<Vec<JournalTemplate>> {
    crate::core::tx::with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Accounting, Access::Read).await?;
            service::templates::get_journal_templates(tx).await
        }) as BoxFuture<'_, TxResult<Vec<JournalTemplate>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_get_journal_template(state: State<'_, AppState>, args: AccountingGetJournalTemplateArgs) -> CmdResult<JournalTemplate> {
    crate::core::tx::with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Accounting, Access::Read).await?;
            service::templates::get_journal_template(tx, args.id).await
        }) as BoxFuture<'_, TxResult<JournalTemplate>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_create_or_update_journal_template(state: State<'_, AppState>, args: AccountingCreateOrUpdateJournalTemplateArgs) -> CmdResult<JournalTemplate> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let id = args.id;
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            service::templates::save_journal_template(tx, cx, input, id).await
        }) as BoxFuture<'_, TxResult<JournalTemplate>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_remove_journal_template(state: State<'_, AppState>, args: AccountingRemoveJournalTemplateArgs) -> CmdResult<()> {
    let id = args.id;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            service::templates::remove_journal_template(tx, cx, id).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

/// Same service function as `accounting_get_journal_template` (12-accounting.md §1's note).
#[tauri::command]
pub async fn accounting_load_template_into_entry(state: State<'_, AppState>, args: AccountingLoadTemplateIntoEntryArgs) -> CmdResult<JournalTemplate> {
    crate::core::tx::with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Accounting, Access::Read).await?;
            service::templates::get_journal_template(tx, args.id).await
        }) as BoxFuture<'_, TxResult<JournalTemplate>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn accounting_post_recurring_template(state: State<'_, AppState>, args: AccountingPostRecurringTemplateArgs) -> CmdResult<JournalEntry> {
    let id = args.id;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            let entry = service::templates::post_recurring_template(tx, cx, &undo, id).await?;
            service::rows::entry_dto(tx, &entry).await
        }) as BoxFuture<'_, TxResult<JournalEntry>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}
