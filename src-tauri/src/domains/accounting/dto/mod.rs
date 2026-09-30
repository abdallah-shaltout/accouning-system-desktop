//! `domains::accounting::dto` (H-3 + 12/12b-accounting): `FiscalYear` and `Account` were created
//! here in Part 03 W2 (02-setup) because `setup_apply_fiscal_year`/`setup_apply_coa_template`
//! return them before the real accounting domain (W5) existed — kept in this file verbatim (same
//! names/types) since `domains::setup` already imports them from here. Everything else (accounts
//! CRUD DTOs beyond the two above, journal/drafts/templates, fiscal-year period-close, VAT) lives
//! in the sibling `accounts.rs`/`journal.rs`/`templates.rs`/`period.rs` and is re-exported flat
//! from this module so callers keep writing `dto::Whatever`.
//! `src/modules/accounting/types/contract.check.ts` checks the wire shapes against
//! `src/modules/accounting/types/index.ts`'s hand-written types.

pub mod accounts;
pub mod journal;
pub mod period;
pub mod templates;

pub use accounts::*;
pub use journal::*;
pub use period::*;
pub use templates::*;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::entities::org::accounts as accounts_entity;
use crate::entities::org::fiscal_years;
use crate::shared::ledger::accounts::SystemRole;
use crate::utils::dates::format_iso_ms;
use crate::utils::id::Id;

// --- FiscalYear (accounting/types/index.ts:187-196) ------------------------------------------------

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct FiscalYear {
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    pub start_date: String,
    pub end_date: String,
    pub is_closed: bool,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closing_entry_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed_by: Option<Id>,
}

impl FiscalYear {
    pub fn from_model(m: fiscal_years::Model) -> Self {
        FiscalYear {
            id: m.id,
            name: m.name,
            start_date: m.start_date.format("%Y-%m-%d").to_string(),
            end_date: m.end_date.format("%Y-%m-%d").to_string(),
            is_closed: m.is_closed,
            closing_entry_id: m.closing_entry_id,
            closed_at: m.closed_at.map(format_iso_ms),
            closed_by: m.closed_by,
        }
    }
}

