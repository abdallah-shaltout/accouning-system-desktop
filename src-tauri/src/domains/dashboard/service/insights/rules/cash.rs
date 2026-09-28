//! Rules 6 (supplier-dues), 7 (vat-deadline), 8 (cash-drawer), 9 (shift-open), 10 (unsettled-cards)
//! (14b-insights.md §3.1).

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::parties::parties::{Column as PartyColumn, Entity as PartyEntity};
use crate::entities::sales::shifts::{Column as ShiftColumn, Entity as ShiftEntity, ShiftStatus};
use crate::shared::balances::open_purchase_orders_for;
use crate::shared::ledger::accounts::SystemRole;
use crate::utils::route::RouteRef;

use super::super::common::{acct_balance, day_of_key, fmt_money, fmt_num, js_num, neg_zero_text, trunc_days, InsightCtx};
use super::super::super::super::dto::{InsightDto, InsightIcon, InsightSeverity};

fn round2(d: Decimal) -> Decimal {
    crate::utils::money::round2(d)
}

/// 6 — supplier dues vs cash (`ir:207-237`).
pub async fn supplier_dues<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    let cutoff = ctx.today + chrono::Duration::days(trunc_days(ctx.thresholds.supplier_due_days));
    let cutoff_key = cutoff.format("%Y-%m-%d").to_string();

    let suppliers = PartyEntity::find().filter(PartyColumn::Kind.eq("supplier")).filter(PartyColumn::DeletedAt.is_null()).all(conn).await.map_err(AppError::from)?;

    let mut due_total = Decimal::ZERO;
    for s in &suppliers {
        let docs = open_purchase_orders_for(conn, s.id).await?;
        for d in &docs {
            let ref_key = match d.due_date {
                Some(due) => due.format("%Y-%m-%d").to_string(),
                None => d.date.key(),
            };
            let ref_day_key = day_of_key(&ref_key);
            if ref_day_key <= cutoff_key.as_str() {
                due_total = round2(due_total + d.outstanding);
            }
        }
    }
    if due_total <= Decimal::ZERO {
        return Ok(Vec::new());
    }
    let cash = round2(acct_balance(conn, SystemRole::Cash).await? + acct_balance(conn, SystemRole::Bank).await?);
    if due_total < cash {
        return Ok(Vec::new());
    }
    Ok(vec![InsightDto {
        id: "supplier-dues:all".to_string(),
        rule_key: "supplier-dues".to_string(),
        severity: InsightSeverity::Warning,
        message: format!(
            "مستحقات {} خلال {} أيام — السيولة المتاحة {}",
            fmt_money(due_total, ctx.numerals),
            js_num(ctx.thresholds.supplier_due_days),
            fmt_money(cash, ctx.numerals)
        ),
        metric: Some(fmt_money(due_total, ctx.numerals)),
        action_label: "التخطيط للسداد".to_string(),
        action_to: RouteRef::list("payments"),
        icon: InsightIcon::Landmark,
        roles: vec![crate::core::auth::Role::Accountant, crate::core::auth::Role::Manager, crate::core::auth::Role::Admin],
        value: due_total,
        created_at: ctx.today.format("%Y-%m-%d").to_string(),
    }])
}

