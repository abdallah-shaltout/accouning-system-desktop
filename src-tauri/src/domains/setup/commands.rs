//! `domains::setup::commands` — thin IPC layer (02-setup.md §1). 22 commands: 19 ports + 3 device.
//! "Settings:Write" commands run with the bootstrap admin during the wizard (D-1) and with the
//! logged-in user afterwards.

use serde::Deserialize;
use tauri::State;
use ts_rs::TS;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_read, with_tx, BoxFuture, TxOpts, TxResult};
use crate::utils::id::Id;

use super::dto::*;
use super::service;

type CmdResult<T> = Result<T, ApiErrorPayload>;

// --- Device (§3.1) ---------------------------------------------------------------------------------

#[tauri::command]
pub async fn setup_get_device_setup_state<R: tauri::Runtime>(app: tauri::AppHandle<R>, state: State<'_, AppState>) -> CmdResult<DeviceSetupState> {
    let app = crate::wry_handle(&app).map_err(ApiErrorPayload::from)?;
    service::device::get_device_setup_state(app, &state).await.map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn setup_provision_main<R: tauri::Runtime>(app: tauri::AppHandle<R>, state: State<'_, AppState>) -> CmdResult<DeviceSetupState> {
    let app = crate::wry_handle(&app).map_err(ApiErrorPayload::from)?;
    service::device::provision_main(app, &state).await.map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupPairTerminalArgs {
    pub host: String,
    #[ts(type = "number")]
    pub port: u16,
    pub code: String,
}

#[tauri::command]
pub async fn setup_pair_terminal<R: tauri::Runtime>(app: tauri::AppHandle<R>, state: State<'_, AppState>, args: SetupPairTerminalArgs) -> CmdResult<DeviceSetupState> {
    let app = crate::wry_handle(&app).map_err(ApiErrorPayload::from)?;
    let input = PairTerminalInput { host: args.host, port: args.port, code: args.code };
    service::device::pair_terminal(app, &state, input).await.map_err(ApiErrorPayload::from)
}

// --- Onboarding progress (§3.3/§3.4) ----------------------------------------------------------------

#[tauri::command]
pub async fn setup_get_onboarding_progress(state: State<'_, AppState>) -> CmdResult<OnboardingProgress> {
    // 1. `with_tx(require_user: false)`: seeds the shell when `users`/`settings` are both empty.
    with_tx(&state, TxOpts { require_user: false }, |tx, cx| {
        Box::pin(async move { service::shell::seed_company_shell(tx, cx).await }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)?;

    // 2. Read `settings.onboarding` -> DTO.
    let progress = with_read(&state, |conn| Box::pin(async move { service::progress::get(conn).await }) as BoxFuture<'_, TxResult<OnboardingProgress>>)
        .await
        .map_err(ApiErrorPayload::from)?;

    // 3. After commit: `ensure_bootstrap_session`. No session afterwards -> UNAUTHORIZED (a finished
    //    wizard is not public data).
    let connection = {
        let guard = state.db.read().unwrap();
        guard.as_ref().map(|db| db.connection.clone())
    };
    if let Some(connection) = connection {
        service::session::ensure_bootstrap_session(&state, &connection)
            .await
            .map_err(|e| ApiErrorPayload::from(e.into_app_error()))?;
    }
    if state.session.read().unwrap().is_none() {
        return Err(ApiErrorPayload::from(crate::core::error::AppError::unauthorized("سجّل الدخول أولاً")));
    }

    Ok(progress)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupSaveOnboardingProgressArgs {
    pub patch: OnboardingProgressPatch,
}

#[tauri::command]
pub async fn setup_save_onboarding_progress(state: State<'_, AppState>, args: SetupSaveOnboardingProgressArgs) -> CmdResult<()> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let patch = args.patch.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::progress::save(tx, cx, patch).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupMarkStepDoneArgs {
    pub key: String,
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_index: Option<i32>,
}

#[tauri::command]
pub async fn setup_mark_step_done(state: State<'_, AppState>, args: SetupMarkStepDoneArgs) -> CmdResult<()> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let key = args.key.clone();
        let step_index = args.step_index;
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::progress::mark_step_done(tx, cx, key, step_index).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupMarkStepSkippedArgs {
    pub key: String,
}

#[tauri::command]
pub async fn setup_mark_step_skipped(state: State<'_, AppState>, args: SetupMarkStepSkippedArgs) -> CmdResult<()> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let key = args.key.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::progress::mark_step_skipped(tx, cx, key).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

// --- Structure steps (§3.5) -------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupApplyBusinessTypeDefaultsArgs {
    pub business_type: String,
}

#[tauri::command]
pub async fn setup_apply_business_type_defaults(state: State<'_, AppState>, args: SetupApplyBusinessTypeDefaultsArgs) -> CmdResult<()> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let business_type = args.business_type.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::business_type::apply(tx, cx, &business_type).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn setup_is_base_currency_locked(state: State<'_, AppState>) -> CmdResult<bool> {
    with_read(&state, |conn| {
        Box::pin(async move { crate::domains::settings::service::currency::is_base_currency_locked(conn).await }) as BoxFuture<'_, TxResult<bool>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupApplyCountryTaxArgs {
    pub input: CountryTaxInput,
}

#[tauri::command]
pub async fn setup_apply_country_tax(state: State<'_, AppState>, args: SetupApplyCountryTaxArgs) -> CmdResult<()> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::country_tax::apply(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupApplyFiscalYearArgs {
    #[ts(type = "number")]
    pub start_month: i32,
    #[ts(type = "number")]
    pub start_day: i32,
    pub go_live_date: String,
}

#[tauri::command]
pub async fn setup_apply_fiscal_year(
    state: State<'_, AppState>,
    args: SetupApplyFiscalYearArgs,
) -> CmdResult<crate::domains::accounting::dto::FiscalYear> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let start_month = args.start_month;
        let start_day = args.start_day;
        let go_live_str = args.go_live_date.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            let go_live = chrono::NaiveDate::parse_from_str(&go_live_str, "%Y-%m-%d")
                .map_err(|_| crate::core::error::AppError::validation("تاريخ غير صالح"))?;
            service::fiscal_year::apply(tx, cx, start_month, start_day, go_live).await
        }) as BoxFuture<'_, TxResult<crate::domains::accounting::dto::FiscalYear>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupApplyBranchesArgs {
    pub branches: Vec<WizardBranchInput>,
}

#[tauri::command]
pub async fn setup_apply_branches(
    state: State<'_, AppState>,
    args: SetupApplyBranchesArgs,
) -> CmdResult<Vec<crate::domains::settings::dto::Branch>> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let branches = args.branches.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::branches::apply(tx, cx, &undo, branches).await
        }) as BoxFuture<'_, TxResult<Vec<crate::domains::settings::dto::Branch>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupApplyCoaTemplateArgs {
    #[ts(type = "import('@/mocks/fixtures/accounts').AccountTemplate")]
    pub template: String,
    #[ts(optional, type = "import('@/modules/core/helpers/countryProfiles').CountryCode")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_type: Option<String>,
}

#[tauri::command]
pub async fn setup_apply_coa_template(
    state: State<'_, AppState>,
    args: SetupApplyCoaTemplateArgs,
) -> CmdResult<Vec<crate::domains::accounting::dto::Account>> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let template = service::coa::template_kind_from_str(&args.template);
        let country = args.country.clone();
        let business_type = args.business_type.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::coa::apply(tx, cx, template, country.as_deref(), business_type.as_deref()).await
        }) as BoxFuture<'_, TxResult<Vec<crate::domains::accounting::dto::Account>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupApplyPaymentMethodsArgs {
    pub methods: Vec<WizardPaymentMethodInput>,
}

