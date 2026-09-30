//! `domains::products::service::products` — the catalog product reads/writes (06-products.md §3,
//! C-1…C-9). C-7 `create_product`'s opening-stock step calls 06b's `record_stock_adjustment`
//! (`service::adjustments`), completed per the plan's required order (06 §9 C1-C9 -> 06b §9 all ->
//! 06 §9 C10).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::{map_unique_violation, AppError};
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::catalog::product_branch_stock;
use crate::entities::catalog::product_prices;
use crate::entities::catalog::products::{self, ActiveModel as ProductActiveModel, Column as ProductColumn, CustomFieldValues, Entity as ProductEntity, ProductUnitPrices, ProductUnits};
use crate::entities::values::StringList;
use crate::shared::activity;
use crate::shared::stock;
use crate::utils::id::Id;
use crate::utils::route::RouteRef;
use crate::utils::text::{like_contains, normalize_arabic};

use super::super::dto::catalog::{
    BranchStock, Product, ProductFilter, ProductInput, ProductPrice, ProductPriceInput, ProductType, ProductUnit as DtoProductUnit,
    ProductUnitPrice as DtoProductUnitPrice, StockMode, WarrantyProvider,
};

// --- DTO assembly (G-39: also reused by 07-purchases/14-analytics) --------------------------------

fn parse_product_type(s: &str) -> ProductType {
    match s {
        "product" => ProductType::Product,
        "service" => ProductType::Service,
        other => panic!("products.type column holds an unrecognized value: {other:?}"),
    }
}

pub(crate) fn product_type_as_str(t: ProductType) -> &'static str {
    match t {
        ProductType::Product => "product",
        ProductType::Service => "service",
    }
}

fn parse_stock_mode(s: Option<&str>) -> Option<StockMode> {
    match s {
        None => None,
        Some("tracked") => Some(StockMode::Tracked),
        Some("none") => Some(StockMode::None),
        Some(other) => panic!("products.stock_mode column holds an unrecognized value: {other:?}"),
    }
}

fn stock_mode_as_str(m: Option<StockMode>) -> Option<&'static str> {
    match m {
        None => None,
        Some(StockMode::Tracked) => Some("tracked"),
        Some(StockMode::None) => Some("none"),
    }
}

fn parse_warranty_provider(s: Option<&str>) -> Option<WarrantyProvider> {
    match s {
        None => None,
        Some("manufacturer") => Some(WarrantyProvider::Manufacturer),
        Some("store") => Some(WarrantyProvider::Store),
        Some(other) => panic!("products.warranty_provider column holds an unrecognized value: {other:?}"),
    }
}

fn warranty_provider_as_str(p: Option<WarrantyProvider>) -> Option<&'static str> {
    match p {
        None => None,
        Some(WarrantyProvider::Manufacturer) => Some("manufacturer"),
        Some(WarrantyProvider::Store) => Some("store"),
    }
}

fn to_dto_unit(u: &products::ProductUnit) -> DtoProductUnit {
    DtoProductUnit {
        id: u.id.clone(),
        unit_id: u.unit_id,
        factor: u.factor,
        barcodes: u.barcodes.clone(),
        price: u.price,
        price_is_auto: u.price_is_auto,
        default_for_sale: u.default_for_sale,
        default_for_purchase: u.default_for_purchase,
        active: u.active,
    }
}

fn from_dto_unit(u: &DtoProductUnit) -> products::ProductUnit {
    products::ProductUnit {
        id: u.id.clone(),
        unit_id: u.unit_id,
        factor: u.factor,
        barcodes: u.barcodes.clone(),
        price: u.price,
        price_is_auto: u.price_is_auto,
        default_for_sale: u.default_for_sale,
        default_for_purchase: u.default_for_purchase,
        active: u.active,
    }
}

fn to_dto_unit_price(p: &products::ProductUnitPrice) -> DtoProductUnitPrice {
    DtoProductUnitPrice { price_list_id: p.price_list_id, unit_id: p.unit_id.clone(), value: p.value }
}

fn from_dto_unit_price(p: &DtoProductUnitPrice) -> products::ProductUnitPrice {
    products::ProductUnitPrice { price_list_id: p.price_list_id, unit_id: p.unit_id.clone(), value: p.value }
}

