//! `reports::service::sales` (13b §3.1, 3.3, 3.4, 3.5, 3.13, 3.14): sales report, discounts, gross
//! profit, returns, branch comparison, profit leakage.

use indexmap::IndexMap;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::catalog::{categories, products};
use crate::entities::inventory::stock_movements;
use crate::entities::org::{branches, users};
use crate::entities::sales::{invoice_lines, invoices, refund_lines, refunds};
use crate::entities::soft_delete::SoftDelete;
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::super::dto::{
    BranchComparisonRow, DateRangeInput, DiscountGroupBy, DiscountReportRow, GrossProfitGroupBy, GrossProfitRow, ProfitLeakageReport, ReturnsReport, ReturnsReportRow, SalesByCashier, SalesByCategory,
    SalesByDay, SalesByMethod, SalesByProduct, SalesReport, SalesReportSummary,
};
use super::common::{self, DateRange};

const SALE_METHOD_LABEL_CASH: &str = "نقداً";
const SALE_METHOD_LABEL_CARD: &str = "بطاقة (مدى/فيزا)";
const SALE_METHOD_LABEL_BANK_TRANSFER: &str = "تحويل بنكي";
const SALE_METHOD_LABEL_CREDIT: &str = "آجل";

/// A live product's essentials, batch-loaded once per command.
struct ProductLookup {
    by_id: std::collections::HashMap<Id, products::Model>,
    categories_by_id: std::collections::HashMap<Id, categories::Model>,
}

impl ProductLookup {
    async fn load<C: ConnectionTrait>(conn: &C) -> TxResult<Self> {
        let products = products::Entity::find_live().all(conn).await.map_err(AppError::from)?;
        let categories = categories::Entity::find_live().all(conn).await.map_err(AppError::from)?;
        Ok(ProductLookup {
            by_id: products.into_iter().map(|p| (p.id, p)).collect(),
            categories_by_id: categories.into_iter().map(|c| (c.id, c)).collect(),
        })
    }

    fn is_product(&self, product_id: Option<Id>) -> bool {
        product_id.and_then(|id| self.by_id.get(&id)).map(|p| p.r#type == "product").unwrap_or(false)
    }

    fn category_name(&self, product_id: Option<Id>) -> String {
        product_id
            .and_then(|id| self.by_id.get(&id))
            .and_then(|p| p.category_id)
            .and_then(|cid| self.categories_by_id.get(&cid))
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "بدون تصنيف".to_string())
    }
}

/// One invoice line, product-key resolved per §3.0 (`freetext-<position>` for free-text lines).
struct LoadedInvoice {
    id: Id,
    date_day: chrono::NaiveDate,
    number: String,
    grand_total: Decimal,
    sub_total: Decimal,
    discount_amount: Decimal,
    discount_rate: Decimal,
    tax_amount: Decimal,
    tax_rate: Decimal,
    cashier_id: Id,
    payment_method: invoices::SalePaymentMethod,
    branch_id: Option<Id>,
    lines: Vec<LoadedInvoiceLine>,
}

struct LoadedInvoiceLine {
    key: String,
    product_id: Option<Id>,
    name: String,
    qty: Decimal,
    price: Decimal,
    cost_price: Decimal,
    discount: Decimal,
    tax_rate: Option<Decimal>,
}

fn product_key(position: i16, product_id: Option<Id>) -> String {
    match product_id {
        Some(id) => id.to_string(),
        None => format!("freetext-{position}"),
    }
}

