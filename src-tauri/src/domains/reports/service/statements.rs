//! `reports::service::statements` (13 §3.1–3.6, 3.13, 3.14): trial balance, P&L (+comparison),
//! cost-center P&L (+budget vs actual), balance sheet, period comparison, business health.

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::org::{cost_center_budgets, cost_centers, fiscal_years};
use crate::entities::soft_delete::SoftDelete;
use crate::utils::dates::BusinessClock;
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::super::dto::{
    BalanceSheet, BusinessHealthReport, BusinessHealthScore, CostCenterBudgetRow, CostCenterPnl, CostCenterPnlColumn, DateRangeInput, DimensionFilter, HealthScoreKey, PeriodComparisonLine,
    ProfitAndLoss, ReportRangeFilter, TrialBalanceRow,
};
use super::common::{self, sum2, AccountsIndex, DateRange};

fn clamp_0_25(d: Decimal) -> Decimal {
    let zero = Decimal::ZERO;
    let twenty_five = Decimal::from(25);
    if d < zero {
        zero
    } else if d > twenty_five {
        twenty_five
    } else {
        d
    }
}

/// **`reports_get_trial_balance`** (13 §3.1).
pub async fn trial_balance<C: ConnectionTrait>(conn: &C, range: &ReportRangeFilter) -> TxResult<Vec<TrialBalanceRow>> {
    let date_range = DateRange::parse(&range.range())?;
    let dim = range.dim();
    let opening = match date_range.from {
        Some(from) => common::movements(conn, &DateRange::to_only(common::day_before(from)), Some(&dim)).await?,
        None => Default::default(),
    };
    let period = common::movements(conn, &date_range, Some(&dim)).await?;
    let idx = AccountsIndex::load(conn).await?;

    let mut candidates: Vec<&common::AccountRow> = idx.iter_ordered().filter(|a| !a.is_group && (opening.contains_key(&a.id) || period.contains_key(&a.id))).collect();
    candidates.sort_by(|a, b| a.code.cmp(&b.code));

    Ok(candidates
        .into_iter()
        .map(|a| {
            let o = opening.get(&a.id).copied().unwrap_or_default();
            let p = period.get(&a.id).copied().unwrap_or_default();
            let closing = round2(o.d - o.c + p.d - p.c);
            let (closing_debit, closing_credit) = if closing > Decimal::ZERO { (closing, Decimal::ZERO) } else { (Decimal::ZERO, -closing) };
            TrialBalanceRow {
                account_id: a.id,
                code: a.code.clone(),
                name: a.name.clone(),
                group_name: idx.group_name_of(a),
                opening_balance: round2(o.d - o.c),
                period_debit: round2(p.d),
                period_credit: round2(p.c),
                closing_debit,
                closing_credit,
            }
        })
        .collect())
}

/// Builds `ProfitAndLoss` for one `(range, dim)` — the base every other statement calls into.
pub async fn compute_pnl<C: ConnectionTrait>(conn: &C, range: &DateRange, dim: Option<&DimensionFilter>) -> TxResult<ProfitAndLoss> {
    let mv = common::movements(conn, range, dim).await?;
    let idx = AccountsIndex::load(conn).await?;
    Ok(common::compute_pnl(&idx, &mv))
}

/// **`reports_get_profit_and_loss`** (13 §3.2).
pub async fn profit_and_loss<C: ConnectionTrait>(conn: &C, range: &ReportRangeFilter) -> TxResult<ProfitAndLoss> {
    let date_range = DateRange::parse(&range.range())?;
    let dim = range.dim();
    compute_pnl(conn, &date_range, Some(&dim)).await
}

/// **`reports_get_profit_and_loss_comparison`** (13 §3.3): `compareRange` reuses `range`'s dim.
pub async fn profit_and_loss_comparison<C: ConnectionTrait>(conn: &C, range: &ReportRangeFilter, compare_range: &DateRangeInput) -> TxResult<(ProfitAndLoss, ProfitAndLoss)> {
    let dim = range.dim();
    let a = DateRange::parse(&range.range())?;
    let b = DateRange::parse(compare_range)?;
    let current = compute_pnl(conn, &a, Some(&dim)).await?;
    let previous = compute_pnl(conn, &b, Some(&dim)).await?;
    Ok((current, previous))
}

