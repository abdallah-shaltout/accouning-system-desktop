//! `party_groups` (parties/types `PartyGroup`).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "party_groups")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    /// `'customer' | 'supplier'`.
    pub kind: String,
    pub name: String,
    pub price_list_id: Option<Id>,
    pub payment_terms_days: Option<i32>,
    #[sea_orm(column_type = "Decimal(Some((9, 4)))", nullable)]
    pub discount_percent: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::parties::Entity")]
    Parties,
}

impl Related<super::parties::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Parties.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
