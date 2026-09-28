//! `quotations` entity (21.02-B, owner B2). Source: `Quotation` in
//! `src/modules/invoices/types/index.ts`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::entities::values::StringList;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "status")]
pub enum QuotationStatus {
    #[sea_orm(string_value = "DRAFT")]
    Draft,
    #[sea_orm(string_value = "SENT")]
    Sent,
    #[sea_orm(string_value = "ACCEPTED")]
    Accepted,
    #[sea_orm(string_value = "REJECTED")]
    Rejected,
    #[sea_orm(string_value = "EXPIRED")]
    Expired,
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
#[sea_orm(table_name = "quotations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub number: String,
    pub date_day: chrono::NaiveDate,
    pub date_instant: Option<chrono::DateTime<chrono::Utc>>,
    /// `DocDate` triple (`m0016` G-22): the desk form stores an ISO instant
    /// (`InvoiceFormPage.vue:182`), which a plain `DATE` truncated.
    pub expiry_date_day: Option<chrono::NaiveDate>,
    pub expiry_date_instant: Option<chrono::DateTime<chrono::Utc>>,
    pub customer_id: Option<Id>,
    pub salesperson_id: Id,
    pub status: QuotationStatus,
    pub discount_rate: Decimal,
    pub discount_amount: Decimal,
    pub tax_amount: Decimal,
    pub sub_total: Decimal,
    pub grand_total: Decimal,
    #[sea_orm(column_type = "Text", nullable)]
    pub note: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub terms: Option<String>,
    pub po_reference: Option<String>,
    #[sea_orm(column_type = "Json", nullable)]
    pub attachment_ids: Option<StringList>,
    pub converted_invoice_id: Option<Id>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_status: SyncStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::quotation_lines::Entity")]
    QuotationLines,
    #[sea_orm(
        belongs_to = "super::invoices::Entity",
        from = "Column::ConvertedInvoiceId",
        to = "super::invoices::Column::Id"
    )]
    ConvertedInvoice,
}

impl Related<super::quotation_lines::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::QuotationLines.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }

    /// `None` when there is no expiry date; `Some` reads `expiry_date_day`/`expiry_date_instant`
    /// back into the shared value type (`m0016` G-22).
    pub fn expiry_date(&self) -> Option<DocDate> {
        self.expiry_date_day.map(|day| doc_date::read(day, self.expiry_date_instant))
    }
}
