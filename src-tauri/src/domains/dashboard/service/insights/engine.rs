//! `dashboard_compute_insights` engine (14b-insights.md §3.0, port of `insightEngine.ts:103-116`).

use sea_orm::ConnectionTrait;

use crate::core::auth::Role;
use crate::core::tx::TxResult;

use super::super::super::dto::InsightDto;
use super::common::{InsightCtx, Thresholds};
use super::rules;

/// Runs the 21 rules in catalogue order (`ir:635-657`); a rule that errors is skipped and logged
/// ("a single bad rule shouldn't break the whole home screen", `ie:108-112`), then keeps only the
/// insights whose `roles` contain `actor_role` (I-5: server authority — the TS side's `role` arg
/// only keys its mirror cache).
pub async fn compute_all<C: ConnectionTrait>(conn: &C, today: chrono::NaiveDate, now: chrono::DateTime<chrono::Utc>, numerals: crate::domains::analytics::dto::Numerals, actor_role: Role) -> TxResult<Vec<InsightDto>> {
    let thresholds = Thresholds::load(conn).await?;
    let ctx = InsightCtx { today, now, thresholds, numerals: numerals.into() };

    let mut all: Vec<InsightDto> = Vec::new();

    macro_rules! run_rule {
        ($key:literal, $f:expr) => {
            match $f.await {
                Ok(mut v) => all.append(&mut v),
                Err(e) => log::warn!(target: "insights", "rule {} failed: {:?}", $key, e),
            }
        };
    }

    run_rule!("reorder", rules::stock::reorder(conn, &ctx));
    run_rule!("dead-stock", rules::stock::dead_stock(conn, &ctx));
    run_rule!("expiring", rules::stock::expiring(conn, &ctx));
    run_rule!("overdue-customers", rules::receivables::overdue_customers(conn, &ctx));
    run_rule!("credit-limit", rules::receivables::credit_limit(conn, &ctx));
    run_rule!("supplier-dues", rules::cash::supplier_dues(conn, &ctx));
    run_rule!("vat-deadline", rules::cash::vat_deadline(conn, &ctx));
    run_rule!("cash-drawer", rules::cash::cash_drawer(conn, &ctx));
    run_rule!("shift-open", rules::cash::shift_open(conn, &ctx));
    run_rule!("unsettled-cards", rules::cash::unsettled_cards(conn, &ctx));
    run_rule!("below-cost", rules::stock::below_cost(conn, &ctx));
    run_rule!("discount-leak", rules::sales::discount_leak(conn, &ctx));
    run_rule!("refund-spike", rules::sales::refund_spike(conn, &ctx));
    run_rule!("recurring-journal-due", rules::accounting::recurring_journal_due(conn, &ctx));
    run_rule!("recurring-expense-due", rules::accounting::recurring_expense_due(conn, &ctx));
    run_rule!("budget", rules::accounting::budget(conn, &ctx));
    run_rule!("opening-balance-equity", rules::accounting::opening_balance_equity(conn, &ctx));
    run_rule!("backup-overdue", rules::accounting::backup_overdue(conn, &ctx));
    run_rule!("year-end", rules::accounting::year_end(conn, &ctx));
    run_rule!("good-news", rules::sales::good_news(conn, &ctx));
    run_rule!("missing-supplier-invoice", rules::stock::missing_supplier_invoice(conn, &ctx));

    all.retain(|i| i.roles.contains(&actor_role));
    Ok(all)
}