fn pnl_column(mv: &common::Movements, idx: &AccountsIndex, cost_center_id: String, name: String) -> CostCenterPnlColumn {
    let pnl = common::compute_pnl(idx, mv);
    CostCenterPnlColumn { cost_center_id, name, net_revenue: pnl.net_revenue, total_cogs: pnl.total_cogs, gross_profit: pnl.gross_profit, total_expenses: pnl.total_expenses, net_income: pnl.net_income }
}

/// **`reports_get_cost_center_profit_and_loss`** (13 §3.4).
pub async fn cost_center_profit_and_loss<C: ConnectionTrait>(conn: &C, range: &DateRangeInput) -> TxResult<CostCenterPnl> {
    let date_range = DateRange::parse(range)?;
    let by_cc = common::movements_by_cost_center(conn, &date_range).await?;
    let idx = AccountsIndex::load(conn).await?;

    let cc_rows = cost_centers::Entity::find_live()
        .order_by_asc_created_at_id()
        .all(conn)
        .await
        .map_err(AppError::from)?;

    let mut centers: Vec<CostCenterPnlColumn> = cc_rows
        .into_iter()
        .filter(|c| by_cc.contains_key(&c.id.to_string()))
        .map(|c| {
            let mv = by_cc.get(&c.id.to_string()).cloned().unwrap_or_default();
            pnl_column(&mv, &idx, c.id.to_string(), c.name)
        })
        .collect();
    centers.sort_by(|a, b| b.net_revenue.cmp(&a.net_revenue));

    let unassigned_mv = by_cc.get(common::unassigned_cc_key()).cloned().unwrap_or_default();
    let unassigned = pnl_column(&unassigned_mv, &idx, common::unassigned_cc_key().to_string(), "غير مخصص".to_string());

    let total_pnl = compute_pnl(conn, &date_range, None).await?;
    let total = CostCenterPnlColumn {
        cost_center_id: "__total__".to_string(),
        name: "الإجمالي".to_string(),
        net_revenue: total_pnl.net_revenue,
        total_cogs: total_pnl.total_cogs,
        gross_profit: total_pnl.gross_profit,
        total_expenses: total_pnl.total_expenses,
        net_income: total_pnl.net_income,
    };

    Ok(CostCenterPnl { centers, unassigned, total })
}

/// **`reports_get_cost_center_budget_vs_actual`** (13 §3.5).
pub async fn cost_center_budget_vs_actual<C: ConnectionTrait>(conn: &C, fiscal_year_id: Id) -> TxResult<Vec<CostCenterBudgetRow>> {
    let fy = fiscal_years::Entity::find_by_id(fiscal_year_id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("السنة المالية غير موجودة"))?;

    let by_cc = common::movements_by_cost_center(conn, &DateRange { from: Some(fy.start_date), to: Some(fy.end_date) }).await?;
    let idx = AccountsIndex::load(conn).await?;

    let cc_rows = cost_centers::Entity::find_live().order_by_asc_created_at_id().all(conn).await.map_err(AppError::from)?;

    let mut rows = Vec::new();
    for c in cc_rows {
        let budgets = cost_center_budgets::Entity::find()
            .filter(cost_center_budgets::Column::CostCenterId.eq(c.id))
            .filter(cost_center_budgets::Column::FiscalYearId.eq(fiscal_year_id))
            .all(conn)
            .await
            .map_err(AppError::from)?;
        let Some(budget_row) = budgets.into_iter().next() else { continue };

        let mv = by_cc.get(&c.id.to_string()).cloned().unwrap_or_default();
        let actual = sum2(common::lines(&idx, "EXPENSE", &mv, 1, |_| true).into_iter().map(|l| l.amount));
        let budget = budget_row.amount;
        let variance_pct = if budget > Decimal::ZERO { round2((actual - budget) / budget * Decimal::from(100)) } else { Decimal::ZERO };
        let near_budget = budget > Decimal::ZERO && actual >= budget * Decimal::new(9, 1); // 0.9
        rows.push(CostCenterBudgetRow { cost_center_id: c.id, name: c.name, budget, actual, variance_pct, near_budget });
    }
    rows.sort_by(|a, b| b.actual.cmp(&a.actual));
    Ok(rows)
}

