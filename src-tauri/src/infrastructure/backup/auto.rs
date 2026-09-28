//! `auto.rs` (17-backup.md §3.4, §3.7) — backup settings read/write (the `store.rs`/device-file
//! merge, same shape as `backupService.ts:36-44`) and `run_auto_backup_if_due` (the daily/close auto
//! backup, Main-PC-only per D-S1/D-S2).

use sea_orm::ConnectionTrait;

use crate::core::device::DeviceSettings;
use crate::core::error::{AppError, AppResult};
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::core::state::AppState;
use crate::domains::settings::dto::StoreSettingsPatch;
use crate::entities::platform::activity::ActivityKind;
use crate::entities::values::BackupPolicy;
use crate::shared::activity::undo::UndoRegistry;
use crate::utils::dates::DocDate;

use super::archive::{backup_file_name, build_archive, read_manifest};
use super::counts;
use super::dataset;
use super::dto::{AutoBackupOutcome, AutoBackupSkipReason, AutoBackupTrigger, BackupKind, BackupSettings, BackupSettingsPatch};

fn backup_policy_to_dto(p: &BackupPolicy, folder: Option<String>) -> BackupSettings {
    BackupSettings {
        auto_enabled: p.auto_enabled,
        auto_time: p.auto_time.clone(),
        folder,
        retention: p.retention,
        last_backup_at: p.last_backup_at.map(crate::utils::dates::format_iso_ms),
        last_backup_kind: p.last_backup_kind.as_deref().and_then(parse_kind),
        last_auto_run_date: p.last_auto_run_date.map(|d| d.format("%Y-%m-%d").to_string()),
        last_backup_failed_at: p.last_backup_failed_at.map(crate::utils::dates::format_iso_ms),
        last_backup_error: p.last_backup_error.clone(),
    }
}

fn parse_kind(s: &str) -> Option<BackupKind> {
    match s {
        "manual" => Some(BackupKind::Manual),
        "auto" => Some(BackupKind::Auto),
        "pre-restore" => Some(BackupKind::PreRestore),
        _ => None,
    }
}

fn kind_str(k: BackupKind) -> &'static str {
    match k {
        BackupKind::Manual => "manual",
        BackupKind::Auto => "auto",
        BackupKind::PreRestore => "pre-restore",
    }
}

/// `backup_settings(conn, device)` (§3.4): `{ ...DEFAULT_BACKUP_SETTINGS, ...settings.backup, folder:
/// device.backup_folder }`.
pub async fn backup_settings<C: ConnectionTrait>(conn: &C, device: &DeviceSettings) -> AppResult<BackupSettings> {
    let row = crate::core::settings::load(conn).await?;
    Ok(match row.backup {
        Some(p) => backup_policy_to_dto(&p, device.backup_folder.clone()),
        None => BackupSettings { folder: device.backup_folder.clone(), ..BackupSettings::default() },
    })
}

/// `save_backup_settings(patch)` (§3.4): `next = { ...backup_settings(), ...patch }`, written back
/// through `settings::service::store::update_settings` (same transaction, same device-file write/
/// restore, same "تحديث إعدادات المتجر" activity row — D-1 merges the row-only fields; `folder`
/// routes through `StoreSettingsPatch.backup`'s `folder` key exactly like a frontend caller would).
pub async fn save_backup_settings<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    device: &DeviceSettings,
    patch: BackupSettingsPatch,
) -> TxResult<(BackupSettings, crate::domains::settings::service::store::DeviceDelta)> {
    let current = backup_settings(conn, device).await?;
    let next = apply_patch(&current, patch);

    let mut value = serde_json::json!({
        "autoEnabled": next.auto_enabled,
        "autoTime": next.auto_time,
        "retention": next.retention,
    });
    let obj = value.as_object_mut().expect("object literal");
    if let Some(v) = &next.last_backup_at {
        obj.insert("lastBackupAt".to_string(), serde_json::Value::String(v.clone()));
    }
    if let Some(v) = next.last_backup_kind {
        obj.insert("lastBackupKind".to_string(), serde_json::Value::String(kind_str(v).to_string()));
    }
    if let Some(v) = &next.last_auto_run_date {
        obj.insert("lastAutoRunDate".to_string(), serde_json::Value::String(v.clone()));
    }
    if let Some(v) = &next.last_backup_failed_at {
        obj.insert("lastBackupFailedAt".to_string(), serde_json::Value::String(v.clone()));
    }
    if let Some(v) = &next.last_backup_error {
        obj.insert("lastBackupError".to_string(), serde_json::Value::String(v.clone()));
    }
    if let Some(folder) = &next.folder {
        obj.insert("folder".to_string(), serde_json::Value::String(folder.clone()));
    }

    let patch = StoreSettingsPatch { backup: Some(value), ..Default::default() };
    let (_row, delta) = crate::domains::settings::service::store::update_settings(conn, cx, registry, patch).await?;
    Ok((next, delta))
}

