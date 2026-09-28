//! `reports::service::cash_flow` (13 §3.11): the indirect-method cash flow statement.

use rust_decimal::Decimal;
use sea_orm::ConnectionTrait;

use crate::core::tx::TxResult;
use crate::utils::dates::BusinessClock;

use super::super::dto::{CashFlowLine, CashFlowStatement, DateRangeInput};
use super::common::{self, AccountsIndex, DateRange};

/// **`reports_get_cash_flow_statement`** (13 §3.11).
pub async fn cash_flow_statement<C: ConnectionTrait>(conn: &C, range: &DateRangeInput, clock: &BusinessClock) -> TxResult<CashFlowStatement> {
    let date_range = DateRange::parse(range)?;
    let idx = AccountsIndex::load(conn).await?;
    let mv_period = common::compute_pnl(&idx, &common::movements(conn, &date_range, None).await?);
    let pnl = mv_period;

    let cash_ids: std::collections::HashSet<_> = idx.iter_ordered().filter(|a| !a.is_group && a.kind == "ASSET" && (a.subtype == "cash" || a.subtype == "bank")).map(|a| a.id).collect();

    let opening_mv = match date_range.from {
        Some(from) => common::movements(conn, &DateRange::to_only(common::day_before(from)), None).await?,
        None => Default::default(),
    };
    let closing_to = date_range.to.unwrap_or_else(|| clock.today());
    let closing_mv = common::movements(conn, &DateRange::to_only(closing_to), None).await?;

    let sum_cash = |mv: &common::Movements| -> Decimal { cash_ids.iter().fold(Decimal::ZERO, |a, id| a + mv.get(id).map(|t| t.d - t.c).unwrap_or_default()) };
    let opening_cash = crate::utils::money::round2(sum_cash(&opening_mv));
    let closing_cash = crate::utils::money::round2(sum_cash(&closing_mv));

    let period_mv = common::movements(conn, &date_range, None).await?;
    let wc_account = |subtype: &str, sign: i64| -> Decimal {
        let Some(acc) = idx.iter_ordered().find(|a| !a.is_group && a.subtype == subtype) else { return Decimal::ZERO };
        let t = period_mv.get(&acc.id).copied().unwrap_or_default();
        crate::utils::money::round2(Decimal::from(sign) * (t.d - t.c))
    };

    let operating_adjustments: Vec<CashFlowLine> = vec![
        CashFlowLine { label: "التغير في الذمم المدينة (العملاء)".to_string(), amount: -wc_account("receivable", 1) },
        CashFlowLine { label: "التغير في المخزون".to_string(), amount: -wc_account("inventory", 1) },
        CashFlowLine { label: "التغير في الذمم الدائنة (الموردون)".to_string(), amount: wc_account("payable", -1) },
    ]
    .into_iter()
    .filter(|l| common::keep_significant(l.amount))
    .collect();
    let operating_cash = crate::utils::money::round2(pnl.net_income + common::sum2(operating_adjustments.iter().map(|l| l.amount)));

    let investing: Vec<CashFlowLine> = idx
        .iter_ordered()
        .filter(|a| !a.is_group && a.kind == "ASSET" && a.subtype == "fixedAsset")
        .map(|a| {
            let t = period_mv.get(&a.id).copied().unwrap_or_default();
            CashFlowLine { label: a.name.clone(), amount: crate::utils::money::round2(-(t.d - t.c)) }
        })
        .filter(|l| common::keep_significant(l.amount))
        .collect();
    let investing_cash = crate::utils::money::round2(common::sum2(investing.iter().map(|l| l.amount)));

    let financing: Vec<CashFlowLine> = idx
        .iter_ordered()
        .filter(|a| {
            !a.is_group
                && ((a.kind == "EQUITY" && a.system_role.as_deref() != Some("retainedEarnings") && a.system_role.as_deref() != Some("currentEarnings"))
                    || (a.kind == "LIABILITY" && a.subtype == "longTermLiability"))
        })
        .map(|a| {
            let t = period_mv.get(&a.id).copied().unwrap_or_default();
            CashFlowLine { label: a.name.clone(), amount: crate::utils::money::round2(t.c - t.d) }
        })
        .filter(|l| common::keep_significant(l.amount))
        .collect();
    let financing_cash = crate::utils::money::round2(common::sum2(financing.iter().map(|l| l.amount)));

    let net_change = crate::utils::money::round2(operating_cash + investing_cash + financing_cash);

    Ok(CashFlowStatement {
        net_income: pnl.net_income,
        operating_adjustments,
        operating_cash,
        investing,
        investing_cash,
        financing,
        financing_cash,
        net_change,
        opening_cash,
        closing_cash,
    })
}
