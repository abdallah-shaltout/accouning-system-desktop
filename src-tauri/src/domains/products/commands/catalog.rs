//! `products` catalog IPC commands (06-products.md §1) — thin layer: parse args, authorize (Inventory
//! area for most; Settings for the custom-field/preset writes per §1's note), open a transaction,
//! call the service, map the error. Reads use `with_read_ctx` (actor cloned before the closure, per
//! the 03-users pattern — `with_read` has no `TxCtx`, G-P6); writes use `with_tx`.

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_read_ctx, with_tx, BoxFuture, TxOpts, TxResult};
use crate::utils::id::Id;

use super::super::dto::catalog::*;
use super::super::service::catalog as service;
use super::super::service::products as products_service;

type CmdResult<T> = Result<T, ApiErrorPayload>;

fn parse_id(raw: &str) -> Result<Id, ApiErrorPayload> {
    raw.parse::<Id>().map_err(|_| crate::core::error::AppError::validation("معرّف غير صالح").into())
}

fn parse_opt_id(raw: &Option<String>) -> Result<Option<Id>, ApiErrorPayload> {
    match raw {
        None => Ok(None),
        Some(s) => parse_id(s).map(Some),
    }
}

// --- Products ---------------------------------------------------------------------------------

#[tauri::command]
pub async fn products_get_products(state: State<'_, AppState>, args: ProductsGetProductsArgs) -> CmdResult<Vec<Product>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            products_service::get_products(tx, args.filter).await
        }) as BoxFuture<'_, TxResult<Vec<Product>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_get_product(state: State<'_, AppState>, args: ProductsGetProductArgs) -> CmdResult<Product> {
    let id = parse_id(&args.id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            products_service::get_product(tx, id).await
        }) as BoxFuture<'_, TxResult<Product>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_find_by_code(state: State<'_, AppState>, args: ProductsFindByCodeArgs) -> CmdResult<Option<Product>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            products_service::find_by_code(tx, &args.code).await
        }) as BoxFuture<'_, TxResult<Option<Product>>>
    })
    .await
    .map_err(Into::into)
}

/// Read-only despite being gated on Write (only the write-access product form calls it, §1 note).
#[tauri::command]
pub async fn products_generate_ean13(state: State<'_, AppState>) -> CmdResult<String> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Write).await?;
            products_service::generate_ean13(tx).await
        }) as BoxFuture<'_, TxResult<String>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_create_product(state: State<'_, AppState>, args: ProductsCreateProductArgs) -> CmdResult<Product> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            products_service::create_product(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<Product>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_update_product(state: State<'_, AppState>, args: ProductsUpdateProductArgs) -> CmdResult<Product> {
    let id = parse_id(&args.id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            products_service::update_product(tx, cx, &undo, id, input).await
        }) as BoxFuture<'_, TxResult<Product>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_suggest_sku(state: State<'_, AppState>, args: ProductsSuggestSkuArgs) -> CmdResult<String> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            products_service::suggest_sku(tx, &args.prefix).await
        }) as BoxFuture<'_, TxResult<String>>
    })
    .await
    .map_err(Into::into)
}

// --- Categories -------------------------------------------------------------------------------

#[tauri::command]
pub async fn products_get_categories(state: State<'_, AppState>) -> CmdResult<Vec<CategoryWithCount>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            service::get_categories(tx).await
        }) as BoxFuture<'_, TxResult<Vec<CategoryWithCount>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_save_category(state: State<'_, AppState>, args: ProductsSaveCategoryArgs) -> CmdResult<Category> {
    let id = parse_opt_id(&args.id)?;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let name = args.name.clone();
        let defaults = args.defaults.clone();
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            service::save_category(tx, cx, name, id, defaults).await
        }) as BoxFuture<'_, TxResult<Category>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_delete_category(state: State<'_, AppState>, args: ProductsDeleteCategoryArgs) -> CmdResult<()> {
    let id = parse_id(&args.id)?;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            service::delete_category(tx, cx, id).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(Into::into)
}

