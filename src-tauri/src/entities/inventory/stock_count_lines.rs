//! `stock_count_lines` — child of `stock_counts`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_count_lines")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub stock_count_id: Id,
    pub position: i16,
    pub product_id: Id,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))")]
    pub system_qty: Decimal,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))", nullable)]
    pub counted_qty: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))")]
    pub unit_cost: Decimal,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::stock_counts::Entity", from = "Column::StockCountId", to = "super::stock_counts::Column::Id")]
    StockCount,
    #[sea_orm(belongs_to = "super::super::catalog::products::Entity", from = "Column::ProductId", to = "super::super::catalog::products::Column::Id")]
    Product,
}

impl Related<super::stock_counts::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockCount.def()
    }
}

impl Related<super::super::catalog::products::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
