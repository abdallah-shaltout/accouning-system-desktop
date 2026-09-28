//! `accounts` (accounting/types `Account`). Soft-delete table (B-1 list). No dedicated `accounting`
//! entity group folder was pre-created for phase B (only `org, catalog, inventory, parties` exist
//! under B1's scope) — filed under `org` since chart-of-accounts setup is organizational/company
//! setup data, same category as branches/currencies/settings. `crate::entities::accounts::Entity`
//! still works either way via `entities/mod.rs`'s flat re-export.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

use crate::entities::soft_delete::{self, SoftDelete};
use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "accounts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub code: String,
    pub name: String,
    pub name_en: Option<String>,
    pub parent_id: Option<Id>,
    pub is_group: bool,
    /// `'ASSET' | 'LIABILITY' | 'EQUITY' | 'REVENUE' | 'EXPENSE'`.
    pub kind: String,
    /// `AccountSubtype` — see `accounting/types/index.ts`.
    pub subtype: String,
    /// `'DEBIT' | 'CREDIT'`.
    pub normal_side: String,
    /// `SystemRole` — nullable, several accounts may share a role (C-15, cash/bank).
    pub system_role: Option<String>,
    #[sea_orm(column_type = "Char(Some(3))", nullable)]
    pub currency: Option<String>,
    pub branch_id: Option<Id>,
    pub requires_party: Option<bool>,
    pub allow_manual: bool,
    pub requires_cost_center: Option<bool>,
    pub active: bool,
    pub can_delete: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub sync_status: String,
    /// Generated (`m0016` G-17): `code` while live, `NULL` once soft-deleted — backs
    /// `uq_accounts_code_live` so a deleted account's code can be reused. Read-only; nothing ever
    /// `Set`s it directly.
    pub code_live: Option<String>,
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
