//! Journal DTOs (12-accounting.md §2): entries, lines, drafts, filters, rows. Conventions as in
//! 11-expenses.md §2 (camelCase, `skip_serializing_none`, `#[ts(optional)]`, `Id`/`Decimal`/
//! `DocDate` wire types). `createdAt`/`postedAt` serialize with `utils::dates::iso_ms`.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::entities::journal::journal_drafts::Model as JournalDraftModel;
use crate::entities::journal::journal_entries::{JournalEntryStatus as EntityStatus, JournalEntryType as EntityType, Model as JournalEntryModel};
use crate::entities::journal::journal_lines::{Model as JournalLineModel, PartyKind as EntityPartyKind};
use crate::utils::id::Id;
use crate::utils::money::serde_number;
use crate::utils::route::RouteRef;

use crate::domains::parties::dto::PartyKind;

/// `JournalEntryType` (`types/index.ts:125`) — explicit renames matching the mock's own spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "accounting/types/gen/")]
pub enum JournalEntryType {
    #[serde(rename = "SYSTEM")]
    #[ts(rename = "SYSTEM")]
    System,
    #[serde(rename = "MANUAL")]
    #[ts(rename = "MANUAL")]
    Manual,
    #[serde(rename = "OPENING")]
    #[ts(rename = "OPENING")]
    Opening,
    #[serde(rename = "CLOSING")]
    #[ts(rename = "CLOSING")]
    Closing,
    #[serde(rename = "VAT_SETTLEMENT")]
    #[ts(rename = "VAT_SETTLEMENT")]
    VatSettlement,
}

impl From<EntityType> for JournalEntryType {
    fn from(t: EntityType) -> Self {
        match t {
            EntityType::System => JournalEntryType::System,
            EntityType::Manual => JournalEntryType::Manual,
            EntityType::Opening => JournalEntryType::Opening,
            EntityType::Closing => JournalEntryType::Closing,
            EntityType::VatSettlement => JournalEntryType::VatSettlement,
        }
    }
}

impl From<JournalEntryType> for EntityType {
    fn from(t: JournalEntryType) -> Self {
        match t {
            JournalEntryType::System => EntityType::System,
            JournalEntryType::Manual => EntityType::Manual,
            JournalEntryType::Opening => EntityType::Opening,
            JournalEntryType::Closing => EntityType::Closing,
            JournalEntryType::VatSettlement => EntityType::VatSettlement,
        }
    }
}

/// `JournalEntryStatus` (`types/index.ts:126`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
#[ts(export_to = "accounting/types/gen/")]
pub enum JournalEntryStatus {
    Draft,
    Posted,
}

/// `JournalSourceKind` (`types/index.ts:79-98`) — 12 variants, camelCase; the canonical enum other
/// domains reuse (analysis §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub enum JournalSourceKind {
    Invoice,
    Refund,
    PurchaseOrder,
    PurchaseReturn,
    Payment,
    StockAdjustment,
    Expense,
    Voucher,
    Settlement,
    Shift,
    FxReval,
    Opening,
}

impl JournalSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            JournalSourceKind::Invoice => "invoice",
            JournalSourceKind::Refund => "refund",
            JournalSourceKind::PurchaseOrder => "purchaseOrder",
            JournalSourceKind::PurchaseReturn => "purchaseReturn",
            JournalSourceKind::Payment => "payment",
            JournalSourceKind::StockAdjustment => "stockAdjustment",
            JournalSourceKind::Expense => "expense",
            JournalSourceKind::Voucher => "voucher",
            JournalSourceKind::Settlement => "settlement",
            JournalSourceKind::Shift => "shift",
            JournalSourceKind::FxReval => "fxReval",
            JournalSourceKind::Opening => "opening",
        }
    }

    /// Inverse of `as_str` — parses the raw camelCase string the DB/model stores (`source_kind:
    /// Option<String>` on `journal_entries`/`journal_drafts`) back into the enum for `JournalSourceRef`
    /// (§3.4: the hand-written contract types `sourceRef.kind` as `JournalSourceKind`, not `string`).
    pub fn from_str_wire(s: &str) -> Option<Self> {
        Some(match s {
            "invoice" => JournalSourceKind::Invoice,
            "refund" => JournalSourceKind::Refund,
            "purchaseOrder" => JournalSourceKind::PurchaseOrder,
            "purchaseReturn" => JournalSourceKind::PurchaseReturn,
            "payment" => JournalSourceKind::Payment,
            "stockAdjustment" => JournalSourceKind::StockAdjustment,
            "expense" => JournalSourceKind::Expense,
            "voucher" => JournalSourceKind::Voucher,
            "settlement" => JournalSourceKind::Settlement,
            "shift" => JournalSourceKind::Shift,
            "fxReval" => JournalSourceKind::FxReval,
            "opening" => JournalSourceKind::Opening,
            _ => return None,
        })
    }
}

