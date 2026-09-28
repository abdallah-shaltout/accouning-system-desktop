//! `cost_centers.rs` — `get_cost_centers`/`create_cost_center`/`update_cost_center`/
//! `delete_cost_center` (01-settings.md §3 "Cost centers").

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::{AppError, AppResult};
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::org::cost_center_budgets::{self, Entity as BudgetEntity};
use crate::entities::org::cost_centers::{ActiveModel, Column, Entity};
use crate::entities::platform::activity::ActivityKind;
use crate::entities::soft_delete::SoftDelete;
use crate::utils::id::Id;
use crate::utils::route::RouteRef;

use super::super::dto::{CostCenter, CostCenterBudget, CostCenterInput, CostCenterPatch};

async fn budgets_for<C: ConnectionTrait>(conn: &C, cost_center_id: Id) -> AppResult<Vec<cost_center_budgets::Model>> {
    BudgetEntity::find().filter(cost_center_budgets::Column::CostCenterId.eq(cost_center_id)).all(conn).await.map_err(AppError::from)
}

/// `getCostCenters` — list live in insertion order, with budgets.
pub async fn list<C: ConnectionTrait>(conn: &C) -> AppResult<Vec<CostCenter>> {
    let rows = Entity::find_live().order_by_asc(Column::CreatedAt).order_by_asc(Column::Id).all(conn).await.map_err(AppError::from)?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let budgets = budgets_for(conn, row.id).await?;
        out.push(CostCenter::from_model_with_budgets(row, budgets));
    }
    Ok(out)
}

async fn replace_budgets<C: ConnectionTrait>(conn: &C, cx: &TxCtx, cost_center_id: Id, budgets: &[CostCenterBudget]) -> TxResult<()> {
    BudgetEntity::delete_many().filter(cost_center_budgets::Column::CostCenterId.eq(cost_center_id)).exec(conn).await.map_err(TxError::from)?;
    for b in budgets {
        let model = cost_center_budgets::ActiveModel {
            id: Set(Id::new()),
            cost_center_id: Set(cost_center_id),
            fiscal_year_id: Set(b.fiscal_year_id),
            amount: Set(b.amount),
            created_at: Set(cx.clock.now),
            updated_at: Set(cx.clock.now),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

/// `createCostCenter` (`branches.ts:223-243`).
pub async fn create<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &crate::shared::activity::undo::UndoRegistry,
    input: CostCenterInput,
) -> TxResult<CostCenter> {
    if input.name.trim().is_empty() {
        return Err(TxError::App(AppError::validation("اسم مركز التكلفة مطلوب")));
    }
    if input.code.trim().is_empty() {
        return Err(TxError::App(AppError::validation("رمز مركز التكلفة مطلوب")));
    }
    let code_lower = input.code.trim().to_lowercase();
    let clash = Entity::find_live().all(conn).await.map_err(TxError::from)?.iter().any(|c| c.code.to_lowercase() == code_lower);
    if clash {
        return Err(TxError::App(AppError::validation("رمز مركز التكلفة مستخدم بالفعل")));
    }

    let id = Id::new();
    let model = ActiveModel {
        id: Set(id),
        code: Set(input.code.trim().to_string()),
        name: Set(input.name.trim().to_string()),
        kind: Set(input.kind.as_str().to_string()),
        parent_id: Set(input.parent_id),
        manager_user_id: Set(input.manager_user_id),
        active: Set(input.active),
        can_delete: Set(true),
        branch_id: Set(None),
        created_at: Set(cx.clock.now),
        updated_at: Set(cx.clock.now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
        code_live: Set(None),
    };
    let inserted = model.insert(conn).await.map_err(TxError::from)?;

    let budgets = input.budgets.unwrap_or_default();
    if !budgets.is_empty() {
        replace_budgets(conn, cx, id, &budgets).await?;
    }

    crate::shared::activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Settings,
        format!("إضافة مركز تكلفة \"{}\"", inserted.name),
        None,
        Some(RouteRef::list("settings-cost-centers")),
    )
    .await?;

    Ok(CostCenter::from_model_with_budgets(inserted, budgets_for(conn, id).await.map_err(TxError::from)?))
}

/// `updateCostCenter` (`branches.ts:245-251`) — `Object.assign` semantics: present keys only, no
/// validation, no trim (Q-5).
pub async fn update<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &crate::shared::activity::undo::UndoRegistry,
    id: Id,
    input: CostCenterPatch,
) -> TxResult<CostCenter> {
    let existing = Entity::find_by_id(id).one(conn).await.map_err(TxError::from)?;
    let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("مركز التكلفة غير موجود"))) };

    let mut model: ActiveModel = existing.into();
    if let Some(v) = input.name {
        model.name = Set(v);
    }
    if let Some(v) = input.code {
        model.code = Set(v);
    }
    if let Some(v) = input.kind {
        model.kind = Set(v.as_str().to_string());
    }
    if let Some(v) = input.parent_id {
        model.parent_id = Set(Some(v));
    }
    if let Some(v) = input.manager_user_id {
        model.manager_user_id = Set(Some(v));
    }
    if let Some(v) = input.active {
        model.active = Set(v);
    }
    model.updated_at = Set(cx.clock.now);
    let updated = model.update(conn).await.map_err(TxError::from)?;

    if let Some(budgets) = &input.budgets {
        replace_budgets(conn, cx, id, budgets).await?;
    }

    crate::shared::activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Settings,
        format!("تعديل مركز التكلفة \"{}\"", updated.name),
        None,
        Some(RouteRef::list("settings-cost-centers")),
    )
    .await?;

    Ok(CostCenter::from_model_with_budgets(updated, budgets_for(conn, id).await.map_err(TxError::from)?))
}

/// `deleteCostCenter` (`branches.ts:253-261`).
pub async fn delete<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &crate::shared::activity::undo::UndoRegistry, id: Id) -> TxResult<()> {
    let existing = Entity::find_by_id(id).one(conn).await.map_err(TxError::from)?;
    let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("مركز التكلفة غير موجود"))) };
    if !existing.can_delete {
        return Err(TxError::App(AppError::forbidden("لا يمكن حذف مركز تكلفة الفرع")));
    }

    let in_use = crate::entities::journal::journal_lines::Entity::find()
        .filter(crate::entities::journal::journal_lines::Column::CostCenterId.eq(id))
        .count(conn)
        .await
        .map_err(TxError::from)?
        > 0;
    if in_use {
        return Err(TxError::App(AppError::forbidden("لا يمكن حذف مركز تكلفة له حركات مرحّلة")));
    }

    Entity::soft_delete(conn, id, cx.clock.now).await.map_err(TxError::from)?;

    crate::shared::activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Settings,
        format!("حذف مركز التكلفة \"{}\"", existing.name),
        None,
        Some(RouteRef::list("settings-cost-centers")),
    )
    .await?;

    Ok(())
}
