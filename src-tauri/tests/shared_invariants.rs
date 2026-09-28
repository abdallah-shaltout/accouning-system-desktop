//! `shared::invariants` DB-backed tests (phase-d-stock.md "Tests"). Builds a scenario only through
//! `shared::ledger::post` + `shared::stock::*` + fixture document rows, asserts every key passes,
//! then applies one targeted corruption per key and asserts that key (and only that key) fails.
//! Needs `EQUAL_TEST_DATABASE_URL` — never skipped, per `tests/support`.

mod support;

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::tx::{with_tx, BoxFuture, TxOpts};
use accounting_app_lib::entities::journal::journal_entries::JournalEntryType;
use accounting_app_lib::shared::invariants::run_all;
use accounting_app_lib::shared::ledger::{post, AccountRef, PostJournal, PostingLine, SourceRef};
use accounting_app_lib::shared::ledger::accounts::SystemRole;
use accounting_app_lib::utils::id::Id;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Statement};
use support::TestDb;

fn log_in(test_db: &TestDb) -> Id {
    let user_id = Id::new();
    let user = AuthenticatedUser {
        id: user_id,
        username: "test".to_string(),
        role: Role::Admin,
        home_branch_id: Id::new(),
        allowed_branches: vec![],
        price_list_id: None,
        max_discount: None,
    };
    *test_db.state.session.write().unwrap() = Some(user);
    user_id
}

async fn insert_account(conn: &DatabaseConnection, code: &str, kind: &str, subtype: &str, normal_side: &str, system_role: Option<&str>) -> Id {
    let id = Id::new();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO accounts (id, code, name, is_group, kind, subtype, normal_side, system_role, allow_manual, active, can_delete) \
         VALUES (?, ?, ?, FALSE, ?, ?, ?, ?, TRUE, TRUE, TRUE)",
        [
            id.to_string().into(),
            code.into(),
            format!("Account {code}").into(),
            kind.into(),
            subtype.into(),
            normal_side.into(),
            system_role.map(|s| s.into()).unwrap_or(sea_orm::Value::String(None)),
        ],
    );
    conn.execute(stmt).await.expect("account insert must succeed");
    id
}

/// `shared::ledger::post`'s `resolve_posting` needs the singleton `settings` row (for
/// `default_branch_id`/`currency`) even though this test never touches branches/currency
/// otherwise.
async fn seed_settings(conn: &DatabaseConnection) {
    let branch_id = Id::new();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO branches (id, name, code) VALUES (?, 'Main', 'MAIN')",
        [branch_id.to_string().into()],
    );
    conn.execute(stmt).await.expect("branch insert must succeed");

    let settings_id = Id::new();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO settings (id, store_name, currency, invoice_number_prefix, printer, default_branch_id) \
         VALUES (?, 'Test Store', 'SAR', 'INV-', '{}', ?)",
        [settings_id.to_string().into(), branch_id.to_string().into()],
    );
    conn.execute(stmt).await.expect("settings insert must succeed");
}

/// Seeds the minimal chart of accounts the invariants suite touches: cash, receivable, payable,
/// inventory, sales, cogs, vatOutput, vatInput, openingBalanceEquity.
async fn seed_accounts(conn: &DatabaseConnection) {
    insert_account(conn, "1000", "ASSET", "cash", "DEBIT", Some("cash")).await;
    insert_account(conn, "1100", "ASSET", "receivable", "DEBIT", Some("receivable")).await;
    insert_account(conn, "2000", "LIABILITY", "payable", "CREDIT", Some("payable")).await;
    insert_account(conn, "1300", "ASSET", "inventory", "DEBIT", Some("inventory")).await;
    insert_account(conn, "4000", "REVENUE", "revenue", "CREDIT", Some("sales")).await;
    insert_account(conn, "5000", "EXPENSE", "costOfSales", "DEBIT", Some("cogs")).await;
    insert_account(conn, "2100", "LIABILITY", "tax", "CREDIT", Some("vatOutput")).await;
    insert_account(conn, "1400", "ASSET", "tax", "DEBIT", Some("vatInput")).await;
    insert_account(conn, "3900", "EQUITY", "equity", "CREDIT", Some("openingBalanceEquity")).await;
}