/// **`reports_get_balance_sheet`** (13 §3.6). Returns the raw pieces so `business_health` (13 §3.14)
/// can build on the exact same computation without a second HTTP-shaped round trip.
pub async fn balance_sheet<C: ConnectionTrait>(conn: &C, as_of: chrono::NaiveDate, dim: Option<&DimensionFilter>) -> TxResult<BalanceSheet> {
    let range = DateRange::to_only(as_of);
    let mv = common::movements(conn, &range, dim).await?;
    let idx = AccountsIndex::load(conn).await?;

    let assets = common::lines(&idx, "ASSET", &mv, 1, |_| true);
    let liabilities = common::lines(&idx, "LIABILITY", &mv, -1, |_| true);
    let equity = common::lines(&idx, "EQUITY", &mv, -1, |_| true);
    let unclosed_earnings = common::compute_pnl(&idx, &mv).net_income;

    let total_assets = sum2(assets.iter().map(|l| l.amount));
    let total_liabilities = sum2(liabilities.iter().map(|l| l.amount));
    let total_equity = round2(sum2(equity.iter().map(|l| l.amount)) + unclosed_earnings);
    let balanced = (total_assets - total_liabilities - total_equity).abs() < Decimal::new(1, 2); // 0.01

    Ok(BalanceSheet { as_of: as_of.format("%Y-%m-%d").to_string(), assets, total_assets, liabilities, total_liabilities, equity, unclosed_earnings, total_equity, balanced })
}

/// **`reports_get_balance_sheet`** command entry: parses `asOf`, validates it.
pub async fn balance_sheet_from_str<C: ConnectionTrait>(conn: &C, as_of: &str, dim: Option<DimensionFilter>) -> TxResult<BalanceSheet> {
    let day = chrono::NaiveDate::parse_from_str(as_of, "%Y-%m-%d").map_err(|_| AppError::validation("تاريخ غير صالح"))?;
    balance_sheet(conn, day, dim.as_ref()).await
}

/// **`reports_get_period_comparison`** (13 §3.13): exactly 5 fixed rows.
pub async fn period_comparison<C: ConnectionTrait>(conn: &C, range_a: &DateRangeInput, range_b: &DateRangeInput) -> TxResult<Vec<PeriodComparisonLine>> {
    let a = compute_pnl(conn, &DateRange::parse(range_a)?, None).await?;
    let b = compute_pnl(conn, &DateRange::parse(range_b)?, None).await?;

    let delta = |x: Decimal, y: Decimal, label: &str| -> PeriodComparisonLine {
        let delta = round2(x - y);
        let delta_pct = if y != Decimal::ZERO { round2((x - y) / y.abs() * Decimal::from(100)) } else { Decimal::ZERO };
        PeriodComparisonLine { label: label.to_string(), a: x, b: y, delta, delta_pct }
    };

    Ok(vec![
        delta(a.net_revenue, b.net_revenue, "صافي الإيرادات"),
        delta(a.total_cogs, b.total_cogs, "تكلفة المبيعات"),
        delta(a.gross_profit, b.gross_profit, "مجمل الربح"),
        delta(a.total_expenses, b.total_expenses, "المصروفات"),
        delta(a.net_income, b.net_income, "صافي الربح"),
    ])
}

