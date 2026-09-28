//! `dashboard` domain (03-domains/14-analytics.md + 14b-insights.md) — home KPIs, the notification
//! drawer's 6 reads, and the insight engine (21 rules + product inline hints). 15 commands total
//! (13 from 14, 2 from 14b). No undo compensators (§5 — pure reads).
//!
//! **Needs from manager** (blocking `cargo check` for this module — see this wave's final report):
//! `pub mod dashboard;` + `domains::dashboard::ipc_signatures()`/`export_bindings()` hooks in
//! `domains/mod.rs`, the matching `domains::dashboard::commands::*` lines in `lib.rs`'s
//! `generate_handler!`. Also G-39 (`products::service::transfers::to_dto` needs a `pub` twin,
//! `transfer_dto`, for `feed::in_transit_transfers` to call — see that file's doc comment).

pub mod commands;
pub mod dto;
pub mod service;

use crate::core::ipc::IpcSig;
use crate::ipc_sig;

/// 14 §1's 13 commands + 14b §1's 2 commands, table order.
pub fn ipc_signatures() -> Vec<IpcSig> {
    use dto as d;

    vec![
        ipc_sig!(dashboard_get_dashboard_summary, (), d::DashboardSummary),
        ipc_sig!(dashboard_get_home_kpis, d::DashboardGetHomeKpisArgs, d::HomeKpis),
        ipc_sig!(dashboard_get_low_stock_products, d::DashboardGetLowStockProductsArgs, Vec<crate::domains::products::dto::catalog::Product>),
        ipc_sig!(dashboard_get_recent_invoices, d::DashboardGetRecentInvoicesArgs, Vec<d::RecentInvoice>),
        ipc_sig!(dashboard_get_recent_activity, d::DashboardGetRecentActivityArgs, Vec<d::RecentActivityEntry>),
        ipc_sig!(dashboard_get_top_products, d::DashboardGetTopProductsArgs, Vec<d::TopProductRow>),
        ipc_sig!(dashboard_get_top_customers, d::DashboardGetTopCustomersArgs, Vec<d::TopCustomerRow>),
        ipc_sig!(dashboard_get_in_transit_transfers, d::DashboardGetInTransitTransfersArgs, Vec<crate::domains::products::dto::inventory::StockTransfer>),
        ipc_sig!(dashboard_get_pending_approval_requests, (), Vec<crate::domains::approvals::dto::ApprovalRequest>),
        ipc_sig!(dashboard_get_last_backup_failed_at, (), Option<String>),
        ipc_sig!(dashboard_get_journal_draft_count, (), i32),
        ipc_sig!(dashboard_get_stock_value_snapshot, (), d::DecimalValue),
        ipc_sig!(dashboard_has_any_products, (), bool),
        ipc_sig!(dashboard_compute_insights, d::DashboardComputeInsightsArgs, Vec<d::InsightDto>),
        ipc_sig!(dashboard_get_product_inline_hints, d::DashboardGetProductInlineHintsArgs, Vec<d::InsightDto>),
    ]
}

/// This domain's DTO exports (14 §2 + 14b §2) — one line the manager appends to
/// `domains::export_bindings` (`domains/mod.rs`) in the same commit that adds `pub mod dashboard;`
/// there.
pub fn export_bindings(cfg: &ts_rs::Config) {
    use ts_rs::TS;
    use dto as d;

    d::SalesTrendDay::export_all(cfg).expect("export SalesTrendDay");
    d::DashboardSummary::export_all(cfg).expect("export DashboardSummary");
    d::HomePeriod::export_all(cfg).expect("export HomePeriod");
    d::HomeKpi::export_all(cfg).expect("export HomeKpi");
    d::GrossProfitKpi::export_all(cfg).expect("export GrossProfitKpi");
    d::ReceivablesKpi::export_all(cfg).expect("export ReceivablesKpi");
    d::HomeTrendPoint::export_all(cfg).expect("export HomeTrendPoint");
    d::HomeKpis::export_all(cfg).expect("export HomeKpis");
    d::DashboardGetHomeKpisArgs::export_all(cfg).expect("export DashboardGetHomeKpisArgs");
    d::DashboardGetLowStockProductsArgs::export_all(cfg).expect("export DashboardGetLowStockProductsArgs");
    d::DashboardGetRecentInvoicesArgs::export_all(cfg).expect("export DashboardGetRecentInvoicesArgs");
    d::DashboardGetRecentActivityArgs::export_all(cfg).expect("export DashboardGetRecentActivityArgs");
    d::DashboardGetTopProductsArgs::export_all(cfg).expect("export DashboardGetTopProductsArgs");
    d::DashboardGetTopCustomersArgs::export_all(cfg).expect("export DashboardGetTopCustomersArgs");
    d::TopProductRow::export_all(cfg).expect("export TopProductRow");
    d::TopCustomerRow::export_all(cfg).expect("export TopCustomerRow");
    d::RecentInvoice::export_all(cfg).expect("export RecentInvoice");
    d::RecentActivityEntry::export_all(cfg).expect("export RecentActivityEntry");
    d::DecimalValue::export_all(cfg).expect("export DecimalValue");
    d::DashboardGetInTransitTransfersArgs::export_all(cfg).expect("export DashboardGetInTransitTransfersArgs");
    d::InsightSeverity::export_all(cfg).expect("export InsightSeverity");
    d::InsightIcon::export_all(cfg).expect("export InsightIcon");
    d::InsightDto::export_all(cfg).expect("export InsightDto");
    d::DashboardComputeInsightsArgs::export_all(cfg).expect("export DashboardComputeInsightsArgs");
    d::DashboardGetProductInlineHintsArgs::export_all(cfg).expect("export DashboardGetProductInlineHintsArgs");
}