/// Assembles the `Product` DTO for one row: `prices` from `product_prices` (unit_id NULL,
/// `created_at, id` order — always `Some(vec)`, possibly empty, Q-7), `stock_by_branch` from
/// `product_branch_stock` (absent when none).
pub async fn product_dto<C: ConnectionTrait>(conn: &C, m: &products::Model) -> TxResult<Product> {
    let mut map = product_dtos(conn, std::slice::from_ref(m)).await?;
    Ok(map.remove(&m.id).expect("product_dtos must return an entry for every input row"))
}

/// Batch variant: one `IN` query per child table for a list of products (C-1/C-2 lists).
pub async fn product_dtos<C: ConnectionTrait>(conn: &C, models: &[products::Model]) -> TxResult<BTreeMap<Id, Product>> {
    let ids: Vec<Id> = models.iter().map(|m| m.id).collect();

    let price_rows = if ids.is_empty() {
        Vec::new()
    } else {
        product_prices::Entity::find()
            .filter(product_prices::Column::ProductId.is_in(ids.clone()))
            .filter(product_prices::Column::UnitId.is_null())
            .order_by_asc(product_prices::Column::CreatedAt)
            .order_by_asc(product_prices::Column::Id)
            .all(conn)
            .await
            .map_err(AppError::from)?
    };
    let mut prices_by_product: BTreeMap<Id, Vec<ProductPrice>> = BTreeMap::new();
    for row in price_rows {
        prices_by_product.entry(row.product_id).or_default().push(ProductPrice { price_list_id: row.price_list_id, value: row.value });
    }

    let branch_rows = if ids.is_empty() {
        Vec::new()
    } else {
        product_branch_stock::Entity::find().filter(product_branch_stock::Column::ProductId.is_in(ids.clone())).all(conn).await.map_err(AppError::from)?
    };
    let mut branch_by_product: BTreeMap<Id, BTreeMap<String, BranchStock>> = BTreeMap::new();
    for row in branch_rows {
        branch_by_product
            .entry(row.product_id)
            .or_default()
            .insert(row.branch_id.to_string(), BranchStock { qty: row.qty, value: row.value });
    }

    let mut out = BTreeMap::new();
    for m in models {
        let prices = prices_by_product.remove(&m.id).unwrap_or_default();
        let stock_by_branch = branch_by_product.remove(&m.id);
        let dto = Product {
            id: m.id,
            name: m.name.clone(),
            name_en: m.name_en.clone(),
            sku: m.sku.clone(),
            barcode: m.barcode.clone(),
            category_id: m.category_id,
            unit_id: m.unit_id,
            r#type: parse_product_type(&m.r#type),
            stock_mode: parse_stock_mode(m.stock_mode.as_deref()),
            cost_price: m.cost_price,
            price: m.price,
            stock_qty: m.stock_qty,
            min_stock: m.min_stock,
            active: m.active,
            image: m.image.clone(),
            prices: Some(prices),
            purchase_account_id: m.purchase_account_id,
            stock_value: m.stock_value,
            stock_by_branch,
            brand: m.brand.clone(),
            tags: m.tags.clone().map(|l| l.0),
            image_ids: m.image_ids.clone().map(|l| l.0),
            description: m.description.clone(),
            units: m.units.clone().map(|u| u.0.iter().map(to_dto_unit).collect()),
            unit_prices: m.unit_prices.clone().map(|u| u.0.iter().map(to_dto_unit_price).collect()),
            min_price: m.min_price,
            sale_tax_id: m.sale_tax_id,
            purchase_tax_id: m.purchase_tax_id,
            revenue_account_id: m.revenue_account_id,
            cogs_account_id: m.cogs_account_id,
            allow_negative_stock: m.allow_negative_stock,
            shelf_location: m.shelf_location.clone(),
            preferred_supplier_id: m.preferred_supplier_id,
            reorder_qty: m.reorder_qty,
            track_batches: m.track_batches,
            expiry_alert_days: m.expiry_alert_days,
            warranty_months: m.warranty_months,
            warranty_provider: parse_warranty_provider(m.warranty_provider.as_deref()),
            weight: m.weight,
            custom_fields: m.custom_fields.clone().map(|c| c.0),
        };
        out.insert(m.id, dto);
    }
    Ok(out)
}

