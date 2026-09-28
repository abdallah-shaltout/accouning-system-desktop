//! Rules 12 (discount-leak), 13 (refund-spike), 20 (good-news) (14b-insights.md §3.1).

use indexmap::IndexMap;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::org::users::Entity as UserEntity;
use crate::entities::sales::invoices::{Column as InvoiceColumn, Entity as InvoiceEntity, InvoiceStatus};
use crate::entities::sales::refunds::{Column as RefundColumn, Entity as RefundEntity};
use crate::utils::money::round2;
use crate::utils::route::RouteRef;

use super::super::common::{fmt_money, fmt_num, InsightCtx};
use super::super::super::super::dto::{InsightDto, InsightIcon, InsightSeverity};

/// 12 — discount leak (`ir:372-409`).
pub async fn discount_leak<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    let week_ago = ctx.today - chrono::Duration::days(7);
    let this_week: Vec<_> = InvoiceEntity::find()
        .filter(InvoiceColumn::Status.ne(InvoiceStatus::Draft))
        .filter(InvoiceColumn::DateDay.gte(week_ago))
        .filter(InvoiceColumn::GrandTotal.gt(Decimal::ZERO))
        .filter(InvoiceColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?;
    if this_week.len() < 4 {
        return Ok(Vec::new());
    }

    let mut by_cashier: IndexMap<crate::utils::id::Id, Vec<Decimal>> = IndexMap::new();
    for inv in &this_week {
        by_cashier.entry(inv.cashier_id).or_default().push(inv.discount_rate);
    }
    let all_rates: Vec<Decimal> = this_week.iter().map(|i| i.discount_rate).collect();
    let avg = if !all_rates.is_empty() { all_rates.iter().sum::<Decimal>() / Decimal::from(all_rates.len() as i64) } else { Decimal::ZERO };
    if avg <= Decimal::ZERO {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for (cashier_id, rates) in by_cashier {
        if rates.len() < 3 {
            continue;
        }
        let cashier_avg = rates.iter().sum::<Decimal>() / Decimal::from(rates.len() as i64);
        if cashier_avg < avg * ctx.thresholds.discount_leak_multiplier {
            continue;
        }
        let user_name = UserEntity::find_by_id(cashier_id).one(conn).await.map_err(AppError::from)?.map(|u| u.name).unwrap_or_else(|| cashier_id.to_string());
        out.push(InsightDto {
            id: format!("discount-leak:{cashier_id}"),
            rule_key: "discount-leak".to_string(),
            severity: InsightSeverity::Warning,
            message: format!(
                "متوسط خصم \"{}\" {}% مقابل {}% لبقية الكاشيرين هذا الأسبوع",
                user_name,
                fmt_num(cashier_avg, 1, ctx.numerals),
                fmt_num(avg, 1, ctx.numerals)
            ),
            metric: Some(format!("{}%", fmt_num(cashier_avg, 1, ctx.numerals))),
            action_label: "تقرير المبيعات".to_string(),
            action_to: RouteRef::list("report-sales"),
            icon: InsightIcon::Percent,
            roles: vec![crate::core::auth::Role::Manager, crate::core::auth::Role::Admin],
            value: cashier_avg,
            created_at: ctx.today.format("%Y-%m-%d").to_string(),
        });
    }
    Ok(out)
}

/// 13 — refund spike (`ir:412-441`).
pub async fn refund_spike<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    let week_ago = ctx.today - chrono::Duration::days(7);
    let four_weeks_ago = ctx.today - chrono::Duration::days(28);

    let this_week = RefundEntity::find()
        .filter(RefundColumn::DateDay.gte(week_ago))
        .filter(RefundColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?;
    if this_week.is_empty() {
        return Ok(Vec::new());
    }
    let prior_weeks = RefundEntity::find()
        .filter(RefundColumn::DateDay.gte(four_weeks_ago))
        .filter(RefundColumn::DateDay.lt(week_ago))
        .filter(RefundColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?;

    let this_week_total: Decimal = this_week.iter().map(|r| r.grand_total).sum();
    let prior_avg_weekly = if !prior_weeks.is_empty() { prior_weeks.iter().map(|r| r.grand_total).sum::<Decimal>() / Decimal::from(3) } else { Decimal::ZERO };
    if prior_avg_weekly <= Decimal::ZERO || this_week_total < prior_avg_weekly * ctx.thresholds.refund_spike_multiplier {
        return Ok(Vec::new());
    }
    Ok(vec![InsightDto {
        id: "refund-spike:all".to_string(),
        rule_key: "refund-spike".to_string(),
        severity: InsightSeverity::Warning,
        message: format!(
            "المرتجعات هذا الأسبوع {} — أعلى من المتوسط ({})",
            fmt_money(this_week_total, ctx.numerals),
            fmt_money(round2(prior_avg_weekly), ctx.numerals)
        ),
        metric: Some(fmt_money(this_week_total, ctx.numerals)),
        action_label: "تقرير المبيعات".to_string(),
        action_to: RouteRef::list("report-sales"),
        icon: InsightIcon::RotateCcw,
        roles: vec![crate::core::auth::Role::Manager, crate::core::auth::Role::Admin],
        value: this_week_total,
        created_at: ctx.today.format("%Y-%m-%d").to_string(),
    }])
}

/// 20 — good news: best sales day (`ir:581-609`).
pub async fn good_news<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    let all = InvoiceEntity::find()
        .filter(InvoiceColumn::Status.ne(InvoiceStatus::Draft))
        .filter(InvoiceColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?;

    let mut by_day: IndexMap<chrono::NaiveDate, Decimal> = IndexMap::new();
    for inv in &all {
        let entry = by_day.entry(inv.date_day).or_insert(Decimal::ZERO);
        *entry = round2(*entry + inv.grand_total);
    }

    let Some(&today_total) = by_day.get(&ctx.today) else { return Ok(Vec::new()) };
    if today_total <= Decimal::ZERO {
        return Ok(Vec::new());
    }
    let best = by_day.iter().filter(|(d, _)| **d != ctx.today).map(|(_, v)| *v).fold(Decimal::ZERO, |a, v| a.max(v));
    if today_total <= best {
        return Ok(Vec::new());
    }
    let today_key = ctx.today.format("%Y-%m-%d").to_string();
    Ok(vec![InsightDto {
        id: format!("good-news:{today_key}"),
        rule_key: "good-news".to_string(),
        severity: InsightSeverity::Positive,
        message: format!("اليوم أفضل يوم مبيعات مسجل بقيمة {} 🎉", fmt_money(today_total, ctx.numerals)),
        metric: Some(fmt_money(today_total, ctx.numerals)),
        action_label: "عرض المبيعات".to_string(),
        action_to: RouteRef::list("invoices"),
        icon: InsightIcon::Sparkles,
        roles: vec![crate::core::auth::Role::Manager, crate::core::auth::Role::Admin],
        value: today_total,
        created_at: today_key,
    }])
}
