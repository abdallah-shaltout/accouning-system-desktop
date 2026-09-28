//! Period-close DTOs (12b-period-close.md §2): fiscal-year input, closing-wizard pre-checks,
//! close/reopen results, VAT totals. `FiscalYear` itself stays in `dto/mod.rs` (H-3, setup depends
//! on it). `CloseYearPreCheck` moves here from `mocks/backend/core.ts:484` and `VatPeriodTotals`
//! from `mocks/backend/journal.ts:227` (type-only moves, zero behaviour change).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::utils::id::Id;
use crate::utils::money::serde_number;

use super::journal::JournalEntry;
use super::FiscalYear;

/// `FiscalYearInput` (`Omit<FiscalYear, 'id'>`, :343) — deserializes all seven fields;
/// `closingEntryId`/`closedAt`/`closedBy` are accepted and ignored (server-owned, P-D3).
#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct FiscalYearInput {
    pub name: String,
    pub start_date: String,
    pub end_date: String,
    pub is_closed: bool,
    #[serde(default)]
    #[ts(optional, type = "string")]
    pub closing_entry_id: Option<Id>,
    #[serde(default)]
    #[ts(optional)]
    pub closed_at: Option<String>,
    #[serde(default)]
    #[ts(optional, type = "string")]
    pub closed_by: Option<Id>,
}

/// `CloseYearPreCheckKey` (`core.ts:485`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub enum CloseYearPreCheckKey {
    Drafts,
    TrialBalance,
    OpeningEquity,
}

/// `CloseYearPreCheck` (`core.ts:484-489`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct CloseYearPreCheck {
    pub key: CloseYearPreCheckKey,
    pub label: String,
    pub passed: bool,
    pub detail: String,
}

/// `CloseYearResult` (inline return of `closeYear`, :385).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct CloseYearResult {
    pub fiscal_year: FiscalYear,
    pub closing_entry: JournalEntry,
    #[ts(optional)]
    pub next_year: Option<FiscalYear>,
}

/// `VatPeriodTotals` (`journal.ts:227-231`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct VatPeriodTotals {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub output_vat: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub input_vat: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub net: Decimal,
}

// --- Command args ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingSaveFiscalYearArgs {
    pub input: FiscalYearInput,
    #[serde(default)]
    #[ts(optional, type = "string")]
    pub id: Option<Id>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingSaveLockDateArgs {
    #[serde(default)]
    #[ts(optional)]
    pub lock_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingGetCloseYearPreChecksArgs {
    #[ts(type = "string")]
    pub fiscal_year_id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingCloseYearArgs {
    #[ts(type = "string")]
    pub fiscal_year_id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingReopenYearArgs {
    #[ts(type = "string")]
    pub fiscal_year_id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingGetVatPeriodTotalsArgs {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingSubmitVatSettlementArgs {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingPayVatSettlementNowArgs {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    #[ts(type = "string")]
    pub payment_method_id: Id,
}
