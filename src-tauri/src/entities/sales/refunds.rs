//! `refunds` entity (21.02-B, owner B2). Source: `Refund` in `src/modules/invoices/types/index.ts`.

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
    #[sea_orm(string_value = "card")]
    Card,
    #[sea_orm(string_value = "bank_transfer")]
    BankTransfer,
    #[sea_orm(string_value = "customer_credit")]
    CustomerCredit,
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
#[sea_orm(table_name = "refunds")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub number: String,
    pub invoice_id: Id,
    pub date_day: chrono::NaiveDate,
    pub date_instant: Option<chrono::DateTime<chrono::Utc>>,
    #[sea_orm(column_type = "Text", nullable)]
    pub reason: Option<String>,
    pub sub_total: Decimal,
    pub tax_amount: Decimal,
    pub grand_total: Decimal,
    pub settled_to_receivable: Decimal,
    pub cash_back: Decimal,
    pub refund_method: Option<RefundMethod>,
    pub credited_to_account: Option<Decimal>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_status: SyncStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::invoices::Entity",
        from = "Column::InvoiceId",
        to = "super::invoices::Column::Id"
    )]
    Invoice,
    #[sea_orm(has_many = "super::refund_lines::Entity")]
    RefundLines,
}

impl Related<super::invoices::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Invoice.def()
    }
}

impl Related<super::refund_lines::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RefundLines.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }
}
