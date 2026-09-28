//! `shared::stock` DB-backed tests (phase-d-stock.md "Tests"). Needs `EQUAL_TEST_DATABASE_URL` —
//! never skipped, per `tests/support`.

mod support;

use std::sync::Arc;

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::events::CollectingEventSink;
use accounting_app_lib::core::tx::{with_tx, BoxFuture, TxOpts};
use accounting_app_lib::shared::stock::batches::{active_batches, consume_fefo, is_batch_expired, receive_batch};
use accounting_app_lib::shared::stock::cost::{cost_at_average, cost_out_at_price, cost_out_sale};
use accounting_app_lib::shared::stock::{apply_change, lock_product, lock_products, StockRef};
use accounting_app_lib::utils::dates::DocDate;
use accounting_app_lib::utils::id::Id;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use support::TestDb;

fn log_in(test_db: &TestDb) {
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

fn doc_date(day: &str) -> DocDate {
    DocDate { day: NaiveDate::parse_from_str(day, "%Y-%m-%d").unwrap(), instant: None }
}

async fn insert_product(conn: &DatabaseConnection, name: &str, track_batches: bool) -> Id {
    let id = Id::new();
    let sku = format!("SKU{}", &id.to_string().replace('-', "")[..8]);
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO products (id, name, sku, type, track_batches) VALUES (?, ?, ?, 'product', ?)",
        [id.to_string().into(), name.into(), sku.into(), track_batches.into()],
    );
    conn.execute(stmt).await.expect("product insert must succeed");
    id
}

async fn insert_service(conn: &DatabaseConnection) -> Id {
    let id = Id::new();
    let sku = format!("SKU{}", &id.to_string().replace('-', "")[..8]);
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO products (id, name, sku, type) VALUES (?, 'Service', ?, 'service')",
        [id.to_string().into(), sku.into()],
    );
    conn.execute(stmt).await.expect("service product insert must succeed");
    id
}

async fn stock_of(conn: &DatabaseConnection, product_id: Id) -> (Decimal, Decimal, Decimal) {
    use accounting_app_lib::entities::catalog::products;
    use sea_orm::EntityTrait;
    let p = products::Entity::find_by_id(product_id).one(conn).await.unwrap().expect("product must exist");
    (p.stock_qty, p.stock_value, p.cost_price)
}

#[tokio::test]
async fn apply_change_review_a1_worked_example() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);

    let product_id = {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        insert_product(conn, "Widget", false).await
    };

    // Opening stock: 9 @ 50 (value 450).
    with_tx(&test_db.state, TxOpts::default(), move |txn, cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>> {
        Box::pin(async move {
            let mut p = lock_product(txn, product_id).await?;
            apply_change(txn, cx, &mut p, dec!(9), dec!(450), "stock_in", StockRef { id: Id::new(), number: "ADJ-1".into() }, &doc_date("2026-01-01"), None)
                .await?;
            Ok(())
        })
    })
    .await
    .unwrap();

    {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        let (qty, value, cost) = stock_of(conn, product_id).await;
        assert_eq!(qty, dec!(9));
        assert_eq!(value, dec!(450));
        assert_eq!(cost, dec!(50.0000));
    }

    // Receipt: 10 @ 60 -> qty 19, value 1050, cost 55.2632.
    with_tx(&test_db.state, TxOpts::default(), move |txn, cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>> {
        Box::pin(async move {
            let mut p = lock_product(txn, product_id).await?;
            apply_change(txn, cx, &mut p, dec!(10), dec!(600), "purchase", StockRef { id: Id::new(), number: "PO-1".into() }, &doc_date("2026-01-02"), None)
                .await?;
            Ok(())
        })
    })
    .await
    .unwrap();

    let (qty, value, cost) = {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        stock_of(conn, product_id).await
    };
    assert_eq!(qty, dec!(19));
    assert_eq!(value, dec!(1050));
    assert_eq!(cost, dec!(55.2632));

    // Sell 1 -> value out = cost_out_sale(...) = 55.26.
    let value_out = cost_out_sale(qty, value, cost, dec!(1));
    assert_eq!(value_out, dec!(55.26));

    with_tx(&test_db.state, TxOpts::default(), move |txn, cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>> {
        Box::pin(async move {
            let mut p = lock_product(txn, product_id).await?;
            apply_change(txn, cx, &mut p, dec!(-1), -value_out, "sale", StockRef { id: Id::new(), number: "INV-1".into() }, &doc_date("2026-01-03"), None)
                .await?;
            Ok(())
        })
    })
    .await
    .unwrap();

    let (qty2, value2, _) = {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        stock_of(conn, product_id).await
    };
    assert_eq!(qty2, dec!(18));
    assert_eq!(value2, dec!(1050) - dec!(55.26));

    // A return at the original cost of 50 re-averages.
    with_tx(&test_db.state, TxOpts::default(), move |txn, cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>> {
        Box::pin(async move {
            let mut p = lock_product(txn, product_id).await?;
            apply_change(txn, cx, &mut p, dec!(1), dec!(50), "sale", StockRef { id: Id::new(), number: "RET-1".into() }, &doc_date("2026-01-04"), None)
                .await?;
            Ok(())
        })
    })
    .await
    .unwrap();

    let (qty3, value3, _) = {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        stock_of(conn, product_id).await
    };
    assert_eq!(qty3, dec!(19));
    assert_eq!(value3, value2 + dec!(50));
}

