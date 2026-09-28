//! `shift_movements` entity (21.02-B, owner B2). Child of `shifts`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "kind")]
pub enum ShiftMovementKind {
    #[sea_orm(string_value = "SALE_CASH")]
    SaleCash,
    #[sea_orm(string_value = "REFUND_CASH")]
    RefundCash,
    #[sea_orm(string_value = "PAY_IN")]
    PayIn,
    #[sea_orm(string_value = "PAY_OUT")]
    PayOut,
    #[sea_orm(string_value = "BANK_DROP")]
    BankDrop,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "shift_movements")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub shift_id: Id,
    pub position: i16,
    pub kind: ShiftMovementKind,
    pub amount: Decimal,
    #[sea_orm(column_type = "Text", nullable)]
    pub note: Option<String>,
    pub ref_id: Option<Id>,
    pub ref_number: Option<String>,
    pub at_day: chrono::NaiveDate,
    pub at_instant: Option<chrono::DateTime<chrono::Utc>>,
    pub by: Id,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::shifts::Entity",
        from = "Column::ShiftId",
        to = "super::shifts::Column::Id"
    )]
    Shift,
}

impl Related<super::shifts::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Shift.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn at(&self) -> DocDate {
        doc_date::read(self.at_day, self.at_instant)
    }
}
