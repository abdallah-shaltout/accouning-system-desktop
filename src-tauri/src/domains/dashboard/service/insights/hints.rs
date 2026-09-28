//! `dashboard_get_product_inline_hints` (14b-insights.md §3.2, port of `insightEngine.ts:164-225`).

use rust_decimal::{Decimal, RoundingStrategy};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::auth::Role;
use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::catalog::products::Entity as ProductEntity;
use crate::entities::sales::invoice_lines::{Column as LineColumn, Entity as LineEntity};
use crate::entities::sales::invoices::{Column as InvoiceColumn, Entity as InvoiceEntity, InvoiceStatus};
use crate::utils::route::RouteRef;

use super::super::super::dto::{InsightDto, InsightIcon, InsightSeverity};
use super::common::Thresholds;

fn round0(d: Decimal) -> Decimal {
    d.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
}

pub async fn get_product_inline_hints<C: ConnectionTrait>(
    conn: &C,
    today: chrono::NaiveDate,
    now: chrono::DateTime<chrono::Utc>,
    product_id: crate::utils::id::Id,
    actor_role: Role,
) -> TxResult<Vec<InsightDto>> {
    let Some(product) = ProductEntity::find_by_id(product_id).one(conn).await.map_err(AppError::from)? else {
        return Ok(Vec::new());
    };
    if !product.active || product.r#type != "product" {
        return Ok(Vec::new());
    }

    let thresholds = Thresholds::load(conn).await?;
    let today_key = today.format("%Y-%m-%d").to_string();
    let mut hints: Vec<InsightDto> = Vec::new();

    if product.stock_mode.as_deref() != Some("none") && product.stock_qty <= product.min_stock.unwrap_or(Decimal::ZERO) {
        hints.push(InsightDto {
            id: format!("reorder-item:{product_id}"),
            rule_key: "reorder-item".to_string(),
            severity: InsightSeverity::Warning,
            message: "هذا المنتج منخفض المخزون — عند حد الطلب أو أقل".to_string(),
            metric: None,
            action_label: "إنشاء أمر شراء".to_string(),
            action_to: RouteRef::list("purchase-new"),
            icon: InsightIcon::PackageX,
            roles: vec![Role::Storekeeper, Role::Manager, Role::Admin],
            value: Decimal::ONE,
            created_at: today_key.clone(),
        });
    }

    if product.price > Decimal::ZERO && product.stock_mode.as_deref() != Some("none") {
        let margin = if product.price <= product.cost_price {
            Decimal::from(-1)
        } else {
            ((product.price - product.cost_price) / product.price) * Decimal::from(100)
        };
        if margin < thresholds.min_margin_pct {
            hints.push(InsightDto {
                id: format!("below-cost-item:{product_id}"),
                rule_key: "below-cost-item".to_string(),
                severity: InsightSeverity::Warning,
                message: if margin < Decimal::ZERO { "سعر البيع أقل من التكلفة".to_string() } else { format!("هامش الربح ضعيف ({}%)", round0(margin)) },
                metric: None,
                action_label: "تعديل السعر".to_string(),
                action_to: RouteRef::detail("product-edit", product_id.to_string()),
                icon: InsightIcon::TrendingDown,
                roles: vec![Role::Manager, Role::Admin],
                value: Decimal::ONE,
                created_at: today_key.clone(),
            });
        }
    }

    // last sale = max raw invoice date key over non-draft invoices with a line for this product.
    let lines = LineEntity::find().filter(LineColumn::ProductId.eq(product_id)).all(conn).await.map_err(AppError::from)?;
    let invoice_ids: Vec<_> = lines.iter().map(|l| l.invoice_id).collect();
    let last_sale: Option<(String, chrono::DateTime<chrono::Utc>)> = if invoice_ids.is_empty() {
        None
    } else {
        let invoices = InvoiceEntity::find()
            .filter(InvoiceColumn::Id.is_in(invoice_ids))
            .filter(InvoiceColumn::Status.ne(InvoiceStatus::Draft))
            .all(conn)
            .await
            .map_err(AppError::from)?;
        invoices.iter().map(|i| (i.date().key(), i.date().instant.unwrap_or_else(|| i.date_day.and_hms_opt(0, 0, 0).unwrap().and_utc()))).max_by(|a, b| a.0.cmp(&b.0))
    };

    let cutoff = today - chrono::Duration::days(super::common::trunc_days(thresholds.dead_stock_days));
    let cutoff_key = cutoff.format("%Y-%m-%d").to_string();
    let stale = match &last_sale {
        Some((key, _)) => super::common::day_of_key(key) < cutoff_key.as_str(),
        None => true,
    };
    if product.stock_qty > Decimal::ZERO && product.stock_value >= thresholds.dead_stock_value && stale {
        let days_text = match &last_sale {
            Some((_, instant)) => {
                let days = (now - *instant).num_milliseconds() / 86_400_000;
                crate::utils::money::js_number_string(Decimal::from(days))
            }
            None => format!("+{}", crate::utils::money::js_number_string(thresholds.dead_stock_days)),
        };
        hints.push(InsightDto {
            id: format!("dead-stock-item:{product_id}"),
            rule_key: "dead-stock-item".to_string(),
            severity: InsightSeverity::Info,
            message: format!("لم يُبع منذ {days_text} يوماً — قيمة المخزون {} ر.س", round0(product.stock_value)),
            metric: None,
            action_label: "عرض المخزون".to_string(),
            action_to: RouteRef::list("movements"),
            icon: InsightIcon::TrendingDown,
            roles: vec![Role::Manager, Role::Admin],
            value: Decimal::ONE,
            created_at: today_key,
        });
    }

    hints.retain(|h| h.roles.contains(&actor_role));
    Ok(hints)
}