fn apply_patch(current: &BackupSettings, patch: BackupSettingsPatch) -> BackupSettings {
    BackupSettings {
        auto_enabled: patch.auto_enabled.unwrap_or(current.auto_enabled),
        auto_time: patch.auto_time.unwrap_or_else(|| current.auto_time.clone()),
        folder: patch.folder.or_else(|| current.folder.clone()),
        retention: patch.retention.unwrap_or(current.retention),
        last_backup_at: patch.last_backup_at.or_else(|| current.last_backup_at.clone()),
        last_backup_kind: patch.last_backup_kind.or(current.last_backup_kind),
        last_auto_run_date: patch.last_auto_run_date.or_else(|| current.last_auto_run_date.clone()),
        last_backup_failed_at: patch.last_backup_failed_at.or_else(|| current.last_backup_failed_at.clone()),
        last_backup_error: patch.last_backup_error.or_else(|| current.last_backup_error.clone()),
    }
}

/// Internal-only patch that *clears* a key instead of leaving it unchanged — `record_backup_saved`
/// needs to drop `lastBackupFailedAt`/`lastBackupError` even though the wire `BackupSettingsPatch`
/// can only mean "unchanged" for an absent key (D-1: `undefined` never crosses JSON). Never exposed
/// through the `settings_save_backup_settings` command.
struct ClearingPatch {
    last_backup_at: Option<String>,
    last_backup_kind: Option<BackupKind>,
    clear_failure: bool,
    last_auto_run_date: Option<String>,
    last_backup_failed_at: Option<String>,
    last_backup_error: Option<String>,
}

async fn save_backup_settings_clearing<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    device: &DeviceSettings,
    patch: ClearingPatch,
) -> TxResult<BackupSettings> {
    let current = backup_settings(conn, device).await?;
    let next = BackupSettings {
        auto_enabled: current.auto_enabled,
        auto_time: current.auto_time.clone(),
        folder: current.folder.clone(),
        retention: current.retention,
        last_backup_at: patch.last_backup_at.or_else(|| current.last_backup_at.clone()),
        last_backup_kind: patch.last_backup_kind.or(current.last_backup_kind),
        last_auto_run_date: patch.last_auto_run_date.or_else(|| current.last_auto_run_date.clone()),
        last_backup_failed_at: if patch.clear_failure { None } else { patch.last_backup_failed_at.or_else(|| current.last_backup_failed_at.clone()) },
        last_backup_error: if patch.clear_failure { None } else { patch.last_backup_error.or_else(|| current.last_backup_error.clone()) },
    };

    let mut value = serde_json::json!({
        "autoEnabled": next.auto_enabled,
        "autoTime": next.auto_time,
        "retention": next.retention,
    });
    let obj = value.as_object_mut().expect("object literal");
    if let Some(v) = &next.last_backup_at {
        obj.insert("lastBackupAt".to_string(), serde_json::Value::String(v.clone()));
    }
    if let Some(v) = next.last_backup_kind {
        obj.insert("lastBackupKind".to_string(), serde_json::Value::String(kind_str(v).to_string()));
    }
    if let Some(v) = &next.last_auto_run_date {
        obj.insert("lastAutoRunDate".to_string(), serde_json::Value::String(v.clone()));
    }
    if let Some(v) = &next.last_backup_failed_at {
        obj.insert("lastBackupFailedAt".to_string(), serde_json::Value::String(v.clone()));
    }
    if let Some(v) = &next.last_backup_error {
        obj.insert("lastBackupError".to_string(), serde_json::Value::String(v.clone()));
    }
    if let Some(folder) = &next.folder {
        obj.insert("folder".to_string(), serde_json::Value::String(folder.clone()));
    }

    let store_patch = StoreSettingsPatch { backup: Some(value), ..Default::default() };
    crate::domains::settings::service::store::update_settings(conn, cx, registry, store_patch).await?;
    Ok(next)
}

