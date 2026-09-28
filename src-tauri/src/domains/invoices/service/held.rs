//! `domains::invoices::service::held` — held sales (08b §3.6).

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::sales::held_sales::{ActiveModel as HeldActiveModel, Entity as HeldEntity, Model as HeldModel};
use crate::utils::id::Id;

use super::super::dto::{HeldSale, HeldSaleInput, HeldSaleLine};

fn held_to_dto(model: &HeldModel) -> HeldSale {
    let lines: Vec<HeldSaleLine> = serde_json::from_value(model.cart.clone()).unwrap_or_default();
    HeldSale {
        id: model.id,
        label: model.label.clone(),
        terminal_id: model.terminal_id.to_string(),
        held_at: model.held_at().key(),
        held_by: model.held_by,
        customer_id: model.customer_id,
        discount_rate: model.discount_rate,
        discount_is_pct: model.discount_is_pct,
        note: model.note.clone(),
        lines,
    }
}

/// `get_held_sales`: this terminal only, newest first.
pub async fn get_held_sales<C: ConnectionTrait>(conn: &C, terminal_id: Id) -> TxResult<Vec<HeldSale>> {
    let mut rows = HeldEntity::find().filter(crate::entities::sales::held_sales::Column::TerminalId.eq(terminal_id)).all(conn).await.map_err(AppError::from)?;
    rows.sort_by(|a, b| (b.held_at_instant, b.held_at_day, b.created_at, b.id).cmp(&(a.held_at_instant, a.held_at_day, a.created_at, a.id)));
    Ok(rows.iter().map(held_to_dto).collect())
}

/// `hold_sale`: no validation, no audit, no events.
pub async fn hold_sale<C: ConnectionTrait>(conn: &C, cx: &TxCtx, input: HeldSaleInput) -> TxResult<HeldSale> {
    let now = cx.clock.now;
    let held_by = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;
    let cart = serde_json::to_value(&input.lines).expect("HeldSaleLine always serializes");
    let id = Id::new();
    let am = HeldActiveModel {
        id: Set(id),
        label: Set(input.label.clone()),
        terminal_id: Set(cx.terminal_id),
        held_at_day: Set(cx.clock.today()),
        held_at_instant: Set(Some(now)),
        held_by: Set(held_by),
        customer_id: Set(input.customer_id),
        discount_rate: Set(input.discount_rate),
        discount_is_pct: Set(input.discount_is_pct),
        note: Set(input.note.clone()),
        cart: Set(cart),
        created_at: Set(now),
        updated_at: Set(now),
        sync_status: Set(crate::entities::sales::held_sales::SyncStatus::Local),
    };
    let inserted = am.insert(conn).await.map_err(AppError::from)?;
    Ok(held_to_dto(&inserted))
}

/// `resume_held_sale`: lock, missing -> NOT_FOUND, hard-delete, return the DTO read before delete.
pub async fn resume_held_sale<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<HeldSale> {
    lock::for_update_by_id(conn, "held_sales", &id.to_string()).await.map_err(AppError::from)?;
    let held = HeldEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("لا يوجد بيع معلّق بهذا المعرف"))?;
    let dto = held_to_dto(&held);
    HeldEntity::delete_by_id(id).exec(conn).await.map_err(AppError::from)?;
    Ok(dto)
}

/// `discard_held_sale`: idempotent delete.
pub async fn discard_held_sale<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<()> {
    HeldEntity::delete_by_id(id).exec(conn).await.map_err(AppError::from)?;
    Ok(())
}

