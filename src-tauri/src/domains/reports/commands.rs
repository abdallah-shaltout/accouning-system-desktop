//! `reports` IPC commands (13 §1, 13b §1) — 32 commands, every one a read:
//! `with_read_ctx` + `ctx.require(tx, Area::Reports, Access::Read)`.

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::core::tx::{with_read_ctx, BoxFuture, TxResult};
use crate::utils::id::Id;

use super::dto::*;
use super::service::{cash_flow, ledgers, purchasing, receivables, sales, statements, stock, vat};

fn parse_id(raw: &str) -> Result<Id, ApiErrorPayload> {
    raw.parse::<Id>().map_err(|_| AppError::validation("معرّف غير صالح").into())
}

fn parse_optional_id(raw: &Option<String>) -> Result<Option<Id>, ApiErrorPayload> {
    match raw {
        Some(s) if !s.is_empty() => parse_id(s).map(Some),
        _ => Ok(None),
    }
}

// =================================================================================================
// 13 — statements & ledgers
// =================================================================================================

#[tauri::command]
pub async fn reports_get_trial_balance(state: State<'_, AppState>, args: ReportsGetTrialBalanceArgs) -> Result<Vec<TrialBalanceRow>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            statements::trial_balance(tx, &range).await
        }) as BoxFuture<'_, TxResult<Vec<TrialBalanceRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_profit_and_loss(state: State<'_, AppState>, args: ReportsGetProfitAndLossArgs) -> Result<ProfitAndLoss, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            statements::profit_and_loss(tx, &range).await
        }) as BoxFuture<'_, TxResult<ProfitAndLoss>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_profit_and_loss_comparison(state: State<'_, AppState>, args: ReportsGetProfitAndLossComparisonArgs) -> Result<PnlComparison, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        let compare_range = args.compare_range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            let (current, previous) = statements::profit_and_loss_comparison(tx, &range, &compare_range).await?;
            Ok(PnlComparison { current, previous })
        }) as BoxFuture<'_, TxResult<PnlComparison>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_cost_center_profit_and_loss(state: State<'_, AppState>, args: ReportsGetCostCenterProfitAndLossArgs) -> Result<CostCenterPnl, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            statements::cost_center_profit_and_loss(tx, &range).await
        }) as BoxFuture<'_, TxResult<CostCenterPnl>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_cost_center_budget_vs_actual(state: State<'_, AppState>, args: ReportsGetCostCenterBudgetVsActualArgs) -> Result<Vec<CostCenterBudgetRow>, ApiErrorPayload> {
    let fiscal_year_id = parse_id(&args.fiscal_year_id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            statements::cost_center_budget_vs_actual(tx, fiscal_year_id).await
        }) as BoxFuture<'_, TxResult<Vec<CostCenterBudgetRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_balance_sheet(state: State<'_, AppState>, args: ReportsGetBalanceSheetArgs) -> Result<BalanceSheet, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let as_of = args.as_of.clone();
        let dim = args.dim.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            statements::balance_sheet_from_str(tx, &as_of, dim).await
        }) as BoxFuture<'_, TxResult<BalanceSheet>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_account_ledger(state: State<'_, AppState>, args: ReportsGetAccountLedgerArgs) -> Result<AccountLedger, ApiErrorPayload> {
    let account_id = parse_id(&args.account_id)?;
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            ledgers::account_ledger(tx, account_id, &range).await
        }) as BoxFuture<'_, TxResult<AccountLedger>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_party_ledger(state: State<'_, AppState>, args: ReportsGetPartyLedgerArgs) -> Result<AccountLedger, ApiErrorPayload> {
    let party_id = parse_id(&args.party_id)?;
    let kind = args.kind;
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            ledgers::party_ledger(tx, kind, party_id, &range).await
        }) as BoxFuture<'_, TxResult<AccountLedger>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_vat_report(state: State<'_, AppState>, args: ReportsGetVatReportArgs) -> Result<VatReport, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            vat::vat_report(tx, &range).await
        }) as BoxFuture<'_, TxResult<VatReport>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_vat_detail(state: State<'_, AppState>, args: ReportsGetVatDetailArgs) -> Result<Vec<VatDetailRow>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            vat::vat_detail(tx, &range).await
        }) as BoxFuture<'_, TxResult<Vec<VatDetailRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_cash_flow_statement(state: State<'_, AppState>, args: ReportsGetCashFlowStatementArgs) -> Result<CashFlowStatement, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            cash_flow::cash_flow_statement(tx, &range, &ctx.clock).await
        }) as BoxFuture<'_, TxResult<CashFlowStatement>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_day_book(state: State<'_, AppState>, args: ReportsGetDayBookArgs) -> Result<Vec<DayBookEntry>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            ledgers::day_book(tx, &range).await
        }) as BoxFuture<'_, TxResult<Vec<DayBookEntry>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_period_comparison(state: State<'_, AppState>, args: ReportsGetPeriodComparisonArgs) -> Result<Vec<PeriodComparisonLine>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range_a = args.range_a.clone();
        let range_b = args.range_b.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            statements::period_comparison(tx, &range_a, &range_b).await
        }) as BoxFuture<'_, TxResult<Vec<PeriodComparisonLine>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_business_health_report(state: State<'_, AppState>, args: ReportsGetBusinessHealthReportArgs) -> Result<BusinessHealthReport, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            statements::business_health_report(tx, &range, &ctx.clock).await
        }) as BoxFuture<'_, TxResult<BusinessHealthReport>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_ledger_targets(state: State<'_, AppState>) -> Result<LedgerTargets, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            ledgers::ledger_targets(tx).await
        }) as BoxFuture<'_, TxResult<LedgerTargets>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_dimension_options(state: State<'_, AppState>) -> Result<DimensionOptions, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            ledgers::dimension_options(tx).await
        }) as BoxFuture<'_, TxResult<DimensionOptions>>
    })
    .await
    .map_err(Into::into)
}

