//! `domains::settings` DTOs — serde camelCase = the TS types, ts-rs bindings exported to
//! `settings/types/gen/` (01-settings.md §2). Every DTO here mirrors the mock's
//! `src/modules/settings/types/{index,dimensions}.ts` shape field-for-field (minus `theme`, C-14).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::entities::org::{branches, cost_centers, currencies, exchange_rates, payment_methods, taxes};
use crate::entities::values as v;
use crate::utils::dates::format_iso_ms;
use crate::utils::id::Id;
use crate::utils::money::serde_number;

// --- Store settings (StoreSettings, index.ts:97-218, minus `theme` — C-14) ----------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "settings/types/gen/")]
pub enum PrinterMode {
    A4,
    Thermal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "settings/types/gen/")]
pub enum PrinterConnectionType {
    Windows,
    Network,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct ThermalPrinterSettings {
    #[ts(optional)]
    pub printer_name: Option<String>,
    pub connection: PrinterConnectionType,
    #[ts(optional)]
    pub host: Option<String>,
    pub dpi: i32,
    pub cut: bool,
    pub open_drawer: bool,
    pub copies: i32,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct PrinterSettings {
    pub mode: PrinterMode,
    #[ts(type = "58 | 80")]
    pub thermal_width_mm: i32,
    #[ts(optional)]
    pub thermal: Option<ThermalPrinterSettings>,
    #[ts(optional)]
    pub a4_printer_name: Option<String>,
    #[ts(optional)]
    pub label_printer_name: Option<String>,
    /// Plan 22 union, not enumerated in Rust — stored/transmitted as a plain string.
    #[ts(optional, type = "import('@/modules/invoices/helpers/invoiceTemplates').A4TemplateId")]
    pub a4_template: Option<String>,
    #[ts(optional, type = "import('@/modules/invoices/helpers/invoiceTemplates').ImageTemplateId")]
    pub image_template: Option<String>,
}

/// A patch to `PrinterSettings` — every key optional; a device key (`thermal`, `a4PrinterName`,
/// `labelPrinterName`) present here is routed to the local device file, never the branch row
/// (01-settings.md §3 step 4).
#[skip_serializing_none]
#[derive(Debug, Clone, Default, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct PrinterPatch {
    #[ts(optional)]
    pub mode: Option<PrinterMode>,
    #[ts(optional, type = "58 | 80")]
    pub thermal_width_mm: Option<i32>,
    #[ts(optional)]
    pub thermal: Option<ThermalPrinterSettings>,
    #[ts(optional)]
    pub a4_printer_name: Option<String>,
    #[ts(optional)]
    pub label_printer_name: Option<String>,
    #[ts(optional, type = "import('@/modules/invoices/helpers/invoiceTemplates').A4TemplateId")]
    pub a4_template: Option<String>,
    #[ts(optional, type = "import('@/modules/invoices/helpers/invoiceTemplates').ImageTemplateId")]
    pub image_template: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct AccountingPolicy {
    #[ts(optional)]
    pub lock_date: Option<String>,
    #[ts(optional, type = "string")]
    pub default_purchase_account_id: Option<Id>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct PosPolicy {
    #[ts(optional)]
    pub override_price: Option<bool>,
    #[ts(optional)]
    pub sell_below_cost: Option<bool>,
    #[ts(optional)]
    pub require_open_shift: Option<bool>,
    #[ts(optional)]
    pub foreign_cash_enabled: Option<bool>,
    #[ts(optional)]
    pub foreign_currency: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub foreign_currency_rate: Option<Decimal>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SalesPolicy {
    #[ts(optional)]
    pub refund_without_receipt: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct FeatureFlags {
    #[ts(optional)]
    pub branches: Option<bool>,
    #[ts(optional)]
    pub currencies: Option<bool>,
    #[ts(optional)]
    pub cost_centers: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct OnboardingState {
    #[ts(optional)]
    pub business_type: Option<String>,
    #[ts(optional)]
    pub go_live_date: Option<String>,
    #[ts(optional)]
    pub completed_step: Option<i32>,
    #[serde(default)]
    #[ts(optional)]
    pub skipped: Option<Vec<String>>,
    #[serde(default)]
    #[ts(optional)]
    pub done: Option<Vec<String>>,
    #[ts(optional)]
    pub finished_at: Option<String>,
    #[ts(optional, type = "string")]
    pub opening_entry_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub closing_entry_id: Option<Id>,
    #[ts(optional, type = "'basic' | 'standard' | 'detailed'")]
    pub coa_template: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct StoreSettings {
    pub store_name: String,
    #[ts(optional)]
    pub logo: Option<String>,
    #[ts(optional)]
    pub stamp: Option<String>,
    #[ts(optional)]
    pub signature: Option<String>,
    pub currency: String,
    #[ts(optional, type = "import('@/modules/core/helpers/countryProfiles').CountryCode")]
    pub country: Option<String>,
    #[ts(optional)]
    pub vat_number: Option<String>,
    #[ts(optional, type = "string")]
    pub default_tax_id: Option<Id>,
    pub invoice_number_prefix: String,
    pub printer: PrinterSettings,
    #[ts(optional)]
    pub prices_include_tax: Option<bool>,
    #[ts(optional)]
    pub address: Option<String>,
    #[ts(optional, type = "import('@/modules/core/types/address').Address")]
    pub national_address: Option<v::Address>,
    #[ts(optional)]
    pub phone: Option<String>,
    #[ts(optional)]
    pub commercial_register: Option<String>,
    #[ts(optional)]
    pub receipt_footer: Option<String>,
    #[ts(optional)]
    pub accounting: Option<AccountingPolicy>,
    #[ts(optional, type = "import('./BackupSettings').BackupSettings")]
    pub backup: Option<serde_json::Value>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub inventory_approval_threshold: Option<Decimal>,
    #[ts(optional, type = "import('@/modules/users/helpers/permissions').RoleAccessOverrides")]
    pub role_access_overrides: Option<v::RoleAccessOverrides>,
    #[ts(optional, type = "Partial<import('@/modules/core/services/insightTypes').InsightThresholds>")]
    pub insight_thresholds: Option<BTreeMap<String, crate::entities::values::JsonDecimal>>,
    #[ts(optional)]
    pub pos: Option<PosPolicy>,
    #[ts(optional)]
    pub sales: Option<SalesPolicy>,
    #[ts(optional)]
    pub features: Option<FeatureFlags>,
    #[ts(optional)]
    pub onboarding: Option<OnboardingState>,
}

/// `Partial<StoreSettings>` — every top-level key optional; a key absent from the JSON object is
/// "unchanged" (never sent), matching `updateSettings(patch: Partial<StoreSettings>)` (01-settings.md §2).
#[skip_serializing_none]
#[derive(Debug, Clone, Default, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct StoreSettingsPatch {
    #[ts(optional)]
    pub store_name: Option<String>,
    #[ts(optional)]
    pub logo: Option<String>,
    #[ts(optional)]
    pub stamp: Option<String>,
    #[ts(optional)]
    pub signature: Option<String>,
    #[ts(optional)]
    pub currency: Option<String>,
    #[ts(optional, type = "import('@/modules/core/helpers/countryProfiles').CountryCode")]
    pub country: Option<String>,
    #[ts(optional)]
    pub vat_number: Option<String>,
    #[ts(optional, type = "string")]
    pub default_tax_id: Option<Id>,
    #[ts(optional)]
    pub invoice_number_prefix: Option<String>,
    #[ts(optional)]
    pub printer: Option<PrinterPatch>,
    #[ts(optional)]
    pub prices_include_tax: Option<bool>,
    #[ts(optional)]
    pub address: Option<String>,
    #[ts(optional, type = "import('@/modules/core/types/address').Address")]
    pub national_address: Option<v::Address>,
    #[ts(optional)]
    pub phone: Option<String>,
    #[ts(optional)]
    pub commercial_register: Option<String>,
    #[ts(optional)]
    pub receipt_footer: Option<String>,
    #[ts(optional)]
    pub accounting: Option<AccountingPolicy>,
    #[ts(optional, type = "import('./BackupSettings').BackupSettings")]
    pub backup: Option<serde_json::Value>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub inventory_approval_threshold: Option<Decimal>,
    #[ts(optional, type = "import('@/modules/users/helpers/permissions').RoleAccessOverrides")]
    pub role_access_overrides: Option<v::RoleAccessOverrides>,
    #[ts(optional, type = "Partial<import('@/modules/core/services/insightTypes').InsightThresholds>")]
    pub insight_thresholds: Option<BTreeMap<String, crate::entities::values::JsonDecimal>>,
    #[ts(optional)]
    pub pos: Option<PosPolicy>,
    #[ts(optional)]
    pub sales: Option<SalesPolicy>,
    #[ts(optional)]
    pub features: Option<FeatureFlags>,
    #[ts(optional)]
    pub onboarding: Option<OnboardingState>,
}

// --- Taxes (Tax, index.ts:21-37) -----------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
#[ts(export_to = "settings/types/gen/")]
pub enum TaxCategory {
    S,
    Z,
    E,
    O,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
#[ts(export_to = "settings/types/gen/")]
pub enum TaxLegacyType {
    Output,
    Input,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub enum TaxDirection {
    Sales,
    Purchase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub enum TaxAccountRole {
    VatOutput,
    VatInput,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct Tax {
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub rate: Decimal,
    #[serde(rename = "type")]
    pub kind: TaxLegacyType,
    pub is_default: bool,
    pub active: bool,
    pub category: TaxCategory,
    pub direction: TaxDirection,
    #[ts(optional)]
    pub exemption_reason: Option<String>,
    #[ts(optional)]
    pub account_role: Option<TaxAccountRole>,
}

impl Tax {
    pub fn from_model(m: taxes::Model) -> Self {
        Tax {
            id: m.id,
            name: m.name,
            rate: m.rate,
            kind: if m.r#type == "OUTPUT" { TaxLegacyType::Output } else { TaxLegacyType::Input },
            is_default: m.is_default,
            active: m.active,
            category: match m.category.as_str() {
                "S" => TaxCategory::S,
                "Z" => TaxCategory::Z,
                "E" => TaxCategory::E,
                _ => TaxCategory::O,
            },
            direction: if m.direction == "sales" { TaxDirection::Sales } else { TaxDirection::Purchase },
            exemption_reason: m.exemption_reason,
            account_role: match m.account_role.as_deref() {
                Some("vatOutput") => Some(TaxAccountRole::VatOutput),
                Some("vatInput") => Some(TaxAccountRole::VatInput),
                _ => None,
            },
        }
    }
}

/// `Omit<Tax, 'id'>` — `direction`/`accountRole` are accepted on the wire (so the request type
/// matches the TS shape) but ignored by the service, which always re-derives them from `type`
/// (01-settings.md §2 "server-derived, never client-set").
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct TaxInput {
    pub name: String,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub rate: Decimal,
    #[serde(rename = "type")]
    pub kind: TaxLegacyType,
    pub is_default: bool,
    pub active: bool,
    pub category: TaxCategory,
    pub direction: TaxDirection,
    #[ts(optional)]
    pub exemption_reason: Option<String>,
    #[ts(optional)]
    pub account_role: Option<TaxAccountRole>,
}

// --- Payment methods (PaymentMethod, index.ts:48-70) ----------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "settings/types/gen/")]
pub enum PaymentMethodType {
    Cash,
    Card,
    BankTransfer,
    Wallet,
    Credit,
    StoreCredit,
}

impl PaymentMethodType {
    pub fn as_str(self) -> &'static str {
        match self {
            PaymentMethodType::Cash => "cash",
            PaymentMethodType::Card => "card",
            PaymentMethodType::BankTransfer => "bank_transfer",
            PaymentMethodType::Wallet => "wallet",
            PaymentMethodType::Credit => "credit",
            PaymentMethodType::StoreCredit => "store_credit",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "cash" => Some(PaymentMethodType::Cash),
            "card" => Some(PaymentMethodType::Card),
            "bank_transfer" => Some(PaymentMethodType::BankTransfer),
            "wallet" => Some(PaymentMethodType::Wallet),
            "credit" => Some(PaymentMethodType::Credit),
            "store_credit" => Some(PaymentMethodType::StoreCredit),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub enum PaymentMethodAccountRole {
    Cash,
    Bank,
    CardClearing,
    WalletClearing,
    Receivable,
}

impl PaymentMethodAccountRole {
    pub fn as_str(self) -> &'static str {
        match self {
            PaymentMethodAccountRole::Cash => "cash",
            PaymentMethodAccountRole::Bank => "bank",
            PaymentMethodAccountRole::CardClearing => "cardClearing",
            PaymentMethodAccountRole::WalletClearing => "walletClearing",
            PaymentMethodAccountRole::Receivable => "receivable",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "cash" => Some(PaymentMethodAccountRole::Cash),
            "bank" => Some(PaymentMethodAccountRole::Bank),
            "cardClearing" => Some(PaymentMethodAccountRole::CardClearing),
            "walletClearing" => Some(PaymentMethodAccountRole::WalletClearing),
            "receivable" => Some(PaymentMethodAccountRole::Receivable),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct BranchAccountOverride {
    #[ts(type = "string")]
    pub branch_id: Id,
    #[ts(type = "string")]
    pub account_id: Id,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct PaymentMethod {
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: PaymentMethodType,
    #[ts(optional)]
    pub icon: Option<String>,
    pub account_role: PaymentMethodAccountRole,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub fee_pct: Decimal,
    #[ts(optional)]
    pub requires_reference: Option<bool>,
    pub show_in_pos: bool,
    pub show_in_payments: bool,
    #[ts(type = "number")]
    pub sort_order: i32,
    #[ts(optional)]
    pub branch_overrides: Option<Vec<BranchAccountOverride>>,
    pub active: bool,
    pub can_delete: bool,
}

impl PaymentMethod {
    pub fn from_model(m: payment_methods::Model) -> Self {
        PaymentMethod {
            id: m.id,
            name: m.name,
            kind: PaymentMethodType::from_str_opt(&m.r#type).unwrap_or(PaymentMethodType::Cash),
            icon: m.icon,
            account_role: PaymentMethodAccountRole::from_str_opt(&m.account_role).unwrap_or(PaymentMethodAccountRole::Cash),
            fee_pct: m.fee_pct,
            requires_reference: m.requires_reference,
            show_in_pos: m.show_in_pos,
            show_in_payments: m.show_in_payments,
            sort_order: m.sort_order as i32,
            branch_overrides: m
                .branch_overrides
                .map(|list| list.0.into_iter().map(|o| BranchAccountOverride { branch_id: o.branch_id, account_id: o.account_id }).collect()),
            active: m.active,
            can_delete: m.can_delete,
        }
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct PaymentMethodInput {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: PaymentMethodType,
    #[ts(optional)]
    pub icon: Option<String>,
    pub account_role: PaymentMethodAccountRole,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub fee_pct: Decimal,
    #[ts(optional)]
    pub requires_reference: Option<bool>,
    pub show_in_pos: bool,
    pub show_in_payments: bool,
    #[ts(type = "number")]
    pub sort_order: i32,
    #[ts(optional)]
    pub branch_overrides: Option<Vec<BranchAccountOverride>>,
    pub active: bool,
}

// --- Branches (Branch, dimensions.ts:13-37) --------------------------------------------------------

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct Branch {
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    pub code: String,
    #[ts(optional)]
    pub address: Option<String>,
    #[ts(optional, type = "import('@/modules/core/types/address').Address")]
    pub national_address: Option<v::Address>,
    #[ts(optional)]
    pub phone: Option<String>,
    #[ts(optional)]
    pub receipt_header: Option<String>,
    #[ts(optional, type = "string")]
    pub cash_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub bank_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub default_price_list_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub cost_center_id: Option<Id>,
    pub active: bool,
    pub can_delete: bool,
    #[ts(optional)]
    pub created_at: Option<String>,
}

impl Branch {
    pub fn from_model(m: branches::Model) -> Self {
        Branch {
            id: m.id,
            name: m.name,
            code: m.code,
            national_address: m.national_address,
            address: m.address,
            phone: m.phone,
            receipt_header: m.receipt_header,
            cash_account_id: m.cash_account_id,
            bank_account_id: m.bank_account_id,
            default_price_list_id: m.default_price_list_id,
            cost_center_id: m.cost_center_id,
            active: m.active,
            can_delete: m.can_delete,
            created_at: Some(format_iso_ms(m.created_at)),
        }
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct BranchInput {
    pub name: String,
    pub code: String,
    #[ts(optional)]
    pub address: Option<String>,
    #[ts(optional, type = "import('@/modules/core/types/address').Address")]
    pub national_address: Option<v::Address>,
    #[ts(optional)]
    pub phone: Option<String>,
    #[ts(optional)]
    pub receipt_header: Option<String>,
    #[ts(optional, type = "string")]
    pub bank_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub default_price_list_id: Option<Id>,
    pub active: bool,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct BranchPatch {
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub code: Option<String>,
    #[ts(optional)]
    pub address: Option<String>,
    #[ts(optional, type = "import('@/modules/core/types/address').Address")]
    pub national_address: Option<v::Address>,
    #[ts(optional)]
    pub phone: Option<String>,
    #[ts(optional)]
    pub receipt_header: Option<String>,
    #[ts(optional, type = "string")]
    pub bank_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub default_price_list_id: Option<Id>,
    #[ts(optional)]
    pub active: Option<bool>,
}

// --- Cost centers (CostCenter, dimensions.ts:43-65) -------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub enum CostCenterType {
    Branch,
    Department,
    Project,
    Other,
}

impl CostCenterType {
    pub fn as_str(self) -> &'static str {
        match self {
            CostCenterType::Branch => "branch",
            CostCenterType::Department => "department",
            CostCenterType::Project => "project",
            CostCenterType::Other => "other",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "branch" => Some(CostCenterType::Branch),
            "department" => Some(CostCenterType::Department),
            "project" => Some(CostCenterType::Project),
            "other" => Some(CostCenterType::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct CostCenterBudget {
    #[ts(type = "string")]
    pub fiscal_year_id: Id,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub amount: Decimal,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct CostCenter {
    #[ts(type = "string")]
    pub id: Id,
    pub code: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: CostCenterType,
    #[ts(optional, type = "string")]
    pub parent_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub manager_user_id: Option<Id>,
    pub active: bool,
    #[ts(optional)]
    pub budgets: Option<Vec<CostCenterBudget>>,
    pub can_delete: bool,
    #[ts(optional, type = "string")]
    pub branch_id: Option<Id>,
}

impl CostCenter {
    pub fn from_model_with_budgets(m: cost_centers::Model, budgets: Vec<crate::entities::org::cost_center_budgets::Model>) -> Self {
        CostCenter {
            id: m.id,
            code: m.code,
            name: m.name,
            kind: CostCenterType::from_str_opt(&m.kind).unwrap_or(CostCenterType::Other),
            parent_id: m.parent_id,
            manager_user_id: m.manager_user_id,
            active: m.active,
            budgets: if budgets.is_empty() {
                None
            } else {
                Some(budgets.into_iter().map(|b| CostCenterBudget { fiscal_year_id: b.fiscal_year_id, amount: b.amount }).collect())
            },
            can_delete: m.can_delete,
            branch_id: m.branch_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct CostCenterInput {
    pub name: String,
    pub code: String,
    #[serde(rename = "type")]
    pub kind: CostCenterType,
    #[ts(optional, type = "string")]
    pub parent_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub manager_user_id: Option<Id>,
    pub active: bool,
    #[ts(optional)]
    pub budgets: Option<Vec<CostCenterBudget>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct CostCenterPatch {
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub code: Option<String>,
    #[ts(optional)]
    pub kind: Option<CostCenterType>,
    #[ts(optional, type = "string")]
    pub parent_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub manager_user_id: Option<Id>,
    #[ts(optional)]
    pub active: Option<bool>,
    #[ts(optional)]
    pub budgets: Option<Vec<CostCenterBudget>>,
}

// --- Currencies / exchange rates (dimensions.ts:71-99) ----------------------------------------------

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct Currency {
    pub code: String,
    pub name_ar: String,
    pub symbol: String,
    #[ts(type = "number")]
    pub decimals: i32,
    pub active: bool,
    #[ts(optional)]
    pub fixed: Option<bool>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub fixed_rate: Option<Decimal>,
}

impl Currency {
    pub fn from_model(m: currencies::Model) -> Self {
        Currency { code: m.code, name_ar: m.name_ar, symbol: m.symbol, decimals: m.decimals, active: m.active, fixed: m.fixed, fixed_rate: m.fixed_rate }
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct CurrencyPatch {
    #[ts(optional)]
    pub name_ar: Option<String>,
    #[ts(optional)]
    pub symbol: Option<String>,
    #[ts(optional, type = "number")]
    pub decimals: Option<i32>,
    #[ts(optional)]
    pub active: Option<bool>,
    #[ts(optional)]
    pub fixed: Option<bool>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub fixed_rate: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct ExchangeRate {
    #[ts(type = "string")]
    pub id: Id,
    pub currency: String,
    pub date: String,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub rate: Decimal,
}

impl ExchangeRate {
    pub fn from_model(m: exchange_rates::Model) -> Self {
        ExchangeRate { id: m.id, currency: m.currency, date: m.date.format("%Y-%m-%d").to_string(), rate: m.rate }
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct ExchangeRateInput {
    pub currency: String,
    pub date: String,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub rate: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub inverse_rate: Option<Decimal>,
}

// --- Revaluation (revaluation.ts, moved to settings/types per 01-settings.md §2) -------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub enum FcBalanceKind {
    Customer,
    Supplier,
    Account,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct FcBalanceRow {
    pub kind: FcBalanceKind,
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    pub currency: String,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub fc_balance: Decimal,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub base_balance: Decimal,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub revalued_base: Decimal,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub gain_loss: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct RevaluationResult {
    #[ts(type = "string")]
    pub entry_id: Id,
    #[ts(type = "string")]
    pub reversal_entry_id: Id,
    pub rows: Vec<FcBalanceRow>,
}

/// `Record<string, number>` with currency insertion order preserved (`OrderedNumberMap`,
/// 01-settings.md §2) — ordered `Vec<(String, Decimal)>` wrapped so it serializes as a plain object.
#[derive(Debug, Clone, Default)]
pub struct OrderedNumberMap(pub Vec<(String, Decimal)>);

impl Serialize for OrderedNumberMap {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            // A JSON number with no float in between (rule 5).
            map.serialize_entry(k, &crate::entities::values::JsonDecimal(*v))?;
        }
        map.end()
    }
}

impl TS for OrderedNumberMap {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name(_cfg: &ts_rs::Config) -> String {
        "Record<string, number>".to_string()
    }
    fn inline(_cfg: &ts_rs::Config) -> String {
        "Record<string, number>".to_string()
    }
    fn inline_flattened(cfg: &ts_rs::Config) -> String {
        Self::inline(cfg)
    }
    fn decl(_cfg: &ts_rs::Config) -> String {
        String::new()
    }
    fn decl_concrete(_cfg: &ts_rs::Config) -> String {
        String::new()
    }
}

// --- Network / LAN sharing (settings/types/network.ts, new) ----------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct PairingInfo {
    pub host_name: String,
    pub addresses: Vec<String>,
    #[ts(type = "number")]
    pub port: u16,
    pub code: String,
}

#[cfg(windows)]
impl From<crate::infrastructure::database::pairing::PairingInfo> for PairingInfo {
    fn from(p: crate::infrastructure::database::pairing::PairingInfo) -> Self {
        PairingInfo { host_name: p.host_name, addresses: p.addresses, port: p.port, code: p.code }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "settings/types/gen/")]
pub enum LanRole {
    Main,
    Terminal,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct LanSharingStatus {
    pub role: LanRole,
    pub provisioned: bool,
    pub lan_sharing: bool,
    #[ts(type = "number")]
    pub connected_terminals: u32,
    #[ts(optional)]
    pub pairing: Option<PairingInfo>,
    #[ts(optional)]
    pub main_host: Option<String>,
}

// Note: `shared::ledger::accounts::SystemRole` (a cousin concept to `Tax.accountRole`/
// `PaymentMethod.accountRole` above, which use this domain's own narrower enums instead) is
// exported by `shared::ledger::accounts` itself and registered in `domains::export_bindings`
// (G-50) — not duplicated here.
