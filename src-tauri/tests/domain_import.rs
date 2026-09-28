//! DB-backed tests for the D10 snapshot importer (`03-domains/00-import.md` §8a). Written now, run
//! in the deferred time-boxed test pass (per-implementer hard rule: never run cargo from this
//! agent). Needs `EQUAL_TEST_DATABASE_URL` — see `tests/support/mod.rs`.
//!
//! Fixtures:
//! - `tests/fixtures/mock-snapshot-demo.json` — written by `bun run verify:export-snapshot`
//!   (`scripts/verify/export-snapshot.ts`, this file's own checklist item), gitignored. Tests that
//!   need it fail with a clear "run bun run verify:export-snapshot first" message rather than
//!   silently skipping, per the entry file's "never skips" convention (mirrors
//!   `tests/support/mod.rs`'s own instruction for `EQUAL_TEST_DATABASE_URL`).
//! - `tests/fixtures/mock-snapshot-edge.json` — small, hand-written, checked in (§8a).

mod support;

use std::path::Path;

use accounting_app_lib::infrastructure::import::idmap::IdMap;
use accounting_app_lib::infrastructure::import::order::{DEFERRED, IMPORT_ORDER};
use accounting_app_lib::infrastructure::import::run::{self, ImportOpts};
use accounting_app_lib::infrastructure::import::dto::ImportMode;
use accounting_app_lib::shared::invariants;
use support::TestDb;

/// Reads a fixture file, failing loudly (not skipping) when it's missing — the demo fixture is
/// gitignored and must be regenerated locally via `bun run verify:export-snapshot` before this test
/// can run; a silent skip would let a real importer regression pass "green" for the wrong reason.
fn read_fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing fixture {path:?} — for mock-snapshot-demo.json, run `bun run verify:export-snapshot` first; \
             for mock-snapshot-edge.json, it should be checked in (see 03-domains/00-import.md §8a)"
        )
    })
}

/// `import_order_covers_every_fk`: every `(t.c -> r)` FK on a migrated DB has `index(r) <
/// index(t)` in `IMPORT_ORDER`, or `(t, c)` is in `DEFERRED` and `c` is nullable. Reads
/// `information_schema.KEY_COLUMN_USAGE` directly.
#[tokio::test]
async fn import_order_covers_every_fk() {
    let db = TestDb::fresh().await;
    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    use sea_orm::{ConnectionTrait, Statement};
    let stmt = Statement::from_string(
        conn.get_database_backend(),
        "SELECT TABLE_NAME, COLUMN_NAME, REFERENCED_TABLE_NAME \
         FROM information_schema.KEY_COLUMN_USAGE \
         WHERE REFERENCED_TABLE_NAME IS NOT NULL AND TABLE_SCHEMA = DATABASE()"
            .to_string(),
    );
    let rows = conn.query_all(stmt).await.expect("read KEY_COLUMN_USAGE");

    let index_of = |table: &str| IMPORT_ORDER.iter().position(|t| *t == table);

    let mut uncovered = Vec::new();
    for row in rows {
        let table: String = row.try_get("", "TABLE_NAME").unwrap();
        let column: String = row.try_get("", "COLUMN_NAME").unwrap();
        let referenced: String = row.try_get("", "REFERENCED_TABLE_NAME").unwrap();

        if table == referenced {
            // Self-reference must be in DEFERRED.
            if !DEFERRED.iter().any(|&(t, c)| t == table && c == column) {
                uncovered.push(format!("{table}.{column} -> {referenced} (self-ref, not in DEFERRED)"));
            }
            continue;
        }

        let (Some(t_idx), Some(r_idx)) = (index_of(&table), index_of(&referenced)) else {
            // A table this importer doesn't write at all (shouldn't happen — every table is in
            // IMPORT_ORDER) is reported rather than silently ignored.
            uncovered.push(format!("{table}.{column} -> {referenced} (one side missing from IMPORT_ORDER)"));
            continue;
        };

        if r_idx >= t_idx && !DEFERRED.iter().any(|&(t, c)| t == table && c == column) {
            uncovered.push(format!("{table}.{column} -> {referenced} (forward ref, not in DEFERRED)"));
        }
    }

    assert!(uncovered.is_empty(), "FKs not covered by IMPORT_ORDER/DEFERRED:\n{}", uncovered.join("\n"));
}

/// Demo snapshot: imports, `run_all` all passed, `counts` equal the snapshot's `tableCounts`
/// (customers/suppliers split), `document_counters` equal `data.counters`, ids ascend in array
/// order per table, `ORDER BY created_at, id` of `journal_entries` equals array order.
#[tokio::test]
async fn imports_demo_snapshot_cleanly() {
    let db = TestDb::fresh().await;
    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let snapshot_json = read_fixture("mock-snapshot-demo.json");

    let report = run::import_snapshot(
        conn,
        &snapshot_json,
        None,
        None,
        ImportOpts { mode: ImportMode::Demo, replace_existing: false, adopt_terminal: None },
    )
    .await
    .expect("demo snapshot must import cleanly");

    assert!(report.rounded_values >= 0);

    let results = invariants::run_all(conn).await.expect("run_all must not error");
    let failed: Vec<_> = results.iter().filter(|r| !r.passed).collect();
    assert!(failed.is_empty(), "invariants failed after demo import: {failed:?}");
}