async fn load_invoices<C: ConnectionTrait>(conn: &C, range: &DateRange, include_draft: bool) -> TxResult<Vec<LoadedInvoice>> {
    let rows = invoices::Entity::find().order_by_asc(invoices::Column::CreatedAt).order_by_asc(invoices::Column::Id).all(conn).await.map_err(AppError::from)?;
    let mut out = Vec::new();
    for inv in rows {
        if !include_draft && inv.status == invoices::InvoiceStatus::Draft {
            continue;
        }
        if !common::in_range(inv.date_day, range) {
            continue;
        }
        let lines = invoice_lines::Entity::find()
            .filter(invoice_lines::Column::InvoiceId.eq(inv.id))
            .order_by_asc(invoice_lines::Column::Position)
            .all(conn)
            .await
            .map_err(AppError::from)?;
        let loaded_lines = lines
            .into_iter()
            .map(|l| LoadedInvoiceLine {
                key: product_key(l.position, l.product_id),
                product_id: l.product_id,
                name: l.name,
                qty: l.qty,
                price: l.price,
                cost_price: l.cost_price,
                discount: l.discount,
                tax_rate: l.tax_rate,
            })
            .collect();
        out.push(LoadedInvoice {
            id: inv.id,
            date_day: inv.date_day,
            number: inv.number.clone(),
            grand_total: inv.grand_total,
            sub_total: inv.sub_total,
            discount_amount: inv.discount_amount,
            discount_rate: inv.discount_rate,
            tax_amount: inv.tax_amount,
            tax_rate: inv.tax_rate,
            cashier_id: inv.cashier_id,
            payment_method: inv.payment_method.clone(),
            branch_id: inv.branch_id,
            lines: loaded_lines,
        });
    }
    Ok(out)
}

async fn users_by_id<C: ConnectionTrait>(conn: &C) -> TxResult<std::collections::HashMap<Id, String>> {
    Ok(users::Entity::find().all(conn).await.map_err(AppError::from)?.into_iter().map(|u| (u.id, u.name)).collect())
}

