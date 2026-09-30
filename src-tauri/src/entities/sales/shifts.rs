//! `shifts` entity (21.02-B, owner B2). Source: `Shift` in `src/modules/invoices/types/index.ts`.
//! `open_key` (generated, C-15 one-open-shift-per-terminal unique) is read-only — no code sets it.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "status")]
pub enum ShiftStatus {
    #[sea_orm(string_value = "OPEN")]
    Open,
    #[sea_orm(string_value = "CLOSED")]
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "handover_mode")]
pub enum HandoverMode {
    #[sea_orm(string_value = "HANDOVER")]
    Handover,
    #[sea_orm(string_value = "DROP")]
    Drop,
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
#[sea_orm(table_name = "shifts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub number: String,
    pub terminal_id: Id,
    pub branch_id: Option<Id>,
    pub status: ShiftStatus,
    pub opened_by: Id,
    pub opened_at_day: chrono::NaiveDate,
    pub opened_at_instant: Option<chrono::DateTime<chrono::Utc>>,
    pub opening_float: Decimal,
    #[sea_orm(column_type = "Json", nullable)]
    pub opening_denominations: Option<serde_json::Value>,
    pub closed_by: Option<Id>,
    pub closed_at_day: Option<chrono::NaiveDate>,
    pub closed_at_instant: Option<chrono::DateTime<chrono::Utc>>,
    pub counted_cash: Option<Decimal>,
    #[sea_orm(column_type = "Json", nullable)]
    pub closing_denominations: Option<serde_json::Value>,
    pub expected_cash: Option<Decimal>,
    pub variance: Option<Decimal>,
    pub handover_mode: Option<HandoverMode>,
    pub force_closed_by: Option<Id>,
    #[sea_orm(column_type = "Text", nullable)]
    pub note: Option<String>,
    /// Generated column (C-15): `CASE WHEN status = 'OPEN' THEN terminal_id END`, read-only.
    pub open_key: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub sync_status: SyncStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::shift_movements::Entity")]
    ShiftMovements,
}

impl Related<super::shift_movements::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ShiftMovements.def()
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    /// `open_key` is a MariaDB `GENERATED ALWAYS … STORED` column —
    /// read-only. An explicit value in an `INSERT`/`UPDATE` is rejected (error 1906, strict mode),
    /// so whatever a caller put there is dropped before every save.
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, sea_orm::DbErr>
    where
        C: sea_orm::ConnectionTrait,
    {
        self.open_key = sea_orm::ActiveValue::NotSet;
        Ok(self)
    }
}
