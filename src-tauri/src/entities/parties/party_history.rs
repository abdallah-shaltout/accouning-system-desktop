//! `party_history` (parties/types `PartyHistoryEntry`). Append-only, child-of-party.

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "party_history")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub party_id: Id,
    /// `'customer' | 'supplier'`.
    pub party_kind: String,
    pub date: NaiveDate,
    #[sea_orm(column_type = "Text")]
    pub message: String,
    pub user_id: Id,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::parties::Entity", from = "Column::PartyId", to = "super::parties::Column::Id")]
    Party,
}

impl Related<super::parties::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Party.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
