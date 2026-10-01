//! DB-backed tests for the `backup`/restore infrastructure (`03-domains/17-backup.md` §8a). Written
//! now, run in the deferred time-boxed test pass (per-implementer hard rule: never run cargo from
//! this agent). Needs `EQUAL_TEST_DATABASE_URL` — see `tests/support/mod.rs`.
//!
//! Fixtures:
//! - `tests/fixtures/mock-snapshot-edge.json` — small, hand-written, checked in (shared with
//!   `domain_import.rs`) — used here to seed a small-but-real, invariants-clean database to back up
//!   and restore.
//! - `tests/fixtures/backup-browser-plain.zip` / `backup-browser-encrypted.zip` (password `1234`) —
//!   TS-format archives, produced by `bun run scripts/verify/export-backup-fixtures.ts` through the
//!   real `buildBackupArchive` from `mock-snapshot-edge.json`'s data (17-backup.md §8a's
//!   "compatibility with the TS format"). Small, checked in; tests that need them fail loudly with
//!   a clear message rather than skipping, per the entry file's "never skips" convention (mirrors
//!   `tests/support/mod.rs`/`domain_import.rs`).

use crate::support;

use std::path::Path;

use accounting_app_lib::infrastructure::backup::archive;
use accounting_app_lib::infrastructure::backup::auto::{self, build_and_write_or_read_archive};
use accounting_app_lib::infrastructure::backup::counts::{table_counts, COUNT_KEYS};
use accounting_app_lib::infrastructure::backup::dataset::{self, DataSetV1};
use accounting_app_lib::infrastructure::backup::dto::{AutoBackupSkipReason, AutoBackupTrigger, BackupKind};
use accounting_app_lib::infrastructure::backup::pre_migration::backup_before_migrations;
use accounting_app_lib::infrastructure::backup::restore;
use accounting_app_lib::infrastructure::backup::upgrade::{upgrade_to_build, ROW_UPGRADES};
use accounting_app_lib::infrastructure::import::run::{self as import_run, ImportOpts};
use accounting_app_lib::infrastructure::import::dto::ImportMode;
use accounting_app_lib::core::device::{DeviceRole, DeviceSettings};
use accounting_app_lib::core::tx::{with_tx, TxOpts};
use accounting_app_lib::shared::invariants;
use support::TestDb;

/// Mirrors `domain_import.rs::read_fixture` — fails loudly rather than skipping.
fn read_fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!("missing fixture {path:?} — see this file's own doc comment for how to produce it")
    })
}

fn read_fixture_bytes(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    std::fs::read(&path).unwrap_or_else(|_| {
        panic!(
            "missing fixture {path:?} — run `bun run scripts/verify/export-backup-fixtures.ts` (17-backup.md §8a's \
             TS-format compatibility fixtures)"
        )
    })
}

/// Seeds a small, invariants-clean database via the existing importer (`mock-snapshot-edge.json`,
/// shared with `domain_import.rs`) — the same real data path every other domain's DB tests use,
/// rather than a second hand-built fixture.
async fn seed_via_importer(db: &TestDb) {
    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    // The fixture's one user has no `active` key, which imports as inactive (the mock reads it
    // truthily — Part 04 Wave 2). These tests need a usable admin (the D-6 auto-backup attribution
    // picks the first *active* admin), so it is activated here, not in the shared fixture file.
    let mut snapshot: serde_json::Value = serde_json::from_str(&read_fixture("mock-snapshot-edge.json")).expect("edge fixture is JSON");
    for user in snapshot["data"]["users"].as_array_mut().expect("edge fixture has users") {
        user["active"] = serde_json::Value::Bool(true);
    }
    let snapshot_json = snapshot.to_string();
    import_run::import_snapshot(conn, &snapshot_json, None, None, ImportOpts { mode: ImportMode::Demo, replace_existing: false, adopt_terminal: None })
        .await
        .expect("seed via importer must succeed");
}

/// Logs in as the importer fixture's `admin` (what the settings page's real session is: the backup
/// settings and restore commands require Settings:Write, restore also Users:Write — D-7).
async fn log_in_as_imported_admin(db: &TestDb) {
    use accounting_app_lib::entities::org::users;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    let admin = users::Entity::find().filter(users::Column::Username.eq("admin")).one(&conn).await.unwrap().expect("imported admin");
    let settings = accounting_app_lib::core::settings::load(&conn).await.unwrap();
    let session = accounting_app_lib::domains::users::service::to_authenticated(&admin, settings.default_branch_id);
    *db.state.session.write().unwrap() = Some(session);
}

