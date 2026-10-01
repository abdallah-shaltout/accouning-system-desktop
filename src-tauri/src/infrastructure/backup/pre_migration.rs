//! `pre_migration.rs` (17-backup.md §3.10, P2-52 handoff) — the automatic backup `core::db::migrate`
//! takes before applying any pending migration on the Main PC (G-45/GB-2). No IPC command; called
//! only from `core::db::migrate` through the `core::db::PreMigrationBackup` trait seam.

use std::path::{Path, PathBuf};

use sea_orm::DatabaseConnection;

use crate::core::tx::with_read_on;

use super::archive::{backup_file_name, build_archive};
use super::counts;
use super::dataset;
use super::dto::{BackupKind, BackupManifest};

const KEEP_NEWEST: usize = 10;

/// `backup_before_migrations` (§3.10): dumps the **old** schema (before `Migrator::up` runs) through
/// one `with_read_on` snapshot, writes an unencrypted `kind: auto` archive to
/// `<out_dir>/pre-migration/<fileName>`, keeps the newest 10 files there, and returns the written
/// path. `Ok(None)` when there is nothing pending (the caller — `core::db::migrate` — already only
/// calls this when `pending` is non-empty, but this stays defensive: a fresh install or an
/// already-current DB must never be treated as a failure to back up).
pub async fn backup_before_migrations(db: &DatabaseConnection, out_dir: &Path) -> Result<Option<PathBuf>, String> {
    use migration::MigratorTrait;
    let applied = migration::Migrator::get_applied_migrations(db).await.map_err(|e| e.to_string())?;
    if applied.is_empty() {
        return Ok(None); // fresh install — nothing applied yet, so there is no old schema to back up.
    }
    let pending = migration::Migrator::get_pending_migrations(db).await.map_err(|e| e.to_string())?;
    if pending.is_empty() {
        return Ok(None); // already current — the caller shouldn't reach here, but stay defensive.
    }

    let out_dir_owned = out_dir.to_path_buf();
    let path = with_read_on(db, move |txn| {
        let out_dir_owned = out_dir_owned.clone();
        Box::pin(async move { dump_and_write(txn, &out_dir_owned).await.map_err(crate::core::tx::TxError::App) })
            as crate::core::tx::BoxFuture<'_, crate::core::tx::TxResult<PathBuf>>
    })
    .await
    .map_err(|e| e.to_string())?;

    prune_pre_migration_dir(&out_dir.join("pre-migration"));

    Ok(Some(path))
}

async fn dump_and_write<C: sea_orm::ConnectionTrait>(conn: &C, out_dir: &Path) -> Result<PathBuf, crate::core::error::AppError> {
    use crate::core::tx::TxError;

    let dataset = dataset::dump_snapshot(conn).await.map_err(TxError::into_app_error)?;
    let schema_version = dataset.db_schema_version;
    let counts_map = counts::table_counts(conn).await.map_err(TxError::into_app_error)?;

    // The settings row belongs to the OLD schema being backed up — read it defensively (a row might
    // not exist yet on a very early schema), falling back to "company" like every other manifest
    // builder does.
    let company = match crate::core::settings::load(conn).await {
        Ok(row) if !row.store_name.trim().is_empty() => row.store_name,
        _ => "company".to_string(),
    };

    let now = chrono::Utc::now();
    let created_at = crate::utils::dates::format_iso_ms(now);
    let mut payload = vec![(
        "data.json".to_string(),
        serde_json::to_vec(&dataset).map_err(|e| crate::core::error::AppError::internal("تعذر بناء النسخة الاحتياطية", Some(e.to_string())))?,
    )];
    // C-16: same reasoning as `auto.rs`'s `build_dataset_archive` — `attachments` is excluded from
    // the generic dataset dump (LONGBLOB columns), packed here instead so a pre-migration backup
    // stays restorable through the same `settings_restore_from_archive` path a manual one uses.
    super::attachments::pack_attachments(conn, &mut payload).await.map_err(crate::core::tx::TxError::into_app_error)?;

    let company_for_manifest = company.clone();
    let built = build_archive(
        move |checksum, encrypted| BackupManifest {
            app: "accounting-app".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            schema_version,
            created_at,
            company: company_for_manifest,
            counts: counts_map,
            checksum,
            encrypted,
            kind: BackupKind::Auto,
        },
        payload,
        None, // P2-44: unencrypted — the backups folder is as protected as the data dir itself.
    )?;

    let dir = out_dir.join("pre-migration");
    std::fs::create_dir_all(&dir).map_err(|e| crate::core::error::AppError::internal("تعذر إنشاء مجلد النسخ الاحتياطي", Some(e.to_string())))?;
    let file_name = backup_file_name(&company, now.naive_utc());
    let final_path = dir.join(&file_name);
    let tmp_path = dir.join(format!("{file_name}.tmp"));
    std::fs::write(&tmp_path, &built.bytes)
        .map_err(|e| crate::core::error::AppError::internal("تعذر كتابة ملف النسخة الاحتياطية", Some(e.to_string())))?;
    std::fs::rename(&tmp_path, &final_path)
        .map_err(|e| crate::core::error::AppError::internal("تعذر كتابة ملف النسخة الاحتياطية", Some(e.to_string())))?;

    Ok(final_path)
}

/// Keeps only the newest 10 files in `pre-migration/` (by filesystem modified time — no
/// `settings`/activity rows exist for this schema-independent folder, so there's no manifest
/// `createdAt` reader in scope here; the file name's own timestamp plus mtime agree in practice).
fn prune_pre_migration_dir(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("zip") {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(modified) = meta.modified() else { continue };
        files.push((modified, path));
    }
    files.sort_by(|a, b| b.0.cmp(&a.0)); // newest first.
    for (_, path) in files.into_iter().skip(KEEP_NEWEST) {
        let _ = std::fs::remove_file(path);
    }
}

/// The `core::db::PreMigrationBackup` trait implementation `lib.rs`'s boot wiring passes to
/// `core::db::migrate` in place of `NoPendingMigrationBackup` (GB-2's "Needs from manager" call
/// site). `out_dir` is resolved once at construction — `ServerPaths::backups()` on a managed server,
/// else `<app_data_dir>/backups` (§3.10).
pub struct RealPreMigrationBackup {
    pub out_dir: PathBuf,
}

#[async_trait::async_trait]
impl crate::core::db::PreMigrationBackup for RealPreMigrationBackup {
    async fn backup_before_migrations(&self, db: &DatabaseConnection) -> Result<(), String> {
        backup_before_migrations(db, &self.out_dir).await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_newest_constant_matches_spec() {
        assert_eq!(KEEP_NEWEST, 10);
    }
}
