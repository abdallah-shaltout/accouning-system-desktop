//! Journal template + recurring-posting DTOs (12-accounting.md §2, §3.5). `JournalTemplateInput`
//! moves here from the mock's `journal.ts:168` (type-only move, zero behaviour change) per the
//! plan's checklist — `src/modules/accounting/types/index.ts` re-declares it and
//! `mocks/backend/journal.ts` re-exports it from there.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::entities::journal::journal_templates::{Model as TemplateModel, RecurrenceEvery as EntityRecurrenceEvery};
use crate::utils::id::Id;
use crate::utils::money::serde_number;

use crate::domains::parties::dto::PartyKind;

/// `JournalTemplateLine` (`types/index.ts:199-220`'s inline `lines` shape).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct JournalTemplateLine {
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
}

/// `RecurrenceEvery` (`types/index.ts:211` inline union) — lowercase, matching the entity's DB enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "accounting/types/gen/")]
pub enum RecurrenceEvery {
    Month,
    Quarter,
    Year,
}

impl From<EntityRecurrenceEvery> for RecurrenceEvery {
    fn from(e: EntityRecurrenceEvery) -> Self {
        match e {
            EntityRecurrenceEvery::Month => RecurrenceEvery::Month,
            EntityRecurrenceEvery::Quarter => RecurrenceEvery::Quarter,
            EntityRecurrenceEvery::Year => RecurrenceEvery::Year,
        }
    }
}

impl From<RecurrenceEvery> for EntityRecurrenceEvery {
    fn from(e: RecurrenceEvery) -> Self {
        match e {
            RecurrenceEvery::Month => EntityRecurrenceEvery::Month,
            RecurrenceEvery::Quarter => EntityRecurrenceEvery::Quarter,
            RecurrenceEvery::Year => EntityRecurrenceEvery::Year,
        }
    }
}

/// `JournalTemplateRecurrence` (`types/index.ts:211-217`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct JournalTemplateRecurrence {
    pub every: RecurrenceEvery,
    pub day: i32,
    pub next_date: String,
    pub auto_post: bool,
}

/// `JournalTemplate` (`types/index.ts:199-220`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct JournalTemplate {
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    pub description: String,
    pub lines: Vec<JournalTemplateLine>,
    #[ts(optional)]
    pub recurrence: Option<JournalTemplateRecurrence>,
    #[ts(type = "string")]
    pub created_at: String,
    #[ts(type = "string")]
    pub created_by: Id,
}

impl JournalTemplate {
    pub fn from_model(m: &TemplateModel) -> Self {
        let lines: Vec<JournalTemplateLine> = serde_json::from_value(m.lines.clone()).unwrap_or_default();
        let recurrence = m.recurrence_every.clone().map(|every| JournalTemplateRecurrence {
            every: every.into(),
            day: m.recurrence_day.unwrap_or(1) as i32,
            next_date: m.recurrence_next_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default(),
            auto_post: m.recurrence_auto_post.unwrap_or(false),
        });
        JournalTemplate {
            id: m.id,
            name: m.name.clone(),
            description: m.description.clone(),
            lines,
            recurrence,
            created_at: crate::utils::dates::format_iso_ms(m.created_at),
            created_by: m.created_by,
        }
    }
}

/// `JournalTemplateInput` (moved from `mocks/backend/journal.ts:168`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct JournalTemplateInput {
    pub name: String,
    pub description: String,
    pub lines: Vec<JournalTemplateLine>,
    #[ts(optional)]
    pub recurrence: Option<JournalTemplateRecurrence>,
}

// --- Command args ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingGetJournalTemplateArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingCreateOrUpdateJournalTemplateArgs {
    pub input: JournalTemplateInput,
    #[serde(default)]
    #[ts(optional, type = "string")]
    pub id: Option<Id>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingRemoveJournalTemplateArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingLoadTemplateIntoEntryArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "accounting/types/gen/")]
pub struct AccountingPostRecurringTemplateArgs {
    #[ts(type = "string")]
    pub id: Id,
}