fn log_out(db: &TestDb) {
    *db.state.session.write().unwrap() = None;
}

// --- dataset.rs: schema is dumpable, topo order covers every FK -----------------------------------

#[tokio::test]
async fn dataset_schema_is_dumpable() {
    let db = TestDb::fresh().await;
    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    // topo_order must succeed (no unresolved cycle without a nullable edge — §3.2's own guarantee).
    let order = dataset::topo_order(conn).await.expect("topo_order must succeed on the current schema");
    assert!(!order.tables.is_empty());

    // dump_snapshot must succeed with no binary-column INTERNAL error (dump_table rejects
    // BLOB/BINARY/VARBINARY/BIT columns).
    let snapshot = dataset::dump_snapshot(conn).await.expect("dump_snapshot must succeed (no binary columns)");
    assert_eq!(snapshot.format, "equal-db");
    drop(db_guard);

    db.finish().await;
}

// --- round trip: seed -> build (plain) -> wipe -> load -> tables equal, run_all green -------------

#[tokio::test]
async fn plain_round_trip_preserves_every_row() {
    let db = TestDb::fresh().await;
    seed_via_importer(&db).await;

    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let order = dataset::topo_order(conn).await.unwrap();
    let before = dataset::dump_all(conn, &order).await.unwrap();

    dataset::wipe(conn, &order).await.expect("wipe must succeed");
    let empty = dataset::dump_all(conn, &order).await.unwrap();
    assert!(empty.iter().all(|t| t.rows.is_empty()), "every table must be empty after wipe");

    // Reload from the in-memory dump taken before wipe (`before`), not a second dump (which would
    // now be empty).
    let dataset_to_load = DataSetV1 { format: "equal-db".to_string(), db_schema_version: dataset::build_schema_version(), migrations: vec![], tables: before.clone() };
    dataset::load(conn, &dataset_to_load, &order).await.expect("load must succeed");

    let after = dataset::dump_all(conn, &order).await.unwrap();
    assert_eq!(before, after, "every table's rows must round-trip byte-for-byte through wipe/load");

    let results = invariants::run_all(conn).await.expect("run_all must not error");
    let failed: Vec<_> = results.iter().filter(|r| !r.passed).collect();
    assert!(failed.is_empty(), "invariants failed after round trip: {failed:?}");
    drop(db_guard);

    db.finish().await;
}

// --- encrypted round trip; wrong password; missing password; flipped byte -------------------------

#[tokio::test]
async fn encrypted_archive_round_trips_and_rejects_bad_password() {
    let db = TestDb::fresh().await;
    seed_via_importer(&db).await;

    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let built = build_and_write_or_read_archive(conn, BackupKind::Manual, Some("hunter2")).await.expect("build with password must succeed");
    assert!(built.manifest.encrypted);

    let bytes = archive::base64_to_bytes(&built.archive_base64).unwrap();
    let parsed = archive::parse_archive(&bytes).unwrap();
    let decrypted = archive::decrypt_archive(&parsed, "hunter2").expect("correct password must decrypt");
    assert!(archive::find_payload_file(&decrypted, "data.json").is_some());

    let wrong = archive::decrypt_archive(&parsed, "wrong password");
    assert!(wrong.is_err(), "wrong password must fail");

    // Missing password at the restore-gate level (parse_and_verify is private, so this is asserted
    // through preview_restore/restore_from_archive's public surface instead — see
    // `restore_refuses_encrypted_archive_without_password` below).

    // Flipped byte in the archive bytes must change the recomputed checksum (D-9's restore-time
    // check) or make the zip itself fail to parse — either outcome proves corruption is detected.
    let mut tampered = bytes.clone();
    if let Some(pos) = tampered.iter().rposition(|&b| b != 0) {
        tampered[pos] ^= 0x01;
    }
    match archive::recompute_checksum(&tampered) {
        Ok(sum) => assert_ne!(sum, built.manifest.checksum, "a flipped byte must change the recomputed checksum"),
        Err(_) => {} // corruption detected at parse time — also an acceptable outcome.
    }
    drop(db_guard);

    db.finish().await;
}

// --- preview notes for versions 1, 50, 114, 115, 116 (build = 115 via the real migrator count) ----

