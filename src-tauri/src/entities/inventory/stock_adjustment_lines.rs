//! `stock_adjustment_lines` — child of `stock_adjustments`.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_adjustment_lines")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub stock_adjustment_id: Id,
    pub position: i16,
    pub product_id: Id,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))", nullable)]
    pub system_qty: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))", nullable)]
    pub counted_qty: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))")]
    pub qty_change: Decimal,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))", nullable)]
    pub unit_cost: Option<Decimal>,
    pub batch_no: Option<String>,
    pub expiry_date: Option<NaiveDate>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::stock_adjustments::Entity", from = "Column::StockAdjustmentId", to = "super::stock_adjustments::Column::Id")]
    StockAdjustment,
    #[sea_orm(belongs_to = "super::super::catalog::products::Entity", from = "Column::ProductId", to = "super::super::catalog::products::Column::Id")]
    Product,
}

impl Related<super::stock_adjustments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockAdjustment.def()
    }
}

impl Related<super::super::catalog::products::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
