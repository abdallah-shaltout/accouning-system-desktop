//! `expenses` table entities (21.02-B, owner B2). Declare each table as `pub mod <table>;`.

pub mod expense_categories;
pub mod expenses;
pub mod recurring_expenses;

pub use expense_categories::Entity as ExpenseCategories;
pub use expenses::Entity as Expenses;
pub use recurring_expenses::Entity as RecurringExpenses;