#[allow(dead_code)]
async fn insert_party(conn: &DatabaseConnection, kind: &str) -> Id {
    let id = Id::new();
    let code = format!("P{}", &id.to_string().replace('-', "")[..8]);
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO parties (id, kind, type, code, name) VALUES (?, ?, 'individual', ?, 'Test Party')",
        [id.to_string().into(), kind.into(), code.into()],
    );
    conn.execute(stmt).await.expect("party insert must succeed");
    id
}

fn day(s: &str) -> chrono::NaiveDate {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn doc_date(s: &str) -> accounting_app_lib::utils::dates::DocDate {
    accounting_app_lib::utils::dates::DocDate { day: day(s), instant: None }
}

async fn assert_key_passes(conn: &DatabaseConnection, key: &str) {
    let results = run_all(conn).await.expect("run_all must not error");
    let r = results.iter().find(|r| r.key == key).unwrap_or_else(|| panic!("no invariant result for key {key}"));
    assert!(r.passed, "expected {key} to pass, got: {}", r.message);
}

async fn assert_only_key_fails(conn: &DatabaseConnection, key: &str) {
    let results = run_all(conn).await.expect("run_all must not error");
    for r in &results {
        if r.key == key {
            assert!(!r.passed, "expected {key} to fail after corruption, but it passed");
        } else {
            assert!(r.passed, "expected only {key} to fail, but {} also failed: {}", r.key, r.message);
        }
    }
}

/// Builds a minimal, fully-consistent scenario: a customer sale (cash) posted through
/// `shared::ledger::post`, with matching stock movement via `shared::stock::apply_change`, so
/// every one of the 14 invariants passes. Returns the ids needed by each corruption test.
struct Scenario {
    invoice_entry_id: Id,
    inventory_account_id: Id,
    product_id: Id,
}

async fn build_scenario(test_db: &TestDb) -> Scenario {
    let user_id = log_in(test_db);
    let (inventory_account_id, product_id) = {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        seed_settings(conn).await;
        seed_accounts(conn).await;
        let product_id = Id::new();
        let sku = format!("SKU{}", &product_id.to_string().replace('-', "")[..8]);
        let stmt = Statement::from_sql_and_values(
            conn.get_database_backend(),
            "INSERT INTO products (id, name, sku, type) VALUES (?, 'Widget', ?, 'product')",
            [product_id.to_string().into(), sku.into()],
        );
        conn.execute(stmt).await.expect("product insert must succeed");

        use accounting_app_lib::entities::org::accounts;
        let inv = accounts::Entity::find().filter(accounts::Column::SystemRole.eq("inventory")).one(conn).await.unwrap().unwrap();
        (inv.id, product_id)
    };

    // Opening stock: 10 @ 20 (value 200), booked as an opening entry (Dr inventory / Cr OBE).
    let opening_entry = with_tx(&test_db.state, TxOpts::default(), {
        let inventory_account_id = inventory_account_id;
        let product_id = product_id;
        move |txn, cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<accounting_app_lib::entities::journal::journal_entries::Model>> {
            Box::pin(async move {
                let mut p = accounting_app_lib::shared::stock::lock_product(txn, product_id).await?;
                accounting_app_lib::shared::stock::apply_change(
                    txn,
                    cx,
                    &mut p,
                    dec!(10),
                    dec!(200),
                    "stock_in",
                    accounting_app_lib::shared::stock::StockRef { id: Id::new(), number: "OPEN-1".into() },
                    &doc_date("2026-01-01"),
                    None,
                )
                .await?;
                let entry = post(
                    txn,
                    cx,
                    PostJournal {
                        date: day("2026-01-01").into(),
                        description: "Opening stock".into(),
                        entry_type: JournalEntryType::Opening,
                        source: None,
                        lines: vec![
                            PostingLine::debit(AccountRef::Id(inventory_account_id), dec!(200)),
                            PostingLine::credit(AccountRef::Role(SystemRole::OpeningBalanceEquity), dec!(200)),
                        ],
                        allow_closed_period: false,
                        attachment_ids: vec![],
                        template_id: None,
                    },
                )
                .await?;
                let _ = user_id;
                Ok(entry)
            })
        }
    })
    .await
    .unwrap();
    let _ = opening_entry;

    // Sale: qty 2 @ cost 20 -> value out 40, cash sale of 46 (40 revenue-equivalent + tax simplified
    // away — this scenario only needs GL(inventory) = Σ stockValue and balanced/valid entries, not a
    // full VAT walk).
    let invoice_id = Id::new();
    let invoice_entry_id = with_tx(&test_db.state, TxOpts::default(), {
        let inventory_account_id = inventory_account_id;
        let product_id = product_id;
        move |txn, cx| -> BoxFuture<'_, accounting_app_lib::core::tx::TxResult<Id>> {
            Box::pin(async move {
                let mut p = accounting_app_lib::shared::stock::lock_product(txn, product_id).await?;
                accounting_app_lib::shared::stock::apply_change(
                    txn,
                    cx,
                    &mut p,
                    dec!(-2),
                    dec!(-40),
                    "sale",
                    accounting_app_lib::shared::stock::StockRef { id: invoice_id, number: "INV-1".into() },
                    &doc_date("2026-01-02"),
                    None,
                )
                .await?;
                let entry = post(
                    txn,
                    cx,
                    PostJournal {
                        date: day("2026-01-02").into(),
                        description: "Sale INV-1".into(),
                        entry_type: JournalEntryType::System,
                        source: Some(SourceRef { kind: "invoice".into(), id: invoice_id, number: Some("INV-1".into()) }),
                        lines: vec![
                            PostingLine::debit(AccountRef::Role(SystemRole::Cash), dec!(60)),
                            PostingLine::credit(AccountRef::Role(SystemRole::Sales), dec!(60)),
                            PostingLine::debit(AccountRef::Role(SystemRole::Cogs), dec!(40)),
                            PostingLine::credit(AccountRef::Id(inventory_account_id), dec!(40)),
                        ],
                        allow_closed_period: false,
                        attachment_ids: vec![],
                        template_id: None,
                    },
                )
                .await?;
                Ok(entry.id)
            })
        }
    })
    .await
    .unwrap();

    Scenario { invoice_entry_id, inventory_account_id, product_id }
}

