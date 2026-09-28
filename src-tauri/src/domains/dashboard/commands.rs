//! `dashboard` IPC commands (03-domains/14-analytics.md §1, 14b-insights.md §1) — 13 dashboard
//! reads + 2 insight reads, `with_read_ctx`.

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::core::tx::{with_read_ctx, BoxFuture, TxResult};
use crate::utils::dates::format_iso_ms;

use super::dto::{
    DashboardComputeInsightsArgs, DashboardGetHomeKpisArgs, DashboardGetInTransitTransfersArgs, DashboardGetLowStockProductsArgs, DashboardGetProductInlineHintsArgs,
    DashboardGetRecentActivityArgs, DashboardGetRecentInvoicesArgs, DashboardGetTopCustomersArgs, DashboardGetTopProductsArgs, DashboardSummary, DecimalValue, HomeKpis, HomePeriod,
    InsightDto, RecentActivityEntry, RecentInvoice, TopCustomerRow, TopProductRow,
};
use super::service::{feed, insights, kpis};

fn require_actor_role(ctx: &crate::core::tx::ReadCtx) -> Result<crate::core::auth::Role, AppError> {
    Ok(ctx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.role)
}

// --- 14-analytics.md §1 ------------------------------------------------------------------------------

#[tauri::command]
pub async fn dashboard_get_dashboard_summary(state: State<'_, AppState>) -> Result<DashboardSummary, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Dashboard, Access::Read).await?;
            kpis::get_dashboard_summary(tx, ctx.clock.today()).await
        }) as BoxFuture<'_, TxResult<DashboardSummary>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn dashboard_get_home_kpis(state: State<'_, AppState>, args: DashboardGetHomeKpisArgs) -> Result<HomeKpis, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Dashboard, Access::Read).await?;
            let period = args.period.unwrap_or(HomePeriod::Today);
            let now_iso = format_iso_ms(ctx.clock.now);
            kpis::get_home_kpis(tx, ctx.clock.today(), &now_iso, period).await
        }) as BoxFuture<'_, TxResult<HomeKpis>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn dashboard_get_low_stock_products(state: State<'_, AppState>, args: DashboardGetLowStockProductsArgs) -> Result<Vec<crate::domains::products::dto::catalog::Product>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Dashboard, Access::Read).await?;
            feed::get_low_stock_products(tx, args.limit.unwrap_or(6).max(1) as usize).await
        }) as BoxFuture<'_, TxResult<Vec<crate::domains::products::dto::catalog::Product>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn dashboard_get_recent_invoices(state: State<'_, AppState>, args: DashboardGetRecentInvoicesArgs) -> Result<Vec<RecentInvoice>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Dashboard, Access::Read).await?;
            feed::get_recent_invoices(tx, args.limit.unwrap_or(8).max(1) as usize).await
        }) as BoxFuture<'_, TxResult<Vec<RecentInvoice>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn dashboard_get_recent_activity(state: State<'_, AppState>, args: DashboardGetRecentActivityArgs) -> Result<Vec<RecentActivityEntry>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Dashboard, Access::Read).await?;
            feed::get_recent_activity(tx, args.limit.unwrap_or(12).max(1) as usize).await
        }) as BoxFuture<'_, TxResult<Vec<RecentActivityEntry>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn dashboard_get_top_products(state: State<'_, AppState>, args: DashboardGetTopProductsArgs) -> Result<Vec<TopProductRow>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Dashboard, Access::Read).await?;
            let period = args.period.unwrap_or(HomePeriod::Month);
            kpis::get_top_products(tx, ctx.clock.today(), period, args.limit.unwrap_or(5).max(1) as usize).await
        }) as BoxFuture<'_, TxResult<Vec<TopProductRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn dashboard_get_top_customers(state: State<'_, AppState>, args: DashboardGetTopCustomersArgs) -> Result<Vec<TopCustomerRow>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Dashboard, Access::Read).await?;
            let period = args.period.unwrap_or(HomePeriod::Month);
            kpis::get_top_customers(tx, ctx.clock.today(), period, args.limit.unwrap_or(5).max(1) as usize).await
        }) as BoxFuture<'_, TxResult<Vec<TopCustomerRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn dashboard_get_in_transit_transfers(
    state: State<'_, AppState>,
    args: DashboardGetInTransitTransfersArgs,
) -> Result<Vec<crate::domains::products::dto::inventory::StockTransfer>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            feed::in_transit_transfers(tx, args.home_branch).await
        }) as BoxFuture<'_, TxResult<Vec<crate::domains::products::dto::inventory::StockTransfer>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn dashboard_get_pending_approval_requests(state: State<'_, AppState>) -> Result<Vec<crate::domains::approvals::dto::ApprovalRequest>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Approvals, Access::Read).await?;
            feed::pending_approval_requests(tx).await
        }) as BoxFuture<'_, TxResult<Vec<crate::domains::approvals::dto::ApprovalRequest>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn dashboard_get_last_backup_failed_at(state: State<'_, AppState>) -> Result<Option<String>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Dashboard, Access::Read).await?;
            feed::last_backup_failed_at(tx).await
        }) as BoxFuture<'_, TxResult<Option<String>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn dashboard_get_journal_draft_count(state: State<'_, AppState>) -> Result<i32, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Dashboard, Access::Read).await?;
            feed::journal_draft_count(tx).await
        }) as BoxFuture<'_, TxResult<i32>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn dashboard_get_stock_value_snapshot(state: State<'_, AppState>) -> Result<DecimalValue, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Dashboard, Access::Read).await?;
            feed::stock_value_snapshot(tx).await.map(DecimalValue)
        }) as BoxFuture<'_, TxResult<DecimalValue>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn dashboard_has_any_products(state: State<'_, AppState>) -> Result<bool, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Dashboard, Access::Read).await?;
            feed::has_any_products(tx).await
        }) as BoxFuture<'_, TxResult<bool>>
    })
    .await
    .map_err(Into::into)
}

// --- 14b-insights.md §1 -----------------------------------------------------------------------------

#[tauri::command]
pub async fn dashboard_compute_insights(state: State<'_, AppState>, args: DashboardComputeInsightsArgs) -> Result<Vec<InsightDto>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Dashboard, Access::Read).await?;
            let role = require_actor_role(ctx)?;
            insights::engine::compute_all(tx, ctx.clock.today(), ctx.clock.now, args.numerals, role).await
        }) as BoxFuture<'_, TxResult<Vec<InsightDto>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn dashboard_get_product_inline_hints(state: State<'_, AppState>, args: DashboardGetProductInlineHintsArgs) -> Result<Vec<InsightDto>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            let role = require_actor_role(ctx)?;
            insights::hints::get_product_inline_hints(tx, ctx.clock.today(), ctx.clock.now, args.product_id, role).await
        }) as BoxFuture<'_, TxResult<Vec<InsightDto>>>
    })
    .await
    .map_err(Into::into)
}
