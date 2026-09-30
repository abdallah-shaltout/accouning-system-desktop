//! `domains::products::service::catalog` — categories/units/price lists/custom field defs
//! (06-products.md §3, C-10…C-13). No activity/audit rows for any of these (analysis §6/D-P4).

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::{map_unique_violation, AppError};
use crate::core::events::ChangeCategory;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::catalog::categories::{self, ActiveModel as CategoryActiveModel, Column as CategoryColumn, Entity as CategoryEntity};
use crate::entities::catalog::custom_field_defs::{self, ActiveModel as CustomFieldActiveModel, Column as CustomFieldColumn, Entity as CustomFieldEntity, FieldOptions};
use crate::entities::catalog::price_lists::{self, ActiveModel as PriceListActiveModel, Column as PriceListColumn, Entity as PriceListEntity};
use crate::entities::catalog::product_prices;
use crate::entities::catalog::products::{Column as ProductColumn, Entity as ProductEntity};
use crate::entities::catalog::units::{self, ActiveModel as UnitActiveModel, Column as UnitColumn, Entity as UnitEntity};
use crate::entities::org::users::{Column as UserColumn, Entity as UserEntity};
use crate::entities::soft_delete::SoftDelete;
use crate::utils::id::Id;

use super::super::dto::catalog::{
    Category, CategoryDefaults, CategoryWithCount, CustomFieldDef, CustomFieldDefInput, CustomFieldType, PriceList, PriceListInput, Unit, UnitExtra, UnitPresetKind,
    UnitWithCount,
};

fn to_category(m: &categories::Model) -> Category {
    Category {
        id: m.id,
        name: m.name.clone(),
        purchase_account_id: m.purchase_account_id,
        revenue_account_id: m.revenue_account_id,
        cogs_account_id: m.cogs_account_id,
        sale_tax_id: m.sale_tax_id,
        purchase_tax_id: m.purchase_tax_id,
    }
}

fn to_unit(m: &units::Model) -> Unit {
    Unit { id: m.id, name: m.name.clone(), symbol: m.symbol.clone(), allows_decimals: m.allows_decimals }
}

fn to_price_list(m: &price_lists::Model) -> PriceList {
    PriceList { id: m.id, name: m.name.clone(), active: m.active, currency: m.currency.clone() }
}

fn parse_field_type(s: &str) -> CustomFieldType {
    match s {
        "text" => CustomFieldType::Text,
        "number" => CustomFieldType::Number,
        "date" => CustomFieldType::Date,
        "list" => CustomFieldType::List,
        "yesno" => CustomFieldType::Yesno,
        other => panic!("custom_field_defs.type column holds an unrecognized value: {other:?}"),
    }
}

fn field_type_as_str(t: CustomFieldType) -> &'static str {
    match t {
        CustomFieldType::Text => "text",
        CustomFieldType::Number => "number",
        CustomFieldType::Date => "date",
        CustomFieldType::List => "list",
        CustomFieldType::Yesno => "yesno",
    }
}

fn to_custom_field_def(m: &custom_field_defs::Model) -> CustomFieldDef {
    CustomFieldDef { id: m.id, name: m.name.clone(), r#type: parse_field_type(&m.r#type), options: m.options.clone().map(|o| o.0), active: m.active, sort_order: m.sort_order }
}

// --- Categories -----------------------------------------------------------------------------------

/// `assertName`'s trim/empty-check half (`catalogService.ts:10-13`) — shared verbatim across
/// categories/units/price lists (`الاسم مطلوب` is the same message for all three tables). The
/// "name used by another live row" half is entity-specific (each table's own `Select`/columns), so
/// each caller below has its own `assert_<table>_name_free` doing that second check.
fn assert_category_name_ok(name: &str) -> TxResult<String> {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err(AppError::validation("الاسم مطلوب").into());
    }
    Ok(trimmed)
}

