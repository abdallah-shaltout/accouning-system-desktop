//! `expenses` DTOs (03-domains/11-expenses.md §2) — mirror `src/modules/expenses/types/index.ts`
//! and `expenseService.ts`'s local `ExpenseRow`. One args struct per command (§3.2 convention).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::entities::expenses::{expense_categories, expenses, recurring_expenses};
use crate::utils::id::Id;
use crate::utils::money::serde_number;

/// `ExpenseCategory` (`types/index.ts:7-17`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub struct ExpenseCategory {
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[ts(type = "string")]
    pub account_id: Id,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_tax_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_cost_center_id: Option<Id>,
    pub active: bool,
    pub can_delete: bool,
}

pub fn category_to_dto(m: &expense_categories::Model) -> ExpenseCategory {
    ExpenseCategory {
        id: m.id,
        name: m.name.clone(),
        icon: m.icon.clone(),
        account_id: m.account_id,
        default_tax_id: m.default_tax_id,
        default_cost_center_id: m.default_cost_center_id,
        active: m.active,
        can_delete: m.can_delete,
    }
}

/// `ExpenseCategoryInput` (`types/index.ts:20`) — `Omit<ExpenseCategory, 'id' | 'canDelete'>`
/// written out flat (Deserialize + TS).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub struct ExpenseCategoryInput {
    pub name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[ts(type = "string")]
    pub account_id: Id,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_tax_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_cost_center_id: Option<Id>,
    pub active: bool,
}

/// `ExpensePaidFrom` (`types/index.ts:23`) — per-field renames (not `rename_all_fields`) so ts-rs
/// emits the exact union shape the mock's discriminated union has.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub enum ExpensePaidFrom {
    Method {
        #[serde(rename = "paymentMethodId")]
        #[ts(type = "string")]
        payment_method_id: Id,
    },
    Credit {
        #[serde(rename = "supplierId")]
        #[ts(type = "string")]
        supplier_id: Id,
    },
}

/// `Expense` (`types/index.ts:25-42`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub struct Expense {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    /// A business day key (`YYYY-MM-DD`) or ISO instant (`DocDate::key()`) — matches the mock's
    /// plain `string` field exactly (no `DocDate` triple here; P2-09's `key()` string wire shape).
    pub date: String,
    #[ts(type = "string")]
    pub category_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    pub is_tax_invoice: bool,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<Id>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub net_amount: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub tax_amount: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier_vat_number: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier_invoice_no: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_center_id: Option<Id>,
    pub paid_from: ExpensePaidFrom,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_ids: Option<Vec<String>>,
    pub repeat_monthly: bool,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurring_template_id: Option<Id>,
    #[ts(type = "string")]
    pub created_by: Id,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<Id>,
}

/// Builds the `paidFrom` union from the entity's split columns.
pub fn paid_from_to_dto(kind: expenses::PaidFromKind, method_id: Option<Id>, supplier_id: Option<Id>) -> ExpensePaidFrom {
    match kind {
        expenses::PaidFromKind::Method => ExpensePaidFrom::Method { payment_method_id: method_id.expect("method paid_from must carry a payment_method_id") },
        expenses::PaidFromKind::Credit => ExpensePaidFrom::Credit { supplier_id: supplier_id.expect("credit paid_from must carry a supplier_id") },
    }
}

/// Splits the DTO union back into the entity's `(kind, method_id, supplier_id)` columns.
pub fn paid_from_from_dto(paid_from: ExpensePaidFrom) -> (expenses::PaidFromKind, Option<Id>, Option<Id>) {
    match paid_from {
        ExpensePaidFrom::Method { payment_method_id } => (expenses::PaidFromKind::Method, Some(payment_method_id), None),
        ExpensePaidFrom::Credit { supplier_id } => (expenses::PaidFromKind::Credit, None, Some(supplier_id)),
    }
}

/// `recurring_expenses::PaidFromKind` -> `expenses::PaidFromKind` — SeaORM generates one
/// `PaidFromKind` copy per entity (both map to the same DB enum `paid_from_kind`), so the two types
/// are structurally identical but nominally distinct; this bridges them at the domain boundary
/// where a recurring-template row's kind needs the `expenses`-flavoured DTO helpers above.
pub fn recurring_paid_from_kind_to_expenses(kind: recurring_expenses::PaidFromKind) -> expenses::PaidFromKind {
    match kind {
        recurring_expenses::PaidFromKind::Method => expenses::PaidFromKind::Method,
        recurring_expenses::PaidFromKind::Credit => expenses::PaidFromKind::Credit,
    }
}

/// `expenses::PaidFromKind` -> `recurring_expenses::PaidFromKind` (the reverse of the above), for
/// writing a `paidFrom` DTO back into a `recurring_expenses::ActiveModel`.
pub fn expenses_paid_from_kind_to_recurring(kind: expenses::PaidFromKind) -> recurring_expenses::PaidFromKind {
    match kind {
        expenses::PaidFromKind::Method => recurring_expenses::PaidFromKind::Method,
        expenses::PaidFromKind::Credit => recurring_expenses::PaidFromKind::Credit,
    }
}

