//! Forced-deadlock retry test (phase-a-foundation.md "Tests": "a forced deadlock (two connections,
//! opposite lock order on a test table) retries and succeeds"). Needs `EQUAL_TEST_DATABASE_URL`.

use crate::support;

use accounting_app_lib::core::error::AppError;
use accounting_app_lib::core::events::ChangeCategory;
use accounting_app_lib::core::tx::{with_tx, BoxFuture, TxError, TxOpts};
use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::utils::id::Id;
use sea_orm::{ConnectionTrait, Database, Statement};
use support::TestDb;

#[tokio::test]
async fn forced_deadlock_retries_and_succeeds() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    conn.execute(Statement::from_string(
        conn.get_database_backend(),
        "CREATE TABLE deadlock_test (id INT PRIMARY KEY, v INT NOT NULL)".to_string(),
    ))
    .await
    .unwrap();
    conn.execute(Statement::from_string(
        conn.get_database_backend(),
        "INSERT INTO deadlock_test (id, v) VALUES (1, 0), (2, 0)".to_string(),
    ))
    .await
    .unwrap();

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

    // A second, independent connection to the same throwaway test database, deliberately taking
    // locks in the opposite order (id=2 then id=1) while `with_tx`'s closure (below) takes id=1
    // then id=2 — the classic two-connection, opposite-lock-order deadlock setup.
    //
    // Which side InnoDB rolls back is not random: it picks the *lighter* transaction (fewer rows
    // changed/locked). So the second connection first inserts a batch of extra rows to make itself
    // the heavier one, and `with_tx`'s closure is the one to close the cycle (it asks for id=2
    // last) — `with_tx` is then always the deadlock victim, and its retry is what's under test.
    let second = Database::connect(test_db.db_url.clone()).await.unwrap();

    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let barrier2 = barrier.clone();

    let opposite_order_task = tokio::spawn(async move {
        use sea_orm::TransactionTrait;
        let txn = second.begin().await.unwrap();
        txn.execute(Statement::from_string(txn.get_database_backend(), "UPDATE deadlock_test SET v = v + 1 WHERE id = 2".to_string()))
            .await
            .unwrap();
        let ballast: Vec<String> = (100..150).map(|i| format!("({i}, 0)")).collect();
        txn.execute(Statement::from_string(txn.get_database_backend(), format!("INSERT INTO deadlock_test (id, v) VALUES {}", ballast.join(", "))))
            .await
            .unwrap();
        barrier2.wait().await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        txn.execute(Statement::from_string(txn.get_database_backend(), "UPDATE deadlock_test SET v = v + 1 WHERE id = 1".to_string()))
            .await
            .expect("the heavier transaction must survive the deadlock");
        txn.commit().await.expect("the heavier transaction must commit");
    });

    let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let attempts_for_closure = attempts.clone();
    let barrier_for_closure = barrier.clone();

    let result: Result<(), AppError> = with_tx(&test_db.state, TxOpts::default(), move |txn, ctx| {
        let attempts_for_closure = attempts_for_closure.clone();
        let barrier_for_closure = barrier_for_closure.clone();
        Box::pin(async move {
            let attempt = attempts_for_closure.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            ctx.touch(ChangeCategory::Ledger);
            txn.execute(Statement::from_string(txn.get_database_backend(), "UPDATE deadlock_test SET v = v + 1 WHERE id = 1".to_string()))
                .await
                .map_err(TxError::from)?;
            if attempt == 0 {
                // Only the first attempt meets the other connection; the retry runs alone.
                barrier_for_closure.wait().await;
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            }
            txn.execute(Statement::from_string(txn.get_database_backend(), "UPDATE deadlock_test SET v = v + 1 WHERE id = 2".to_string()))
                .await
                .map_err(TxError::from)?;
            Ok(())
        }) as BoxFuture<'_, Result<(), TxError>>
    })
    .await;

    opposite_order_task.await.expect("the opposite-order task must not panic");

    assert!(result.is_ok(), "with_tx should recover from a deadlock via retry: {result:?}");
    assert!(attempts.load(std::sync::atomic::Ordering::SeqCst) >= 2, "expected at least one retry after the forced deadlock");

    // Both transactions' increments landed exactly once: id=1 and id=2 each +1 from the other
    // connection and +1 from the (retried) `with_tx` closure.
    let rows = conn
        .query_all(Statement::from_string(conn.get_database_backend(), "SELECT v FROM deadlock_test WHERE id IN (1, 2) ORDER BY id".to_string()))
        .await
        .unwrap();
    let values: Vec<i32> = rows.iter().map(|r| r.try_get_by_index::<i32>(0).unwrap()).collect();
    assert_eq!(values, vec![2, 2]);
}
