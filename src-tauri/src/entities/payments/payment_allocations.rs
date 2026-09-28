//! `payment_allocations` entity (21.02-B, owner B2). Child of `payments`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "target_kind")]
pub enum PaymentAllocationTargetKind {
    #[sea_orm(string_value = "invoice")]
    Invoice,
    #[sea_orm(string_value = "purchaseOrder")]
    PurchaseOrder,
    #[sea_orm(string_value = "opening")]
    Opening,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "payment_allocations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub payment_id: Id,
    pub position: i16,
    pub target_kind: PaymentAllocationTargetKind,
    // Polymorphic (B-1): no FK.
    pub target_id: Id,
    pub target_number: String,
    pub amount: Decimal,
    pub date_day: chrono::NaiveDate,
    pub date_instant: Option<chrono::DateTime<chrono::Utc>>,
    pub amount_fc: Option<Decimal>,
    pub fx_gain_loss: Option<Decimal>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::payments::Entity",
        from = "Column::PaymentId",
        to = "super::payments::Column::Id"
    )]
    Payment,
}

impl Related<super::payments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Payment.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }
}
