//! `card_settlements` entity (21.02-B, owner B2). Source: `CardSettlement` in
//! `src/modules/vouchers/types/index.ts`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

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
#[sea_orm(table_name = "card_settlements")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub number: String,
    pub date_day: chrono::NaiveDate,
    pub date_instant: Option<chrono::DateTime<chrono::Utc>>,
    pub gross_amount: Decimal,
    pub deposit_amount: Decimal,
    pub fee_amount: Decimal,
    #[sea_orm(column_type = "Text", nullable)]
    pub note: Option<String>,
    pub created_by: Id,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_status: SyncStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::card_settlement_groups::Entity")]
    CardSettlementGroups,
}

impl Related<super::card_settlement_groups::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CardSettlementGroups.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }
}