#[tokio::test]
async fn selling_the_whole_remaining_stock_takes_exactly_stock_value() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let product_id = {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        insert_product(conn, "Widget", false).await
    };

    with_tx(&test_db.state, TxOpts::default(), move |txn, cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>> {
        Box::pin(async move {
            let mut p = lock_product(txn, product_id).await?;
            apply_change(txn, cx, &mut p, dec!(5), dec!(275.37), "stock_in", StockRef { id: Id::new(), number: "ADJ-1".into() }, &doc_date("2026-01-01"), None)
                .await?;
            Ok(())
        })
    })
    .await
    .unwrap();

    let (qty, value, cost) = {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        stock_of(conn, product_id).await
    };
    let value_out = cost_out_sale(qty, value, cost, qty);
    assert_eq!(value_out, value, "selling every unit must take exactly the remaining stock_value");

    with_tx(&test_db.state, TxOpts::default(), move |txn, cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>> {
        Box::pin(async move {
            let mut p = lock_product(txn, product_id).await?;
            apply_change(txn, cx, &mut p, -qty, -value_out, "sale", StockRef { id: Id::new(), number: "INV-1".into() }, &doc_date("2026-01-02"), None)
                .await?;
            Ok(())
        })
    })
    .await
    .unwrap();

    let (qty2, value2, cost2) = {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        stock_of(conn, product_id).await
    };
    assert_eq!(qty2, Decimal::ZERO);
    assert_eq!(value2, Decimal::ZERO);
    assert_eq!(cost2, Decimal::ZERO);
}

#[tokio::test]
async fn purchase_return_variance_guard_books_the_shortfall() {
    // stock_qty=2, stock_value=100; returning 2 @ 75 would want to remove 150 -> variance guard.
    let (value_out, variance) = cost_out_at_price(dec!(2), dec!(100), dec!(2), dec!(75));
    assert_eq!(value_out, dec!(100));
    assert_eq!(variance, dec!(50));
}

#[tokio::test]
async fn branch_rows_always_sum_to_the_product_totals() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let product_id = {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        insert_product(conn, "Widget", false).await
    };

    let branch_a = Id::new();
    let branch_b = Id::new();

    with_tx(&test_db.state, TxOpts::default(), move |txn, cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>> {
        Box::pin(async move {
            let mut p = lock_product(txn, product_id).await?;
            apply_change(txn, cx, &mut p, dec!(4), dec!(200), "stock_in", StockRef { id: Id::new(), number: "A".into() }, &doc_date("2026-01-01"), Some(branch_a))
                .await?;
            apply_change(txn, cx, &mut p, dec!(6), dec!(300), "stock_in", StockRef { id: Id::new(), number: "B".into() }, &doc_date("2026-01-01"), Some(branch_b))
                .await?;
            Ok(())
        })
    })
    .await
    .unwrap();

    use accounting_app_lib::entities::catalog::product_branch_stock;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let branch_rows = product_branch_stock::Entity::find()
        .filter(product_branch_stock::Column::ProductId.eq(product_id))
        .all(conn)
        .await
        .unwrap();
    let branch_qty_sum: Decimal = branch_rows.iter().fold(Decimal::ZERO, |a, r| a + r.qty);
    let branch_value_sum: Decimal = branch_rows.iter().fold(Decimal::ZERO, |a, r| a + r.value);

    let (qty, value, _) = stock_of(conn, product_id).await;
    assert_eq!(branch_qty_sum, qty);
    assert_eq!(branch_value_sum, value);
}

