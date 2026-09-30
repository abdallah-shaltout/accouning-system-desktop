//! B2's DB-backed tests (phase-b-entities.md "Tests"): `entities_match_schema` for every entity
//! (B1's and B2's), migrate up -> down to zero -> up, and the constraint tests that touch B2's
//! tables (journal_lines CHECKs, journal_entries balance CHECK, open-shift unique, default-template
//! unique, `uq_card_settlement_groups_date_method`, the composite party FK, an invoice with
//! lines/tenders round-trip). Needs `EQUAL_TEST_DATABASE_URL` — never skipped, per `tests/support`.

use crate::support;

use accounting_app_lib::utils::id::Id;
use rust_decimal::Decimal;
use sea_orm::sea_query::Iden;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, Iterable, QueryFilter, Statement};
use support::{unique_tail, TestDb};

/// One entity's declared column names (via SeaORM's `Column: Iden`, which yields the exact
/// snake_case DB name) — compared against `information_schema.COLUMNS` for that table.
fn declared_columns<E: EntityTrait>() -> Vec<String>
where
    E::Column: Iterable,
{
    E::Column::iter().map(|c| c.to_string()).collect()
}

/// The generated columns an entity MAY leave unmodeled: the `DocDate` bridge's `<field>_key`
/// columns (added next to `<field>_day`/`<field>_instant` by `add_doc_date_columns` /
/// `add_doc_date_key`, whose doc comments explain why the string form need not be on the entity —
/// only `DocDate` is) and the standalone partial-uniqueness keys `open_key` (`shifts`) and
/// `default_key` (`print_templates`). Some entities declare them anyway because a controller reads
/// them (e.g. the B1 inventory entities' `date_key`, `shifts.open_key`), so they are *optional*:
/// excluded from the schema side only when the entity does not declare them. Every other column —
/// including other `GENERATED` ones like `sku_live`/`name_live`/`barcode_live` — must match.
fn is_optional_generated_key(name: &str, extra: &str, all_names: &[String]) -> bool {
    if !extra.to_uppercase().contains("GENERATED") {
        return false;
    }
    if name == "open_key" || name == "default_key" {
        return true;
    }
    match name.strip_suffix("_key") {
        Some(stem) => all_names.iter().any(|n| *n == format!("{stem}_day")),
        None => false,
    }
}

/// A table's columns as `(COLUMN_NAME, EXTRA)`, in ordinal order.
async fn schema_columns(conn: &sea_orm::DatabaseConnection, table: &str) -> Vec<(String, String)> {
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "SELECT COLUMN_NAME, EXTRA FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? ORDER BY ORDINAL_POSITION",
        [table.into()],
    );
    let rows = conn.query_all(stmt).await.expect("information_schema query failed");
    rows.iter()
        .map(|r| (r.try_get::<String>("", "COLUMN_NAME").unwrap(), r.try_get::<String>("", "EXTRA").unwrap_or_default()))
        .collect()
}

/// The table's columns the entity must declare: all of them except the optional generated keys
/// (`is_optional_generated_key`) the entity leaves out.
async fn modeled_schema_columns(conn: &sea_orm::DatabaseConnection, table: &str, declared: &[String]) -> Vec<String> {
    let cols = schema_columns(conn, table).await;
    let all_names: Vec<String> = cols.iter().map(|(n, _)| n.clone()).collect();
    cols.into_iter()
        .filter(|(name, extra)| declared.contains(name) || !is_optional_generated_key(name, extra, &all_names))
        .map(|(name, _)| name)
        .collect()
}

/// Asserts an entity's declared column set equals the real table's **modeled** column set
/// (order-independent; see `is_optional_generated_key` for the only columns an entity may omit),
/// and that `SELECT * ... LIMIT 0` succeeds.
macro_rules! assert_entity_matches_schema {
    ($conn:expr, $table:literal, $entity:ty) => {{
        let mut declared = declared_columns::<$entity>();
        let mut actual = modeled_schema_columns($conn, $table, &declared).await;
        declared.sort();
        actual.sort();
        assert_eq!(declared, actual, "{}: entity columns vs information_schema.COLUMNS mismatch", $table);

        let sql = format!("SELECT * FROM `{}` LIMIT 0", $table);
        $conn.execute(Statement::from_string($conn.get_database_backend(), sql)).await.unwrap_or_else(|e| panic!("{}: SELECT * LIMIT 0 failed: {e}", $table));
    }};
}

