//! `purchase_return_lines` entity (21.02-B, owner B2). Child of `purchase_returns`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "purchase_return_lines")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub purchase_return_id: Id,
    pub position: i16,
    pub product_id: Id,
    pub qty: Decimal,
    pub cost_price: Decimal,
    pub batch_id: Option<Id>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::purchase_returns::Entity",
        from = "Column::PurchaseReturnId",
        to = "super::purchase_returns::Column::Id"
    )]
    PurchaseReturn,
}

impl Related<super::purchase_returns::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PurchaseReturn.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
