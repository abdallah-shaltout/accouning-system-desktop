//! `stock_transfer_lines` — child of `stock_transfers`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_transfer_lines")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub stock_transfer_id: Id,
    pub position: i16,
    pub product_id: Id,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))")]
    pub qty: Decimal,
    /// The product's own `ProductUnit.id` (a free string such as `pu-panadol-box`), not a `units`
    /// row id — `VARCHAR(64)`, no FK (m0020).
    pub unit_id: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))", nullable)]
    pub unit_factor: Option<Decimal>,
    pub batch_id: Option<Id>,
    pub batch_no: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))", nullable)]
    pub received_qty: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))", nullable)]
    pub unit_cost: Option<Decimal>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::stock_transfers::Entity", from = "Column::StockTransferId", to = "super::stock_transfers::Column::Id")]
    StockTransfer,
    #[sea_orm(belongs_to = "super::super::catalog::products::Entity", from = "Column::ProductId", to = "super::super::catalog::products::Column::Id")]
    Product,
}

impl Related<super::stock_transfers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StockTransfer.def()
    }
}

impl Related<super::super::catalog::products::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