#[tokio::test]
async fn service_and_untracked_products_are_a_no_op() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let service_id = {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        insert_service(conn).await
    };

    with_tx(&test_db.state, TxOpts::default(), move |txn, cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>> {
        Box::pin(async move {
            let mut p = lock_product(txn, service_id).await?;
            apply_change(txn, cx, &mut p, dec!(5), dec!(500), "sale", StockRef { id: Id::new(), number: "INV-1".into() }, &doc_date("2026-01-01"), None).await?;
            Ok(())
        })
    })
    .await
    .unwrap();

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let (qty, value, _) = stock_of(conn, service_id).await;
    assert_eq!(qty, Decimal::ZERO);
    assert_eq!(value, Decimal::ZERO);

    use accounting_app_lib::entities::inventory::stock_movements;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let movements = stock_movements::Entity::find().filter(stock_movements::Column::ProductId.eq(service_id)).all(conn).await.unwrap();
    assert!(movements.is_empty(), "a service product must never get a stock_movements row");
}

#[tokio::test]
async fn fefo_earliest_expiry_first_undated_last_expired_skipped() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let product_id = {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        insert_product(conn, "Milk", true).await
    };

    with_tx(&test_db.state, TxOpts::default(), move |txn, cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>> {
        Box::pin(async move {
            let mut p = lock_product(txn, product_id).await?;
            apply_change(txn, cx, &mut p, dec!(30), dec!(300), "stock_in", StockRef { id: Id::new(), number: "R".into() }, &doc_date("2026-01-01"), None).await?;

            // Expired batch (skipped unless allow_expired), near-expiry batch, undated batch.
            receive_batch(txn, cx, product_id, dec!(10), dec!(10), "EXPIRED".into(), Some(NaiveDate::parse_from_str("2020-01-01", "%Y-%m-%d").unwrap()), &doc_date("2026-01-01"), &StockRef { id: Id::new(), number: "R1".into() }).await?;
            receive_batch(txn, cx, product_id, dec!(10), dec!(10), "NEAR".into(), Some(NaiveDate::parse_from_str("2026-02-01", "%Y-%m-%d").unwrap()), &doc_date("2026-01-01"), &StockRef { id: Id::new(), number: "R2".into() }).await?;
            receive_batch(txn, cx, product_id, dec!(10), dec!(10), "UNDATED".into(), None, &doc_date("2026-01-01"), &StockRef { id: Id::new(), number: "R3".into() }).await?;
            Ok(())
        })
    })
    .await
    .unwrap();

    let today = NaiveDate::parse_from_str("2026-01-15", "%Y-%m-%d").unwrap();

    {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        let batches = active_batches(conn, product_id).await.unwrap();
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].batch_no, "EXPIRED"); // earliest expiry, sorts first regardless of expired-ness.
        assert_eq!(batches[1].batch_no, "NEAR");
        assert_eq!(batches[2].batch_no, "UNDATED"); // undated sorts last.
        assert!(is_batch_expired(&batches[0], today));
        assert!(!is_batch_expired(&batches[1], today));
    }

    // Consume 15 without allow_expired: skips EXPIRED, draws 10 from NEAR then 5 from UNDATED.
    with_tx(&test_db.state, TxOpts::default(), move |txn, cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>> {
        Box::pin(async move {
            let draws = consume_fefo(txn, cx, product_id, dec!(15), false, today).await?;
            assert_eq!(draws.len(), 2);
            assert_eq!(draws[0].qty, dec!(10));
            assert_eq!(draws[1].qty, dec!(5));
            Ok(())
        })
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn fefo_shortfall_returns_partial_draws_with_no_error() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let product_id = {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        insert_product(conn, "Milk", true).await
    };

    with_tx(&test_db.state, TxOpts::default(), move |txn, cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>> {
        Box::pin(async move {
            let mut p = lock_product(txn, product_id).await?;
            apply_change(txn, cx, &mut p, dec!(5), dec!(50), "stock_in", StockRef { id: Id::new(), number: "R".into() }, &doc_date("2026-01-01"), None).await?;
            receive_batch(txn, cx, product_id, dec!(5), dec!(10), "B1".into(), None, &doc_date("2026-01-01"), &StockRef { id: Id::new(), number: "R1".into() }).await?;
            Ok(())
        })
    })
    .await
    .unwrap();

    let today = NaiveDate::parse_from_str("2026-01-15", "%Y-%m-%d").unwrap();
    with_tx(&test_db.state, TxOpts::default(), move |txn, cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>> {
        Box::pin(async move {
            // Ask for more than exists — must return a partial draw, never an error.
            let draws = consume_fefo(txn, cx, product_id, dec!(100), false, today).await?;
            assert_eq!(draws.len(), 1);
            assert_eq!(draws[0].qty, dec!(5));
            Ok(())
        })
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn qty_rounds_to_2dp() {
    assert_eq!(cost_at_average(dec!(1.005), dec!(10)), dec!(10.05));
}

#[tokio::test]
async fn concurrent_apply_change_on_the_same_product_serializes() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let product_id = {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        insert_product(conn, "Widget", false).await
    };

    // A second, independent `AppState`/connection to the same throwaway database (so the two
    // `with_tx` calls are genuinely concurrent transactions, not sequential closures on one
    // connection) — built the same way `TestDb::fresh()` builds the first one, minus the
    // create/migrate steps (the database already exists).
    let second_state = {
        use accounting_app_lib::core::device::{DeviceRole, DeviceSettings};
        use accounting_app_lib::core::state::{AppState, Db, DbStatus};
        use accounting_app_lib::core::terminal::TerminalIdentity;
        let connection = sea_orm::Database::connect(test_db.db_url.clone()).await.expect("second connection must succeed");
        let terminal = TerminalIdentity { terminal_id: Id::new(), created_at: chrono::Utc::now() };
        let device = DeviceSettings { role: DeviceRole::Main, ..Default::default() };
        let events = Arc::new(CollectingEventSink::default());
        let state = AppState::new(std::env::temp_dir(), terminal, device, events);
        *state.db.write().unwrap() = Some(Db { connection });
        *state.db_status.write().unwrap() = DbStatus::Connected;
        let user = AuthenticatedUser {
            id: Id::new(),
            username: "test2".to_string(),
            role: Role::Admin,
            home_branch_id: Id::new(),
            allowed_branches: vec![],
            price_list_id: None,
            max_discount: None,
        };
        *state.session.write().unwrap() = Some(user);
        state
    };

    // Run two `apply_change` calls concurrently and assert the final state equals the sequential
    // result (10 + 20 = 30 units, guarded by the row lock rather than lost to a race).
    let fut_a = with_tx(&test_db.state, TxOpts::default(), move |txn, cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>> {
        Box::pin(async move {
            let mut p = lock_product(txn, product_id).await?;
            apply_change(txn, cx, &mut p, dec!(10), dec!(100), "stock_in", StockRef { id: Id::new(), number: "A".into() }, &doc_date("2026-01-01"), None).await?;
            Ok(())
        })
    });

    let fut_b = with_tx(&second_state, TxOpts::default(), move |txn, cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>> {
        Box::pin(async move {
            let mut p = lock_product(txn, product_id).await?;
            apply_change(txn, cx, &mut p, dec!(20), dec!(200), "stock_in", StockRef { id: Id::new(), number: "B".into() }, &doc_date("2026-01-01"), None).await?;
            Ok(())
        })
    });

    let timeout = tokio::time::Duration::from_secs(15);
    let (a, b) = tokio::time::timeout(timeout, async { tokio::join!(fut_a, fut_b) })
        .await
        .expect("both transactions must complete without deadlocking");
    a.unwrap();
    b.unwrap();

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let (qty, value, _) = stock_of(conn, product_id).await;
    assert_eq!(qty, dec!(30));
    assert_eq!(value, dec!(300));
}

// Only exercised for its side effect (compile-time check that `lock_products` locks a set).
#[tokio::test]
async fn lock_products_locks_a_set_sorted_by_id() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let (id_a, id_b) = {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        (insert_product(conn, "A", false).await, insert_product(conn, "B", false).await)
    };

    with_tx(&test_db.state, TxOpts::default(), move |txn, _cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>> {
        Box::pin(async move {
            let locked = lock_products(txn, &[id_b, id_a]).await?;
            assert_eq!(locked.len(), 2);
            assert!(locked.contains_key(&id_a));
            assert!(locked.contains_key(&id_b));
            Ok(())
        })
    })
    .await
    .unwrap();
}
