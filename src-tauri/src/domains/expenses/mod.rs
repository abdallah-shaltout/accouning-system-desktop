//! `expenses` domain (03-domains/11-expenses.md) — expense categories, one-shot expenses,
//! recurring-expense templates. 11 IPC commands (§1); no undo compensators (§5 — not undoable via
//! the registry, a posted expense is corrected by a manual journal entry).

pub mod commands;
pub mod dto;
pub mod service;

use crate::core::ipc::IpcSig;
use crate::ipc_sig;

/// §1's 11 commands, in table order.
pub fn ipc_signatures() -> Vec<IpcSig> {
    vec![
        ipc_sig!(expenses_get_expense_categories, (), Vec<dto::ExpenseCategory>),
        ipc_sig!(expenses_save_expense_category, dto::ExpensesSaveExpenseCategoryArgs, dto::ExpenseCategory),
        ipc_sig!(expenses_delete_expense_category, dto::ExpensesDeleteExpenseCategoryArgs, ()),
        ipc_sig!(expenses_get_expenses, dto::ExpensesGetExpensesArgs, Vec<dto::ExpenseRow>),
        ipc_sig!(expenses_get_expense, dto::ExpensesGetExpenseArgs, dto::ExpenseRow),
        ipc_sig!(expenses_create_expense, dto::ExpensesCreateExpenseArgs, dto::Expense),
        ipc_sig!(expenses_get_recurring_expenses, (), Vec<dto::RecurringExpense>),
        ipc_sig!(expenses_save_recurring_expense, dto::ExpensesSaveRecurringExpenseArgs, dto::RecurringExpense),
        ipc_sig!(expenses_delete_recurring_expense, dto::ExpensesDeleteRecurringExpenseArgs, ()),
        ipc_sig!(expenses_get_due_recurring_expenses, (), Vec<dto::RecurringExpense>),
        ipc_sig!(expenses_post_due_recurring_expense, dto::ExpensesPostDueRecurringExpenseArgs, dto::Expense),
    ]
}

/// G-8a: this domain's DTO exports (§2) — the manager calls this one line from the top-level
/// `domains::export_bindings` hook (`domains/mod.rs`, manager-owned) in the same commit that adds
/// `pub mod expenses;` there.
pub fn export_bindings(cfg: &ts_rs::Config) {
    use ts_rs::TS;
    dto::ExpenseCategory::export_all(cfg).expect("export ExpenseCategory");
    dto::ExpenseCategoryInput::export_all(cfg).expect("export ExpenseCategoryInput");
    dto::ExpensePaidFrom::export_all(cfg).expect("export ExpensePaidFrom");
    dto::Expense::export_all(cfg).expect("export Expense");
    dto::ExpenseRow::export_all(cfg).expect("export ExpenseRow");
    dto::ExpenseInput::export_all(cfg).expect("export ExpenseInput");
    dto::ExpenseFilter::export_all(cfg).expect("export ExpenseFilter");
    dto::RecurringExpense::export_all(cfg).expect("export RecurringExpense");
    dto::RecurringExpenseInput::export_all(cfg).expect("export RecurringExpenseInput");
    dto::ExpensesSaveExpenseCategoryArgs::export_all(cfg).expect("export ExpensesSaveExpenseCategoryArgs");
    dto::ExpensesDeleteExpenseCategoryArgs::export_all(cfg).expect("export ExpensesDeleteExpenseCategoryArgs");
    dto::ExpensesGetExpensesArgs::export_all(cfg).expect("export ExpensesGetExpensesArgs");
    dto::ExpensesGetExpenseArgs::export_all(cfg).expect("export ExpensesGetExpenseArgs");
    dto::ExpensesCreateExpenseArgs::export_all(cfg).expect("export ExpensesCreateExpenseArgs");
    dto::ExpensesSaveRecurringExpenseArgs::export_all(cfg).expect("export ExpensesSaveRecurringExpenseArgs");
    dto::ExpensesDeleteRecurringExpenseArgs::export_all(cfg).expect("export ExpensesDeleteRecurringExpenseArgs");
    dto::ExpensesPostDueRecurringExpenseArgs::export_all(cfg).expect("export ExpensesPostDueRecurringExpenseArgs");
}