fn doc_date_from_iso(iso: &str) -> Option<DocDate> {
    chrono::DateTime::parse_from_rfc3339(iso).ok().map(|dt| {
        let utc = dt.with_timezone(&chrono::Utc);
        DocDate { day: utc.date_naive(), instant: Some(utc) }
    })
}

/// `record_backup_saved(manifest, kind)` (`afterBackupSaved`, §3.4): sets `lastBackupAt`/
/// `lastBackupKind` and **clears** `lastBackupFailedAt`/`lastBackupError` (a successful backup of any
/// kind resets the "an automatic backup failed" notification), then logs the kind-specific activity
/// message.
pub async fn record_backup_saved<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    device: &DeviceSettings,
    created_at: &str,
    kind: BackupKind,
) -> TxResult<()> {
    save_backup_settings_clearing(
        conn,
        cx,
        registry,
        device,
        ClearingPatch {
            last_backup_at: Some(created_at.to_string()),
            last_backup_kind: Some(kind),
            clear_failure: true,
            last_auto_run_date: None,
            last_backup_failed_at: None,
            last_backup_error: None,
        },
    )
    .await?;

    let message = match kind {
        BackupKind::Manual => "إنشاء نسخة احتياطية يدوية",
        BackupKind::Auto => "نسخة احتياطية تلقائية",
        BackupKind::PreRestore => "نسخة احتياطية قبل الاستعادة",
    };
    crate::shared::activity::log(conn, cx, registry, ActivityKind::Settings, message, doc_date_from_iso(created_at), None).await?;
    Ok(())
}

/// `record_backup_failed(message)` (§3.4) — its own best-effort write; callers swallow any error
/// from this (the mock's own `.catch(() => {})`).
pub async fn record_backup_failed<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    device: &DeviceSettings,
    message: &str,
) -> TxResult<()> {
    save_backup_settings_clearing(
        conn,
        cx,
        registry,
        device,
        ClearingPatch {
            last_backup_at: None,
            last_backup_kind: None,
            clear_failure: false,
            last_auto_run_date: None,
            last_backup_failed_at: Some(crate::utils::dates::format_iso_ms(cx.clock.now)),
            last_backup_error: Some(message.to_string()),
        },
    )
    .await?;
    Ok(())
}

/// `run_auto_backup_if_due(trigger)` (§3.7). Returns the outcome; never returns `Err` for a business
/// failure during the backup itself (`record_backup_failed` + `{ ran: false, error }`, mirroring the
/// mock's swallow-and-report behavior) — only a hard infrastructure error (no DB connection, etc.)
/// propagates as an `AppError`.
pub async fn run_auto_backup_if_due(state: &AppState, trigger: AutoBackupTrigger) -> AppResult<AutoBackupOutcome> {
    let device = state.device.read().unwrap().clone();
    let is_main = device.role == crate::core::device::DeviceRole::Main || crate::core::device::debug_db_url_override().is_some();
    if !is_main {
        return Ok(AutoBackupOutcome { ran: false, path: None, skipped: Some(AutoBackupSkipReason::NotMain), error: None });
    }

    let registry = state.undo.clone();
    let device_for_tx = device.clone();
    let outcome = crate::core::tx::with_tx(state, crate::core::tx::TxOpts { require_user: false }, move |txn, cx| {
        let registry = registry.clone();
        let device = device_for_tx.clone();
        Box::pin(run_auto_backup_in_tx(txn, cx, registry, device, trigger)) as crate::core::tx::BoxFuture<'_, TxResult<AutoBackupOutcome>>
    })
    .await?;

    Ok(outcome)
}