// --- C-1 get_products -------------------------------------------------------------------------

/// **`get_products(filter)`** (`productService.ts:14-26`): `WHERE (include_inactive OR active) [AND
/// category_id = ?] [AND type = ?] [AND the low-stock predicate] [AND search_normalized LIKE ...]`.
pub async fn get_products<C: ConnectionTrait>(conn: &C, filter: Option<ProductFilter>) -> TxResult<Vec<Product>> {
    let filter = filter.unwrap_or(ProductFilter { search: None, category_id: None, r#type: None, low_stock_only: None, include_inactive: None });

    let mut query = ProductEntity::find();
    if !filter.include_inactive.unwrap_or(false) {
        query = query.filter(ProductColumn::Active.eq(true));
    }
    if let Some(category_id) = filter.category_id {
        query = query.filter(ProductColumn::CategoryId.eq(category_id));
    }
    if let Some(t) = filter.r#type {
        query = query.filter(ProductColumn::Type.eq(product_type_as_str(t)));
    }
    if let Some(search) = filter.search.as_deref().filter(|s| !s.trim().is_empty()) {
        let needle = normalize_arabic(Some(search));
        if !needle.is_empty() {
            query = query.filter(ProductColumn::SearchNormalized.like(like_contains(&needle)));
        }
    }

    let mut rows = query.order_by_asc(ProductColumn::CreatedAt).order_by_asc(ProductColumn::Id).all(conn).await.map_err(AppError::from)?;

    // `isLowStock` (`productService.ts:10-12`): type='product' && stockMode !== 'none' && stockQty
    // <= (minStock ?? 0) — applied in Rust (not SQL) so the exact JS predicate (including the
    // `minStock ?? 0` default) is never approximated by a raw-SQL COALESCE.
    if filter.low_stock_only.unwrap_or(false) {
        rows.retain(|p| p.r#type == "product" && p.stock_mode.as_deref() != Some("none") && p.stock_qty <= p.min_stock.unwrap_or(Decimal::ZERO));
    }

    let map = product_dtos(conn, &rows).await?;
    Ok(rows.iter().map(|r| map.get(&r.id).cloned().expect("dto for every row")).collect())
}

// --- C-2 get_product --------------------------------------------------------------------------

pub async fn get_product<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<Product> {
    let row = ProductEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("المنتج غير موجود"))?;
    product_dto(conn, &row).await
}

// --- C-3 find_by_code -------------------------------------------------------------------------

/// **`find_by_code(code)`** (`:36-41`): first row (by `created_at, id`) with `active AND (barcode =
/// c OR LOWER(sku) = LOWER(c))`; `c = code.trim()`. Unit barcodes are not searched (Q-1).
pub async fn find_by_code<C: ConnectionTrait>(conn: &C, code: &str) -> TxResult<Option<Product>> {
    // No empty-code special case: the mock doesn't have one either (`code.trim()` compared as-is,
    // `productService.ts:38-39`) — a blank trimmed code simply matches nothing, since validation
    // never allows an empty SKU/barcode to exist in the table.
    let c = code.trim();
    let rows = ProductEntity::find()
        .filter(ProductColumn::Active.eq(true))
        .order_by_asc(ProductColumn::CreatedAt)
        .order_by_asc(ProductColumn::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let needle = c.to_lowercase();
    let found = rows.into_iter().find(|p| p.barcode.as_deref() == Some(c) || p.sku.to_lowercase() == needle);
    match found {
        Some(row) => Ok(Some(product_dto(conn, &row).await?)),
        None => Ok(None),
    }
}

// --- C-4 generate_ean13 -----------------------------------------------------------------------

/// **`generate_ean13()`** (`:109-121`): up to 20 attempts; free when no row's `barcode` equals it and
/// no row's `units` JSON contains it in any unit's `barcodes`.
pub async fn generate_ean13<C: ConnectionTrait>(conn: &C) -> TxResult<String> {
    for _ in 0..20 {
        let mut rand_bytes = [0u8; 4];
        getrandom::fill(&mut rand_bytes).map_err(|e| AppError::internal("تعذر توليد رقم عشوائي", Some(e.to_string())))?;
        let n = u32::from_le_bytes(rand_bytes) % 1_000_000_000;
        let body_str = format!("628{n:09}");
        let digits: Vec<u32> = body_str.chars().map(|c| c.to_digit(10).unwrap()).collect();
        let total: u32 = digits.iter().enumerate().map(|(i, d)| d * if i % 2 == 0 { 1 } else { 3 }).sum();
        let check = (10 - (total % 10)) % 10;
        let code = format!("{body_str}{check}");

        let by_barcode = ProductEntity::find().filter(ProductColumn::Barcode.eq(code.clone())).one(conn).await.map_err(AppError::from)?;
        if by_barcode.is_some() {
            continue;
        }
        // Unit barcodes live in a JSON column — `JSON_SEARCH` via raw SQL (no ORM support for it).
        let stmt = sea_orm::Statement::from_sql_and_values(
            conn.get_database_backend(),
            "SELECT id FROM products WHERE JSON_SEARCH(units, 'one', ?, NULL, '$[*].barcodes[*]') IS NOT NULL LIMIT 1",
            [code.clone().into()],
        );
        let hit = conn.query_one(stmt).await.map_err(AppError::from)?;
        if hit.is_none() {
            return Ok(code);
        }
    }
    Err(AppError::validation("تعذر توليد باركود فريد — حاول مرة أخرى").into())
}

// --- C-5 validate / C-6 normalize -----------------------------------------------------------------

/// A subset of a locked/loaded product the validation and cost-guard steps need — built from either
/// a plain `products::Model` (create: `None`) or the existing row (update).
pub struct ExistingProduct<'a> {
    pub stock_qty: Decimal,
    pub units: Option<&'a [products::ProductUnit]>,
}

/// **C-5 `validate`** (`productService.ts:68-88` then `validateUnits` `:48-66`), exact order.
pub async fn validate<C: ConnectionTrait>(conn: &C, input: &ProductInput, except_id: Option<Id>, existing: Option<&ExistingProduct<'_>>) -> TxResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::validation("اسم المنتج مطلوب").into());
    }
    if input.sku.trim().is_empty() {
        return Err(AppError::validation("رمز المنتج (SKU) مطلوب").into());
    }

    let sku_needle = input.sku.trim().to_lowercase();
    let mut others = ProductEntity::find().all(conn).await.map_err(AppError::from)?;
    if let Some(except) = except_id {
        others.retain(|p| p.id != except);
    }
    if others.iter().any(|p| p.sku.to_lowercase() == sku_needle) {
        return Err(AppError::conflict("رمز المنتج مستخدم لمنتج آخر").into());
    }
    if let Some(barcode) = input.barcode.as_deref().filter(|b| !b.is_empty()) {
        if others.iter().any(|p| p.barcode.as_deref() == Some(barcode)) {
            return Err(AppError::conflict("الباركود مستخدم لمنتج آخر").into());
        }
    }
    // Other products' unit barcodes (Q-1: main barcode and unit barcodes are checked separately).
    let mut other_unit_barcodes: std::collections::HashSet<String> = std::collections::HashSet::new();
    for p in &others {
        if let Some(units) = &p.units {
            for u in &units.0 {
                for bc in &u.barcodes {
                    other_unit_barcodes.insert(bc.clone());
                }
            }
        }
    }
    if let Some(units) = &input.units {
        for u in units {
            for bc in &u.barcodes {
                if !bc.is_empty() && other_unit_barcodes.contains(bc) {
                    return Err(AppError::conflict(format!("الباركود \"{bc}\" مستخدم في منتج آخر")).into());
                }
            }
        }
    }

    if input.price < Decimal::ZERO || input.cost_price < Decimal::ZERO {
        return Err(AppError::validation("الأسعار لا يمكن أن تكون سالبة").into());
    }
    if let Some(min_price) = input.min_price {
        if min_price > input.price {
            return Err(AppError::validation("الحد الأدنى للسعر أكبر من سعر البيع").into());
        }
    }

    validate_units(input.units.as_deref(), existing)?;
    Ok(())
}