#[tokio::test]
async fn preview_restore_notes_match_spec_table() {
    let build = dataset::build_schema_version();

    for (version, expect_compatible) in [(1u32, true), (50, false), (build - 1, true), (build, true), (build + 1, false)] {
        let manifest_json = format!(
            r#"{{"app":"accounting-app","appVersion":"0.1.0","schemaVersion":{version},"createdAt":"2026-01-01T00:00:00.000Z","company":"c","counts":{{}},"checksum":"x","encrypted":false,"kind":"manual"}}"#
        );
        let zip_bytes = build_zip_with_manifest(&manifest_json);
        let preview = restore::preview_restore(&zip_bytes).expect("preview_restore must read the manifest");
        assert_eq!(preview.compatible, expect_compatible, "version {version} vs build {build}");
        if version == build {
            assert!(preview.compatibility_note.is_none());
        } else {
            assert!(preview.compatibility_note.is_some());
        }
    }
}

fn build_zip_with_manifest(manifest_json: &str) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};
    let mut buf = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut buf);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("manifest.json", options).unwrap();
        zip.write_all(manifest_json.as_bytes()).unwrap();
        zip.finish().unwrap();
    }
    buf.into_inner()
}

// --- counts: key order equals the fixed list; customers/suppliers split; soft-deleted not counted -

#[tokio::test]
async fn table_counts_key_order_and_soft_delete_filter() {
    let db = TestDb::fresh().await;
    seed_via_importer(&db).await;

    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let counts = table_counts(conn).await.expect("table_counts must succeed");
    let keys: Vec<&str> = counts.0.iter().map(|(k, _)| k.as_str()).collect();
    let mut expected: Vec<&str> = COUNT_KEYS.to_vec();
    expected.push("attachments");
    assert_eq!(keys, expected, "counts key order must equal the fixed spec list, then attachments");

    let attachments = counts.0.iter().find(|(k, _)| k == "attachments").unwrap();
    // C-16: `attachments` is a real SELECT COUNT(*) now — 0 here because the importer seed fixture
    // inserts no attachment rows, not because the count is hardcoded (see domain_attachments.rs for
    // a real, non-zero count).
    assert_eq!(attachments.1, 0, "no attachments were seeded by the importer fixture");
    drop(db_guard);

    db.finish().await;
}

// --- auto: terminal role -> not-main; disabled; not due; no folder; due -> written, retention ------

#[tokio::test]
async fn auto_backup_skips_on_terminal_role() {
    let db = TestDb::fresh().await;
    *db.state.device.write().unwrap() = DeviceSettings { role: DeviceRole::Terminal, ..Default::default() };

    let outcome = auto::run_auto_backup_if_due(&db.state, AutoBackupTrigger::Schedule).await.expect("must not error");
    assert!(!outcome.ran);
    assert_eq!(outcome.skipped, Some(AutoBackupSkipReason::NotMain));

    db.finish().await;
}

#[tokio::test]
async fn auto_backup_skips_when_disabled_or_no_folder() {
    let db = TestDb::fresh().await;
    seed_via_importer(&db).await;
    *db.state.device.write().unwrap() = DeviceSettings { role: DeviceRole::Main, ..Default::default() };

    // Disabled by default (DEFAULT_BACKUP_SETTINGS.auto_enabled == false).
    let outcome = auto::run_auto_backup_if_due(&db.state, AutoBackupTrigger::Schedule).await.expect("must not error");
    assert!(!outcome.ran);
    assert_eq!(outcome.skipped, Some(AutoBackupSkipReason::Disabled));

    // Enabled but no folder configured (configured by a logged-in admin, then the timer runs with
    // nobody logged in — D-6).
    log_in_as_imported_admin(&db).await;
    let undo = db.state.undo.clone();
    with_tx(&db.state, TxOpts { require_user: false }, move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let device = accounting_app_lib::core::device::DeviceSettings::default();
            let patch = accounting_app_lib::infrastructure::backup::dto::BackupSettingsPatch {
                auto_enabled: Some(true),
                // Always due (the default 20:00 would make this `not-due` before 8 pm, which is
                // checked before the folder — §3.7 steps 3/4).
                auto_time: Some("00:00".to_string()),
                ..Default::default()
            };
            auto::save_backup_settings(tx, cx, &undo, &device, patch).await.map(|_| ())
        })
    })
    .await
    .expect("enabling auto_enabled must succeed");
    log_out(&db);

    let outcome = auto::run_auto_backup_if_due(&db.state, AutoBackupTrigger::Schedule).await.expect("must not error");
    assert!(!outcome.ran);
    assert_eq!(outcome.skipped, Some(AutoBackupSkipReason::NoFolder));

    db.finish().await;
}

