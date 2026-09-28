//! Phase A DB-backed tests (phase-a-foundation.md "Tests"): `Id` DB round-trip/order, error errno
//! mapping against a real DB, and `with_tx` transaction behavior. Needs `EQUAL_TEST_DATABASE_URL`
//! (see `tests/support/mod.rs`) — never skipped when absent, panics with a clear message instead.

mod support;

use std::sync::Arc;

use accounting_app_lib::core::error::AppError;
use accounting_app_lib::core::events::{ChangeCategory, EventSink};
use accounting_app_lib::core::tx::{with_read, with_tx, BoxFuture, TxError, TxOpts};
use accounting_app_lib::utils::id::Id;
use sea_orm::{ConnectionTrait, Statement};
use support::TestDb;

#[tokio::test]
async fn id_round_trips_through_a_uuid_column_and_sorts_by_generation_order() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    conn.execute(Statement::from_string(
        conn.get_database_backend(),
        "CREATE TABLE id_order_test (id UUID PRIMARY KEY, seq INT NOT NULL)".to_string(),
    ))
    .await
    .unwrap();

    // Generate 1,000 ids in order, but insert them in a shuffled (reversed) order — proves the
    // native UUID column's ORDER BY reflects generation order (UUIDv7 time-ordering), not
    // insertion order.
    let mut generated: Vec<Id> = (0..1000).map(|_| Id::new()).collect();
    let mut insert_order = generated.clone();
    insert_order.reverse();

    for (seq, id) in insert_order.iter().enumerate() {
        let stmt = Statement::from_sql_and_values(
            conn.get_database_backend(),
            "INSERT INTO id_order_test (id, seq) VALUES (?, ?)",
            [id.to_string().into(), (seq as i64).into()],
        );
        conn.execute(stmt).await.unwrap();
    }

    let stmt = Statement::from_string(conn.get_database_backend(), "SELECT id FROM id_order_test ORDER BY id".to_string());
    let rows = conn.query_all(stmt).await.unwrap();
    let read_back: Vec<Id> = rows.iter().map(|r| r.try_get::<Id>("", "id").unwrap()).collect();

    generated.sort(); // Id's own Ord agrees with UUIDv7 generation order.
    assert_eq!(read_back, generated, "ORDER BY id on a native UUID column must match UUIDv7 generation order");
}

#[tokio::test]
async fn errno_mapping_against_a_real_db() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    conn.execute(Statement::from_string(
        conn.get_database_backend(),
        "CREATE TABLE uniq_test (id INT PRIMARY KEY, code VARCHAR(20) NOT NULL, UNIQUE KEY uq_uniq_test_code (code))".to_string(),
    ))
    .await
    .unwrap();
    conn.execute(Statement::from_string(conn.get_database_backend(), "INSERT INTO uniq_test (id, code) VALUES (1, 'A')".to_string()))
        .await
        .unwrap();

    let dup = conn
        .execute(Statement::from_string(conn.get_database_backend(), "INSERT INTO uniq_test (id, code) VALUES (2, 'A')".to_string()))
        .await;
    let err = dup.expect_err("duplicate key must fail");
    let app_err = AppError::from(err);
    assert!(matches!(app_err, AppError::Conflict { .. }));
    assert_eq!(app_err.to_string(), "هذا السجل موجود بالفعل");
}

#[tokio::test]
async fn with_tx_commit_persists_data() {
    let test_db = TestDb::fresh().await;
    setup_probe_table(&test_db).await;
    log_in(&test_db);

    let result: Result<(), AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        Box::pin(async move {
            ctx.touch(ChangeCategory::Ledger);
            let stmt = Statement::from_string(txn.get_database_backend(), "INSERT INTO probe (id) VALUES (1)".to_string());
            txn.execute(stmt).await.map_err(TxError::from)?;
            Ok(())
        }) as BoxFuture<'_, Result<(), TxError>>
    })
    .await;
    assert!(result.is_ok());

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let row = conn
        .query_one(Statement::from_string(conn.get_database_backend(), "SELECT COUNT(*) AS c FROM probe".to_string()))
        .await
        .unwrap()
        .unwrap();
    let count: i64 = row.try_get("", "c").unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn with_tx_error_rolls_back() {
    let test_db = TestDb::fresh().await;
    setup_probe_table(&test_db).await;
    log_in(&test_db);

    let result: Result<(), AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, ctx| {
        Box::pin(async move {
            ctx.touch(ChangeCategory::Ledger);
            let stmt = Statement::from_string(txn.get_database_backend(), "INSERT INTO probe (id) VALUES (1)".to_string());
            txn.execute(stmt).await.map_err(TxError::from)?;
            Err(TxError::from(AppError::validation("deliberate failure")))
        }) as BoxFuture<'_, Result<(), TxError>>
    })
    .await;
    assert!(result.is_err());

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let row = conn
        .query_one(Statement::from_string(conn.get_database_backend(), "SELECT COUNT(*) AS c FROM probe".to_string()))
        .await
        .unwrap()
        .unwrap();
    let count: i64 = row.try_get("", "c").unwrap();
    assert_eq!(count, 0, "a rolled-back transaction must leave no trace");
}

