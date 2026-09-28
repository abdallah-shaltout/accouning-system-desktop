//! `product_batches` (products/types `ProductBatch`). Child-of-`products` (FEFO batch rows always
//! belong to one product). `source_ref_id` is polymorphic — no FK (B-1).

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "product_batches")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub product_id: Id,
    pub batch_no: String,
    pub expiry_date: Option<NaiveDate>,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))")]
    pub qty: Decimal,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))")]
    pub unit_cost: Decimal,
    pub supplier_id: Option<Id>,
    /// `DocDate` triple (`m0016` G-28c): the mock writes an ISO instant here
    /// (`confirmPurchaseOrder`/opening or completion dates), which a plain `DATE` truncated.
    pub received_date_day: NaiveDate,
    pub received_date_instant: Option<DateTime<Utc>>,
    /// Polymorphic ref (purchase/stock-in doc this batch came from) — no FK (B-1).
    pub source_ref_id: Option<Id>,
    pub source_ref_number: Option<String>,
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

impl Model {
    /// The `DocDate` bridge (B-4): reads `received_date_day`/`received_date_instant` back into the
    /// shared value type (`m0016` G-28c).
    pub fn received_date(&self) -> DocDate {
        doc_date::read(self.received_date_day, self.received_date_instant)
    }
}
