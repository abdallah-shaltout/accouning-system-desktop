//! `refund_lines` entity (21.02-B, owner B2). Child of `refunds`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "refund_lines")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub refund_id: Id,
    pub position: i16,
    pub invoice_line_id: Id,
    pub qty: Decimal,
    pub restock: Option<bool>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::refunds::Entity",
        from = "Column::RefundId",
        to = "super::refunds::Column::Id"
    )]
    Refund,
    #[sea_orm(
        belongs_to = "super::invoice_lines::Entity",
        from = "Column::InvoiceLineId",
        to = "super::invoice_lines::Column::Id"
    )]
    InvoiceLine,
}

impl Related<super::refunds::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Refund.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