async fn run_auto_backup_in_tx<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: std::sync::Arc<UndoRegistry>,
    device: DeviceSettings,
    trigger: AutoBackupTrigger,
) -> TxResult<AutoBackupOutcome> {
    let settings = backup_settings(conn, &device).await?;
    if !settings.auto_enabled {
        return Ok(AutoBackupOutcome { ran: false, path: None, skipped: Some(AutoBackupSkipReason::Disabled), error: None });
    }

    if trigger == AutoBackupTrigger::Schedule {
        // Q-1: `lastAutoRunDate` is the UTC day; "due" itself uses the business-clock local time —
        // ported as-is (kept intentionally inconsistent near midnight in UTC+2/+3, per the mock).
        let today_key = cx.clock.now.format("%Y-%m-%d").to_string();
        if settings.last_auto_run_date.as_deref() == Some(today_key.as_str()) {
            return Ok(AutoBackupOutcome { ran: false, path: None, skipped: Some(AutoBackupSkipReason::NotDue), error: None });
        }
        let (hour, minute) = local_hour_minute(cx);
        let (due_hour, due_minute) = parse_hh_mm(&settings.auto_time).unwrap_or((20, 0));
        let due = hour > due_hour || (hour == due_hour && minute >= due_minute);
        if !due {
            return Ok(AutoBackupOutcome { ran: false, path: None, skipped: Some(AutoBackupSkipReason::NotDue), error: None });
        }
    }

    let Some(folder) = settings.folder.clone() else {
        return Ok(AutoBackupOutcome { ran: false, path: None, skipped: Some(AutoBackupSkipReason::NoFolder), error: None });
    };

    match build_and_write_auto_backup(conn, cx, &registry, &device, &folder).await {
        Ok(path) => {
            if trigger == AutoBackupTrigger::Schedule {
                let today_key = cx.clock.now.format("%Y-%m-%d").to_string();
                save_backup_settings_clearing(
                    conn,
                    cx,
                    &registry,
                    &device,
                    ClearingPatch {
                        last_backup_at: None,
                        last_backup_kind: None,
                        clear_failure: false,
                        last_auto_run_date: Some(today_key),
                        last_backup_failed_at: None,
                        last_backup_error: None,
                    },
                )
                .await?;
            }
            prune_folder(&folder, settings.retention);
            Ok(AutoBackupOutcome { ran: true, path: Some(path), skipped: None, error: None })
        }
        Err(e) => {
            let message = e.to_string();
            // Best-effort: a failure recording the failure itself must never mask the original error.
            let _ = record_backup_failed(conn, cx, &registry, &device, &message).await;
            Ok(AutoBackupOutcome { ran: false, path: None, skipped: None, error: Some(message) })
        }
    }
}

/// The business-clock local wall-clock time (hour, minute) — `BusinessClock.tz` is `Some` when
/// `settings.timezone` is configured, else the OS-local timezone (mirrors `today()`'s own fallback,
/// `utils/dates.rs`'s `local_date_key`).
fn local_hour_minute(cx: &TxCtx) -> (u32, u32) {
    use chrono::{Local, Timelike};
    match cx.clock.tz {
        Some(tz) => {
            let local = cx.clock.now.with_timezone(&tz);
            (local.hour(), local.minute())
        }
        None => {
            let local = cx.clock.now.with_timezone(&Local);
            (local.hour(), local.minute())
        }
    }
}

fn parse_hh_mm(s: &str) -> Option<(u32, u32)> {
    let mut parts = s.split(':');
    let h: u32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    Some((h, m))
}

