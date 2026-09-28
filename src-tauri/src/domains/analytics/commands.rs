//! `analytics` IPC commands (03-domains/14-analytics.md §1) — 3 read-only reports, `with_read_ctx`,
//! `Analytics / Read`.

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_read_ctx, BoxFuture, TxResult};

use super::dto::{AnalyticsGetCustomerAnalyticsArgs, AnalyticsGetProductAnalyticsArgs, AnalyticsGetSalesAnalyticsArgs, CustomerAnalytics, ProductAnalytics, SalesAnalytics};
use super::service;

#[tauri::command]
pub async fn analytics_get_sales_analytics(state: State<'_, AppState>, args: AnalyticsGetSalesAnalyticsArgs) -> Result<SalesAnalytics, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Analytics, Access::Read).await?;
            let today = ctx.clock.today();
            service::sales::get_sales_analytics(tx, today, args.days, args.date_style.into(), args.numerals.into()).await
        }) as BoxFuture<'_, TxResult<SalesAnalytics>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_get_product_analytics(state: State<'_, AppState>, args: AnalyticsGetProductAnalyticsArgs) -> Result<ProductAnalytics, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Analytics, Access::Read).await?;
            let today = ctx.clock.today();
            service::products::get_product_analytics(tx, today, args.days, args.limit).await
        }) as BoxFuture<'_, TxResult<ProductAnalytics>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn analytics_get_customer_analytics(state: State<'_, AppState>, args: AnalyticsGetCustomerAnalyticsArgs) -> Result<CustomerAnalytics, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Analytics, Access::Read).await?;
            let today = ctx.clock.today();
            service::customers::get_customer_analytics(tx, today, args.days, args.limit).await
        }) as BoxFuture<'_, TxResult<CustomerAnalytics>>
    })
    .await
    .map_err(Into::into)
}