/// **`reports_get_sales_report`** (13b §3.1).
pub async fn sales_report<C: ConnectionTrait>(conn: &C, range: &DateRangeInput) -> TxResult<SalesReport> {
    let date_range = DateRange::parse(range)?;
    let invoices = load_invoices(conn, &date_range, false).await?;
    let refund_rows = refunds::Entity::find().all(conn).await.map_err(AppError::from)?;
    let refunds: Vec<_> = refund_rows.into_iter().filter(|r| common::in_range(r.date_day, &date_range)).collect();

    let products = ProductLookup::load(conn).await?;

    // byProduct: raw accumulation, one round at the end (per-product), per §3.1 step 2-3.
    let mut by_product: IndexMap<String, (String, Decimal, Decimal, Decimal)> = IndexMap::new(); // key -> (name, qty, revenue_raw, cost_raw)
    for inv in &invoices {
        let factor = Decimal::ONE - inv.discount_rate / Decimal::from(100);
        let inclusive = inv.tax_amount > Decimal::ZERO && (inv.sub_total - inv.discount_amount - inv.grand_total).abs() < Decimal::new(1, 2);
        for line in &inv.lines {
            let entry = by_product.entry(line.key.clone()).or_insert((line.name.clone(), Decimal::ZERO, Decimal::ZERO, Decimal::ZERO));
            entry.1 += line.qty;
            let divisor = if inclusive { Decimal::ONE + line.tax_rate.unwrap_or(inv.tax_rate) / Decimal::from(100) } else { Decimal::ONE };
            entry.2 += ((line.qty * line.price - line.discount) * factor) / divisor;
            entry.3 += if products.is_product(line.product_id) { line.qty * line.cost_price } else { Decimal::ZERO };
        }
    }
    let mut product_rows: Vec<SalesByProduct> = by_product
        .into_iter()
        .map(|(key, (name, qty, revenue_raw, cost_raw))| {
            let revenue = round2(revenue_raw);
            let cost = round2(cost_raw);
            SalesByProduct { product_id: key, name, qty, revenue, cost, profit: round2(revenue_raw - cost_raw) }
        })
        .collect();
    product_rows.sort_by(|a, b| b.revenue.cmp(&a.revenue));

    // byCategory: iterate productRows in that order, key = category name.
    let mut by_category: IndexMap<String, (Decimal, Decimal)> = IndexMap::new(); // name -> (qty, revenue)
    for r in &product_rows {
        let product_id = r.product_id.parse::<Id>().ok();
        let name = products.category_name(product_id);
        let entry = by_category.entry(name).or_insert((Decimal::ZERO, Decimal::ZERO));
        entry.0 += r.qty;
        entry.1 = round2(entry.1 + r.revenue);
    }
    let mut category_rows: Vec<SalesByCategory> = by_category.into_iter().map(|(name, (qty, revenue))| SalesByCategory { name, qty, revenue }).collect();
    category_rows.sort_by(|a, b| b.revenue.cmp(&a.revenue));

    // byDay
    let mut by_day: IndexMap<String, (i32, Decimal)> = IndexMap::new();
    for inv in &invoices {
        let key = inv.date_day.format("%Y-%m-%d").to_string();
        let entry = by_day.entry(key).or_insert((0, Decimal::ZERO));
        entry.0 += 1;
        entry.1 = round2(entry.1 + inv.grand_total);
    }
    let mut day_rows: Vec<SalesByDay> = by_day.into_iter().map(|(date, (invoice_count, total))| SalesByDay { date, invoices: invoice_count, total }).collect();
    day_rows.sort_by(|a, b| a.date.cmp(&b.date));

    // byMethod
    let mut by_method: IndexMap<String, (i32, Decimal)> = IndexMap::new();
    for inv in &invoices {
        let key = match inv.payment_method {
            invoices::SalePaymentMethod::Cash => "cash",
            invoices::SalePaymentMethod::Card => "card",
            invoices::SalePaymentMethod::BankTransfer => "bank_transfer",
            invoices::SalePaymentMethod::Credit => "credit",
        };
        let entry = by_method.entry(key.to_string()).or_insert((0, Decimal::ZERO));
        entry.0 += 1;
        entry.1 = round2(entry.1 + inv.grand_total);
    }
    let mut method_rows: Vec<SalesByMethod> = by_method
        .into_iter()
        .map(|(key, (count, total))| {
            let label = match key.as_str() {
                "cash" => SALE_METHOD_LABEL_CASH,
                "card" => SALE_METHOD_LABEL_CARD,
                "bank_transfer" => SALE_METHOD_LABEL_BANK_TRANSFER,
                _ => SALE_METHOD_LABEL_CREDIT,
            };
            SalesByMethod { method: label.to_string(), count, total }
        })
        .collect();
    method_rows.sort_by(|a, b| b.total.cmp(&a.total));

    // byCashier
    let users_map = users_by_id(conn).await?;
    let mut by_cashier: IndexMap<Id, (i32, Decimal)> = IndexMap::new();
    for inv in &invoices {
        let entry = by_cashier.entry(inv.cashier_id).or_insert((0, Decimal::ZERO));
        entry.0 += 1;
        entry.1 = round2(entry.1 + inv.grand_total);
    }
    let mut cashier_rows: Vec<SalesByCashier> =
        by_cashier.into_iter().map(|(id, (count, total))| SalesByCashier { name: users_map.get(&id).cloned().unwrap_or_else(|| "—".to_string()), count, total }).collect();
    cashier_rows.sort_by(|a, b| b.total.cmp(&a.total));

    let gross_sales = common::sum2(invoices.iter().map(|i| i.sub_total));
    let discounts = common::sum2(invoices.iter().map(|i| i.discount_amount));
    let vat = common::sum2(invoices.iter().map(|i| i.tax_amount));
    let total = common::sum2(invoices.iter().map(|i| i.grand_total));
    let net_sales = round2(total - vat);
    let refund_total = common::sum2(refunds.iter().map(|r| r.grand_total));
    let cogs = common::sum2(product_rows.iter().map(|r| r.cost));

    let summary = SalesReportSummary {
        invoice_count: invoices.len() as i32,
        gross_sales,
        discounts,
        net_sales,
        vat,
        total,
        refunds: refund_total,
        net_after_refunds: round2(total - refund_total),
        cogs,
        gross_profit: round2(net_sales - cogs),
        average_invoice: if !invoices.is_empty() { round2(total / Decimal::from(invoices.len() as i64)) } else { Decimal::ZERO },
    };

    Ok(SalesReport { summary, by_day: day_rows, by_product: product_rows, by_category: category_rows, by_method: method_rows, by_cashier: cashier_rows })
}