/// Exact, case-sensitive match (the mock's `x.name.trim() === trimmed`): filtered on the
/// `utf8mb4_bin` `name_live` column (m0016 G-28a), not `name` (`utf8mb4_unicode_ci`, which would
/// refuse "food" next to "Food").
async fn assert_category_name_free<C: ConnectionTrait>(conn: &C, trimmed: &str, except_id: Option<Id>) -> TxResult<()> {
    let mut query = CategoryEntity::find_live().filter(CategoryColumn::NameLive.eq(trimmed.to_string()));
    if let Some(except) = except_id {
        query = query.filter(CategoryColumn::Id.ne(except));
    }
    if query.one(conn).await.map_err(AppError::from)?.is_some() {
        return Err(AppError::conflict("الاسم مستخدم من قبل").into());
    }
    Ok(())
}

pub async fn get_categories<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<CategoryWithCount>> {
    let rows = CategoryEntity::find_live().order_by_asc(CategoryColumn::CreatedAt).order_by_asc(CategoryColumn::Id).all(conn).await.map_err(AppError::from)?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let count = ProductEntity::find().filter(ProductColumn::CategoryId.eq(row.id)).count(conn).await.map_err(AppError::from)?;
        out.push(CategoryWithCount::new(to_category(&row), count as u32));
    }
    Ok(out)
}

pub async fn save_category<C: ConnectionTrait>(conn: &C, cx: &TxCtx, name: String, id: Option<Id>, defaults: Option<CategoryDefaults>) -> TxResult<Category> {
    let trimmed = assert_category_name_ok(&name)?;
    assert_category_name_free(conn, &trimmed, id).await?;

    if let Some(id) = id {
        let existing = CategoryEntity::find_live().filter(CategoryColumn::Id.eq(id)).one(conn).await.map_err(AppError::from)?;
        let Some(existing) = existing else { return Err(AppError::not_found("التصنيف غير موجود").into()) };
        let mut am: CategoryActiveModel = existing.into();
        am.name = Set(trimmed);
        if let Some(d) = &defaults {
            am.purchase_account_id = Set(d.purchase_account_id);
            am.revenue_account_id = Set(d.revenue_account_id);
            am.cogs_account_id = Set(d.cogs_account_id);
            am.sale_tax_id = Set(d.sale_tax_id);
            am.purchase_tax_id = Set(d.purchase_tax_id);
        }
        am.updated_at = Set(cx.clock.now);
        let updated = am
            .update(conn)
            .await
            .map_err(|e| map_unique_violation(e, "uq_categories_name_live", || "الاسم مستخدم من قبل".to_string()))?;
        cx.touch(ChangeCategory::Catalog);
        return Ok(to_category(&updated));
    }

    let now = cx.clock.now;
    let am = CategoryActiveModel {
        id: Set(Id::new()),
        name: Set(trimmed),
        parent_id: Set(None),
        purchase_account_id: Set(defaults.as_ref().and_then(|d| d.purchase_account_id)),
        revenue_account_id: Set(defaults.as_ref().and_then(|d| d.revenue_account_id)),
        cogs_account_id: Set(defaults.as_ref().and_then(|d| d.cogs_account_id)),
        sale_tax_id: Set(defaults.as_ref().and_then(|d| d.sale_tax_id)),
        purchase_tax_id: Set(defaults.as_ref().and_then(|d| d.purchase_tax_id)),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
        name_live: Set(None),
    };
    let inserted = am
        .insert(conn)
        .await
        .map_err(|e| map_unique_violation(e, "uq_categories_name_live", || "الاسم مستخدم من قبل".to_string()))?;
    cx.touch(ChangeCategory::Catalog);
    Ok(to_category(&inserted))
}

pub async fn delete_category<C: ConnectionTrait>(conn: &C, cx: &TxCtx, id: Id) -> TxResult<()> {
    lock::for_update_by_id(conn, "categories", &id.to_string()).await.map_err(AppError::from)?;
    let in_use = ProductEntity::find().filter(ProductColumn::CategoryId.eq(id)).count(conn).await.map_err(AppError::from)?;
    if in_use > 0 {
        return Err(AppError::conflict("لا يمكن حذف تصنيف مرتبط بمنتجات").into());
    }
    // A missing id is a silent no-op (the mock filters nothing, `catalogService.ts:43`).
    if CategoryEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.is_some() {
        CategoryEntity::soft_delete(conn, id, cx.clock.now).await.map_err(AppError::from)?;
        cx.touch(ChangeCategory::Catalog);
    }
    Ok(())
}