#[tokio::test]
async fn entities_match_schema_for_every_table() {
    use accounting_app_lib::entities::{catalog, expenses, inventory, journal, org, parties, payments, platform, purchases, sales};

    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    // --- org (B1) ---
    assert_entity_matches_schema!(conn, "branches", org::Branches);
    assert_entity_matches_schema!(conn, "cost_centers", org::CostCenters);
    assert_entity_matches_schema!(conn, "cost_center_budgets", org::CostCenterBudgets);
    assert_entity_matches_schema!(conn, "fiscal_years", org::FiscalYears);
    assert_entity_matches_schema!(conn, "currencies", org::Currencies);
    assert_entity_matches_schema!(conn, "exchange_rates", org::ExchangeRates);
    assert_entity_matches_schema!(conn, "taxes", org::Taxes);
    assert_entity_matches_schema!(conn, "settings", org::Settings);
    assert_entity_matches_schema!(conn, "payment_methods", org::PaymentMethods);
    assert_entity_matches_schema!(conn, "accounts", org::Accounts);
    assert_entity_matches_schema!(conn, "users", org::Users);
    assert_entity_matches_schema!(conn, "credentials", org::Credentials);

    // --- catalog (B1) ---
    assert_entity_matches_schema!(conn, "categories", catalog::Categories);
    assert_entity_matches_schema!(conn, "units", catalog::Units);
    assert_entity_matches_schema!(conn, "price_lists", catalog::PriceLists);
    assert_entity_matches_schema!(conn, "product_prices", catalog::ProductPrices);
    assert_entity_matches_schema!(conn, "custom_field_defs", catalog::CustomFieldDefs);
    assert_entity_matches_schema!(conn, "products", catalog::Products);
    assert_entity_matches_schema!(conn, "product_branch_stock", catalog::ProductBranchStock);
    assert_entity_matches_schema!(conn, "product_batches", catalog::ProductBatches);

    // --- inventory (B1) ---
    assert_entity_matches_schema!(conn, "stock_adjustments", inventory::StockAdjustments);
    assert_entity_matches_schema!(conn, "stock_adjustment_lines", inventory::StockAdjustmentLines);
    assert_entity_matches_schema!(conn, "stock_movements", inventory::StockMovements);
    assert_entity_matches_schema!(conn, "stock_counts", inventory::StockCounts);
    assert_entity_matches_schema!(conn, "stock_count_lines", inventory::StockCountLines);
    assert_entity_matches_schema!(conn, "debit_note_drafts", inventory::DebitNoteDrafts);
    assert_entity_matches_schema!(conn, "stock_transfers", inventory::StockTransfers);
    assert_entity_matches_schema!(conn, "stock_transfer_lines", inventory::StockTransferLines);

    // --- parties (B1) ---
    assert_entity_matches_schema!(conn, "parties", parties::Parties);
    assert_entity_matches_schema!(conn, "party_groups", parties::PartyGroups);
    assert_entity_matches_schema!(conn, "party_history", parties::PartyHistory);

    // --- sales (B2) ---
    assert_entity_matches_schema!(conn, "invoices", sales::Invoices);
    assert_entity_matches_schema!(conn, "invoice_lines", sales::InvoiceLines);
    assert_entity_matches_schema!(conn, "invoice_tenders", sales::InvoiceTenders);
    assert_entity_matches_schema!(conn, "refunds", sales::Refunds);
    assert_entity_matches_schema!(conn, "refund_lines", sales::RefundLines);
    assert_entity_matches_schema!(conn, "quotations", sales::Quotations);
    assert_entity_matches_schema!(conn, "quotation_lines", sales::QuotationLines);
    assert_entity_matches_schema!(conn, "held_sales", sales::HeldSales);
    assert_entity_matches_schema!(conn, "shifts", sales::Shifts);
    assert_entity_matches_schema!(conn, "shift_movements", sales::ShiftMovements);

    // --- purchases (B2) ---
    assert_entity_matches_schema!(conn, "purchase_orders", purchases::PurchaseOrders);
    assert_entity_matches_schema!(conn, "purchase_order_lines", purchases::PurchaseOrderLines);
    assert_entity_matches_schema!(conn, "purchase_returns", purchases::PurchaseReturns);
    assert_entity_matches_schema!(conn, "purchase_return_lines", purchases::PurchaseReturnLines);

    // --- payments (B2) ---
    assert_entity_matches_schema!(conn, "payments", payments::Payments);
    assert_entity_matches_schema!(conn, "payment_allocations", payments::PaymentAllocations);
    assert_entity_matches_schema!(conn, "vouchers", payments::Vouchers);
    assert_entity_matches_schema!(conn, "card_settlements", payments::CardSettlements);
    assert_entity_matches_schema!(conn, "card_settlement_groups", payments::CardSettlementGroups);

    // --- expenses (B2) ---
    assert_entity_matches_schema!(conn, "expense_categories", expenses::ExpenseCategories);
    assert_entity_matches_schema!(conn, "expenses", expenses::Expenses);
    assert_entity_matches_schema!(conn, "recurring_expenses", expenses::RecurringExpenses);

    // --- journal (B2) ---
    assert_entity_matches_schema!(conn, "journal_entries", journal::JournalEntries);
    assert_entity_matches_schema!(conn, "journal_lines", journal::JournalLines);
    assert_entity_matches_schema!(conn, "journal_drafts", journal::JournalDrafts);
    assert_entity_matches_schema!(conn, "journal_draft_lines", journal::JournalDraftLines);
    assert_entity_matches_schema!(conn, "journal_templates", journal::JournalTemplates);

    // --- platform (B2) ---
    assert_entity_matches_schema!(conn, "activity", platform::Activity);
    assert_entity_matches_schema!(conn, "audit", platform::Audit);
    assert_entity_matches_schema!(conn, "approval_requests", platform::ApprovalRequests);
    assert_entity_matches_schema!(conn, "print_templates", platform::PrintTemplates);
}

