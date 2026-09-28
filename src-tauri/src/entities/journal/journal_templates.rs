//! `journal_templates` entity (21.02-B, owner B2). Source: `JournalTemplate` in
//! `src/modules/accounting/types/index.ts`. Soft-delete table (B-1). `lines`/`recurrence` kept as
//! JSON (small, fixed-shape sub-documents — no ledger reader ever queries into them individually,
//! unlike posted `journal_lines`).

use sea_orm::entity::prelude::*;

use crate::entities::soft_delete::SoftDelete;
use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "recurrence_every")]
pub enum RecurrenceEvery {
    #[sea_orm(string_value = "month")]
    Month,
    #[sea_orm(string_value = "quarter")]
    Quarter,
    #[sea_orm(string_value = "year")]
    Year,
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
#[sea_orm(table_name = "journal_templates")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub name: String,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    #[sea_orm(column_type = "Json")]
    pub lines: serde_json::Value,
    pub recurrence_every: Option<RecurrenceEvery>,
    pub recurrence_day: Option<i8>,
    pub recurrence_next_date: Option<chrono::NaiveDate>,
    pub recurrence_auto_post: Option<bool>,
    pub created_by: Id,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_status: SyncStatus,
    /// Generated column (P2-16 soft-delete): `CASE WHEN deleted_at IS NULL THEN name END`, read-only.
    pub name_live: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl SoftDelete for Entity {
    fn id_column() -> Self::Column {
        Column::Id
    }

    fn deleted_at_column() -> Self::Column {
        Column::DeletedAt
    }

    fn deleted_at_active_model(at: Option<chrono::DateTime<chrono::Utc>>) -> ActiveModel {
        ActiveModel {
            deleted_at: crate::entities::soft_delete::deleted_at_value(at),
            ..Default::default()
        }
    }
}