#[tauri::command]
pub async fn setup_apply_payment_methods(state: State<'_, AppState>, args: SetupApplyPaymentMethodsArgs) -> CmdResult<()> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let methods = args.methods.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::payment_methods::apply(tx, cx, methods).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

// --- Opening (§3.6) ----------------------------------------------------------------------------------

#[tauri::command]
pub async fn setup_get_opening_balance_equity_net(state: State<'_, AppState>) -> CmdResult<Money> {
    with_read(&state, |conn| {
        Box::pin(async move {
            let net = service::opening::get_opening_balance_equity_net(conn).await?;
            Ok(Money(net))
        }) as BoxFuture<'_, TxResult<Money>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn setup_is_first_use_posted(state: State<'_, AppState>) -> CmdResult<bool> {
    with_read(&state, |conn| Box::pin(async move { service::opening::is_first_use_posted(conn).await }) as BoxFuture<'_, TxResult<bool>>)
        .await
        .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupPostOpeningBalancesArgs {
    pub input: OpeningEntryInput,
    pub close_target: CloseTarget,
}

#[tauri::command]
pub async fn setup_post_opening_balances(state: State<'_, AppState>, args: SetupPostOpeningBalancesArgs) -> CmdResult<PostOpeningBalancesResult> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let close_target = args.close_target;
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            service::opening::post_opening_balances(tx, cx, &undo, input, close_target).await
        }) as BoxFuture<'_, TxResult<PostOpeningBalancesResult>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupPostOpeningStockArgs {
    #[ts(type = "string")]
    pub branch_id: Id,
    pub date: String,
    pub lines: Vec<OpeningStockLine>,
}

#[tauri::command]
pub async fn setup_post_opening_stock(state: State<'_, AppState>, args: SetupPostOpeningStockArgs) -> CmdResult<()> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let branch_id = args.branch_id;
        let date_str = args.date.clone();
        let lines = args.lines.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            let date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|_| crate::core::error::AppError::validation("تاريخ غير صالح"))?;
            service::opening::post_opening_stock(tx, cx, &undo, branch_id, date, lines).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupRecloseOpeningBalanceEquityArgs {
    pub date: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<CloseTarget>,
}

#[tauri::command]
pub async fn setup_reclose_opening_balance_equity(state: State<'_, AppState>, args: SetupRecloseOpeningBalanceEquityArgs) -> CmdResult<()> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let date_str = args.date.clone();
        let target = args.target.unwrap_or(CloseTarget::Capital);
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Accounting, Access::Write).await?;
            let date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|_| crate::core::error::AppError::validation("تاريخ غير صالح"))?;
            service::opening::reclose_opening_balance_equity(tx, cx, &undo, date, target).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

// --- Party opening (§3.7) -----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupPostPartyOpeningArgs {
    pub input: PartyOpeningInput,
}

#[tauri::command]
pub async fn setup_post_party_opening(state: State<'_, AppState>, args: SetupPostPartyOpeningArgs) -> CmdResult<Option<String>> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Parties, Access::Write).await?;
            let id = service::party_opening::post_party_opening(tx, cx, &undo, input).await?;
            Ok(id.map(|i| i.to_string()))
        }) as BoxFuture<'_, TxResult<Option<String>>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupReversePartyOpeningArgs {
    #[ts(type = "string")]
    pub entry_id: Id,
}

