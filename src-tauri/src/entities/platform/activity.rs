//! `activity` entity (21.02-B, owner B2). Append-only feed. Source: `ActivityEntry` in
//! `src/modules/core/types/index.ts`. No FK on nothing polymorphic here (entity_id doesn't exist
//! on this table — `link` carries the deep-link route instead).

use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::entities::values::RouteRefValue;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "kind")]
pub enum ActivityKind {
    #[sea_orm(string_value = "sale")]
    Sale,
    #[sea_orm(string_value = "refund")]
    Refund,
    #[sea_orm(string_value = "purchase")]
    Purchase,
    #[sea_orm(string_value = "purchase_return")]
    PurchaseReturn,
    #[sea_orm(string_value = "payment")]
    Payment,
    #[sea_orm(string_value = "stock")]
    Stock,
    #[sea_orm(string_value = "journal")]
    Journal,
    #[sea_orm(string_value = "product")]
    Product,
    #[sea_orm(string_value = "party")]
    Party,
    #[sea_orm(string_value = "user")]
    User,
    #[sea_orm(string_value = "settings")]
    Settings,
    #[sea_orm(string_value = "auth")]
    Auth,
    #[sea_orm(string_value = "shift")]
    Shift,
    #[sea_orm(string_value = "expense")]
    Expense,
    #[sea_orm(string_value = "voucher")]
    Voucher,
    #[sea_orm(string_value = "approval")]
    Approval,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "activity")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub date_day: chrono::NaiveDate,
    pub date_instant: Option<chrono::DateTime<chrono::Utc>>,
    pub user_id: Id,
    pub kind: ActivityKind,
    #[sea_orm(column_type = "Text")]
    pub message: String,
    #[sea_orm(column_type = "Json", nullable)]
    pub link: Option<RouteRefValue>,
    pub audit_id: Option<Id>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::audit::Entity",
        from = "Column::AuditId",
        to = "super::audit::Column::Id"
    )]
    Audit,
}

impl Related<super::audit::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Audit.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }
}