// --- Units ------------------------------------------------------------------------------------

#[tauri::command]
pub async fn products_get_units(state: State<'_, AppState>) -> CmdResult<Vec<UnitWithCount>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            service::get_units(tx).await
        }) as BoxFuture<'_, TxResult<Vec<UnitWithCount>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_save_unit(state: State<'_, AppState>, args: ProductsSaveUnitArgs) -> CmdResult<Unit> {
    let id = parse_opt_id(&args.id)?;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let name = args.name.clone();
        let extra = args.extra.clone();
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            service::save_unit(tx, cx, name, id, extra).await
        }) as BoxFuture<'_, TxResult<Unit>>
    })
    .await
    .map_err(Into::into)
}

/// Area **Settings**/Write — only `ProductsSettingsPage.vue` calls it (§1).
#[tauri::command]
pub async fn products_apply_unit_preset(state: State<'_, AppState>, args: ProductsApplyUnitPresetArgs) -> CmdResult<Vec<Unit>> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::apply_unit_preset(tx, cx, args.kind).await
        }) as BoxFuture<'_, TxResult<Vec<Unit>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_delete_unit(state: State<'_, AppState>, args: ProductsDeleteUnitArgs) -> CmdResult<()> {
    let id = parse_id(&args.id)?;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            service::delete_unit(tx, cx, id).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(Into::into)
}

// --- Price lists ------------------------------------------------------------------------------

/// Also called by `UserEditorPage`/`UserListPage` (admin — Inventory/Read granted) — stays a single
/// area (§1 note).
#[tauri::command]
pub async fn products_get_price_lists(state: State<'_, AppState>) -> CmdResult<Vec<PriceList>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            service::get_price_lists(tx).await
        }) as BoxFuture<'_, TxResult<Vec<PriceList>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_save_price_list(state: State<'_, AppState>, args: ProductsSavePriceListArgs) -> CmdResult<PriceList> {
    let id = parse_opt_id(&args.id)?;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            service::save_price_list(tx, cx, input, id).await
        }) as BoxFuture<'_, TxResult<PriceList>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_delete_price_list(state: State<'_, AppState>, args: ProductsDeletePriceListArgs) -> CmdResult<()> {
    let id = parse_id(&args.id)?;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            service::delete_price_list(tx, cx, id).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_set_price_list_values(state: State<'_, AppState>, args: ProductsSetPriceListValuesArgs) -> CmdResult<()> {
    let price_list_id = parse_id(&args.price_list_id)?;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let values = args.values.clone();
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            service::set_price_list_values(tx, cx, price_list_id, values).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(Into::into)
}

// --- Custom field defs --------------------------------------------------------------------------

/// Also read by `ProductFormPage` (Inventory/Read) — stays Inventory/Read (§1 note).
#[tauri::command]
pub async fn products_get_custom_field_defs(state: State<'_, AppState>) -> CmdResult<Vec<CustomFieldDef>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            service::get_custom_field_defs(tx).await
        }) as BoxFuture<'_, TxResult<Vec<CustomFieldDef>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_save_custom_field_def(state: State<'_, AppState>, args: ProductsSaveCustomFieldDefArgs) -> CmdResult<CustomFieldDef> {
    let id = parse_opt_id(&args.id)?;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::save_custom_field_def(tx, cx, input, id).await
        }) as BoxFuture<'_, TxResult<CustomFieldDef>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_delete_custom_field_def(state: State<'_, AppState>, args: ProductsDeleteCustomFieldDefArgs) -> CmdResult<()> {
    let id = parse_id(&args.id)?;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::delete_custom_field_def(tx, cx, id).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_reorder_custom_field_defs(state: State<'_, AppState>, args: ProductsReorderCustomFieldDefsArgs) -> CmdResult<()> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let ordered_ids = args.ordered_ids.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::reorder_custom_field_defs(tx, cx, ordered_ids).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(Into::into)
}
