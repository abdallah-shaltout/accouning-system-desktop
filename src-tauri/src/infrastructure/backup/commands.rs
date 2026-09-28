//! `infrastructure::backup::commands` — the 8 `settings_*` IPC commands (17-backup.md §1). Thin
//! layer: args struct, `require`, `with_read`/`with_tx`, map error — same shape as
//! `domains::settings::commands`. Names keep the `settings_` prefix (the service stays
//! `settings/services/backupService.ts`; the domain flips with `usesRust('settings')`) even though
//! the Rust module lives under `infrastructure/` (architecture-rule exemption, like `import`).

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::ipc::IpcSig;
use crate::core::state::AppState;
use crate::core::tx::{with_read, with_read_ctx, with_tx, BoxFuture, TxError, TxOpts, TxResult};
use crate::ipc_sig;

use super::auto::{backup_settings, build_and_write_or_read_archive, record_backup_saved, run_auto_backup_if_due, save_backup_settings};
use super::counts::table_counts;
use super::dto::{
    BackupArchive, BackupSettings, RestorePreview, SettingsBuildBackupArchiveArgs, SettingsPreviewRestoreArgs, SettingsRecordBackupSavedArgs,
    SettingsRestoreFromArchiveArgs, SettingsRunAutoBackupIfDueArgs, SettingsSaveBackupSettingsArgs,
};
use super::restore;

type CmdResult<T> = Result<T, ApiErrorPayload>;

#[tauri::command]
pub async fn settings_backup_settings(state: State<'_, AppState>) -> CmdResult<BackupSettings> {
    let device = state.device.read().unwrap().clone();
    with_read_ctx(&state, move |conn, cx| {
        Box::pin(async move {
            cx.require(conn, Area::Settings, Access::Read).await?;
            backup_settings(conn, &device).await.map_err(TxError::App)
        }) as BoxFuture<'_, TxResult<BackupSettings>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn settings_save_backup_settings(state: State<'_, AppState>, args: SettingsSaveBackupSettingsArgs) -> CmdResult<BackupSettings> {
    let device = state.device.read().unwrap().clone();
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let patch = args.patch.clone();
        let device = device.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            let (next, _delta) = save_backup_settings(tx, cx, &undo, &device, patch).await?;
            Ok(next)
        }) as BoxFuture<'_, TxResult<BackupSettings>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn settings_preview_backup_counts(state: State<'_, AppState>) -> CmdResult<super::dto::OrderedCounts> {
    with_read_ctx(&state, |conn, cx| {
        Box::pin(async move {
            cx.require(conn, Area::Settings, Access::Read).await?;
            table_counts(conn).await
        }) as BoxFuture<'_, TxResult<super::dto::OrderedCounts>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn settings_build_backup_archive(state: State<'_, AppState>, args: SettingsBuildBackupArchiveArgs) -> CmdResult<BackupArchive> {
    let actor = state.session.read().unwrap().clone();
    let kind = args.kind;
    let password = args.password.clone();
    with_read(&state, move |conn| {
        Box::pin(async move {
            crate::core::settings::require(conn, actor.as_ref(), Area::Settings, Access::Write).await.map_err(TxError::App)?;
            build_and_write_or_read_archive(conn, kind, password.as_deref()).await.map_err(TxError::App)
        }) as BoxFuture<'_, TxResult<BackupArchive>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn settings_record_backup_saved(state: State<'_, AppState>, args: SettingsRecordBackupSavedArgs) -> CmdResult<()> {
    let device = state.device.read().unwrap().clone();
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let manifest = args.manifest.clone();
        let kind = args.kind;
        let device = device.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Settings, Access::Write).await?;
            record_backup_saved(tx, cx, &undo, &device, &manifest.created_at, kind).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn settings_run_auto_backup_if_due(state: State<'_, AppState>, args: SettingsRunAutoBackupIfDueArgs) -> CmdResult<super::dto::AutoBackupOutcome> {
    run_auto_backup_if_due(&state, args.trigger).await.map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn settings_preview_restore(state: State<'_, AppState>, args: SettingsPreviewRestoreArgs) -> CmdResult<RestorePreview> {
    // Bytes only (§1: no transaction) — still gated behind a session + Settings:Read check so an
    // unauthenticated caller can't probe archive contents.
    let actor = state.session.read().unwrap().clone();
    with_read(&state, move |conn| {
        Box::pin(async move {
            crate::core::settings::require(conn, actor.as_ref(), Area::Settings, Access::Read).await?;
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(ApiErrorPayload::from)?;

    let bytes = super::archive::base64_to_bytes(&args.archive_base64).map_err(ApiErrorPayload::from)?;
    restore::preview_restore(&bytes).map_err(ApiErrorPayload::from)
}

#[tauri::command]
pub async fn settings_restore_from_archive(state: State<'_, AppState>, args: SettingsRestoreFromArchiveArgs) -> CmdResult<()> {
    restore::restore_from_archive(&state, &args.archive_base64, args.password.as_deref()).await.map_err(ApiErrorPayload::from)
}

pub fn ipc_signatures() -> Vec<IpcSig> {
    vec![
        ipc_sig!(settings_backup_settings, (), BackupSettings),
        ipc_sig!(settings_save_backup_settings, SettingsSaveBackupSettingsArgs, BackupSettings),
        ipc_sig!(settings_preview_backup_counts, (), super::dto::OrderedCounts),
        ipc_sig!(settings_build_backup_archive, SettingsBuildBackupArchiveArgs, BackupArchive),
        ipc_sig!(settings_record_backup_saved, SettingsRecordBackupSavedArgs, ()),
        ipc_sig!(settings_run_auto_backup_if_due, SettingsRunAutoBackupIfDueArgs, super::dto::AutoBackupOutcome),
        ipc_sig!(settings_preview_restore, SettingsPreviewRestoreArgs, RestorePreview),
        ipc_sig!(settings_restore_from_archive, SettingsRestoreFromArchiveArgs, ()),
    ]
}
