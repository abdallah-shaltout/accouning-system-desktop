//! `analytics_get_customer_analytics` (14-analytics.md §3.3, port of `analyticsService.ts:180-215`).

use indexmap::IndexMap;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::parties::parties::Entity as PartyEntity;
use crate::entities::sales::invoices::{Column as InvoiceColumn, Entity as InvoiceEntity, InvoiceStatus};
use crate::utils::money::round2;

use super::super::dto::{CustomerAnalytics, CustomerShare};
use super::common::{validate_days, validate_limit};

pub async fn get_customer_analytics<C: ConnectionTrait>(conn: &C, today: chrono::NaiveDate, days: Option<i32>, limit: Option<i32>) -> TxResult<CustomerAnalytics> {
    let days = validate_days(days)?;
    let limit = validate_limit(limit, 8)? as usize;
    let cutoff = today - chrono::Duration::days(days as i64);
    let cutoff_key = cutoff.format("%Y-%m-%d").to_string();

    // `soldInvoices()` in full, for firstSale-by-customer (all time, not bounded by `days`).
    let all_sold = InvoiceEntity::find()
        .filter(InvoiceColumn::Status.ne(InvoiceStatus::Draft))
        .filter(InvoiceColumn::DeletedAt.is_null())
        .order_by_asc(InvoiceColumn::CreatedAt)
        .order_by_asc(InvoiceColumn::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;

    // Quirk Q-3: `firstSale` is the mock's raw `inv.date` string (may be an ISO instant), compared
    // as a byte string against the day-key `cutoffKey` — use `DocDate::key()`, never `date_day`.
    let mut first_sale: IndexMap<crate::utils::id::Id, String> = IndexMap::new();
    for inv in &all_sold {
        let Some(customer_id) = inv.customer_id else { continue };
        let key = inv.date().key();
        match first_sale.get(&customer_id) {
            Some(prev) if inv.date().key() >= *prev => {}
            _ => {
                first_sale.insert(customer_id, key);
            }
        }
    }

    let recent = InvoiceEntity::find()
        .filter(InvoiceColumn::Status.ne(InvoiceStatus::Draft))
        .filter(InvoiceColumn::DateDay.gte(cutoff))
        .filter(InvoiceColumn::CustomerId.is_not_null())
        .filter(InvoiceColumn::DeletedAt.is_null())
        .order_by_asc(InvoiceColumn::CreatedAt)
        .order_by_asc(InvoiceColumn::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;

    // distinct recent customer ids, first-appearance order (IndexMap/IndexSet semantics).
    let mut recent_customer_order: IndexMap<crate::utils::id::Id, ()> = IndexMap::new();
    for inv in &recent {
        if let Some(cid) = inv.customer_id {
            recent_customer_order.entry(cid).or_insert(());
        }
    }

    let mut new_count = 0i32;
    let mut returning_count = 0i32;
    for &customer_id in recent_customer_order.keys() {
        match first_sale.get(&customer_id) {
            Some(fs) if fs.as_str() >= cutoff_key.as_str() => new_count += 1,
            _ => returning_count += 1,
        }
    }

    let active_count = recent_customer_order.len() as i32;
    let segment_insight = if active_count > 0 {
        let pct = round2((Decimal::from(new_count) / Decimal::from(active_count)) * Decimal::from(100));
        format!("{}% من العملاء النشطين هذه الفترة عملاء جدد", crate::utils::money::js_number_string(pct))
    } else {
        "لا توجد بيانات كافية بعد".to_string()
    };

    let mut by_customer: IndexMap<crate::utils::id::Id, Decimal> = IndexMap::new();
    for inv in &recent {
        if let Some(cid) = inv.customer_id {
            let entry = by_customer.entry(cid).or_insert(Decimal::ZERO);
            *entry = round2(*entry + inv.grand_total);
        }
    }

    let customer_ids: Vec<crate::utils::id::Id> = by_customer.keys().copied().collect();
    let customers = if customer_ids.is_empty() {
        Vec::new()
    } else {
        PartyEntity::find().filter(crate::entities::parties::parties::Column::Id.is_in(customer_ids)).all(conn).await.map_err(AppError::from)?
    };

    let mut sorted: Vec<CustomerShare> = by_customer
        .into_iter()
        .map(|(id, total)| {
            let name = customers.iter().find(|c| c.id == id).map(|c| c.name.clone()).unwrap_or_else(|| "—".to_string());
            CustomerShare { id, name, total }
        })
        .collect();
    sorted.sort_by(|a, b| b.total.cmp(&a.total));

    let total_revenue_raw: Decimal = sorted.iter().map(|c| c.total).sum();
    let total_revenue = if total_revenue_raw == Decimal::ZERO { Decimal::ONE } else { total_revenue_raw };
    let top10_sum: Decimal = sorted.iter().take(10).map(|c| c.total).sum();
    let top10_share_pct = round2((top10_sum / total_revenue) * Decimal::from(100));
    let concentration_insight = if sorted.len() >= 10 {
        format!("أفضل 10 عملاء يمثلون {}% من إجمالي الإيرادات", crate::utils::money::js_number_string(top10_share_pct))
    } else {
        "عدد العملاء غير كافٍ لحساب التركّز بدقة".to_string()
    };

    let top_customers: Vec<CustomerShare> = sorted.iter().take(limit).cloned().collect();

    Ok(CustomerAnalytics { new_count, returning_count, segment_insight, top_customers, top10_share_pct, concentration_insight })
}
