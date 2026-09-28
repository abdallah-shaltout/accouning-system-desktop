//! `analytics` domain DTOs (03-domains/14-analytics.md §2) — a Rust port of
//! `src/modules/analytics/services/analyticsService.ts`'s 3 return shapes plus their args. Exported
//! to `analytics/types/gen/` (the Vue module is `analytics`).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::utils::money::serde_number;

/// `format.ts`'s `Numerals` (`'latn' | 'arab'`) — the wire/DTO-facing twin of
/// `utils::format::Numerals` (G-36), which has no serde/TS derives of its own since it is a pure
/// display-formatting helper type, not a DTO. Command args carry this one; service code converts
/// it to `utils::format::Numerals` at the boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "analytics/types/gen/")]
pub enum Numerals {
    Latn,
    Arab,
}

impl From<Numerals> for crate::utils::format::Numerals {
    fn from(value: Numerals) -> Self {
        match value {
            Numerals::Latn => crate::utils::format::Numerals::Latn,
            Numerals::Arab => crate::utils::format::Numerals::Arab,
        }
    }
}

/// `format.ts`'s `dateFormatStyle` (`'dmy' | 'ymd'`) — the wire/DTO-facing twin of
/// `utils::format::DateStyle` (G-36).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "analytics/types/gen/")]
pub enum DateStyle {
    Dmy,
    Ymd,
}

impl From<DateStyle> for crate::utils::format::DateStyle {
    fn from(value: DateStyle) -> Self {
        match value {
            DateStyle::Dmy => crate::utils::format::DateStyle::Dmy,
            DateStyle::Ymd => crate::utils::format::DateStyle::Ymd,
        }
    }
}

// --- المبيعات (sales) -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "analytics/types/gen/")]
pub struct SalesTrendPoint {
    pub date: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "analytics/types/gen/")]
pub struct WeekdayRow {
    pub day: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub avg: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "analytics/types/gen/")]
pub struct PaymentMixRow {
    pub method: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub pct: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "analytics/types/gen/")]
pub struct SalesAnalytics {
    pub trend: Vec<SalesTrendPoint>,
    pub trend_insight: String,
    pub by_weekday: Vec<WeekdayRow>,
    pub weekday_insight: String,
    pub payment_mix: Vec<PaymentMixRow>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub avg_invoice: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub avg_items_per_invoice: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub returns_rate_pct: Decimal,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "analytics/types/gen/")]
pub struct AnalyticsGetSalesAnalyticsArgs {
    #[ts(optional)]
    pub days: Option<i32>,
    pub date_style: DateStyle,
    pub numerals: Numerals,
}

// --- المنتجات (products) --------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "analytics/types/gen/")]
pub struct ProductProfitRow {
    /// A product id, or `freetext-<position>` for a free-text line (13b §3.0 line-key convention).
    pub id: String,
    pub name: String,
    pub sku: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub revenue: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub gross_profit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub margin_pct: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "analytics/types/gen/")]
pub struct ProductAnalytics {
    pub top: Vec<ProductProfitRow>,
    pub bottom: Vec<ProductProfitRow>,
    pub insight: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "analytics/types/gen/")]
pub struct AnalyticsGetProductAnalyticsArgs {
    #[ts(optional)]
    pub days: Option<i32>,
    #[ts(optional)]
    pub limit: Option<i32>,
}

// --- العملاء (customers) --------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "analytics/types/gen/")]
pub struct CustomerShare {
    #[ts(type = "string")]
    pub id: crate::utils::id::Id,
    pub name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "analytics/types/gen/")]
pub struct CustomerAnalytics {
    pub new_count: i32,
    pub returning_count: i32,
    pub segment_insight: String,
    pub top_customers: Vec<CustomerShare>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub top10_share_pct: Decimal,
    pub concentration_insight: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "analytics/types/gen/")]
pub struct AnalyticsGetCustomerAnalyticsArgs {
    #[ts(optional)]
    pub days: Option<i32>,
    #[ts(optional)]
    pub limit: Option<i32>,
}
