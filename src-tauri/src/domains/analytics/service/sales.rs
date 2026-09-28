//! `analytics_get_sales_analytics` (14-analytics.md §3.1, port of `analyticsService.ts:42-105`).

use std::collections::BTreeMap;

use indexmap::IndexMap;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::org::payment_methods::Entity as PaymentMethodEntity;
use crate::entities::sales::invoice_tenders::{Column as TenderColumn, Entity as TenderEntity};
use crate::entities::sales::invoices::{Column as InvoiceColumn, Entity as InvoiceEntity, InvoiceStatus};
use crate::entities::sales::refunds::{Column as RefundColumn, Entity as RefundEntity};
use crate::utils::format::{self, DateStyle, Numerals};
use crate::utils::money::round2;

use super::super::dto::{PaymentMixRow, SalesAnalytics, SalesTrendPoint, WeekdayRow};
use super::common::{js_num, validate_days};

const WEEKDAY_LABELS: [&str; 7] = ["الأحد", "الاثنين", "الثلاثاء", "الأربعاء", "الخميس", "الجمعة", "السبت"];

pub async fn get_sales_analytics<C: ConnectionTrait>(
    conn: &C,
    today: chrono::NaiveDate,
    days: Option<i32>,
    date_style: DateStyle,
    numerals: Numerals,
) -> TxResult<SalesAnalytics> {
    let days = validate_days(days)?;

    // --- 1/2. trend + trendInsight ---------------------------------------------------------------
    let from = today - chrono::Duration::days((days - 1) as i64);
    let sold_rows = InvoiceEntity::find()
        .filter(InvoiceColumn::Status.ne(InvoiceStatus::Draft))
        .filter(InvoiceColumn::DateDay.gte(from))
        .filter(InvoiceColumn::DateDay.lte(today))
        .filter(InvoiceColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let refund_rows = RefundEntity::find()
        .filter(RefundColumn::DateDay.gte(from))
        .filter(RefundColumn::DateDay.lte(today))
        .filter(RefundColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?;

    let mut gross_by_day: BTreeMap<chrono::NaiveDate, Decimal> = BTreeMap::new();
    for inv in &sold_rows {
        *gross_by_day.entry(inv.date_day).or_insert(Decimal::ZERO) += inv.grand_total;
    }
    let mut refunds_by_day: BTreeMap<chrono::NaiveDate, Decimal> = BTreeMap::new();
    for r in &refund_rows {
        *refunds_by_day.entry(r.date_day).or_insert(Decimal::ZERO) += r.grand_total;
    }

    let mut trend: Vec<SalesTrendPoint> = Vec::with_capacity(days as usize);
    for d in (0..days).rev() {
        let day = today - chrono::Duration::days(d as i64);
        let gross = gross_by_day.get(&day).copied().unwrap_or(Decimal::ZERO);
        let refunds = refunds_by_day.get(&day).copied().unwrap_or(Decimal::ZERO);
        trend.push(SalesTrendPoint { date: day.format("%Y-%m-%d").to_string(), total: round2(gross - refunds) });
    }

    let trend_len = Decimal::from(trend.len().max(1) as i64);
    let avg_total: Decimal = trend.iter().map(|t| t.total).sum::<Decimal>() / trend_len;
    let best_day = trend.iter().enumerate().fold(None::<(usize, Decimal)>, |acc, (i, t)| match acc {
        None => Some((i, t.total)),
        Some((_, best)) if t.total > best => Some((i, t.total)),
        other => other,
    });
    let trend_insight = match best_day {
        Some((idx, best_total)) if avg_total > Decimal::ZERO => {
            let day = today - chrono::Duration::days((trend.len() - 1 - idx) as i64);
            let pct = round2(((best_total - avg_total) / avg_total) * Decimal::from(100));
            format!(
                "أفضل يوم كان {} بمبيعات {} ر.س — أعلى بنسبة {}% من المتوسط",
                format::format_date_key(day, date_style, numerals),
                js_num(round2(best_total)),
                js_num(pct)
            )
        }
        _ => "لا توجد بيانات كافية بعد لهذه الفترة".to_string(),
    };

    // --- 3. recent (cutoff, no upper bound) --------------------------------------------------------
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

    // --- 4/5. byWeekday + weekdayInsight ------------------------------------------------------------
    use chrono::Datelike;
    let mut weekday_totals = [Decimal::ZERO; 7];
    let mut weekday_counts = [0i64; 7];
    for inv in &recent {
        // JS `getDay()`: 0 = Sunday. `chrono::Weekday::num_days_from_sunday()` matches.
        let dow = inv.date_day.weekday().num_days_from_sunday() as usize;
        weekday_totals[dow] += inv.grand_total;
        weekday_counts[dow] += 1;
    }
    let by_weekday: Vec<WeekdayRow> = (0..7)
        .map(|i| {
            let total = round2(weekday_totals[i]);
            let avg = if weekday_counts[i] > 0 { round2(weekday_totals[i] / Decimal::from(weekday_counts[i])) } else { Decimal::ZERO };
            WeekdayRow { day: WEEKDAY_LABELS[i].to_string(), total, avg }
        })
        .collect();

    let best_weekday_idx = by_weekday.iter().enumerate().fold(None::<(usize, Decimal)>, |acc, (i, w)| match acc {
        None => Some((i, w.avg)),
        Some((_, best)) if w.avg > best => Some((i, w.avg)),
        other => other,
    });
    let weekday_insight = match best_weekday_idx {
        Some((idx, best_avg)) if best_avg > Decimal::ZERO => {
            let rest_sum: Decimal = by_weekday.iter().enumerate().filter(|(i, _)| *i != idx).map(|(_, w)| w.avg).sum();
            let rest_avg = rest_sum / Decimal::from(6);
            if rest_avg > Decimal::ZERO {
                let pct = round2(((best_avg - rest_avg) / rest_avg) * Decimal::from(100));
                format!(
                    "{} هو أعلى يوم مبيعاً بمتوسط {} ر.س — أعلى بـ {}% من بقية الأيام",
                    by_weekday[idx].day,
                    format_short(best_avg),
                    js_num(pct)
                )
            } else {
                "لا توجد بيانات كافية بعد".to_string()
            }
        }
        _ => "لا توجد بيانات كافية بعد".to_string(),
    };

    // --- 6. paymentMix -------------------------------------------------------------------------------
    let recent_ids: Vec<_> = recent.iter().map(|i| i.id).collect();
    let tenders = if recent_ids.is_empty() {
        Vec::new()
    } else {
        TenderEntity::find()
            .filter(TenderColumn::InvoiceId.is_in(recent_ids))
            .order_by_asc(TenderColumn::Position)
            .all(conn)
            .await
            .map_err(AppError::from)?
    };
    let methods = PaymentMethodEntity::find().all(conn).await.map_err(AppError::from)?;
    let method_name = |id: crate::utils::id::Id| methods.iter().find(|m| m.id == id).map(|m| m.name.clone());

    let mut by_method: IndexMap<String, Decimal> = IndexMap::new();
    for t in &tenders {
        let label = method_name(t.payment_method_id).unwrap_or_else(|| "أخرى".to_string());
        let entry = by_method.entry(label).or_insert(Decimal::ZERO);
        *entry = round2(*entry + t.amount);
    }
    let method_total: Decimal = {
        let s: Decimal = by_method.values().copied().sum();
        if s == Decimal::ZERO { Decimal::ONE } else { s }
    };
    let mut payment_mix: Vec<PaymentMixRow> = by_method
        .into_iter()
        .map(|(method, total)| PaymentMixRow { method, total, pct: round2((total / method_total) * Decimal::from(100)) })
        .collect();
    payment_mix.sort_by(|a, b| b.total.cmp(&a.total));

    // --- 7. avgInvoice / avgItemsPerInvoice / returnsRatePct ------------------------------------------
    let recent_line_ids: Vec<_> = recent.iter().map(|i| i.id).collect();
    let lines = if recent_line_ids.is_empty() {
        Vec::new()
    } else {
        crate::entities::sales::invoice_lines::Entity::find()
            .filter(crate::entities::sales::invoice_lines::Column::InvoiceId.is_in(recent_line_ids))
            .all(conn)
            .await
            .map_err(AppError::from)?
    };
    let total_items: Decimal = lines.iter().map(|l| l.qty).sum();
    let n = Decimal::from(recent.len() as i64);
    let avg_invoice = if !recent.is_empty() {
        round2(recent.iter().map(|i| i.grand_total).sum::<Decimal>() / n)
    } else {
        Decimal::ZERO
    };
    let avg_items_per_invoice = if !recent.is_empty() { round2(total_items / n) } else { Decimal::ZERO };
    let recent_refunds_count = RefundEntity::find()
        .filter(RefundColumn::DateDay.gte(cutoff))
        .filter(RefundColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?
        .len();
    let returns_rate_pct = if !recent.is_empty() {
        round2((Decimal::from(recent_refunds_count as i64) / n) * Decimal::from(100))
    } else {
        Decimal::ZERO
    };

    Ok(SalesAnalytics {
        trend,
        trend_insight,
        by_weekday,
        weekday_insight,
        payment_mix,
        avg_invoice,
        avg_items_per_invoice,
        returns_rate_pct,
    })
}

fn format_short(v: Decimal) -> String {
    if v >= Decimal::from(1000) {
        format!("{}K", js_num(round2(v / Decimal::from(1000))))
    } else {
        js_num(round2(v))
    }
}
