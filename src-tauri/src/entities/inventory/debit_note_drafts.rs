//! `debit_note_drafts` (products/types `DebitNoteDraft`). Short-lived worklist row, hard-deletable
//! (B-9's posted-document delete-guard list does NOT include this table).

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::entities::doc_date;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebitNoteDraftLine {
    pub product_id: Id,
    pub batch_id: Id,
    #[serde(with = "crate::utils::money::serde_number")]
    pub qty: Decimal,
    #[serde(with = "crate::utils::money::serde_number")]
    pub unit_cost: Decimal,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, sea_orm::FromJsonQueryResult)]
#[serde(transparent)]
pub struct DebitNoteDraftLines(pub Vec<DebitNoteDraftLine>);

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "debit_note_drafts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub number: String,
    pub supplier_id: Id,
    pub date_day: NaiveDate,
    pub date_instant: Option<DateTime<Utc>>,
    #[sea_orm(column_name = "date_key")]
    pub date_key: String,
    /// Always `'DRAFT'`.
    pub status: String,
    #[sea_orm(column_type = "Json")]
    pub lines: DebitNoteDraftLines,
    #[sea_orm(column_type = "Text", nullable)]
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Model {
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::super::parties::parties::Entity", from = "Column::SupplierId", to = "super::super::parties::parties::Column::Id")]
    Supplier,
}

impl Related<super::super::parties::parties::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Supplier.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
