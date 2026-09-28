//! Recurring expense templates (03-domains/11-expenses.md §3.8-3.12) — ports
//! `expenses.ts:120-182` plus the E-D4 concurrency fix (analysis §5).

use chrono::{Datelike, NaiveDate};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::expenses::expense_categories::{Column as CategoryColumn, Entity as CategoryEntity};
use crate::entities::expenses::recurring_expenses::{ActiveModel, Column, Entity};
use crate::entities::soft_delete::SoftDelete;
use crate::shared::activity;
use crate::shared::ledger::period::assert_open_period;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

use super::super::dto::{paid_from_from_dto, recurring_to_dto, Expense, ExpenseInput, RecurringExpense, RecurringExpenseInput};
use super::expenses::record_expense;

const TEMPLATE_NOT_FOUND: &str = "القالب غير موجود";

/// **3.8 `get_recurring_expenses`** — live rows in insertion order.
pub async fn get_recurring_expenses<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<RecurringExpense>> {
    let rows = Entity::find_live().order_by_asc(Column::CreatedAt).order_by_asc(Column::Id).all(conn).await.map_err(TxError::from)?;
    Ok(rows.iter().map(recurring_to_dto).collect())
}

/// **3.9 `save_recurring_expense(input, id)`** — `expenses.ts:136-149`.
pub async fn save_recurring_expense<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    input: RecurringExpenseInput,
    id: Option<Id>,
) -> TxResult<RecurringExpense> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(TxError::App(AppError::validation("الاسم مطلوب")));
    }
    let category_exists = CategoryEntity::find_live().filter(CategoryColumn::Id.eq(input.category_id)).one(conn).await.map_err(TxError::from)?.is_some();
    if !category_exists {
        return Err(TxError::App(AppError::validation("اختر تصنيف المصروف")));
    }
    if !(1..=28).contains(&input.day) {
        return Err(TxError::App(AppError::validation("يوم الاستحقاق بين 1 و 28")));
    }
    let next_date = NaiveDate::parse_from_str(&input.next_date, "%Y-%m-%d").map_err(|_| AppError::validation("التاريخ غير صالح"))?;

    let (paid_from_kind, paid_from_method_id, paid_from_supplier_id) = paid_from_from_dto(input.paid_from);
    let paid_from_kind = super::super::dto::expenses_paid_from_kind_to_recurring(paid_from_kind);

    let saved = if let Some(id) = id {
        lock::for_update_by_id(conn, "recurring_expenses", &id.to_string()).await.map_err(TxError::from)?;
        let existing = Entity::find_live().filter(Column::Id.eq(id)).one(conn).await.map_err(TxError::from)?;
        let Some(existing) = existing else { return Err(TxError::App(AppError::not_found(TEMPLATE_NOT_FOUND))) };

        let mut model: ActiveModel = existing.into();
        model.name = Set(name);
        model.category_id = Set(input.category_id);
        model.amount = Set(input.amount);
        model.is_tax_invoice = Set(input.is_tax_invoice);
        model.paid_from_kind = Set(paid_from_kind);
        model.paid_from_payment_method_id = Set(paid_from_method_id);
        model.paid_from_supplier_id = Set(paid_from_supplier_id);
        model.day = Set(input.day as i8);
        model.next_date = Set(next_date);
        model.auto_post = Set(input.auto_post);
        model.active = Set(input.active);
        if input.tax_id.is_some() {
            model.tax_id = Set(input.tax_id);
        }
        if input.description.is_some() {
            model.description = Set(input.description.clone());
        }
        model.updated_at = Set(cx.clock.now);
        model.update(conn).await.map_err(TxError::from)?
    } else {
        let model = ActiveModel {
            id: Set(Id::new()),
            name: Set(name),
            category_id: Set(input.category_id),
            amount: Set(input.amount),
            is_tax_invoice: Set(input.is_tax_invoice),
            tax_id: Set(input.tax_id),
            paid_from_kind: Set(paid_from_kind),
            paid_from_payment_method_id: Set(paid_from_method_id),
            paid_from_supplier_id: Set(paid_from_supplier_id),
            description: Set(input.description.clone()),
            day: Set(input.day as i8),
            next_date: Set(next_date),
            auto_post: Set(input.auto_post),
            active: Set(input.active),
            created_at: Set(cx.clock.now),
            updated_at: Set(cx.clock.now),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::expenses::recurring_expenses::SyncStatus::Local),
        };
        model.insert(conn).await.map_err(TxError::from)?
    };

    Ok(recurring_to_dto(&saved))
}