#[tokio::test]
async fn migrate_up_down_to_zero_up_succeeds() {
    use migration::MigratorTrait;
    let test_db = TestDb::fresh().await; // already migrated up once by TestDb::fresh().
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    migration::Migrator::down(conn, None).await.expect("migrate down to zero must succeed");
    migration::Migrator::up(conn, None).await.expect("migrate back up from zero must succeed");
}

#[tokio::test]
async fn journal_lines_checks_reject_two_sided_and_negative() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let entry_id = insert_balanced_journal_entry(conn, Decimal::new(1000, 2)).await;

    // Two-sided (both debit and credit > 0) must be rejected by ck_journal_lines_one_sided.
    let two_sided = insert_journal_line(conn, entry_id, 0, Decimal::new(500, 2), Decimal::new(500, 2)).await;
    assert!(two_sided.is_err(), "a two-sided journal line must violate ck_journal_lines_one_sided");

    // Negative debit must be rejected by ck_journal_lines_non_negative.
    let negative = insert_journal_line(conn, entry_id, 1, Decimal::new(-100, 2), Decimal::ZERO).await;
    assert!(negative.is_err(), "a negative journal line amount must violate ck_journal_lines_non_negative");
}

#[tokio::test]
async fn journal_entries_balance_check_rejects_unbalanced_totals() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let result = insert_journal_entry_header(conn, Decimal::new(1000, 2), Decimal::new(900, 2)).await;
    assert!(result.is_err(), "total_debit != total_credit must violate ck_journal_entries_balanced");
}

#[tokio::test]
async fn shifts_open_key_unique_rejects_a_second_open_shift_on_the_same_terminal() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let terminal_id = Id::new();
    let user_id = insert_minimal_user(conn).await;

    insert_shift(conn, terminal_id, user_id, "OPEN").await.expect("first OPEN shift must succeed");
    let second = insert_shift(conn, terminal_id, user_id, "OPEN").await;
    assert!(second.is_err(), "a second OPEN shift on the same terminal must violate uq_shifts_open_key");

    // A CLOSED shift on the same terminal is fine (open_key is NULL for non-OPEN rows).
    let closed = insert_shift(conn, terminal_id, user_id, "CLOSED").await;
    assert!(closed.is_ok(), "a CLOSED shift must not be blocked by the open-shift unique");
}