/// `validateUnits` (`:48-66`).
fn validate_units(units: Option<&[DtoProductUnit]>, existing: Option<&ExistingProduct<'_>>) -> TxResult<()> {
    let Some(units) = units.filter(|u| !u.is_empty()) else { return Ok(()) };

    for u in units {
        if !(u.factor > Decimal::ZERO) {
            return Err(AppError::validation("عامل تحويل الوحدة يجب أن يكون أكبر من صفر").into());
        }
    }
    let base_count = units.iter().filter(|u| u.factor == Decimal::ONE).count();
    if base_count != 1 {
        return Err(AppError::validation("يجب أن تكون وحدة واحدة فقط بعامل تحويل = 1 (الوحدة الأساسية)").into());
    }

    if let Some(existing) = existing {
        if existing.stock_qty > Decimal::new(1, 4) {
            if let Some(old_units) = existing.units {
                for old_unit in old_units {
                    if let Some(still_there) = units.iter().find(|u| u.id == old_unit.id) {
                        if still_there.factor != old_unit.factor {
                            return Err(AppError::validation(format!(
                                "لا يمكن تغيير عامل تحويل وحدة \"{}\" بعد تحرك المخزون — أضف وحدة جديدة وعطّل القديمة بدلاً من ذلك",
                                old_unit.unit_id
                            ))
                            .into());
                        }
                    }
                }
            }
        }
    }

    let mut seen = std::collections::HashSet::new();
    for u in units {
        for bc in &u.barcodes {
            if bc.is_empty() {
                continue;
            }
            if !seen.insert(bc.clone()) {
                return Err(AppError::validation("نفس الباركود مستخدم أكثر من مرة في وحدات هذا المنتج").into());
            }
        }
    }
    Ok(())
}

