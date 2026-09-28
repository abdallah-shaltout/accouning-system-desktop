//! Expense categories (03-domains/11-expenses.md §3.1-3.3) — ports `expenses.ts:18-38`.

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::{map_unique_violation, AppError};
use crate::core::lock;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::expenses::expense_categories::{ActiveModel, Column, Entity};
use crate::entities::expenses::expenses::{Column as ExpenseColumn, Entity as ExpenseEntity};
use crate::entities::soft_delete::SoftDelete;
use crate::shared::ledger::accounts;
use crate::utils::id::Id;

use super::super::dto::{category_to_dto, ExpenseCategory, ExpenseCategoryInput};

const NAME_LIVE_CONSTRAINT: &str = "uq_expense_categories_name_live";

/// **3.1 `get_expense_categories`** — live rows in insertion order (mock array order, "unsorted").
pub async fn get_expense_categories<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<ExpenseCategory>> {
    let rows = Entity::find_live().order_by_asc(Column::CreatedAt).order_by_asc(Column::Id).all(conn).await.map_err(TxError::from)?;
    Ok(rows.iter().map(category_to_dto).collect())
}

async fn name_clashes<C: ConnectionTrait>(conn: &C, name: &str, except_id: Option<Id>) -> TxResult<bool> {
    let needle = name.to_lowercase();
    let mut q = Entity::find_live().filter(Column::Name.eq(name));
    if let Some(id) = except_id {
        q = q.filter(Column::Id.ne(id));
    }
    // The DB unique index folds on `name_live` (exact string per the migration's collation), but a
    // conservative case-insensitive pre-check here avoids a needless round trip to the DB error
    // path in the common case; the DB's own constraint (`map_unique` below) is the real gate.
    let rows = q.all(conn).await.map_err(TxError::from)?;
    Ok(rows.iter().any(|r| r.name.to_lowercase() == needle))
}

/// **3.2 `save_expense_category(input, id)`** — ports `expenses.ts:18-30`.
pub async fn save_expense_category<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    input: ExpenseCategoryInput,
    id: Option<Id>,
) -> TxResult<ExpenseCategory> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(TxError::App(AppError::validation("الاسم مطلوب")));
    }

    // Existence check runs before the id-existence check, as in the mock (`expenses.ts:19-20`).
    accounts::account_by_id(conn, input.account_id).await.map_err(TxError::from)?;

    let saved = if let Some(id) = id {
        lock::for_update_by_id(conn, "expense_categories", &id.to_string()).await.map_err(TxError::from)?;
        let existing = Entity::find_live().filter(Column::Id.eq(id)).one(conn).await.map_err(TxError::from)?;
        let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("التصنيف غير موجود"))) };

        if name_clashes(conn, &name, Some(id)).await? {
            return Err(TxError::App(AppError::conflict("اسم التصنيف مستخدم من قبل")));
        }

        let mut model: ActiveModel = existing.into();
        model.name = Set(name);
        model.account_id = Set(input.account_id);
        model.active = Set(input.active);
        if input.icon.is_some() {
            model.icon = Set(input.icon);
        }
        if input.default_tax_id.is_some() {
            model.default_tax_id = Set(input.default_tax_id);
        }
        if input.default_cost_center_id.is_some() {
            model.default_cost_center_id = Set(input.default_cost_center_id);
        }
        model
            .update(conn)
            .await
            .map_err(|e| map_unique_violation(e, NAME_LIVE_CONSTRAINT, || "اسم التصنيف مستخدم من قبل".to_string()))?
    } else {
        if name_clashes(conn, &name, None).await? {
            return Err(TxError::App(AppError::conflict("اسم التصنيف مستخدم من قبل")));
        }
        let model = ActiveModel {
            id: Set(Id::new()),
            name: Set(name),
            icon: Set(input.icon),
            account_id: Set(input.account_id),
            default_tax_id: Set(input.default_tax_id),
            default_cost_center_id: Set(input.default_cost_center_id),
            active: Set(input.active),
            can_delete: Set(true),
            created_at: Set(cx.clock.now),
            updated_at: Set(cx.clock.now),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::expenses::expense_categories::SyncStatus::Local),
            name_live: Set(None),
        };
        model
            .insert(conn)
            .await
            .map_err(|e| map_unique_violation(e, NAME_LIVE_CONSTRAINT, || "اسم التصنيف مستخدم من قبل".to_string()))?
    };

    Ok(category_to_dto(&saved))
}

/// **3.3 `delete_expense_category(id)`** — `expenses.ts:32-38`.
pub async fn delete_expense_category<C: ConnectionTrait>(conn: &C, cx: &TxCtx, id: Id) -> TxResult<()> {
    lock::for_update_by_id(conn, "expense_categories", &id.to_string()).await.map_err(TxError::from)?;
    let existing = Entity::find_live().filter(Column::Id.eq(id)).one(conn).await.map_err(TxError::from)?;
    let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("التصنيف غير موجود"))) };

    if !existing.can_delete {
        return Err(TxError::App(AppError::forbidden("لا يمكن حذف تصنيف أساسي — يمكن إلغاء تفعيله فقط")));
    }

    let in_use = ExpenseEntity::find().filter(ExpenseColumn::CategoryId.eq(id)).count(conn).await.map_err(TxError::from)? > 0;
    if in_use {
        return Err(TxError::App(AppError::conflict("لا يمكن حذف تصنيف له مصروفات مسجلة — قم بإلغاء تفعيله بدلاً من ذلك")));
    }

    Entity::soft_delete(conn, id, cx.clock.now).await.map_err(TxError::from)?;
    Ok(())
}
