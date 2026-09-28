//! `custom_field_defs` (products/types `CustomFieldDef`). Soft-delete table (B-1 list).

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::entities::soft_delete::{self, SoftDelete};
use crate::utils::id::Id;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, sea_orm::FromJsonQueryResult)]
#[serde(transparent)]
pub struct FieldOptions(pub Vec<String>);

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "custom_field_defs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub name: String,
    /// `'text' | 'number' | 'date' | 'list' | 'yesno'`.
    pub r#type: String,
    #[sea_orm(column_type = "Json", nullable)]
    pub options: Option<FieldOptions>,
    pub active: bool,
    pub sort_order: i16,
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