/// 7 — VAT deadline (`ir:240-263`). Quirk Q-1: fires only on the last day of the month (`daysLeft`
/// is `-0`), and the message always prints the `NEG_ZERO` text.
pub async fn vat_deadline<C: ConnectionTrait>(_conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    use chrono::Datelike;

    if ctx.thresholds.vat_deadline_days < Decimal::ZERO {
        return Ok(Vec::new());
    }

    // periodEnd = last day of the month before this one; deadline = last day of the month after
    // periodEnd (= this month's last day). JS: `new Date(periodEnd.getFullYear(), periodEnd.getMonth() + 2, 0)`
    // — `getMonth()` is 0-based, so this uses `month0()`, not the 1-based `month()`.
    let period_end = last_day_of_previous_month(ctx.today);
    let deadline = last_day_of_month(period_end.year(), period_end.month0() + 2);
    let days_left = -(super::super::common::days_between(deadline, ctx.today));
    if days_left < 0 || Decimal::from(days_left) > ctx.thresholds.vat_deadline_days {
        return Ok(Vec::new());
    }
    let quarter = (period_end.month0() / 3) + 1;
    let neg_zero = neg_zero_text(ctx.numerals);
    Ok(vec![InsightDto {
        id: format!("vat-deadline:{}-q{}", period_end.year(), quarter),
        rule_key: "vat-deadline".to_string(),
        severity: InsightSeverity::Warning,
        message: format!("إقرار الربع {quarter} مستحق خلال {neg_zero} أيام"),
        metric: Some(format!("{neg_zero} يوم")),
        action_label: "تسوية الضريبة".to_string(),
        action_to: RouteRef::list("vat-settlement"),
        icon: InsightIcon::FileWarning,
        roles: vec![crate::core::auth::Role::Accountant, crate::core::auth::Role::Manager, crate::core::auth::Role::Admin],
        value: (ctx.thresholds.vat_deadline_days + Decimal::ONE) * Decimal::from(1000),
        created_at: ctx.today.format("%Y-%m-%d").to_string(),
    }])
}

fn last_day_of_month(year: i32, month0_based_overflow: u32) -> chrono::NaiveDate {
    // `month0_based_overflow` may exceed 11 (periodEnd.month + 2 in JS `Date` semantics rolls the
    // year forward) — normalize to a real (year, month) pair first.
    let total_months0 = (year as i64) * 12 + (month0_based_overflow as i64);
    let y = (total_months0.div_euclid(12)) as i32;
    let m0 = total_months0.rem_euclid(12) as u32; // 0-based month of the *next* month.
    // JS `new Date(y, m0, 0)` = the last day of month `m0 - 1` (1-based) in year `y`, i.e. the day
    // before the 1st of month `m0`.
    let first_of_next = chrono::NaiveDate::from_ymd_opt(y, m0 + 1, 1).unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(y + 1, 1, 1).unwrap());
    first_of_next - chrono::Duration::days(1)
}

fn last_day_of_previous_month(today: chrono::NaiveDate) -> chrono::NaiveDate {
    use chrono::Datelike;
    // `new Date(today.year, today.month0, 0)` — the day before the 1st of `today`'s month.
    let first_of_this_month = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    first_of_this_month - chrono::Duration::days(1)
}

/// 8 — cash in drawer (`ir:266-284`).
pub async fn cash_drawer<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    let cash = acct_balance(conn, SystemRole::Cash).await?;
    if cash <= ctx.thresholds.cash_drawer_limit {
        return Ok(Vec::new());
    }
    Ok(vec![InsightDto {
        id: "cash-drawer:all".to_string(),
        rule_key: "cash-drawer".to_string(),
        severity: InsightSeverity::Warning,
        message: format!("النقدية في الصندوق {} — أودِعها في البنك", fmt_money(cash, ctx.numerals)),
        metric: Some(fmt_money(cash, ctx.numerals)),
        action_label: "سند تحويل".to_string(),
        action_to: RouteRef::list("vouchers"),
        icon: InsightIcon::Banknote,
        roles: vec![crate::core::auth::Role::Manager, crate::core::auth::Role::Admin, crate::core::auth::Role::Cashier],
        value: cash,
        created_at: ctx.today.format("%Y-%m-%d").to_string(),
    }])
}

