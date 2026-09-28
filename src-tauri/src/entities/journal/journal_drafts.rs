//! `journal_drafts` entity (21.02-B, owner B2). Kept SEPARATE from `journal_entries` per P2-14 —
//! every ledger/balance/report reader scans `journal_entries` expecting only posted GL impact.
//! Source: `JournalEntry` (draft shape) in `src/modules/accounting/types/index.ts`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::entities::values::StringList;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "type")]
pub enum JournalEntryType {
    #[sea_orm(string_value = "SYSTEM")]
    System,
    #[sea_orm(string_value = "MANUAL")]
    Manual,
    #[sea_orm(string_value = "OPENING")]
    Opening,
    #[sea_orm(string_value = "CLOSING")]
    Closing,
    #[sea_orm(string_value = "VAT_SETTLEMENT")]
    VatSettlement,
}

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "sync_status")]
pub enum SyncStatus {
    #[sea_orm(string_value = "local")]
    Local,
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "synced")]
    Synced,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "journal_drafts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub number: Option<String>,
    pub date_day: chrono::NaiveDate,
    pub date_instant: Option<chrono::DateTime<chrono::Utc>>,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    pub r#type: JournalEntryType,
    pub source_kind: Option<String>,
    pub source_id: Option<Id>,
    pub source_number: Option<String>,
    pub total_debit: Decimal,
    pub total_credit: Decimal,
    pub created_by: Id,
    #[sea_orm(column_type = "Json", nullable)]
    pub attachment_ids: Option<StringList>,
    pub template_id: Option<Id>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_status: SyncStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::journal_draft_lines::Entity")]
    JournalDraftLines,
}

impl Related<super::journal_draft_lines::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalDraftLines.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }
}
