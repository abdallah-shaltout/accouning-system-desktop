//! `expense_categories` entity (21.02-B, owner B2). Source: `ExpenseCategory` in
//! `src/modules/expenses/types/index.ts`. Soft-delete table (B-1).

use sea_orm::entity::prelude::*;

use crate::entities::soft_delete::SoftDelete;
use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "sync_status")]
pub enum SyncStatus {
    #[sea_orm(string_value = "local")]
    Local,
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "synced")]
    Synced,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "expense_categories")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub name: String,
    pub icon: Option<String>,
    pub account_id: Id,
    pub default_tax_id: Option<Id>,
    pub default_cost_center_id: Option<Id>,
    pub active: bool,
    pub can_delete: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_status: SyncStatus,
    /// Generated column (P2-16 soft-delete): `CASE WHEN deleted_at IS NULL THEN name END`, read-only.
    pub name_live: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::expenses::Entity")]
    Expenses,
    #[sea_orm(has_many = "super::recurring_expenses::Entity")]
    RecurringExpenses,
}

impl Related<super::expenses::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Expenses.def()
    }
}

impl Related<super::recurring_expenses::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RecurringExpenses.def()
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    /// `name_live` is a MariaDB `GENERATED ALWAYS … STORED` column —
    /// read-only. An explicit value in an `INSERT`/`UPDATE` is rejected (error 1906, strict mode),
    /// so whatever a caller put there is dropped before every save.
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, sea_orm::DbErr>
    where
        C: sea_orm::ConnectionTrait,
    {
        self.name_live = sea_orm::ActiveValue::NotSet;
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

    fn deleted_at_active_model(at: Option<chrono::DateTime<chrono::Utc>>) -> ActiveModel {
        ActiveModel {
            deleted_at: crate::entities::soft_delete::deleted_at_value(at),
            ..Default::default()
        }
    }
}
