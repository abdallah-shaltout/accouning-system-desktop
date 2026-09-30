//! `journal_entries` entity (21.02-B, owner B2). Source: `JournalEntry` in
//! `src/modules/accounting/types/index.ts`. `search_normalized` (P2-38) filled by the write path;
//! `ck_journal_entries_balanced` (P2-39) enforced at the DB level too (migration m0012).

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
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "status")]
pub enum JournalEntryStatus {
    #[sea_orm(string_value = "DRAFT")]
    Draft,
    #[sea_orm(string_value = "POSTED")]
    Posted,
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
#[sea_orm(table_name = "journal_entries")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub number: String,
    pub date_day: chrono::NaiveDate,
    pub date_instant: Option<chrono::DateTime<chrono::Utc>>,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    pub r#type: JournalEntryType,
    pub status: JournalEntryStatus,
    /// `sourceRef.kind` — polymorphic (B-1): no FK on `source_id`.
    pub source_kind: Option<String>,
    pub source_id: Option<Id>,
    pub source_number: Option<String>,
    pub total_debit: Decimal,
    pub total_credit: Decimal,
    pub reversed: bool,
    pub reversal_of_id: Option<Id>,
    #[sea_orm(column_type = "Text", nullable)]
    pub reversal_reason: Option<String>,
    pub created_by: Id,
    pub posted_by: Option<Id>,
    pub posted_at_day: Option<chrono::NaiveDate>,
    pub posted_at_instant: Option<chrono::DateTime<chrono::Utc>>,
    #[sea_orm(column_type = "Json", nullable)]
    pub attachment_ids: Option<StringList>,
    pub template_id: Option<Id>,
    #[sea_orm(column_type = "Text", nullable)]
    pub search_normalized: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_status: SyncStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::journal_lines::Entity")]
    JournalLines,
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ReversalOfId",
        to = "Column::Id"
    )]
    ReversalOf,
}

impl Related<super::journal_lines::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalLines.def()
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    /// B-6/B-7 (P2-38): recomputes `search_normalized` from the exact field set
    /// `matchesJournalFilter`'s `includesText([e.number, e.description, e.sourceRef?.number],
    /// filter.search)` uses (`src/modules/accounting/services/accountingService.ts`) — the
    /// `JournalListPage.vue` search box's placeholder ("رقم القيد، البيان، أو المستند": entry
    /// number, description, or document) confirms the same three fields. Recomputed whenever any
    /// of them is `Set`; a field left `Unchanged` is read from `self`, one left `NotSet` on a partial
    /// update from the stored row (one `find_by_id`), so the haystack reflects the full row state.
    async fn before_save<C>(mut self, db: &C, insert: bool) -> Result<Self, sea_orm::DbErr>
    where
        C: sea_orm::ConnectionTrait,
    {
        let touched = self.number.is_set() || self.description.is_set() || self.source_number.is_set();
        if touched {
            let stored = match self.id.try_as_ref() {
                Some(id) if !insert && (self.number.is_not_set() || self.description.is_not_set() || self.source_number.is_not_set()) => Entity::find_by_id(*id).one(db).await?,
                _ => None,
            };
            let number = self.number.try_as_ref().map(|s| s.as_str()).or(stored.as_ref().map(|m| m.number.as_str()));
            let description = self.description.try_as_ref().map(|s| s.as_str()).or(stored.as_ref().map(|m| m.description.as_str()));
            let source_number = match self.source_number.try_as_ref() {
                Some(v) => v.as_deref(),
                None => stored.as_ref().and_then(|m| m.source_number.as_deref()),
            };
            self.search_normalized = sea_orm::ActiveValue::Set(Some(crate::utils::text::search_haystack(&[number, description, source_number])));
        }
        Ok(self)
    }
}

impl Model {
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }

    pub fn posted_at(&self) -> Option<DocDate> {
        self.posted_at_day.map(|day| doc_date::read(day, self.posted_at_instant))
    }
}