/// `JournalSourceRef` (inline `types/index.ts:135`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct JournalSourceRef {
    pub kind: JournalSourceKind,
    #[ts(type = "string")]
    pub id: Id,
    #[ts(optional)]
    pub number: Option<String>,
}

/// `JournalLine` (`types/index.ts:100`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct JournalLine {
    #[ts(type = "string")]
    pub id: Id,
    #[ts(type = "string")]
    pub account_id: Id,
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub debit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub credit: Decimal,
    #[ts(optional)]
    pub party_kind: Option<PartyKind>,
    #[ts(optional, type = "string")]
    pub party_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub branch_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub cost_center_id: Option<Id>,
    #[ts(optional)]
    pub currency: Option<String>,
    #[serde(default, with = "serde_number::option")]
    #[ts(optional, type = "number")]
    pub amount_fc: Option<Decimal>,
    #[serde(default, with = "serde_number::option")]
    #[ts(optional, type = "number")]
    pub rate: Option<Decimal>,
}

fn party_kind_from_entity(kind: EntityPartyKind) -> PartyKind {
    match kind {
        EntityPartyKind::Customer => PartyKind::Customer,
        EntityPartyKind::Supplier => PartyKind::Supplier,
    }
}

impl JournalLine {
    pub fn from_model(m: &JournalLineModel) -> Self {
        JournalLine {
            id: m.id,
            account_id: m.account_id,
            description: m.description.clone(),
            debit: m.debit,
            credit: m.credit,
            party_kind: m.party_kind.map(party_kind_from_entity),
            party_id: m.party_id,
            branch_id: m.branch_id,
            cost_center_id: m.cost_center_id,
            currency: m.currency.clone(),
            amount_fc: m.amount_fc,
            rate: m.rate,
        }
    }
}

/// `JournalEntry` (`types/index.ts:128`). `reversed` is emitted only when `true` (the mock leaves
/// it `undefined` otherwise — `Option<bool>` + `skip_serializing_none` gives exactly that on the
/// wire when the caller sets it to `None` for a non-reversed entry, and callers must never set
/// `Some(false)`). `posted_by`/`posted_at` absent on drafts.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct JournalEntry {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    /// `DocDate::key()` — a business day (`YYYY-MM-DD`) or ISO instant, matching the mock's plain
    /// `string` field exactly (no `DocDate` triple here; the same wire convention every other
    /// domain's response DTOs use for a date field).
    pub date: String,
    pub description: String,
    #[serde(rename = "type")]
    pub kind: JournalEntryType,
    pub status: JournalEntryStatus,
    #[ts(optional)]
    pub source_ref: Option<JournalSourceRef>,
    pub lines: Vec<JournalLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total_debit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total_credit: Decimal,
    #[ts(optional)]
    pub reversed: Option<bool>,
    #[ts(optional, type = "string")]
    pub reversal_of_id: Option<Id>,
    #[ts(optional)]
    pub reversal_reason: Option<String>,
    #[ts(type = "string")]
    pub created_by: Id,
    #[ts(type = "string")]
    pub created_at: String,
    #[ts(optional, type = "string")]
    pub posted_by: Option<Id>,
    #[ts(optional)]
    pub posted_at: Option<String>,
    #[ts(optional)]
    pub attachment_ids: Option<Vec<String>>,
    #[ts(optional, type = "string")]
    pub template_id: Option<Id>,
}

impl JournalEntry {
    /// `entry_dto` (§3.1) — a posted entry.
    pub fn from_model(m: &JournalEntryModel, lines: &[JournalLineModel]) -> Self {
        JournalEntry {
            id: m.id,
            number: m.number.clone(),
            date: m.date().key(),
            description: m.description.clone(),
            kind: m.r#type.clone().into(),
            status: match m.status {
                EntityStatus::Draft => JournalEntryStatus::Draft,
                EntityStatus::Posted => JournalEntryStatus::Posted,
            },
            source_ref: m.source_kind.as_deref().and_then(JournalSourceKind::from_str_wire).map(|kind| JournalSourceRef { kind, id: m.source_id.expect("source_kind implies source_id"), number: m.source_number.clone() }),
            lines: lines.iter().map(JournalLine::from_model).collect(),
            total_debit: m.total_debit,
            total_credit: m.total_credit,
            reversed: if m.reversed { Some(true) } else { None },
            reversal_of_id: m.reversal_of_id,
            reversal_reason: m.reversal_reason.clone(),
            created_by: m.created_by,
            created_at: crate::utils::dates::format_iso_ms(m.created_at),
            posted_by: m.posted_by,
            posted_at: m.posted_at().map(|d| d.key()),
            attachment_ids: m.attachment_ids.clone().map(|l| l.0),
            template_id: m.template_id,
        }
    }

