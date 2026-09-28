//! `reports::service::stock` (13b §3.2, 3.8, 3.9, 3.10, 3.11): inventory, low stock, dead stock,
//! stocktake variances, transfers.

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::catalog::{categories, products};
use crate::entities::inventory::{stock_count_lines, stock_counts, stock_movements, stock_transfer_lines, stock_transfers};
use crate::entities::org::branches;
use crate::entities::soft_delete::SoftDelete;
use crate::utils::dates::BusinessClock;
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::text::compare_ar;

use super::super::dto::{DateRangeInput, DeadStockRow, InventoryReportRow, LowStockRow, StockStatus, StocktakeVarianceRow, TransferReportRow};
use super::common::{self, DateRange};

fn category_name(categories: &std::collections::HashMap<Id, categories::Model>, category_id: Option<Id>) -> String {
    category_id.and_then(|id| categories.get(&id)).map(|c| c.name.clone()).unwrap_or_else(|| "بدون تصنيف".to_string())
}

/// **`reports_get_inventory_report`** (13b §3.2).
pub async fn inventory_report<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<InventoryReportRow>> {
    let rows = products::Entity::find_live().filter(products::Column::Type.eq("product")).filter(products::Column::Active.eq(true)).all(conn).await.map_err(AppError::from)?;
    let categories: std::collections::HashMap<Id, categories::Model> = categories::Entity::find_live().all(conn).await.map_err(AppError::from)?.into_iter().map(|c| (c.id, c)).collect();

    let mut out: Vec<InventoryReportRow> = rows
        .into_iter()
        .map(|p| {
            let min_stock = p.min_stock.unwrap_or(Decimal::ZERO);
            let status = if p.stock_qty <= Decimal::ZERO { StockStatus::Out } else if p.stock_qty <= min_stock { StockStatus::Low } else { StockStatus::Ok };
            InventoryReportRow {
                product_id: p.id,
                name: p.name,
                sku: p.sku,
                category: category_name(&categories, p.category_id),
                qty: p.stock_qty,
                min_stock,
                cost_price: p.cost_price,
                price: p.price,
                cost_value: round2(p.stock_qty * p.cost_price),
                retail_value: round2(p.stock_qty * p.price),
                status,
            }
        })
        .collect();
    out.sort_by(|a, b| compare_ar(&a.category, &b.category).then_with(|| compare_ar(&a.name, &b.name)));
    Ok(out)
}

/// **`reports_get_low_stock_report`** (13b §3.8).
pub async fn low_stock_report<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<LowStockRow>> {
    let rows = products::Entity::find_live().filter(products::Column::Type.eq("product")).filter(products::Column::Active.eq(true)).all(conn).await.map_err(AppError::from)?;
    let categories: std::collections::HashMap<Id, categories::Model> = categories::Entity::find_live().all(conn).await.map_err(AppError::from)?.into_iter().map(|c| (c.id, c)).collect();

    let mut out: Vec<LowStockRow> = rows
        .into_iter()
        .filter(|p| p.stock_qty <= p.min_stock.unwrap_or(Decimal::ZERO))
        .map(|p| {
            let min_stock = p.min_stock.unwrap_or(Decimal::ZERO);
            let reorder_qty = p.reorder_qty.unwrap_or(Decimal::ZERO);
            let branch_a = if p.reorder_qty.is_some() { reorder_qty } else { min_stock * Decimal::from(2) };
            let branch_b = min_stock - p.stock_qty + reorder_qty;
            let suggested_qty = branch_a.max(branch_b);
            LowStockRow {
                product_id: p.id,
                name: p.name,
                sku: p.sku,
                category: category_name(&categories, p.category_id),
                qty: p.stock_qty,
                min_stock,
                suggested_qty,
                cost_value: round2(p.stock_qty * p.cost_price),
            }
        })
        .collect();
    out.sort_by(|a, b| a.qty.cmp(&b.qty));
    Ok(out)
}

/// **`reports_get_dead_stock_report`** (13b §3.9).
pub async fn dead_stock_report<C: ConnectionTrait>(conn: &C, days: Option<i32>, clock: &BusinessClock) -> TxResult<Vec<DeadStockRow>> {
    let days = days.unwrap_or(60);
    if days < 0 {
        return Err(AppError::validation("عدد الأيام غير صالح").into());
    }
    let today = clock.today();

    // The mock's own "last `reason === 'sale'` movement per product" is a full-table max-scan, not
    // an indexed lookup (A§9 perf note) — loading the (product_id, date_key) pairs and reducing to a
    // max in Rust reproduces it exactly and keeps this file free of a raw `GROUP BY`/`MAX()` query
    // builder, matching every other aggregate in this codebase (`shared::invariants::ledger`).
    let sale_rows: Vec<(Id, String)> = stock_movements::Entity::find()
        .filter(stock_movements::Column::Reason.eq("sale"))
        .select_only()
        .column(stock_movements::Column::ProductId)
        .column(stock_movements::Column::DateKey)
        .into_tuple()
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let mut last_sale: std::collections::HashMap<Id, String> = std::collections::HashMap::new();
    for (product_id, key) in sale_rows {
        let entry = last_sale.entry(product_id).or_insert_with(|| key.clone());
        if key > *entry {
            *entry = key;
        }
    }

    let rows = products::Entity::find_live()
        .filter(products::Column::Type.eq("product"))
        .filter(products::Column::Active.eq(true))
        .filter(products::Column::StockQty.gt(Decimal::ZERO))
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let categories: std::collections::HashMap<Id, categories::Model> = categories::Entity::find_live().all(conn).await.map_err(AppError::from)?.into_iter().map(|c| (c.id, c)).collect();

    let mut out: Vec<DeadStockRow> = rows
        .into_iter()
        .map(|p| {
            let last_sale_date = last_sale.get(&p.id).cloned();
            let days_since_sale = match &last_sale_date {
                Some(key) => common::days_between(today, common::day_of_key(key, clock)),
                None => 9_007_199_254_740_991,
            };
            DeadStockRow {
                product_id: p.id,
                name: p.name,
                sku: p.sku,
                category: category_name(&categories, p.category_id),
                qty: p.stock_qty,
                cost_value: round2(p.stock_qty * p.cost_price),
                last_sale_date,
                days_since_sale,
            }
        })
        .filter(|r| r.days_since_sale >= days as i64)
        .collect();
    out.sort_by(|a, b| b.cost_value.cmp(&a.cost_value));
    Ok(out)
}