/// The normalized fields C-6 `normalize` produces (`opening_qty`/`stock_by_branch` dropped).
pub struct Normalized {
    pub name: String,
    pub name_en: Option<String>,
    pub sku: String,
    pub barcode: Option<String>,
    pub category_id: Option<Id>,
    pub unit_id: Option<Id>,
    pub r#type: ProductType,
    pub stock_mode: Option<StockMode>,
    pub cost_price: Decimal,
    pub price: Decimal,
    pub min_stock: Option<Decimal>,
    pub active: bool,
    pub image: Option<String>,
    pub prices: Vec<ProductPrice>,
    pub purchase_account_id: Option<Id>,
    pub brand: Option<String>,
    pub tags: Option<Vec<String>>,
    pub image_ids: Option<Vec<String>>,
    pub description: Option<String>,
    pub units: Option<Vec<DtoProductUnit>>,
    pub unit_prices: Option<Vec<DtoProductUnitPrice>>,
    pub min_price: Option<Decimal>,
    pub sale_tax_id: Option<Id>,
    pub purchase_tax_id: Option<Id>,
    pub revenue_account_id: Option<Id>,
    pub cogs_account_id: Option<Id>,
    pub allow_negative_stock: Option<bool>,
    pub shelf_location: Option<String>,
    pub preferred_supplier_id: Option<Id>,
    pub reorder_qty: Option<Decimal>,
    pub track_batches: Option<bool>,
    pub expiry_alert_days: Option<i32>,
    pub warranty_months: Option<i32>,
    pub warranty_provider: Option<WarrantyProvider>,
    pub weight: Option<Decimal>,
    pub custom_fields: Option<BTreeMap<String, serde_json::Value>>,
}

fn normalize_price(p: &ProductPriceInput) -> Option<ProductPrice> {
    p.value.map(|v| ProductPrice { price_list_id: p.price_list_id, value: v })
}

