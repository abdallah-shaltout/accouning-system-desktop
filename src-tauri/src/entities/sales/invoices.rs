//! `invoices` entity (21.02-B, owner B2). Source: `src/modules/invoices/types/index.ts` `Invoice`,
//! table created by `migration::m0008_sales`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::entities::values::StringList;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "status")]
pub enum InvoiceStatus {
    #[sea_orm(string_value = "DRAFT")]
    Draft,
    #[sea_orm(string_value = "COMPLETED")]
    Completed,
    #[sea_orm(string_value = "REFUNDED")]
    Refunded,
}

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "payment_status")]
pub enum PaymentStatus {
    #[sea_orm(string_value = "UNPAID")]
    Unpaid,
    #[sea_orm(string_value = "PARTIALLY_PAID")]
    PartiallyPaid,
    #[sea_orm(string_value = "PAID")]
    Paid,
}

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "payment_method")]
pub enum SalePaymentMethod {
    #[sea_orm(string_value = "cash")]
    Cash,
    #[sea_orm(string_value = "card")]
    Card,
    #[sea_orm(string_value = "bank_transfer")]
    BankTransfer,
    #[sea_orm(string_value = "credit")]
    Credit,
}

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "source")]
pub enum InvoiceSource {
    #[sea_orm(string_value = "POS")]
    Pos,
    #[sea_orm(string_value = "DESK")]
    Desk,
}

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "invoice_type")]
pub enum InvoiceType {
    #[sea_orm(string_value = "STANDARD")]
    Standard,
    #[sea_orm(string_value = "SIMPLIFIED")]
    Simplified,
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
#[sea_orm(table_name = "invoices")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub number: String,
    pub date_day: chrono::NaiveDate,
    pub date_instant: Option<chrono::DateTime<chrono::Utc>>,
    pub customer_id: Option<Id>,
    pub cashier_id: Id,
    pub status: InvoiceStatus,
    pub payment_status: PaymentStatus,
    pub sub_total: Decimal,
    pub discount_rate: Decimal,
    pub discount_amount: Decimal,
    pub tax_rate: Decimal,
    pub tax_amount: Decimal,
    pub grand_total: Decimal,
    pub payment_method: SalePaymentMethod,
    pub paid_amount: Decimal,
    pub refunded_amount: Decimal,
    pub tendered_amount: Option<Decimal>,
    pub due_date: Option<chrono::NaiveDate>,
    #[sea_orm(column_type = "Text", nullable)]
    pub note: Option<String>,
    pub source: Option<InvoiceSource>,
    pub shift_id: Option<Id>,
    pub branch_id: Option<Id>,
    pub invoice_type: Option<InvoiceType>,
    pub po_reference: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub terms: Option<String>,
    #[sea_orm(column_type = "Json", nullable)]
    pub attachment_ids: Option<StringList>,
    pub currency: Option<String>,
    pub exchange_rate: Option<Decimal>,
    pub cost_center_id: Option<Id>,
    #[sea_orm(column_type = "Text", nullable)]
    pub search_normalized: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_status: SyncStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::invoice_lines::Entity")]
    InvoiceLines,
    #[sea_orm(has_many = "super::invoice_tenders::Entity")]
    InvoiceTenders,
}

impl Related<super::invoice_lines::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::InvoiceLines.def()
    }
}

impl Related<super::invoice_tenders::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::InvoiceTenders.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// The `DocDate` bridge (B-4): reads `date_day`/`date_instant` back into the shared value type.
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }
}