// --- Units ----------------------------------------------------------------------------------------

async fn assert_unit_name_free<C: ConnectionTrait>(conn: &C, trimmed: &str, except_id: Option<Id>) -> TxResult<()> {
    let mut query = UnitEntity::find_live().filter(UnitColumn::NameLive.eq(trimmed.to_string()));
    if let Some(except) = except_id {
        query = query.filter(UnitColumn::Id.ne(except));
    }
    if query.one(conn).await.map_err(AppError::from)?.is_some() {
        return Err(AppError::conflict("الاسم مستخدم من قبل").into());
    }
    Ok(())
}

pub async fn get_units<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<UnitWithCount>> {
    let rows = UnitEntity::find_live().order_by_asc(UnitColumn::CreatedAt).order_by_asc(UnitColumn::Id).all(conn).await.map_err(AppError::from)?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        // `productCount` counts products whose **base** `unitId` matches only (Q-2).
        let count = ProductEntity::find().filter(ProductColumn::UnitId.eq(row.id)).count(conn).await.map_err(AppError::from)?;
        out.push(UnitWithCount::new(to_unit(&row), count as u32));
    }
    Ok(out)
}

pub async fn save_unit<C: ConnectionTrait>(conn: &C, cx: &TxCtx, name: String, id: Option<Id>, extra: Option<UnitExtra>) -> TxResult<Unit> {
    let trimmed = assert_category_name_ok(&name)?;
    assert_unit_name_free(conn, &trimmed, id).await?;

    if let Some(id) = id {
        let existing = UnitEntity::find_live().filter(UnitColumn::Id.eq(id)).one(conn).await.map_err(AppError::from)?;
        let Some(existing) = existing else { return Err(AppError::not_found("الوحدة غير موجودة").into()) };
        let mut am: UnitActiveModel = existing.into();
        am.name = Set(trimmed);
        if let Some(e) = &extra {
            if e.symbol.is_some() {
                am.symbol = Set(e.symbol.clone());
            }
            if e.allows_decimals.is_some() {
                am.allows_decimals = Set(e.allows_decimals);
            }
        }
        am.updated_at = Set(cx.clock.now);
        let updated = am.update(conn).await.map_err(|e| map_unique_violation(e, "uq_units_name_live", || "الاسم مستخدم من قبل".to_string()))?;
        cx.touch(ChangeCategory::Catalog);
        return Ok(to_unit(&updated));
    }

    let now = cx.clock.now;
    let am = UnitActiveModel {
        id: Set(Id::new()),
        name: Set(trimmed),
        symbol: Set(extra.as_ref().and_then(|e| e.symbol.clone())),
        allows_decimals: Set(extra.as_ref().and_then(|e| e.allows_decimals)),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
        name_live: Set(None),
    };
    let inserted = am.insert(conn).await.map_err(|e| map_unique_violation(e, "uq_units_name_live", || "الاسم مستخدم من قبل".to_string()))?;
    cx.touch(ChangeCategory::Catalog);
    Ok(to_unit(&inserted))
}

pub async fn delete_unit<C: ConnectionTrait>(conn: &C, cx: &TxCtx, id: Id) -> TxResult<()> {
    lock::for_update_by_id(conn, "units", &id.to_string()).await.map_err(AppError::from)?;
    let in_use = ProductEntity::find().filter(ProductColumn::UnitId.eq(id)).count(conn).await.map_err(AppError::from)?;
    if in_use > 0 {
        return Err(AppError::conflict("لا يمكن حذف وحدة مرتبطة بمنتجات").into());
    }
    if UnitEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.is_some() {
        UnitEntity::soft_delete(conn, id, cx.clock.now).await.map_err(AppError::from)?;
        cx.touch(ChangeCategory::Catalog);
    }
    Ok(())
}