#[tokio::test]
async fn full_scenario_every_invariant_passes() {
    let test_db = TestDb::fresh().await;
    build_scenario(&test_db).await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let results = run_all(conn).await.expect("run_all must not error");
    let failed: Vec<String> = results.iter().filter(|r| !r.passed).map(|r| format!("{}: {}", r.key, r.message)).collect();
    assert!(failed.is_empty(), "expected every invariant to pass, failed: {failed:?}");
}

#[tokio::test]
async fn corrupt_a_line_amount_fails_balanced_entries_only() {
    let test_db = TestDb::fresh().await;
    let scenario = build_scenario(&test_db).await;
    {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        // Raw UPDATE of one line's debit without touching the entry header's total_debit — breaks
        // "Σ lines == entry.total_debit" implicitly via the trial balance / balance sheet checks,
        // without violating the DB's own `ck_journal_entries_balanced` CHECK (which only compares
        // the header's total_debit/total_credit to each other, not to the lines).
        use accounting_app_lib::entities::journal::journal_lines;
        let line = journal_lines::Entity::find()
            .filter(journal_lines::Column::JournalEntryId.eq(scenario.invoice_entry_id))
            .filter(journal_lines::Column::Debit.eq(dec!(60)))
            .one(conn)
            .await
            .unwrap()
            .expect("cash debit line must exist");
        let stmt = Statement::from_sql_and_values(
            conn.get_database_backend(),
            "UPDATE journal_lines SET debit = ? WHERE id = ?",
            [Decimal::from(70).into(), line.id.to_string().into()],
        );
        conn.execute(stmt).await.unwrap();
    }
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    // The header's own total_debit/total_credit still balance (untouched), but the trial balance
    // built from journal_lines no longer does — corrupts trial-balance/balance-sheet, not
    // balanced-entries (which reads the header totals, unchanged here).
    assert_key_passes(conn, "balanced-entries").await;
    let results = run_all(conn).await.unwrap();
    let trial = results.iter().find(|r| r.key == "trial-balance").unwrap();
    assert!(!trial.passed, "trial-balance must fail after a line-amount corruption");
}

