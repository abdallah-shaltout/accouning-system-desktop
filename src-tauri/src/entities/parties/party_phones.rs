//! `party_phones` — child of `PartyCommon.phones`.

use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "party_phones")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub party_id: Id,
    pub position: i16,
    /// `'mobile' | 'work' | 'whatsapp'`.
    pub label: String,
    pub number: String,
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