#[tokio::test]
async fn print_templates_default_key_unique_rejects_a_second_default_per_branch_and_kind() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let branch_id = insert_minimal_branch(conn).await;

    insert_print_template(conn, branch_id, "invoice", true).await.expect("first default template must succeed");
    let second = insert_print_template(conn, branch_id, "invoice", true).await;
    assert!(second.is_err(), "a second default template for the same (branch, kind) must violate uq_print_templates_default_key");

    // A non-default template for the same branch/kind is fine.
    let non_default = insert_print_template(conn, branch_id, "invoice", false).await;
    assert!(non_default.is_ok(), "a non-default template must not be blocked by the default-template unique");
}

#[tokio::test]
async fn card_settlement_groups_date_method_unique_rejects_a_duplicate() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let user_id = insert_minimal_user(conn).await;
    let payment_method_id = insert_minimal_payment_method(conn).await;
    let settlement_id = insert_card_settlement(conn, user_id).await;
    let settlement_id_2 = insert_card_settlement(conn, user_id).await;

    let day = "2026-01-15";
    insert_card_settlement_group(conn, settlement_id, 0, day, payment_method_id).await.expect("first group must succeed");
    let dup = insert_card_settlement_group(conn, settlement_id_2, 0, day, payment_method_id).await;
    assert!(dup.is_err(), "a duplicate (date, payment_method_id) must violate uq_card_settlement_groups_date_method");
}

#[tokio::test]
async fn composite_party_fk_rejects_a_kind_mismatch() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let account_id = insert_minimal_account(conn).await;
    let user_id = insert_minimal_user(conn).await;
    let customer_id = insert_minimal_party(conn, "customer").await;
    let entry_id = insert_balanced_journal_entry(conn, Decimal::new(1000, 2)).await;

    // party_kind = 'supplier' pointing at a row that is actually kind='customer' must be rejected
    // by the composite FK (party_id, party_kind) -> parties(id, kind).
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO journal_lines (id, journal_entry_id, position, account_id, debit, credit, party_kind, party_id) \
         VALUES (?, ?, ?, ?, ?, ?, 'supplier', ?)",
        [
            Id::new().to_string().into(),
            entry_id.to_string().into(),
            0i32.into(),
            account_id.to_string().into(),
            Decimal::new(1000, 2).into(),
            Decimal::ZERO.into(),
            customer_id.to_string().into(),
        ],
    );
    let result = conn.execute(stmt).await;
    assert!(result.is_err(), "party_kind='supplier' pointing at a customer row must violate the composite FK");
    let _ = user_id;
}

#[tokio::test]
async fn invoice_with_lines_and_tenders_round_trips() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let user_id = insert_minimal_user(conn).await;
    let product_id = insert_minimal_product(conn).await;
    let payment_method_id = insert_minimal_payment_method(conn).await;

    let invoice_id = Id::new();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO invoices (id, number, date_day, cashier_id, status, payment_status, sub_total, \
         discount_rate, discount_amount, tax_rate, tax_amount, grand_total, payment_method, paid_amount, \
         refunded_amount) VALUES (?, ?, ?, ?, 'COMPLETED', 'PAID', ?, 0, 0, ?, ?, ?, 'cash', ?, 0)",
        [
            invoice_id.to_string().into(),
            "INV-0001".into(),
            "2026-01-15".into(),
            user_id.to_string().into(),
            Decimal::new(10000, 2).into(),
            Decimal::new(1500, 4).into(),
            Decimal::new(1500, 2).into(),
            Decimal::new(11500, 2).into(),
            Decimal::new(11500, 2).into(),
        ],
    );
    conn.execute(stmt).await.expect("invoice header insert must succeed");

    let line_id = Id::new();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO invoice_lines (id, invoice_id, position, product_id, name, qty, price, cost_price, discount, is_free_text) \
         VALUES (?, ?, 0, ?, 'Widget', ?, ?, ?, 0, FALSE)",
        [
            line_id.to_string().into(),
            invoice_id.to_string().into(),
            product_id.to_string().into(),
            Decimal::new(20000, 4).into(),
            Decimal::new(5000, 4).into(),
            Decimal::new(3000, 4).into(),
        ],
    );
    conn.execute(stmt).await.expect("invoice line insert must succeed");

    let tender_id = Id::new();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO invoice_tenders (id, invoice_id, position, payment_method_id, amount) VALUES (?, ?, 0, ?, ?)",
        [
            tender_id.to_string().into(),
            invoice_id.to_string().into(),
            payment_method_id.to_string().into(),
            Decimal::new(11500, 2).into(),
        ],
    );
    conn.execute(stmt).await.expect("invoice tender insert must succeed");

    // Round-trip: read back the invoice, its one line, and its one tender.
    use accounting_app_lib::entities::sales;
    let invoice = sales::Invoices::find_by_id(invoice_id).one(conn).await.unwrap().expect("invoice must read back");
    assert_eq!(invoice.grand_total, Decimal::new(11500, 2));
    assert_eq!(invoice.number, "INV-0001");

    let lines: Vec<sales::invoice_lines::Model> =
        sales::InvoiceLines::find().filter(sales::invoice_lines::Column::InvoiceId.eq(invoice_id)).all(conn).await.unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].price, Decimal::new(5000, 4));

    let tenders: Vec<sales::invoice_tenders::Model> =
        sales::InvoiceTenders::find().filter(sales::invoice_tenders::Column::InvoiceId.eq(invoice_id)).all(conn).await.unwrap();
    assert_eq!(tenders.len(), 1);
    assert_eq!(tenders[0].amount, Decimal::new(11500, 2));
}

