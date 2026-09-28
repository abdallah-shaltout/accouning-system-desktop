//! `dashboard_get_dashboard_summary`, `dashboard_get_home_kpis`, `dashboard_get_top_products`,
//! `dashboard_get_top_customers` (14-analytics.md §3.4, §3.5, §3.9, §3.10).

use indexmap::IndexMap;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::catalog::products::Entity as ProductEntity;
use crate::entities::sales::invoice_lines::{Column as LineColumn, Entity as LineEntity};
use crate::entities::sales::invoices::{Column as InvoiceColumn, Entity as InvoiceEntity, InvoiceStatus};
use crate::entities::sales::refund_lines::{Column as RefundLineColumn, Entity as RefundLineEntity};
use crate::entities::sales::refunds::{Column as RefundColumn, Entity as RefundEntity};
use crate::shared::ledger::accounts::SystemRole;
use crate::shared::totals::document_outstanding;
use crate::utils::money::round2;

use super::super::dto::{DashboardSummary, GrossProfitKpi, HomeKpi, HomeKpis, HomePeriod, HomeTrendPoint, ReceivablesKpi, SalesTrendDay, SparklinePoint, TopCustomerRow, TopProductRow};
use super::common::{account_balance, account_balance_as_of};
use super::feed::low_stock_query;

fn line_key(product_id: Option<crate::utils::id::Id>, position: i16) -> String {
    match product_id {
        Some(id) => id.to_string(),
        None => format!("freetext-{position}"),
    }
}

/// `soldInvoices()` filtered to a day range (`i.date_day` between `from`/`to` inclusive).
async fn sold_in_range<C: ConnectionTrait>(conn: &C, from: chrono::NaiveDate, to: chrono::NaiveDate) -> Result<Vec<crate::entities::sales::invoices::Model>, AppError> {
    InvoiceEntity::find()
        .filter(InvoiceColumn::Status.ne(InvoiceStatus::Draft))
        .filter(InvoiceColumn::DateDay.gte(from))
        .filter(InvoiceColumn::DateDay.lte(to))
        .filter(InvoiceColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)
}

async fn refunds_in_range<C: ConnectionTrait>(conn: &C, from: chrono::NaiveDate, to: chrono::NaiveDate) -> Result<Vec<crate::entities::sales::refunds::Model>, AppError> {
    RefundEntity::find()
        .filter(RefundColumn::DateDay.gte(from))
        .filter(RefundColumn::DateDay.lte(to))
        .filter(RefundColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)
}

/// `netSalesFor` (`dashboardService.ts:110-114`).
async fn net_sales_for<C: ConnectionTrait>(conn: &C, from: chrono::NaiveDate, to: chrono::NaiveDate) -> Result<Decimal, AppError> {
    let gross: Decimal = sold_in_range(conn, from, to).await?.iter().map(|i| i.grand_total).sum();
    let refunds: Decimal = refunds_in_range(conn, from, to).await?.iter().map(|r| r.grand_total).sum();
    Ok(round2(gross - refunds))
}

// --- 3.4 dashboard_get_dashboard_summary -----------------------------------------------------------

