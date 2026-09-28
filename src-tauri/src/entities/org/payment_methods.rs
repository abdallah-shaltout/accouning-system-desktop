//! `payment_methods` (settings/types `PaymentMethod`). Soft-delete table (B-1 list).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::entities::soft_delete::{self, SoftDelete};
use crate::utils::id::Id;

/// `PaymentMethod.branchOverrides` — `{ branchId, accountId }[]`. Inert (P2-… no per-branch UI
/// yet, per the TS type's own doc comment) but stored as-is for forward compatibility.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sea_orm::FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct BranchOverride {
    pub branch_id: Id,
    pub account_id: Id,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, sea_orm::FromJsonQueryResult)]
#[serde(transparent)]
pub struct BranchOverrides(pub Vec<BranchOverride>);

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "payment_methods")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub name: String,
    /// `'cash' | 'card' | 'bank_transfer' | 'wallet' | 'credit' | 'store_credit'`.
    pub r#type: String,
    pub icon: Option<String>,
    /// `'cash' | 'bank' | 'cardClearing' | 'walletClearing' | 'receivable'`.
    pub account_role: String,
    #[sea_orm(column_type = "Decimal(Some((9, 4)))")]
    pub fee_pct: Decimal,
    pub requires_reference: Option<bool>,
    pub show_in_pos: bool,
    pub show_in_payments: bool,
    pub sort_order: i16,
    #[sea_orm(column_type = "Json", nullable)]
    pub branch_overrides: Option<BranchOverrides>,
    pub active: bool,
    pub can_delete: bool,
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