// --- minimal-row helpers (bare inserts satisfying NOT NULL/FK constraints, nothing more) ---------


async fn insert_minimal_user(conn: &sea_orm::DatabaseConnection) -> Id {
    let id = Id::new();
    let username = format!("user_{}", id.to_string().replace('-', ""));
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO users (id, username, name, role) VALUES (?, ?, 'Test User', 'admin')",
        [id.to_string().into(), username.into()],
    );
    conn.execute(stmt).await.expect("minimal user insert must succeed");
    id
}

async fn insert_minimal_branch(conn: &sea_orm::DatabaseConnection) -> Id {
    let id = Id::new();
    let code = format!("BR{}", unique_tail(id, 8));
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO branches (id, name, code) VALUES (?, 'Test Branch', ?)",
        [id.to_string().into(), code.into()],
    );
    conn.execute(stmt).await.expect("minimal branch insert must succeed");
    id
}

async fn insert_minimal_account(conn: &sea_orm::DatabaseConnection) -> Id {
    let id = Id::new();
    let code = format!("A{}", unique_tail(id, 6));
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO accounts (id, code, name, is_group, kind, subtype, normal_side, allow_manual, active, can_delete) \
         VALUES (?, ?, 'Test Account', FALSE, 'ASSET', 'cash', 'DEBIT', TRUE, TRUE, TRUE)",
        [id.to_string().into(), code.into()],
    );
    conn.execute(stmt).await.expect("minimal account insert must succeed");
    id
}

async fn insert_minimal_payment_method(conn: &sea_orm::DatabaseConnection) -> Id {
    let id = Id::new();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO payment_methods (id, name, type, account_role) VALUES (?, 'Cash', 'cash', 'cash')",
        [id.to_string().into()],
    );
    conn.execute(stmt).await.expect("minimal payment method insert must succeed");
    id
}

async fn insert_minimal_product(conn: &sea_orm::DatabaseConnection) -> Id {
    let id = Id::new();
    let sku = format!("SKU{}", unique_tail(id, 8));
    let unit_id = insert_minimal_unit(conn).await;
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO products (id, sku, name, unit_id, cost_price, price) VALUES (?, ?, 'Widget', ?, 0, 0)",
        [id.to_string().into(), sku.into(), unit_id.to_string().into()],
    );
    conn.execute(stmt).await.expect("minimal product insert must succeed");
    id
}

async fn insert_minimal_unit(conn: &sea_orm::DatabaseConnection) -> Id {
    let id = Id::new();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO units (id, name) VALUES (?, ?)",
        [id.to_string().into(), format!("Each {}", unique_tail(id, 8)).into()],
    );
    conn.execute(stmt).await.expect("minimal unit insert must succeed");
    id
}

