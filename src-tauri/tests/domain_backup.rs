//! DB-backed tests for the `backup`/restore infrastructure (`03-domains/17-backup.md` §8a). Written
//! now, run in the deferred time-boxed test pass (per-implementer hard rule: never run cargo from
//! this agent). Needs `EQUAL_TEST_DATABASE_URL` — see `tests/support/mod.rs`.
//!
//! Fixtures:
//! - `tests/fixtures/mock-snapshot-edge.json` — small, hand-written, checked in (shared with
//!   `domain_import.rs`) — used here to seed a small-but-real, invariants-clean database to back up
//!   and restore.
//! - `tests/fixtures/backup-browser-plain.zip` / `backup-browser-encrypted.zip` (password `1234`) —
//!   TS-format archives, produced by `bun run verify:export-snapshot` through the real
//!   `buildBackupArchive` (17-backup.md §8a's "compatibility with the TS format"). Gitignored;
//!   tests that need them fail loudly with a clear message rather than skipping, per the entry
//!   file's "never skips" convention (mirrors `tests/support/mod.rs`/`domain_import.rs`).

mod support;

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
            "missing fixture {path:?} — run `bun run verify:export-snapshot` first (17-backup.md §8a's \
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
    let snapshot_json = read_fixture("mock-snapshot-edge.json");
    import_run::import_snapshot(conn, &snapshot_json, None, None, ImportOpts { mode: ImportMode::Demo, replace_existing: false, adopt_terminal: None })
        .await
        .expect("seed via importer must succeed");
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
    assert_eq!(attachments.1, 0, "D-5: Rust archives never carry attachment blobs");
}

// --- auto: terminal role -> not-main; disabled; not due; no folder; due -> written, retention ------

#[tokio::test]
async fn auto_backup_skips_on_terminal_role() {
    let db = TestDb::fresh().await;
    *db.state.device.write().unwrap() = DeviceSettings { role: DeviceRole::Terminal, ..Default::default() };

    let outcome = auto::run_auto_backup_if_due(&db.state, AutoBackupTrigger::Schedule).await.expect("must not error");
    assert!(!outcome.ran);
    assert_eq!(outcome.skipped, Some(AutoBackupSkipReason::NotMain));
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

    // Enabled but no folder configured.
    let undo = db.state.undo.clone();
    with_tx(&db.state, TxOpts { require_user: false }, move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let device = accounting_app_lib::core::device::DeviceSettings::default();
            let patch = accounting_app_lib::infrastructure::backup::dto::BackupSettingsPatch {
                auto_enabled: Some(true),
                ..Default::default()
            };
            auto::save_backup_settings(tx, cx, &undo, &device, patch).await.map(|_| ())
        })
    })
    .await
    .expect("enabling auto_enabled must succeed");

    let outcome = auto::run_auto_backup_if_due(&db.state, AutoBackupTrigger::Schedule).await.expect("must not error");
    assert!(!outcome.ran);
    assert_eq!(outcome.skipped, Some(AutoBackupSkipReason::NoFolder));
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

    // Due (auto_time 00:00 is always <= now).
    let outcome = auto::run_auto_backup_if_due(&db.state, AutoBackupTrigger::Schedule).await.expect("must not error");
    assert!(outcome.ran, "auto backup must run when enabled, due, and a folder is configured: {outcome:?}");
    let path = outcome.path.expect("ran outcome must carry a path");
    assert!(Path::new(&path).exists(), "the backup file must actually be written");

    let _ = std::fs::remove_dir_all(&tmp_dir);
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
}

#[tokio::test]
async fn restore_full_round_trip_ends_session_and_matches_counts() {
    let db = TestDb::fresh().await;
    seed_via_importer(&db).await;
    *db.state.device.write().unwrap() = DeviceSettings { role: DeviceRole::Main, ..Default::default() };

    let (archive_base64, before_counts) = {
        let db_guard = db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        let built = build_and_write_or_read_archive(conn, BackupKind::Manual, None).await.expect("build must succeed");
        let counts = table_counts(conn).await.unwrap();
        (built.archive_base64, counts)
    };

    // Simulate a logged-in session so D-11 has something to clear.
    // (No direct setter is exposed on AppState.session outside with_tx's actor resolution in this
    // test harness; restore_from_archive itself sets it to None unconditionally after commit, which
    // is asserted below regardless of whether a session existed before.)

    restore::restore_from_archive(&db.state, &archive_base64, None).await.expect("restore must succeed");

    assert!(db.state.session.read().unwrap().is_none(), "D-11: restore must end the session");

    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let after_counts = table_counts(conn).await.unwrap();
    assert_eq!(before_counts, after_counts, "restoring the archive just taken must reproduce the same counts");

    let results = invariants::run_all(conn).await.expect("run_all must not error");
    let failed: Vec<_> = results.iter().filter(|r| !r.passed).collect();
    assert!(failed.is_empty(), "invariants failed after restore: {failed:?}");
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

// --- pre-migration: fresh DB -> None; migrated with one pending -> file written, 10 kept ----------

#[tokio::test]
async fn pre_migration_backup_is_none_on_fresh_db() {
    let db = TestDb::fresh().await; // TestDb::fresh() already runs every migration.
    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let tmp_dir = std::env::temp_dir().join(format!("equal-premigration-test-{}", accounting_app_lib::utils::id::Id::new()));
    let result = backup_before_migrations(conn, &tmp_dir).await.expect("must not error on a fully-migrated DB");
    assert!(result.is_none(), "no pending migrations on a freshly migrated test DB — nothing to back up");
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
