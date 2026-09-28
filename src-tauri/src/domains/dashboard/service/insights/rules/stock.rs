//! Rules 1 (reorder), 2 (dead-stock), 3 (expiring), 11 (below-cost), 21 (missing-supplier-invoice)
//! (14b-insights.md §3.1).

use indexmap::IndexMap;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::catalog::product_batches::Entity as BatchEntity;
use crate::entities::catalog::products::{Column as ProductColumn, Entity as ProductEntity, Model as ProductModel};
use crate::entities::parties::parties::Entity as PartyEntity;
use crate::entities::purchases::purchase_orders::{Column as PoColumn, Entity as PoEntity, PurchaseStatus};
use crate::entities::sales::invoice_lines::Entity as LineEntity;
use crate::entities::sales::invoices::{Column as InvoiceColumn, Entity as InvoiceEntity, InvoiceStatus};
use crate::utils::money::round2;
use crate::utils::route::RouteRef;

use super::super::common::{day_of_key, days_between, fmt_money, fmt_num, trunc_days, InsightCtx};
use super::super::super::super::dto::{InsightDto, InsightIcon, InsightSeverity};

fn low_stock_products(rows: &[ProductModel]) -> Vec<&ProductModel> {
    rows.iter()
        .filter(|p| p.active && p.r#type == "product" && p.stock_mode.as_deref() != Some("none") && p.stock_qty <= p.min_stock.unwrap_or(Decimal::ZERO))
        .collect()
}

/// 1 — reorder (`ir:52-80`).
pub async fn reorder<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    let all = ProductEntity::find().filter(ProductColumn::DeletedAt.is_null()).all(conn).await.map_err(AppError::from)?;
    let low = low_stock_products(&all);
    if low.is_empty() {
        return Ok(Vec::new());
    }

    let mut by_supplier: IndexMap<String, Vec<&ProductModel>> = IndexMap::new();
    for p in &low {
        let key = p.preferred_supplier_id.map(|id| id.to_string()).unwrap_or_else(|| "__none__".to_string());
        by_supplier.entry(key).or_default().push(p);
    }

    let mut out = Vec::new();
    for (supplier_id, items) in by_supplier {
        let supplier = match supplier_id.parse::<crate::utils::id::Id>() {
            Ok(id) => PartyEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?,
            Err(_) => None,
        };
        let n = items.len();
        let label = match &supplier {
            Some(s) => format!("لدى مورد \"{}\"", s.name),
            None => "بلا مورد مفضل".to_string(),
        };
        let action_to = match &supplier {
            Some(_) => RouteRef::list("purchase-new").with_query("supplier", supplier_id.clone()),
            None => RouteRef::list("products").with_query("stock", "low"),
        };
        out.push(InsightDto {
            id: format!("reorder:{supplier_id}"),
            rule_key: "reorder".to_string(),
            severity: InsightSeverity::Warning,
            message: format!("{n} {} عند حد الطلب {label}", if n == 1 { "صنف" } else { "أصناف" }),
            metric: Some(format!("{} صنف", fmt_num(Decimal::from(n as i64), 0, ctx.numerals))),
            action_label: "إنشاء أمر شراء".to_string(),
            action_to,
            icon: InsightIcon::PackageX,
            roles: vec![crate::core::auth::Role::Storekeeper, crate::core::auth::Role::Manager, crate::core::auth::Role::Admin],
            value: Decimal::from((n * 100) as i64),
            created_at: ctx.today.format("%Y-%m-%d").to_string(),
        });
    }
    Ok(out)
}

/// 2 — dead-stock (`ir:83-118`).
pub async fn dead_stock<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    let cutoff = ctx.today - chrono::Duration::days(trunc_days(ctx.thresholds.dead_stock_days));
    let cutoff_key = cutoff.format("%Y-%m-%d").to_string();

    let invoices = InvoiceEntity::find()
        .filter(InvoiceColumn::Status.ne(InvoiceStatus::Draft))
        .filter(InvoiceColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let invoice_ids: Vec<_> = invoices.iter().map(|i| i.id).collect();
    let lines = if invoice_ids.is_empty() {
        Vec::new()
    } else {
        LineEntity::find().filter(crate::entities::sales::invoice_lines::Column::InvoiceId.is_in(invoice_ids)).all(conn).await.map_err(AppError::from)?
    };
    let invoice_key_by_id: std::collections::HashMap<_, _> = invoices.iter().map(|i| (i.id, i.date().key())).collect();

    let mut last_sale: std::collections::HashMap<crate::utils::id::Id, String> = std::collections::HashMap::new();
    for line in &lines {
        let Some(product_id) = line.product_id else { continue };
        let Some(key) = invoice_key_by_id.get(&line.invoice_id) else { continue };
        match last_sale.get(&product_id) {
            Some(prev) if key.as_str() <= prev.as_str() => {}
            _ => {
                last_sale.insert(product_id, key.clone());
            }
        }
    }

    let all = ProductEntity::find().filter(ProductColumn::DeletedAt.is_null()).all(conn).await.map_err(AppError::from)?;
    let stale: Vec<&ProductModel> = all
        .iter()
        .filter(|p| {
            if !p.active || p.r#type != "product" || p.stock_mode.as_deref() == Some("none") || p.stock_qty <= Decimal::ZERO {
                return false;
            }
            if p.stock_value < ctx.thresholds.dead_stock_value {
                return false;
            }
            match last_sale.get(&p.id) {
                Some(key) => day_of_key(key) < cutoff_key.as_str(),
                None => true,
            }
        })
        .collect();

    if stale.is_empty() {
        return Ok(Vec::new());
    }
    let value = round2(stale.iter().map(|p| p.stock_value).sum());
    Ok(vec![InsightDto {
        id: "dead-stock:all".to_string(),
        rule_key: "dead-stock".to_string(),
        severity: InsightSeverity::Warning,
        message: format!(
            "{} صنف بلا مبيعات منذ {} يوماً — بضاعة راكدة بقيمة {}",
            stale.len(),
            crate::utils::money::js_number_string(ctx.thresholds.dead_stock_days),
            fmt_money(value, ctx.numerals)
        ),
        metric: Some(fmt_money(value, ctx.numerals)),
        action_label: "عرض القائمة".to_string(),
        action_to: RouteRef::list("products"),
        icon: InsightIcon::TrendingDown,
        roles: vec![crate::core::auth::Role::Manager, crate::core::auth::Role::Admin],
        value,
        created_at: ctx.today.format("%Y-%m-%d").to_string(),
    }])
}