    /// `draft_dto` (§3.1) — a saved draft. Draft lines carry no `currency` column separately; the
    /// entity has one, so `JournalLine::from_model` on the draft's shape is built inline here since
    /// `journal_draft_lines::Model` is a distinct SeaORM type from `journal_lines::Model`.
    pub fn from_draft_model(m: &JournalDraftModel, lines: &[crate::entities::journal::journal_draft_lines::Model], base_currency: &str) -> Self {
        JournalEntry {
            id: m.id,
            number: m.number.clone().unwrap_or_default(),
            date: m.date().key(),
            description: m.description.clone(),
            kind: match m.r#type {
                crate::entities::journal::journal_drafts::JournalEntryType::System => JournalEntryType::System,
                crate::entities::journal::journal_drafts::JournalEntryType::Manual => JournalEntryType::Manual,
                crate::entities::journal::journal_drafts::JournalEntryType::Opening => JournalEntryType::Opening,
                crate::entities::journal::journal_drafts::JournalEntryType::Closing => JournalEntryType::Closing,
                crate::entities::journal::journal_drafts::JournalEntryType::VatSettlement => JournalEntryType::VatSettlement,
            },
            status: JournalEntryStatus::Draft,
            source_ref: m.source_kind.as_deref().and_then(JournalSourceKind::from_str_wire).map(|kind| JournalSourceRef { kind, id: m.source_id.expect("source_kind implies source_id"), number: m.source_number.clone() }),
            lines: lines
                .iter()
                .map(|l| JournalLine {
                    id: l.id,
                    account_id: l.account_id,
                    description: l.description.clone(),
                    debit: l.debit,
                    credit: l.credit,
                    party_kind: l.party_kind.as_ref().map(|k| match k {
                        crate::entities::journal::journal_draft_lines::PartyKind::Customer => PartyKind::Customer,
                        crate::entities::journal::journal_draft_lines::PartyKind::Supplier => PartyKind::Supplier,
                    }),
                    party_id: l.party_id,
                    branch_id: l.branch_id,
                    cost_center_id: l.cost_center_id,
                    currency: l.currency.clone().or_else(|| Some(base_currency.to_string())),
                    amount_fc: l.amount_fc,
                    rate: l.rate,
                })
                .collect(),
            total_debit: m.total_debit,
            total_credit: m.total_credit,
            reversed: None,
            reversal_of_id: None,
            reversal_reason: None,
            created_by: m.created_by,
            created_at: crate::utils::dates::format_iso_ms(m.created_at),
            posted_by: None,
            posted_at: None,
            attachment_ids: m.attachment_ids.clone().map(|l| l.0),
            template_id: m.template_id,
        }
    }
}

/// `JournalEntryInputLine` (part of `JournalEntryInput`, `types/index.ts:151`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct JournalEntryInputLine {
    #[ts(type = "string")]
    pub account_id: Id,
    #[ts(optional)]
    pub description: Option<String>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub debit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub credit: Decimal,
    #[ts(optional)]
    pub party_kind: Option<PartyKind>,
    #[ts(optional, type = "string")]
    pub party_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub branch_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub cost_center_id: Option<Id>,
}

/// `JournalEntryInput` (`types/index.ts:151`). `reference` is accepted but ignored (as in the
/// mock). `date` stays a raw string on the wire.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct JournalEntryInput {
    pub date: String,
    pub description: String,
    #[ts(optional)]
    pub reference: Option<String>,
    pub lines: Vec<JournalEntryInputLine>,
    #[ts(optional)]
    pub attachment_ids: Option<Vec<String>>,
    #[ts(optional)]
    pub as_draft: Option<bool>,
    #[ts(optional, type = "string")]
    pub template_id: Option<Id>,
}