/// **`reports_get_discounts_report`** (13b §3.3).
pub async fn discounts_report<C: ConnectionTrait>(conn: &C, range: &DateRangeInput, group_by: DiscountGroupBy) -> TxResult<Vec<DiscountReportRow>> {
    let date_range = DateRange::parse(range)?;
    let invoices = load_invoices(conn, &date_range, false).await?;
    let users_map = users_by_id(conn).await?;

    let mut rows: Vec<DiscountReportRow> = match group_by {
        DiscountGroupBy::Cashier => {
            let mut map: IndexMap<Id, (String, i32, Decimal, Decimal)> = IndexMap::new();
            for inv in &invoices {
                let label = users_map.get(&inv.cashier_id).cloned().unwrap_or_else(|| inv.cashier_id.to_string());
                let entry = map.entry(inv.cashier_id).or_insert((label, 0, Decimal::ZERO, Decimal::ZERO));
                entry.1 += 1;
                let list_value = inv.lines.iter().fold(Decimal::ZERO, |a, l| a + l.qty * l.price);
                let charged = round2(list_value - inv.discount_amount - inv.lines.iter().fold(Decimal::ZERO, |a, l| a + l.discount));
                entry.2 = round2(entry.2 + list_value);
                entry.3 = round2(entry.3 + charged);
            }
            map.into_iter()
                .map(|(key, (label, invoice_count, list_value, charged_value))| DiscountReportRow {
                    key: key.to_string(),
                    label,
                    invoice_count,
                    list_value,
                    charged_value,
                    discount_value: round2(list_value - charged_value),
                    discount_pct: if list_value > Decimal::ZERO { round2((list_value - charged_value) / list_value * Decimal::from(100)) } else { Decimal::ZERO },
                })
                .collect()
        }
        DiscountGroupBy::Product => {
            let mut map: IndexMap<String, (String, i32, Decimal, Decimal)> = IndexMap::new();
            for inv in &invoices {
                for l in &inv.lines {
                    let entry = map.entry(l.key.clone()).or_insert((l.name.clone(), 0, Decimal::ZERO, Decimal::ZERO));
                    entry.1 += 1;
                    let list_value = round2(l.qty * l.price);
                    let charged = round2(list_value - l.discount);
                    entry.2 = round2(entry.2 + list_value);
                    entry.3 = round2(entry.3 + charged);
                }
            }
            map.into_iter()
                .map(|(key, (label, invoice_count, list_value, charged_value))| DiscountReportRow {
                    key,
                    label,
                    invoice_count,
                    list_value,
                    charged_value,
                    discount_value: round2(list_value - charged_value),
                    discount_pct: if list_value > Decimal::ZERO { round2((list_value - charged_value) / list_value * Decimal::from(100)) } else { Decimal::ZERO },
                })
                .collect()
        }
    };
    rows.retain(|r| r.discount_value > Decimal::new(1, 3));
    rows.sort_by(|a, b| b.discount_value.cmp(&a.discount_value));
    Ok(rows)
}

