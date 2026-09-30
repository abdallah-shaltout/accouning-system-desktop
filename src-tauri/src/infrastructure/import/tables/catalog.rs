//! `units, categories, price_lists, products, product_prices, product_branch_stock,
//! product_batches, custom_field_defs`.

use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait};

use crate::core::tx::{TxError, TxResult};
use crate::entities::catalog::categories::ActiveModel as CategoryActiveModel;
use crate::entities::catalog::custom_field_defs::{ActiveModel as CustomFieldDefActiveModel, FieldOptions};
use crate::entities::catalog::price_lists::ActiveModel as PriceListActiveModel;
use crate::entities::catalog::product_batches::ActiveModel as ProductBatchActiveModel;
use crate::entities::catalog::product_branch_stock::ActiveModel as ProductBranchStockActiveModel;
use crate::entities::catalog::product_prices::ActiveModel as ProductPriceActiveModel;
use crate::entities::catalog::products::{ActiveModel as ProductActiveModel, CustomFieldValues, ProductUnit, ProductUnitPrice, ProductUnitPrices, ProductUnits, StockByBranch, BranchStockEntry};
use crate::entities::catalog::units::ActiveModel as UnitActiveModel;
use crate::entities::values::StringList;
use crate::infrastructure::import::idmap::{is_freetext_product_id, IdMap};
use crate::infrastructure::import::model::{CategoryV1, CustomFieldDefV1, PriceListV1, ProductBatchV1, ProductV1, UnitV1};
use crate::infrastructure::import::tables::{parse_doc_date, resolve_created_at, strict_ref};
use crate::utils::id::Id;
use crate::utils::money::{round2, round4, round_qty};