/// **`reports_get_business_health_report`** (13 §3.14).
pub async fn business_health_report<C: ConnectionTrait>(conn: &C, range: &DateRangeInput, clock: &BusinessClock) -> TxResult<BusinessHealthReport> {
    let as_of = match range.to.as_deref().filter(|s| !s.is_empty()) {
        Some(s) => chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| AppError::validation("تاريخ غير صالح"))?,
        None => clock.today(),
    };
    let bs = balance_sheet(conn, as_of, None).await?;
    let date_range = DateRange::parse(range)?;
    let pnl = compute_pnl(conn, &date_range, None).await?;
    let idx = AccountsIndex::load(conn).await?;

    let current_assets = sum2(bs.assets.iter().filter(|l| idx.get(l.account_id).map(|a| a.subtype != "fixedAsset").unwrap_or(true)).map(|l| l.amount));
    let current_liabilities = sum2(bs.liabilities.iter().map(|l| l.amount));
    let current_ratio = if current_liabilities > Decimal::ZERO {
        current_assets / current_liabilities
    } else if current_assets > Decimal::ZERO {
        Decimal::from(2)
    } else {
        Decimal::ONE
    };
    let liquidity_score = clamp_0_25(round2(current_ratio / Decimal::from(2) * Decimal::from(25)));

    let net_margin_pct = if pnl.net_revenue > Decimal::ZERO { pnl.net_income / pnl.net_revenue * Decimal::from(100) } else { Decimal::ZERO };
    let profitability_score = clamp_0_25(round2(net_margin_pct / Decimal::from(20) * Decimal::from(25)));

    let dte = if bs.total_equity > Decimal::ZERO {
        bs.total_liabilities / bs.total_equity
    } else if bs.total_liabilities > Decimal::ZERO {
        Decimal::from(2)
    } else {
        Decimal::ZERO
    };
    let debt_score = clamp_0_25(round2(Decimal::from(25) - dte * Decimal::from(25)));

    let ar_account = idx.iter_ordered().find(|a| !a.is_group && a.subtype == "receivable");
    let ar_balance = match ar_account {
        Some(ar) => sum2(bs.assets.iter().filter(|l| l.account_id == ar.id).map(|l| l.amount)),
        None => Decimal::ZERO,
    };
    let days = match (range.from.as_deref().filter(|s| !s.is_empty()), range.to.as_deref().filter(|s| !s.is_empty())) {
        (Some(from), Some(to)) => {
            let from = chrono::NaiveDate::parse_from_str(from, "%Y-%m-%d").map_err(|_| AppError::validation("تاريخ غير صالح"))?;
            let to = chrono::NaiveDate::parse_from_str(to, "%Y-%m-%d").map_err(|_| AppError::validation("تاريخ غير صالح"))?;
            std::cmp::max(1, common::days_between(to, from))
        }
        _ => 30,
    };
    let dso = if pnl.net_revenue > Decimal::ZERO { ar_balance / pnl.net_revenue * Decimal::from(days) } else { Decimal::ZERO };
    let collection_score = clamp_0_25(round2(Decimal::from(25) - dso / Decimal::from(60) * Decimal::from(25)));

    let scores = vec![
        BusinessHealthScore {
            key: HealthScoreKey::Liquidity,
            label: "السيولة".to_string(),
            score: liquidity_score,
            value: round2(current_ratio),
            explanation: format!("نسبة التداول {} — الأصول المتداولة مقابل الالتزامات المتداولة", common::js_num(round2(current_ratio))),
        },
        BusinessHealthScore {
            key: HealthScoreKey::Profitability,
            label: "الربحية".to_string(),
            score: profitability_score,
            value: round2(net_margin_pct),
            explanation: format!("هامش صافي الربح {}%", common::js_num(round2(net_margin_pct))),
        },
        BusinessHealthScore {
            key: HealthScoreKey::Debt,
            label: "المديونية".to_string(),
            score: debt_score,
            value: round2(dte),
            explanation: format!("نسبة الالتزامات إلى حقوق الملكية {}", common::js_num(round2(dte))),
        },
        BusinessHealthScore {
            key: HealthScoreKey::Collection,
            label: "التحصيل".to_string(),
            score: collection_score,
            value: round2(dso),
            explanation: format!("متوسط أيام تحصيل الذمم (DSO) {} يوماً", common::js_num(round2(dso))),
        },
    ];
    let total = round2(sum2(scores.iter().map(|s| s.score)));
    Ok(BusinessHealthReport { total, scores })
}

/// Small extension trait: `(created_at, id)` order, the mock's array-insertion order, used by every
/// live-row list in this file.
trait OrderByCreatedAtId: Sized {
    fn order_by_asc_created_at_id(self) -> Self;
}

impl OrderByCreatedAtId for sea_orm::Select<cost_centers::Entity> {
    fn order_by_asc_created_at_id(self) -> Self {
        use sea_orm::QueryOrder;
        self.order_by_asc(cost_centers::Column::CreatedAt).order_by_asc(cost_centers::Column::Id)
    }
}