/// **`reports_get_gross_profit_report`** (13b §3.4).
pub async fn gross_profit_report<C: ConnectionTrait>(conn: &C, range: &DateRangeInput, group_by: GrossProfitGroupBy) -> TxResult<Vec<GrossProfitRow>> {
    let date_range = DateRange::parse(range)?;
    let invoices = load_invoices(conn, &date_range, false).await?;
    let products = ProductLookup::load(conn).await?;

    let mut rows: Vec<GrossProfitRow> = match group_by {
        GrossProfitGroupBy::Invoice => invoices
            .iter()
            .map(|inv| {
                let revenue = round2(inv.sub_total - inv.discount_amount);
                let cost = round2(inv.lines.iter().fold(Decimal::ZERO, |a, l| a + if products.is_product(l.product_id) { l.qty * l.cost_price } else { Decimal::ZERO }));
                let qty = round2(inv.lines.iter().fold(Decimal::ZERO, |a, l| a + l.qty));
                GrossProfitRow { key: inv.id.to_string(), label: inv.number.clone(), qty, revenue, cost, profit: round2(revenue - cost), margin_pct: margin_pct(revenue, revenue - cost) }
            })
            .collect(),
        GrossProfitGroupBy::Product | GrossProfitGroupBy::Category => {
            let by_category = matches!(group_by, GrossProfitGroupBy::Category);
            let mut map: IndexMap<String, (String, Decimal, Decimal, Decimal)> = IndexMap::new();
            for inv in &invoices {
                let factor = Decimal::ONE - inv.discount_rate / Decimal::from(100);
                for l in &inv.lines {
                    let key = if by_category { products.category_name(l.product_id) } else { l.key.clone() };
                    let label = if by_category { key.clone() } else { l.name.clone() };
                    let entry = map.entry(key).or_insert((label, Decimal::ZERO, Decimal::ZERO, Decimal::ZERO));
                    entry.1 += l.qty;
                    entry.2 = round2(entry.2 + (l.qty * l.price - l.discount) * factor);
                    entry.3 = round2(entry.3 + if products.is_product(l.product_id) { l.qty * l.cost_price } else { Decimal::ZERO });
                }
            }
            map.into_iter()
                .map(|(key, (label, qty, revenue, cost))| GrossProfitRow { key, label, qty: round2(qty), revenue, cost, profit: round2(revenue - cost), margin_pct: margin_pct(revenue, revenue - cost) })
                .collect()
        }
    };
    rows.sort_by(|a, b| b.profit.cmp(&a.profit));
    Ok(rows)
}

fn margin_pct(revenue: Decimal, profit: Decimal) -> Decimal {
    if revenue > Decimal::ZERO {
        round2(profit / revenue * Decimal::from(100))
    } else {
        Decimal::ZERO
    }
}

