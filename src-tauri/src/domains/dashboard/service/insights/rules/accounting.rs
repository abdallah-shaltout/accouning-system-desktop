//! Rules 14 (recurring-journal-due), 15 (recurring-expense-due), 16 (budget),
//! 17 (opening-balance-equity), 18 (backup-overdue), 19 (year-end) (14b-insights.md §3.1).

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::expenses::recurring_expenses::{Column as RecurringExpenseColumn, Entity as RecurringExpenseEntity};
use crate::entities::journal::journal_templates::{Column as TemplateColumn, Entity as TemplateEntity};
use crate::entities::org::cost_center_budgets::{Column as BudgetColumn, Entity as BudgetEntity};
use crate::entities::org::cost_centers::{Column as CostCenterColumn, Entity as CostCenterEntity};
use crate::entities::org::fiscal_years::Entity as FiscalYearEntity;
use crate::shared::ledger::accounts::SystemRole;
use crate::utils::route::RouteRef;

use super::super::common::{acct_balance, days_between, fmt_money, fmt_num, InsightCtx};
use super::super::super::super::dto::{InsightDto, InsightIcon, InsightSeverity};

/// 14 — recurring journal due (`ir:444-458`).
pub async fn recurring_journal_due<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    let due = TemplateEntity::find()
        .filter(TemplateColumn::RecurrenceNextDate.lte(ctx.today))
        .filter(TemplateColumn::RecurrenceNextDate.is_not_null())
        .filter(TemplateColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?;

    Ok(due
        .iter()
        .map(|t| InsightDto {
            id: format!("recurring-journal:{}", t.id),
            rule_key: "recurring-journal-due".to_string(),
            severity: InsightSeverity::Info,
            message: format!("قيد متكرر \"{}\" مستحق الترحيل", t.name),
            metric: None,
            action_label: "ترحيل الآن".to_string(),
            action_to: RouteRef::list("journal-templates"),
            icon: InsightIcon::Repeat,
            roles: vec![crate::core::auth::Role::Accountant, crate::core::auth::Role::Admin],
            value: Decimal::from(200),
            created_at: ctx.today.format("%Y-%m-%d").to_string(),
        })
        .collect())
}

/// 15 — recurring expense due (`ir:460-475`).
pub async fn recurring_expense_due<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    let due = RecurringExpenseEntity::find()
        .filter(RecurringExpenseColumn::Active.eq(true))
        .filter(RecurringExpenseColumn::NextDate.lte(ctx.today))
        .filter(RecurringExpenseColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?;

    Ok(due
        .iter()
        .map(|r| InsightDto {
            id: format!("recurring-expense:{}", r.id),
            rule_key: "recurring-expense-due".to_string(),
            severity: InsightSeverity::Info,
            message: format!("مصروف متكرر \"{}\" بمبلغ {} مستحق التسجيل", r.name, fmt_money(r.amount, ctx.numerals)),
            metric: Some(fmt_money(r.amount, ctx.numerals)),
            action_label: "تسجيل الآن".to_string(),
            action_to: RouteRef::list("expenses"),
            icon: InsightIcon::Repeat,
            roles: vec![crate::core::auth::Role::Accountant, crate::core::auth::Role::Admin],
            value: r.amount,
            created_at: ctx.today.format("%Y-%m-%d").to_string(),
        })
        .collect())
}