// =================================================================================================
// 13b — operational reports
// =================================================================================================

#[tauri::command]
pub async fn reports_get_sales_report(state: State<'_, AppState>, args: ReportsGetSalesReportArgs) -> Result<SalesReport, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            sales::sales_report(tx, &range).await
        }) as BoxFuture<'_, TxResult<SalesReport>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_inventory_report(state: State<'_, AppState>) -> Result<Vec<InventoryReportRow>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            stock::inventory_report(tx).await
        }) as BoxFuture<'_, TxResult<Vec<InventoryReportRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_discounts_report(state: State<'_, AppState>, args: ReportsGetDiscountsReportArgs) -> Result<Vec<DiscountReportRow>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        let group_by = args.group_by;
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            sales::discounts_report(tx, &range, group_by).await
        }) as BoxFuture<'_, TxResult<Vec<DiscountReportRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_gross_profit_report(state: State<'_, AppState>, args: ReportsGetGrossProfitReportArgs) -> Result<Vec<GrossProfitRow>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        let group_by = args.group_by;
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            sales::gross_profit_report(tx, &range, group_by).await
        }) as BoxFuture<'_, TxResult<Vec<GrossProfitRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_returns_report(state: State<'_, AppState>, args: ReportsGetReturnsReportArgs) -> Result<ReturnsReport, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            sales::returns_report(tx, &range).await
        }) as BoxFuture<'_, TxResult<ReturnsReport>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_expenses_report(state: State<'_, AppState>, args: ReportsGetExpensesReportArgs) -> Result<ExpensesReport, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            purchasing::expenses_report(tx, &range).await
        }) as BoxFuture<'_, TxResult<ExpensesReport>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_shifts_report(state: State<'_, AppState>, args: ReportsGetShiftsReportArgs) -> Result<Vec<ShiftReportRow>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            purchasing::shifts_report(tx, &range).await
        }) as BoxFuture<'_, TxResult<Vec<ShiftReportRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_low_stock_report(state: State<'_, AppState>) -> Result<Vec<LowStockRow>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            stock::low_stock_report(tx).await
        }) as BoxFuture<'_, TxResult<Vec<LowStockRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_dead_stock_report(state: State<'_, AppState>, args: ReportsGetDeadStockReportArgs) -> Result<Vec<DeadStockRow>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let days = args.days;
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            stock::dead_stock_report(tx, days, &ctx.clock).await
        }) as BoxFuture<'_, TxResult<Vec<DeadStockRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_stocktake_variances(state: State<'_, AppState>, args: ReportsGetStocktakeVariancesArgs) -> Result<Vec<StocktakeVarianceRow>, ApiErrorPayload> {
    let count_id = parse_optional_id(&args.count_id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            stock::stocktake_variances(tx, count_id).await
        }) as BoxFuture<'_, TxResult<Vec<StocktakeVarianceRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_transfers_report(state: State<'_, AppState>, args: ReportsGetTransfersReportArgs) -> Result<Vec<TransferReportRow>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            stock::transfers_report(tx, &range).await
        }) as BoxFuture<'_, TxResult<Vec<TransferReportRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_purchases_report(state: State<'_, AppState>, args: ReportsGetPurchasesReportArgs) -> Result<PurchasesReport, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            purchasing::purchases_report(tx, &range).await
        }) as BoxFuture<'_, TxResult<PurchasesReport>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_branch_comparison(state: State<'_, AppState>, args: ReportsGetBranchComparisonArgs) -> Result<Vec<BranchComparisonRow>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            sales::branch_comparison(tx, &range).await
        }) as BoxFuture<'_, TxResult<Vec<BranchComparisonRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_profit_leakage_report(state: State<'_, AppState>, args: ReportsGetProfitLeakageReportArgs) -> Result<ProfitLeakageReport, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let range = args.range.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            sales::profit_leakage_report(tx, &range).await
        }) as BoxFuture<'_, TxResult<ProfitLeakageReport>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_aging_report(state: State<'_, AppState>, args: ReportsGetAgingReportArgs) -> Result<Vec<AgingReportRow>, ApiErrorPayload> {
    let kind = args.kind;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            receivables::aging_report(tx, kind, &ctx.clock).await
        }) as BoxFuture<'_, TxResult<Vec<AgingReportRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn reports_get_overdue_report(state: State<'_, AppState>, args: ReportsGetOverdueReportArgs) -> Result<Vec<OverdueRow>, ApiErrorPayload> {
    let kind = args.kind;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Reports, Access::Read).await?;
            receivables::overdue_report(tx, kind, &ctx.clock).await
        }) as BoxFuture<'_, TxResult<Vec<OverdueRow>>>
    })
    .await
    .map_err(Into::into)
}

