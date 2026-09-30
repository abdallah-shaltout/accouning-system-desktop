//! The 2 importer IPC commands (`03-domains/00-import.md` §3.4): `setup_inspect_legacy_snapshot`
//! (read-only) and `setup_import_snapshot` (the whole importer, one write transaction).

use tauri::State;

use crate::core::device::DeviceRole;
use crate::core::dto::ApiErrorPayload;
use crate::core::error::AppError;
use crate::core::ipc::IpcSig;
use crate::core::state::AppState;
use crate::core::tx::{with_read_on, with_tx, TxOpts};
use crate::ipc_sig;

use super::dto::{ImportMode, ImportSnapshotResult, LegacySnapshotSummary, SetupImportSnapshotArgs, SetupInspectLegacySnapshotArgs};
use super::run::{self, ImportOpts};

fn current_connection(state: &AppState) -> Result<sea_orm::DatabaseConnection, AppError> {
    state
        .db
        .read()
        .unwrap()
        .as_ref()
        .map(|db| db.connection.clone())
        .ok_or_else(|| AppError::internal("لا يوجد اتصال بقاعدة البيانات", None))
}

#[tauri::command]
pub async fn setup_inspect_legacy_snapshot(
    state: State<'_, AppState>,
    args: SetupInspectLegacySnapshotArgs,
) -> Result<LegacySnapshotSummary, ApiErrorPayload> {
    let connection = current_connection(&state).map_err(ApiErrorPayload::from)?;
    let snapshot_json = args.snapshot_json.clone();
    let templates_json = args.templates_json.clone();
    with_read_on(&connection, move |tx| {
        Box::pin(async move { run::inspect(tx, &snapshot_json, templates_json.as_deref()).await })
    })
    .await
    .map_err(ApiErrorPayload::from)
}

/// D-3: only the Main PC (or the debug DB URL override) may import — a terminal must never push its
/// browser data into the branch DB.
fn require_main_or_debug_db(state: &AppState) -> Result<(), AppError> {
    let role = state.device.read().unwrap().role;
    let is_debug_db = crate::core::device::debug_db_url_override().is_some();
    if role == DeviceRole::Main || is_debug_db {
        Ok(())
    } else {
        Err(AppError::forbidden("استيراد البيانات متاح على الجهاز الرئيسي فقط"))
    }
}

/// Plan 21 Part 04 E-2: demo data is a development tool. A release build refuses `mode: Demo`
/// outright, so demo books can never land in a real company's (empty) database, where a release build
/// could never remove them again (no wipe outside debug builds, §3.2 step 1). The welcome page hides
/// the demo card in a release desktop build too (`devToolsService.canLoadDemoData`); this is the
/// authority. `release` is a parameter so the rule is unit-testable from a debug test build.
fn refuse_demo_in_release(mode: ImportMode, release: bool) -> Result<(), AppError> {
    if release && matches!(mode, ImportMode::Demo) {
        Err(AppError::forbidden("البيانات التجريبية غير متاحة في النسخة النهائية"))
    } else {
        Ok(())
    }
}

#[tauri::command]
pub async fn setup_import_snapshot(state: State<'_, AppState>, args: SetupImportSnapshotArgs) -> Result<ImportSnapshotResult, ApiErrorPayload> {
    require_main_or_debug_db(&state).map_err(ApiErrorPayload::from)?;
    refuse_demo_in_release(args.mode, !cfg!(debug_assertions)).map_err(ApiErrorPayload::from)?;

    // The JSON string is parsed once outside the closure (the closure may re-run on a deadlock
    // retry, P2-07) — `run::import_snapshot` re-parses it internally per call, but the args
    // themselves (strings) are cheap to clone into the retryable closure; the actual `serde_json`
    // parse happening once per attempt (not once per call) is an acceptable cost here since a
    // deadlock retry on an import (a rare, single-writer, pre-go-live operation) is itself rare.
    let snapshot_json = args.snapshot_json.clone();
    let templates_json = args.templates_json.clone();
    let template_branch_id = args.template_branch_id.clone();
    let mode = args.mode;
    let replace_existing = args.replace_existing;
    let adopt_terminal = if matches!(mode, ImportMode::Legacy) { Some(state.terminal.terminal_id) } else { None };

    let report = with_tx(&state, TxOpts { require_user: false }, move |tx, _cx| {
        let snapshot_json = snapshot_json.clone();
        let templates_json = templates_json.clone();
        let template_branch_id = template_branch_id.clone();
        Box::pin(async move {
            run::import_snapshot(
                tx,
                &snapshot_json,
                templates_json.as_deref(),
                template_branch_id.as_deref(),
                ImportOpts { mode, replace_existing, adopt_terminal },
            )
            .await
        })
    })
    .await
    .map_err(ApiErrorPayload::from)?;

    // Step 15's device write happens here, after commit — never overwriting a value this device's
    // own `device-settings.json` already has (D-12).
    if matches!(mode, ImportMode::Legacy) {
        let mut device = crate::core::device::load(&state.app_data_dir).map_err(|e| {
            ApiErrorPayload::from(AppError::internal("تعذر قراءة إعدادات الجهاز", Some(e.to_string())))
        })?;
        let df = &report.device_fields;
        if device.printer.thermal.is_none() && df.thermal_printer_name.is_some() {
            device.printer.thermal = Some(crate::core::device::ThermalPrinterConfig {
                printer_name: df.thermal_printer_name.clone(),
                ..Default::default()
            });
        }
        if device.printer.a4_printer_name.is_none() && df.a4_printer_name.is_some() {
            device.printer.a4_printer_name = df.a4_printer_name.clone();
        }
        if device.printer.label_printer_name.is_none() && df.label_printer_name.is_some() {
            device.printer.label_printer_name = df.label_printer_name.clone();
        }
        if device.backup_folder.is_none() && df.backup_folder.is_some() {
            device.backup_folder = df.backup_folder.clone();
        }
        crate::core::device::save(&state.app_data_dir, &device).map_err(|e| {
            ApiErrorPayload::from(AppError::internal("تعذر حفظ إعدادات الجهاز", Some(e.to_string())))
        })?;
        *state.device.write().unwrap() = device;
    }

    log::info!(
        target: "import",
        "import_snapshot ({:?}): {} table(s) imported, {} rounded value(s), default branch {}",
        mode,
        report.counts.0.len(),
        report.rounded_values,
        report.default_branch_id
    );

    Ok(ImportSnapshotResult {
        counts: report.counts,
        rounded_values: report.rounded_values,
        default_branch_id: report.default_branch_id.to_string(),
    })
}

pub fn ipc_signatures() -> Vec<IpcSig> {
    vec![
        ipc_sig!(setup_inspect_legacy_snapshot, SetupInspectLegacySnapshotArgs, LegacySnapshotSummary),
        ipc_sig!(setup_import_snapshot, SetupImportSnapshotArgs, ImportSnapshotResult),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_refuses_demo_import_only() {
        let refused = refuse_demo_in_release(ImportMode::Demo, true).unwrap_err();
        assert!(matches!(ApiErrorPayload::from(refused).code, crate::core::dto::ApiErrorCode::Forbidden));
        assert!(refuse_demo_in_release(ImportMode::Legacy, true).is_ok());
        assert!(refuse_demo_in_release(ImportMode::Demo, false).is_ok());
        assert!(refuse_demo_in_release(ImportMode::Legacy, false).is_ok());
    }
}