/// **`reports_get_returns_report`** (13b §3.5).
pub async fn returns_report<C: ConnectionTrait>(conn: &C, range: &DateRangeInput) -> TxResult<ReturnsReport> {
    let date_range = DateRange::parse(range)?;
    let invoices = load_invoices(conn, &date_range, false).await?;
    let total_invoices = invoices.len() as i32;

    let refund_rows = refunds::Entity::find().order_by_asc(refunds::Column::CreatedAt).order_by_asc(refunds::Column::Id).all(conn).await.map_err(AppError::from)?;
    let refunds_in_range: Vec<_> = refund_rows.into_iter().filter(|r| common::in_range(r.date_day, &date_range)).collect();

    // Load each refund's lines.
    let mut refund_lines_by_refund: std::collections::HashMap<Id, Vec<refund_lines::Model>> = std::collections::HashMap::new();
    for r in &refunds_in_range {
        let lines = refund_lines::Entity::find().filter(refund_lines::Column::RefundId.eq(r.id)).order_by_asc(refund_lines::Column::Position).all(conn).await.map_err(AppError::from)?;
        refund_lines_by_refund.insert(r.id, lines);
    }

    // byReason
    let mut by_reason: IndexMap<String, (i32, Decimal, Decimal)> = IndexMap::new();
    for r in &refunds_in_range {
        let reason = r.reason.clone().filter(|s| !s.is_empty()).unwrap_or_else(|| "غير محدد".to_string());
        let lines = refund_lines_by_refund.get(&r.id).cloned().unwrap_or_default();
        let qty = lines.iter().fold(Decimal::ZERO, |a, l| a + l.qty);
        let entry = by_reason.entry(reason).or_insert((0, Decimal::ZERO, Decimal::ZERO));
        entry.0 += 1;
        entry.1 += qty; // raw accumulate, matches sum() then += — mock uses `row.qty += sum(...)`.
        entry.2 = round2(entry.2 + r.grand_total);
    }
    let mut by_reason_rows: Vec<ReturnsReportRow> =
        by_reason.into_iter().map(|(key, (count, qty, amount))| ReturnsReportRow { key: key.clone(), label: key, count, qty, amount }).collect();
    by_reason_rows.sort_by(|a, b| b.amount.cmp(&a.amount));

    // byProduct: match refund -> its invoice (any date) -> invoice line by invoiceLineId.
    let invoice_ids: Vec<Id> = refunds_in_range.iter().map(|r| r.invoice_id).collect();
    let invoices_by_id: std::collections::HashMap<Id, invoices::Model> = if invoice_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        invoices::Entity::find().filter(invoices::Column::Id.is_in(invoice_ids.iter().copied())).all(conn).await.map_err(AppError::from)?.into_iter().map(|i| (i.id, i)).collect()
    };
    let mut invoice_lines_by_invoice: std::collections::HashMap<Id, Vec<invoice_lines::Model>> = std::collections::HashMap::new();
    for id in invoices_by_id.keys() {
        let lines = invoice_lines::Entity::find().filter(invoice_lines::Column::InvoiceId.eq(*id)).all(conn).await.map_err(AppError::from)?;
        invoice_lines_by_invoice.insert(*id, lines);
    }

    let products = ProductLookup::load(conn).await?;
    let mut by_product: IndexMap<Id, (i32, Decimal, Decimal)> = IndexMap::new();
    for r in &refunds_in_range {
        if !invoices_by_id.contains_key(&r.invoice_id) {
            continue;
        }
        let inv_lines = invoice_lines_by_invoice.get(&r.invoice_id).cloned().unwrap_or_default();
        let lines = refund_lines_by_refund.get(&r.id).cloned().unwrap_or_default();
        for rl in &lines {
            let Some(inv_line) = inv_lines.iter().find(|l| l.id == rl.invoice_line_id) else { continue };
            let Some(product_id) = inv_line.product_id else { continue };
            let entry = by_product.entry(product_id).or_insert((0, Decimal::ZERO, Decimal::ZERO));
            entry.0 += 1;
            entry.1 += rl.qty;
            entry.2 = round2(entry.2 + rl.qty * inv_line.price);
        }
    }
    let mut by_product_rows: Vec<ReturnsReportRow> = by_product
        .into_iter()
        .map(|(id, (count, qty, amount))| ReturnsReportRow { key: id.to_string(), label: products.by_id.get(&id).map(|p| p.name.clone()).unwrap_or_else(|| id.to_string()), count, qty, amount })
        .collect();
    by_product_rows.sort_by(|a, b| b.amount.cmp(&a.amount));

    // byCashier: refund's invoice's cashierId, or "—" when the invoice is missing.
    let users_map = users_by_id(conn).await?;
    let mut by_cashier: IndexMap<String, (i32, Decimal, Decimal)> = IndexMap::new();
    for r in &refunds_in_range {
        let cashier_key = invoices_by_id.get(&r.invoice_id).map(|i| i.cashier_id.to_string()).unwrap_or_else(|| "—".to_string());
        let lines = refund_lines_by_refund.get(&r.id).cloned().unwrap_or_default();
        let qty = lines.iter().fold(Decimal::ZERO, |a, l| a + l.qty);
        let entry = by_cashier.entry(cashier_key).or_insert((0, Decimal::ZERO, Decimal::ZERO));
        entry.0 += 1;
        entry.1 += qty;
        entry.2 = round2(entry.2 + r.grand_total);
    }
    let mut by_cashier_rows: Vec<ReturnsReportRow> = by_cashier
        .into_iter()
        .map(|(key, (count, qty, amount))| {
            let label = key.parse::<Id>().ok().and_then(|id| users_map.get(&id).cloned()).unwrap_or_else(|| key.clone());
            ReturnsReportRow { key, label, count, qty, amount }
        })
        .collect();
    by_cashier_rows.sort_by(|a, b| b.amount.cmp(&a.amount));

    let total_refunds = refunds_in_range.len() as i32;
    let return_rate_pct = if total_invoices > 0 { round2(Decimal::from(total_refunds) / Decimal::from(total_invoices) * Decimal::from(100)) } else { Decimal::ZERO };

    Ok(ReturnsReport { total_invoices, total_refunds, return_rate_pct, by_reason: by_reason_rows, by_product: by_product_rows, by_cashier: by_cashier_rows })
}