/// 13 §1 (16) + 13b §1 (16) = 32 commands, in each file's table order.
pub fn ipc_signatures() -> Vec<crate::core::ipc::IpcSig> {
    vec![
        crate::ipc_sig!(reports_get_trial_balance, ReportsGetTrialBalanceArgs, Vec<TrialBalanceRow>),
        crate::ipc_sig!(reports_get_profit_and_loss, ReportsGetProfitAndLossArgs, ProfitAndLoss),
        crate::ipc_sig!(reports_get_profit_and_loss_comparison, ReportsGetProfitAndLossComparisonArgs, PnlComparison),
        crate::ipc_sig!(reports_get_cost_center_profit_and_loss, ReportsGetCostCenterProfitAndLossArgs, CostCenterPnl),
        crate::ipc_sig!(reports_get_cost_center_budget_vs_actual, ReportsGetCostCenterBudgetVsActualArgs, Vec<CostCenterBudgetRow>),
        crate::ipc_sig!(reports_get_balance_sheet, ReportsGetBalanceSheetArgs, BalanceSheet),
        crate::ipc_sig!(reports_get_account_ledger, ReportsGetAccountLedgerArgs, AccountLedger),
        crate::ipc_sig!(reports_get_party_ledger, ReportsGetPartyLedgerArgs, AccountLedger),
        crate::ipc_sig!(reports_get_vat_report, ReportsGetVatReportArgs, VatReport),
        crate::ipc_sig!(reports_get_vat_detail, ReportsGetVatDetailArgs, Vec<VatDetailRow>),
        crate::ipc_sig!(reports_get_cash_flow_statement, ReportsGetCashFlowStatementArgs, CashFlowStatement),
        crate::ipc_sig!(reports_get_day_book, ReportsGetDayBookArgs, Vec<DayBookEntry>),
        crate::ipc_sig!(reports_get_period_comparison, ReportsGetPeriodComparisonArgs, Vec<PeriodComparisonLine>),
        crate::ipc_sig!(reports_get_business_health_report, ReportsGetBusinessHealthReportArgs, BusinessHealthReport),
        crate::ipc_sig!(reports_get_ledger_targets, (), LedgerTargets),
        crate::ipc_sig!(reports_get_dimension_options, (), DimensionOptions),
        crate::ipc_sig!(reports_get_sales_report, ReportsGetSalesReportArgs, SalesReport),
        crate::ipc_sig!(reports_get_inventory_report, (), Vec<InventoryReportRow>),
        crate::ipc_sig!(reports_get_discounts_report, ReportsGetDiscountsReportArgs, Vec<DiscountReportRow>),
        crate::ipc_sig!(reports_get_gross_profit_report, ReportsGetGrossProfitReportArgs, Vec<GrossProfitRow>),
        crate::ipc_sig!(reports_get_returns_report, ReportsGetReturnsReportArgs, ReturnsReport),
        crate::ipc_sig!(reports_get_expenses_report, ReportsGetExpensesReportArgs, ExpensesReport),
        crate::ipc_sig!(reports_get_shifts_report, ReportsGetShiftsReportArgs, Vec<ShiftReportRow>),
        crate::ipc_sig!(reports_get_low_stock_report, (), Vec<LowStockRow>),
        crate::ipc_sig!(reports_get_dead_stock_report, ReportsGetDeadStockReportArgs, Vec<DeadStockRow>),
        crate::ipc_sig!(reports_get_stocktake_variances, ReportsGetStocktakeVariancesArgs, Vec<StocktakeVarianceRow>),
        crate::ipc_sig!(reports_get_transfers_report, ReportsGetTransfersReportArgs, Vec<TransferReportRow>),
        crate::ipc_sig!(reports_get_purchases_report, ReportsGetPurchasesReportArgs, PurchasesReport),
        crate::ipc_sig!(reports_get_branch_comparison, ReportsGetBranchComparisonArgs, Vec<BranchComparisonRow>),
        crate::ipc_sig!(reports_get_profit_leakage_report, ReportsGetProfitLeakageReportArgs, ProfitLeakageReport),
        crate::ipc_sig!(reports_get_aging_report, ReportsGetAgingReportArgs, Vec<AgingReportRow>),
        crate::ipc_sig!(reports_get_overdue_report, ReportsGetOverdueReportArgs, Vec<OverdueRow>),
    ]
}
