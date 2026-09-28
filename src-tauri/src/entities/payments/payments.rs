//! `payments` entity (21.02-B, owner B2). Source: `Payment` in
//! `src/modules/payments/types/index.ts`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "type")]
pub enum PaymentType {
    #[sea_orm(string_value = "RECEIVED")]
    Received,
    #[sea_orm(string_value = "PAID")]
    Paid,
}

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "target_type")]
pub enum PaymentTargetType {
    #[sea_orm(string_value = "customer")]
    Customer,
    #[sea_orm(string_value = "supplier")]
    Supplier,
}

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "method")]
pub enum PaymentMethodKind {
    #[sea_orm(string_value = "cash")]
    Cash,
    #[sea_orm(string_value = "card")]
    Card,
    #[sea_orm(string_value = "bank_transfer")]
    BankTransfer,
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
#[sea_orm(table_name = "payments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub number: String,
    pub date_day: chrono::NaiveDate,
    pub date_instant: Option<chrono::DateTime<chrono::Utc>>,
    pub r#type: PaymentType,
    pub target_type: PaymentTargetType,
    // Polymorphic (B-1): no FK.
    pub target_id: Id,
    pub target_ref: Option<String>,
    pub target_ref_number: Option<String>,
    pub amount: Decimal,
    pub method: PaymentMethodKind,
    #[sea_orm(column_type = "Text", nullable)]
    pub note: Option<String>,
    pub branch_id: Option<Id>,
    pub currency: Option<String>,
    pub amount_fc: Option<Decimal>,
    pub rate: Option<Decimal>,
    pub fx_gain_loss: Option<Decimal>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_status: SyncStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::payment_allocations::Entity")]
    PaymentAllocations,
}

impl Related<super::payment_allocations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PaymentAllocations.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }
}
