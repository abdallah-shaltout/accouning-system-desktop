//! `analytics` domain (03-domains/14-analytics.md) — 3 read-only reports for the `/analytics`
//! route: sales trend/weekday/payment-mix, product profit top/bottom, customer new-vs-returning +
//! revenue concentration. No undo compensators (§5 — pure reads, nothing to undo).
//!
//! **Needs from manager** (blocking `cargo check` for this module — see this wave's final report):
//! `pub mod analytics;` + `domains::analytics::ipc_signatures()`/`export_bindings()` hooks in
//! `domains/mod.rs`, the matching `domains::analytics::commands::*` lines in `lib.rs`'s
//! `generate_handler!`.

pub mod commands;
pub mod dto;
pub mod service;

use crate::core::ipc::IpcSig;
use crate::ipc_sig;

/// §1's 3 commands, table order.
pub fn ipc_signatures() -> Vec<IpcSig> {
    use dto as d;

    vec![
        ipc_sig!(analytics_get_sales_analytics, d::AnalyticsGetSalesAnalyticsArgs, d::SalesAnalytics),
        ipc_sig!(analytics_get_product_analytics, d::AnalyticsGetProductAnalyticsArgs, d::ProductAnalytics),
        ipc_sig!(analytics_get_customer_analytics, d::AnalyticsGetCustomerAnalyticsArgs, d::CustomerAnalytics),
    ]
}

/// This domain's DTO exports (§2) — one line the manager appends to `domains::export_bindings`
/// (`domains/mod.rs`) in the same commit that adds `pub mod analytics;` there.
pub fn export_bindings(cfg: &ts_rs::Config) {
    use ts_rs::TS;
    use dto as d;

    d::Numerals::export_all(cfg).expect("export analytics::Numerals");
    d::DateStyle::export_all(cfg).expect("export analytics::DateStyle");
    d::SalesTrendPoint::export_all(cfg).expect("export SalesTrendPoint");
    d::WeekdayRow::export_all(cfg).expect("export WeekdayRow");
    d::PaymentMixRow::export_all(cfg).expect("export PaymentMixRow");
    d::SalesAnalytics::export_all(cfg).expect("export SalesAnalytics");
    d::AnalyticsGetSalesAnalyticsArgs::export_all(cfg).expect("export AnalyticsGetSalesAnalyticsArgs");
    d::ProductProfitRow::export_all(cfg).expect("export ProductProfitRow");
    d::ProductAnalytics::export_all(cfg).expect("export ProductAnalytics");
    d::AnalyticsGetProductAnalyticsArgs::export_all(cfg).expect("export AnalyticsGetProductAnalyticsArgs");
    d::CustomerShare::export_all(cfg).expect("export CustomerShare");
    d::CustomerAnalytics::export_all(cfg).expect("export CustomerAnalytics");
    d::AnalyticsGetCustomerAnalyticsArgs::export_all(cfg).expect("export AnalyticsGetCustomerAnalyticsArgs");
}
