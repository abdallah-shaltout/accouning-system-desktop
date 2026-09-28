//! `product_prices` — child of PriceList/Product override (C-08 naming: not `price_list_values`).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "product_prices")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub product_id: Id,
    pub price_list_id: Id,
    pub unit_id: Option<Id>,
    #[sea_orm(column_type = "Decimal(Some((19, 2)))")]
    pub value: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::products::Entity", from = "Column::ProductId", to = "super::products::Column::Id")]
    Product,
    #[sea_orm(belongs_to = "super::price_lists::Entity", from = "Column::PriceListId", to = "super::price_lists::Column::Id")]
    PriceList,
}

impl Related<super::products::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl Related<super::price_lists::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PriceList.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