#[tokio::test]
async fn change_versions_bumped_only_on_commit_and_one_event_after_commit() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);

    let conn_before = version_of(&test_db, "ledger").await;

    // A failing transaction must not bump the version.
    let _: Result<(), AppError> = with_tx(&test_db.state, TxOpts::default(), |_txn, ctx| {
        Box::pin(async move {
            ctx.touch(ChangeCategory::Ledger);
            Err(TxError::from(AppError::validation("nope")))
        }) as BoxFuture<'_, Result<(), TxError>>
    })
    .await;
    assert_eq!(version_of(&test_db, "ledger").await, conn_before, "version must not bump on a rolled-back transaction");

    // A committing transaction must bump it exactly once and emit exactly one event.
    let result: Result<(), AppError> = with_tx(&test_db.state, TxOpts::default(), |_txn, ctx| {
        Box::pin(async move {
            ctx.touch(ChangeCategory::Ledger);
            Ok(())
        }) as BoxFuture<'_, Result<(), TxError>>
    })
    .await;
    assert!(result.is_ok());
    assert_eq!(version_of(&test_db, "ledger").await, conn_before + 1);

    let collector = test_db.state.events.clone();
    // Downcast via the trait object's concrete type isn't available generically; the collector
    // was constructed by TestDb::fresh() as a CollectingEventSink, so assert through the same
    // Arc<dyn EventSink> the state holds by re-emitting is not possible — instead check the
    // change_seen cursor AppState maintains, which is only raised on a real emit.
    let seen = test_db.state.change_seen.lock().unwrap();
    assert_eq!(seen.get(&ChangeCategory::Ledger).copied(), Some(1), "change_seen must be raised exactly once, only after commit");
    drop(seen);
    let _ = collector; // keep for readability of intent above
}

#[tokio::test]
async fn isolation_level_is_read_committed() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);

    let result: Result<String, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, _ctx| {
        Box::pin(async move {
            // A locking read makes sure this transaction has a live InnoDB trx row to inspect.
            txn.execute(Statement::from_string(txn.get_database_backend(), "SELECT category FROM change_versions LIMIT 1 FOR UPDATE".to_string()))
                .await
                .map_err(TxError::from)?;
            let row = txn
                // `@@transaction_isolation` is the *session* default; the level `with_tx` set applies
                // to this transaction only, so read it from the live InnoDB transaction instead.
                .query_one(Statement::from_string(
                    txn.get_database_backend(),
                    "SELECT REPLACE(trx_isolation_level, ' ', '-') AS lvl FROM information_schema.INNODB_TRX WHERE trx_mysql_thread_id = CONNECTION_ID()".to_string(),
                ))
                .await
                .map_err(TxError::from)?
                .unwrap();
            let lvl: String = row.try_get("", "lvl").map_err(TxError::from)?;
            Ok(lvl)
        }) as BoxFuture<'_, Result<String, TxError>>
    })
    .await;
    assert_eq!(result.unwrap(), "READ-COMMITTED");
}

#[tokio::test]
async fn with_read_refuses_a_write() {
    let test_db = TestDb::fresh().await;
    setup_probe_table(&test_db).await;

    let result: Result<(), AppError> = with_read(&test_db.state, |txn| {
        Box::pin(async move {
            let stmt = Statement::from_string(txn.get_database_backend(), "INSERT INTO probe (id) VALUES (99)".to_string());
            txn.execute(stmt).await.map_err(TxError::from)?;
            Ok(())
        }) as BoxFuture<'_, Result<(), TxError>>
    })
    .await;
    assert!(result.is_err(), "a write inside with_read's READ ONLY transaction must be refused by the server");
}

async fn setup_probe_table(test_db: &TestDb) {
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    conn.execute(Statement::from_string(conn.get_database_backend(), "CREATE TABLE probe (id INT PRIMARY KEY)".to_string()))
        .await
        .unwrap();
}

async fn version_of(test_db: &TestDb, category: &str) -> i64 {
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "SELECT version FROM change_versions WHERE category = ?",
        [category.into()],
    );
    let row = conn.query_one(stmt).await.unwrap().unwrap();
    row.try_get("", "version").unwrap()
}

/// `with_tx(require_user: true)` needs a logged-in session — stamps a minimal one directly into
/// `AppState` (Phase A has no real `users_login` command yet; that's Part 03).
fn log_in(test_db: &TestDb) {
    use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
    let user = AuthenticatedUser {
        id: Id::new(),
        username: "test".to_string(),
        role: Role::Admin,
        home_branch_id: Id::new(),
        allowed_branches: vec![],
        price_list_id: None,
        max_discount: None,
    };
    *test_db.state.session.write().unwrap() = Some(user);
}

#[allow(dead_code)]
fn assert_send<T: Send>() {}

#[allow(dead_code)]
fn keep_arc(_: Arc<dyn EventSink>) {}
