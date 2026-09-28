//! `branches` (settings/types/dimensions.ts `Branch`). Not a soft-delete table — `active BOOL`
//! flips per B-2's correction to the phase-b inventory (see the migration's own doc comment).

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

use crate::entities::values::Address;
use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "branches")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub name: String,
    pub code: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub address: Option<String>,
    #[sea_orm(column_type = "Json", nullable)]
    pub national_address: Option<Address>,
    pub phone: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub receipt_header: Option<String>,
    pub cash_account_id: Option<Id>,
    pub bank_account_id: Option<Id>,
    pub default_price_list_id: Option<Id>,
    pub cost_center_id: Option<Id>,
    pub active: bool,
    pub can_delete: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub sync_status: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::cost_centers::Entity")]
    CostCenters,
}

impl Related<super::cost_centers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CostCenters.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