struct PresetUnit {
    name: &'static str,
    symbol: &'static str,
    /// `None` = the preset carries no `allowsDecimals` key at all (`catalogService.ts` `UNIT_PRESETS`),
    /// so the unit reads back without one, exactly like the mock's.
    allows_decimals: Option<bool>,
}

fn unit_presets(kind: UnitPresetKind) -> &'static [PresetUnit] {
    match kind {
        UnitPresetKind::Pharmacy => &[
            PresetUnit { name: "علبة", symbol: "box", allows_decimals: None },
            PresetUnit { name: "شريط", symbol: "strip", allows_decimals: None },
            PresetUnit { name: "قرص", symbol: "tab", allows_decimals: None },
            PresetUnit { name: "زجاجة", symbol: "btl", allows_decimals: None },
            PresetUnit { name: "أمبول", symbol: "amp", allows_decimals: None },
        ],
        UnitPresetKind::Clothing => &[
            PresetUnit { name: "قطعة", symbol: "pc", allows_decimals: None },
            PresetUnit { name: "طقم", symbol: "set", allows_decimals: None },
            PresetUnit { name: "درزن", symbol: "dz", allows_decimals: None },
        ],
        UnitPresetKind::Supermarket => &[
            PresetUnit { name: "حبة", symbol: "pc", allows_decimals: None },
            PresetUnit { name: "كرتون", symbol: "ctn", allows_decimals: None },
            PresetUnit { name: "كيلو", symbol: "kg", allows_decimals: Some(true) },
            PresetUnit { name: "جرام", symbol: "g", allows_decimals: Some(true) },
            PresetUnit { name: "لتر", symbol: "l", allows_decimals: Some(true) },
        ],
    }
}

pub async fn apply_unit_preset<C: ConnectionTrait>(conn: &C, cx: &TxCtx, kind: UnitPresetKind) -> TxResult<Vec<Unit>> {
    let existing_names: std::collections::HashSet<String> =
        UnitEntity::find_live().all(conn).await.map_err(AppError::from)?.into_iter().map(|u| u.name).collect();

    let mut created = Vec::new();
    let now = cx.clock.now;
    for preset in unit_presets(kind) {
        if existing_names.contains(preset.name) {
            continue;
        }
        let am = UnitActiveModel {
            id: Set(Id::new()),
            name: Set(preset.name.to_string()),
            symbol: Set(Some(preset.symbol.to_string())),
            allows_decimals: Set(preset.allows_decimals),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
            name_live: Set(None),
        };
        let inserted = am.insert(conn).await.map_err(AppError::from)?;
        created.push(to_unit(&inserted));
    }
    if !created.is_empty() {
        cx.touch(ChangeCategory::Catalog);
    }
    Ok(created)
}

// --- Price lists ------------------------------------------------------------------------------

async fn assert_price_list_name_free<C: ConnectionTrait>(conn: &C, trimmed: &str, except_id: Option<Id>) -> TxResult<()> {
    let mut query = PriceListEntity::find_live().filter(PriceListColumn::NameLive.eq(trimmed.to_string()));
    if let Some(except) = except_id {
        query = query.filter(PriceListColumn::Id.ne(except));
    }
    if query.one(conn).await.map_err(AppError::from)?.is_some() {
        return Err(AppError::conflict("الاسم مستخدم من قبل").into());
    }
    Ok(())
}

pub async fn get_price_lists<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<PriceList>> {
    let rows = PriceListEntity::find_live().order_by_asc(PriceListColumn::CreatedAt).order_by_asc(PriceListColumn::Id).all(conn).await.map_err(AppError::from)?;
    Ok(rows.iter().map(to_price_list).collect())
}