/// **`reports_get_stocktake_variances`** (13b §3.10).
pub async fn stocktake_variances<C: ConnectionTrait>(conn: &C, count_id: Option<Id>) -> TxResult<Vec<StocktakeVarianceRow>> {
    let mut query = stock_counts::Entity::find().filter(stock_counts::Column::Status.eq("COMPLETED"));
    if let Some(id) = count_id {
        query = query.filter(stock_counts::Column::Id.eq(id));
    }
    let counts = query.order_by_asc(stock_counts::Column::CreatedAt).order_by_asc(stock_counts::Column::Id).all(conn).await.map_err(AppError::from)?;

    let products_map: std::collections::HashMap<Id, products::Model> = products::Entity::find_live().all(conn).await.map_err(AppError::from)?.into_iter().map(|p| (p.id, p)).collect();
    let categories: std::collections::HashMap<Id, categories::Model> = categories::Entity::find_live().all(conn).await.map_err(AppError::from)?.into_iter().map(|c| (c.id, c)).collect();

    let mut rows = Vec::new();
    for c in &counts {
        let lines = stock_count_lines::Entity::find().filter(stock_count_lines::Column::StockCountId.eq(c.id)).order_by_asc(stock_count_lines::Column::Position).all(conn).await.map_err(AppError::from)?;
        for l in lines {
            let Some(product) = products_map.get(&l.product_id) else { continue };
            let counted_qty = l.counted_qty.unwrap_or(Decimal::ZERO);
            let system_qty = l.system_qty;
            let qty_variance = round2(counted_qty - system_qty);
            if qty_variance.abs() < Decimal::new(1, 4) {
                continue;
            }
            let value_variance = round2(qty_variance * product.cost_price);
            rows.push(StocktakeVarianceRow {
                count_id: c.id,
                count_number: c.number.clone(),
                product_id: l.product_id,
                name: product.name.clone(),
                category: category_name(&categories, product.category_id),
                counted_qty,
                system_qty,
                qty_variance,
                value_variance,
            });
        }
    }
    rows.sort_by(|a, b| b.value_variance.abs().cmp(&a.value_variance.abs()));
    Ok(rows)
}

/// **`reports_get_transfers_report`** (13b §3.11).
pub async fn transfers_report<C: ConnectionTrait>(conn: &C, range: &DateRangeInput) -> TxResult<Vec<TransferReportRow>> {
    let date_range = DateRange::parse(range)?;
    let transfers = stock_transfers::Entity::find().order_by_asc(stock_transfers::Column::CreatedAt).order_by_asc(stock_transfers::Column::Id).all(conn).await.map_err(AppError::from)?;
    let transfers: Vec<_> = transfers.into_iter().filter(|t| common::in_range(t.date_day, &date_range)).collect();

    let branch_ids: std::collections::HashSet<Id> = transfers.iter().flat_map(|t| [t.from_branch_id, t.to_branch_id]).collect();
    let branches_map: std::collections::HashMap<Id, String> = if branch_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        branches::Entity::find().filter(branches::Column::Id.is_in(branch_ids.iter().copied())).all(conn).await.map_err(AppError::from)?.into_iter().map(|b| (b.id, b.name)).collect()
    };

    let mut rows = Vec::new();
    for t in &transfers {
        let lines = stock_transfer_lines::Entity::find().filter(stock_transfer_lines::Column::StockTransferId.eq(t.id)).all(conn).await.map_err(AppError::from)?;
        let sent_qty = common::sum2(lines.iter().map(|l| l.qty));
        let received_qty = common::sum2(lines.iter().map(|l| l.received_qty.unwrap_or(if t.status == "RECEIVED" { l.qty } else { Decimal::ZERO })));
        rows.push(TransferReportRow {
            id: t.id,
            number: t.number.clone(),
            date: t.date().key(),
            from_branch: branches_map.get(&t.from_branch_id).cloned().unwrap_or_else(|| t.from_branch_id.to_string()),
            to_branch: branches_map.get(&t.to_branch_id).cloned().unwrap_or_else(|| t.to_branch_id.to_string()),
            status: t.status.clone(),
            sent_qty,
            received_qty,
            shortage_qty: round2(sent_qty - received_qty),
            shortage_value: t.shortage_value.unwrap_or(Decimal::ZERO),
        });
    }
    rows.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(rows)
}
