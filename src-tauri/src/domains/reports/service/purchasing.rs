//! `reports::service::purchasing` (13b §3.6, 3.7, 3.12): expenses, shifts, purchases.

use indexmap::IndexMap;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::expenses::{expense_categories, expenses};
use crate::entities::org::users;
use crate::entities::purchases::{purchase_order_lines, purchase_orders, purchase_returns};
use crate::entities::sales::{invoices, shifts};
use crate::entities::soft_delete::SoftDelete;
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::super::dto::{DateRangeInput, ExpensesByCategory, ExpensesByMonth, ExpensesReport, PurchasesBySupplier, PurchasesByProduct, PurchasesReport, PurchasesReportSummary, ShiftReportRow};
use super::common::{self, DateRange};

/// **`reports_get_expenses_report`** (13b §3.6).
pub async fn expenses_report<C: ConnectionTrait>(conn: &C, range: &DateRangeInput) -> TxResult<ExpensesReport> {
    let date_range = DateRange::parse(range)?;
    let rows = expenses::Entity::find().filter(expenses::Column::DeletedAt.is_null()).all(conn).await.map_err(AppError::from)?;
    let rows: Vec<_> = rows.into_iter().filter(|e| common::in_range(e.date_day, &date_range)).collect();

    let category_ids: std::collections::HashSet<Id> = rows.iter().map(|e| e.category_id).collect();
    let categories: std::collections::HashMap<Id, String> = if category_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        expense_categories::Entity::find_live()
            .filter(expense_categories::Column::Id.is_in(category_ids.iter().copied()))
            .all(conn)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(|c| (c.id, c.name))
            .collect()
    };

    let mut by_category_map: IndexMap<Id, Decimal> = IndexMap::new();
    for e in &rows {
        let entry = by_category_map.entry(e.category_id).or_insert(Decimal::ZERO);
        *entry = round2(*entry + e.amount);
    }
    let mut by_category: Vec<ExpensesByCategory> = by_category_map
        .into_iter()
        .map(|(id, amount)| ExpensesByCategory { category_id: id, name: categories.get(&id).cloned().unwrap_or_else(|| id.to_string()), amount })
        .collect();
    by_category.sort_by(|a, b| b.amount.cmp(&a.amount));

    let mut by_month_map: IndexMap<String, Decimal> = IndexMap::new();
    for e in &rows {
        // The business day's month (`localDateKey(e.date).slice(0,7)`), never a UTC truncation.
        let month = e.date_day.format("%Y-%m").to_string();
        let entry = by_month_map.entry(month).or_insert(Decimal::ZERO);
        *entry = round2(*entry + e.amount);
    }
    let mut by_month: Vec<ExpensesByMonth> = by_month_map.into_iter().map(|(month, amount)| ExpensesByMonth { month, amount }).collect();
    by_month.sort_by(|a, b| a.month.cmp(&b.month));

    let total = common::sum2(rows.iter().map(|e| e.amount));
    Ok(ExpensesReport { total, by_category, by_month })
}

/// **`reports_get_shifts_report`** (13b §3.7). Quirk Q-1 kept literally: a shift's `salesTotal` only
/// counts an invoice whose **day key string** is not-less-than the opening day key and not-greater-
/// than the closing instant key, compared as byte strings.
pub async fn shifts_report<C: ConnectionTrait>(conn: &C, range: &DateRangeInput) -> TxResult<Vec<ShiftReportRow>> {
    let date_range = DateRange::parse(range)?;
    let rows = shifts::Entity::find().filter(shifts::Column::Status.eq(shifts::ShiftStatus::Closed)).all(conn).await.map_err(AppError::from)?;
    let rows: Vec<_> = rows.into_iter().filter(|s| common::in_range(s.opened_at_day, &date_range)).collect();

    let users_map: std::collections::HashMap<Id, String> = users::Entity::find().all(conn).await.map_err(AppError::from)?.into_iter().map(|u| (u.id, u.name)).collect();

    // Non-draft invoices, loaded once (the shift set is typically small; the invoice set per
    // cashier is filtered in Rust below, same shape as the mock's own `db.invoices.filter(...)`).
    let all_invoices = invoices::Entity::find().all(conn).await.map_err(AppError::from)?;

    let mut out = Vec::new();
    for s in &rows {
        let opened_key = s.opened_at().key();
        let closed_key = s.closed_at().map(|d| d.key()).unwrap_or_else(|| opened_key.clone());
        let sales_total = common::sum2(all_invoices.iter().filter(|i| {
            i.status != invoices::InvoiceStatus::Draft && i.cashier_id == s.opened_by && {
                // The mock's `inDateRange(i.date, …)` compares the invoice's local **day key**
                // (`localDateKey`), not its instant, against the shift's raw instant strings (Q-1).
                let k = i.date_day.format("%Y-%m-%d").to_string();
                !(k.as_str() < opened_key.as_str()) && !(k.as_str() > closed_key.as_str())
            }
        }).map(|i| i.grand_total));

        out.push(ShiftReportRow {
            id: s.id,
            number: s.number.clone(),
            terminal_id: s.terminal_id,
            opened_at: opened_key,
            closed_at: s.closed_at().map(|d| d.key()),
            opened_by: users_map.get(&s.opened_by).cloned().unwrap_or_else(|| s.opened_by.to_string()),
            closed_by: s.closed_by.map(|id| users_map.get(&id).cloned().unwrap_or_else(|| id.to_string())),
            expected_cash: s.expected_cash.unwrap_or(Decimal::ZERO),
            counted_cash: s.counted_cash.unwrap_or(Decimal::ZERO),
            variance: s.variance.unwrap_or(Decimal::ZERO),
            sales_total,
        });
    }
    out.sort_by(|a, b| b.opened_at.cmp(&a.opened_at));
    Ok(out)
}

