//! `domains::settings::commands` — thin IPC layer (01-settings.md §1). Each command: args struct,
//! `require`, `with_tx`/`with_read`, map error. 33 commands total.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::Deserialize;
use tauri::State;
use ts_rs::TS;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_read, with_tx, BoxFuture, TxError, TxOpts, TxResult};
use crate::utils::id::Id;

use super::dto::*;
use super::service;

type CmdResult<T> = Result<T, ApiErrorPayload>;

// --- Store settings ------------------------------------------------------------------------------

#[tauri::command]
pub async fn settings_get_settings(state: State<'_, AppState>) -> CmdResult<StoreSettings> {
    let device = state.device.read().unwrap().clone();
    with_read(&state, move |conn| {
        Box::pin(async move { service::store::get_settings(conn, &device).await.map_err(TxError::App) }) as BoxFuture<'_, TxResult<StoreSettings>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsUpdateSettingsArgs {
    pub patch: StoreSettingsPatch,
}

#[tauri::command]
pub async fn settings_update_settings(state: State<'_, AppState>, args: SettingsUpdateSettingsArgs) -> CmdResult<StoreSettings> {
    // D-3: a patch touching roleAccessOverrides also needs Users:Write.
    let touches_role_overrides = args.patch.role_access_overrides.is_some();
    let old_device = state.device.read().unwrap().clone();
    let undo = state.undo.clone();

    // D-6: the transaction computes the device delta; only on a successful commit is it applied
    // to AppState/device-settings.json — an error path never half-writes the device file.
    let patch = args.patch;
    let delta = with_tx(&state, TxOpts::default(), move |tx, cx| {
        let patch = patch.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            if touches_role_overrides {
                cx.require(tx, Area::Users, Access::Write).await?;
            }
            let (_row, delta) = service::store::update_settings(tx, cx, &undo, patch).await?;
            Ok(delta)
        }) as BoxFuture<'_, TxResult<service::store::DeviceDelta>>
    })
    .await
    .map_err(ApiErrorPayload::from)?;

    if !delta.is_empty() {
        let mut new_device = old_device.clone();
        delta.apply(&mut new_device);
        if let Err(e) = crate::core::device::save(&state.app_data_dir, &new_device) {
            log::error!("failed to save device-settings.json after settings_update_settings: {e}");
        }
        *state.device.write().unwrap() = new_device;
    }

    let device = state.device.read().unwrap().clone();
    with_read(&state, move |conn| {
        Box::pin(async move { service::store::get_settings(conn, &device).await.map_err(TxError::App) }) as BoxFuture<'_, TxResult<StoreSettings>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

// --- Taxes -----------------------------------------------------------------------------------------

#[tauri::command]
pub async fn settings_get_taxes(state: State<'_, AppState>) -> CmdResult<Vec<Tax>> {
    with_read(&state, |conn| Box::pin(async move { service::taxes::list(conn).await.map_err(TxError::App) }) as BoxFuture<'_, TxResult<Vec<Tax>>>)
        .await
        .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsSaveTaxArgs {
    pub input: TaxInput,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
}

#[tauri::command]
pub async fn settings_save_tax(state: State<'_, AppState>, args: SettingsSaveTaxArgs) -> CmdResult<Tax> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let id = args.id;
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::taxes::save(tx, cx, &undo, input, id).await
        }) as BoxFuture<'_, TxResult<Tax>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsDeleteTaxArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[tauri::command]
pub async fn settings_delete_tax(state: State<'_, AppState>, args: SettingsDeleteTaxArgs) -> CmdResult<()> {
    let id = args.id;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::taxes::delete(tx, cx, &undo, id).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

// --- Payment methods -------------------------------------------------------------------------------

#[tauri::command]
pub async fn settings_get_payment_methods(state: State<'_, AppState>) -> CmdResult<Vec<PaymentMethod>> {
    with_read(&state, |conn| Box::pin(async move { service::payment_methods::list(conn).await.map_err(TxError::App) }) as BoxFuture<'_, TxResult<Vec<PaymentMethod>>>)
        .await
        .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsSavePaymentMethodArgs {
    pub input: PaymentMethodInput,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
}

#[tauri::command]
pub async fn settings_save_payment_method(state: State<'_, AppState>, args: SettingsSavePaymentMethodArgs) -> CmdResult<PaymentMethod> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let id = args.id;
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::payment_methods::save(tx, cx, &undo, input, id).await
        }) as BoxFuture<'_, TxResult<PaymentMethod>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsReorderPaymentMethodsArgs {
    #[ts(type = "string[]")]
    pub ordered_ids: Vec<Id>,
}

#[tauri::command]
pub async fn settings_reorder_payment_methods(state: State<'_, AppState>, args: SettingsReorderPaymentMethodsArgs) -> CmdResult<()> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let ids = args.ordered_ids.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::payment_methods::reorder(tx, ids).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsDeletePaymentMethodArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[tauri::command]
pub async fn settings_delete_payment_method(state: State<'_, AppState>, args: SettingsDeletePaymentMethodArgs) -> CmdResult<()> {
    let id = args.id;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::payment_methods::delete(tx, cx, &undo, id).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

// --- Branches ----------------------------------------------------------------------------------

#[tauri::command]
pub async fn settings_get_branches(state: State<'_, AppState>) -> CmdResult<Vec<Branch>> {
    with_read(&state, |conn| Box::pin(async move { service::branches::list(conn).await.map_err(TxError::App) }) as BoxFuture<'_, TxResult<Vec<Branch>>>)
        .await
        .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsCreateBranchArgs {
    pub input: BranchInput,
}

#[tauri::command]
pub async fn settings_create_branch(state: State<'_, AppState>, args: SettingsCreateBranchArgs) -> CmdResult<Branch> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::branches::create(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<Branch>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsUpdateBranchArgs {
    #[ts(type = "string")]
    pub id: Id,
    pub input: BranchPatch,
}

#[tauri::command]
pub async fn settings_update_branch(state: State<'_, AppState>, args: SettingsUpdateBranchArgs) -> CmdResult<Branch> {
    let id = args.id;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::branches::update(tx, cx, &undo, id, input).await
        }) as BoxFuture<'_, TxResult<Branch>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsDeactivateBranchArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[tauri::command]
pub async fn settings_deactivate_branch(state: State<'_, AppState>, args: SettingsDeactivateBranchArgs) -> CmdResult<Branch> {
    let id = args.id;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::branches::deactivate(tx, cx, &undo, id).await
        }) as BoxFuture<'_, TxResult<Branch>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsReactivateBranchArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[tauri::command]
pub async fn settings_reactivate_branch(state: State<'_, AppState>, args: SettingsReactivateBranchArgs) -> CmdResult<Branch> {
    let id = args.id;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::branches::reactivate(tx, cx, &undo, id).await
        }) as BoxFuture<'_, TxResult<Branch>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

// --- Cost centers ------------------------------------------------------------------------------

#[tauri::command]
pub async fn settings_get_cost_centers(state: State<'_, AppState>) -> CmdResult<Vec<CostCenter>> {
    with_read(&state, |conn| Box::pin(async move { service::cost_centers::list(conn).await.map_err(TxError::App) }) as BoxFuture<'_, TxResult<Vec<CostCenter>>>)
        .await
        .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsCreateCostCenterArgs {
    pub input: CostCenterInput,
}

#[tauri::command]
pub async fn settings_create_cost_center(state: State<'_, AppState>, args: SettingsCreateCostCenterArgs) -> CmdResult<CostCenter> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::cost_centers::create(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<CostCenter>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsUpdateCostCenterArgs {
    #[ts(type = "string")]
    pub id: Id,
    pub input: CostCenterPatch,
}

#[tauri::command]
pub async fn settings_update_cost_center(state: State<'_, AppState>, args: SettingsUpdateCostCenterArgs) -> CmdResult<CostCenter> {
    let id = args.id;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::cost_centers::update(tx, cx, &undo, id, input).await
        }) as BoxFuture<'_, TxResult<CostCenter>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsDeleteCostCenterArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[tauri::command]
pub async fn settings_delete_cost_center(state: State<'_, AppState>, args: SettingsDeleteCostCenterArgs) -> CmdResult<()> {
    let id = args.id;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::cost_centers::delete(tx, cx, &undo, id).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

// --- Currencies ----------------------------------------------------------------------------------

#[tauri::command]
pub async fn settings_get_currencies(state: State<'_, AppState>) -> CmdResult<Vec<Currency>> {
    with_read(&state, |conn| Box::pin(async move { service::currency::list_currencies(conn).await.map_err(TxError::App) }) as BoxFuture<'_, TxResult<Vec<Currency>>>)
        .await
        .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsGetExchangeRatesArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
}

#[tauri::command]
pub async fn settings_get_exchange_rates(state: State<'_, AppState>, args: SettingsGetExchangeRatesArgs) -> CmdResult<Vec<ExchangeRate>> {
    with_read(&state, move |conn| {
        Box::pin(async move { service::currency::list_exchange_rates(conn, args.currency.clone()).await.map_err(TxError::App) }) as BoxFuture<'_, TxResult<Vec<ExchangeRate>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsCreateCurrencyArgs {
    pub input: Currency,
}

#[tauri::command]
pub async fn settings_create_currency(state: State<'_, AppState>, args: SettingsCreateCurrencyArgs) -> CmdResult<Currency> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::currency::create(tx, cx, input).await
        }) as BoxFuture<'_, TxResult<Currency>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsUpdateCurrencyArgs {
    pub code: String,
    pub patch: CurrencyPatch,
}

#[tauri::command]
pub async fn settings_update_currency(state: State<'_, AppState>, args: SettingsUpdateCurrencyArgs) -> CmdResult<Currency> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let patch = args.patch.clone();
        let code = args.code.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::currency::update(tx, cx, code, patch).await
        }) as BoxFuture<'_, TxResult<Currency>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsSaveExchangeRateArgs {
    pub input: ExchangeRateInput,
}

#[tauri::command]
pub async fn settings_save_exchange_rate(state: State<'_, AppState>, args: SettingsSaveExchangeRateArgs) -> CmdResult<ExchangeRate> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::currency::save_exchange_rate(tx, cx, input).await
        }) as BoxFuture<'_, TxResult<ExchangeRate>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn settings_is_base_currency_locked(state: State<'_, AppState>) -> CmdResult<bool> {
    with_read(&state, |conn| Box::pin(async move { service::currency::is_base_currency_locked(conn).await }) as BoxFuture<'_, TxResult<bool>>)
        .await
        .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsSetBaseCurrencyArgs {
    pub code: String,
}

#[tauri::command]
pub async fn settings_set_base_currency(state: State<'_, AppState>, args: SettingsSetBaseCurrencyArgs) -> CmdResult<()> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let code = args.code.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::currency::set_base_currency(tx, cx, &undo, code).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

// --- Revaluation -----------------------------------------------------------------------------------

/// Rule 5: rates arrive as JSON numbers but are parsed straight into `Decimal` (`JsonDecimal`),
/// never through a float.
fn rates_to_decimal_map(rates: BTreeMap<String, crate::entities::values::JsonDecimal>) -> BTreeMap<String, Decimal> {
    rates.into_iter().map(|(k, v)| (k, v.0)).collect()
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsGetRevaluationPreviewArgs {
    #[ts(type = "Record<string, number>")]
    pub rates: BTreeMap<String, crate::entities::values::JsonDecimal>,
}

#[tauri::command]
pub async fn settings_get_revaluation_preview(state: State<'_, AppState>, args: SettingsGetRevaluationPreviewArgs) -> CmdResult<Vec<FcBalanceRow>> {
    with_read(&state, move |conn| {
        Box::pin(async move {
            let rates = rates_to_decimal_map(args.rates);
            service::revaluation::open_fc_balances(conn, &rates).await.map_err(TxError::App)
        }) as BoxFuture<'_, TxResult<Vec<FcBalanceRow>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn settings_get_default_revaluation_rates(state: State<'_, AppState>) -> CmdResult<OrderedNumberMap> {
    with_read(&state, |conn| {
        Box::pin(async move { service::revaluation::default_revaluation_rates(conn).await.map_err(TxError::App) }) as BoxFuture<'_, TxResult<OrderedNumberMap>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsPostRevaluationArgs {
    pub date: String,
    #[ts(type = "Record<string, number>")]
    pub rates: BTreeMap<String, crate::entities::values::JsonDecimal>,
}

#[tauri::command]
pub async fn settings_post_revaluation(state: State<'_, AppState>, args: SettingsPostRevaluationArgs) -> CmdResult<RevaluationResult> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let date_str = args.date.clone();
        let rates = args.rates.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            let date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .map_err(|_| crate::core::error::AppError::validation("تاريخ غير صالح"))?;
            let rates = rates_to_decimal_map(rates);
            service::revaluation::post_revaluation(tx, cx, &undo, date, &rates).await
        }) as BoxFuture<'_, TxResult<RevaluationResult>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

// --- Network / LAN sharing -------------------------------------------------------------------------

#[tauri::command]
pub async fn settings_get_lan_sharing_status(state: State<'_, AppState>) -> CmdResult<LanSharingStatus> {
    // Access check only — the rest reads AppState/server.json/keyring, no DB transaction.
    let actor = state.session.read().unwrap().clone();
    with_read(&state, move |conn| {
        Box::pin(async move {
            crate::core::settings::require(conn, actor.as_ref(), Area::Settings, Access::Read).await?;
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)?;

    service::network::get_status(&state).await.map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn settings_enable_lan_sharing<R: tauri::Runtime>(app: tauri::AppHandle<R>, state: State<'_, AppState>) -> CmdResult<PairingInfo> {
    let app = crate::wry_handle(&app).map_err(ApiErrorPayload::from)?;
    let actor = state.session.read().unwrap().clone();
    let role = state.device.read().unwrap().role;
    with_read(&state, move |conn| {
        Box::pin(async move { service::network::require_main_write_parts(conn, actor.as_ref(), role).await.map_err(TxError::App) }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)?;

    let info = service::network::enable(app, &state).await.map_err(ApiErrorPayload::from)?;

    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            crate::shared::activity::log(
                tx,
                cx,
                &undo,
                crate::entities::platform::activity::ActivityKind::Settings,
                "تفعيل مشاركة قاعدة البيانات على الشبكة",
                None,
                Some(crate::utils::route::RouteRef::list("settings-network")),
            )
            .await
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .map_err(ApiErrorPayload::from)?;

    Ok(info)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsDisableLanSharingArgs {
    pub confirm_disconnect: bool,
}

#[tauri::command]
pub async fn settings_disable_lan_sharing<R: tauri::Runtime>(app: tauri::AppHandle<R>, state: State<'_, AppState>, args: SettingsDisableLanSharingArgs) -> CmdResult<LanSharingStatus> {
    let app = crate::wry_handle(&app).map_err(ApiErrorPayload::from)?;
    let actor = state.session.read().unwrap().clone();
    let role = state.device.read().unwrap().role;
    with_read(&state, move |conn| {
        Box::pin(async move { service::network::require_main_write_parts(conn, actor.as_ref(), role).await.map_err(TxError::App) }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)?;

    let status = service::network::disable(app, &state, args.confirm_disconnect).await.map_err(ApiErrorPayload::from)?;

    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            crate::shared::activity::log(
                tx,
                cx,
                &undo,
                crate::entities::platform::activity::ActivityKind::Settings,
                "إيقاف مشاركة قاعدة البيانات على الشبكة",
                None,
                Some(crate::utils::route::RouteRef::list("settings-network")),
            )
            .await
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .map_err(ApiErrorPayload::from)?;

    Ok(status)
}

#[tauri::command]
pub async fn settings_rotate_pairing_code(state: State<'_, AppState>) -> CmdResult<PairingInfo> {
    let actor = state.session.read().unwrap().clone();
    let role = state.device.read().unwrap().role;
    with_read(&state, move |conn| {
        Box::pin(async move { service::network::require_main_write_parts(conn, actor.as_ref(), role).await.map_err(TxError::App) }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)?;

    let info = service::network::rotate_pairing_code(&state).await.map_err(ApiErrorPayload::from)?;

    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            crate::shared::activity::log(
                tx,
                cx,
                &undo,
                crate::entities::platform::activity::ActivityKind::Settings,
                "تغيير رمز اقتران أجهزة الكاشير",
                None,
                Some(crate::utils::route::RouteRef::list("settings-network")),
            )
            .await
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .map_err(ApiErrorPayload::from)?;

    Ok(info)
}

#[tauri::command]
pub async fn settings_reconnect_backend<R: tauri::Runtime>(app: tauri::AppHandle<R>, state: State<'_, AppState>) -> CmdResult<()> {
    let app = crate::wry_handle(&app).map_err(ApiErrorPayload::from)?;
    service::network::reconnect(app, &state).await.map_err(ApiErrorPayload::from)
}

// --- ipc_signatures --------------------------------------------------------------------------------

pub fn ipc_signatures() -> Vec<crate::core::ipc::IpcSig> {
    vec![
        crate::ipc_sig!(settings_get_settings, (), StoreSettings),
        crate::ipc_sig!(settings_update_settings, SettingsUpdateSettingsArgs, StoreSettings),
        crate::ipc_sig!(settings_get_taxes, (), Vec<Tax>),
        crate::ipc_sig!(settings_save_tax, SettingsSaveTaxArgs, Tax),
        crate::ipc_sig!(settings_delete_tax, SettingsDeleteTaxArgs, ()),
        crate::ipc_sig!(settings_get_payment_methods, (), Vec<PaymentMethod>),
        crate::ipc_sig!(settings_save_payment_method, SettingsSavePaymentMethodArgs, PaymentMethod),
        crate::ipc_sig!(settings_reorder_payment_methods, SettingsReorderPaymentMethodsArgs, ()),
        crate::ipc_sig!(settings_delete_payment_method, SettingsDeletePaymentMethodArgs, ()),
        crate::ipc_sig!(settings_get_branches, (), Vec<Branch>),
        crate::ipc_sig!(settings_create_branch, SettingsCreateBranchArgs, Branch),
        crate::ipc_sig!(settings_update_branch, SettingsUpdateBranchArgs, Branch),
        crate::ipc_sig!(settings_deactivate_branch, SettingsDeactivateBranchArgs, Branch),
        crate::ipc_sig!(settings_reactivate_branch, SettingsReactivateBranchArgs, Branch),
        crate::ipc_sig!(settings_get_cost_centers, (), Vec<CostCenter>),
        crate::ipc_sig!(settings_create_cost_center, SettingsCreateCostCenterArgs, CostCenter),
        crate::ipc_sig!(settings_update_cost_center, SettingsUpdateCostCenterArgs, CostCenter),
        crate::ipc_sig!(settings_delete_cost_center, SettingsDeleteCostCenterArgs, ()),
        crate::ipc_sig!(settings_get_currencies, (), Vec<Currency>),
        crate::ipc_sig!(settings_get_exchange_rates, SettingsGetExchangeRatesArgs, Vec<ExchangeRate>),
        crate::ipc_sig!(settings_create_currency, SettingsCreateCurrencyArgs, Currency),
        crate::ipc_sig!(settings_update_currency, SettingsUpdateCurrencyArgs, Currency),
        crate::ipc_sig!(settings_save_exchange_rate, SettingsSaveExchangeRateArgs, ExchangeRate),
        crate::ipc_sig!(settings_is_base_currency_locked, (), bool),
        crate::ipc_sig!(settings_set_base_currency, SettingsSetBaseCurrencyArgs, ()),
        crate::ipc_sig!(settings_get_revaluation_preview, SettingsGetRevaluationPreviewArgs, Vec<FcBalanceRow>),
        crate::ipc_sig!(settings_get_default_revaluation_rates, (), OrderedNumberMap),
        crate::ipc_sig!(settings_post_revaluation, SettingsPostRevaluationArgs, RevaluationResult),
        crate::ipc_sig!(settings_get_lan_sharing_status, (), LanSharingStatus),
        crate::ipc_sig!(settings_enable_lan_sharing, (), PairingInfo),
        crate::ipc_sig!(settings_disable_lan_sharing, SettingsDisableLanSharingArgs, LanSharingStatus),
        crate::ipc_sig!(settings_rotate_pairing_code, (), PairingInfo),
        crate::ipc_sig!(settings_reconnect_backend, (), ()),
    ]
}
