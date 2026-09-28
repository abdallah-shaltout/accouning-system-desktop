//! `stock_counts` (products/types `StockCount`). Header + `stock_count_lines` child.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "stock_counts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub number: String,
    /// `'OPEN' | 'REVIEW' | 'COMPLETED'`.
    pub status: String,
    /// `'all' | 'category' | 'location'`.
    pub scope: String,
    pub category_id: Option<Id>,
    pub location: Option<String>,
    pub blind: bool,
    pub started_at: DateTime<Utc>,
    pub started_by: Id,
    #[sea_orm(column_type = "Text", nullable)]
    pub note: Option<String>,
    pub adjustment_id: Option<Id>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::stock_count_lines::Entity")]
    Lines,
    #[sea_orm(belongs_to = "super::stock_adjustments::Entity", from = "Column::AdjustmentId", to = "super::stock_adjustments::Column::Id")]
    Adjustment,
}

impl Related<super::stock_count_lines::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lines.def()
    }
}

impl Related<super::stock_adjustments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Adjustment.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