/// 16 — budget (`ir:478-509`). Quirk Q-5: compares raw date-key strings, so an entry stamped with
/// an instant on the fiscal year's last day is excluded (a `YYYY-MM-DDTHH:...` string always sorts
/// after `YYYY-MM-DD`, i.e. after the plain `end` key it's compared against).
pub async fn budget<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    use crate::entities::journal::journal_entries::Entity as EntryEntity;
    use crate::entities::journal::journal_lines::{Column as LineColumn, Entity as LineEntity};
    use crate::entities::org::accounts::Entity as AccountEntity;

    use crate::entities::org::fiscal_years::Column as FiscalYearColumn;
    let years = FiscalYearEntity::find().order_by_asc(FiscalYearColumn::CreatedAt).order_by_asc(FiscalYearColumn::Id).all(conn).await.map_err(AppError::from)?;
    let fy = years
        .iter()
        .find(|f| !f.is_closed && f.start_date <= ctx.today && f.end_date >= ctx.today)
        .or_else(|| years.iter().find(|f| !f.is_closed));
    let Some(fy) = fy else { return Ok(Vec::new()) };

    let cost_centers = CostCenterEntity::find().filter(CostCenterColumn::DeletedAt.is_null()).all(conn).await.map_err(AppError::from)?;
    let start_key = fy.start_date.format("%Y-%m-%d").to_string();
    let end_key = fy.end_date.format("%Y-%m-%d").to_string();

    let mut out = Vec::new();
    for c in &cost_centers {
        let Some(budget_row) = BudgetEntity::find()
            .filter(BudgetColumn::CostCenterId.eq(c.id))
            .filter(BudgetColumn::FiscalYearId.eq(fy.id))
            .one(conn)
            .await
            .map_err(AppError::from)?
        else {
            continue;
        };
        if budget_row.amount <= Decimal::ZERO {
            continue;
        }

        let lines = LineEntity::find().filter(LineColumn::CostCenterId.eq(c.id)).all(conn).await.map_err(AppError::from)?;
        let mut actual = Decimal::ZERO;
        for line in &lines {
            let Some(entry) = EntryEntity::find_by_id(line.journal_entry_id).one(conn).await.map_err(AppError::from)? else { continue };
            let entry_key = entry.date().key();
            if entry_key.as_str() < start_key.as_str() || entry_key.as_str() > end_key.as_str() {
                continue;
            }
            let Some(account) = AccountEntity::find_by_id(line.account_id).one(conn).await.map_err(AppError::from)? else { continue };
            if account.kind == "EXPENSE" {
                actual += line.debit - line.credit;
            }
        }
        let pct = (actual / budget_row.amount) * Decimal::from(100);
        if pct < ctx.thresholds.budget_near_pct {
            continue;
        }
        out.push(InsightDto {
            id: format!("budget:{}", c.id),
            rule_key: "budget".to_string(),
            severity: if pct >= Decimal::from(100) { InsightSeverity::Critical } else { InsightSeverity::Warning },
            message: format!(
                "مركز التكلفة \"{}\" تجاوز {}% من ميزانيته ({} من {})",
                c.name,
                fmt_num(pct, 0, ctx.numerals),
                fmt_money(actual, ctx.numerals),
                fmt_money(budget_row.amount, ctx.numerals)
            ),
            metric: Some(format!("{}%", fmt_num(pct, 0, ctx.numerals))),
            action_label: "تقرير مراكز التكلفة".to_string(),
            action_to: RouteRef::list("report-cost-centers"),
            icon: InsightIcon::PiggyBank,
            roles: vec![crate::core::auth::Role::Manager, crate::core::auth::Role::Admin],
            value: actual,
            created_at: ctx.today.format("%Y-%m-%d").to_string(),
        });
    }
    Ok(out)
}

/// 17 — opening balance equity (`ir:512-530`).
pub async fn opening_balance_equity<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    let balance = acct_balance(conn, SystemRole::OpeningBalanceEquity).await?;
    if balance.abs() < Decimal::new(1, 2) {
        return Ok(Vec::new());
    }
    Ok(vec![InsightDto {
        id: "opening-balance-equity:all".to_string(),
        rule_key: "opening-balance-equity".to_string(),
        severity: InsightSeverity::Info,
        message: format!("رصيد حساب \"أرصدة افتتاحية\" {} — يحتاج إقفال إلى رأس المال", fmt_money(balance, ctx.numerals)),
        metric: Some(fmt_money(balance, ctx.numerals)),
        action_label: "قيد يومية".to_string(),
        action_to: RouteRef::list("journal-new"),
        icon: InsightIcon::ScrollText,
        roles: vec![crate::core::auth::Role::Accountant, crate::core::auth::Role::Admin],
        value: balance.abs(),
        created_at: ctx.today.format("%Y-%m-%d").to_string(),
    }])
}

