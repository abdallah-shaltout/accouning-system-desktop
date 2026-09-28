//! `exchange_rates` (settings/types/dimensions.ts `ExchangeRate`).

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "exchange_rates")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    #[sea_orm(column_type = "Char(Some(3))")]
    pub currency: String,
    pub date: NaiveDate,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub rate: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::currencies::Entity", from = "Column::Currency", to = "super::currencies::Column::Code")]
    Currency,
}

impl Related<super::currencies::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Currency.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
