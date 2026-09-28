//! `cost_center_budgets` — child of `CostCenter.budgets`.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "cost_center_budgets")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub cost_center_id: Id,
    pub fiscal_year_id: Id,
    #[sea_orm(column_type = "Decimal(Some((19, 2)))")]
    pub amount: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::cost_centers::Entity", from = "Column::CostCenterId", to = "super::cost_centers::Column::Id")]
    CostCenter,
    #[sea_orm(belongs_to = "super::fiscal_years::Entity", from = "Column::FiscalYearId", to = "super::fiscal_years::Column::Id")]
    FiscalYear,
}

impl Related<super::cost_centers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CostCenter.def()
    }
}

impl Related<super::fiscal_years::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FiscalYear.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