/// **C-6 `normalize`** (`:90-106`).
pub fn normalize(input: &ProductInput) -> Normalized {
    let is_service = matches!(input.r#type, ProductType::Service);
    Normalized {
        name: input.name.trim().to_string(),
        name_en: input.name_en.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string),
        sku: input.sku.trim().to_string(),
        barcode: input.barcode.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string),
        category_id: input.category_id,
        unit_id: input.unit_id,
        r#type: input.r#type,
        stock_mode: input.stock_mode,
        // `costPrice: fields.type === 'service' ? fields.costPrice ?? 0 : fields.costPrice`
        // (`productService.ts:101`) — `costPrice` is a required `Decimal` field on the wire, so the
        // mock's `?? 0` is a no-op here exactly as the plan's C-6 note says; both branches read the
        // same field, kept as an explicit match (not collapsed) so this stays visibly intentional.
        cost_price: match input.r#type {
            ProductType::Service => input.cost_price,
            ProductType::Product => input.cost_price,
        },
        price: input.price,
        min_stock: if is_service { None } else { input.min_stock },
        active: input.active,
        image: input.image.clone(),
        prices: input.prices.as_deref().unwrap_or(&[]).iter().filter_map(normalize_price).collect(),
        purchase_account_id: input.purchase_account_id,
        brand: input.brand.clone(),
        tags: input.tags.clone().map(|t| t.into_iter().filter(|s| !s.is_empty()).collect()),
        image_ids: input.image_ids.clone(),
        description: input.description.clone(),
        units: input.units.clone().map(|us| {
            us.into_iter()
                .map(|u| DtoProductUnit { barcodes: u.barcodes.into_iter().filter(|b| !b.is_empty()).collect(), ..u })
                .collect()
        }),
        unit_prices: input.unit_prices.clone(),
        min_price: input.min_price,
        sale_tax_id: input.sale_tax_id,
        purchase_tax_id: input.purchase_tax_id,
        revenue_account_id: input.revenue_account_id,
        cogs_account_id: input.cogs_account_id,
        allow_negative_stock: input.allow_negative_stock,
        shelf_location: input.shelf_location.clone(),
        preferred_supplier_id: input.preferred_supplier_id,
        reorder_qty: input.reorder_qty,
        track_batches: input.track_batches,
        expiry_alert_days: input.expiry_alert_days,
        warranty_months: input.warranty_months,
        warranty_provider: input.warranty_provider,
        weight: input.weight,
        custom_fields: input.custom_fields.clone(),
    }
}

fn apply_normalized_to_active_model(am: &mut ProductActiveModel, n: &Normalized) {
    am.name = Set(n.name.clone());
    am.name_en = Set(n.name_en.clone());
    am.sku = Set(n.sku.clone());
    am.barcode = Set(n.barcode.clone());
    am.category_id = Set(n.category_id);
    am.unit_id = Set(n.unit_id);
    am.r#type = Set(product_type_as_str(n.r#type).to_string());
    am.stock_mode = Set(stock_mode_as_str(n.stock_mode).map(str::to_string));
    am.price = Set(n.price);
    am.min_stock = Set(n.min_stock);
    am.active = Set(n.active);
    am.image = Set(n.image.clone());
    am.purchase_account_id = Set(n.purchase_account_id);
    am.brand = Set(n.brand.clone());
    am.tags = Set(n.tags.clone().map(StringList));
    am.image_ids = Set(n.image_ids.clone().map(StringList));
    am.description = Set(n.description.clone());
    am.units = Set(n.units.as_ref().map(|us| ProductUnits(us.iter().map(from_dto_unit).collect())));
    am.unit_prices = Set(n.unit_prices.as_ref().map(|us| ProductUnitPrices(us.iter().map(from_dto_unit_price).collect())));
    am.min_price = Set(n.min_price);
    am.sale_tax_id = Set(n.sale_tax_id);
    am.purchase_tax_id = Set(n.purchase_tax_id);
    am.revenue_account_id = Set(n.revenue_account_id);
    am.cogs_account_id = Set(n.cogs_account_id);
    am.allow_negative_stock = Set(n.allow_negative_stock);
    am.shelf_location = Set(n.shelf_location.clone());
    am.preferred_supplier_id = Set(n.preferred_supplier_id);
    am.reorder_qty = Set(n.reorder_qty);
    am.track_batches = Set(n.track_batches);
    am.expiry_alert_days = Set(n.expiry_alert_days);
    am.warranty_months = Set(n.warranty_months);
    am.warranty_provider = Set(warranty_provider_as_str(n.warranty_provider).map(str::to_string));
    am.weight = Set(n.weight);
    am.custom_fields = Set(n.custom_fields.clone().map(CustomFieldValues));
}

