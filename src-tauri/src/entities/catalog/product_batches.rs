//! `product_batches` (products/types `ProductBatch`). Child-of-`products` (FEFO batch rows always
//! belong to one product). `source_ref_id` is polymorphic — no FK (B-1).

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

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
    pub received_date: NaiveDate,
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
