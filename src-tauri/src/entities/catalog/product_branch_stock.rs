//! `product_branch_stock` — child of Product per-branch stock (v2 phase 9).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "product_branch_stock")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub product_id: Id,
    pub branch_id: Id,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))")]
    pub qty: Decimal,
    #[sea_orm(column_type = "Decimal(Some((19, 2)))")]
    pub value: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::products::Entity", from = "Column::ProductId", to = "super::products::Column::Id")]
    Product,
}

impl Related<super::products::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
