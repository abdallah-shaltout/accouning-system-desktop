//! Accounts CRUD DTOs beyond the `Account`/`AccountKind`/`AccountSubtype`/`NormalSide` already
//! declared in `dto/mod.rs` (12-accounting.md §2). `AccountWithBalance` mirrors
//! `accountingService.ts:26`'s flat type; command args follow the one-struct-per-command
//! convention (§3.2 of 11-expenses.md, reused here).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::utils::id::Id;
use crate::utils::money::serde_number;

use super::{Account, AccountInput};

/// `AccountWithBalance` (`accountingService.ts:26`): every `Account` field + `balance`,
/// `debitTotal`, `creditTotal`, `hasPostings`. Written out flat (not `#[serde(flatten)]`) so the
/// generated TS type is a plain object `Equals` can match against the intersection type.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountWithBalance {
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
    pub kind: super::AccountKind,
    pub subtype: super::AccountSubtype,
    pub normal_side: super::NormalSide,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_role: Option<crate::shared::ledger::accounts::SystemRole>,
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
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub balance: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub debit_total: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub credit_total: Decimal,
    pub has_postings: bool,
}

impl AccountWithBalance {
    pub fn from_account(a: Account, debit_total: Decimal, credit_total: Decimal, balance: Decimal, has_postings: bool) -> Self {
        AccountWithBalance {
            id: a.id,
            code: a.code,
            name: a.name,
            name_en: a.name_en,
            parent_id: a.parent_id,
            is_group: a.is_group,
            kind: a.kind,
            subtype: a.subtype,
            normal_side: a.normal_side,
            system_role: a.system_role,
            currency: a.currency,
            branch_id: a.branch_id,
            requires_party: a.requires_party,
            allow_manual: a.allow_manual,
            requires_cost_center: a.requires_cost_center,
            active: a.active,
            can_delete: a.can_delete,
            balance,
            debit_total,
            credit_total,
            has_postings,
        }
    }
}

/// `DateRange` (inline `{ from?: string; to?: string }`, `accountingService.ts:40`).
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct DateRange {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

// --- Command args ---------------------------------------------------------------------------

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingGetAccountsArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<DateRange>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingSaveAccountArgs {
    pub input: AccountInput,
    #[serde(default)]
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingDeleteAccountArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingReparentAccountArgs {
    #[ts(type = "string")]
    pub id: Id,
    #[ts(type = "string | null")]
    pub new_parent_id: Option<Id>,
}
