//! `stock_transfers` (products/types `StockTransfer`). Header + `stock_transfer_lines` child.
//! `sent_at`/`received_at`/`rejected_at` are DocDate triples too (B-1's field list).

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_transfers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub number: String,
    pub from_branch_id: Id,
    pub to_branch_id: Id,
    /// `'DRAFT' | 'SENT' | 'RECEIVED' | 'REJECTED'`.
    pub status: String,
    pub date_day: NaiveDate,
    pub date_instant: Option<DateTime<Utc>>,
    #[sea_orm(column_name = "date_key")]
    pub date_key: String,
    pub sent_at_day: Option<NaiveDate>,
    pub sent_at_instant: Option<DateTime<Utc>>,
    #[sea_orm(column_name = "sent_at_key")]
    pub sent_at_key: Option<String>,
    pub received_at_day: Option<NaiveDate>,
    pub received_at_instant: Option<DateTime<Utc>>,
    #[sea_orm(column_name = "received_at_key")]
    pub received_at_key: Option<String>,
    pub rejected_at_day: Option<NaiveDate>,
    pub rejected_at_instant: Option<DateTime<Utc>>,
    #[sea_orm(column_name = "rejected_at_key")]
    pub rejected_at_key: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub note: Option<String>,
    pub sent_by: Option<Id>,
    pub received_by: Option<Id>,
    pub rejected_by: Option<Id>,
    #[sea_orm(column_type = "Text", nullable)]
    pub reject_reason: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((19, 2)))", nullable)]
    pub shortage_value: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub sync_status: String,
}

impl Model {
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }

    /// `None` when the transfer was never sent — `sent_at_day`/`sent_at_instant` are both SQL
    /// `NULL` until the SENT transition writes them.
    pub fn sent_at(&self) -> Option<DocDate> {
        self.sent_at_day.map(|day| doc_date::read(day, self.sent_at_instant))
    }

    pub fn received_at(&self) -> Option<DocDate> {
        self.received_at_day.map(|day| doc_date::read(day, self.received_at_instant))
    }

    pub fn rejected_at(&self) -> Option<DocDate> {
        self.rejected_at_day.map(|day| doc_date::read(day, self.rejected_at_instant))
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::stock_transfer_lines::Entity")]
    Lines,
}

impl Related<super::stock_transfer_lines::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lines.def()
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    /// `date_key`, `sent_at_key`, `received_at_key`, `rejected_at_key` are MariaDB `GENERATED ALWAYS … STORED` columns —
    /// read-only. An explicit value in an `INSERT`/`UPDATE` is rejected (error 1906, strict mode),
    /// so whatever a caller put there is dropped before every save.
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, sea_orm::DbErr>
    where
        C: sea_orm::ConnectionTrait,
    {
        self.date_key = sea_orm::ActiveValue::NotSet;
        self.sent_at_key = sea_orm::ActiveValue::NotSet;
        self.received_at_key = sea_orm::ActiveValue::NotSet;
        self.rejected_at_key = sea_orm::ActiveValue::NotSet;
        Ok(self)
    }
}