/// Extension so `shifts::Model` exposes `DocDate` accessors the same way every other DocDate entity
/// does (the entity itself doesn't define `opened_at`/`closed_at` helpers yet — added here rather
/// than editing the shared entity file, which is out of this domain's owned paths).
trait ShiftDocDates {
    fn opened_at(&self) -> crate::utils::dates::DocDate;
    fn closed_at(&self) -> Option<crate::utils::dates::DocDate>;
}
impl ShiftDocDates for shifts::Model {
    fn opened_at(&self) -> crate::utils::dates::DocDate {
        crate::entities::doc_date::read(self.opened_at_day, self.opened_at_instant)
    }
    fn closed_at(&self) -> Option<crate::utils::dates::DocDate> {
        self.closed_at_day.map(|day| crate::entities::doc_date::read(day, self.closed_at_instant))
    }
}

/// **`reports_get_purchases_report`** (13b §3.12).
pub async fn purchases_report<C: ConnectionTrait>(conn: &C, range: &DateRangeInput) -> TxResult<PurchasesReport> {
    let date_range = DateRange::parse(range)?;
    let pos = purchase_orders::Entity::find().filter(purchase_orders::Column::Status.eq(purchase_orders::PurchaseStatus::Received)).all(conn).await.map_err(AppError::from)?;
    let pos: Vec<_> = pos.into_iter().filter(|p| common::in_range(p.date_day, &date_range)).collect();

    let returns = purchase_returns::Entity::find().all(conn).await.map_err(AppError::from)?;
    let returns: Vec<_> = returns.into_iter().filter(|r| common::in_range(r.date_day, &date_range)).collect();

    let supplier_ids: std::collections::HashSet<Id> = pos.iter().map(|p| p.supplier_id).collect();
    let suppliers: std::collections::HashMap<Id, String> = if supplier_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        crate::entities::parties::parties::Entity::find()
            .filter(crate::entities::parties::parties::Column::Id.is_in(supplier_ids.iter().copied()))
            .all(conn)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(|p| (p.id, p.name))
            .collect()
    };

    let mut by_supplier_map: IndexMap<Id, (i32, Decimal)> = IndexMap::new();
    for p in &pos {
        let entry = by_supplier_map.entry(p.supplier_id).or_insert((0, Decimal::ZERO));
        entry.0 += 1;
        entry.1 = round2(entry.1 + p.grand_total);
    }
    let mut by_supplier: Vec<PurchasesBySupplier> = by_supplier_map
        .into_iter()
        .map(|(id, (count, total))| PurchasesBySupplier { supplier_id: id.to_string(), name: suppliers.get(&id).cloned().unwrap_or_else(|| id.to_string()), count, total })
        .collect();
    by_supplier.sort_by(|a, b| b.total.cmp(&a.total));

    let mut lines_by_po: std::collections::HashMap<Id, Vec<purchase_order_lines::Model>> = std::collections::HashMap::new();
    for p in &pos {
        let lines = purchase_order_lines::Entity::find().filter(purchase_order_lines::Column::PurchaseOrderId.eq(p.id)).all(conn).await.map_err(AppError::from)?;
        lines_by_po.insert(p.id, lines);
    }
    let product_ids: std::collections::HashSet<Id> = lines_by_po.values().flatten().map(|l| l.product_id).collect();
    let products: std::collections::HashMap<Id, String> = if product_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        crate::entities::catalog::products::Entity::find()
            .filter(crate::entities::catalog::products::Column::Id.is_in(product_ids.iter().copied()))
            .all(conn)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(|p| (p.id, p.name))
            .collect()
    };

    let mut by_product_map: IndexMap<Id, (Decimal, Decimal)> = IndexMap::new();
    for lines in lines_by_po.values() {
        for l in lines {
            let entry = by_product_map.entry(l.product_id).or_insert((Decimal::ZERO, Decimal::ZERO));
            entry.0 += l.qty;
            entry.1 = round2(entry.1 + l.qty * l.cost_price);
        }
    }
    let mut by_product: Vec<PurchasesByProduct> = by_product_map
        .into_iter()
        .map(|(id, (qty, total))| PurchasesByProduct {
            product_id: id.to_string(),
            name: products.get(&id).cloned().unwrap_or_else(|| id.to_string()),
            qty: round2(qty),
            total,
            avg_price: if qty > Decimal::ZERO { round2(total / qty) } else { Decimal::ZERO },
        })
        .collect();
    by_product.sort_by(|a, b| b.total.cmp(&a.total));

    let summary = PurchasesReportSummary {
        po_count: pos.len() as i32,
        gross_purchases: common::sum2(pos.iter().map(|p| p.sub_total)),
        vat: common::sum2(pos.iter().map(|p| p.tax_amount)),
        total: common::sum2(pos.iter().map(|p| p.grand_total)),
        returns: common::sum2(returns.iter().map(|r| r.grand_total)),
    };

    Ok(PurchasesReport { summary, by_supplier, by_product })
}
