//! `domains::setup::dto` (02-setup.md §2). Serde camelCase = the TS types; every DTO here mirrors
//! `src/modules/setup/types/index.ts` field-for-field (moved there from the mock per §2's "TS
//! moves first"). Reused enums (`AccountTemplate`, `PaymentMethodType`, `PaymentMethodAccountRole`)
//! come from `domains::settings::dto` — never redeclared here.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::domains::settings::dto::{PaymentMethodAccountRole, PaymentMethodType};
use crate::entities::values as v;
use crate::utils::id::Id;
use crate::utils::money::serde_number;

/// `number` — the wire shape of `getOpeningBalanceEquityNet`'s return. `rust_decimal::Decimal` has
/// no `ts-rs` impl in this crate (every other DTO field carrying money uses `#[ts(type = "number")]`
/// on a struct field instead), and this command's entire return value is a bare decimal with no
/// struct to hang that attribute on — same manual `TS` impl shape as `parties::dto::OptionalMoney`
/// and `settings::dto::OrderedNumberMap`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Money(pub Decimal);

impl Serialize for Money {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_number::serialize(&self.0, serializer)
    }
}

impl TS for Money {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name(_cfg: &ts_rs::Config) -> String {
        "number".to_string()
    }
    fn inline(_cfg: &ts_rs::Config) -> String {
        "number".to_string()
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

// --- Onboarding progress (setupService.ts:52-59) ----------------------------------------------------

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct OnboardingProgress {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_type: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub go_live_date: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_step: Option<i32>,
    pub skipped: Vec<String>,
    pub done: Vec<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening_entry_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closing_entry_id: Option<Id>,
    #[ts(optional, type = "import('@/mocks/fixtures/accounts').AccountTemplate")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coa_template: Option<String>,
}

/// `Partial<OnboardingProgress>` — every key optional; a key absent means "unchanged".
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct OnboardingProgressPatch {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_type: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub go_live_date: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_step: Option<i32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<Vec<String>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done: Option<Vec<String>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening_entry_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closing_entry_id: Option<Id>,
    #[ts(optional, type = "import('@/mocks/fixtures/accounts').AccountTemplate")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coa_template: Option<String>,
}

// --- Structure steps (setup.ts) ----------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct CountryTaxInput {
    #[ts(type = "import('@/modules/core/helpers/countryProfiles').CountryCode")]
    pub country: String,
    pub currency: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat_registered: Option<bool>,
    pub prices_include_tax: bool,
    pub extra_currencies: Vec<ExtraCurrency>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct ExtraCurrency {
    pub code: String,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub rate: Decimal,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct WizardBranchInput {
    pub name: String,
    pub code: String,
    #[ts(optional, type = "import('@/modules/core/types/address').Address")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<v::Address>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct WizardPaymentMethodInput {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: PaymentMethodType,
    pub account_role: PaymentMethodAccountRole,
    pub active: bool,
}

// --- Opening (opening.ts:26-258) ---------------------------------------------------------------------

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct OpeningCashLine {
    #[ts(type = "string")]
    pub account_id: Id,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub amount: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_fc: Option<Decimal>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<Decimal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "setup/types/gen/")]
pub enum PostingSide {
    Debit,
    Credit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub enum PartyKindWire {
    Customer,
    Supplier,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct OpeningPartyLine {
    pub party_kind: PartyKindWire,
    #[ts(type = "string")]
    pub party_id: Id,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub amount: Decimal,
    pub side: PostingSide,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct OpeningOtherLine {
    #[ts(type = "string")]
    pub account_id: Id,
    pub side: PostingSide,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub amount: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// `Omit<OpeningEntryInput, 'createdBy'>` (setup.md §2) — the wire input has no `createdBy` field;
/// the actor comes from `TxCtx`.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct OpeningEntryInput {
    pub date: String,
    pub cash: Vec<OpeningCashLine>,
    pub customers: Vec<OpeningPartyLine>,
    pub suppliers: Vec<OpeningPartyLine>,
    pub other: Vec<OpeningOtherLine>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub enum CloseTarget {
    Capital,
    OwnerCurrent,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct OpeningStockLine {
    #[ts(type = "string")]
    pub product_id: Id,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub qty: Decimal,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub unit_cost: Decimal,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_no: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct PostOpeningBalancesResult {
    #[ts(type = "string")]
    pub opening_entry_id: Id,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closing_entry_id: Option<Id>,
}

/// `Omit<PartyOpeningBalanceInput, 'createdBy'>` (setup.md §2).
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct PartyOpeningInput {
    pub party_kind: PartyKindWire,
    #[ts(type = "string")]
    pub party_id: Id,
    #[ts(type = "number")]
    #[serde(with = "serde_number")]
    pub amount: Decimal,
    pub side: PostingSide,
    /// Never tz-shifted (setup.md §2) — a plain business day.
    pub as_of_date: String,
}

// --- Device setup (Part 02 handoff §9) ----------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "setup/types/gen/")]
pub enum DeviceRoleWire {
    Main,
    Terminal,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct DeviceSetupState {
    pub configured: bool,
    pub role: DeviceRoleWire,
    pub can_host_database: bool,
    pub has_users: bool,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct PairTerminalInput {
    pub host: String,
    #[ts(type = "number")]
    pub port: u16,
    pub code: String,
}