pub async fn save_price_list<C: ConnectionTrait>(conn: &C, cx: &TxCtx, input: PriceListInput, id: Option<Id>) -> TxResult<PriceList> {
    let trimmed = assert_category_name_ok(&input.name)?;
    assert_price_list_name_free(conn, &trimmed, id).await?;

    if let Some(id) = id {
        let existing = PriceListEntity::find_live().filter(PriceListColumn::Id.eq(id)).one(conn).await.map_err(AppError::from)?;
        let Some(existing) = existing else { return Err(AppError::not_found("قائمة الأسعار غير موجودة").into()) };
        let mut am: PriceListActiveModel = existing.into();
        am.name = Set(trimmed);
        am.active = Set(input.active);
        am.updated_at = Set(cx.clock.now);
        let updated = am
            .update(conn)
            .await
            .map_err(|e| map_unique_violation(e, "uq_price_lists_name_live", || "الاسم مستخدم من قبل".to_string()))?;
        cx.touch(ChangeCategory::Catalog);
        return Ok(to_price_list(&updated));
    }

    let now = cx.clock.now;
    let am = PriceListActiveModel {
        id: Set(Id::new()),
        name: Set(trimmed),
        active: Set(input.active),
        currency: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
        name_live: Set(None),
    };
    let inserted = am
        .insert(conn)
        .await
        .map_err(|e| map_unique_violation(e, "uq_price_lists_name_live", || "الاسم مستخدم من قبل".to_string()))?;
    cx.touch(ChangeCategory::Catalog);
    Ok(to_price_list(&inserted))
}

pub async fn delete_price_list<C: ConnectionTrait>(conn: &C, cx: &TxCtx, id: Id) -> TxResult<()> {
    lock::for_update_by_id(conn, "price_lists", &id.to_string()).await.map_err(AppError::from)?;
    let assigned = UserEntity::find().filter(UserColumn::PriceListId.eq(id)).count(conn).await.map_err(AppError::from)?;
    if assigned > 0 {
        return Err(AppError::conflict("قائمة الأسعار مسندة لمستخدمين — أزل الإسناد أولاً").into());
    }
    product_prices::Entity::delete_many()
        .filter(product_prices::Column::PriceListId.eq(id))
        .exec(conn)
        .await
        .map_err(AppError::from)?;
    if PriceListEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.is_some() {
        PriceListEntity::soft_delete(conn, id, cx.clock.now).await.map_err(AppError::from)?;
    }
    cx.touch(ChangeCategory::Catalog);
    Ok(())
}

pub async fn set_price_list_values<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    price_list_id: Id,
    values: std::collections::BTreeMap<String, Option<rust_decimal::Decimal>>,
) -> TxResult<()> {
    if PriceListEntity::find_live().filter(PriceListColumn::Id.eq(price_list_id)).one(conn).await.map_err(AppError::from)?.is_none() {
        return Err(AppError::not_found("قائمة الأسعار غير موجودة").into());
    }

    for (product_id_str, value) in values {
        let Ok(product_id) = product_id_str.parse::<Id>() else { continue };
        let product = ProductEntity::find_by_id(product_id).one(conn).await.map_err(AppError::from)?;
        let Some(product) = product else { continue };

        product_prices::Entity::delete_many()
            .filter(product_prices::Column::ProductId.eq(product_id))
            .filter(product_prices::Column::PriceListId.eq(price_list_id))
            .exec(conn)
            .await
            .map_err(AppError::from)?;

        if let Some(v) = value {
            if v < rust_decimal::Decimal::ZERO {
                return Err(AppError::validation(format!("سعر \"{}\" لا يمكن أن يكون سالباً", product.name)).into());
            }
            let am = product_prices::ActiveModel {
                id: Set(Id::new()),
                product_id: Set(product_id),
                price_list_id: Set(price_list_id),
                unit_id: Set(None),
                value: Set(v),
                created_at: Set(cx.clock.now),
                updated_at: Set(cx.clock.now),
            };
            am.insert(conn).await.map_err(AppError::from)?;
        }
    }
    cx.touch(ChangeCategory::Catalog);
    Ok(())
}

// --- Custom field defs --------------------------------------------------------------------------