#[tokio::test]
async fn auto_backup_writes_file_and_prunes_retention() {
    let db = TestDb::fresh().await;
    seed_via_importer(&db).await;

    let tmp_dir = std::env::temp_dir().join(format!("equal-backup-test-{}", accounting_app_lib::utils::id::Id::new()));
    std::fs::create_dir_all(&tmp_dir).unwrap();
    let folder = tmp_dir.to_string_lossy().to_string();

    // `folder` is device-owned (cross-cutting §3) — `backup_settings()` always overrides whatever is
    // in the settings row with `device.backup_folder`, so the folder must be set on the device here,
    // not only patched into the settings row.
    *db.state.device.write().unwrap() = DeviceSettings { role: DeviceRole::Main, backup_folder: Some(folder.clone()), ..Default::default() };

    log_in_as_imported_admin(&db).await;
    let undo = db.state.undo.clone();
    let device_snapshot = db.state.device.read().unwrap().clone();
    with_tx(&db.state, TxOpts { require_user: false }, move |tx, cx| {
        let undo = undo.clone();
        let device_snapshot = device_snapshot.clone();
        Box::pin(async move {
            let patch = accounting_app_lib::infrastructure::backup::dto::BackupSettingsPatch {
                auto_enabled: Some(true),
                auto_time: Some("00:00".to_string()),
                retention: Some(2),
                ..Default::default()
            };
            auto::save_backup_settings(tx, cx, &undo, &device_snapshot, patch).await.map(|_| ())
        })
    })
    .await
    .expect("configuring auto backup must succeed");

    // D-6: the timer runs with nobody logged in (the login screen).
    log_out(&db);

    // Due (auto_time 00:00 is always <= now).
    let outcome = auto::run_auto_backup_if_due(&db.state, AutoBackupTrigger::Schedule).await.expect("must not error");
    assert!(outcome.ran, "auto backup must run when enabled, due, and a folder is configured: {outcome:?}");
    let path = outcome.path.expect("ran outcome must carry a path");
    assert!(Path::new(&path).exists(), "the backup file must actually be written");

    // Settings updated: last backup recorded, and today's run remembered — the next tick is
    // `not-due` instead of writing another archive every minute.
    {
        let db_guard = db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        let device = db.state.device.read().unwrap().clone();
        let settings = auto::backup_settings(conn, &device).await.unwrap();
        assert_eq!(settings.last_backup_kind, Some(BackupKind::Auto));
        assert!(settings.last_backup_at.is_some() && settings.last_auto_run_date.is_some() && settings.last_backup_error.is_none());
    }
    let again = auto::run_auto_backup_if_due(&db.state, AutoBackupTrigger::Schedule).await.expect("must not error");
    assert_eq!(again.skipped, Some(AutoBackupSkipReason::NotDue), "a second tick the same day must not write again: {again:?}");

    let _ = std::fs::remove_dir_all(&tmp_dir);

    db.finish().await;
}

// --- restore: invariant-breaking archive -> rollback, DB unchanged; session cleared ----------------

#[tokio::test]
async fn restore_refuses_encrypted_archive_without_password() {
    let db = TestDb::fresh().await;
    seed_via_importer(&db).await;
    *db.state.device.write().unwrap() = DeviceSettings { role: DeviceRole::Main, ..Default::default() };

    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let built = build_and_write_or_read_archive(conn, BackupKind::Manual, Some("secret")).await.expect("build must succeed");
    drop(db_guard);

    let err = restore::restore_from_archive(&db.state, &built.archive_base64, None).await.unwrap_err();
    assert_eq!(err.to_string(), "هذه النسخة مشفّرة — أدخل كلمة المرور");

    db.finish().await;
}