/// **3.10 `delete_recurring_expense(id)`**.
pub async fn delete_recurring_expense<C: ConnectionTrait>(conn: &C, cx: &TxCtx, id: Id) -> TxResult<()> {
    lock::for_update_by_id(conn, "recurring_expenses", &id.to_string()).await.map_err(TxError::from)?;
    let existing = Entity::find_live().filter(Column::Id.eq(id)).one(conn).await.map_err(TxError::from)?;
    if existing.is_none() {
        return Err(TxError::App(AppError::not_found(TEMPLATE_NOT_FOUND)));
    }
    Entity::soft_delete(conn, id, cx.clock.now).await.map_err(TxError::from)?;
    Ok(())
}

/// **3.11 `due_recurring_expenses`** — `pub` so 14-analytics' `recurring-expense-due` insight rule
/// can call this directly instead of re-deriving the filter (analysis §7).
pub async fn due_recurring_expenses<C: ConnectionTrait>(conn: &C, today: NaiveDate) -> TxResult<Vec<RecurringExpense>> {
    let rows = Entity::find_live()
        .filter(Column::Active.eq(true))
        .filter(Column::NextDate.lte(today))
        .order_by_asc(Column::CreatedAt)
        .order_by_asc(Column::Id)
        .all(conn)
        .await
        .map_err(TxError::from)?;
    Ok(rows.iter().map(recurring_to_dto).collect())
}

/// `nextMonthDate` (`expenses.ts:130-134`): year/month of `next_date` plus one month (December
/// rolls to January of the next year), day `min(day, 28)` — always a valid date.
fn next_month_date(day: i8, from: NaiveDate) -> NaiveDate {
    let (year, month) = if from.month() == 12 { (from.year() + 1, 1) } else { (from.year(), from.month() + 1) };
    let clamped_day = (day as u32).min(28);
    NaiveDate::from_ymd_opt(year, month, clamped_day).expect("year/month/day(<=28) is always a valid calendar date")
}

/// **3.12 `post_due_recurring_expense(id)`** — `expenses.ts:163-182` plus the E-D4 fix.
pub async fn post_due_recurring_expense<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::undo::UndoRegistry,
    id: Id,
) -> TxResult<Expense> {
    // 1. Read the template (live, no lock yet).
    let template = Entity::find_live().filter(Column::Id.eq(id)).one(conn).await.map_err(TxError::from)?.ok_or_else(|| AppError::not_found(TEMPLATE_NOT_FOUND))?;

    // 2. Build the ExpenseInput from the template's current values.
    let today = cx.clock.today();
    let date = DocDate { day: today, instant: Some(cx.clock.now) };
    let paid_from = super::super::dto::paid_from_to_dto(
        super::super::dto::recurring_paid_from_kind_to_expenses(template.paid_from_kind.clone()),
        template.paid_from_payment_method_id,
        template.paid_from_supplier_id,
    );
    let input = ExpenseInput {
        date: date.key(),
        category_id: template.category_id,
        amount: template.amount,
        is_tax_invoice: template.is_tax_invoice,
        tax_id: template.tax_id,
        supplier_vat_number: None,
        supplier_invoice_no: None,
        cost_center_id: None,
        paid_from,
        description: template.description.clone(),
        attachment_ids: None,
        repeat_monthly: Some(true),
        recurring_template_id: Some(id),
    };

    // 3. Period locks, then re-verify "active and due" under the template's row lock (E-D4).
    assert_open_period(conn, &today, false).await?;
    lock::for_update_by_id(conn, "recurring_expenses", &id.to_string()).await.map_err(TxError::from)?;
    let template = Entity::find_live().filter(Column::Id.eq(id)).one(conn).await.map_err(TxError::from)?;
    let still_due = template.as_ref().map(|t| t.active && t.next_date <= today).unwrap_or(false);
    let Some(template) = template else {
        return Err(TxError::App(AppError::conflict("المصروف المتكرر لم يعد مستحقاً — ربما سُجّل من جهاز آخر")));
    };
    if !still_due {
        return Err(TxError::App(AppError::conflict("المصروف المتكرر لم يعد مستحقاً — ربما سُجّل من جهاز آخر")));
    }

    // 4. Record the expense.
    let expense = record_expense(conn, cx, registry, input, Some(id)).await?;

    // 5. Advance `next_date`.
    let advanced = next_month_date(template.day, template.next_date);
    let mut model: ActiveModel = template.into();
    model.next_date = Set(advanced);
    model.updated_at = Set(cx.clock.now);
    model.update(conn).await.map_err(TxError::from)?;

    Ok(expense)
}