#[tauri::command]
pub async fn setup_reverse_party_opening(state: State<'_, AppState>, args: SetupReversePartyOpeningArgs) -> CmdResult<()> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let entry_id = args.entry_id;
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Parties, Access::Write).await?;
            // The command passes `allow_closed_period = true, reason = None` (mock parity, §3.7).
            service::party_opening::reverse_party_opening(tx, cx, &undo, entry_id, true, None).await?;
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

// --- Finish --------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn setup_finish_onboarding(state: State<'_, AppState>) -> CmdResult<()> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::progress::finish(tx, cx).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)?;

    // After commit: clear the bootstrap session (the wizard then routes to login).
    service::progress::clear_bootstrap_session_after_finish(&state);
    Ok(())
}

// --- ipc_signatures --------------------------------------------------------------------------------

pub fn ipc_signatures() -> Vec<crate::core::ipc::IpcSig> {
    vec![
        crate::ipc_sig!(setup_get_device_setup_state, (), DeviceSetupState),
        crate::ipc_sig!(setup_provision_main, (), DeviceSetupState),
        crate::ipc_sig!(setup_pair_terminal, SetupPairTerminalArgs, DeviceSetupState),
        crate::ipc_sig!(setup_get_onboarding_progress, (), OnboardingProgress),
        crate::ipc_sig!(setup_save_onboarding_progress, SetupSaveOnboardingProgressArgs, ()),
        crate::ipc_sig!(setup_mark_step_done, SetupMarkStepDoneArgs, ()),
        crate::ipc_sig!(setup_mark_step_skipped, SetupMarkStepSkippedArgs, ()),
        crate::ipc_sig!(setup_apply_business_type_defaults, SetupApplyBusinessTypeDefaultsArgs, ()),
        crate::ipc_sig!(setup_is_base_currency_locked, (), bool),
        crate::ipc_sig!(setup_apply_country_tax, SetupApplyCountryTaxArgs, ()),
        crate::ipc_sig!(setup_apply_fiscal_year, SetupApplyFiscalYearArgs, crate::domains::accounting::dto::FiscalYear),
        crate::ipc_sig!(setup_apply_branches, SetupApplyBranchesArgs, Vec<crate::domains::settings::dto::Branch>),
        crate::ipc_sig!(setup_apply_coa_template, SetupApplyCoaTemplateArgs, Vec<crate::domains::accounting::dto::Account>),
        crate::ipc_sig!(setup_apply_payment_methods, SetupApplyPaymentMethodsArgs, ()),
        crate::ipc_sig!(setup_get_opening_balance_equity_net, (), Money),
        crate::ipc_sig!(setup_is_first_use_posted, (), bool),
        crate::ipc_sig!(setup_post_opening_balances, SetupPostOpeningBalancesArgs, PostOpeningBalancesResult),
        crate::ipc_sig!(setup_post_opening_stock, SetupPostOpeningStockArgs, ()),
        crate::ipc_sig!(setup_reclose_opening_balance_equity, SetupRecloseOpeningBalanceEquityArgs, ()),
        crate::ipc_sig!(setup_post_party_opening, SetupPostPartyOpeningArgs, Option<String>),
        crate::ipc_sig!(setup_reverse_party_opening, SetupReversePartyOpeningArgs, ()),
        crate::ipc_sig!(setup_finish_onboarding, (), ()),
    ]
}