/// 18 — backup overdue (`ir:533-553`). I-4: reads `settings.backup.last_backup_at` from the DB.
pub async fn backup_overdue<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    let settings = crate::core::settings::load(conn).await?;
    let last_backup_at = settings.backup.and_then(|b| b.last_backup_at);
    let days_since = last_backup_at.map(|at| days_between(ctx.today, crate::utils::dates::local_date_key(at, None)));
    let is_overdue = match days_since {
        Some(d) => Decimal::from(d) >= ctx.thresholds.backup_overdue_days,
        None => true, // Infinity >= threshold in the mock.
    };
    if !is_overdue {
        return Ok(Vec::new());
    }
    let (message, metric, value) = match days_since {
        Some(d) => (
            format!("لم يتم أخذ نسخة احتياطية منذ {} يوماً", fmt_num(Decimal::from(d), 0, ctx.numerals)),
            Some(format!("{} يوم", fmt_num(Decimal::from(d), 0, ctx.numerals))),
            Decimal::from(d),
        ),
        None => ("لم يتم أخذ أي نسخة احتياطية بعد".to_string(), None, Decimal::from(9999)),
    };
    Ok(vec![InsightDto {
        id: "backup-overdue:all".to_string(),
        rule_key: "backup-overdue".to_string(),
        severity: InsightSeverity::Critical,
        message,
        metric,
        action_label: "أخذ نسخة الآن".to_string(),
        action_to: RouteRef::list("settings-backup"),
        icon: InsightIcon::DatabaseBackup,
        roles: vec![crate::core::auth::Role::Admin],
        value,
        created_at: ctx.today.format("%Y-%m-%d").to_string(),
    }])
}

/// 19 — year end (`ir:556-578`). Quirk Q-1: sign inversion — `d = today − end` (a positive `d`
/// means the year already ended `d` days ago; a `-0` is treated as "not ended" since `-0 < 0` is
/// false in both JS and Rust).
pub async fn year_end<C: ConnectionTrait>(conn: &C, ctx: &InsightCtx) -> TxResult<Vec<InsightDto>> {
    use crate::entities::org::fiscal_years::Column as FiscalYearColumn;
    let years = FiscalYearEntity::find().order_by_asc(FiscalYearColumn::CreatedAt).order_by_asc(FiscalYearColumn::Id).all(conn).await.map_err(AppError::from)?;
    let mut out = Vec::new();
    for fy in &years {
        if fy.is_closed {
            continue;
        }
        let d = -days_between(fy.end_date, ctx.today);
        if d < -365 || Decimal::from(d) > ctx.thresholds.year_end_days {
            continue;
        }
        let ended = d < 0;
        out.push(InsightDto {
            id: format!("year-end:{}", fy.id),
            rule_key: "year-end".to_string(),
            severity: if ended { InsightSeverity::Critical } else { InsightSeverity::Warning },
            message: if ended {
                format!("السنة المالية \"{}\" انتهت ولم تُقفل بعد", fy.name)
            } else {
                format!("السنة المالية \"{}\" تنتهي خلال {} يوماً", fy.name, fmt_num(Decimal::from(d), 0, ctx.numerals))
            },
            metric: if ended { None } else { Some(format!("{} يوم", fmt_num(Decimal::from(d), 0, ctx.numerals))) },
            action_label: "إغلاق السنة".to_string(),
            action_to: RouteRef::list("fiscal-years"),
            icon: InsightIcon::CalendarRange,
            roles: vec![crate::core::auth::Role::Accountant, crate::core::auth::Role::Admin],
            value: if ended { Decimal::from(5000) } else { ctx.thresholds.year_end_days - Decimal::from(d) },
            created_at: ctx.today.format("%Y-%m-%d").to_string(),
        });
    }
    Ok(out)
}