/// 9 — shift not closed (`ir:287-310`). Quirk Q-2: "now" is today's UTC midnight, not the current
/// instant.
pub async fn shift_open<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    let now = ctx.today.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let shifts = ShiftEntity::find().filter(ShiftColumn::Status.eq(ShiftStatus::Open)).all(conn).await.map_err(AppError::from)?;

    let mut out = Vec::new();
    for s in &shifts {
        let opened = s.opened_at_instant.unwrap_or_else(|| s.opened_at_day.and_hms_opt(0, 0, 0).unwrap().and_utc());
        let hours = Decimal::from((now - opened).num_milliseconds()) / Decimal::from(3_600_000);
        let threshold_hours = ctx.thresholds.shift_open_hours;
        if hours < threshold_hours {
            continue;
        }
        let h = hours.floor().to_i64().unwrap_or(0);
        out.push(InsightDto {
            id: format!("shift-open:{}", s.id),
            rule_key: "shift-open".to_string(),
            severity: InsightSeverity::Warning,
            message: format!("وردية {} مفتوحة منذ {} ساعة", s.number, fmt_num(Decimal::from(h), 0, ctx.numerals)),
            metric: Some(format!("{} ساعة", fmt_num(Decimal::from(h), 0, ctx.numerals))),
            action_label: "إغلاق قسري".to_string(),
            action_to: RouteRef::list("pos-shifts"),
            icon: InsightIcon::Clock,
            roles: vec![crate::core::auth::Role::Manager, crate::core::auth::Role::Admin],
            value: Decimal::from(h),
            created_at: ctx.today.format("%Y-%m-%d").to_string(),
        });
    }
    Ok(out)
}

/// 10 — unsettled card/wallet clearing (`ir:313-343`).
pub async fn unsettled_cards<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    use crate::entities::journal::journal_entries::{Column as EntryColumn, Entity as EntryEntity};
    use crate::entities::journal::journal_lines::{Column as LineColumn, Entity as LineEntity};
    use crate::entities::org::accounts::{Column as AccountColumn, Entity as AccountEntity};

    let cutoff = ctx.today - chrono::Duration::days(trunc_days(ctx.thresholds.unsettled_clearing_days));

    let clearing_accounts = AccountEntity::find()
        .filter(AccountColumn::SystemRole.is_in([SystemRole::CardClearing.as_str(), SystemRole::WalletClearing.as_str()]))
        .filter(AccountColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?;
    if clearing_accounts.is_empty() {
        return Ok(Vec::new());
    }
    let account_ids: Vec<_> = clearing_accounts.iter().map(|a| a.id).collect();

    let entries = EntryEntity::find().filter(EntryColumn::DateDay.lte(cutoff)).all(conn).await.map_err(AppError::from)?;
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    let entry_ids: Vec<_> = entries.iter().map(|e| e.id).collect();

    let lines = LineEntity::find()
        .filter(LineColumn::AccountId.is_in(account_ids))
        .filter(LineColumn::JournalEntryId.is_in(entry_ids))
        .filter(LineColumn::Debit.gt(Decimal::ZERO))
        .all(conn)
        .await
        .map_err(AppError::from)?;
    if lines.is_empty() {
        return Ok(Vec::new());
    }
    let total = round2(lines.iter().map(|l| l.debit).sum());
    Ok(vec![InsightDto {
        id: "unsettled-cards:all".to_string(),
        rule_key: "unsettled-cards".to_string(),
        severity: InsightSeverity::Info,
        message: format!(
            "تحصيلات بطاقات/محافظ غير مسواة منذ أكثر من {} أيام بقيمة {}",
            js_num(ctx.thresholds.unsettled_clearing_days),
            fmt_money(total, ctx.numerals)
        ),
        metric: Some(fmt_money(total, ctx.numerals)),
        action_label: "تسوية البطاقات".to_string(),
        action_to: RouteRef::list("card-settlements"),
        icon: InsightIcon::CreditCard,
        roles: vec![crate::core::auth::Role::Accountant, crate::core::auth::Role::Admin],
        value: total,
        created_at: ctx.today.format("%Y-%m-%d").to_string(),
    }])
}
