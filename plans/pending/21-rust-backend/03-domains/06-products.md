# 21 · 03.06 — `products` (catalog: products, categories, units, price lists, custom fields)

> **Status:** planned 2026-09-28, not implemented. Wave **W2** (entry file §4). Depends on: 01-settings
> (settings row, `default_branch_id`, taxes, `inventory_approval_threshold`), 03-users (`User.price_list_id`
> guard, manager-PIN grant G-P3), Part 02 `shared::stock` / `shared::activity` / `core::lock`, and the
> manager tasks G-P1…G-P10 in §7.
>
> **Split (size rule):** the `products` domain is one Rust module, `domains/products/`, planned in two files
> because one file would pass ~600 lines. **This file = the catalog half** (`productService.ts`,
> `catalogService.ts`, 22 commands). **[`06b-inventory.md`](06b-inventory.md) = the inventory half**
> (`inventoryService.ts`, `transferService.ts`: adjustments, movements, batches/expiry, stock counts, branch
> transfers, 25 commands). One implementer does both, in the order given by §9 of each file
> (06 §9 C1–C9 → 06b §9 all → 06 §9 C10).

**Goal.** Port the 22 catalog `port` functions to `domains/products/` so each returns exactly the DTO the
mock returns. Product create/update, SKU/barcode uniqueness and the opening-stock adjustment become one
DB transaction; every stock/average-cost write goes through `shared::stock` (rule 3). Nothing about the
catalog numbers changes; the only behaviour changes are the ones listed in §7 (atomicity, DB backstops).

**Read first.** [`../01-frontend-analysis/products.md`](../01-frontend-analysis/products.md) §1 (rows 24–46),
§2, §3 (rows 123–151), §4, §5 (rows 220–222, 227), §6, §8, §9 — the primary input, cited not re-derived ·
mock: `src/modules/products/services/productService.ts:10-170`, `catalogService.ts:10-215` ·
types: `src/modules/products/types/index.ts:1-185` · Rust: `src-tauri/src/shared/stock/mod.rs`
(`lock_product`, `lock_products`, `init_product_stock`, `set_cost_when_empty`), `entities/catalog/{products,
categories,units,price_lists,custom_field_defs,product_prices,product_branch_stock}.rs`,
`migration/src/m0005_catalog.rs`, `utils/text.rs` (`normalize_arabic`, `like_contains`),
`entities/soft_delete.rs` (`find_live`, `soft_delete`), `core/error.rs` (`map_unique`).

## 1. Commands

Area/Access from the calling pages (`products/routes/index.ts:6-26` → area `inventory`;
`settings/pages/ProductsSettingsPage.vue` is the only caller of the custom-field/preset writes → area
`settings`). Reads run in `with_read` and call `crate::core::settings::require(tx, actor.as_ref(), area,
Access::Read)` first (actor cloned from `state.session` before the closure, same pattern as 03-users §1,
because `with_read` has no `TxCtx` — G-P6). Writes run in `with_tx(TxOpts::default())` and call
`cx.require(tx, area, Access::Write)` first.