// --- Account (accounting/types/index.ts:31-63) ------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
#[ts(export_to = "accounting/types/gen/")]
pub enum AccountKind {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

impl AccountKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AccountKind::Asset => "ASSET",
            AccountKind::Liability => "LIABILITY",
            AccountKind::Equity => "EQUITY",
            AccountKind::Revenue => "REVENUE",
            AccountKind::Expense => "EXPENSE",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "ASSET" => Some(AccountKind::Asset),
            "LIABILITY" => Some(AccountKind::Liability),
            "EQUITY" => Some(AccountKind::Equity),
            "REVENUE" => Some(AccountKind::Revenue),
            "EXPENSE" => Some(AccountKind::Expense),
            _ => None,
        }
    }

    /// `NORMAL_SIDE` (`fixtures/accounts.ts:31-37`).
    pub fn normal_side(self) -> NormalSide {
        match self {
            AccountKind::Asset | AccountKind::Expense => NormalSide::Debit,
            AccountKind::Liability | AccountKind::Equity | AccountKind::Revenue => NormalSide::Credit,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub enum AccountSubtype {
    Cash,
    Bank,
    Clearing,
    Receivable,
    Payable,
    Inventory,
    Tax,
    Prepaid,
    OtherCurrentAsset,
    FixedAsset,
    AccumulatedDepreciation,
    CurrentLiability,
    LongTermLiability,
    Equity,
    Revenue,
    OtherIncome,
    CostOfSales,
    OperatingExpense,
    OtherExpense,
    ZakatTax,
}

impl AccountSubtype {
    pub fn as_str(self) -> &'static str {
        match self {
            AccountSubtype::Cash => "cash",
            AccountSubtype::Bank => "bank",
            AccountSubtype::Clearing => "clearing",
            AccountSubtype::Receivable => "receivable",
            AccountSubtype::Payable => "payable",
            AccountSubtype::Inventory => "inventory",
            AccountSubtype::Tax => "tax",
            AccountSubtype::Prepaid => "prepaid",
            AccountSubtype::OtherCurrentAsset => "otherCurrentAsset",
            AccountSubtype::FixedAsset => "fixedAsset",
            AccountSubtype::AccumulatedDepreciation => "accumulatedDepreciation",
            AccountSubtype::CurrentLiability => "currentLiability",
            AccountSubtype::LongTermLiability => "longTermLiability",
            AccountSubtype::Equity => "equity",
            AccountSubtype::Revenue => "revenue",
            AccountSubtype::OtherIncome => "otherIncome",
            AccountSubtype::CostOfSales => "costOfSales",
            AccountSubtype::OperatingExpense => "operatingExpense",
            AccountSubtype::OtherExpense => "otherExpense",
            AccountSubtype::ZakatTax => "zakatTax",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "cash" => Some(AccountSubtype::Cash),
            "bank" => Some(AccountSubtype::Bank),
            "clearing" => Some(AccountSubtype::Clearing),
            "receivable" => Some(AccountSubtype::Receivable),
            "payable" => Some(AccountSubtype::Payable),
            "inventory" => Some(AccountSubtype::Inventory),
            "tax" => Some(AccountSubtype::Tax),
            "prepaid" => Some(AccountSubtype::Prepaid),
            "otherCurrentAsset" => Some(AccountSubtype::OtherCurrentAsset),
            "fixedAsset" => Some(AccountSubtype::FixedAsset),
            "accumulatedDepreciation" => Some(AccountSubtype::AccumulatedDepreciation),
            "currentLiability" => Some(AccountSubtype::CurrentLiability),
            "longTermLiability" => Some(AccountSubtype::LongTermLiability),
            "equity" => Some(AccountSubtype::Equity),
            "revenue" => Some(AccountSubtype::Revenue),
            "otherIncome" => Some(AccountSubtype::OtherIncome),
            "costOfSales" => Some(AccountSubtype::CostOfSales),
            "operatingExpense" => Some(AccountSubtype::OperatingExpense),
            "otherExpense" => Some(AccountSubtype::OtherExpense),
            "zakatTax" => Some(AccountSubtype::ZakatTax),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
#[ts(export_to = "accounting/types/gen/")]
pub enum NormalSide {
    Debit,
    Credit,
}

impl NormalSide {
    pub fn as_str(self) -> &'static str {
        match self {
            NormalSide::Debit => "DEBIT",
            NormalSide::Credit => "CREDIT",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "DEBIT" => Some(NormalSide::Debit),
            "CREDIT" => Some(NormalSide::Credit),
            _ => None,
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct Account {
    #[ts(type = "string")]
    pub id: Id,
    pub code: String,
    pub name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_en: Option<String>,
    #[ts(type = "string | null")]
    #[serialize_always] // `null` for a root account, as the TS type says (not absent)
    pub parent_id: Option<Id>,
    pub is_group: bool,
    pub kind: AccountKind,
    pub subtype: AccountSubtype,
    pub normal_side: NormalSide,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_role: Option<SystemRole>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_party: Option<bool>,
    pub allow_manual: bool,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_cost_center: Option<bool>,
    pub active: bool,
    pub can_delete: bool,
}

impl Account {
    pub fn from_model(m: accounts_entity::Model) -> Self {
        Account {
            id: m.id,
            code: m.code,
            name: m.name,
            name_en: m.name_en,
            parent_id: m.parent_id,
            is_group: m.is_group,
            kind: AccountKind::from_str_opt(&m.kind).unwrap_or(AccountKind::Asset),
            subtype: AccountSubtype::from_str_opt(&m.subtype).unwrap_or(AccountSubtype::OtherCurrentAsset),
            normal_side: NormalSide::from_str_opt(&m.normal_side).unwrap_or(NormalSide::Debit),
            system_role: m.system_role.as_deref().and_then(|s| s.parse().ok()),
            currency: m.currency,
            branch_id: m.branch_id,
            requires_party: m.requires_party,
            allow_manual: m.allow_manual,
            requires_cost_center: m.requires_cost_center,
            active: m.active,
            can_delete: m.can_delete,
        }
    }
}

/// `AccountInput` (`types/index.ts:65-77`). `parent_id` is optional on the wire (absent -> the
/// mock's `input.parentId || null`, A-D2/§3.2 step 3). `name_en`/`requires_party` are the two
/// fields an update may omit to keep the stored value (A-D2) — `save_account` only writes them when
/// `Some`.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountInput {
    pub code: String,
    pub name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_en: Option<String>,
    #[serde(default)]
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Id>,
    pub is_group: bool,
    pub kind: AccountKind,
    pub subtype: AccountSubtype,
    pub normal_side: NormalSide,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_party: Option<bool>,
    pub allow_manual: bool,
    pub active: bool,
}
