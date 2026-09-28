//! `approval_requests` entity (21.02-B, owner B2). Source: `ApprovalRequest` in
//! `src/modules/approvals/types/index.ts`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::entities::values::RouteRefValue;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "kind")]
pub enum ApprovalKind {
    #[sea_orm(string_value = "discount")]
    Discount,
    #[sea_orm(string_value = "write_off")]
    WriteOff,
    #[sea_orm(string_value = "below_cost")]
    BelowCost,
}

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "status")]
pub enum ApprovalStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "approved")]
    Approved,
    #[sea_orm(string_value = "rejected")]
    Rejected,
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
#[sea_orm(table_name = "approval_requests")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub kind: ApprovalKind,
    #[sea_orm(column_type = "Text")]
    pub summary: String,
    pub value: Decimal,
    #[sea_orm(column_type = "Text", nullable)]
    pub request_note: Option<String>,
    pub requested_by: Id,
    pub requested_by_name: String,
    pub requested_at_day: chrono::NaiveDate,
    pub requested_at_instant: Option<chrono::DateTime<chrono::Utc>>,
    pub status: ApprovalStatus,
    pub decided_by: Option<Id>,
    pub decided_by_name: Option<String>,
    pub decided_at_day: Option<chrono::NaiveDate>,
    pub decided_at_instant: Option<chrono::DateTime<chrono::Utc>>,
    #[sea_orm(column_type = "Text", nullable)]
    pub decision_comment: Option<String>,
    #[sea_orm(column_type = "Json", nullable)]
    pub link: Option<RouteRefValue>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub sync_status: SyncStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn requested_at(&self) -> DocDate {
        doc_date::read(self.requested_at_day, self.requested_at_instant)
    }

    pub fn decided_at(&self) -> Option<DocDate> {
        self.decided_at_day.map(|day| doc_date::read(day, self.decided_at_instant))
    }
}
