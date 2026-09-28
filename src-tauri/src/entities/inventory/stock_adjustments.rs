//! `stock_adjustments` (products/types `StockAdjustment`). Header + `stock_adjustment_lines` child.

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_adjustments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub number: String,
    /// `'STOCK_IN' | 'LOSS' | 'STOCKTAKE'`.
    pub r#type: String,
    pub date_day: NaiveDate,
    pub date_instant: Option<DateTime<Utc>>,
    #[sea_orm(column_name = "date_key")]
    pub date_key: String,
    /// `'DRAFT' | 'COMPLETED'`.
    pub status: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub note: Option<String>,
    /// `'opening' | 'owner_contribution' | 'gift' | 'found' | 'other'` — STOCK_IN only (A3).
    pub reason: Option<String>,
    pub offset_account_id: Option<Id>,
    pub approved_by: Option<Id>,
    pub approved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub sync_status: String,
}

impl Model {
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::stock_adjustment_lines::Entity")]
    Lines,
}

impl Related<super::stock_adjustment_lines::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lines.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