/// 3 — expiring batches (`ir:121-143`).
pub async fn expiring<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    let batches = BatchEntity::find().all(conn).await.map_err(AppError::from)?;
    let soon: Vec<_> = batches
        .iter()
        .filter(|b| {
            if b.qty <= Decimal::new(1, 4) || b.expiry_date.is_none() {
                return false;
            }
            let days = days_between(b.expiry_date.unwrap(), ctx.today);
            days >= 0 && Decimal::from(days) <= ctx.thresholds.expiry_alert_days
        })
        .collect();
    if soon.is_empty() {
        return Ok(Vec::new());
    }
    let n = soon.len();
    Ok(vec![InsightDto {
        id: "expiring:all".to_string(),
        rule_key: "expiring".to_string(),
        severity: InsightSeverity::Warning,
        message: format!(
            "{n} {} خلال {} يوماً",
            if n == 1 { "تشغيلة تنتهي" } else { "تشغيلات تنتهي" },
            crate::utils::money::js_number_string(ctx.thresholds.expiry_alert_days)
        ),
        metric: Some(format!("{} تشغيلة", fmt_num(Decimal::from(n as i64), 0, ctx.numerals))),
        action_label: "تقرير الصلاحية".to_string(),
        action_to: RouteRef::list("expiry"),
        icon: InsightIcon::TimerOff,
        roles: vec![crate::core::auth::Role::Storekeeper, crate::core::auth::Role::Manager, crate::core::auth::Role::Admin],
        value: Decimal::from((n * 80) as i64),
        created_at: ctx.today.format("%Y-%m-%d").to_string(),
    }])
}

/// 11 — below-cost prices (`ir:346-369`).
pub async fn below_cost<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    let all = ProductEntity::find().filter(ProductColumn::DeletedAt.is_null()).all(conn).await.map_err(AppError::from)?;
    let bad: Vec<_> = all
        .iter()
        .filter(|p| {
            if !p.active || p.r#type != "product" || p.price <= Decimal::ZERO {
                return false;
            }
            if p.price <= p.cost_price {
                return true;
            }
            let margin = ((p.price - p.cost_price) / p.price) * Decimal::from(100);
            margin < ctx.thresholds.min_margin_pct
        })
        .collect();
    if bad.is_empty() {
        return Ok(Vec::new());
    }
    let n = bad.len();
    Ok(vec![InsightDto {
        id: "below-cost:all".to_string(),
        rule_key: "below-cost".to_string(),
        severity: InsightSeverity::Warning,
        message: format!("{n} صنف بسعر أقل من التكلفة أو بهامش ربح ضعيف"),
        metric: Some(format!("{} صنف", fmt_num(Decimal::from(n as i64), 0, ctx.numerals))),
        action_label: "مراجعة الأسعار".to_string(),
        action_to: RouteRef::list("products"),
        icon: InsightIcon::TrendingDown,
        roles: vec![crate::core::auth::Role::Manager, crate::core::auth::Role::Admin],
        value: Decimal::from((n * 50) as i64),
        created_at: ctx.today.format("%Y-%m-%d").to_string(),
    }])
}

/// 21 — missing supplier invoice (`ir:615-633`).
pub async fn missing_supplier_invoice<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    let missing = PoEntity::find()
        .filter(PoColumn::Status.eq(PurchaseStatus::Received))
        .filter(PoColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .filter(|po| po.supplier_invoice_no.as_deref().unwrap_or("").is_empty())
        .count();
    if missing == 0 {
        return Ok(Vec::new());
    }
    Ok(vec![InsightDto {
        id: "missing-supplier-invoice:all".to_string(),
        rule_key: "missing-supplier-invoice".to_string(),
        severity: InsightSeverity::Info,
        message: format!("{missing} أمر شراء مستلم بلا رقم فاتورة مورد"),
        metric: Some(format!("{} أمر", fmt_num(Decimal::from(missing as i64), 0, ctx.numerals))),
        action_label: "عرض المشتريات".to_string(),
        action_to: RouteRef::list("purchases"),
        icon: InsightIcon::AlertTriangle,
        roles: vec![crate::core::auth::Role::Accountant, crate::core::auth::Role::Manager, crate::core::auth::Role::Admin],
        value: Decimal::from((missing * 20) as i64),
        created_at: ctx.today.format("%Y-%m-%d").to_string(),
    }])
}