pub async fn get_dashboard_summary<C: ConnectionTrait>(conn: &C, today: chrono::NaiveDate) -> TxResult<DashboardSummary> {
    let sold_today = sold_in_range(conn, today, today).await?;
    let refunds_today = refunds_in_range(conn, today, today).await?;

    let unpaid: Vec<_> = InvoiceEntity::find()
        .filter(InvoiceColumn::Status.ne(InvoiceStatus::Draft))
        .filter(InvoiceColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .filter(|i| document_outstanding(i.grand_total - i.refunded_amount, i.paid_amount) > Decimal::ZERO)
        .collect();

    let cash_on_hand = account_balance(conn, SystemRole::Cash).await?;
    let bank_balance = account_balance(conn, SystemRole::Bank).await?;

    let mut sales_trend = Vec::with_capacity(14);
    for d in (0..14).rev() {
        let day = today - chrono::Duration::days(d);
        let gross: Decimal = sold_in_range(conn, day, day).await?.iter().map(|i| i.grand_total).sum();
        let refunds: Decimal = refunds_in_range(conn, day, day).await?.iter().map(|r| r.grand_total).sum();
        sales_trend.push(SalesTrendDay { date: day.format("%Y-%m-%d").to_string(), total: round2(gross - refunds) });
    }

    let low_stock_count = low_stock_query(conn).await?.len() as i32;
    let unpaid_total: Decimal = unpaid.iter().map(|i| document_outstanding(i.grand_total - i.refunded_amount, i.paid_amount)).sum();

    Ok(DashboardSummary {
        today_sales: round2(sold_today.iter().map(|i| i.grand_total).sum::<Decimal>() - refunds_today.iter().map(|r| r.grand_total).sum::<Decimal>()),
        today_invoice_count: sold_today.len() as i32,
        unpaid_invoice_count: unpaid.len() as i32,
        unpaid_invoice_total: unpaid_total,
        low_stock_count,
        cash_position: round2(cash_on_hand + bank_balance),
        cash_on_hand,
        bank_balance,
        sales_trend,
    })
}

// --- 3.5 dashboard_get_home_kpis --------------------------------------------------------------------

pub struct PeriodRange {
    pub from: chrono::NaiveDate,
    pub to: chrono::NaiveDate,
    pub prev_from: chrono::NaiveDate,
    pub prev_to: chrono::NaiveDate,
    pub span_days: i64,
}

/// `periodRange` (`dashboardService.ts:88-104`).
pub fn period_range(period: HomePeriod, today: chrono::NaiveDate) -> PeriodRange {
    let to = today;
    let from = match period {
        HomePeriod::Today => today,
        HomePeriod::Week => today - chrono::Duration::days(6),
        HomePeriod::Month => today - chrono::Duration::days(29),
    };
    let span_days = (to - from).num_days() + 1;
    let prev_to = from - chrono::Duration::days(1);
    let prev_from = prev_to - chrono::Duration::days(span_days - 1);
    PeriodRange { from, to, prev_from, prev_to, span_days }
}

/// `grossProfitFor` (`dashboardService.ts:116-139`).
async fn gross_profit_for<C: ConnectionTrait>(conn: &C, from: chrono::NaiveDate, to: chrono::NaiveDate) -> Result<(Decimal, Decimal), AppError> {
    let invoices = sold_in_range(conn, from, to).await?;
    let invoice_ids: Vec<_> = invoices.iter().map(|i| i.id).collect();
    let lines = if invoice_ids.is_empty() {
        Vec::new()
    } else {
        LineEntity::find().filter(LineColumn::InvoiceId.is_in(invoice_ids)).all(conn).await.map_err(AppError::from)?
    };

    let mut profit = Decimal::ZERO;
    let mut revenue = Decimal::ZERO;
    for line in &lines {
        let net = line.net.unwrap_or_else(|| round2(line.qty * line.price - line.discount));
        revenue += net;
        profit += net - line.qty * line.cost_price;
    }

    let refunds = refunds_in_range(conn, from, to).await?;
    for r in &refunds {
        let Some(original) = InvoiceEntity::find_by_id(r.invoice_id).one(conn).await.map_err(AppError::from)? else { continue };
        let refund_lines = RefundLineEntity::find().filter(RefundLineColumn::RefundId.eq(r.id)).all(conn).await.map_err(AppError::from)?;
        let orig_lines = LineEntity::find().filter(LineColumn::InvoiceId.eq(original.id)).all(conn).await.map_err(AppError::from)?;
        for rl in &refund_lines {
            let Some(orig_line) = orig_lines.iter().find(|l| l.id == rl.invoice_line_id) else { continue };
            let orig_net = orig_line.net.unwrap_or_else(|| round2(orig_line.qty * orig_line.price - orig_line.discount));
            let qty_or_one = if orig_line.qty == Decimal::ZERO { Decimal::ONE } else { orig_line.qty };
            let unit_net = orig_net / qty_or_one;
            let net = round2(unit_net * rl.qty);
            revenue -= net;
            profit -= net - rl.qty * orig_line.cost_price;
        }
    }

    Ok((round2(profit), round2(revenue)))
}

/// `receivablesOverdue` (`dashboardService.ts:141-148`) — quirk Q-4: a due date of "now" (business
/// clock instant, ISO string compare) counts as already overdue.
async fn receivables_overdue<C: ConnectionTrait>(conn: &C, now_iso: &str) -> Result<Decimal, AppError> {
    let rows = InvoiceEntity::find()
        .filter(InvoiceColumn::Status.ne(InvoiceStatus::Draft))
        .filter(InvoiceColumn::Status.ne(InvoiceStatus::Refunded))
        .filter(InvoiceColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let mut total = Decimal::ZERO;
    for i in &rows {
        let Some(due) = i.due_date() else { continue };
        let outstanding = document_outstanding(i.grand_total - i.refunded_amount, i.paid_amount);
        if outstanding <= Decimal::ZERO {
            continue;
        }
        if due.key().as_str() < now_iso {
            total += outstanding;
        }
    }
    Ok(round2(total))
}

fn change_pct(current: Decimal, previous: Decimal) -> Option<Decimal> {
    if previous == Decimal::ZERO {
        if current == Decimal::ZERO {
            Some(Decimal::ZERO)
        } else {
            None
        }
    } else {
        Some(round2(((current - previous) / previous.abs()) * Decimal::from(100)))
    }
}

async fn daily_series<C: ConnectionTrait>(conn: &C, days: i64, to: chrono::NaiveDate) -> Result<Vec<Decimal>, AppError> {
    let mut out = Vec::with_capacity(days as usize);
    for d in (0..days).rev() {
        let day = to - chrono::Duration::days(d);
        out.push(net_sales_for(conn, day, day).await?);
    }
    Ok(out)
}

pub async fn get_home_kpis<C: ConnectionTrait>(conn: &C, today: chrono::NaiveDate, now_iso: &str, period: HomePeriod) -> TxResult<HomeKpis> {
    let range: PeriodRange = period_range(period, today);

    let net_sales = net_sales_for(conn, range.from, range.to).await?;
    let prev_net_sales = net_sales_for(conn, range.prev_from, range.prev_to).await?;
    let (gp_profit, gp_revenue) = gross_profit_for(conn, range.from, range.to).await?;
    let (prev_gp_profit, _prev_gp_revenue) = gross_profit_for(conn, range.prev_from, range.prev_to).await?;

    let cash_on_hand = account_balance(conn, SystemRole::Cash).await?;
    let bank_balance = account_balance(conn, SystemRole::Bank).await?;
    let cash = round2(cash_on_hand + bank_balance);
    let cash_prev = round2(account_balance_as_of(conn, SystemRole::Cash, range.prev_to).await? + account_balance_as_of(conn, SystemRole::Bank, range.prev_to).await?);

    let receivables_total: Decimal = InvoiceEntity::find()
        .filter(InvoiceColumn::Status.ne(InvoiceStatus::Draft))
        .filter(InvoiceColumn::Status.ne(InvoiceStatus::Refunded))
        .filter(InvoiceColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?
        .iter()
        .map(|i| document_outstanding(i.grand_total - i.refunded_amount, i.paid_amount))
        .sum();
    let receivables_total = round2(receivables_total);

    let receivables_prev: Decimal = InvoiceEntity::find()
        .filter(InvoiceColumn::Status.ne(InvoiceStatus::Draft))
        .filter(InvoiceColumn::Status.ne(InvoiceStatus::Refunded))
        .filter(InvoiceColumn::DateDay.lte(range.prev_to))
        .filter(InvoiceColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?
        .iter()
        .map(|i| document_outstanding(i.grand_total - i.refunded_amount, i.paid_amount))
        .sum();
    let receivables_prev = round2(receivables_prev);

    let spark = daily_series(conn, 14, today).await?;
    let spark_dto: Vec<SparklinePoint> = spark.into_iter().map(SparklinePoint::from).collect();

    let span_days = (range.to - range.from).num_days() + 1;
    let mut sales_trend = Vec::with_capacity(span_days as usize);
    for d in (0..span_days).rev() {
        let day = range.to - chrono::Duration::days(d);
        let prev_day = range.prev_to - chrono::Duration::days(d);
        sales_trend.push(HomeTrendPoint {
            date: day.format("%Y-%m-%d").to_string(),
            total: net_sales_for(conn, day, day).await?,
            previous_total: net_sales_for(conn, prev_day, prev_day).await?,
        });
    }

    let overdue = receivables_overdue(conn, now_iso).await?;
    let margin_pct = if gp_revenue > Decimal::ZERO { round2((gp_profit / gp_revenue) * Decimal::from(100)) } else { Decimal::ZERO };

    Ok(HomeKpis {
        net_sales: HomeKpi { value: net_sales, previous: prev_net_sales, change_pct: change_pct(net_sales, prev_net_sales), sparkline: spark_dto.clone() },
        gross_profit: GrossProfitKpi {
            value: gp_profit,
            previous: prev_gp_profit,
            change_pct: change_pct(gp_profit, prev_gp_profit),
            sparkline: spark_dto.clone(),
            margin_pct,
        },
        cash: HomeKpi { value: cash, previous: cash_prev, change_pct: change_pct(cash, cash_prev), sparkline: spark_dto.clone() },
        receivables: ReceivablesKpi {
            value: receivables_total,
            previous: receivables_prev,
            change_pct: change_pct(receivables_total, receivables_prev),
            sparkline: spark_dto,
            overdue,
        },
        sales_trend,
    })
}

// --- 3.9 dashboard_get_top_products -------------------------------------------------------------------

pub async fn get_top_products<C: ConnectionTrait>(conn: &C, today: chrono::NaiveDate, period: HomePeriod, limit: usize) -> TxResult<Vec<TopProductRow>> {
    let range: PeriodRange = period_range(period, today);
    let invoices = sold_in_range(conn, range.from, range.to).await?;
    let invoice_ids: Vec<_> = invoices.iter().map(|i| i.id).collect();
    let lines = if invoice_ids.is_empty() {
        Vec::new()
    } else {
        LineEntity::find().filter(LineColumn::InvoiceId.is_in(invoice_ids)).all(conn).await.map_err(AppError::from)?
    };

    struct Agg {
        qty: Decimal,
        profit: Decimal,
    }
    let mut by_product: IndexMap<String, Agg> = IndexMap::new();
    for line in &lines {
        let net = line.net.unwrap_or_else(|| round2(line.qty * line.price - line.discount));
        let key = line_key(line.product_id, line.position);
        let entry = by_product.entry(key).or_insert(Agg { qty: Decimal::ZERO, profit: Decimal::ZERO });
        entry.qty += line.qty;
        entry.profit += net - line.qty * line.cost_price;
    }

    let ids: Vec<_> = by_product.keys().filter_map(|k| k.parse::<crate::utils::id::Id>().ok()).collect();
    let products = if ids.is_empty() {
        Vec::new()
    } else {
        ProductEntity::find().filter(crate::entities::catalog::products::Column::Id.is_in(ids)).all(conn).await.map_err(AppError::from)?
    };

    let mut rows: Vec<TopProductRow> = by_product
        .into_iter()
        .map(|(key, v)| {
            let product = key.parse::<crate::utils::id::Id>().ok().and_then(|id| products.iter().find(|p| p.id == id));
            TopProductRow {
                id: key,
                name: product.map(|p| p.name.clone()).unwrap_or_else(|| "—".to_string()),
                sku: product.map(|p| p.sku.clone()).unwrap_or_default(),
                qty: round2(v.qty),
                gross_profit: round2(v.profit),
            }
        })
        .collect();
    rows.sort_by(|a, b| b.gross_profit.cmp(&a.gross_profit));
    rows.truncate(limit);
    Ok(rows)
}

// --- 3.10 dashboard_get_top_customers -----------------------------------------------------------------

pub async fn get_top_customers<C: ConnectionTrait>(conn: &C, today: chrono::NaiveDate, period: HomePeriod, limit: usize) -> TxResult<Vec<TopCustomerRow>> {
    let range: PeriodRange = period_range(period, today);
    let invoices = sold_in_range(conn, range.from, range.to).await?;

    let mut by_customer: IndexMap<crate::utils::id::Id, Decimal> = IndexMap::new();
    for inv in &invoices {
        if let Some(cid) = inv.customer_id {
            let entry = by_customer.entry(cid).or_insert(Decimal::ZERO);
            *entry = round2(*entry + inv.grand_total);
        }
    }

    let ids: Vec<_> = by_customer.keys().copied().collect();
    let customers = if ids.is_empty() {
        Vec::new()
    } else {
        crate::entities::parties::parties::Entity::find()
            .filter(crate::entities::parties::parties::Column::Id.is_in(ids))
            .all(conn)
            .await
            .map_err(AppError::from)?
    };

    let mut rows: Vec<TopCustomerRow> = by_customer
        .into_iter()
        .map(|(id, total)| {
            let name = customers.iter().find(|c| c.id == id).map(|c| c.name.clone()).unwrap_or_else(|| "—".to_string());
            TopCustomerRow { id, name, total }
        })
        .collect();
    rows.sort_by(|a, b| b.total.cmp(&a.total));
    rows.truncate(limit);
    Ok(rows)
}
