//! `settings` — the singleton branch-DB row (branch-scoped `StoreSettings` fields only, per
//! cross-cutting.md §3; device fields live in `device-settings.json`, not this table).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::values::{
    AccountingPolicy, Address, BackupPolicy, FeatureFlags, InsightThresholds, OnboardingState, PosPolicy, PrinterSettings, RoleAccessOverrides,
    SalesPolicy,
};
use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "settings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    /// Always `1` — `ck_settings_singleton` enforces one row.
    pub singleton: i8,
    pub store_name: String,
    #[sea_orm(column_type = "custom(\"MEDIUMTEXT\")", nullable)]
    pub logo: Option<String>,
    #[sea_orm(column_type = "custom(\"MEDIUMTEXT\")", nullable)]
    pub stamp: Option<String>,
    #[sea_orm(column_type = "custom(\"MEDIUMTEXT\")", nullable)]
    pub signature: Option<String>,
    #[sea_orm(column_type = "Char(Some(3))")]
    pub currency: String,
    #[sea_orm(column_type = "Char(Some(2))", nullable)]
    pub country: Option<String>,
    pub vat_number: Option<String>,
    pub default_tax_id: Option<Id>,
    pub invoice_number_prefix: String,
    #[sea_orm(column_type = "Json")]
    pub printer: PrinterSettings,
    pub prices_include_tax: bool,
    #[sea_orm(column_type = "Text", nullable)]
    pub address: Option<String>,
    #[sea_orm(column_type = "Json", nullable)]
    pub national_address: Option<Address>,
    pub phone: Option<String>,
    pub commercial_register: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub receipt_footer: Option<String>,
    #[sea_orm(column_type = "Json", nullable)]
    pub accounting: Option<AccountingPolicy>,
    #[sea_orm(column_type = "Json", nullable)]
    pub backup: Option<BackupPolicy>,
    #[sea_orm(column_type = "Decimal(Some((19, 2)))", nullable)]
    pub inventory_approval_threshold: Option<Decimal>,
    #[sea_orm(column_type = "Json", nullable)]
    pub role_access_overrides: Option<RoleAccessOverrides>,
    #[sea_orm(column_type = "Json", nullable)]
    pub insight_thresholds: Option<InsightThresholds>,
    #[sea_orm(column_type = "Json", nullable)]
    pub pos: Option<PosPolicy>,
    #[sea_orm(column_type = "Json", nullable)]
    pub sales: Option<SalesPolicy>,
    #[sea_orm(column_type = "Json", nullable)]
    pub features: Option<FeatureFlags>,
    #[sea_orm(column_type = "Json", nullable)]
    pub onboarding: Option<OnboardingState>,
    /// P2-08: IANA timezone name — `None` falls back to the Main PC's OS timezone.
    pub timezone: Option<String>,
    /// P2-20: every settings row must resolve to a real branch.
    pub default_branch_id: Id,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match *self {}
    }
}

impl ActiveModelBehavior for ActiveModel {}
