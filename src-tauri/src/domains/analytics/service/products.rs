//! `analytics_get_product_analytics` (14-analytics.md §3.2, port of `analyticsService.ts:129-167`).

use indexmap::IndexMap;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::catalog::products::Entity as ProductEntity;
use crate::entities::sales::invoice_lines::{Column as LineColumn, Entity as LineEntity};
use crate::entities::sales::invoices::{Column as InvoiceColumn, Entity as InvoiceEntity, InvoiceStatus};
use crate::utils::money::round2;

use super::super::dto::{ProductAnalytics, ProductProfitRow};
use super::common::{js_num, validate_days, validate_limit};

/// 13b §3.0 line-key convention: a product line keys by its `product_id`; a free-text line keys by
/// `freetext-<position>` (never grouped with another free-text line at a different position).
fn line_key(product_id: Option<crate::utils::id::Id>, position: i16) -> String {
    match product_id {
        Some(id) => id.to_string(),
        None => format!("freetext-{position}"),
    }
}

pub async fn get_product_analytics<C: ConnectionTrait>(conn: &C, today: chrono::NaiveDate, days: Option<i32>, limit: Option<i32>) -> TxResult<ProductAnalytics> {
    let days = validate_days(days)?;
    let limit = validate_limit(limit, 8)? as usize;
    let cutoff = today - chrono::Duration::days(days as i64);

    let recent = InvoiceEntity::find()
        .filter(InvoiceColumn::Status.ne(InvoiceStatus::Draft))
        .filter(InvoiceColumn::DateDay.gte(cutoff))
        .filter(InvoiceColumn::DeletedAt.is_null())
        .order_by_asc(InvoiceColumn::CreatedAt)
        .order_by_asc(InvoiceColumn::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let recent_ids: Vec<_> = recent.iter().map(|i| i.id).collect();
    let lines = if recent_ids.is_empty() {
        Vec::new()
    } else {
        LineEntity::find()
            .filter(LineColumn::InvoiceId.is_in(recent_ids))
            .order_by_asc(LineColumn::Position)
            .all(conn)
            .await
            .map_err(AppError::from)?
    };

    struct Agg {
        qty: Decimal,
        revenue: Decimal,
        profit: Decimal,
    }

    let mut by_product: IndexMap<String, Agg> = IndexMap::new();
    for line in &lines {
        let net = line.net.unwrap_or_else(|| round2(line.qty * line.price - line.discount));
        let key = line_key(line.product_id, line.position);
        let entry = by_product.entry(key).or_insert(Agg { qty: Decimal::ZERO, revenue: Decimal::ZERO, profit: Decimal::ZERO });
        entry.qty += line.qty;
        entry.revenue += net;
        entry.profit += net - line.qty * line.cost_price;
    }

    let product_ids: Vec<crate::utils::id::Id> = by_product.keys().filter_map(|k| k.parse::<crate::utils::id::Id>().ok()).collect();
    let products = if product_ids.is_empty() {
        Vec::new()
    } else {
        ProductEntity::find().filter(crate::entities::catalog::products::Column::Id.is_in(product_ids)).all(conn).await.map_err(AppError::from)?
    };

    let rows: Vec<ProductProfitRow> = by_product
        .into_iter()
        .map(|(key, v)| {
            let product = key.parse::<crate::utils::id::Id>().ok().and_then(|id| products.iter().find(|p| p.id == id));
            let revenue = round2(v.revenue);
            ProductProfitRow {
                id: key,
                name: product.map(|p| p.name.clone()).unwrap_or_else(|| "—".to_string()),
                sku: product.map(|p| p.sku.clone()).unwrap_or_default(),
                qty: round2(v.qty),
                revenue,
                gross_profit: round2(v.profit),
                margin_pct: if revenue > Decimal::ZERO { round2((v.profit / revenue) * Decimal::from(100)) } else { Decimal::ZERO },
            }
        })
        .collect();

    let mut sorted = rows;
    sorted.sort_by(|a, b| b.gross_profit.cmp(&a.gross_profit));
    let top: Vec<ProductProfitRow> = sorted.iter().take(limit).cloned().collect();
    // `slice(-limit).reverse()` in JS: the bottom `limit` items of the desc-sorted array, reversed
    // (so the worst is first) — exactly `sorted.iter().rev().take(limit)`.
    let bottom: Vec<ProductProfitRow> = sorted.iter().rev().take(limit).cloned().collect();

    let insight = match top.first() {
        Some(first) => format!("\"{}\" هو الأعلى ربحاً بإجمالي {} ر.س خلال آخر {} يوماً", first.name, js_num(round2(first.gross_profit)), days),
        None => "لا توجد مبيعات كافية بعد لهذه الفترة".to_string(),
    };

    Ok(ProductAnalytics { top, bottom, insight })
}