| Mock fn (file:line) | Disp. | Rust command | Args → Return | Area / Access | Tx | Events (`cx.touch`) |
|---|---|---|---|---|---|---|
| `isLowStock` (`productService.ts:10`) | frontend | — | untouched | — | — | — |
| `getProducts` (`:14`) | port | `products_get_products` | `ProductsGetProductsArgs { filter: Option<ProductFilter> }` → `Vec<Product>` | Inventory/Read | read | — |
| `getProduct` (`:28`) | port | `products_get_product` | `ProductsGetProductArgs { id }` → `Product` | Inventory/Read | read | — |
| `findByCode` (`:36`) | port | `products_find_by_code` | `ProductsFindByCodeArgs { code }` → `Option<Product>` | Inventory/Read | read | — |
| `generateEan13` (`:109`) | port | `products_generate_ean13` | `()` → `String` | Inventory/Write | read | — |
| `createProduct` (`:123`) | port | `products_create_product` | `ProductsCreateProductArgs { input: ProductInput }` → `Product` | Inventory/Write | tx | Catalog (+ Ledger via opening post) |
| `updateProduct` (`:141`) | port | `products_update_product` | `ProductsUpdateProductArgs { id, input: ProductInput }` → `Product` | Inventory/Write | tx | Catalog |
| `suggestSku` (`:163`) | port | `products_suggest_sku` | `ProductsSuggestSkuArgs { prefix }` → `String` | Inventory/Read | read | — |
| `getCategories` (`catalogService.ts:19`) | port | `products_get_categories` | `()` → `Vec<CategoryWithCount>` | Inventory/Read | read | — |
| `saveCategory` (`:24`) | port | `products_save_category` | `ProductsSaveCategoryArgs { name, id?, defaults?: CategoryDefaults }` → `Category` | Inventory/Write | tx | Catalog |
| `deleteCategory` (`:40`) | port | `products_delete_category` | `ProductsDeleteCategoryArgs { id }` → `()` | Inventory/Write | tx | Catalog |
| `getUnits` (`:49`) | port | `products_get_units` | `()` → `Vec<UnitWithCount>` | Inventory/Read | read | — |
| `saveUnit` (`:54`) | port | `products_save_unit` | `ProductsSaveUnitArgs { name, id?, extra?: UnitExtra }` → `Unit` | Inventory/Write | tx | Catalog |
| `applyUnitPreset` (`:93`) | port | `products_apply_unit_preset` | `ProductsApplyUnitPresetArgs { kind: UnitPresetKind }` → `Vec<Unit>` | Settings/Write | tx | Catalog only if ≥1 created |
| `deleteUnit` (`:109`) | port | `products_delete_unit` | `ProductsDeleteUnitArgs { id }` → `()` | Inventory/Write | tx | Catalog |
| `getPriceLists` (`:118`) | port | `products_get_price_lists` | `()` → `Vec<PriceList>` | Inventory/Read | read | — |
| `savePriceList` (`:123`) | port | `products_save_price_list` | `ProductsSavePriceListArgs { input: PriceListInput, id? }` → `PriceList` | Inventory/Write | tx | Catalog |
| `deletePriceList` (`:139`) | port | `products_delete_price_list` | `ProductsDeletePriceListArgs { id }` → `()` | Inventory/Write | tx | Catalog |
| `setPriceListValues` (`:150`) | port | `products_set_price_list_values` | `ProductsSetPriceListValuesArgs { priceListId, values: BTreeMap<String, Option<Decimal>> }` → `()` | Inventory/Write | tx | Catalog |
| `getCustomFieldDefs` (`:172`) | port | `products_get_custom_field_defs` | `()` → `Vec<CustomFieldDef>` | Inventory/Read | read | — |
| `saveCustomFieldDef` (`:179`) | port | `products_save_custom_field_def` | `ProductsSaveCustomFieldDefArgs { input: CustomFieldDefInput, id? }` → `CustomFieldDef` | Settings/Write | tx | Catalog |
| `deleteCustomFieldDef` (`:199`) | port | `products_delete_custom_field_def` | `ProductsDeleteCustomFieldDefArgs { id }` → `()` | Settings/Write | tx | Catalog |
| `reorderCustomFieldDefs` (`:208`) | port | `products_reorder_custom_field_defs` | `ProductsReorderCustomFieldDefsArgs { orderedIds: Vec<Id> }` → `()` | Settings/Write | tx | Catalog |
| `onCatalogChanged` (`:218`) | frontend (event subscription, not `wrap`ped) | — | untouched | — | — | — |

22 commands. `generateEan13` is marked Write because only the product form (write access) calls it
(`UnitsEditor.vue`); it runs in `with_read` (it reads only). `getPriceLists` is also called by
`UserEditorPage`/`UserListPage` (admin — has Inventory/Read). `getCustomFieldDefs` stays Inventory/Read
because `ProductFormPage` reads it too.

## 2. DTOs (`domains/products/dto/catalog.rs`, every type `#[ts(export_to = "products/types/gen/")]`)

Conventions (Part 02 `core/dto.rs` header, P2-33): `#[serde(rename_all = "camelCase")]`;
`#[serde_with::skip_serializing_none]` + `#[ts(optional)]` on every `Option` (absent, never `null`);
`Decimal` → `#[serde(with = "crate::utils::money::serde_number")]` (`::option` for `Option`) +
`#[ts(type = "number")]`; `Id` → `#[ts(type = "string")]`; request structs also `#[serde(default)]` on
every `Option`. Args structs derive `Deserialize, Clone, TS`.