/// Replaces a product's `product_prices` (`unit_id` NULL) rows with `prices`, in input order.
async fn replace_prices<C: ConnectionTrait>(conn: &C, cx: &TxCtx, product_id: Id, prices: &[ProductPrice]) -> TxResult<()> {
    product_prices::Entity::delete_many()
        .filter(product_prices::Column::ProductId.eq(product_id))
        .filter(product_prices::Column::UnitId.is_null())
        .exec(conn)
        .await
        .map_err(AppError::from)?;
    for p in prices {
        let am = product_prices::ActiveModel {
            id: Set(Id::new()),
            product_id: Set(product_id),
            price_list_id: Set(p.price_list_id),
            unit_id: Set(None),
            value: Set(p.value),
            created_at: Set(cx.clock.now),
            updated_at: Set(cx.clock.now),
        };
        am.insert(conn).await.map_err(AppError::from)?;
    }
    Ok(())
}

// --- C-7 create_product -----------------------------------------------------------------------

/// **C-7 `create_product(input)`** (`productService.ts:123-139`), one transaction: G-P4 numbering
/// lock -> validate -> normalize -> insert (with `init_product_stock`) -> insert prices -> opening
/// STOCK_IN adjustment (step 6, `:130-135`; only when `type == product && stock_mode != none &&
/// opening_qty > 0`) -> activity log -> touch Catalog. Its activity row is written BEFORE the
/// product's (mock order, §3 C-7 step 6) — the adjustment goes through 06b's
/// `record_stock_adjustment` (STOCK_IN, reason `opening`, unit cost = the product's `cost_price`,
/// posts `Dr inventory / Cr openingBalanceEquity`); above the approval threshold it raises
/// `FORBIDDEN` and the whole transaction (including the just-inserted product row) rolls back —
/// strictly safer than the mock's partial write (Q-5).
pub async fn create_product<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::UndoRegistry,
    input: ProductInput,
) -> TxResult<Product> {
    // G-P4c: serializes SKU/barcode/unit-barcode pre-checks + the write across concurrent creates.
    // `SequenceLock::ProductCodes` is a manager-owned addition to `shared::numbering` — see this
    // wave's "Needs from manager" note; until it lands this call is commented out at the call site
    // is NOT an option (the plan requires it before the pre-checks), so this line assumes it exists.
    crate::shared::numbering::lock(conn, crate::shared::numbering::SequenceLock::ProductCodes).await?;

    validate(conn, &input, None, None).await?;
    let normalized = normalize(&input);

    let id = Id::new();
    let now = cx.clock.now;
    let mut am = ProductActiveModel {
        id: Set(id),
        // stock_qty/stock_value are left to their column DEFAULT 0 (m0005): only shared::stock
        // writes them (D-5).
        cost_price: Set(normalized.cost_price),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
        ..Default::default()
    };
    apply_normalized_to_active_model(&mut am, &normalized);
    stock::init_product_stock(&mut am, normalized.cost_price);

    // `map_unique_violation` checks the constraint name BEFORE `AppError::from` erases it (G-3/PG-3),
    // so the SKU and barcode unique violations each get their own message even though both come from
    // the same `INSERT` — the pre-checks above already serialize this under the ProductCodes lock,
    // so the DB backstop only fires in a genuine race (G-P4).
    let inserted = am
        .insert(conn)
        .await
        .map_err(|e| match map_unique_violation(e, "uq_products_sku_live", || "رمز المنتج مستخدم لمنتج آخر".to_string()) {
            crate::core::tx::TxError::Db(e) => map_unique_violation(e, "uq_products_barcode_live", || "الباركود مستخدم لمنتج آخر".to_string()),
            other => other,
        })?;

    replace_prices(conn, cx, id, &normalized.prices).await?;

    // Step 6 (`:130-135`): opening STOCK_IN adjustment, only for a stock-tracked product with a
    // positive opening qty. Its activity row is written before the product's row below (mock
    // order) — `record_stock_adjustment` logs internally, so simply calling it first here achieves
    // that ordering without any extra bookkeeping.
    let opening_qty = input.opening_qty.unwrap_or(Decimal::ZERO);
    if matches!(normalized.r#type, ProductType::Product) && !matches!(normalized.stock_mode, Some(StockMode::None)) && opening_qty > Decimal::ZERO {
        let now_date = crate::utils::dates::DocDate { day: cx.clock.today(), instant: Some(now) };
        let opening_input = super::super::dto::inventory::StockAdjustmentInput {
            r#type: super::super::dto::inventory::StockAdjustmentType::StockIn,
            date: now_date.key(),
            note: Some(format!("رصيد افتتاحي — {}", normalized.name)),
            reason: Some(super::super::dto::inventory::StockInReason::Opening),
            offset_account_id: None,
            lines: vec![super::super::dto::inventory::StockAdjustmentLineInput {
                product_id: id,
                qty_change: Some(opening_qty),
                counted_qty: None,
                batch_no: None,
                expiry_date: None,
            }],
            approved_by: None,
        };
        super::adjustments::record_stock_adjustment(conn, cx, registry, opening_input, false, super::adjustments::ApprovalCheck::none()).await?;
    }

    activity::log(
        conn,
        cx,
        registry,
        crate::entities::platform::activity::ActivityKind::Product,
        format!("إضافة المنتج {}", inserted.name),
        None,
        Some(RouteRef::detail("product", id.to_string())),
    )
    .await?;
    cx.touch(crate::core::events::ChangeCategory::Catalog);

    // Re-read: the opening adjustment (when applied) updated this row's stock_qty/stock_value via
    // `shared::stock::apply_change` — `product_dto` must reflect that, not the pre-adjustment insert.
    let final_row = ProductEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("المنتج غير موجود"))?;
    product_dto(conn, &final_row).await
}