pub async fn insert_units<C: ConnectionTrait>(conn: &C, rows: &[UnitV1], id_map: &IdMap, import_base: chrono::DateTime<chrono::Utc>) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let model = UnitActiveModel {
            id: Set(id),
            name: Set(row.name.clone()),
            symbol: Set(row.symbol.clone()),
            allows_decimals: Set(row.allows_decimals),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
            name_live: sea_orm::ActiveValue::NotSet,
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

pub async fn insert_categories<C: ConnectionTrait>(
    conn: &C,
    rows: &[CategoryV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let model = CategoryActiveModel {
            id: Set(id),
            name: Set(row.name.clone()),
            // Deferred (self-reference).
            parent_id: Set(None),
            purchase_account_id: Set(strict_ref(id_map, row.purchase_account_id.as_deref(), "categories")?),
            revenue_account_id: Set(strict_ref(id_map, row.revenue_account_id.as_deref(), "categories")?),
            cogs_account_id: Set(strict_ref(id_map, row.cogs_account_id.as_deref(), "categories")?),
            sale_tax_id: Set(strict_ref(id_map, row.sale_tax_id.as_deref(), "categories")?),
            purchase_tax_id: Set(strict_ref(id_map, row.purchase_tax_id.as_deref(), "categories")?),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
            name_live: sea_orm::ActiveValue::NotSet,
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

pub async fn insert_price_lists<C: ConnectionTrait>(
    conn: &C,
    rows: &[PriceListV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let model = PriceListActiveModel {
            id: Set(id),
            name: Set(row.name.clone()),
            active: Set(row.active),
            currency: Set(row.currency.clone().map(|c| c.to_uppercase())),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
            name_live: sea_orm::ActiveValue::NotSet,
        };
        model.insert(conn).await.map_err(TxError::from)?;

        // `price_lists.values` (`PriceListValueV1`) become `product_prices` rows — but only once
        // the referenced product has been assigned an id. Since `products` is inserted AFTER
        // `price_lists` in IMPORT_ORDER, this pass only *records* nothing here: `product_prices`
        // rows for a price list's own `.values` are instead inserted from `insert_products` below
        // (which runs after both `price_lists` and `products` exist), reading `price_list.values`
        // via the caller-supplied slice — see `insert_product_prices`.
        let _ = &row.values;
    }
    Ok(())
}

/// Called by `run.rs` right after both `price_lists` and `products` have been inserted (their own
/// `IMPORT_ORDER` slots): builds the `product_prices` rows (`unit_id = NULL`, the base-unit price per
/// price list) from every `Product.prices` entry — where the TS mock keeps them — in product array
/// order, then from any legacy `PriceList.values` entry (an older snapshot shape), now that both
/// sides of the FK resolve. A price for a price list or product that isn't in the snapshot is
/// skipped (the mock deletes a price list without touching its products' `prices`).
/// `unitPrices` stay a JSON column on the product row (`insert_products`).
pub async fn insert_product_prices<C: ConnectionTrait>(
    conn: &C,
    price_lists: &[PriceListV1],
    products: &[ProductV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    // Collected into owned tuples up front: holding the borrowing iterator chain across the
    // `.await` below trips rustc's higher-ranked `Send` inference for the command's boxed future.
    let mut entries: Vec<(String, String, Option<String>, rust_decimal::Decimal)> = Vec::new();
    for p in products {
        for price in p.prices.iter().flatten() {
            entries.push((p.id.clone(), price.price_list_id.clone(), None, price.value));
        }
    }
    for pl in price_lists {
        for v in &pl.values {
            entries.push((v.product_id.clone(), pl.id.clone(), v.unit_id.clone(), v.value));
        }
    }

    for (index, (product_old, price_list_old, unit_old, value)) in entries.into_iter().enumerate() {
        let (Some(product_id), Some(price_list_id)) = (id_map.resolve(&product_old), id_map.resolve(&price_list_old)) else { continue };
        let unit_id = unit_old.as_deref().and_then(|old| id_map.resolve(old));
        let amount = round2(value);
        if amount != value {
            *rounded += 1;
        }
        let model = ProductPriceActiveModel {
            id: Set(Id::new()),
            product_id: Set(product_id),
            price_list_id: Set(price_list_id),
            unit_id: Set(unit_id),
            value: Set(amount),
            created_at: Set(resolve_created_at(None, import_base, index)),
            updated_at: Set(resolve_created_at(None, import_base, index)),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

pub async fn insert_custom_field_defs<C: ConnectionTrait>(
    conn: &C,
    rows: &[CustomFieldDefV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let model = CustomFieldDefActiveModel {
            id: Set(id),
            name: Set(row.name.clone()),
            r#type: Set(row.kind.clone()),
            options: Set(row.options.clone().map(FieldOptions)),
            active: Set(row.active),
            sort_order: Set(row.sort_order),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

/// Products, per-row `insert` (never `insert_many`, §3.2.6) so `products::ActiveModelBehavior::
/// before_save` computes `search_normalized`. Also inserts each product's `product_branch_stock`
/// rows (from `stockByBranch`) — `product_prices` rows for a product's own `unitPrices` are also
/// inserted here (not from the price-list pass, which only covers `PriceList.values`).
pub async fn insert_products<C: ConnectionTrait>(
    conn: &C,
    rows: &[ProductV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);

        let cost_price = round4(row.cost_price);
        if cost_price != row.cost_price {
            *rounded += 1;
        }
        let price = round2(row.price);
        if price != row.price {
            *rounded += 1;
        }
        let stock_qty = round_qty(row.stock_qty);
        if stock_qty != row.stock_qty {
            *rounded += 1;
        }
        let stock_value = round2(row.stock_value);
        if stock_value != row.stock_value {
            *rounded += 1;
        }

        let units: Option<ProductUnits> = row.units.as_ref().map(|list| {
            ProductUnits(
                list.iter()
                    .map(|u| ProductUnit {
                        id: u.id.clone().unwrap_or_else(|| Id::new().to_string()),
                        unit_id: id_map.resolve(&u.unit_id).unwrap_or_else(|| id_map.resolve_or_mint(&u.unit_id)),
                        factor: round4(u.factor),
                        barcodes: u.barcodes.clone(),
                        price: round2(u.price),
                        price_is_auto: u.price_is_auto,
                        default_for_sale: u.default_for_sale,
                        default_for_purchase: u.default_for_purchase,
                        active: u.active,
                    })
                    .collect(),
            )
        });

        let unit_prices: Option<ProductUnitPrices> = row.unit_prices.as_ref().map(|list| {
            ProductUnitPrices(
                list.iter()
                    .filter_map(|up| {
                        let price_list_id = id_map.resolve(&up.price_list_id)?;
                        // `unitId` is the product's own `ProductUnit.id` (kept verbatim in `units`
                        // above) or `'__base__'` — never an id-map key, so it's copied as is.
                        Some(ProductUnitPrice { price_list_id, unit_id: up.unit_id.clone(), value: round2(up.value) })
                    })
                    .collect(),
            )
        });

        let stock_by_branch: Option<StockByBranch> = row.stock_by_branch.as_ref().map(|map| {
            let mut out = std::collections::BTreeMap::new();
            for (branch_old, entry) in map {
                if let Some(branch_id) = id_map.resolve(branch_old) {
                    out.insert(branch_id.to_string(), BranchStockEntry { qty: round_qty(entry.qty), value: round2(entry.value) });
                }
            }
            StockByBranch(out)
        });

        let custom_fields: Option<CustomFieldValues> = row.custom_fields.as_ref().map(|map| {
            let mut out = std::collections::BTreeMap::new();
            for (field_def_old, value) in map {
                let key = id_map.resolve(field_def_old).map(|id| id.to_string()).unwrap_or_else(|| field_def_old.clone());
                out.insert(key, value.clone());
            }
            CustomFieldValues(out)
        });

        let model = ProductActiveModel {
            id: Set(id),
            name: Set(row.name.clone()),
            name_en: Set(row.name_en.clone()),
            sku: Set(row.sku.clone()),
            barcode: Set(row.barcode.clone()),
            category_id: Set(strict_ref(id_map, row.category_id.as_deref(), "products")?),
            unit_id: Set(strict_ref(id_map, row.unit_id.as_deref(), "products")?),
            r#type: Set(row.kind.clone()),
            stock_mode: Set(row.stock_mode.clone()),
            cost_price: Set(cost_price),
            price: Set(price),
            stock_qty: Set(stock_qty),
            min_stock: Set(row.min_stock.map(round_qty)),
            active: Set(row.active),
            image: Set(row.image.clone()),
            purchase_account_id: Set(strict_ref(id_map, row.purchase_account_id.as_deref(), "products")?),
            stock_value: Set(stock_value),
            stock_by_branch: Set(stock_by_branch),
            brand: Set(row.brand.clone()),
            tags: Set(row.tags.clone().map(StringList)),
            image_ids: Set(row.image_ids.clone().map(StringList)),
            description: Set(row.description.clone()),
            units: Set(units),
            unit_prices: Set(unit_prices),
            min_price: Set(row.min_price.map(round2)),
            sale_tax_id: Set(strict_ref(id_map, row.sale_tax_id.as_deref(), "products")?),
            purchase_tax_id: Set(strict_ref(id_map, row.purchase_tax_id.as_deref(), "products")?),
            revenue_account_id: Set(strict_ref(id_map, row.revenue_account_id.as_deref(), "products")?),
            cogs_account_id: Set(strict_ref(id_map, row.cogs_account_id.as_deref(), "products")?),
            allow_negative_stock: Set(row.allow_negative_stock),
            shelf_location: Set(row.shelf_location.clone()),
            preferred_supplier_id: Set(strict_ref(id_map, row.preferred_supplier_id.as_deref(), "products")?),
            reorder_qty: Set(row.reorder_qty.map(round_qty)),
            track_batches: Set(row.track_batches),
            expiry_alert_days: Set(row.expiry_alert_days),
            warranty_months: Set(row.warranty_months),
            warranty_provider: Set(row.warranty_provider.clone()),
            weight: Set(row.weight.map(round4)),
            custom_fields: Set(custom_fields),
            search_normalized: sea_orm::ActiveValue::NotSet, // computed by before_save.
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
            sku_live: sea_orm::ActiveValue::NotSet,
            barcode_live: sea_orm::ActiveValue::NotSet,
        };
        model.insert(conn).await.map_err(TxError::from)?;

        if let Some(map) = &row.stock_by_branch {
            for (branch_old, entry) in map {
                let Some(branch_id) = id_map.resolve(branch_old) else { continue };
                let qty = round_qty(entry.qty);
                let value = round2(entry.value);
                if qty != entry.qty || value != entry.value {
                    *rounded += 1;
                }
                let stock_model = ProductBranchStockActiveModel {
                    id: Set(Id::new()),
                    product_id: Set(id),
                    branch_id: Set(branch_id),
                    qty: Set(qty),
                    value: Set(value),
                    created_at: Set(resolve_created_at(None, import_base, i)),
                    updated_at: Set(resolve_created_at(None, import_base, i)),
                };
                stock_model.insert(conn).await.map_err(TxError::from)?;
            }
        }
    }
    Ok(())
}

pub async fn insert_product_batches<C: ConnectionTrait>(
    conn: &C,
    rows: &[ProductBatchV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(product_id) = id_map.resolve(&row.product_id) else {
            return Err(TxError::App(crate::core::error::AppError::validation("تعذر الاستيراد: مرجع غير موجود في product_batches")));
        };
        let (received_day, received_instant) = parse_doc_date(&row.received_date, tz)
            .ok_or_else(|| crate::core::error::AppError::validation("تاريخ غير صالح في product_batches"))?;
        let expiry_date = row.expiry_date.as_deref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
        let qty = round_qty(row.qty);
        let unit_cost = round4(row.unit_cost);
        if qty != row.qty || unit_cost != row.unit_cost {
            *rounded += 1;
        }
        // Polymorphic (B-1): resolve_or_mint so a later reference to the same old string (e.g. a
        // stock movement's `ref_id`) maps consistently to the same new id.
        let source_ref_id = row.source_ref_id.as_deref().map(|old| id_map.resolve_or_mint(old));
        let _ = is_freetext_product_id; // referenced by sales/purchases table modules, kept in scope here for discoverability.
        let model = ProductBatchActiveModel {
            id: Set(id),
            product_id: Set(product_id),
            batch_no: Set(row.batch_no.clone()),
            expiry_date: Set(expiry_date),
            qty: Set(qty),
            unit_cost: Set(unit_cost),
            supplier_id: Set(strict_ref(id_map, row.supplier_id.as_deref(), "product_batches")?),
            received_date_day: Set(received_day),
            received_date_instant: Set(received_instant),
            source_ref_id: Set(source_ref_id),
            source_ref_number: Set(row.source_ref_number.clone()),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}