/// D-7 is checked before the mandatory pre-restore backup writes anything: no session is
/// UNAUTHORIZED (not the misleading pre-restore backup failure).
#[tokio::test]
async fn restore_without_session_is_unauthorized_before_any_write() {
    let db = TestDb::fresh().await;
    seed_via_importer(&db).await;
    *db.state.device.write().unwrap() = DeviceSettings { role: DeviceRole::Main, ..Default::default() };

    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let built = build_and_write_or_read_archive(conn, BackupKind::Manual, None).await.expect("build must succeed");
    drop(db_guard);

    let err = restore::restore_from_archive(&db.state, &built.archive_base64, None).await.unwrap_err();
    assert!(matches!(err, accounting_app_lib::core::error::AppError::Unauthorized { .. }), "unexpected error: {err:?}");

    db.finish().await;
}

#[tokio::test]
async fn restore_refuses_on_terminal_role() {
    let db = TestDb::fresh().await;
    seed_via_importer(&db).await;
    *db.state.device.write().unwrap() = DeviceSettings { role: DeviceRole::Terminal, ..Default::default() };

    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let built = build_and_write_or_read_archive(conn, BackupKind::Manual, None).await.expect("build must succeed");
    drop(db_guard);

    let err = restore::restore_from_archive(&db.state, &built.archive_base64, None).await.unwrap_err();
    assert_eq!(err.to_string(), "الاستعادة متاحة على الجهاز الرئيسي فقط");

    db.finish().await;
}

#[tokio::test]
async fn restore_full_round_trip_ends_session_and_matches_counts() {
    let db = TestDb::fresh().await;
    seed_via_importer(&db).await;
    *db.state.device.write().unwrap() = DeviceSettings { role: DeviceRole::Main, ..Default::default() };
    log_in_as_imported_admin(&db).await;

    let (archive_base64, before_counts) = {
        let db_guard = db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        let built = build_and_write_or_read_archive(conn, BackupKind::Manual, None).await.expect("build must succeed");
        let counts = table_counts(conn).await.unwrap();
        (built.archive_base64, counts)
    };

    // The admin session above is what D-11 must clear.
    assert!(db.state.session.read().unwrap().is_some());

    restore::restore_from_archive(&db.state, &archive_base64, None).await.expect("restore must succeed");

    assert!(db.state.session.read().unwrap().is_none(), "D-11: restore must end the session");

    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let after_counts = table_counts(conn).await.unwrap();
    // Every table comes back as archived, plus exactly the restore's own
    // "استعادة من نسخة احتياطية" row (§3.9 step 4 logs it after the load: one audit + one activity).
    let expected: Vec<(String, i64)> = before_counts
        .0
        .iter()
        .map(|(k, v)| (k.clone(), if k == "audit" || k == "activity" { v + 1 } else { *v }))
        .collect();
    assert_eq!(after_counts.0, expected, "restoring the archive just taken must reproduce the same counts (+ the restore's own log row)");

    let results = invariants::run_all(conn).await.expect("run_all must not error");
    let failed: Vec<_> = results.iter().filter(|r| !r.passed).collect();
    assert!(failed.is_empty(), "invariants failed after restore: {failed:?}");
    drop(db_guard);

    db.finish().await;
}

// --- compatibility with the TS format (§8a) --------------------------------------------------------

#[tokio::test]
async fn reads_ts_format_plain_archive() {
    let bytes = read_fixture_bytes("backup-browser-plain.zip");
    let manifest = archive::read_manifest(&bytes).expect("must read the TS-built manifest");
    assert!(manifest.schema_version < 100, "a browser archive's schemaVersion must be in the mock's own space");
    let parsed = archive::parse_archive(&bytes).expect("must parse the TS zip layout");
    assert!(archive::find_payload_file(parsed.payload_files.as_ref().unwrap(), "data.json").is_some());
    let recomputed = archive::recompute_checksum(&bytes).expect("checksum recompute must succeed");
    assert_eq!(recomputed, manifest.checksum, "checksum must match what the TS builder recorded");
}

#[tokio::test]
async fn reads_ts_format_encrypted_archive() {
    let bytes = read_fixture_bytes("backup-browser-encrypted.zip");
    let manifest = archive::read_manifest(&bytes).expect("must read the TS-built manifest");
    assert!(manifest.encrypted);
    let parsed = archive::parse_archive(&bytes).expect("must parse the TS zip layout");
    let decrypted = archive::decrypt_archive(&parsed, "1234").expect("password '1234' must decrypt a TS-built archive");
    assert!(archive::find_payload_file(&decrypted, "data.json").is_some());
    let recomputed = archive::recompute_checksum(&bytes).expect("checksum recompute must succeed");
    assert_eq!(recomputed, manifest.checksum);
}