/// Shared dataset+counts+manifest assembly (§3.5, §3.7): dumps the current snapshot, counts, and
/// company name, then builds the zip — the one place both the manual `build_backup_archive` (a
/// read-only command, no write) and the auto-backup writer below assemble a `BuiltArchive` from,
/// so the manifest shape never drifts between the two callers.
async fn build_dataset_archive<C: ConnectionTrait>(
    conn: &C,
    created_at: String,
    kind: BackupKind,
    password: Option<&str>,
) -> Result<super::archive::BuiltArchive, AppError> {
    let dataset = dataset::dump_snapshot(conn).await.map_err(TxError::into_app_error)?;
    let schema_version = dataset.db_schema_version;
    let counts_map = counts::table_counts(conn).await.map_err(TxError::into_app_error)?;
    let row = crate::core::settings::load(conn).await?;
    let company = if row.store_name.trim().is_empty() { "company".to_string() } else { row.store_name.clone() };

    let payload = vec![("data.json".to_string(), serde_json::to_vec(&dataset).map_err(|e| AppError::internal("تعذر بناء النسخة الاحتياطية", Some(e.to_string())))?)];

    let company_for_manifest = company.clone();
    build_archive(
        move |checksum, encrypted| super::dto::BackupManifest {
            app: "accounting-app".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            schema_version,
            created_at,
            company: company_for_manifest,
            counts: counts_map,
            checksum,
            encrypted,
            kind,
        },
        payload,
        password,
    )
}

/// `build_backup_archive(kind, password)` (§3.5): the manual/on-demand build, `with_read` only (no
/// write happens here — the frontend saves the file through the native dialog, then
/// `settings_record_backup_saved` runs). Returns the archive's bytes as base64 plus its file name.
pub async fn build_and_write_or_read_archive<C: ConnectionTrait>(conn: &C, kind: BackupKind, password: Option<&str>) -> Result<super::dto::BackupArchive, AppError> {
    let created_at = crate::utils::dates::format_iso_ms(chrono::Utc::now());
    let built = build_dataset_archive(conn, created_at, kind, password).await?;
    let file_name = backup_file_name(&built.manifest.company, chrono::Utc::now().naive_utc());
    Ok(super::dto::BackupArchive { manifest: built.manifest, file_name, archive_base64: super::archive::bytes_to_base64(&built.bytes) })
}

async fn build_and_write_auto_backup<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    device: &DeviceSettings,
    folder: &str,
) -> Result<String, AppError> {
    let created_at = crate::utils::dates::format_iso_ms(cx.clock.now);
    let built = build_dataset_archive(conn, created_at, BackupKind::Auto, None).await?;

    let file_name = backup_file_name(&built.manifest.company, cx.clock.now.naive_utc());
    let full_path = format!("{}/{}", folder.trim_end_matches(['/', '\\']), file_name);
    write_file_creating_dirs(&full_path, &built.bytes)?;

    record_backup_saved(conn, cx, registry, device, &built.manifest.created_at, BackupKind::Auto).await.map_err(TxError::into_app_error)?;

    Ok(full_path)
}

fn write_file_creating_dirs(path: &str, bytes: &[u8]) -> Result<(), AppError> {
    let path_buf = std::path::PathBuf::from(path);
    if let Some(parent) = path_buf.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::internal("تعذر إنشاء مجلد النسخ الاحتياطي", Some(e.to_string())))?;
    }
    // Temp file + rename (§3.7 step 5) so a crash mid-write never leaves a half-written .zip that a
    // later `readManifest` scan would trip over.
    let tmp_path = path_buf.with_extension("zip.tmp");
    std::fs::write(&tmp_path, bytes).map_err(|e| AppError::internal("تعذر كتابة ملف النسخة الاحتياطية", Some(e.to_string())))?;
    std::fs::rename(&tmp_path, &path_buf).map_err(|e| AppError::internal("تعذر كتابة ملف النسخة الاحتياطية", Some(e.to_string())))?;
    Ok(())
}

/// §3.7 step 5's prune: every `*.zip` in `folder` whose manifest parses, newest `createdAt` first,
/// delete beyond `retention`. Errors (unreadable file, permission) are ignored per file — a retention
/// sweep must never fail the backup that just succeeded.
fn prune_folder(folder: &str, retention: i32) {
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    let mut dated: Vec<(String, std::path::PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("zip") {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let Ok(manifest) = read_manifest(&bytes) else { continue };
        dated.push((manifest.created_at, path));
    }
    dated.sort_by(|a, b| b.0.cmp(&a.0)); // newest first.
    let retention = retention.max(0) as usize;
    for (_, path) in dated.into_iter().skip(retention) {
        let _ = std::fs::remove_file(path);
    }
}
