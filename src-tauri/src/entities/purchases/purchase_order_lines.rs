//! `purchase_order_lines` entity (21.02-B, owner B2). Child of `purchase_orders`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "purchase_order_lines")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub purchase_order_id: Id,
    pub position: i16,
    pub product_id: Id,
    pub qty: Decimal,
    pub cost_price: Decimal,
    pub unit_id: Option<Id>,
    pub unit_factor: Option<Decimal>,
    pub discount: Option<Decimal>,
    pub discount_is_pct: Option<bool>,
    pub tax_id: Option<Id>,
    pub received_qty: Option<Decimal>,
    pub batch_no: Option<String>,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub landed_cost_share: Option<Decimal>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::purchase_orders::Entity",
        from = "Column::PurchaseOrderId",
        to = "super::purchase_orders::Column::Id"
    )]
    PurchaseOrder,
}

impl Related<super::purchase_orders::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PurchaseOrder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
