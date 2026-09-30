//! `products` catalog DTOs (06-products.md §2) — mirrors `src/modules/products/types/index.ts:1-185`
//! field-for-field. Conventions per `core/dto.rs` header (P2-33): `camelCase`, `Option` fields
//! `skip_serializing_none` + `#[ts(optional)]` (absent, never `null`), `Decimal` via
//! `utils::money::serde_number` + `#[ts(type = "number")]`, `Id` as `#[ts(type = "string")]`.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::utils::id::Id;
use crate::utils::money::serde_number;

// --- Product / units / prices --------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "products/types/gen/")]
pub enum ProductType {
    Product,
    Service,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "products/types/gen/")]
pub enum StockMode {
    Tracked,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "products/types/gen/")]
pub enum WarrantyProvider {
    Manufacturer,
    Store,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductUnit {
    pub id: String,
    #[ts(type = "string")]
    pub unit_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub factor: Decimal,
    pub barcodes: Vec<String>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub price: Decimal,
    pub price_is_auto: bool,
    pub default_for_sale: bool,
    pub default_for_purchase: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductUnitPrice {
    #[ts(type = "string")]
    pub price_list_id: Id,
    /// The product's `ProductUnit.id` (or `'__base__'`), not a `units` row id — see the entity.
    pub unit_id: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub value: Decimal,
}

/// `Product.prices` element (`{ priceListId, value }`, `productService.ts:83`). Also reused as
/// `ProductInput`'s price line, where `value` may arrive as JSON `null` — `normalize` filters those
/// (C-6), so this DTO's `value` stays plain `Decimal` (never `null`) on both sides of the wire; the
/// request-only variant that tolerates a null/missing value is `ProductPriceInput` below.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductPrice {
    #[ts(type = "string")]
    pub price_list_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub value: Decimal,
}

/// The request-side price line: `value` may be `null`/absent in `ProductInput.prices` (the form can
/// send a cleared cell) — C-6 `normalize` drops entries whose value isn't a real number.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductPriceInput {
    #[ts(type = "string")]
    pub price_list_id: Id,
    #[serde(default, with = "serde_number::option")]
    #[ts(type = "number")]
    pub value: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct BranchStock {
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub qty: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub value: Decimal,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct Product {
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_en: Option<String>,
    pub sku: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub barcode: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<Id>,
    pub r#type: ProductType,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stock_mode: Option<StockMode>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cost_price: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub price: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub stock_qty: Decimal,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_stock: Option<Decimal>,
    pub active: bool,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prices: Option<Vec<ProductPrice>>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_account_id: Option<Id>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub stock_value: Decimal,
    #[ts(optional, type = "Record<string, import('./BranchStock').BranchStock>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stock_by_branch: Option<BTreeMap<String, BranchStock>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_ids: Option<Vec<String>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<Vec<ProductUnit>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_prices: Option<Vec<ProductUnitPrice>>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_price: Option<Decimal>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sale_tax_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_tax_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revenue_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cogs_account_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_negative_stock: Option<bool>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shelf_location: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_supplier_id: Option<Id>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reorder_qty: Option<Decimal>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_batches: Option<bool>,
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_alert_days: Option<i32>,
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warranty_months: Option<i32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warranty_provider: Option<WarrantyProvider>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<Decimal>,
    #[ts(optional, type = "Record<string, string | number | boolean | undefined>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_fields: Option<BTreeMap<String, serde_json::Value>>,
}

/// `ProductInput` (`Omit<Product, 'id'|'stockQty'|'stockValue'> & { openingQty? }`) — request-only.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductInput {
    pub name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_en: Option<String>,
    pub sku: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub barcode: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<Id>,
    pub r#type: ProductType,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stock_mode: Option<StockMode>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub cost_price: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub price: Decimal,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_stock: Option<Decimal>,
    pub active: bool,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prices: Option<Vec<ProductPriceInput>>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_account_id: Option<Id>,
    #[ts(optional, type = "Record<string, import('./BranchStock').BranchStock>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stock_by_branch: Option<BTreeMap<String, BranchStock>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_ids: Option<Vec<String>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<Vec<ProductUnit>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_prices: Option<Vec<ProductUnitPrice>>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_price: Option<Decimal>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sale_tax_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_tax_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revenue_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cogs_account_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_negative_stock: Option<bool>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shelf_location: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_supplier_id: Option<Id>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reorder_qty: Option<Decimal>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_batches: Option<bool>,
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_alert_days: Option<i32>,
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warranty_months: Option<i32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warranty_provider: Option<WarrantyProvider>,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<Decimal>,
    #[ts(optional, type = "Record<string, string | number | boolean | undefined>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_fields: Option<BTreeMap<String, serde_json::Value>>,
    /// Only honored on create — becomes an opening STOCK_IN adjustment (06 C-7).
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening_qty: Option<Decimal>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductFilter {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ProductType>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low_stock_only: Option<bool>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_inactive: Option<bool>,
}

// --- Categories / units / price lists / custom fields --------------------------------------------

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct Category {
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revenue_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cogs_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sale_tax_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_tax_id: Option<Id>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct CategoryDefaults {
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revenue_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cogs_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sale_tax_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_tax_id: Option<Id>,
}

/// `CategoryWithCount` (`Category & { productCount }`, `catalogService.ts:19`) — a flat struct (not
/// `#[serde(flatten)]`) so the generated TS type is a plain object literal `contract.check.ts`'s
/// `Equals<Gen, Flat<Category & {...}>>` can match, matching this codebase's existing convention
/// (see `domains/expenses/dto.rs`'s `ExpenseRow` doc comment for the same reasoning).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct CategoryWithCount {
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revenue_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cogs_account_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sale_tax_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_tax_id: Option<Id>,
    #[ts(type = "number")]
    pub product_count: u32,
}

impl CategoryWithCount {
    pub fn new(base: Category, product_count: u32) -> Self {
        Self {
            id: base.id,
            name: base.name,
            purchase_account_id: base.purchase_account_id,
            revenue_account_id: base.revenue_account_id,
            cogs_account_id: base.cogs_account_id,
            sale_tax_id: base.sale_tax_id,
            purchase_tax_id: base.purchase_tax_id,
            product_count,
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct Unit {
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_decimals: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct UnitExtra {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_decimals: Option<bool>,
}

/// `UnitWithCount` (`Unit & { productCount }`, `catalogService.ts:49`) — flat struct, same reasoning
/// as `CategoryWithCount` above.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct UnitWithCount {
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_decimals: Option<bool>,
    #[ts(type = "number")]
    pub product_count: u32,
}

impl UnitWithCount {
    pub fn new(base: Unit, product_count: u32) -> Self {
        Self { id: base.id, name: base.name, symbol: base.symbol, allows_decimals: base.allows_decimals, product_count }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "products/types/gen/")]
pub enum UnitPresetKind {
    Pharmacy,
    Clothing,
    Supermarket,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct PriceList {
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    pub active: bool,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct PriceListInput {
    pub name: String,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "products/types/gen/")]
pub enum CustomFieldType {
    Text,
    Number,
    Date,
    List,
    Yesno,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct CustomFieldDef {
    #[ts(type = "string")]
    pub id: Id,
    pub name: String,
    pub r#type: CustomFieldType,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    pub active: bool,
    #[ts(type = "number")]
    pub sort_order: i16,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct CustomFieldDefInput {
    pub name: String,
    pub r#type: CustomFieldType,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    pub active: bool,
}

// --- Command args (one struct per command, §3.2 convention) ---------------------------------------

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsGetProductsArgs {
    #[ts(optional)]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<ProductFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsGetProductArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsFindByCodeArgs {
    pub code: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsCreateProductArgs {
    pub input: ProductInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsUpdateProductArgs {
    pub id: String,
    pub input: ProductInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsSuggestSkuArgs {
    pub prefix: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsSaveCategoryArgs {
    pub name: String,
    #[ts(optional)]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[ts(optional)]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<CategoryDefaults>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsDeleteCategoryArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsSaveUnitArgs {
    pub name: String,
    #[ts(optional)]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[ts(optional)]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<UnitExtra>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsApplyUnitPresetArgs {
    pub kind: UnitPresetKind,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsDeleteUnitArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsSavePriceListArgs {
    pub input: PriceListInput,
    #[ts(optional)]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsDeletePriceListArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsSetPriceListValuesArgs {
    pub price_list_id: String,
    #[serde(with = "decimal_option_map")]
    #[ts(type = "Record<string, number | null>")]
    pub values: BTreeMap<String, Option<Decimal>>,
}

/// `BTreeMap<String, Option<Decimal>>` on the wire as `Record<string, number | null>` — C-12
/// `setPriceListValues` (`{ productId: price | null }`, `catalogService.ts:150`). Reuses
/// `utils::money::serde_number`'s JSON-number parsing rule per-entry (rule 5), just applied over a
/// map instead of a scalar field — kept local to this args struct rather than a second exported
/// helper in `utils::money`, since no other command needs a map of nullable decimals.
mod decimal_option_map {
    use super::*;
    use serde::Deserializer;

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<BTreeMap<String, Option<Decimal>>, D::Error> {
        let raw: BTreeMap<String, Option<serde_json::Value>> = BTreeMap::deserialize(deserializer)?;
        let mut out = BTreeMap::new();
        for (k, v) in raw {
            let parsed = match v {
                None | Some(serde_json::Value::Null) => None,
                Some(other) => Some(crate::utils::money::decimal_from_json_value(&other).map_err(serde::de::Error::custom)?),
            };
            out.insert(k, parsed);
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsSaveCustomFieldDefArgs {
    pub input: CustomFieldDefInput,
    #[ts(optional)]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsDeleteCustomFieldDefArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "products/types/gen/")]
pub struct ProductsReorderCustomFieldDefsArgs {
    pub ordered_ids: Vec<String>,
}