/// `JournalFilter` (`types/index.ts:171`) — all optional.
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct JournalFilter {
    #[ts(optional)]
    pub r#type: Option<JournalEntryType>,
    #[ts(optional)]
    pub from: Option<String>,
    #[ts(optional)]
    pub to: Option<String>,
    #[ts(optional)]
    pub search: Option<String>,
    #[ts(optional, type = "string")]
    pub account_id: Option<Id>,
    #[ts(optional, type = "string")]
    pub party_id: Option<Id>,
    #[ts(optional)]
    pub status: Option<JournalEntryStatus>,
    #[ts(optional)]
    pub has_attachments: Option<bool>,
    #[ts(optional)]
    pub reversed: Option<bool>,
    #[ts(optional, type = "string")]
    pub user_id: Option<Id>,
    #[serde(default, with = "serde_number::option")]
    #[ts(optional, type = "number")]
    pub min_amount: Option<Decimal>,
    #[serde(default, with = "serde_number::option")]
    #[ts(optional, type = "number")]
    pub max_amount: Option<Decimal>,
    #[ts(optional)]
    pub source_kind: Option<JournalSourceKind>,
}

/// `JournalRow` (`accountingService.ts:158`) — flat: `JournalEntry` fields + `createdByName`,
/// `sourceLink`, `sourceLabel`, `attachmentCount`.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct JournalRow {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    pub date: String,
    pub description: String,
    #[serde(rename = "type")]
    pub kind: JournalEntryType,
    pub status: JournalEntryStatus,
    #[ts(optional)]
    pub source_ref: Option<JournalSourceRef>,
    pub lines: Vec<JournalLine>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total_debit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total_credit: Decimal,
    #[ts(optional)]
    pub reversed: Option<bool>,
    #[ts(optional, type = "string")]
    pub reversal_of_id: Option<Id>,
    #[ts(optional)]
    pub reversal_reason: Option<String>,
    #[ts(type = "string")]
    pub created_by: Id,
    #[ts(type = "string")]
    pub created_at: String,
    #[ts(optional, type = "string")]
    pub posted_by: Option<Id>,
    #[ts(optional)]
    pub posted_at: Option<String>,
    #[ts(optional)]
    pub attachment_ids: Option<Vec<String>>,
    #[ts(optional, type = "string")]
    pub template_id: Option<Id>,
    pub created_by_name: String,
    #[ts(optional, type = "import('@/modules/core/types/route').AppRoute")]
    pub source_link: Option<RouteRef>,
    #[ts(optional)]
    pub source_label: Option<String>,
    pub attachment_count: u32,
}

impl JournalRow {
    pub fn from_entry(e: JournalEntry, created_by_name: String, source_link: Option<RouteRef>, source_label: Option<String>) -> Self {
        let attachment_count = e.attachment_ids.as_ref().map(|v| v.len() as u32).unwrap_or(0);
        JournalRow {
            id: e.id,
            number: e.number,
            date: e.date,
            description: e.description,
            kind: e.kind,
            status: e.status,
            source_ref: e.source_ref,
            lines: e.lines,
            total_debit: e.total_debit,
            total_credit: e.total_credit,
            reversed: e.reversed,
            reversal_of_id: e.reversal_of_id,
            reversal_reason: e.reversal_reason,
            created_by: e.created_by,
            created_at: e.created_at,
            posted_by: e.posted_by,
            posted_at: e.posted_at,
            attachment_ids: e.attachment_ids,
            template_id: e.template_id,
            created_by_name,
            source_link,
            source_label,
            attachment_count,
        }
    }
}

/// `JournalEntryDetail` (inline return of `getJournalEntry`, :260) — `JournalRow` fields +
/// `reversedById?`/`reversedByNumber?`, `related`.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct JournalEntryDetail {
    #[serde(flatten)]
    pub row: JournalRow,
    #[ts(optional, type = "string")]
    pub reversed_by_id: Option<Id>,
    #[ts(optional)]
    pub reversed_by_number: Option<String>,
    pub related: Vec<JournalRow>,
}

/// `LinkedJournalEntry` (`accountingService.ts:219`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct LinkedJournalEntry {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    pub description: String,
}

// --- Command args ---------------------------------------------------------------------------

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingGetJournalEntriesArgs {
    #[ts(optional)]
    pub filter: Option<JournalFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingGetJournalEntriesForSourceArgs {
    pub source_kind: String,
    pub source_id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingGetJournalEntriesPagedArgs {
    pub query: crate::core::dto::PagedQuery<JournalFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingGetJournalEntryArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingCreateJournalEntryArgs {
    pub input: JournalEntryInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingUpdateJournalDraftArgs {
    #[ts(type = "string")]
    pub id: Id,
    pub input: JournalEntryInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingPostJournalDraftArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingDeleteJournalDraftArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingReverseJournalEntryArgs {
    #[ts(type = "string")]
    pub id: Id,
    pub date: String,
    pub reason: String,
}
