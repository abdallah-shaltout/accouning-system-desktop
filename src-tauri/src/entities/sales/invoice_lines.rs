//! `invoice_lines` entity (21.02-B, owner B2). Child of `invoices`. Source: `InvoiceLine` in
//! `src/modules/invoices/types/index.ts`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "invoice_lines")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub invoice_id: Id,
    pub position: i16,
    /// `NULL` for a free-text line (`m0016` G-22; was `NOT NULL` before D-I2 was decided — the DTO
    /// layer re-synthesizes `freetext-{position}` for these rows, `08-invoices.md` §2).
    pub product_id: Option<Id>,
    pub name: String,
    pub qty: Decimal,
    pub price: Decimal,
    pub cost_price: Decimal,
    pub discount: Decimal,
    pub tax_id: Option<Id>,
    pub tax_category: Option<String>,
    pub tax_rate: Option<Decimal>,
    pub net: Option<Decimal>,
    pub vat: Option<Decimal>,
    pub unit_id: Option<Id>,
    pub unit_factor: Option<Decimal>,
    pub list_price: Option<Decimal>,
    #[sea_orm(column_type = "Text", nullable)]
    pub price_override_reason: Option<String>,
    pub batch_id: Option<Id>,
    pub batch_no: Option<String>,
    pub is_free_text: bool,
    pub revenue_account_id: Option<Id>,
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
