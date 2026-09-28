//! `card_settlement_groups` entity (21.02-B, owner B2). Child of `card_settlements`. Carries
//! `uq_card_settlement_groups_date_method(date_day, payment_method_id)` — the concurrency guard so
//! two terminals can't settle the same day×method twice (vouchers.md recommendation).

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "card_settlement_groups")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub card_settlement_id: Id,
    pub position: i16,
    pub date_day: chrono::NaiveDate,
    pub payment_method_id: Id,
    pub amount: Decimal,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::card_settlements::Entity",
        from = "Column::CardSettlementId",
        to = "super::card_settlements::Column::Id"
    )]
    CardSettlement,
}

impl Related<super::card_settlements::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CardSettlement.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
