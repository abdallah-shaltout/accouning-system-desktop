//! `currencies` (settings/types/dimensions.ts `Currency`). PK is the ISO `code` itself (CHAR(3)) —
//! the TS type has no `id` field (B-2 correction to the B-1 PK rule, recorded in the migration's
//! own doc comment).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "currencies")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Char(Some(3))")]
    pub code: String,
    pub name_ar: String,
    pub symbol: String,
    pub decimals: i32,
    pub active: bool,
    pub fixed: Option<bool>,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))", nullable)]
    pub fixed_rate: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::exchange_rates::Entity")]
    ExchangeRates,
}

impl Related<super::exchange_rates::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExchangeRates.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
