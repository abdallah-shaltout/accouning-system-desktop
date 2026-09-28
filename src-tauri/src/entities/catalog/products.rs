//! `products` (products/types `Product`). Soft-delete table (B-1 list). `search_normalized` is one
//! of the 4 tables filled by the write path (P2-38/B-6/B-7, owned by whichever Part 03 controller
//! writes products — this entity only declares the column).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::entities::soft_delete::{self, SoftDelete};
use crate::entities::values::StringList;
use crate::utils::id::Id;

/// `Product.units` — one row per `ProductUnit` (id/unitId/factor/barcodes/price/priceIsAuto/
/// defaultForSale/defaultForPurchase/active). Small, never independently queried by a dedicated
/// reader (unlike `product_prices`/`product_branch_stock`), so it's JSON per B-1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductUnit {
    pub id: String,
    pub unit_id: Id,
    #[serde(with = "crate::utils::money::serde_number")]
    pub factor: Decimal,
    pub barcodes: Vec<String>,
    #[serde(with = "crate::utils::money::serde_number")]
    pub price: Decimal,
    pub price_is_auto: bool,
    pub default_for_sale: bool,
    pub default_for_purchase: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, sea_orm::FromJsonQueryResult)]
#[serde(transparent)]
pub struct ProductUnits(pub Vec<ProductUnit>);

/// `Product.unitPrices` — per price-list × unit override.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductUnitPrice {
    pub price_list_id: Id,
    pub unit_id: Id,
    #[serde(with = "crate::utils::money::serde_number")]
    pub value: Decimal,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, sea_orm::FromJsonQueryResult)]
#[serde(transparent)]
pub struct ProductUnitPrices(pub Vec<ProductUnitPrice>);

/// `Product.stockByBranch` — `Record<branchId, { qty, value }>` (v2 phase 9, undefined/empty on a
/// single-branch company).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchStockEntry {
    #[serde(with = "crate::utils::money::serde_number")]
    pub qty: Decimal,
    #[serde(with = "crate::utils::money::serde_number")]
    pub value: Decimal,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, sea_orm::FromJsonQueryResult)]
#[serde(transparent)]
pub struct StockByBranch(pub std::collections::BTreeMap<String, BranchStockEntry>);

/// `Product.customFields` — `Record<fieldDefId, string | number | boolean | undefined>`, an open
/// map keyed by `CustomFieldDef.id` (schema lives in `custom_field_defs`, not here).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, sea_orm::FromJsonQueryResult)]
#[serde(transparent)]
pub struct CustomFieldValues(pub std::collections::BTreeMap<String, serde_json::Value>);

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "products")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub name: String,
    pub name_en: Option<String>,
    pub sku: String,
    pub barcode: Option<String>,
    pub category_id: Option<Id>,
    pub unit_id: Option<Id>,
    /// `'product' | 'service'`.
    pub r#type: String,
    /// `'tracked' | 'none'` — `None` (SQL NULL) means "tracked" (legacy default).
    pub stock_mode: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))")]
    pub cost_price: Decimal,
    #[sea_orm(column_type = "Decimal(Some((19, 2)))")]
    pub price: Decimal,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))")]
    pub stock_qty: Decimal,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))", nullable)]
    pub min_stock: Option<Decimal>,
    pub active: bool,
    #[sea_orm(column_type = "custom(\"MEDIUMTEXT\")", nullable)]
    pub image: Option<String>,
    pub purchase_account_id: Option<Id>,
    #[sea_orm(column_type = "Decimal(Some((19, 2)))")]
    pub stock_value: Decimal,
    #[sea_orm(column_type = "Json", nullable)]
    pub stock_by_branch: Option<StockByBranch>,
    pub brand: Option<String>,
    #[sea_orm(column_type = "Json", nullable)]
    pub tags: Option<StringList>,
    #[sea_orm(column_type = "Json", nullable)]
    pub image_ids: Option<StringList>,
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    #[sea_orm(column_type = "Json", nullable)]
    pub units: Option<ProductUnits>,
    #[sea_orm(column_type = "Json", nullable)]
    pub unit_prices: Option<ProductUnitPrices>,
    #[sea_orm(column_type = "Decimal(Some((19, 2)))", nullable)]
    pub min_price: Option<Decimal>,
    pub sale_tax_id: Option<Id>,
    pub purchase_tax_id: Option<Id>,
    pub revenue_account_id: Option<Id>,
    pub cogs_account_id: Option<Id>,
    pub allow_negative_stock: Option<bool>,
    pub shelf_location: Option<String>,
    pub preferred_supplier_id: Option<Id>,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))", nullable)]
    pub reorder_qty: Option<Decimal>,
    pub track_batches: Option<bool>,
    pub expiry_alert_days: Option<i32>,
    pub warranty_months: Option<i32>,
    /// `'manufacturer' | 'store'`.
    pub warranty_provider: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((19, 4)))", nullable)]
    pub weight: Option<Decimal>,
    #[sea_orm(column_type = "Json", nullable)]
    pub custom_fields: Option<CustomFieldValues>,
    #[sea_orm(column_type = "Text", nullable)]
    pub search_normalized: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub sync_status: String,
    #[sea_orm(column_name = "sku_live")]
    pub sku_live: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::categories::Entity", from = "Column::CategoryId", to = "super::categories::Column::Id")]
    Category,
    #[sea_orm(has_many = "super::product_prices::Entity")]
    ProductPrices,
    #[sea_orm(has_many = "super::product_branch_stock::Entity")]
    BranchStock,
    #[sea_orm(has_many = "super::product_batches::Entity")]
    Batches,
}

impl Related<super::categories::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Category.def()
    }
}

impl Related<super::product_prices::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductPrices.def()
    }
}

impl Related<super::product_branch_stock::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BranchStock.def()
    }
}

impl Related<super::product_batches::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Batches.def()
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    /// B-6/B-7 (P2-38): recomputes `search_normalized` whenever any of the searched fields
    /// (`name`, `sku`, `barcode` — the exact set `ProductListPage.vue`'s `matchesSearch([p.name,
    /// p.sku, p.barcode], search.value)` uses today) is `Set`. A field left `Unchanged`/`NotSet`
    /// on a partial update is read from `self` via `try_as_ref()` (works for both `Set` and
    /// `Unchanged`), so the haystack always reflects the row's current values, not just the ones
    /// this particular update touched — the strict "recompute from full row state" choice, not
    /// "only recompute when literally every field is present".
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, sea_orm::DbErr>
    where
        C: sea_orm::ConnectionTrait,
    {
        let touched = self.name.is_set() || self.sku.is_set() || self.barcode.is_set();
        if touched {
            let name = self.name.try_as_ref().map(|s| s.as_str());
            let sku = self.sku.try_as_ref().map(|s| s.as_str());
            let barcode = self.barcode.try_as_ref().and_then(|o| o.as_deref());
            self.search_normalized = sea_orm::ActiveValue::Set(Some(crate::utils::text::search_haystack(&[name, sku, barcode])));
        }
        Ok(self)
    }
}

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
