//! `restore.rs` (17-backup.md §3.8, §3.9) — `preview_restore` (read-only, just the manifest) and
//! `restore_from_archive` (the whole replace: gates, decrypt+checksum, mandatory pre-restore
//! backup, one `with_tx` that wipes+loads or re-runs the legacy importer, then clears the session).

use sea_orm::ConnectionTrait;

use crate::core::device::{DeviceRole, DeviceSettings};
use crate::core::error::{AppError, AppResult};
use crate::core::events::ChangeCategory;
use crate::core::state::AppState;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::platform::activity::ActivityKind;
use crate::infrastructure::import::run::{import_snapshot, ImportOpts};
use crate::infrastructure::import::dto::ImportMode;
use crate::shared::activity::{record, AuditInput};
use crate::shared::activity::undo::UndoRegistry;
use crate::utils::id::Id;

use super::archive::{backup_file_name, build_archive, decrypt_archive, find_payload_file, parse_archive, recompute_checksum};
use super::auto::record_backup_saved;
use super::counts;
use super::dataset::{self, DataSetV1};
use super::dto::{BackupKind, BackupManifest, RestorePreview};
use super::upgrade::upgrade_to_build;

/// `db_schema_version(conn)`'s space starts at 100 (§3.2 D-3) — anything below is a browser
/// (mock) archive's `SCHEMA_VERSION`.
const RUST_SCHEMA_FLOOR: u32 = 100;

/// The mock's own highest known `SCHEMA_VERSION` (`persist.ts:18` = 1 at the time this was written)
/// — a browser archive whose `schemaVersion` is above that (but still `< RUST_SCHEMA_FLOOR`) was
/// produced by a *newer* browser build than this Rust build's importer understands (§3.8).
const BROWSER_SCHEMA_KNOWN_MAX: u32 = 1;

const NOTE_UPGRADE: &str = "سيتم ترقية بيانات هذه النسخة تلقائياً إلى الإصدار الحالي عند الاستعادة";
const NOTE_TOO_NEW: &str = "هذه النسخة أُنشئت بإصدار أحدث من التطبيق الحالي — يلزم تحديث التطبيق قبل الاستعادة";

/// `preview_restore(archive)` (§3.8): reads the manifest only, decides compatibility from
/// `schemaVersion` without touching the database.
pub fn preview_restore(bytes: &[u8]) -> AppResult<RestorePreview> {
    let manifest = super::archive::read_manifest(bytes)?;
    let build = dataset::build_schema_version();
    let (compatible, compatibility_note) = classify_version(manifest.schema_version, build);
    Ok(RestorePreview { manifest, compatible, compatibility_note })
}

/// Shared classification (§3.8) used by both `preview_restore` and `restore_from_archive` (the
/// latter re-derives it right before committing, so a build upgrade between preview and confirm
/// can never restore an archive the preview would have called incompatible).
fn classify_version(schema_version: u32, build: u32) -> (bool, Option<String>) {
    if schema_version < RUST_SCHEMA_FLOOR {
        if schema_version <= BROWSER_SCHEMA_KNOWN_MAX {
            (true, Some(NOTE_UPGRADE.to_string()))
        } else {
            (false, Some(NOTE_TOO_NEW.to_string()))
        }
    } else if schema_version > build {
        (false, Some(NOTE_TOO_NEW.to_string()))
    } else if schema_version < build {
        (true, Some(NOTE_UPGRADE.to_string()))
    } else {
        (true, None)
    }
}

/// Gate (§3.9 step 1, D-8): Main PC only (or the debug DB URL override), and refused while any LAN
/// terminal is connected. Windows-only LAN sharing means every other platform trivially has zero
/// connected terminals — `get_status`'s own `FORBIDDEN` on non-Windows is not a "terminals
/// connected" condition, so it is not surfaced here as one.
async fn require_restore_allowed(state: &AppState) -> AppResult<()> {
    let device = state.device.read().unwrap().clone();
    let is_main = device.role == DeviceRole::Main || crate::core::device::debug_db_url_override().is_some();
    if !is_main {
        return Err(AppError::forbidden("الاستعادة متاحة على الجهاز الرئيسي فقط"));
    }

    #[cfg(windows)]
    {
        if let Ok(status) = crate::domains::settings::service::network::get_status(state).await {
            if status.lan_sharing && status.connected_terminals > 0 {
                return Err(AppError::conflict(format!(
                    "أغلق البرنامج على أجهزة الكاشير أولاً — عدد الأجهزة المتصلة: {}",
                    status.connected_terminals
                )));
            }
        }
    }

    Ok(())
}

