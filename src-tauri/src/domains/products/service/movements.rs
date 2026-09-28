//! `domains::products::service::movements` — stock ledger reads (06b-inventory.md §3 C-M1/C-M2).

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::dto::{PagedQuery, PagedResult, SortDir};
use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::catalog::products::Entity as ProductEntity;
use crate::entities::inventory::stock_movements::{Column as MovementColumn, Entity as MovementEntity, Model as MovementModel};
use crate::entities::purchases::purchase_returns::Entity as PurchaseReturnEntity;
use crate::entities::sales::refunds::Entity as RefundEntity;
use crate::utils::id::Id;
use crate::utils::route::RouteRef;

use super::super::dto::inventory::{MovementFilter, StockMovement, StockMovementReason, StockMovementRow};

fn to_dto(m: &MovementModel) -> StockMovement {
    StockMovement {
        id: m.id,
        date: m.date().key(),
        product_id: m.product_id,
        qty_change: m.qty_change,
        value_change: m.value_change,
        reason: StockMovementReason::parse(&m.reason).unwrap_or(StockMovementReason::StockIn),
        ref_id: m.ref_id,
        ref_number: m.ref_number.clone(),
        balance_after: m.balance_after,
        batch_id: m.batch_id,
    }
}

/// `refLink` (`:141-158`, Q-I4): `sale` -> invoice detail; `refund` -> its invoice (absent when the
/// refund row is missing); `purchase` -> purchase detail; `purchase_return` -> its purchase order
/// (absent when the return row is missing); any other reason -> `adjustment` detail.
async fn ref_link<C: ConnectionTrait>(conn: &C, m: &MovementModel) -> TxResult<Option<RouteRef>> {
    match m.reason.as_str() {
        "sale" => Ok(Some(RouteRef::detail("invoice", m.ref_id.to_string()))),
        "refund" => {
            let refund = RefundEntity::find_by_id(m.ref_id).one(conn).await.map_err(AppError::from)?;
            Ok(refund.map(|r| RouteRef::detail("invoice", r.invoice_id.to_string())))
        }
        "purchase" => Ok(Some(RouteRef::detail("purchase", m.ref_id.to_string()))),
        "purchase_return" => {
            let ret = PurchaseReturnEntity::find_by_id(m.ref_id).one(conn).await.map_err(AppError::from)?;
            Ok(ret.map(|r| RouteRef::detail("purchase", r.purchase_order_id.to_string())))
        }
        _ => Ok(Some(RouteRef::detail("adjustment", m.ref_id.to_string()))),
    }
}

async fn product_name<C: ConnectionTrait>(conn: &C, product_id: Id) -> TxResult<String> {
    let p = ProductEntity::find_by_id(product_id).one(conn).await.map_err(AppError::from)?;
    Ok(p.map(|p| p.name).unwrap_or_else(|| "—".to_string()))
}

fn apply_filter(mut query: sea_orm::Select<MovementEntity>, filter: &MovementFilter) -> sea_orm::Select<MovementEntity> {
    if let Some(pid) = filter.product_id {
        query = query.filter(MovementColumn::ProductId.eq(pid));
    }
    if let Some(reason) = filter.reason {
        query = query.filter(MovementColumn::Reason.eq(reason.as_str()));
    }
    query
}

/// **C-M1 `get_stock_movements(filter)`** (`:96-107`).
pub async fn get_stock_movements<C: ConnectionTrait>(conn: &C, filter: Option<MovementFilter>) -> TxResult<Vec<StockMovementRow>> {
    let filter = filter.unwrap_or(MovementFilter { product_id: None, reason: None, from: None, to: None });
    let query = apply_filter(MovementEntity::find(), &filter);
    let mut rows = query.all(conn).await.map_err(AppError::from)?;
    rows.retain(|m| crate::utils::dates::in_date_range(m.date_day, filter.from.as_deref(), filter.to.as_deref()));
    rows.sort_by(|a, b| b.date().key().cmp(&a.date().key()));

    let mut out = Vec::with_capacity(rows.len());
    for m in &rows {
        let name = product_name(conn, m.product_id).await?;
        let link = ref_link(conn, m).await?;
        out.push(StockMovementRow::new(to_dto(m), name, link));
    }
    Ok(out)
}

/// **C-M2 `get_stock_movements_paged(query)`** (`:112-139`).
pub async fn get_stock_movements_paged<C: ConnectionTrait>(conn: &C, query: PagedQuery<MovementFilter>) -> TxResult<PagedResult<StockMovementRow>> {
    let filter = query.filters.clone().unwrap_or(MovementFilter { product_id: None, reason: None, from: None, to: None });
    let base_query = apply_filter(MovementEntity::find(), &filter);
    let mut rows = base_query.all(conn).await.map_err(AppError::from)?;
    rows.retain(|m| crate::utils::dates::in_date_range(m.date_day, filter.from.as_deref(), filter.to.as_deref()));

    let mut dtos = Vec::with_capacity(rows.len());
    for m in &rows {
        let name = product_name(conn, m.product_id).await?;
        let link = ref_link(conn, m).await?;
        dtos.push(StockMovementRow::new(to_dto(m), name, link));
    }

    let total = dtos.len() as u32;

    match &query.sort {
        Some(sort) => {
            let dir = sort.dir.clone();
            dtos.sort_by(|a, b| {
                let ord = match sort.key.as_str() {
                    "qtyChange" => a.qty_change.cmp(&b.qty_change),
                    "valueChange" => a.value_change.cmp(&b.value_change),
                    "balanceAfter" => a.balance_after.unwrap_or_default().cmp(&b.balance_after.unwrap_or_default()),
                    "date" => a.date.cmp(&b.date),
                    "productName" => a.product_name.cmp(&b.product_name),
                    "reason" => a.reason.as_str().cmp(b.reason.as_str()),
                    "refNumber" => a.ref_number.as_deref().unwrap_or("").cmp(b.ref_number.as_deref().unwrap_or("")),
                    _ => std::cmp::Ordering::Equal,
                };
                match &dir {
                    SortDir::Asc => ord,
                    SortDir::Desc => ord.reverse(),
                }
            });
        }
        None => dtos.sort_by(|a, b| b.date.cmp(&a.date)),
    }

    let page = query.page.max(1) as usize;
    let page_size = query.page_size.max(1) as usize;
    let start = (page - 1) * page_size;
    let page_rows = dtos.into_iter().skip(start).take(page_size).collect();

    Ok(PagedResult { rows: page_rows, total, totals: None })
}
