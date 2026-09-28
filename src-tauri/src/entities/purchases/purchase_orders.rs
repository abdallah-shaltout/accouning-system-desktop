//! `purchase_orders` entity (21.02-B, owner B2). Source: `PurchaseOrder` in
//! `src/modules/purchases/types/index.ts`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::entities::values::StringList;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "status")]
pub enum PurchaseStatus {
    #[sea_orm(string_value = "DRAFT")]
    Draft,
    #[sea_orm(string_value = "ORDERED")]
    Ordered,
    #[sea_orm(string_value = "RECEIVED")]
    Received,
    #[sea_orm(string_value = "CANCELED")]
    Canceled,
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
#[sea_orm(table_name = "purchase_orders")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub number: String,
    pub supplier_id: Id,
    pub date_day: chrono::NaiveDate,
    pub date_instant: Option<chrono::DateTime<chrono::Utc>>,
    pub status: PurchaseStatus,
    pub sub_total: Decimal,
    pub tax_rate: Decimal,
    pub tax_amount: Decimal,
    pub grand_total: Decimal,
    pub payment_status: PaymentStatus,
    pub paid_amount: Decimal,
    pub returned_amount: Decimal,
    #[sea_orm(column_type = "Text", nullable)]
    pub note: Option<String>,
    pub invoice_discount_pct: Option<Decimal>,
    pub invoice_discount_amount: Option<Decimal>,
    #[sea_orm(column_type = "Json", nullable)]
    pub landed_costs: Option<serde_json::Value>,
    pub supplier_invoice_no: Option<String>,
    pub supplier_invoice_date: Option<chrono::NaiveDate>,
    pub vat_not_recoverable: Option<bool>,
    pub sent_at: Option<chrono::DateTime<chrono::Utc>>,
    pub backorder_of_id: Option<Id>,
    /// `DocDate` triple (`m0016` G-28c): `confirmPurchaseOrder` writes an ISO instant here, which a
    /// plain `DATE` truncated.
    pub received_date_day: Option<chrono::NaiveDate>,
    pub received_date_instant: Option<chrono::DateTime<chrono::Utc>>,
    #[sea_orm(column_type = "Json", nullable)]
    pub attachment_ids: Option<StringList>,
    pub cost_center_id: Option<Id>,
    pub branch_id: Option<Id>,
    pub currency: Option<String>,
    pub exchange_rate: Option<Decimal>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_status: SyncStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::purchase_order_lines::Entity")]
    PurchaseOrderLines,
}

impl Related<super::purchase_order_lines::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PurchaseOrderLines.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }

    /// `None` when the order hasn't been received; `Some` reads `received_date_day`/
    /// `received_date_instant` back into the shared value type (`m0016` G-28c).
    pub fn received_date(&self) -> Option<DocDate> {
        self.received_date_day.map(|day| doc_date::read(day, self.received_date_instant))
    }
}
