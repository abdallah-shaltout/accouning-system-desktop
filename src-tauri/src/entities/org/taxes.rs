//! `taxes` (settings/types `Tax`). Soft-delete table (B-1 list).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::soft_delete::{self, SoftDelete};
use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "taxes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub name: String,
    #[sea_orm(column_type = "Decimal(Some((9, 4)))")]
    pub rate: Decimal,
    /// v1 legacy: `'OUTPUT' | 'INPUT'`.
    pub r#type: String,
    pub is_default: bool,
    pub active: bool,
    /// ZATCA category: `'S' | 'Z' | 'E' | 'O'`.
    pub category: String,
    /// `'sales' | 'purchase'`.
    pub direction: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub exemption_reason: Option<String>,
    /// `'vatOutput' | 'vatInput'`.
    pub account_role: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub sync_status: String,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match *self {}
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl SoftDelete for Entity {
    fn id_column() -> Self::Column {
        Column::Id
    }

    fn deleted_at_column() -> Self::Column {
        Column::DeletedAt
    }

    fn deleted_at_active_model(at: Option<DateTime<Utc>>) -> Self::ActiveModel {
        ActiveModel { deleted_at: soft_delete::deleted_at_value(at), ..Default::default() }
    }
}
