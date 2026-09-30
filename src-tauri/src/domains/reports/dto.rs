//! `reports` DTOs (03-domains/13-reports.md §2, 13b-reports-operational.md §2). Mirrors
//! `src/modules/reports/types/index.ts`. Every money/ratio field is `Decimal` via
//! `crate::utils::money::serde_number` + `#[ts(type = "number")]` (13 §2 conventions). `Id` fields
//! are `#[ts(type = "string")]`. Optional TS fields use `Option` + `#[skip_serializing_none]` +
//! `#[ts(optional)]`. Dates are the mock's key string (`DocDate::key()`), typed `String`. No report
//! is paged (13 R-1) — every command returns a whole array/object.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::BTreeMap;
use ts_rs::TS;

use crate::domains::settings::dto::TaxCategory;
use crate::utils::id::Id;
use crate::utils::money::serde_number;
use crate::utils::route::RouteRef;

// =================================================================================================
// Shared request shapes (13 §2)
// =================================================================================================

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct DateRangeInput {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct DimensionFilter {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_center_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
}

/// `ReportRangeFilter` = `DateRangeInput & DimensionFilter` (`#[serde(flatten)]`; TS is an
/// intersection type, checked via `Simplify<T>` in `contract.check.ts`, G-38).
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportRangeFilter {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_center_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
}

impl ReportRangeFilter {
    pub fn range(&self) -> DateRangeInput {
        DateRangeInput { from: self.from.clone(), to: self.to.clone() }
    }
    pub fn dim(&self) -> DimensionFilter {
        DimensionFilter { branch_id: self.branch_id.clone(), cost_center_id: self.cost_center_id.clone(), currency: self.currency.clone() }
    }
}

/// `'customer' | 'supplier'` — shared with the `parties` domain's own `PartyKind` (same wire shape;
/// kept as a local copy per-domain convention, matching `parties::dto::PartyKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "reports/types/gen/")]
pub enum PartyKindArg {
    Customer,
    Supplier,
}

// =================================================================================================
// Statements / ledgers (13 §2)
// =================================================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct TrialBalanceRow {
    #[ts(type = "string")]
    pub account_id: Id,
    pub code: String,
    pub name: String,
    pub group_name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub opening_balance: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub period_debit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub period_credit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub closing_debit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub closing_credit: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct StatementLine {
    #[ts(type = "string")]
    pub account_id: Id,
    pub code: String,
    pub name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ProfitAndLoss {
    pub revenue: Vec<StatementLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub net_revenue: Decimal,
    pub cogs: Vec<StatementLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total_cogs: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub gross_profit: Decimal,
    pub expenses: Vec<StatementLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total_expenses: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub net_income: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct PnlComparison {
    pub current: ProfitAndLoss,
    pub previous: ProfitAndLoss,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct CostCenterPnlColumn {
    /// Holds `'__unassigned__'` / `'__total__'` for the synthetic rows — not an `Id`.
    pub cost_center_id: String,
    pub name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub net_revenue: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total_cogs: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub gross_profit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total_expenses: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub net_income: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct CostCenterPnl {
    pub centers: Vec<CostCenterPnlColumn>,
    pub unassigned: CostCenterPnlColumn,
    pub total: CostCenterPnlColumn,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct CostCenterBudgetRow {
    #[ts(type = "string")]
    pub cost_center_id: Id,
    pub name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub budget: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub actual: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub variance_pct: Decimal,
    pub near_budget: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct BalanceSheet {
    pub as_of: String,
    pub assets: Vec<StatementLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total_assets: Decimal,
    pub liabilities: Vec<StatementLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total_liabilities: Decimal,
    pub equity: Vec<StatementLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub unclosed_earnings: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total_equity: Decimal,
    pub balanced: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
#[ts(export_to = "reports/types/gen/")]
pub enum NormalSide {
    Debit,
    Credit,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct LedgerRow {
    #[ts(type = "string")]
    pub id: Id,
    pub date: String,
    #[ts(type = "string")]
    pub entry_id: Id,
    pub entry_number: String,
    pub description: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub debit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub credit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub balance: Decimal,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct AccountLedger {
    pub title: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    pub normal_side: NormalSide,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub opening_balance: Decimal,
    pub rows: Vec<LedgerRow>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total_debit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total_credit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub closing_balance: Decimal,
    // contract-ok: AppRoute is imported, not generated
    #[ts(type = "Record<string, import('@/modules/core/types/route').AppRoute>")]
    pub row_links: BTreeMap<String, RouteRef>,
}

// =================================================================================================
// VAT (13 §2)
// =================================================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct VatBucket {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub taxable: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub vat: Decimal,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct VatCategoryBox {
    pub category: TaxCategory,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub rate: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub net: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub vat: Decimal,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct VatReport {
    pub sales: VatBucket,
    pub sales_returns: VatBucket,
    pub purchases: VatBucket,
    pub purchase_returns: VatBucket,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub output_vat: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub input_vat: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub net_payable: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub ledger_output: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub ledger_input: Decimal,
    pub sales_boxes: Vec<VatCategoryBox>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub enum VatDocKind {
    Sale,
    SalesReturn,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct VatDetailRow {
    pub id: String,
    pub date: String,
    pub document_number: String,
    pub document_kind: VatDocKind,
    pub product_name: String,
    pub category: TaxCategory,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub rate: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub net: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub vat: Decimal,
}

// =================================================================================================
// Cash flow (13 §2)
// =================================================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct CashFlowLine {
    pub label: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct CashFlowStatement {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub net_income: Decimal,
    pub operating_adjustments: Vec<CashFlowLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub operating_cash: Decimal,
    pub investing: Vec<CashFlowLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub investing_cash: Decimal,
    pub financing: Vec<CashFlowLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub financing_cash: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub net_change: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub opening_cash: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub closing_cash: Decimal,
}

// =================================================================================================
// Day book (13 §2)
// =================================================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct DayBookLine {
    pub account_code: String,
    pub account_name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub debit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub credit: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct DayBookEntry {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    pub date: String,
    pub description: String,
    pub lines: Vec<DayBookLine>,
}

// =================================================================================================
// Period comparison / business health (13 §2)
// =================================================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct PeriodComparisonLine {
    pub label: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub a: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub b: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub delta: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub delta_pct: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub enum HealthScoreKey {
    Liquidity,
    Profitability,
    Debt,
    Collection,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct BusinessHealthScore {
    pub key: HealthScoreKey,
    pub label: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub score: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub value: Decimal,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct BusinessHealthReport {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
    pub scores: Vec<BusinessHealthScore>,
}

// =================================================================================================
// Lookups (13 §2)
// =================================================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct LedgerTarget {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct LedgerTargets {
    pub accounts: Vec<LedgerTarget>,
    pub customers: Vec<LedgerTarget>,
    pub suppliers: Vec<LedgerTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct DimensionOption {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct CurrencyOption {
    pub code: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct DimensionOptions {
    pub branches: Vec<DimensionOption>,
    pub cost_centers: Vec<DimensionOption>,
    pub currencies: Vec<CurrencyOption>,
}

// =================================================================================================
// 13b — operational reports
// =================================================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct SalesReportSummary {
    pub invoice_count: i32,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub gross_sales: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub discounts: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub net_sales: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub vat: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub refunds: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub net_after_refunds: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cogs: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub gross_profit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub average_invoice: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct SalesByDay {
    pub date: String,
    pub invoices: i32,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct SalesByProduct {
    /// Holds `freetext-<position>` for free-text lines (13b §3.0) — not always a real `Id`.
    pub product_id: String,
    pub name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub revenue: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cost: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub profit: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct SalesByCategory {
    pub name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub revenue: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct SalesByMethod {
    pub method: String,
    pub count: i32,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct SalesByCashier {
    pub name: String,
    pub count: i32,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct SalesReport {
    pub summary: SalesReportSummary,
    pub by_day: Vec<SalesByDay>,
    pub by_product: Vec<SalesByProduct>,
    pub by_category: Vec<SalesByCategory>,
    pub by_method: Vec<SalesByMethod>,
    pub by_cashier: Vec<SalesByCashier>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "reports/types/gen/")]
pub enum StockStatus {
    Ok,
    Low,
    Out,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct InventoryReportRow {
    #[ts(type = "string")]
    pub product_id: Id,
    pub name: String,
    pub sku: String,
    pub category: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub min_stock: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cost_price: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub price: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cost_value: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub retail_value: Decimal,
    pub status: StockStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct DiscountReportRow {
    pub key: String,
    pub label: String,
    pub invoice_count: i32,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub list_value: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub charged_value: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub discount_value: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub discount_pct: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "reports/types/gen/")]
pub enum DiscountGroupBy {
    Cashier,
    Product,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "reports/types/gen/")]
pub enum GrossProfitGroupBy {
    Invoice,
    Product,
    Category,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct GrossProfitRow {
    pub key: String,
    pub label: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub revenue: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cost: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub profit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub margin_pct: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReturnsReportRow {
    pub key: String,
    pub label: String,
    pub count: i32,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReturnsReport {
    pub total_invoices: i32,
    pub total_refunds: i32,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub return_rate_pct: Decimal,
    pub by_reason: Vec<ReturnsReportRow>,
    pub by_product: Vec<ReturnsReportRow>,
    pub by_cashier: Vec<ReturnsReportRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ExpensesByCategory {
    #[ts(type = "string")]
    pub category_id: Id,
    pub name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ExpensesByMonth {
    /// `YYYY-MM`.
    pub month: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ExpensesReport {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
    pub by_category: Vec<ExpensesByCategory>,
    pub by_month: Vec<ExpensesByMonth>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ShiftReportRow {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    #[ts(type = "string")]
    pub terminal_id: Id,
    pub opened_at: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
    pub opened_by: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_by: Option<String>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub expected_cash: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub counted_cash: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub variance: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub sales_total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct LowStockRow {
    #[ts(type = "string")]
    pub product_id: Id,
    pub name: String,
    pub sku: String,
    pub category: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub min_stock: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub suggested_qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cost_value: Decimal,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct DeadStockRow {
    #[ts(type = "string")]
    pub product_id: Id,
    pub name: String,
    pub sku: String,
    pub category: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cost_value: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sale_date: Option<String>,
    /// Holds `9007199254740991` (`Number.MAX_SAFE_INTEGER`) for "never sold" (R-10) — exact as a
    /// JSON number.
    #[ts(type = "number")]
    pub days_since_sale: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct StocktakeVarianceRow {
    #[ts(type = "string")]
    pub count_id: Id,
    pub count_number: String,
    #[ts(type = "string")]
    pub product_id: Id,
    pub name: String,
    pub category: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub counted_qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub system_qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty_variance: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub value_variance: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct TransferReportRow {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    pub date: String,
    pub from_branch: String,
    pub to_branch: String,
    /// The transfer status enum's wire value (`'DRAFT' | 'SENT' | 'RECEIVED' | 'REJECTED'`).
    pub status: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub sent_qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub received_qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub shortage_qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub shortage_value: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct PurchasesReportSummary {
    pub po_count: i32,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub gross_purchases: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub vat: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub returns: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct PurchasesBySupplier {
    /// Falls back to the id text when the supplier's name is missing (13b §2).
    pub supplier_id: String,
    pub name: String,
    pub count: i32,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct PurchasesByProduct {
    pub product_id: String,
    pub name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub avg_price: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct PurchasesReport {
    pub summary: PurchasesReportSummary,
    pub by_supplier: Vec<PurchasesBySupplier>,
    pub by_product: Vec<PurchasesByProduct>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct BranchComparisonRow {
    #[ts(type = "string")]
    pub branch_id: Id,
    pub name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub sales: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub gross_profit: Decimal,
    pub invoice_count: i32,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub average_invoice: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ProfitLeakageReport {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub net_sales: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub discounts: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub returns: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub write_offs: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub shrinkage: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total_leakage: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub leakage_pct: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct AgingReportRow {
    #[ts(type = "string")]
    pub party_id: Id,
    pub name: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub current: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub b30: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub b60: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub b90plus: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub enum OverdueDocKind {
    Invoice,
    PurchaseOrder,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct OverdueRow {
    #[ts(type = "string")]
    pub id: Id,
    pub kind: OverdueDocKind,
    pub number: String,
    #[ts(type = "string")]
    pub party_id: Id,
    pub party_name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    pub date: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[ts(type = "number")]
    pub days_overdue: i64,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub outstanding: Decimal,
}

// =================================================================================================
// Command args (one struct per command, camelCase) — 13 §1 / 13b §1
// =================================================================================================

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetTrialBalanceArgs {
    pub range: ReportRangeFilter,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetProfitAndLossArgs {
    pub range: ReportRangeFilter,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetProfitAndLossComparisonArgs {
    pub range: ReportRangeFilter,
    pub compare_range: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetCostCenterProfitAndLossArgs {
    pub range: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetCostCenterBudgetVsActualArgs {
    pub fiscal_year_id: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetBalanceSheetArgs {
    pub as_of: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dim: Option<DimensionFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetAccountLedgerArgs {
    pub account_id: String,
    pub range: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetPartyLedgerArgs {
    pub kind: PartyKindArg,
    pub party_id: String,
    pub range: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetVatReportArgs {
    pub range: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetVatDetailArgs {
    pub range: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetCashFlowStatementArgs {
    pub range: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetDayBookArgs {
    pub range: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetPeriodComparisonArgs {
    pub range_a: DateRangeInput,
    pub range_b: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetBusinessHealthReportArgs {
    pub range: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetSalesReportArgs {
    pub range: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetDiscountsReportArgs {
    pub range: DateRangeInput,
    pub group_by: DiscountGroupBy,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetGrossProfitReportArgs {
    pub range: DateRangeInput,
    pub group_by: GrossProfitGroupBy,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetReturnsReportArgs {
    pub range: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetExpensesReportArgs {
    pub range: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetShiftsReportArgs {
    pub range: DateRangeInput,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetDeadStockReportArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<i32>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetStocktakeVariancesArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetTransfersReportArgs {
    pub range: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetPurchasesReportArgs {
    pub range: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetBranchComparisonArgs {
    pub range: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetProfitLeakageReportArgs {
    pub range: DateRangeInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetAgingReportArgs {
    pub kind: PartyKindArg,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "reports/types/gen/")]
pub struct ReportsGetOverdueReportArgs {
    pub kind: PartyKindArg,
}

/// G-8a: this domain's DTO exports — the manager calls this one line from `domains::export_bindings`
/// (`domains/mod.rs`) in the same commit that adds `pub mod reports;` there, per this checklist item
/// in `03-domains/13-reports.md` §9 / `13b-reports-operational.md` §9.
pub fn export_bindings(cfg: &ts_rs::Config) {
    use ts_rs::TS;
    DateRangeInput::export_all(cfg).expect("export DateRangeInput");
    DimensionFilter::export_all(cfg).expect("export DimensionFilter");
    ReportRangeFilter::export_all(cfg).expect("export ReportRangeFilter");
    PartyKindArg::export_all(cfg).expect("export PartyKindArg");
    TrialBalanceRow::export_all(cfg).expect("export TrialBalanceRow");
    StatementLine::export_all(cfg).expect("export StatementLine");
    ProfitAndLoss::export_all(cfg).expect("export ProfitAndLoss");
    PnlComparison::export_all(cfg).expect("export PnlComparison");
    CostCenterPnlColumn::export_all(cfg).expect("export CostCenterPnlColumn");
    CostCenterPnl::export_all(cfg).expect("export CostCenterPnl");
    CostCenterBudgetRow::export_all(cfg).expect("export CostCenterBudgetRow");
    BalanceSheet::export_all(cfg).expect("export BalanceSheet");
    NormalSide::export_all(cfg).expect("export NormalSide");
    LedgerRow::export_all(cfg).expect("export LedgerRow");
    AccountLedger::export_all(cfg).expect("export AccountLedger");
    VatBucket::export_all(cfg).expect("export VatBucket");
    VatCategoryBox::export_all(cfg).expect("export VatCategoryBox");
    VatReport::export_all(cfg).expect("export VatReport");
    VatDocKind::export_all(cfg).expect("export VatDocKind");
    VatDetailRow::export_all(cfg).expect("export VatDetailRow");
    CashFlowLine::export_all(cfg).expect("export CashFlowLine");
    CashFlowStatement::export_all(cfg).expect("export CashFlowStatement");
    DayBookLine::export_all(cfg).expect("export DayBookLine");
    DayBookEntry::export_all(cfg).expect("export DayBookEntry");
    PeriodComparisonLine::export_all(cfg).expect("export PeriodComparisonLine");
    HealthScoreKey::export_all(cfg).expect("export HealthScoreKey");
    BusinessHealthScore::export_all(cfg).expect("export BusinessHealthScore");
    BusinessHealthReport::export_all(cfg).expect("export BusinessHealthReport");
    LedgerTarget::export_all(cfg).expect("export LedgerTarget");
    LedgerTargets::export_all(cfg).expect("export LedgerTargets");
    DimensionOption::export_all(cfg).expect("export DimensionOption");
    CurrencyOption::export_all(cfg).expect("export CurrencyOption");
    DimensionOptions::export_all(cfg).expect("export DimensionOptions");

    SalesReportSummary::export_all(cfg).expect("export SalesReportSummary");
    SalesByDay::export_all(cfg).expect("export SalesByDay");
    SalesByProduct::export_all(cfg).expect("export SalesByProduct");
    SalesByCategory::export_all(cfg).expect("export SalesByCategory");
    SalesByMethod::export_all(cfg).expect("export SalesByMethod");
    SalesByCashier::export_all(cfg).expect("export SalesByCashier");
    SalesReport::export_all(cfg).expect("export SalesReport");
    StockStatus::export_all(cfg).expect("export StockStatus");
    InventoryReportRow::export_all(cfg).expect("export InventoryReportRow");
    DiscountReportRow::export_all(cfg).expect("export DiscountReportRow");
    DiscountGroupBy::export_all(cfg).expect("export DiscountGroupBy");
    GrossProfitGroupBy::export_all(cfg).expect("export GrossProfitGroupBy");
    GrossProfitRow::export_all(cfg).expect("export GrossProfitRow");
    ReturnsReportRow::export_all(cfg).expect("export ReturnsReportRow");
    ReturnsReport::export_all(cfg).expect("export ReturnsReport");
    ExpensesByCategory::export_all(cfg).expect("export ExpensesByCategory");
    ExpensesByMonth::export_all(cfg).expect("export ExpensesByMonth");
    ExpensesReport::export_all(cfg).expect("export ExpensesReport");
    ShiftReportRow::export_all(cfg).expect("export ShiftReportRow");
    LowStockRow::export_all(cfg).expect("export LowStockRow");
    DeadStockRow::export_all(cfg).expect("export DeadStockRow");
    StocktakeVarianceRow::export_all(cfg).expect("export StocktakeVarianceRow");
    TransferReportRow::export_all(cfg).expect("export TransferReportRow");
    PurchasesReportSummary::export_all(cfg).expect("export PurchasesReportSummary");
    PurchasesBySupplier::export_all(cfg).expect("export PurchasesBySupplier");
    PurchasesByProduct::export_all(cfg).expect("export PurchasesByProduct");
    PurchasesReport::export_all(cfg).expect("export PurchasesReport");
    BranchComparisonRow::export_all(cfg).expect("export BranchComparisonRow");
    ProfitLeakageReport::export_all(cfg).expect("export ProfitLeakageReport");
    AgingReportRow::export_all(cfg).expect("export AgingReportRow");
    OverdueDocKind::export_all(cfg).expect("export OverdueDocKind");
    OverdueRow::export_all(cfg).expect("export OverdueRow");

    ReportsGetTrialBalanceArgs::export_all(cfg).expect("export ReportsGetTrialBalanceArgs");
    ReportsGetProfitAndLossArgs::export_all(cfg).expect("export ReportsGetProfitAndLossArgs");
    ReportsGetProfitAndLossComparisonArgs::export_all(cfg).expect("export ReportsGetProfitAndLossComparisonArgs");
    ReportsGetCostCenterProfitAndLossArgs::export_all(cfg).expect("export ReportsGetCostCenterProfitAndLossArgs");
    ReportsGetCostCenterBudgetVsActualArgs::export_all(cfg).expect("export ReportsGetCostCenterBudgetVsActualArgs");
    ReportsGetBalanceSheetArgs::export_all(cfg).expect("export ReportsGetBalanceSheetArgs");
    ReportsGetAccountLedgerArgs::export_all(cfg).expect("export ReportsGetAccountLedgerArgs");
    ReportsGetPartyLedgerArgs::export_all(cfg).expect("export ReportsGetPartyLedgerArgs");
    ReportsGetVatReportArgs::export_all(cfg).expect("export ReportsGetVatReportArgs");
    ReportsGetVatDetailArgs::export_all(cfg).expect("export ReportsGetVatDetailArgs");
    ReportsGetCashFlowStatementArgs::export_all(cfg).expect("export ReportsGetCashFlowStatementArgs");
    ReportsGetDayBookArgs::export_all(cfg).expect("export ReportsGetDayBookArgs");
    ReportsGetPeriodComparisonArgs::export_all(cfg).expect("export ReportsGetPeriodComparisonArgs");
    ReportsGetBusinessHealthReportArgs::export_all(cfg).expect("export ReportsGetBusinessHealthReportArgs");
    ReportsGetSalesReportArgs::export_all(cfg).expect("export ReportsGetSalesReportArgs");
    ReportsGetDiscountsReportArgs::export_all(cfg).expect("export ReportsGetDiscountsReportArgs");
    ReportsGetGrossProfitReportArgs::export_all(cfg).expect("export ReportsGetGrossProfitReportArgs");
    ReportsGetReturnsReportArgs::export_all(cfg).expect("export ReportsGetReturnsReportArgs");
    ReportsGetExpensesReportArgs::export_all(cfg).expect("export ReportsGetExpensesReportArgs");
    ReportsGetShiftsReportArgs::export_all(cfg).expect("export ReportsGetShiftsReportArgs");
    ReportsGetDeadStockReportArgs::export_all(cfg).expect("export ReportsGetDeadStockReportArgs");
    ReportsGetStocktakeVariancesArgs::export_all(cfg).expect("export ReportsGetStocktakeVariancesArgs");
    ReportsGetTransfersReportArgs::export_all(cfg).expect("export ReportsGetTransfersReportArgs");
    ReportsGetPurchasesReportArgs::export_all(cfg).expect("export ReportsGetPurchasesReportArgs");
    ReportsGetBranchComparisonArgs::export_all(cfg).expect("export ReportsGetBranchComparisonArgs");
    ReportsGetProfitLeakageReportArgs::export_all(cfg).expect("export ReportsGetProfitLeakageReportArgs");
    ReportsGetAgingReportArgs::export_all(cfg).expect("export ReportsGetAgingReportArgs");
    ReportsGetOverdueReportArgs::export_all(cfg).expect("export ReportsGetOverdueReportArgs");
}