struct DecryptedArchive {
    manifest: BackupManifest,
    data_json: Vec<u8>,
    /// Every payload file (unencrypted archive: as stored; encrypted: unpacked) — used to find
    /// `attachments/<id>.*` entries (C-16) alongside `data.json`.
    files: super::archive::PayloadFiles,
}

/// Parses, decrypts (if needed) and checksum-verifies the archive (§3.9 step 2). Returns the raw
/// `data.json` bytes either way — the caller decides (by `schemaVersion`) whether to hand them to
/// the legacy importer or to `serde_json`-deserialize them as a `DataSetV1`.
fn parse_and_verify(bytes: &[u8], password: Option<&str>) -> AppResult<DecryptedArchive> {
    let parsed = parse_archive(bytes)?;

    if parsed.manifest.encrypted && password.map(str::trim).unwrap_or("").is_empty() {
        return Err(AppError::validation("هذه النسخة مشفّرة — أدخل كلمة المرور"));
    }

    let files = if parsed.manifest.encrypted {
        let password = password.expect("checked non-empty above");
        decrypt_archive(&parsed, password)?
    } else {
        parsed.payload_files.clone().expect("unencrypted archive always carries payload_files")
    };
    let data_json = find_payload_file(&files, "data.json")
        .map(|b| b.to_vec())
        .ok_or_else(|| AppError::validation("ملف النسخة الاحتياطية غير صالح: data.json مفقود"))?;

    // D-9: verify the checksum before anything is restored — a corrupted/tampered archive must
    // never partially apply.
    let recomputed = recompute_checksum(bytes)?;
    if recomputed != parsed.manifest.checksum {
        return Err(AppError::validation("المجموع الاختباري غير مطابق — الملف قد يكون تالفاً"));
    }

    Ok(DecryptedArchive { manifest: parsed.manifest, data_json, files })
}

