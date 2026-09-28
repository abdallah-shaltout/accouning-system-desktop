//! `templates` IPC commands (03-domains/15-templates.md §1) — thin layer: parse args, authorize,
//! open a transaction, call the service, map the error. Reads are session-only (T-3: every role
//! prints); writes need `Settings / Write` (the designer lives under `/settings/templates`).

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_tx, TxOpts};

use super::dto::{
    PdfTemplate, TemplatesCreateTemplateArgs, TemplatesDeleteTemplateArgs, TemplatesDuplicateTemplateArgs, TemplatesGetDefaultTemplateArgs,
    TemplatesGetTemplateArgs, TemplatesImportTemplateArgs, TemplatesListTemplatesArgs, TemplatesResetTemplateToDefaultsArgs,
    TemplatesSaveTemplateArgs, TemplatesSetAsDefaultArgs,
};
use super::service;

/// T-2/T-3: every command runs `ensure_seeded` first and needs only a session (no area) to read —
/// so reads use `with_tx` too (seeding is a write), gated by nothing beyond "logged in".
#[tauri::command]
pub async fn templates_list_templates(state: State<'_, AppState>, args: TemplatesListTemplatesArgs) -> Result<Vec<PdfTemplate>, ApiErrorPayload> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let kind = args.kind;
        Box::pin(async move { service::list_templates(tx, cx, kind).await })
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn templates_get_template(state: State<'_, AppState>, args: TemplatesGetTemplateArgs) -> Result<Option<PdfTemplate>, ApiErrorPayload> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let id = args.id.clone();
        Box::pin(async move { service::get_template(tx, cx, &id).await })
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn templates_get_default_template(
    state: State<'_, AppState>,
    args: TemplatesGetDefaultTemplateArgs,
) -> Result<Option<PdfTemplate>, ApiErrorPayload> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let kind = args.kind;
        Box::pin(async move { service::get_default_template(tx, cx, kind).await })
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn templates_save_template(state: State<'_, AppState>, args: TemplatesSaveTemplateArgs) -> Result<PdfTemplate, ApiErrorPayload> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let template = args.template.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::save_template(tx, cx, template).await
        })
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn templates_set_as_default(state: State<'_, AppState>, args: TemplatesSetAsDefaultArgs) -> Result<(), ApiErrorPayload> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let id = args.id.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::set_as_default(tx, cx, &id).await
        })
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn templates_duplicate_template(
    state: State<'_, AppState>,
    args: TemplatesDuplicateTemplateArgs,
) -> Result<Option<PdfTemplate>, ApiErrorPayload> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let id = args.id.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::duplicate_template(tx, cx, &id).await
        })
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn templates_delete_template(state: State<'_, AppState>, args: TemplatesDeleteTemplateArgs) -> Result<(), ApiErrorPayload> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let id = args.id.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::delete_template(tx, cx, &id).await
        })
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn templates_reset_template_to_defaults(
    state: State<'_, AppState>,
    args: TemplatesResetTemplateToDefaultsArgs,
) -> Result<Option<PdfTemplate>, ApiErrorPayload> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let id = args.id.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::reset_template_to_defaults(tx, cx, &id).await
        })
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn templates_import_template(state: State<'_, AppState>, args: TemplatesImportTemplateArgs) -> Result<PdfTemplate, ApiErrorPayload> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let json = args.json.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::import_template(tx, cx, json).await
        })
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn templates_create_template(state: State<'_, AppState>, args: TemplatesCreateTemplateArgs) -> Result<PdfTemplate, ApiErrorPayload> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let kind = args.kind;
        let base_template_id = args.base_template_id;
        let name = args.name.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            service::create_template(tx, cx, kind, base_template_id, name).await
        })
    })
    .await
    .map_err(Into::into)
}