/// **`reports_get_branch_comparison`** (13b §3.13).
pub async fn branch_comparison<C: ConnectionTrait>(conn: &C, range: &DateRangeInput) -> TxResult<Vec<BranchComparisonRow>> {
    let date_range = DateRange::parse(range)?;
    let invoices = load_invoices(conn, &date_range, false).await?;
    let products = ProductLookup::load(conn).await?;

    let active_branches = branches::Entity::find()
        .filter(branches::Column::DeletedAt.is_null())
        .filter(branches::Column::Active.eq(true))
        .order_by_asc(branches::Column::CreatedAt)
        .order_by_asc(branches::Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;

    let mut rows: Vec<BranchComparisonRow> = active_branches
        .into_iter()
        .map(|b| {
            let branch_invoices: Vec<&LoadedInvoice> = invoices.iter().filter(|i| i.branch_id == Some(b.id)).collect();
            let sales = common::sum2(branch_invoices.iter().map(|i| i.grand_total));
            let cogs = common::sum2(branch_invoices.iter().map(|i| {
                common::sum2(i.lines.iter().map(|l| if products.is_product(l.product_id) { l.qty * l.cost_price } else { Decimal::ZERO }))
            }));
            let gross_profit = round2(common::sum2(branch_invoices.iter().map(|i| i.sub_total - i.discount_amount)) - cogs);
            let invoice_count = branch_invoices.len() as i32;
            BranchComparisonRow {
                branch_id: b.id,
                name: b.name,
                sales: round2(sales),
                gross_profit,
                invoice_count,
                average_invoice: if invoice_count > 0 { round2(sales / Decimal::from(invoice_count as i64)) } else { Decimal::ZERO },
            }
        })
        .collect();
    rows.sort_by(|a, b| b.sales.cmp(&a.sales));
    Ok(rows)
}

/// **`reports_get_profit_leakage_report`** (13b §3.14).
pub async fn profit_leakage_report<C: ConnectionTrait>(conn: &C, range: &DateRangeInput) -> TxResult<ProfitLeakageReport> {
    let date_range = DateRange::parse(range)?;
    let invoices = load_invoices(conn, &date_range, false).await?;
    let refund_rows = refunds::Entity::find().all(conn).await.map_err(AppError::from)?;
    let refunds_in_range: Vec<_> = refund_rows.into_iter().filter(|r| common::in_range(r.date_day, &date_range)).collect();

    let net_sales = round2(common::sum2(invoices.iter().map(|i| i.sub_total - i.discount_amount)));
    let discounts = round2(
        common::sum2(invoices.iter().map(|i| i.discount_amount)) + common::sum2(invoices.iter().map(|i| common::sum2(i.lines.iter().map(|l| l.discount)))),
    );
    let returns = common::sum2(refunds_in_range.iter().map(|r| r.sub_total));

    let movements_rows = stock_movements::Entity::find().all(conn).await.map_err(AppError::from)?;
    let write_offs = common::sum2(
        movements_rows.iter().filter(|m| m.reason == "loss" && common::in_range(m.date_day, &date_range)).map(|m| m.value_change.abs()),
    );
    let shrinkage = common::sum2(
        movements_rows.iter().filter(|m| m.reason == "stocktake" && m.qty_change < Decimal::ZERO && common::in_range(m.date_day, &date_range)).map(|m| m.value_change.abs()),
    );

    let total_leakage = round2(discounts + returns + write_offs + shrinkage);
    let leakage_pct = if net_sales > Decimal::ZERO { round2(total_leakage / net_sales * Decimal::from(100)) } else { Decimal::ZERO };

    Ok(ProfitLeakageReport { net_sales, discounts, returns, write_offs, shrinkage, total_leakage, leakage_pct })
}