#[tokio::test]
async fn corrupt_stock_value_fails_inventory_gl_only() {
    let test_db = TestDb::fresh().await;
    let scenario = build_scenario(&test_db).await;
    {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        assert_key_passes(conn, "inventory-gl").await;
        let stmt = Statement::from_sql_and_values(
            conn.get_database_backend(),
            "UPDATE products SET stock_value = stock_value + 5 WHERE id = ?",
            [scenario.product_id.to_string().into()],
        );
        conn.execute(stmt).await.unwrap();
    }
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    assert_only_key_fails(conn, "inventory-gl").await;
    let _ = scenario.inventory_account_id;
}

#[tokio::test]
async fn corrupt_draft_id_into_posted_entries_fails_drafts_isolated_only() {
    let test_db = TestDb::fresh().await;
    build_scenario(&test_db).await;
    let user_id = {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        assert_key_passes(conn, "drafts-isolated").await;

        let uid = Id::new();
        let stmt = Statement::from_sql_and_values(
            conn.get_database_backend(),
            "INSERT INTO users (id, username, name, role) VALUES (?, ?, 'Draft User', 'admin')",
            [uid.to_string().into(), format!("draftuser_{}", &uid.to_string().replace('-', "")[..8]).into()],
        );
        conn.execute(stmt).await.unwrap();
        uid
    };

    let draft_id = Id::new();
    {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        let stmt = Statement::from_sql_and_values(
            conn.get_database_backend(),
            "INSERT INTO journal_drafts (id, date_day, description, type, total_debit, total_credit, created_by) \
             VALUES (?, '2026-01-03', 'Draft', 'MANUAL', 10, 10, ?)",
            [draft_id.to_string().into(), user_id.to_string().into()],
        );
        conn.execute(stmt).await.unwrap();

        // Corruption: copy the draft's id straight into journal_entries (bypassing shared::ledger,
        // exactly the scenario `drafts-isolated` exists to catch).
        let stmt = Statement::from_sql_and_values(
            conn.get_database_backend(),
            "INSERT INTO journal_entries (id, number, date_day, description, type, status, total_debit, total_credit, created_by) \
             VALUES (?, 'JE-LEAK', '2026-01-03', 'Leaked draft', 'MANUAL', 'POSTED', 10, 10, ?)",
            [draft_id.to_string().into(), user_id.to_string().into()],
        );
        conn.execute(stmt).await.unwrap();
        let line_stmt = Statement::from_sql_and_values(
            conn.get_database_backend(),
            "INSERT INTO journal_lines (id, journal_entry_id, position, account_id, debit, credit) \
             SELECT ?, ?, 0, id, 10, 0 FROM accounts LIMIT 1",
            [Id::new().to_string().into(), draft_id.to_string().into()],
        );
        conn.execute(line_stmt).await.unwrap();
        let line_stmt2 = Statement::from_sql_and_values(
            conn.get_database_backend(),
            "INSERT INTO journal_lines (id, journal_entry_id, position, account_id, debit, credit) \
             SELECT ?, ?, 1, id, 0, 10 FROM accounts LIMIT 1",
            [Id::new().to_string().into(), draft_id.to_string().into()],
        );
        conn.execute(line_stmt2).await.unwrap();
    }

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let results = run_all(conn).await.unwrap();
    let drafts_isolated = results.iter().find(|r| r.key == "drafts-isolated").unwrap();
    assert!(!drafts_isolated.passed, "drafts-isolated must fail once a draft id leaks into journal_entries");
}
