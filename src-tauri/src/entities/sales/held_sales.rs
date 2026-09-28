//! `held_sales` entity (21.02-B, owner B2). Source: `HeldSale` in
//! `src/modules/invoices/types/index.ts`. Per-terminal parked cart; hard-deletable (B-1).

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::entities::values::HeldSaleCart;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

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
#[sea_orm(table_name = "held_sales")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub label: Option<String>,
    pub terminal_id: Id,
    pub held_at_day: chrono::NaiveDate,
    pub held_at_instant: Option<chrono::DateTime<chrono::Utc>>,
    pub held_by: Id,
    pub customer_id: Option<Id>,
    pub discount_rate: Decimal,
    pub discount_is_pct: bool,
    #[sea_orm(column_type = "Text", nullable)]
    pub note: Option<String>,
    #[sea_orm(column_type = "Json")]
    pub cart: HeldSaleCart,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub sync_status: SyncStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn held_at(&self) -> DocDate {
        doc_date::read(self.held_at_day, self.held_at_instant)
    }
}
