//! `cost_centers` (settings/types/dimensions.ts `CostCenter`). Soft-delete table (B-1 list).

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

use crate::entities::soft_delete::{self, SoftDelete};
use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "cost_centers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub code: String,
    pub name: String,
    pub kind: String,
    pub parent_id: Option<Id>,
    pub manager_user_id: Option<Id>,
    pub active: bool,
    pub can_delete: bool,
    pub branch_id: Option<Id>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub sync_status: String,
    /// Generated column (read-only) — never set by app code.
    #[sea_orm(column_name = "code_live")]
    pub code_live: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::cost_center_budgets::Entity")]
    Budgets,
    #[sea_orm(belongs_to = "super::branches::Entity", from = "Column::BranchId", to = "super::branches::Column::Id")]
    Branch,
}

impl Related<super::cost_center_budgets::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Budgets.def()
    }
}

impl Related<super::branches::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Branch.def()
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    /// `code_live` is a MariaDB `GENERATED ALWAYS … STORED` column —
    /// read-only. An explicit value in an `INSERT`/`UPDATE` is rejected (error 1906, strict mode),
    /// so whatever a caller put there is dropped before every save.
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, sea_orm::DbErr>
    where
        C: sea_orm::ConnectionTrait,
    {
        self.code_live = sea_orm::ActiveValue::NotSet;
        Ok(self)
    }
}

impl SoftDelete for Entity {
    fn id_column() -> Self::Column {
        Column::Id
    }

    fn deleted_at_column() -> Self::Column {
        Column::DeletedAt
    }

    fn deleted_at_active_model(at: Option<DateTime<Utc>>) -> Self::ActiveModel {
        ActiveModel { deleted_at: soft_delete::deleted_at_value(at), ..Default::default() }
    }
}
