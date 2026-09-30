//! `stock_movements` (products/types `StockMovement`). Append-only ledger row — never
//! updated/deleted after insert (B-9's architecture test enforces this for posted-document
//! entities generally; `stock_movements` specifically is append-only per its own doc comment in
//! `db.ts`). `ref_id` is polymorphic — no FK (B-1).

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_movements")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub date_day: NaiveDate,
    pub date_instant: Option<DateTime<Utc>>,
    #[sea_orm(column_name = "date_key")]
    pub date_key: String,
    pub product_id: Id,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))")]
    pub qty_change: Decimal,
    #[sea_orm(column_type = "Decimal(Some((19, 2)))")]
    pub value_change: Decimal,
    /// `'sale'|'purchase'|'stock_in'|'loss'|'stocktake'|'refund'|'purchase_return'|'transfer_out'|'transfer_in'`.
    pub reason: String,
    /// Polymorphic ref — no FK (B-1).
    pub ref_id: Id,
    pub ref_number: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))", nullable)]
    pub balance_after: Option<Decimal>,
    pub batch_id: Option<Id>,
    pub created_at: DateTime<Utc>,
}

impl Model {
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::super::catalog::products::Entity", from = "Column::ProductId", to = "super::super::catalog::products::Column::Id")]
    Product,
    #[sea_orm(belongs_to = "super::super::catalog::product_batches::Entity", from = "Column::BatchId", to = "super::super::catalog::product_batches::Column::Id")]
    Batch,
}

impl Related<super::super::catalog::products::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl Related<super::super::catalog::product_batches::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Batch.def()
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    /// `date_key` is a MariaDB `GENERATED ALWAYS … STORED` column —
    /// read-only. An explicit value in an `INSERT`/`UPDATE` is rejected (error 1906, strict mode),
    /// so whatever a caller put there is dropped before every save.
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, sea_orm::DbErr>
    where
        C: sea_orm::ConnectionTrait,
    {
        self.date_key = sea_orm::ActiveValue::NotSet;
        Ok(self)
    }
}
