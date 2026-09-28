//! `recurring_expenses` entity (21.02-B, owner B2). Source: `RecurringExpense` in
//! `src/modules/expenses/types/index.ts`. Soft-delete table (B-1). `day` has
//! `CHECK (day BETWEEN 1 AND 28)` at the DB level (migration m0011).

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::soft_delete::SoftDelete;
use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "paid_from_kind")]
pub enum PaidFromKind {
    #[sea_orm(string_value = "method")]
    Method,
    #[sea_orm(string_value = "credit")]
    Credit,
}

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
#[sea_orm(table_name = "recurring_expenses")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub name: String,
    pub category_id: Id,
    pub amount: Decimal,
    pub is_tax_invoice: bool,
    pub tax_id: Option<Id>,
    pub paid_from_kind: PaidFromKind,
    pub paid_from_payment_method_id: Option<Id>,
    pub paid_from_supplier_id: Option<Id>,
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    pub day: i8,
    pub next_date: chrono::NaiveDate,
    pub auto_post: bool,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_status: SyncStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::expense_categories::Entity",
        from = "Column::CategoryId",
        to = "super::expense_categories::Column::Id"
    )]
    ExpenseCategory,
}

impl Related<super::expense_categories::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExpenseCategory.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

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
