//! `purchase_returns` entity (21.02-B, owner B2). Source: `PurchaseReturn` in
//! `src/modules/purchases/types/index.ts`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "refund_method")]
pub enum RefundMethod {
    #[sea_orm(string_value = "cash")]
    Cash,
    #[sea_orm(string_value = "bank_transfer")]
    BankTransfer,
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
#[sea_orm(table_name = "purchase_returns")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub number: String,
    pub purchase_order_id: Id,
    pub supplier_id: Id,
    pub date_day: chrono::NaiveDate,
    pub date_instant: Option<chrono::DateTime<chrono::Utc>>,
    #[sea_orm(column_type = "Text", nullable)]
    pub reason: Option<String>,
    pub sub_total: Decimal,
    pub tax_amount: Decimal,
    pub grand_total: Decimal,
    pub settled_to_payable: Decimal,
    pub cash_back: Decimal,
    pub refund_method: RefundMethod,
    pub from_draft_id: Option<Id>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_status: SyncStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::purchase_orders::Entity",
        from = "Column::PurchaseOrderId",
        to = "super::purchase_orders::Column::Id"
    )]
    PurchaseOrder,
    #[sea_orm(has_many = "super::purchase_return_lines::Entity")]
    PurchaseReturnLines,
}

impl Related<super::purchase_orders::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PurchaseOrder.def()
    }
}

impl Related<super::purchase_return_lines::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PurchaseReturnLines.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }
}