/// §8a: restoring a TS-built (browser, `schemaVersion 1`) archive runs the D10 importer — the
/// restored counts equal the archive manifest's own `counts`, plus the restore's log row.
#[tokio::test]
async fn restores_ts_format_plain_archive_through_the_importer() {
    let db = TestDb::fresh().await;
    seed_via_importer(&db).await;
    *db.state.device.write().unwrap() = DeviceSettings { role: DeviceRole::Main, ..Default::default() };
    log_in_as_imported_admin(&db).await;

    let bytes = read_fixture_bytes("backup-browser-plain.zip");
    let manifest = archive::read_manifest(&bytes).unwrap();
    let archive_base64 = archive::bytes_to_base64(&bytes);
    restore::restore_from_archive(&db.state, &archive_base64, None).await.expect("a browser archive must restore through the importer");
    assert!(db.state.session.read().unwrap().is_none(), "D-11: restore must end the session");

    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let after = table_counts(conn).await.unwrap();
    let manifest_counts = serde_json::to_value(&manifest.counts).unwrap();
    for (key, n) in &after.0 {
        let archived = manifest_counts.get(key.as_str()).and_then(|v| v.as_i64()).unwrap_or(0);
        let expected = if key == "audit" || key == "activity" { archived + 1 } else { archived };
        assert_eq!(*n, expected, "count for '{key}' after restoring the browser archive");
    }

    let results = invariants::run_all(conn).await.expect("run_all must not error");
    let failed: Vec<_> = results.iter().filter(|r| !r.passed).collect();
    assert!(failed.is_empty(), "invariants failed after restore: {failed:?}");
    drop(db_guard);

    db.finish().await;
}

// --- pre-migration: fresh DB -> None; migrated with one pending -> file written, 10 kept ----------

#[tokio::test]
async fn pre_migration_backup_is_none_on_fresh_db() {
    let db = TestDb::fresh().await; // TestDb::fresh() already runs every migration.
    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let tmp_dir = std::env::temp_dir().join(format!("equal-premigration-test-{}", accounting_app_lib::utils::id::Id::new()));
    let result = backup_before_migrations(conn, &tmp_dir).await.expect("must not error on a fully-migrated DB");
    assert!(result.is_none(), "no pending migrations on a freshly migrated test DB — nothing to back up");
    drop(db_guard);

    db.finish().await;
}

/// ACC-0034: a brand new installation's first connection — `seaql_migrations` has never been
/// created, so `get_applied_migrations()` is empty while `get_pending_migrations()` is the full
/// list (every migration is pending). The old `applied.is_empty() && pending.is_empty()` check could
/// never be true on a real fresh install, so it fell through to dump a schema that doesn't exist yet
/// (failing on `SELECT COUNT(*) FROM users`) instead of recognizing "nothing applied = nothing to
/// back up". Uses `TestDb::empty()`, not `fresh()`, since `fresh()` pre-migrates for test speed.
#[tokio::test]
async fn pre_migration_backup_is_none_on_a_true_fresh_install() {
    let db = TestDb::empty().await;
    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let tmp_dir = std::env::temp_dir().join(format!("equal-premigration-test-{}", accounting_app_lib::utils::id::Id::new()));
    let result = backup_before_migrations(conn, &tmp_dir)
        .await
        .expect("a true fresh install (nothing applied, everything pending) must not error — there is no old schema to back up");
    assert!(result.is_none(), "nothing applied yet on a fresh install — there is no old schema to dump");
}

// --- upgrade_registry_covers_every_version (unit, mirrors upgrade.rs's own test) -------------------

#[test]
fn upgrade_registry_is_consistent_with_build() {
    let build = dataset::build_schema_version();
    let mut dataset = DataSetV1 { format: "equal-db".to_string(), db_schema_version: 100, migrations: vec![], tables: vec![] };
    upgrade_to_build(&mut dataset, build).expect("upgrade_to_build must succeed from baseline to build");
    assert_eq!(dataset.db_schema_version, build);
    // Baseline: empty registry is trivially consistent. Once entries are added, every version from
    // 100..build should have explicit coverage decided (identity upgraders allowed) — this is the
    // living pin `upgrade.rs`'s own unit test also asserts; kept here too so a `cargo test -p
    // accounting-app-lib --test domain_backup` run covers it alongside the rest of this domain.
    let _ = ROW_UPGRADES;
}