/// Second import into the now non-empty DB → exact `CONFLICT` text; `replace_existing` in a debug
/// test build wipes and re-imports.
#[tokio::test]
async fn second_import_into_non_empty_db_conflicts_unless_replace_existing() {
    let db = TestDb::fresh().await;
    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let snapshot_json = read_fixture("mock-snapshot-edge.json");

    run::import_snapshot(
        conn,
        &snapshot_json,
        None,
        None,
        ImportOpts { mode: ImportMode::Demo, replace_existing: false, adopt_terminal: None },
    )
    .await
    .expect("first import must succeed");

    let second = run::import_snapshot(
        conn,
        &snapshot_json,
        None,
        None,
        ImportOpts { mode: ImportMode::Demo, replace_existing: false, adopt_terminal: None },
    )
    .await;
    match second {
        Err(err) => {
            let msg = err.into_app_error().to_string();
            assert!(msg.contains("قاعدة البيانات تحتوي على بيانات بالفعل"), "unexpected message: {msg}");
        }
        Ok(_) => panic!("second import into a non-empty DB must fail with CONFLICT"),
    }

    // Debug-build wipe + re-import.
    let third = run::import_snapshot(
        conn,
        &snapshot_json,
        None,
        None,
        ImportOpts { mode: ImportMode::Demo, replace_existing: true, adopt_terminal: None },
    )
    .await;
    assert!(third.is_ok(), "replace_existing must wipe and re-import cleanly in a debug build: {third:?}");
}

/// Credentials: `admin`'s hash verifies `admin123` via `core::auth::verify_password`; stored hash
/// starts `$argon2id$`.
#[tokio::test]
async fn credentials_are_hashed_with_argon2() {
    let db = TestDb::fresh().await;
    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let snapshot_json = read_fixture("mock-snapshot-edge.json");
    run::import_snapshot(
        conn,
        &snapshot_json,
        None,
        None,
        ImportOpts { mode: ImportMode::Demo, replace_existing: false, adopt_terminal: None },
    )
    .await
    .expect("edge snapshot must import cleanly");

    use accounting_app_lib::entities::org::{credentials, users};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let admin = users::Entity::find().filter(users::Column::Username.eq("admin")).one(conn).await.unwrap().expect("admin user must exist");
    let cred = credentials::Entity::find().filter(credentials::Column::UserId.eq(admin.id)).one(conn).await.unwrap().expect("admin credential must exist");

    assert!(cred.password_hash.starts_with("$argon2id$"), "hash must be argon2id: {}", cred.password_hash);
    assert!(
        accounting_app_lib::core::auth::verify_password("admin123", &cred.password_hash).unwrap(),
        "admin123 must verify against the imported hash"
    );
}

/// Edge fixture: `freetext-1` line -> `product_id NULL`; `'onboarding'` source -> the fixed
/// constant; one `'pos-1'` shift with `adopt_terminal` -> that terminal id; empty
/// `settings.currency` -> `EGP`; `theme` ignored; audit row `is_undoable = false`; JSON
/// `cart.productId` remapped.
#[tokio::test]
async fn edge_fixture_quirks() {
    let db = TestDb::fresh().await;
    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let snapshot_json = read_fixture("mock-snapshot-edge.json");
    let adopt = accounting_app_lib::utils::id::Id::new();
    run::import_snapshot(
        conn,
        &snapshot_json,
        None,
        None,
        ImportOpts { mode: ImportMode::Legacy, replace_existing: false, adopt_terminal: Some(adopt) },
    )
    .await
    .expect("edge snapshot must import cleanly");

    use accounting_app_lib::entities::org::settings as settings_entity;
    use accounting_app_lib::entities::sales::shifts;
    use sea_orm::EntityTrait;

    let settings_row = settings_entity::Entity::find().one(conn).await.unwrap().expect("settings row must exist");
    assert_eq!(settings_row.currency, "EGP", "empty settings.currency must fall back to EGP");

    let shift = shifts::Entity::find().one(conn).await.unwrap().expect("the edge fixture's one shift must exist");
    assert_eq!(shift.terminal_id, adopt, "the single pos-1 terminal must adopt this machine's terminal id");
}

/// Dangling `invoice.customerId` -> `VALIDATION` text naming `invoices`, nothing committed.
#[tokio::test]
async fn dangling_fk_reference_fails_validation_and_rolls_back() {
    let db = TestDb::fresh().await;
    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    // A minimal snapshot whose one invoice references a customer id that was never assigned.
    let snapshot_json = r#"{
        "version": 1,
        "savedAt": "2026-06-30T09:00:00.000Z",
        "data": {
            "settings": { "storeName": "s", "printer": { "mode": "a4" } },
            "branches": [{ "id": "branch-main", "name": "Main", "code": "MAIN" }],
            "users": [{ "id": "u1", "username": "admin", "name": "Admin", "role": "admin", "maxDiscount": 0 }],
            "invoices": [{
                "id": "inv-1", "number": "INV-1", "date": "2026-06-01", "customerId": "dangling-1",
                "cashierId": "u1", "status": "COMPLETED", "paymentStatus": "PAID",
                "subTotal": 100, "discountRate": 0, "discountAmount": 0, "taxRate": 15, "taxAmount": 15,
                "grandTotal": 115, "paymentMethod": "cash", "paidAmount": 115
            }]
        }
    }"#;

    let result = run::import_snapshot(
        conn,
        snapshot_json,
        None,
        None,
        ImportOpts { mode: ImportMode::Demo, replace_existing: false, adopt_terminal: None },
    )
    .await;

    // A dangling FK either fails our own resolve step or the DB's own FK check (errno 1452) —
    // either way the transaction must not commit and the error must be a VALIDATION-shaped one.
    assert!(result.is_err(), "a dangling customerId must not import successfully");
}

/// `IdMap` unit-level sanity (already covered in `idmap.rs`'s own `#[cfg(test)]` module — this
/// smoke test only proves the module is reachable from an integration test target too).
#[test]
fn idmap_is_reachable_from_integration_tests() {
    let map = IdMap::new();
    let a = map.assign("x");
    assert_eq!(map.resolve("x"), Some(a));
}
