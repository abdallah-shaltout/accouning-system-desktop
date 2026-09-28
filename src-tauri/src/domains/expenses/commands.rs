//! `domains::expenses::commands` — thin IPC layer (11-expenses.md §1). Each command: args struct
//! (where needed), authorize, `with_tx`/`with_read`, map error. 11 commands total.

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_read, with_tx, BoxFuture, TxOpts, TxResult};

use super::dto::{
    Expense, ExpenseCategory, ExpenseRow, ExpensesCreateExpenseArgs, ExpensesDeleteExpenseCategoryArgs, ExpensesDeleteRecurringExpenseArgs,
    ExpensesGetExpenseArgs, ExpensesGetExpensesArgs, ExpensesPostDueRecurringExpenseArgs, ExpensesSaveExpenseCategoryArgs, ExpensesSaveRecurringExpenseArgs,
    RecurringExpense,
};
use super::service;

type CmdResult<T> = Result<T, ApiErrorPayload>;

// --- Categories (Settings → Expenses) -----------------------------------------------------------

/// Reachable from `ExpenseCategoriesSettingsPage.vue` (area `settings`) as well as the expense
/// form/list (area `expenses`) — both grant `Read` here, matching the mock's single unguarded
/// `getExpenseCategories` (11-expenses.md §1's "Reason for Settings/Write on category writes").
#[tauri::command]
pub async fn expenses_get_expense_categories(state: State<'_, AppState>) -> CmdResult<Vec<ExpenseCategory>> {
    with_read(&state, |tx| {
        Box::pin(async move { service::categories::get_expense_categories(tx).await }) as BoxFuture<'_, TxResult<Vec<ExpenseCategory>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn expenses_save_expense_category(state: State<'_, AppState>, args: ExpensesSaveExpenseCategoryArgs) -> CmdResult<ExpenseCategory> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let id = args.id;
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::categories::save_expense_category(tx, cx, input, id).await
        }) as BoxFuture<'_, TxResult<ExpenseCategory>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn expenses_delete_expense_category(state: State<'_, AppState>, args: ExpensesDeleteExpenseCategoryArgs) -> CmdResult<()> {
    let id = args.id;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::categories::delete_expense_category(tx, cx, id).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

// --- Expenses ------------------------------------------------------------------------------------

#[tauri::command]
pub async fn expenses_get_expenses(state: State<'_, AppState>, args: ExpensesGetExpensesArgs) -> CmdResult<Vec<ExpenseRow>> {
    with_read(&state, move |tx| {
        Box::pin(async move { service::expenses::get_expenses(tx, args.filter).await }) as BoxFuture<'_, TxResult<Vec<ExpenseRow>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn expenses_get_expense(state: State<'_, AppState>, args: ExpensesGetExpenseArgs) -> CmdResult<ExpenseRow> {
    with_read(&state, move |tx| {
        Box::pin(async move { service::expenses::get_expense(tx, args.id).await }) as BoxFuture<'_, TxResult<ExpenseRow>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn expenses_create_expense(state: State<'_, AppState>, args: ExpensesCreateExpenseArgs) -> CmdResult<Expense> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Expenses, Access::Write).await?;
            service::expenses::create_expense(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<Expense>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

// --- Recurring expenses ---------------------------------------------------------------------------

#[tauri::command]
pub async fn expenses_get_recurring_expenses(state: State<'_, AppState>) -> CmdResult<Vec<RecurringExpense>> {
    with_read(&state, |tx| {
        Box::pin(async move { service::recurring::get_recurring_expenses(tx).await }) as BoxFuture<'_, TxResult<Vec<RecurringExpense>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn expenses_save_recurring_expense(state: State<'_, AppState>, args: ExpensesSaveRecurringExpenseArgs) -> CmdResult<RecurringExpense> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let id = args.id;
        Box::pin(async move {
            cx.require(tx, Area::Expenses, Access::Write).await?;
            service::recurring::save_recurring_expense(tx, cx, input, id).await
        }) as BoxFuture<'_, TxResult<RecurringExpense>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn expenses_delete_recurring_expense(state: State<'_, AppState>, args: ExpensesDeleteRecurringExpenseArgs) -> CmdResult<()> {
    let id = args.id;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Expenses, Access::Write).await?;
            service::recurring::delete_recurring_expense(tx, cx, id).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

/// `getDueRecurringExpenses` needs "today" (`cx.clock.today()`), so unlike the other reads this
/// runs in `with_tx` rather than `with_read` (11-expenses.md §1) — it writes nothing, but
/// `with_read`'s plain `&DatabaseTransaction` has no business clock attached.
#[tauri::command]
pub async fn expenses_get_due_recurring_expenses(state: State<'_, AppState>) -> CmdResult<Vec<RecurringExpense>> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Expenses, Access::Read).await?;
            let today = cx.clock.today();
            service::recurring::due_recurring_expenses(tx, today).await
        }) as BoxFuture<'_, TxResult<Vec<RecurringExpense>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn expenses_post_due_recurring_expense(state: State<'_, AppState>, args: ExpensesPostDueRecurringExpenseArgs) -> CmdResult<Expense> {
    let id = args.id;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Expenses, Access::Write).await?;
            service::recurring::post_due_recurring_expense(tx, cx, &undo, id).await
        }) as BoxFuture<'_, TxResult<Expense>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}
