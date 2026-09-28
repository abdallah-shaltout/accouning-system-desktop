//! `expenses` entity (21.02-B, owner B2). Source: `Expense` in
//! `src/modules/expenses/types/index.ts`. `paid_from` (a TS discriminated union) is split into
//! `paid_from_kind` + the two nullable target columns (mirrors `ExpensePaidFrom`).

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::entities::values::StringList;
use crate::utils::dates::DocDate;
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
#[sea_orm(table_name = "expenses")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub number: String,
    pub date_day: chrono::NaiveDate,
    pub date_instant: Option<chrono::DateTime<chrono::Utc>>,
    pub category_id: Id,
    pub amount: Decimal,
    pub is_tax_invoice: bool,
    pub tax_id: Option<Id>,
    pub net_amount: Decimal,
    pub tax_amount: Decimal,
    pub supplier_vat_number: Option<String>,
    pub supplier_invoice_no: Option<String>,
    pub cost_center_id: Option<Id>,
    pub paid_from_kind: PaidFromKind,
    pub paid_from_payment_method_id: Option<Id>,
    pub paid_from_supplier_id: Option<Id>,
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    #[sea_orm(column_type = "Json", nullable)]
    pub attachment_ids: Option<StringList>,
    pub repeat_monthly: bool,
    pub recurring_template_id: Option<Id>,
    pub created_by: Id,
    pub branch_id: Option<Id>,
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

impl Model {
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }
}