// --- C-8 update_product -----------------------------------------------------------------------

/// **C-8 `update_product(id, input)`** (`:141-160`).
pub async fn update_product<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, id: Id, input: ProductInput) -> TxResult<Product> {
    crate::shared::numbering::lock(conn, crate::shared::numbering::SequenceLock::ProductCodes).await?;

    let mut locked = stock::lock_product(conn, id).await?;
    let existing_units = locked.model.units.clone();
    let existing = ExistingProduct { stock_qty: locked.stock_qty(), units: existing_units.as_ref().map(|u| u.0.as_slice()) };
    validate(conn, &input, Some(id), Some(&existing)).await?;

    if product_type_as_str(input.r#type) != locked.model.r#type && locked.stock_qty() != Decimal::ZERO {
        return Err(AppError::validation("لا يمكن تحويل منتج له رصيد مخزون إلى خدمة — صفّر المخزون أولاً").into());
    }

    let normalized = normalize(&input);
    let mut am = ProductActiveModel { id: Set(id), updated_at: Set(cx.clock.now), ..Default::default() };
    apply_normalized_to_active_model(&mut am, &normalized);
    let updated = am
        .update(conn)
        .await
        .map_err(|e| match map_unique_violation(e, "uq_products_sku_live", || "رمز المنتج مستخدم لمنتج آخر".to_string()) {
            crate::core::tx::TxError::Db(e) => map_unique_violation(e, "uq_products_barcode_live", || "الباركود مستخدم لمنتج آخر".to_string()),
            other => other,
        })?;
    locked.model = updated.clone();

    stock::set_cost_when_empty(conn, cx, &mut locked, normalized.cost_price).await?;

    replace_prices(conn, cx, id, &normalized.prices).await?;

    activity::log(
        conn,
        cx,
        registry,
        crate::entities::platform::activity::ActivityKind::Product,
        format!("تعديل المنتج {}", locked.model.name),
        None,
        Some(RouteRef::detail("product", id.to_string())),
    )
    .await?;
    cx.touch(crate::core::events::ChangeCategory::Catalog);

    let final_row = ProductEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("المنتج غير موجود"))?;
    product_dto(conn, &final_row).await
}

// --- C-9 suggest_sku --------------------------------------------------------------------------

/// **`suggest_sku(prefix)`** (`:163-170`): `SELECT sku FROM products WHERE sku LIKE CONCAT(?, '-%')`.
pub async fn suggest_sku<C: ConnectionTrait>(conn: &C, prefix: &str) -> TxResult<String> {
    let pattern = format!("{}-%", like_escape(prefix));
    let rows = ProductEntity::find().filter(ProductColumn::Sku.like(pattern)).all(conn).await.map_err(AppError::from)?;
    let expected_prefix = format!("{prefix}-");
    let max_suffix = rows
        .iter()
        .filter(|p| p.sku.starts_with(&expected_prefix))
        .map(|p| p.sku[expected_prefix.len()..].parse::<i64>().unwrap_or(0))
        .max()
        .unwrap_or(0);
    Ok(format!("{prefix}-{:03}", max_suffix.max(0) + 1))
}

fn like_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c == '%' || c == '_' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