/// The mandatory pre-restore backup (§3.9 step 3): built and written in its own `with_read`
/// snapshot + its own settings write, entirely separate from the restore transaction — a failure
/// here must leave the database completely untouched (nothing has been wiped yet).
async fn take_pre_restore_backup(state: &AppState, registry: std::sync::Arc<UndoRegistry>, device: &DeviceSettings) -> AppResult<()> {
    let out_dir = pre_migration_sibling_backups_dir(state);
    let dir = out_dir.join("pre-restore");

    let built = crate::core::tx::with_read(state, move |conn| {
        Box::pin(async move { build_pre_restore_archive(conn).await.map_err(TxError::App) }) as crate::core::tx::BoxFuture<'_, TxResult<super::archive::BuiltArchive>>
    })
    .await
    .map_err(|e| AppError::internal("تعذر أخذ نسخة احتياطية قبل الاستعادة — لم تتم الاستعادة", Some(e.to_string())))?;

    std::fs::create_dir_all(&dir).map_err(|e| AppError::internal("تعذر أخذ نسخة احتياطية قبل الاستعادة — لم تتم الاستعادة", Some(e.to_string())))?;
    let final_path = dir.join(backup_file_name(&built.manifest.company, chrono::Utc::now().naive_utc()));
    let tmp_path = final_path.with_extension("zip.tmp");
    std::fs::write(&tmp_path, &built.bytes).map_err(|e| AppError::internal("تعذر أخذ نسخة احتياطية قبل الاستعادة — لم تتم الاستعادة", Some(e.to_string())))?;
    std::fs::rename(&tmp_path, &final_path).map_err(|e| AppError::internal("تعذر أخذ نسخة احتياطية قبل الاستعادة — لم تتم الاستعادة", Some(e.to_string())))?;

    let manifest = built.manifest;
    let device = device.clone();
    crate::core::tx::with_tx(state, crate::core::tx::TxOpts { require_user: false }, move |tx, cx| {
        let manifest = manifest.clone();
        let device = device.clone();
        let registry = registry.clone();
        Box::pin(async move { record_backup_saved(tx, cx, &registry, &device, &manifest.created_at, BackupKind::PreRestore).await })
            as crate::core::tx::BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(|e| AppError::internal("تعذر أخذ نسخة احتياطية قبل الاستعادة — لم تتم الاستعادة", Some(e.to_string())))?;

    Ok(())
}

async fn build_pre_restore_archive<C: ConnectionTrait>(conn: &C) -> Result<super::archive::BuiltArchive, AppError> {
    let dataset = dataset::dump_snapshot(conn).await.map_err(TxError::into_app_error)?;
    let schema_version = dataset.db_schema_version;
    let counts_map = counts::table_counts(conn).await.map_err(TxError::into_app_error)?;
    let row = crate::core::settings::load(conn).await?;
    let company = if row.store_name.trim().is_empty() { "company".to_string() } else { row.store_name.clone() };
    let created_at = crate::utils::dates::format_iso_ms(chrono::Utc::now());
    let mut payload = vec![(
        "data.json".to_string(),
        serde_json::to_vec(&dataset).map_err(|e| AppError::internal("تعذر بناء النسخة الاحتياطية", Some(e.to_string())))?,
    )];
    // C-16: same reasoning as `auto.rs`'s `build_dataset_archive` — the mandatory pre-restore safety
    // backup must carry attachments too, or a restore of it would silently lose every attachment.
    super::attachments::pack_attachments(conn, &mut payload).await.map_err(TxError::into_app_error)?;
    let company_for_manifest = company.clone();
    build_archive(
        move |checksum, encrypted| BackupManifest {
            app: "accounting-app".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            schema_version,
            created_at,
            company: company_for_manifest,
            counts: counts_map,
            checksum,
            encrypted,
            kind: BackupKind::PreRestore,
        },
        payload,
        None,
    )
}

/// `<app_data_dir>/backups`, or `ServerPaths::backups()` on a managed server — mirrors
/// `pre_migration.rs`'s own `out_dir` resolution (§3.10) so pre-restore and pre-migration archives
/// live under the same root, in sibling folders (`pre-migration/`, `pre-restore/`).
fn pre_migration_sibling_backups_dir(state: &AppState) -> std::path::PathBuf {
    if let Some(paths) = crate::infrastructure::database::paths::ServerPaths::machine() {
        return paths.backups();
    }
    state.app_data_dir.join("backups")
}

/// `restore_from_archive(archive, password)` (§3.9). Ends every session on this machine after a
/// successful commit (D-11) — the caller (the IPC command) clears `state.session` once this
/// returns `Ok`.
pub async fn restore_from_archive(state: &AppState, archive_base64: &str, password: Option<&str>) -> AppResult<()> {
    require_restore_allowed(state).await?;

    let bytes = super::archive::base64_to_bytes(archive_base64)?;
    let decrypted = parse_and_verify(&bytes, password)?;

    let build = dataset::build_schema_version();
    let (compatible, note) = classify_version(decrypted.manifest.schema_version, build);
    if !compatible {
        return Err(AppError::validation(note.unwrap_or_default()));
    }

    // D-7 before anything is written: the step-4 transaction re-checks Settings:Write + Users:Write,
    // but the pre-restore backup below already writes a file and an activity row — an unauthorized
    // caller must be refused here, not after that (and not with the misleading pre-restore error).
    let actor = state.session.read().unwrap().clone();
    crate::core::tx::with_read(state, move |conn| {
        let actor = actor.clone();
        Box::pin(async move {
            crate::core::settings::require(conn, actor.as_ref(), crate::core::auth::Area::Settings, crate::core::auth::Access::Write).await?;
            crate::core::settings::require(conn, actor.as_ref(), crate::core::auth::Area::Users, crate::core::auth::Access::Write).await?;
            Ok(())
        }) as crate::core::tx::BoxFuture<'_, TxResult<()>>
    })
    .await?;

    let device = state.device.read().unwrap().clone();
    let registry = state.undo.clone();

    // Step 3: the mandatory pre-restore backup, in its own snapshot/transaction, BEFORE anything
    // is touched — any failure here means nothing is restored (D-12).
    take_pre_restore_backup(state, registry.clone(), &device).await?;

    let schema_version = decrypted.manifest.schema_version;
    let created_at = decrypted.manifest.created_at.clone();
    let data_json = decrypted.data_json.clone();
    let files = decrypted.files.clone();
    let terminal_id = state.terminal.terminal_id;

    let outcome = crate::core::tx::with_tx(state, crate::core::tx::TxOpts { require_user: false }, move |tx, cx| {
        let data_json = data_json.clone();
        let files = files.clone();
        let created_at = created_at.clone();
        let registry = registry.clone();
        Box::pin(run_restore_in_tx(tx, cx, registry, schema_version, created_at, data_json, files, terminal_id)) as crate::core::tx::BoxFuture<'_, TxResult<RestoreOutcome>>
    })
    .await?;

    // Step 5: after commit — clear the session (restored users/passwords may differ, D-11) and, for
    // a browser archive, merge the importer's device fields the same way 00-import's command does.
    *state.session.write().unwrap() = None;

    if let Some(device_fields) = outcome.device_fields {
        let mut new_device = crate::core::device::load(&state.app_data_dir)
            .map_err(|e| AppError::internal("تعذر قراءة إعدادات الجهاز", Some(e.to_string())))?;
        if new_device.printer.thermal.is_none() && device_fields.thermal_printer_name.is_some() {
            new_device.printer.thermal = Some(crate::core::device::ThermalPrinterConfig {
                printer_name: device_fields.thermal_printer_name.clone(),
                ..Default::default()
            });
        }
        if new_device.printer.a4_printer_name.is_none() && device_fields.a4_printer_name.is_some() {
            new_device.printer.a4_printer_name = device_fields.a4_printer_name.clone();
        }
        if new_device.printer.label_printer_name.is_none() && device_fields.label_printer_name.is_some() {
            new_device.printer.label_printer_name = device_fields.label_printer_name.clone();
        }
        if new_device.backup_folder.is_none() && device_fields.backup_folder.is_some() {
            new_device.backup_folder = device_fields.backup_folder.clone();
        }
        crate::core::device::save(&state.app_data_dir, &new_device)
            .map_err(|e| AppError::internal("تعذر حفظ إعدادات الجهاز", Some(e.to_string())))?;
        *state.device.write().unwrap() = new_device;
    }

    Ok(())
}

struct RestoreOutcome {
    device_fields: Option<crate::infrastructure::import::settings::DeviceFields>,
}

async fn run_restore_in_tx<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: std::sync::Arc<UndoRegistry>,
    schema_version: u32,
    created_at: String,
    data_json: Vec<u8>,
    files: super::archive::PayloadFiles,
    terminal_id: Id,
) -> TxResult<RestoreOutcome> {
    cx.require(conn, crate::core::auth::Area::Settings, crate::core::auth::Access::Write).await?;
    cx.require(conn, crate::core::auth::Area::Users, crate::core::auth::Access::Write).await?;

    let device_fields = if schema_version < RUST_SCHEMA_FLOOR {
        // Browser archive: wipe then re-run the legacy importer (00-import), which ends with its
        // own `run_all` invariants check.
        let order = dataset::topo_order(conn).await?;
        // `wipe` empties `document_counters` too (a Rust archive reloads it from its dump), but the
        // importer only UPDATEs the migration-seeded rows (`numbering::set_counter`) — re-seed every
        // kind that existed at 0 first, so its step 10 finds them and the code locks stay 0.
        let counter_kinds = document_counter_kinds(conn).await?;
        dataset::wipe(conn, &order).await?;
        reseed_document_counters(conn, &counter_kinds).await?;

        let snapshot_json = String::from_utf8(data_json)
            .map_err(|_| TxError::App(AppError::validation("ملف النسخة الاحتياطية غير صالح")))?;
        let data_value: serde_json::Value = serde_json::from_str(&snapshot_json)
            .map_err(|_| TxError::App(AppError::validation("ملف النسخة الاحتياطية غير صالح")))?;
        let envelope = serde_json::json!({ "version": schema_version, "savedAt": created_at, "data": data_value });
        let envelope_json = serde_json::to_string(&envelope)
            .map_err(|e| TxError::App(AppError::internal("تعذر قراءة النسخة الاحتياطية", Some(e.to_string()))))?;

        let report = import_snapshot(
            conn,
            &envelope_json,
            None,
            None,
            ImportOpts { mode: ImportMode::Legacy, replace_existing: false, adopt_terminal: Some(terminal_id) },
        )
        .await?;
        Some(report.device_fields)
    } else {
        // Rust archive: upgrade row-shape if older than this build, then wipe + load, then
        // invariants.
        let mut dataset_v1: DataSetV1 = serde_json::from_slice(&data_json)
            .map_err(|_| TxError::App(AppError::validation("ملف النسخة الاحتياطية غير صالح")))?;
        let build = dataset::build_schema_version();
        upgrade_to_build(&mut dataset_v1, build).map_err(|msg| TxError::App(AppError::internal("تعذرت ترقية بيانات النسخة الاحتياطية", Some(msg))))?;

        let order = dataset::topo_order(conn).await?;
        dataset::wipe(conn, &order).await?;
        dataset::load(conn, &dataset_v1, &order).await?;

        // C-16: `attachments` is excluded from `dataset`'s generic dump/load (LONGBLOB columns) —
        // restore it separately from the archive's `attachments/<id>.*` entries, if any (an archive
        // written before C-16 has none, and simply restores zero attachments).
        super::attachments::restore_attachments(conn, &files).await?;

        let results = crate::shared::invariants::run_all(conn).await.map_err(TxError::App)?;
        if let Some(first_failed) = results.iter().find(|r| !r.passed) {
            return Err(TxError::App(AppError::validation(format!(
                "تعذر الاستعادة — البيانات لا تحقق قاعدة \"{}\": {}",
                first_failed.doc, first_failed.message
            ))));
        }
        None
    };

    // D-10: attribute the activity row to the current actor if that id still exists among the
    // restored users, else the first active admin of the restored data (FK-safe: `audit.user_id`/
    // `activity.user_id` must reference a row that now exists).
    let user_id = resolve_restore_actor(conn, cx.actor.as_ref().map(|a| a.id)).await?;

    record(
        conn,
        cx,
        &registry,
        AuditInput {
            entity: "settings".to_string(),
            entity_id: Id::new(),
            entity_label: None,
            action: crate::entities::platform::audit::AuditAction::Update,
            before: None,
            after: None,
            user_id: Some(user_id),
            branch_id: None,
            at: None,
            reason: None,
            message: "استعادة من نسخة احتياطية".to_string(),
            link: None,
            activity_kind: Some(ActivityKind::Settings),
            undo: None,
        },
    )
    .await?;

    cx.touch(ChangeCategory::Ledger);
    cx.touch(ChangeCategory::Catalog);
    cx.touch(ChangeCategory::Parties);

    Ok(RestoreOutcome { device_fields })
}

/// Every `document_counters.kind` present now (the migrations' seed set, whatever it grows to).
async fn document_counter_kinds<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<String>> {
    let stmt = sea_orm::Statement::from_string(conn.get_database_backend(), "SELECT `kind` FROM `document_counters` ORDER BY `kind`".to_string());
    let rows = conn.query_all(stmt).await.map_err(TxError::from)?;
    rows.iter().map(|r| r.try_get::<String>("", "kind").map_err(TxError::from)).collect()
}

async fn reseed_document_counters<C: ConnectionTrait>(conn: &C, kinds: &[String]) -> TxResult<()> {
    for kind in kinds {
        let stmt = sea_orm::Statement::from_sql_and_values(
            conn.get_database_backend(),
            "INSERT INTO `document_counters` (`kind`, `value`) VALUES (?, 0) ON DUPLICATE KEY UPDATE `value` = 0",
            [kind.clone().into()],
        );
        conn.execute(stmt).await.map_err(TxError::from)?;
    }
    Ok(())
}

/// D-10's lookup: the current actor's id if it exists among the just-restored `users` rows, else
/// the oldest active admin. Raw SQL (not the `users` entity) because this runs against whatever
/// schema the restore just loaded, inside the same transaction — consistent with the rest of this
/// module reading `information_schema`/tables directly rather than through entities.
async fn resolve_restore_actor<C: ConnectionTrait>(conn: &C, current_actor: Option<Id>) -> TxResult<Id> {
    use sea_orm::Statement;

    if let Some(id) = current_actor {
        let stmt = Statement::from_sql_and_values(conn.get_database_backend(), "SELECT `id` FROM `users` WHERE `id` = ? LIMIT 1", [id.to_string().into()]);
        if conn.query_one(stmt).await.map_err(TxError::from)?.is_some() {
            return Ok(id);
        }
    }

    let stmt = Statement::from_string(
        conn.get_database_backend(),
        "SELECT `id` FROM `users` WHERE `active` = 1 AND `role` = 'admin' ORDER BY `created_at` ASC LIMIT 1".to_string(),
    );
    if let Some(row) = conn.query_one(stmt).await.map_err(TxError::from)? {
        let id: Id = row.try_get("", "id").map_err(TxError::from)?;
        return Ok(id);
    }

    // No admin at all in the restored data (should never happen — invariants/importer require
    // one) — fall back to any user so the activity row is never orphaned.
    let stmt = Statement::from_string(conn.get_database_backend(), "SELECT `id` FROM `users` ORDER BY `created_at` ASC LIMIT 1".to_string());
    if let Some(row) = conn.query_one(stmt).await.map_err(TxError::from)? {
        let id: Id = row.try_get("", "id").map_err(TxError::from)?;
        return Ok(id);
    }

    Err(TxError::App(AppError::internal("تعذر إسناد عملية الاستعادة — لا يوجد مستخدمون في البيانات المستعادة", None)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_version_matches_spec_table() {
        let build = 115;
        assert_eq!(classify_version(1, build), (true, Some(NOTE_UPGRADE.to_string())));
        assert_eq!(classify_version(50, build), (false, Some(NOTE_TOO_NEW.to_string())));
        assert_eq!(classify_version(114, build), (true, Some(NOTE_UPGRADE.to_string())));
        assert_eq!(classify_version(115, build), (true, None));
        assert_eq!(classify_version(116, build), (false, Some(NOTE_TOO_NEW.to_string())));
    }
}