pub fn expense_to_dto(m: &expenses::Model) -> Expense {
    Expense {
        id: m.id,
        number: m.number.clone(),
        date: m.date().key(),
        category_id: m.category_id,
        amount: m.amount,
        is_tax_invoice: m.is_tax_invoice,
        tax_id: m.tax_id,
        net_amount: m.net_amount,
        tax_amount: m.tax_amount,
        supplier_vat_number: m.supplier_vat_number.clone(),
        supplier_invoice_no: m.supplier_invoice_no.clone(),
        cost_center_id: m.cost_center_id,
        paid_from: paid_from_to_dto(m.paid_from_kind.clone(), m.paid_from_payment_method_id, m.paid_from_supplier_id),
        description: m.description.clone(),
        attachment_ids: m.attachment_ids.clone().map(|l| l.0),
        repeat_monthly: m.repeat_monthly,
        recurring_template_id: m.recurring_template_id,
        created_by: m.created_by,
        branch_id: m.branch_id,
    }
}

/// `ExpenseRow` (`services/expenseService.ts:23`) — flat struct: every `Expense` field +
/// `categoryName` (no `#[serde(flatten)]`, so the generated type is a plain object `Equals` can
/// match).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub struct ExpenseRow {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    pub date: String,
    #[ts(type = "string")]
    pub category_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    pub is_tax_invoice: bool,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<Id>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub net_amount: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub tax_amount: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier_vat_number: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier_invoice_no: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_center_id: Option<Id>,
    pub paid_from: ExpensePaidFrom,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_ids: Option<Vec<String>>,
    pub repeat_monthly: bool,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurring_template_id: Option<Id>,
    #[ts(type = "string")]
    pub created_by: Id,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<Id>,
    pub category_name: String,
}

pub fn expense_row_to_dto(m: &expenses::Model, category_name: String) -> ExpenseRow {
    let e = expense_to_dto(m);
    ExpenseRow {
        id: e.id,
        number: e.number,
        date: e.date,
        category_id: e.category_id,
        amount: e.amount,
        is_tax_invoice: e.is_tax_invoice,
        tax_id: e.tax_id,
        net_amount: e.net_amount,
        tax_amount: e.tax_amount,
        supplier_vat_number: e.supplier_vat_number,
        supplier_invoice_no: e.supplier_invoice_no,
        cost_center_id: e.cost_center_id,
        paid_from: e.paid_from,
        description: e.description,
        attachment_ids: e.attachment_ids,
        repeat_monthly: e.repeat_monthly,
        recurring_template_id: e.recurring_template_id,
        created_by: e.created_by,
        branch_id: e.branch_id,
        category_name,
    }
}

/// `ExpenseInput` (`types/index.ts:50-62`). `date` stays a raw string on the wire — the service
/// parses it with `RawDocDate::parse` then resolves it against the transaction's `BusinessClock`.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub struct ExpenseInput {
    pub date: String,
    #[ts(type = "string")]
    pub category_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    pub is_tax_invoice: bool,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier_vat_number: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier_invoice_no: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_center_id: Option<Id>,
    pub paid_from: ExpensePaidFrom,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_ids: Option<Vec<String>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_monthly: Option<bool>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurring_template_id: Option<Id>,
}

/// `ExpenseFilter` (`types/index.ts:66-71`) — all four fields optional strings.
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub struct ExpenseFilter {
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
}

/// `RecurringExpense` (`types/index.ts:74-87`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub struct RecurringExpense {
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    #[ts(type = "string")]
    pub category_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    pub is_tax_invoice: bool,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<Id>,
    pub paid_from: ExpensePaidFrom,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub day: i32,
    pub next_date: chrono::NaiveDate,
    pub auto_post: bool,
    pub active: bool,
}

pub fn recurring_to_dto(m: &recurring_expenses::Model) -> RecurringExpense {
    RecurringExpense {
        id: m.id,
        name: m.name.clone(),
        category_id: m.category_id,
        amount: m.amount,
        is_tax_invoice: m.is_tax_invoice,
        tax_id: m.tax_id,
        paid_from: paid_from_to_dto(recurring_paid_from_kind_to_expenses(m.paid_from_kind.clone()), m.paid_from_payment_method_id, m.paid_from_supplier_id),
        description: m.description.clone(),
        day: m.day as i32,
        next_date: m.next_date,
        auto_post: m.auto_post,
        active: m.active,
    }
}

/// `RecurringExpenseInput` (`types/index.ts:90-100`) — independent struct (the TS type does not
/// use `Omit`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub struct RecurringExpenseInput {
    pub name: String,
    #[ts(type = "string")]
    pub category_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    pub is_tax_invoice: bool,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<Id>,
    pub paid_from: ExpensePaidFrom,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub day: i32,
    pub next_date: String,
    pub auto_post: bool,
    pub active: bool,
}

// --- Command args (§1, §3.2: one struct per command, camelCase) --------------------------------

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub struct ExpensesSaveExpenseCategoryArgs {
    pub input: ExpenseCategoryInput,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub struct ExpensesDeleteExpenseCategoryArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub struct ExpensesGetExpensesArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<ExpenseFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub struct ExpensesGetExpenseArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub struct ExpensesCreateExpenseArgs {
    pub input: ExpenseInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub struct ExpensesSaveRecurringExpenseArgs {
    pub input: RecurringExpenseInput,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub struct ExpensesDeleteRecurringExpenseArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "expenses/types/gen/")]
pub struct ExpensesPostDueRecurringExpenseArgs {
    #[ts(type = "string")]
    pub id: Id,
}