async fn insert_minimal_party(conn: &sea_orm::DatabaseConnection, kind: &str) -> Id {
    let id = Id::new();
    let code = format!("P{}", unique_tail(id, 8));
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO parties (id, kind, code, name) VALUES (?, ?, ?, 'Test Party')",
        [id.to_string().into(), kind.into(), code.into()],
    );
    conn.execute(stmt).await.expect("minimal party insert must succeed");
    id
}

async fn insert_journal_entry_header(conn: &sea_orm::DatabaseConnection, debit: Decimal, credit: Decimal) -> Result<Id, sea_orm::DbErr> {
    let user_id = insert_minimal_user(conn).await;
    let id = Id::new();
    let number = format!("JE-{}", unique_tail(id, 8));
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO journal_entries (id, number, date_day, description, type, status, total_debit, total_credit, created_by) \
         VALUES (?, ?, '2026-01-15', 'Test entry', 'MANUAL', 'POSTED', ?, ?, ?)",
        [id.to_string().into(), number.into(), debit.into(), credit.into(), user_id.to_string().into()],
    );
    conn.execute(stmt).await?;
    Ok(id)
}

async fn insert_balanced_journal_entry(conn: &sea_orm::DatabaseConnection, amount: Decimal) -> Id {
    insert_journal_entry_header(conn, amount, amount).await.expect("balanced journal entry header must insert")
}

async fn insert_journal_line(conn: &sea_orm::DatabaseConnection, entry_id: Id, position: i32, debit: Decimal, credit: Decimal) -> Result<(), sea_orm::DbErr> {
    let account_id = insert_minimal_account(conn).await;
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO journal_lines (id, journal_entry_id, position, account_id, debit, credit) VALUES (?, ?, ?, ?, ?, ?)",
        [
            Id::new().to_string().into(),
            entry_id.to_string().into(),
            position.into(),
            account_id.to_string().into(),
            debit.into(),
            credit.into(),
        ],
    );
    conn.execute(stmt).await?;
    Ok(())
}

async fn insert_shift(conn: &sea_orm::DatabaseConnection, terminal_id: Id, user_id: Id, status: &str) -> Result<(), sea_orm::DbErr> {
    let id = Id::new();
    let number = format!("SH-{}", unique_tail(id, 8));
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO shifts (id, number, terminal_id, status, opened_by, opened_at_day, opening_float) \
         VALUES (?, ?, ?, ?, ?, '2026-01-15', 0)",
        [id.to_string().into(), number.into(), terminal_id.to_string().into(), status.into(), user_id.to_string().into()],
    );
    conn.execute(stmt).await?;
    Ok(())
}

async fn insert_print_template(conn: &sea_orm::DatabaseConnection, branch_id: Id, kind: &str, is_default: bool) -> Result<(), sea_orm::DbErr> {
    let id = Id::new();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO print_templates (id, name, kind, base_template_id, options, is_default, branch_id) \
         VALUES (?, 'Test Template', ?, 'invoice_standard', '{}', ?, ?)",
        [id.to_string().into(), kind.into(), is_default.into(), branch_id.to_string().into()],
    );
    conn.execute(stmt).await?;
    Ok(())
}

async fn insert_card_settlement(conn: &sea_orm::DatabaseConnection, user_id: Id) -> Id {
    let id = Id::new();
    let number = format!("CS-{}", unique_tail(id, 8));
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO card_settlements (id, number, date_day, gross_amount, deposit_amount, fee_amount, created_by) \
         VALUES (?, ?, '2026-01-15', 100, 98, 2, ?)",
        [id.to_string().into(), number.into(), user_id.to_string().into()],
    );
    conn.execute(stmt).await.expect("minimal card settlement insert must succeed");
    id
}

async fn insert_card_settlement_group(
    conn: &sea_orm::DatabaseConnection,
    settlement_id: Id,
    position: i32,
    day: &str,
    payment_method_id: Id,
) -> Result<(), sea_orm::DbErr> {
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO card_settlement_groups (id, card_settlement_id, position, date_day, payment_method_id, amount) \
         VALUES (?, ?, ?, ?, ?, 50)",
        [
            Id::new().to_string().into(),
            settlement_id.to_string().into(),
            position.into(),
            day.into(),
            payment_method_id.to_string().into(),
        ],
    );
    conn.execute(stmt).await?;
    Ok(())
}
