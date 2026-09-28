//! `invoice_tenders` entity (21.02-B, owner B2). Child of `invoices`. Source: `Tender` in
//! `src/modules/invoices/types/index.ts`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "invoice_tenders")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub invoice_id: Id,
    pub position: i16,
    pub payment_method_id: Id,
    pub amount: Decimal,
    pub reference: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::invoices::Entity",
        from = "Column::InvoiceId",
        to = "super::invoices::Column::Id"
    )]
    Invoice,
}

impl Related<super::invoices::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Invoice.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