pub async fn get_custom_field_defs<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<CustomFieldDef>> {
    let rows = CustomFieldEntity::find_live()
        .order_by_asc(CustomFieldColumn::SortOrder)
        .order_by_asc(CustomFieldColumn::CreatedAt)
        .order_by_asc(CustomFieldColumn::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(rows.iter().map(to_custom_field_def).collect())
}

pub async fn save_custom_field_def<C: ConnectionTrait>(conn: &C, cx: &TxCtx, input: CustomFieldDefInput, id: Option<Id>) -> TxResult<CustomFieldDef> {
    if input.name.trim().is_empty() {
        return Err(AppError::validation("اسم الحقل مطلوب").into());
    }
    if matches!(input.r#type, CustomFieldType::List) && input.options.as_ref().map(|o| o.is_empty()).unwrap_or(true) {
        return Err(AppError::validation("أضف خيارات لحقل من نوع قائمة").into());
    }

    if let Some(id) = id {
        let existing = CustomFieldEntity::find_live().filter(CustomFieldColumn::Id.eq(id)).one(conn).await.map_err(AppError::from)?;
        let Some(existing) = existing else { return Err(AppError::not_found("الحقل غير موجود").into()) };
        let mut am: CustomFieldActiveModel = existing.into();
        am.name = Set(input.name.trim().to_string());
        am.r#type = Set(field_type_as_str(input.r#type).to_string());
        am.active = Set(input.active);
        if input.options.is_some() {
            am.options = Set(input.options.clone().map(FieldOptions));
        }
        am.updated_at = Set(cx.clock.now);
        let updated = am.update(conn).await.map_err(AppError::from)?;
        cx.touch(ChangeCategory::Catalog);
        return Ok(to_custom_field_def(&updated));
    }

    let sort_order = CustomFieldEntity::find_live().count(conn).await.map_err(AppError::from)? as i16 + 1;
    let now = cx.clock.now;
    let am = CustomFieldActiveModel {
        id: Set(Id::new()),
        name: Set(input.name.trim().to_string()),
        r#type: Set(field_type_as_str(input.r#type).to_string()),
        options: Set(input.options.clone().map(FieldOptions)),
        active: Set(input.active),
        sort_order: Set(sort_order),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    let inserted = am.insert(conn).await.map_err(AppError::from)?;
    cx.touch(ChangeCategory::Catalog);
    Ok(to_custom_field_def(&inserted))
}

pub async fn delete_custom_field_def<C: ConnectionTrait>(conn: &C, cx: &TxCtx, id: Id) -> TxResult<()> {
    lock::for_update_by_id(conn, "custom_field_defs", &id.to_string()).await.map_err(AppError::from)?;

    let field_key = format!("$.\"{id}\"");
    let stmt = sea_orm::Statement::from_sql_and_values(
        conn.get_database_backend(),
        "SELECT id FROM products WHERE JSON_CONTAINS_PATH(custom_fields, 'one', ?) LIMIT 1",
        [field_key.into()],
    );
    if conn.query_one(stmt).await.map_err(AppError::from)?.is_some() {
        return Err(AppError::conflict("لا يمكن حذف حقل مستخدم في بيانات منتجات — عطّله بدلاً من ذلك").into());
    }

    if CustomFieldEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.is_some() {
        CustomFieldEntity::soft_delete(conn, id, cx.clock.now).await.map_err(AppError::from)?;
        cx.touch(ChangeCategory::Catalog);
    }
    Ok(())
}

pub async fn reorder_custom_field_defs<C: ConnectionTrait>(conn: &C, cx: &TxCtx, ordered_ids: Vec<String>) -> TxResult<()> {
    for (i, id_str) in ordered_ids.iter().enumerate() {
        let Ok(id) = id_str.parse::<Id>() else { continue };
        let existing = CustomFieldEntity::find_live().filter(CustomFieldColumn::Id.eq(id)).one(conn).await.map_err(AppError::from)?;
        let Some(existing) = existing else { continue };
        let mut am: CustomFieldActiveModel = existing.into();
        am.sort_order = Set(i as i16 + 1);
        am.updated_at = Set(cx.clock.now);
        am.update(conn).await.map_err(AppError::from)?;
    }
    cx.touch(ChangeCategory::Catalog);
    Ok(())
}