| Rust DTO | TS type (file:line) | Field notes |
|---|---|---|
| `Product` | `Product` (`types/index.ts:66-141`) | `r#type: ProductType`; `stock_mode: Option<StockMode>`; `cost_price`/`price`/`stock_qty`/`stock_value` `Decimal`; `min_stock`/`reorder_qty`/`min_price`/`weight` `Option<Decimal>`; `prices: Option<Vec<ProductPrice>>` from `product_prices` rows with `unit_id IS NULL` ordered `created_at, id` (absent when no rows — the mock's `normalize` always writes an array, so **create/update keep `Some(vec![])`**: see §3 C-6 note); `stock_by_branch: Option<BTreeMap<String, BranchStock>>` built from `product_branch_stock` rows (absent when none, like the mock before the first movement); `units: Option<Vec<ProductUnit>>` / `unit_prices: Option<Vec<ProductUnitPrice>>` from the JSON columns; `custom_fields: Option<BTreeMap<String, serde_json::Value>>` with `#[ts(optional, type = "Record<string, string \| number \| boolean \| undefined>")]`; `tags`/`image_ids: Option<Vec<String>>`; `expiry_alert_days`/`warranty_months: Option<i32>` `#[ts(type = "number")]`. |
| `ProductType` / `StockMode` / `WarrantyProvider` | `types/index.ts:1,11,138` | `#[serde(rename_all = "lowercase")]`. Entity stores the same strings; parse with `match`, unknown → `AppError::internal`. |
| `ProductUnit` / `ProductUnitPrice` | `:14-35` | Reuse the entity structs' field shapes (`entities/catalog/products.rs:17-47`) but as DTO structs with `#[derive(TS)]` (the entity structs have no TS). `unit_id` `Id`; `factor`/`price`/`value` `Decimal`. |
| `ProductPrice` | inline `{ priceListId: string; value: number }` (`:83`) | Input side: `value: Option<Decimal>` with `serde_number::option` + `#[ts(type = "number")]` (not optional) — the form may send `null`, which `normalize` filters (`productService.ts:102`); the TS type stays `number`. |
| `BranchStock` | inline `{ qty: number; value: number }` (`:103`) | both `Decimal`. |
| `ProductInput` | `ProductInput` (`:143-146`) | Every `Product` field except `id`/`stockQty`/`stockValue`, plus `opening_qty: Option<Decimal>`; **includes** `stock_by_branch` (the `Omit` keeps it) which the service ignores. Unknown extra keys (the import descriptor sends `{...product}`, `descriptors.ts:298`) are ignored by serde (no `deny_unknown_fields`). |
| `ProductFilter` | `:148-154` | all optional; `type` → `Option<ProductType>`. |
| `Category` / `CategoryDefaults` | `:156-166`; `Partial<Pick<Category,…>>` (`catalogService.ts:24`) | `CategoryDefaults` = the five `Option<Id>` account/tax fields. |
| `CategoryWithCount` / `UnitWithCount` | `Category & { productCount }` / `Unit & { productCount }` (`catalogService.ts:19,49`) | `#[serde(flatten)] base` + `product_count: u32` `#[ts(type = "number")]`. ts-rs flattens into one object literal. |
| `Unit` / `UnitExtra` | `:169-174`; `Partial<Pick<Unit,'symbol'\|'allowsDecimals'>>` | |
| `UnitPresetKind` | `:177` | `#[serde(rename_all = "lowercase")]`. |
| `PriceList` / `PriceListInput` | `:179-185`; `{ name; active }` (`catalogService.ts:123`) | `currency: Option<String>` (never written here). |
| `CustomFieldDef` / `CustomFieldDefInput` / `CustomFieldType` | `:54-64`; `catalogService.ts:177` | `sort_order: i16` `#[ts(type = "number")]`; `options: Option<Vec<String>>`; type `lowercase`. |

`contract.check.ts` (`src/modules/products/types/contract.check.ts`, new; 06b appends its own lines) —
one `Expect<Equals<Gen…, …>>` each for: `Product`, `ProductUnit`, `ProductUnitPrice`, `ProductInput`,
`ProductFilter`, `Category`, `Unit`, `UnitPresetKind`, `PriceList`, `CustomFieldDef`, `CustomFieldType`,
`CustomFieldDefInput` (import from `../services/catalogService`), and
`Equals<GenCategoryWithCount, Flat<Category & { productCount: number }>>` /
`Equals<GenUnitWithCount, Flat<Unit & { productCount: number }>>` with a file-local
`type Flat<T> = { [K in keyof T]: T[K] }` (the identity `Equals` treats an intersection and its flattened
object as different). If a pair can't be made equal, change the **Rust** DTO (entry §3.4).

## 3. Service logic (`domains/products/service/{products,catalog}.rs`)

Common: `now = cx.clock.now`; product reads use plain `products::Entity::find()` (products is not in the
soft-delete rule list, `tests/architecture_rules.rs:47`); categories/units/price lists/custom field defs
read **only** through `SoftDelete::find_live()`; list order = `ORDER BY created_at, id` (mock array
order) unless stated. `fn product_dto(conn, model) -> TxResult<Product>` assembles `prices` +
`stock_by_branch` (two child reads; a batch variant `product_dtos(conn, models)` does one `IN` query per
child table for lists).

**C-1 `get_products(filter)`** (`productService.ts:14-26`): SQL `WHERE (include_inactive OR active)
[AND category_id = ?] [AND type = ?] [AND type='product' AND COALESCE(stock_mode,'tracked') <> 'none' AND
stock_qty <= COALESCE(min_stock, 0)] [AND search_normalized LIKE like_contains(normalize_arabic(search))]`
— the low-stock clause is the ported `isLowStock` predicate (`:10-12`), the search clause replaces
`includesText([name, sku, barcode])` via the P2-38 column that `products::ActiveModel::before_save`
fills (so every product write in this file goes through `ActiveModelTrait::insert/update`, never
`Entity::update(..).exec`). Empty/whitespace search → no clause (`matchesSearch` returns true,
`search.ts:37-38`).

**C-2 `get_product(id)`** (`:28-33`): missing → `NOT_FOUND` `المنتج غير موجود`.

**C-3 `find_by_code(code)`** (`:36-41`): `c = code.trim()`; first row (by `created_at, id`) with
`active AND (barcode = c OR LOWER(sku) = LOWER(c))` (table collation `utf8mb4_bin`, so `barcode` is exact);
none → `Ok(None)` (a scan miss is not an error). Unit barcodes are **not** searched (mock quirk Q-1).

**C-4 `generate_ean13()`** (`:109-121`): up to 20 attempts: body = `"628"` + 9 random digits
(`getrandom::fill` → `u32 % 1_000_000_000`, zero-padded); check digit = `(10 − Σ dᵢ·(i even ? 1 : 3) mod
10) mod 10`; free when no row has `barcode = code` and no row's `units` JSON contains it
(`JSON_SEARCH(units, 'one', ?, NULL, '$[*].barcodes[*]') IS NULL`). After 20 → `VALIDATION`
`تعذر توليد باركود فريد — حاول مرة أخرى`.

**C-5 `validate(input, except_id, existing)`** — shared by create/update, exact order of
`productService.ts:68-88` then `validateUnits` `:48-66`:
1. `name.trim()` empty → `اسم المنتج مطلوب`. 2. `sku.trim()` empty → `رمز المنتج (SKU) مطلوب`.
3. another product (`id <> except`) with `LOWER(sku) = LOWER(TRIM(input.sku))` → `CONFLICT`
`رمز المنتج مستخدم لمنتج آخر`.
4. `input.barcode` non-empty (as sent, untrimmed — `:74`) and another product has `barcode = input.barcode`
→ `CONFLICT` `الباركود مستخدم لمنتج آخر`.
5. for each unit, each non-empty barcode `bc` found in another product's `units[*].barcodes` →
`CONFLICT` `` الباركود "${bc}" مستخدم في منتج آخر `` (first hit in input order).
6. `price < 0 || cost_price < 0` → `الأسعار لا يمكن أن تكون سالبة`.
7. `min_price` present and `> price` → `الحد الأدنى للسعر أكبر من سعر البيع`.
8. `validateUnits` (skipped when `units` absent/empty): any `factor <= 0` → `عامل تحويل الوحدة يجب أن يكون
أكبر من صفر`; count of `factor == 1` ≠ 1 → `يجب أن تكون وحدة واحدة فقط بعامل تحويل = 1 (الوحدة الأساسية)`;
`existing.stock_qty > 0.0001` and an existing unit (same `id`) whose factor changed →
`` لا يمكن تغيير عامل تحويل وحدة "${oldUnit.unitId}" بعد تحرك المخزون — أضف وحدة جديدة وعطّل القديمة بدلاً من ذلك ``;
duplicate non-empty barcodes across the input's own units → `نفس الباركود مستخدم أكثر من مرة في وحدات هذا المنتج`.
All untagged errors are `VALIDATION`. Steps 3–5 run after the G-P4 product-code lock (§4).

**C-6 `normalize(input)`** (`:90-106`): trim `name`/`sku`; `name_en`/`barcode` trimmed-or-`None`;
`category_id`/`unit_id` empty → `None`; `min_stock = None` for `service`; `cost_price` for a service =
`cost_price` (the mock's `?? 0` is a no-op because the field is required); `prices` = entries whose
`value` is `Some` (always `Some(vec)`, possibly empty); units' barcodes filtered non-empty; `tags` filtered
non-empty. `opening_qty` and `stock_by_branch` are dropped.

**C-7 `create_product(input)`** (`:123-139`), in one transaction:
1. `cx.require(Inventory, Write)`. 2. `numbering::lock(SequenceLock::ProductCodes)` (G-P4). 3. C-5 with
`except_id = None`. 4. C-6; build `products::ActiveModel` (new `Id`, `sync_status = local`) with
`shared::stock::init_product_stock(&mut am, normalized.cost_price)` (qty 0, value 0, cost = input cost);
`am.insert(conn)` — a `uq_products_sku_live` violation maps via `AppError::map_unique("uq_products_sku_live",
|| "رمز المنتج مستخدم لمنتج آخر")`, `uq_products_barcode_live` (G-P4) via the barcode message.
5. insert `product_prices` rows (`unit_id NULL`) in input order.
6. if `type == product && stock_mode != none && opening_qty > 0` → 06b
`service::adjustments::record_stock_adjustment(conn, cx, reg, StockAdjustmentInput { type: STOCK_IN, date:
DocDate { day: cx.clock.today(), instant: Some(now) }, note: "رصيد افتتاحي — {name}", reason: opening,
lines: [{ productId, qtyChange: opening_qty }] }, as_draft = false, approval = ApprovalCheck::none())`
(`:130-135`; unit cost = the product's `cost_price`; posts `Dr inventory / Cr openingBalanceEquity`, A3).
Its activity row is written **before** the product's (mock order).
7. `shared::activity::log(conn, cx, reg, Product, "إضافة المنتج {name}", Some(DocDate::now), Some(RouteRef::detail("product", id)))`.
8. `cx.touch(Catalog)`; return `product_dto` re-read after the adjustment (stock fields reflect the opening).

**C-8 `update_product(id, input)`** (`:141-160`):
1. require. 2. `numbering::lock(ProductCodes)`. 3. `shared::stock::lock_product(conn, id)` (row lock,
serialises with concurrent stock movements; missing → `NOT_FOUND` `المنتج غير موجود`, same text as `:144`).
4. C-5 with `except_id = Some(id)`, `existing = locked.model`. 5. `input.type != current type &&
stock_qty != 0` → `لا يمكن تحويل منتج له رصيد مخزون إلى خدمة — صفّر المخزون أولاً`.
6. C-6; update **every** `ProductInput` column except `cost_price` from the normalized input (full replace:
absent optional → cleared — D-P3) via `into_active_model()` + `Set` + `.update(conn)`; never `Set`
`stock_qty`/`stock_value`/`cost_price` here (architecture rule D-5).
7. `shared::stock::set_cost_when_empty(conn, cx, &mut locked, normalized.cost_price)` — applies only when
`stock_qty <= 0.0001`, silent no-op otherwise (`:150-155`).
8. replace `product_prices` (`unit_id NULL`) rows: delete all for the product, insert input order.
9. `log(Product, "تعديل المنتج {name}", …, detail("product", id))`; `touch(Catalog)`; return `product_dto`.

**C-9 `suggest_sku(prefix)`** (`:163-170`): `SELECT sku FROM products WHERE sku LIKE CONCAT(?, '-%')`
(escaped prefix, bin collation = JS `startsWith`); suffix parsed as integer, non-numeric → 0;
`format!("{prefix}-{:03}", max(0, …) + 1)`. Reads only (D8 note §4).

**C-10 categories** (`catalogService.ts:10-45`): `assert_name(table, name, except_id)` = `trim`, empty →
`الاسم مطلوب`; live row with `name COLLATE utf8mb4_bin = trimmed AND id <> except` → `CONFLICT`
`الاسم مستخدم من قبل` (exact, like `x.name.trim() === trimmed`, `:13`).
- `get_categories`: live rows + `product_count` = `COUNT(*)` of products with that `category_id` (all
  products, active or not — `:21`).
- `save_category(name, id, defaults)`: name check **before** existence (`:26-29`); update: live row
  missing → `NOT_FOUND` `التصنيف غير موجود`; set `name`, and each `defaults` field that is present
  (merge — `Object.assign` with keys the IPC JSON dropped = keep, D-P3); create: new row. Unique violation
  on `uq_categories_name_live` → the same `CONFLICT` text.
- `delete_category(id)`: `core::lock::for_update_by_id("categories", id)`; any product with that
  `category_id` → `CONFLICT` `لا يمكن حذف تصنيف مرتبط بمنتجات`; else `soft_delete` (P2-16). A missing id is
  a silent no-op (the mock filters nothing, `:43`).

**C-11 units** (`:49-114`): same pattern; `get_units` counts products whose **base** `unit_id` matches
(`:51`); `save_unit` → `الوحدة غير موجودة`; `extra` merge for `symbol`/`allows_decimals`;
`delete_unit` refuses `لا يمكن حذف وحدة مرتبطة بمنتجات` (base unit only, `:111`).
`apply_unit_preset(kind)`: the three preset lists copied verbatim from `:71-91` into a `const` table;
existing live names (exact) skipped; insert the rest in list order; return only the created ones;
`touch(Catalog)` only when ≥1 created (`:105`).

**C-12 price lists** (`:118-166`): `get_price_lists` live rows. `save_price_list(input, id)`:
`assert_name` → update (`قائمة الأسعار غير موجودة`) sets `name` + `active`, or create.
`delete_price_list(id)`: lock the row; any user with `price_list_id = id` → `CONFLICT`
`قائمة الأسعار مسندة لمستخدمين — أزل الإسناد أولاً`; then `DELETE FROM product_prices WHERE price_list_id
= ?` (the mock's cascade `:144`) and `soft_delete` the list. `set_price_list_values(list_id, values)`:
live list missing → `NOT_FOUND` `قائمة الأسعار غير موجودة`; for each `(product_id, value)` **in the map's
iteration order** (`BTreeMap` — see Q-4): product missing → skip; `None` → delete that product's
`(list, unit NULL)` row; `Some(v)`: `v < 0` → `` سعر "${product.name}" لا يمكن أن يكون سالباً `` (whole
transaction rolls back — the atomicity the analysis requires, row 42); else delete the old row and insert
a new one (a new `Id` sorts last — the mock appends, `:161`).

**C-13 custom fields** (`:172-215`): `get_custom_field_defs` ordered `sort_order, created_at, id`.
`save_custom_field_def(input, id)`: `name.trim()` empty → `اسم الحقل مطلوب`; `type = list` with no
options → `أضف خيارات لحقل من نوع قائمة`; update: missing → `NOT_FOUND` `الحقل غير موجود`, set
`name` (trimmed), `type`, `active`, and `options` when present (merge); create: `sort_order = COUNT(live) + 1`
(`:191`). `delete_custom_field_def(id)`: lock the row; any product with
`JSON_CONTAINS_PATH(custom_fields, 'one', CONCAT('$."', ?, '"'))` → `CONFLICT`
`لا يمكن حذف حقل مستخدم في بيانات منتجات — عطّله بدلاً من ذلك`; else `soft_delete`.
`reorder_custom_field_defs(ids)`: for index `i`, set `sort_order = i + 1` where the live row exists (others
keep their order, `:210-213`).

No activity/audit rows for C-10…C-13 (analysis §6: the mock writes none; D-P4).

## 4. Concurrency (D8)

Effective lock order used by every command in 06/06b/07 (follows `core/lock.rs`'s documented order; the
entry file §3.3 wording differs — G-P8): document row(s) → product rows (`lock_products`, sorted by id)
→ domain document counters (`next_number`) → inside `ledger::post`: settings (S) → fiscal year (S) →
journal counter → `change_versions` (by `with_tx`).

- **SKU / barcode / unit-barcode races** (analysis §5 row 220): C-7/C-8 first take
  `numbering::lock(SequenceLock::ProductCodes)` (new lock row, G-P4) so the three pre-checks and the write
  are serialised across terminals; `uq_products_sku_live` and the new `uq_products_barcode_live` are the DB
  backstop; unit barcodes live in JSON (no index possible), so the lock is their only guard.
- **Update vs stock movement:** C-8 holds `lock_product` (the same `FOR UPDATE` every stock writer takes),
  so the type-change guard and `set_cost_when_empty` see the committed `stock_qty`.
- **Category/unit/price-list names** (row 221): exact pre-check + `uq_*_name_live` (G-P4 changes its
  collation to `utf8mb4_bin` so the backstop matches the exact rule).
- **Delete vs concurrent assignment** (row 227): delete takes `FOR UPDATE` on the master row; the reference
  `COUNT` runs after it; a product saved concurrently with a just-deleted category keeps a soft-deleted id
  (FK still satisfied — soft delete never removes the row).
- **`suggest_sku`** (row 222): read-only suggestion, not a reservation; a duplicate is caught by the create
  path's lock + unique key (`CONFLICT`, user re-suggests). No numbering lock (D-P5).
- `sort_order = COUNT+1` for custom fields can duplicate under a race (display order only; accepted,
  analysis row 44).

## 5. Undo

Not undoable via the registry (phase-e E-5; analysis §4 rows 195–201). No `UndoSpec`, no compensator,
no `register_undo` in this half. The opening-stock adjustment follows 06b §5.

## 6. Frontend switch lines (dormant, entry §3.4)

`src/modules/products/services/productService.ts` — inside each `wrap`, first statement:
`getProducts` → `if (usesRust('products')) { const rows = await backendCall('products_get_products', { filter }); rememberBranchStock(rows); return rows; }`;
`getProduct` → same with `products_get_product` and `rememberBranchStock([row])`;
`findByCode` → `return backendCall('products_find_by_code', { code })`; `generateEan13` →
`backendCall('products_generate_ean13')`; `createProduct` → `{ input }`; `updateProduct` → `{ id, input }`;
`suggestSku` → `{ prefix }`. `rememberBranchStock` is a new 8-line exported helper in `productService.ts`
(a `Map<productId, stockByBranch>` filled only on the Rust path) that 06b's reclassified
`branchStockQty` reads (06b D-I1).
`catalogService.ts`: `getCategories`, `getUnits`, `getPriceLists`, `getCustomFieldDefs` →
`return backendCall('<cmd>')`; `saveCategory` → `{ name, id, defaults }`; `saveUnit` → `{ name, id, extra }`;
`applyUnitPreset` → `{ kind }`; `savePriceList` → `{ input, id }`; `saveCustomFieldDef` → `{ input, id }`;
`setPriceListValues` → `{ priceListId, values }`; the `Promise<void>` ones (`deleteCategory`,
`deleteUnit`, `deletePriceList`, `deleteCustomFieldDef`, `reorderCustomFieldDefs`,
`setPriceListValues`) use `{ await backendCall('<cmd>', {…}); return; }` (a `null` result is not
assignable to `void`). `isLowStock` and `onCatalogChanged` are not touched.

## 7. Known mock quirks (kept) · Decisions · Manager tasks

**Quirks kept (behaviour-exact, listed for a later decision):**
- Q-1 `findByCode` ignores unit barcodes (`:39`), and C-5 step 5 checks unit barcodes only against other
  products' **unit** barcodes, not their main `barcode` (and vice versa, `:74-84`).
- Q-2 `productCount` counts inactive products and only the base `unitId` (analysis row 35).
- Q-3 `deletePriceList` strips `prices` but leaves the list's entries inside `unitPrices` JSON (`:144`), and
  doesn't check `branches.default_price_list_id`.
- Q-4 `setPriceListValues` applies keys in JS insertion order; Rust uses `BTreeMap` (sorted) order. Only
  observable as *which* negative-price product is named when several are negative (the whole call fails
  either way). Accepted; parity case P-C9 uses one negative value.
- Q-5 create with an opening qty above the approval threshold: the mock pushes the product, then throws
  `FORBIDDEN` from the opening adjustment (product persists, no stock). Rust rolls the whole create back.
  Strictly safer; parity case P-C3 asserts the error and treats "product exists" as a mock-only artifact.
- Q-6 `sortOrder = length + 1` can collide after deletes (no gap-filling, `:191`).

**Decisions (strictest option, logged):**
- D-P1 `batchAlertTone`/`branchStockQty` are reclassified `frontend` in 06b (sync template calls) — see 06b D-I1.
- D-P2 Master-data deletes are soft deletes (P2-16); every read uses `find_live()`; the reference checks
  count live references only where the referencing table itself is live-filtered.
- D-P3 Update semantics: `updateProduct` is a full replace of `ProductInput` (both real callers send the full
  object, `ProductFormPage`, `descriptors.ts:298`); `saveCategory.defaults`/`saveUnit.extra`/
  `saveCustomFieldDef.options` merge (keys dropped by JSON = keep), matching `Object.assign` over IPC JSON.
- D-P4 Catalog master-data writes stay without audit rows (analysis §8/§9 left it as a policy question —
  kept at parity; see open question O-1).
- D-P5 `suggestSku` stays a plain read (no reservation); the create path's lock + unique key is the guard.
- D-P6 Product `image` (data URL) and `imageIds` are stored as today (C-16 still open).

**Open questions for the user:** O-1 add audit rows to the ten catalog master-data writes (analysis §9,
recommends yes)? O-2 should `findByCode` also match unit barcodes (POS scan of a carton barcode)?

**Manager tasks (Part 02 gaps found while planning 06/06b/07):**
- G-P1 `shared::totals`: port `computeInvoiceTotals` + `spreadProportionally` + `paymentStatusFor`
  (`invoices/helpers/totals.ts:87-237`) — needed by 07 and 08 in the same wave (W3). Before W3.
- G-P2 `shared::org`: `branch_prefix(conn, branch_id)` (`branches.ts:205-209`, counts live branches),
  `default_cost_center_for(conn, branch_id, explicit)` (`:264-268`), `purchase_tax_rate(conn)`
  (`core.ts:468-471`). Needed by 06b (transfer numbers, W2), 07, 08. Before W2.
- G-P3 Manager-PIN grants: `AppState.approval_grants` (approver `Id` → `Instant`, 10-minute TTL, per terminal
  process) with `grant(id)` / `is_granted(id) -> bool`; 03-users' `users_verify_manager_pin` calls `grant`
  on success. Used by 06b C-A4 (server-side re-verification the analysis §8 requires). Before W2.
- G-P4 Migration `m0016`: (a) `categories/units/price_lists.name_live` → `utf8mb4_bin` (P2-17 says exact
  names are `bin`; `m0005_catalog.rs:61-68` built them `unicode_ci`, which refuses "Food"/"food" that the
  mock allows); (b) `products.barcode_live` generated column + `uq_products_barcode_live`; (c) seed
  `document_counters` row `productCodesLock` + `SequenceLock::ProductCodes`; (d)
  `purchase_orders.received_date_instant` and `product_batches.received_date_instant` (+ generated `_key`)
  because the mock stores ISO instants there (`purchases.ts:297` via `confirmPurchaseOrder`,
  `inventory.ts:167` via opening/completion dates) — without them `receivedDate` loses the time part.
- G-P5 `architecture_rules` needle `"cost_price: Set("` (`tests/architecture_rules.rs:87`) also matches
  `purchase_order_lines.cost_price` / `purchase_return_lines.cost_price`; narrow it to product models before W3.
- G-P6 `with_read` has no `TxCtx` (no clock, no `require`): reads here use the 03-users pattern; a
  `with_read_ctx` would remove the boilerplate (optional).
- G-P7 Cross-domain DTOs needed by 07 before their owners' waves: `PaymentStatus` (TS in
  `invoices/types/index.ts:2`) and `Payment` (`payments/types/index.ts:28`) + a `payment_dtos(conn, ids)`
  reader (payments is W4, purchases W3).
- G-P8 Reconcile the lock-order text in entry §3.3 with `core/lock.rs` (this plan follows lock.rs, §4).
- G-P9 `tests/support`: a shared `seed_company(conn)` (settings row, default branch, fiscal year, the
  system-role accounts) for all domain tests.
- G-P10 `scripts/contract/config.ts` overrides: `products.batchAlertTone`, `products.branchStockQty` →
  `frontend` (06b D-I1). Part 04: flip `products` and `purchases` **together** (06b §6 note).

## 8. Tests

**(a) `src-tauri/tests/domain_products.rs`** (catalog part; `TestDb::fresh()`, G-P9 seed, logged-in admin;
posting tests end with `shared::invariants::run_all(conn)` all green):
- create: happy path returns the DTO (`prices: []`, `stockQty 0`); each C-5 message in order (name, sku,
  sku dup case-insensitive, barcode dup, unit-barcode dup, negative price, minPrice, factor ≤ 0, two base
  units, own-barcode dup); SKU dup through the unique key (bypass the pre-check) maps to the same text.
- create with `openingQty 5` at cost 10: one STOCK_IN `ADJ-` adjustment, movement `stock_in`, product
  qty 5 / value 50 / cost 10, journal `Dr inventory 50 / Cr openingBalanceEquity 50`, invariants green;
  activity order = adjustment row then product row.
- create with opening above `inventory_approval_threshold` → `FORBIDDEN` and **no** product row.
- update: type change with stock → message; cost edit ignored when stock > 0.0001, applied when 0; factor
  change after stock → message; `prices` replaced; `search_normalized` recomputed.
- find_by_code: barcode exact, SKU case-insensitive, inactive → `None`; generate_ean13 check digit valid.
- suggest_sku: `MEN-007` + `MEN-x` → `MEN-008`; empty → `MEN-001`.
- categories/units/price lists: exact-name conflict, "Food" vs "food" allowed (after G-P4a), delete guards,
  soft delete frees the name, `productCount`, unit presets idempotent.
- price lists: user-assigned delete refused; delete cascades `product_prices`; `setPriceListValues` null
  removes, negative rolls back everything.
- custom fields: list without options refused; delete referenced refused; reorder partial.
- concurrency: two parallel `create_product` with the same SKU → one `Product`, one `CONFLICT`; same unit
  barcode → one wins (G-P4c lock).

**(b) Parity cases for Part 04** (mock vs Rust, ids mapped): P-C1 list/filter/search incl. Arabic
normalization (`أحمد`/`احمد`, `٣`/`3`); P-C2 create without opening; P-C3 create with opening (and the
threshold error); P-C4 update (cost discard, type guard); P-C5 each C-5 validation message; P-C6
category/unit/price-list CRUD + counts; P-C7 unit preset twice; P-C8 price-list delete cascade; P-C9
`setPriceListValues` with null + one negative; P-C10 custom-field save/reorder/delete-guard.

## 9. Checklist (implementation order)

- [ ] C0 Confirm G-P2, G-P3, G-P4, G-P6, G-P9 are in place (manager, before W2).
- [ ] C1 `domains/products/mod.rs` (`pub mod commands; pub mod service; pub mod dto;`, `ipc_signatures()`,
      `export_bindings(cfg)`), `commands/mod.rs`, `service/mod.rs`, `dto/mod.rs`; ask the manager to add
      `pub mod products;` + hooks in `domains/mod.rs`.
- [ ] C2 `dto/catalog.rs`: every DTO in §2 + the 22 Args structs.
- [ ] C3 `service/products.rs`: `product_dto`/`product_dtos`, C-1…C-4, C-9.
- [ ] C4 `service/products.rs`: C-5 `validate`, C-6 `normalize`.
- [ ] C5 `service/products.rs`: C-8 `update_product`.
- [ ] C6 `service/catalog.rs`: `assert_name`, C-10, C-11 (+ preset table), C-12, C-13.
- [ ] C7 `commands/catalog.rs`: 22 thin commands (require → service), `ipc_sig!` lines.
- [ ] C8 `types/contract.check.ts` (catalog lines) and the 22 switch lines + `rememberBranchStock` (§6).
- [ ] C9 `tests/domain_products.rs` catalog tests (§8a).
- [ ] → implement **06b §9 in full**, then:
- [ ] C10 `service/products.rs`: C-7 `create_product` (calls 06b `record_stock_adjustment`) + its tests.

## Gate

`cargo check` clean (manager's throttled wave build); DB tests written (run in the deferred, time-boxed
pass); 22 switch lines present; `contract.check.ts` entries type-check after `bun run bindings`;
`bun run memory:check` shows the 22 commands invoked + registered, 0 contract gaps. DB tests and parity
cases run in the deferred pass (entry §3.6).
