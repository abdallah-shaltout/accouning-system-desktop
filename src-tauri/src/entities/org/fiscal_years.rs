//! `fiscal_years` (accounting/types `FiscalYear`).

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "fiscal_years")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub is_closed: bool,
    /// FK added in m0015 (forward ref into `journal_entries`).
    pub closing_entry_id: Option<Id>,
    pub closed_at: Option<DateTime<Utc>>,
    pub closed_by: Option<Id>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::cost_center_budgets::Entity")]
    Budgets,
}

impl Related<super::cost_center_budgets::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Budgets.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
