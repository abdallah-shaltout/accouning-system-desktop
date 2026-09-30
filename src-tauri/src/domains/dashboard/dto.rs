//! `dashboard` domain DTOs (03-domains/14-analytics.md §2 + 14b-insights.md §2). Exported to
//! `core/types/gen/` (the Vue module is `core`).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::core::auth::Role;
use crate::utils::id::Id;
use crate::utils::money::serde_number;
use crate::utils::route::RouteRef;

// --- dashboard_get_dashboard_summary (14 §3.4) -----------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct SalesTrendDay {
    pub date: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct DashboardSummary {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub today_sales: Decimal,
    pub today_invoice_count: i32,
    pub unpaid_invoice_count: i32,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub unpaid_invoice_total: Decimal,
    pub low_stock_count: i32,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cash_position: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cash_on_hand: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub bank_balance: Decimal,
    pub sales_trend: Vec<SalesTrendDay>,
}

// --- dashboard_get_home_kpis (14 §3.5) ---------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "core/types/gen/")]
pub enum HomePeriod {
    Today,
    Week,
    Month,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct HomeKpi {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub value: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub previous: Decimal,
    /// Not `skip_serializing_if` — the TS type is `number | null`, always present.
    #[serde(with = "serde_number::option")]
    #[ts(type = "number | null")]
    pub change_pct: Option<Decimal>,
    #[ts(type = "number[]")]
    pub sparkline: Vec<SparklinePoint>,
}

/// A single sparkline value crossing the wire as a plain JSON number (P2-33) — `Vec<Decimal>`
/// itself has no `serde(with = ...)` hook for element-wise (de)serialization, so this thin
/// transparent wrapper gives each element the same `serde_number` treatment as every other
/// `Decimal` field, while still exporting as `number[]` to TS.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SparklinePoint(#[serde(with = "serde_number")] pub Decimal);

impl From<Decimal> for SparklinePoint {
    fn from(value: Decimal) -> Self {
        SparklinePoint(value)
    }
}

/// `HomeKpi & { marginPct: number }` (flat struct — Simplify-checked, G-38).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct GrossProfitKpi {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub value: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub previous: Decimal,
    #[serde(with = "serde_number::option")]
    #[ts(type = "number | null")]
    pub change_pct: Option<Decimal>,
    #[ts(type = "number[]")]
    pub sparkline: Vec<SparklinePoint>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub margin_pct: Decimal,
}

/// `HomeKpi & { overdue: number }` (flat struct — Simplify-checked, G-38).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct ReceivablesKpi {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub value: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub previous: Decimal,
    #[serde(with = "serde_number::option")]
    #[ts(type = "number | null")]
    pub change_pct: Option<Decimal>,
    #[ts(type = "number[]")]
    pub sparkline: Vec<SparklinePoint>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub overdue: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct HomeTrendPoint {
    pub date: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
    /// Always set by the server (`#[ts(optional)]` only to match the TS `previousTotal?` spelling).
    #[ts(optional, type = "number")]
    #[serde(with = "serde_number")]
    pub previous_total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct HomeKpis {
    pub net_sales: HomeKpi,
    pub gross_profit: GrossProfitKpi,
    pub cash: HomeKpi,
    pub receivables: ReceivablesKpi,
    pub sales_trend: Vec<HomeTrendPoint>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct DashboardGetHomeKpisArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<HomePeriod>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct DashboardGetLowStockProductsArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct DashboardGetRecentInvoicesArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct DashboardGetRecentActivityArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct DashboardGetTopProductsArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<HomePeriod>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct DashboardGetTopCustomersArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<HomePeriod>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct TopProductRow {
    pub id: String,
    pub name: String,
    pub sku: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub gross_profit: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct TopCustomerRow {
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
}

/// `Invoice & { customerName?: string }` — `#[serde(flatten)]` over the 08 `Invoice` DTO.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct RecentInvoice {
    #[serde(flatten)]
    #[ts(flatten)]
    pub invoice: crate::domains::invoices::dto::Invoice,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_name: Option<String>,
}

/// `ActivityEntry & { userName?: string }` — `#[serde(flatten)]` over `core::dto::ActivityEntry`.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct RecentActivityEntry {
    #[serde(flatten)]
    #[ts(flatten)]
    pub entry: crate::core::dto::ActivityEntry,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
}

/// A `Decimal` that crosses as a plain `number` — `ipc_sig!` needs a concrete `TS` return type for
/// the 3 mirrored scalar reads (`dashboard_get_stock_value_snapshot`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(transparent)]
#[ts(export_to = "core/types/gen/")]
pub struct DecimalValue(#[serde(with = "serde_number")] #[ts(type = "number")] pub Decimal);

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct DashboardGetInTransitTransfersArgs {
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home_branch: Option<Id>,
}

// =================================================================================================
// 14b-insights.md — the insight engine
// =================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "core/types/gen/")]
pub enum InsightSeverity {
    Critical,
    Warning,
    Info,
    Positive,
}

/// The 18 icon names the mock's `insightRules.ts`/`insightEngine.ts` reference — serialized
/// verbatim (PascalCase, the Lucide component names) so the frontend's `INSIGHT_ICONS` map can key
/// off the same string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "core/types/gen/")]
pub enum InsightIcon {
    AlertTriangle,
    Banknote,
    CalendarRange,
    Clock,
    CreditCard,
    DatabaseBackup,
    FileWarning,
    Landmark,
    PackageX,
    Percent,
    PiggyBank,
    Repeat,
    RotateCcw,
    ScrollText,
    Sparkles,
    TimerOff,
    TrendingDown,
    UserX,
}

/// `Insight` (`insightTypes.ts:13-31`) minus the Vue `icon: Component` (this ships the icon *key*;
/// the frontend maps it to the actual component, `toInsight` in `insightEngine.ts`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct InsightDto {
    pub id: String,
    pub rule_key: String,
    pub severity: InsightSeverity,
    pub message: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric: Option<String>,
    pub action_label: String,
    #[ts(type = "import('@/modules/core/types/route').AppRoute")]
    pub action_to: RouteRef,
    pub icon: InsightIcon,
    #[ts(type = "import('../../../users/types').Role[]")]
    pub roles: Vec<Role>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub value: Decimal,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct DashboardComputeInsightsArgs {
    pub numerals: crate::domains::analytics::dto::Numerals,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct DashboardGetProductInlineHintsArgs {
    #[ts(type = "string")]
    pub product_id: Id,
}
