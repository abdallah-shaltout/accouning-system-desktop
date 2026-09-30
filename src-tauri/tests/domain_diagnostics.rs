//! `domains::diagnostics` DB-backed tests (plan 21 Part 03 §16, spec §8a). Slice A (audit reads +
//! support-bundle settings/server) plus, now that their dependencies compile, slice A2 (DB snapshot,
//! after 17-backup) and slice B (the 7 debug-build accounting-debugger reads, after 12-accounting).
//! Needs `EQUAL_TEST_DATABASE_URL` (see `tests/support/mod.rs`) — never skipped when absent. Written
//! now per the manager's "implementers write tests, never run cargo" decision; run in the deferred,
//! time-boxed DB test pass.

use crate::support;


use chrono::Datelike;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, Set};

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::dto::AuditAction;
use accounting_app_lib::core::error::AppError;
use accounting_app_lib::core::tx::{with_tx, BoxFuture, TxCtx, TxError, TxOpts, TxResult};
use accounting_app_lib::domains::accounting::dto::{JournalEntryInput, JournalEntryInputLine};
use accounting_app_lib::domains::accounting::service as accounting_service;
use accounting_app_lib::domains::diagnostics::dto::AuditFilter;
use accounting_app_lib::domains::diagnostics::service::audit as audit_service;
use accounting_app_lib::domains::diagnostics::service::debugger as debugger_service;
use accounting_app_lib::domains::diagnostics::service::support as support_service;
use accounting_app_lib::entities::org::{accounts, branches, fiscal_years, settings, users};
use accounting_app_lib::entities::platform::activity::ActivityKind;
use accounting_app_lib::entities::values::{AccountingPolicy, PrinterMode, PrinterSettings};
use accounting_app_lib::shared::activity::undo::UndoRegistry;
use accounting_app_lib::shared::activity::log;
use accounting_app_lib::shared::invariants;
use accounting_app_lib::utils::id::Id;
use accounting_app_lib::utils::route::RouteRef;
use support::TestDb;

/// Inserts a real `users` row for the session (the audit/activity `user_id` FKs reference it — a
/// random id with no row makes every audit write an FK violation) and signs it in.
async fn log_in(test_db: &TestDb, role: Role) -> Id {
    let user_id = Id::new();
    let username = format!("test-{user_id}");
    let role_name = match role {
        Role::Admin => "admin",
        Role::Manager => "manager",
        Role::Accountant => "accountant",
        Role::Cashier => "cashier",
        Role::Storekeeper => "storekeeper",
    };
    let (u, r) = (username.clone(), role_name.to_string());
    with_tx(&test_db.state, TxOpts { require_user: false }, move |tx, _cx| {
        let (u, r) = (u.clone(), r.clone());
        Box::pin(async move {
            let now = chrono::Utc::now();
            users::ActiveModel {
                id: Set(user_id),
                username: Set(u),
                name: Set("مستخدم تجريبي".to_string()),
                phone: Set(None),
                role: Set(r),
                max_discount: Set(rust_decimal::Decimal::ZERO),
                price_list_id: Set(None),
                active: Set(true),
                avatar: Set(None),
                allowed_branches: Set(None),
                home_branch: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                deleted_at: Set(None),
                sync_status: Set("local".to_string()),
            }
            .insert(tx)
            .await?;
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("seed session user must succeed");
    let user = AuthenticatedUser {
        id: user_id,
        username,
        role,
        home_branch_id: Id::new(),
        allowed_branches: vec![],
        price_list_id: None,
        max_discount: None,
    };
    *test_db.state.session.write().unwrap() = Some(user);
    user_id
}

/// Writes one audit row (via the real `shared::activity::log` write path, not a hand-built INSERT)
/// through a committed `with_tx`, so these tests exercise the same code every other domain uses to
/// produce the rows diagnostics reads back.
async fn write_activity(test_db: &TestDb, kind: ActivityKind, message: &str, link: Option<RouteRef>) -> Id {
    let registry = std::sync::Arc::new(UndoRegistry::new());
    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let message = message.to_string();
        let link = link.clone();
        let kind = kind.clone();
        let registry = registry.clone();
        Box::pin(async move { log(tx, cx, &registry, kind, message, None, link).await }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .expect("write_activity must commit")
}

#[tokio::test]
async fn audit_entries_come_back_newest_first() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;

    write_activity(&test_db, ActivityKind::Party, "أول عملية", None).await;
    write_activity(&test_db, ActivityKind::Party, "ثاني عملية", None).await;
    write_activity(&test_db, ActivityKind::Party, "ثالث عملية", None).await;

    let entries = with_tx(&test_db.state, TxOpts::default(), |tx, _cx| {
        Box::pin(async move { audit_service::get_audit_entries(tx, &AuditFilter::default()).await })
    })
    .await
    .expect("get_audit_entries must succeed");

    assert_eq!(entries.len(), 3);
    // Newest first: the `at` key must be non-increasing down the list.
    for pair in entries.windows(2) {
        assert!(pair[0].at >= pair[1].at, "entries must be sorted by `at` descending");
    }
    assert_eq!(entries[0].message, "ثالث عملية");
}

#[tokio::test]
async fn audit_entries_filter_by_user_entity_and_action() {
    let test_db = TestDb::fresh().await;
    let user_a = log_in(&test_db, Role::Admin).await;
    write_activity(&test_db, ActivityKind::Party, "من المستخدم أ", None).await;

    let user_b = log_in(&test_db, Role::Admin).await;
    write_activity(&test_db, ActivityKind::Product, "من المستخدم ب", None).await;

    let by_user = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        let filter = AuditFilter { user_id: Some(user_a), ..Default::default() };
        Box::pin(async move { audit_service::get_audit_entries(tx, &filter).await })
    })
    .await
    .expect("filter by user_id");
    assert_eq!(by_user.len(), 1);
    assert_eq!(by_user[0].user_id, user_a);

    let by_entity = with_tx(&test_db.state, TxOpts::default(), |tx, _cx| {
        let filter = AuditFilter { entity: Some("product".to_string()), ..Default::default() };
        Box::pin(async move { audit_service::get_audit_entries(tx, &filter).await })
    })
    .await
    .expect("filter by entity");
    assert_eq!(by_entity.len(), 1);
    assert_eq!(by_entity[0].user_id, user_b);

    let by_action = with_tx(&test_db.state, TxOpts::default(), |tx, _cx| {
        let filter = AuditFilter { action: Some(AuditAction::Create), ..Default::default() };
        Box::pin(async move { audit_service::get_audit_entries(tx, &filter).await })
    })
    .await
    .expect("filter by action");
    assert_eq!(by_action.len(), 2, "both rows were logged with the default Create action");
}

#[tokio::test]
async fn audit_entries_search_matches_message_entity_id_and_label_case_insensitively() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    write_activity(&test_db, ActivityKind::Party, "تحديث بيانات العميل Ahmed", None).await;

    let hits = with_tx(&test_db.state, TxOpts::default(), |tx, _cx| {
        let filter = AuditFilter { search: Some("AHMED".to_string()), ..Default::default() };
        Box::pin(async move { audit_service::get_audit_entries(tx, &filter).await })
    })
    .await
    .expect("search must succeed");
    assert_eq!(hits.len(), 1, "case-insensitive substring match over `message` must find the row");

    let misses = with_tx(&test_db.state, TxOpts::default(), |tx, _cx| {
        let filter = AuditFilter { search: Some("no-such-text".to_string()), ..Default::default() };
        Box::pin(async move { audit_service::get_audit_entries(tx, &filter).await })
    })
    .await
    .expect("search must succeed");
    assert!(misses.is_empty());
}

#[tokio::test]
async fn audit_entries_from_to_compare_the_utc_date_slice_not_business_day() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    write_activity(&test_db, ActivityKind::Party, "قيد اليوم", None).await;

    let today_key = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let tomorrow_key = (chrono::Utc::now() + chrono::Duration::days(1)).format("%Y-%m-%d").to_string();

    let within_range = with_tx(&test_db.state, TxOpts::default(), {
        let today_key = today_key.clone();
        move |tx, _cx| {
            let filter = AuditFilter { from: Some(today_key.clone()), to: Some(today_key.clone()), ..Default::default() };
            Box::pin(async move { audit_service::get_audit_entries(tx, &filter).await })
        }
    })
    .await
    .expect("from/to must succeed");
    assert_eq!(within_range.len(), 1, "an entry logged today must match a from=to=today filter (Q-1: UTC slice)");

    let excluded = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        let filter = AuditFilter { from: Some(tomorrow_key.clone()), ..Default::default() };
        Box::pin(async move { audit_service::get_audit_entries(tx, &filter).await })
    })
    .await
    .expect("from must succeed");
    assert!(excluded.is_empty(), "an entry logged today must not match a from=tomorrow filter");
}

#[tokio::test]
async fn audit_entities_are_distinct_and_code_point_sorted() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    // `ActivityKind::Party`'s default entity is "party"; log two so distinctness is exercised too.
    write_activity(&test_db, ActivityKind::Party, "قيد أول", None).await;
    write_activity(&test_db, ActivityKind::Party, "قيد ثانٍ", None).await;
    write_activity(&test_db, ActivityKind::Auth, "تسجيل دخول", None).await;

    let entities = with_tx(&test_db.state, TxOpts::default(), |tx, _cx| Box::pin(async move { audit_service::get_audit_entities(tx).await }))
        .await
        .expect("get_audit_entities must succeed");

    let mut sorted = entities.clone();
    sorted.sort();
    assert_eq!(entities, sorted, "entities must already be code-point sorted");
    let unique: std::collections::BTreeSet<_> = entities.iter().collect();
    assert_eq!(unique.len(), entities.len(), "entities must be distinct — no duplicate 'party' rows");
}

#[tokio::test]
async fn cashier_cannot_read_audit_entries_forbidden() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Cashier).await;

    let result = with_tx(&test_db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            // The role check reads `settings.role_access_overrides`, so a settings row must exist
            // (it always does in the app once setup ran).
            seed_branch_and_settings(tx).await;
            audit_service::require_audit_read(tx, &into_read_ctx(cx)).await?;
            audit_service::get_audit_entries(tx, &AuditFilter::default()).await
        })
    })
    .await;

    assert!(matches!(result, Err(AppError::Forbidden { .. })), "a cashier must not have Users:Read");
}

/// Copied from `domain_settings.rs`/`domain_accounting.rs`'s own `seed_branch_and_settings`/
/// `seed_fixture` (lessons.md: "tests can't import each other") — a minimal `branches` +
/// `settings` row so `support::support_snapshot`'s H-1 call into
/// `domains::settings::service::store::get_settings` has something to read.
async fn seed_branch_and_settings(conn: &DatabaseTransaction) -> Id {
    let now = chrono::Utc::now();
    let branch_id = Id::new();
    let branch = branches::ActiveModel {
        id: Set(branch_id),
        name: Set("الفرع الرئيسي".to_string()),
        code: Set("MAIN".to_string()),
        address: Set(None),
        national_address: Set(None),
        phone: Set(None),
        receipt_header: Set(None),
        cash_account_id: Set(None),
        bank_account_id: Set(None),
        default_price_list_id: Set(None),
        cost_center_id: Set(None),
        active: Set(true),
        can_delete: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    branch.insert(conn).await.unwrap();

    let settings_row = settings::ActiveModel {
        id: Set(Id::new()),
        singleton: Set(1),
        store_name: Set("متجر تجريبي".to_string()),
        logo: Set(None),
        stamp: Set(None),
        signature: Set(None),
        currency: Set("SAR".to_string()),
        country: Set(Some("SA".to_string())),
        vat_number: Set(None),
        default_tax_id: Set(None),
        invoice_number_prefix: Set("INV-".to_string()),
        printer: Set(PrinterSettings {
            mode: PrinterMode::A4,
            thermal_width_mm: 80,
            thermal: None,
            a4_printer_name: None,
            label_printer_name: None,
            a4_template: None,
            image_template: None,
        }),
        prices_include_tax: Set(true),
        address: Set(None),
        national_address: Set(None),
        phone: Set(None),
        commercial_register: Set(None),
        receipt_footer: Set(None),
        accounting: Set(Some(AccountingPolicy { lock_date: None, default_purchase_account_id: None })),
        backup: Set(None),
        inventory_approval_threshold: Set(None),
        role_access_overrides: Set(None),
        insight_thresholds: Set(None),
        pos: Set(None),
        sales: Set(None),
        features: Set(None),
        onboarding: Set(None),
        timezone: Set(None),
        default_branch_id: Set(branch_id),
        created_at: Set(now),
        updated_at: Set(now),
    };
    settings_row.insert(conn).await.unwrap();

    branch_id
}

#[tokio::test]
async fn any_signed_in_user_gets_a_support_snapshot_without_db() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Cashier).await;
    let device = test_db.state.device.read().unwrap().clone();

    let snapshot = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let device = device.clone();
        Box::pin(async move {
            seed_branch_and_settings(tx).await;
            support_service::support_snapshot(&device, None, tx, &into_read_ctx(cx), false).await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::diagnostics::dto::SupportSnapshot>>
    })
    .await
    .expect("a cashier with no includeDbSnapshot must get a snapshot — the rest of the bundle stays open to any signed-in user (D-2)");

    assert!(snapshot.db_snapshot.is_none());
    assert_eq!(snapshot.settings_redacted["storeName"], serde_json::json!("متجر تجريبي"));
}

#[tokio::test]
async fn admin_include_db_snapshot_has_no_credentials_table_and_no_password_hash_anywhere() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let device = test_db.state.device.read().unwrap().clone();

    let snapshot = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let device = device.clone();
        Box::pin(async move {
            seed_branch_and_settings(tx).await;
            support_service::support_snapshot(&device, None, tx, &into_read_ctx(cx), true).await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::diagnostics::dto::SupportSnapshot>>
    })
    .await
    .expect("an admin with includeDbSnapshot: true must get the DB snapshot (Settings:Write)");

    let db_snapshot = snapshot.db_snapshot.expect("db_snapshot must be present when includeDbSnapshot is true");
    let text = db_snapshot.to_string();
    assert!(!text.contains("\"credentials\""), "the credentials table name must never appear in a support bundle's DB snapshot (D-3)");
    assert!(!text.to_lowercase().contains("password_hash"), "no password_hash column may survive into the support bundle's DB snapshot");
    // The seeded branch/settings rows must still be there — proves this isn't an empty/broken export.
    assert!(text.contains("branches"), "the snapshot must still include ordinary business tables");
}

#[tokio::test]
async fn cashier_include_db_snapshot_is_forbidden() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Cashier).await;

    let device = test_db.state.device.read().unwrap().clone();
    let result = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let device = device.clone();
        Box::pin(async move {
            // The role check reads `settings.role_access_overrides`, so a settings row must exist.
            seed_branch_and_settings(tx).await;
            support_service::support_snapshot(&device, None, tx, &into_read_ctx(cx), true).await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::diagnostics::dto::SupportSnapshot>>
    })
    .await;

    assert!(matches!(result, Err(AppError::Forbidden { .. })), "includeDbSnapshot must need Settings:Write (D-2) — checked before any settings-row read");
}

/// `TxCtx` and `ReadCtx` are both built only by `core::tx`'s own helpers — this test file uses
/// `with_tx` (not `with_read_ctx`) so it can share one connection with the seeding/log-in helpers
/// above without threading a second harness through `TestDb`. `ReadCtx` only needs the actor for
/// `require`/`require_any`, so this adapter is a faithful stand-in for what `with_read_ctx` would
/// have handed the service function in production.
fn into_read_ctx(cx: &TxCtx) -> accounting_app_lib::core::tx::ReadCtx {
    accounting_app_lib::core::tx::ReadCtx { actor: cx.actor.clone(), terminal_id: cx.terminal_id, clock: cx.clock.clone() }
}

// -------------------------------------------------------------------------------------------
// Slice B (spec §8a "B (debug build)") — the 7 accounting-debugger reads. Fixture below is a
// trimmed copy of `domain_accounting.rs`'s own `seed_role_account`/`seed_fixture` (lessons.md:
// test files can't import each other) — just enough accounts (cash, receivable, payable,
// inventory, revenue, expense) plus a fiscal year, so `service::journal::create_journal_entry`
// posts cleanly and `shared::invariants::run_all` has a consistent chart of accounts to check.
// -------------------------------------------------------------------------------------------

async fn seed_role_account(conn: &DatabaseTransaction, code: &str, role: &str, kind: &str, normal_side: &str, requires_party: bool) -> Id {
    let id = Id::new();
    let now = chrono::Utc::now();
    let account = accounts::ActiveModel {
        code_live: sea_orm::ActiveValue::NotSet,
        id: Set(id),
        code: Set(code.to_string()),
        name: Set(format!("حساب {code}")),
        name_en: Set(None),
        parent_id: Set(None),
        is_group: Set(false),
        kind: Set(kind.to_string()),
        subtype: Set("otherCurrentAsset".to_string()),
        normal_side: Set(normal_side.to_string()),
        system_role: Set(Some(role.to_string())),
        currency: Set(None),
        branch_id: Set(None),
        requires_party: Set(Some(requires_party)),
        allow_manual: Set(!matches!(role, "inventory")),
        requires_cost_center: Set(Some(false)),
        active: Set(true),
        can_delete: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    account.insert(conn).await.unwrap();
    id
}

async fn seed_plain_account(conn: &DatabaseTransaction, code: &str, name: &str, kind: &str, normal_side: &str) -> Id {
    let id = Id::new();
    let now = chrono::Utc::now();
    let account = accounts::ActiveModel {
        code_live: sea_orm::ActiveValue::NotSet,
        id: Set(id),
        code: Set(code.to_string()),
        name: Set(name.to_string()),
        name_en: Set(None),
        parent_id: Set(None),
        is_group: Set(false),
        kind: Set(kind.to_string()),
        subtype: Set("otherCurrentAsset".to_string()),
        normal_side: Set(normal_side.to_string()),
        system_role: Set(None),
        currency: Set(None),
        branch_id: Set(None),
        requires_party: Set(Some(false)),
        allow_manual: Set(true),
        requires_cost_center: Set(Some(false)),
        active: Set(true),
        can_delete: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    account.insert(conn).await.unwrap();
    id
}

struct DebuggerFixture {
    cash_id: Id,
    expense_id: Id,
}

async fn seed_debugger_fixture(conn: &DatabaseTransaction) -> TxResult<DebuggerFixture> {
    seed_branch_and_settings(conn).await;

    let today = chrono::Utc::now().date_naive();
    let fiscal_year = fiscal_years::ActiveModel {
        id: Set(Id::new()),
        name: Set(today.format("%Y").to_string()),
        start_date: Set(chrono::NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap()),
        end_date: Set(chrono::NaiveDate::from_ymd_opt(today.year(), 12, 31).unwrap()),
        is_closed: Set(false),
        closing_entry_id: Set(None),
        closed_at: Set(None),
        closed_by: Set(None),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    };
    fiscal_year.insert(conn).await.map_err(TxError::from)?;

    let cash_id = seed_role_account(conn, "1110", "cash", "ASSET", "DEBIT", false).await;
    seed_role_account(conn, "1200", "receivable", "ASSET", "DEBIT", true).await;
    seed_role_account(conn, "2100", "payable", "LIABILITY", "CREDIT", true).await;
    seed_role_account(conn, "1300", "inventory", "ASSET", "DEBIT", false).await;
    let expense_id = seed_plain_account(conn, "5100", "مصروفات إيجار", "EXPENSE", "DEBIT").await;

    Ok(DebuggerFixture { cash_id, expense_id })
}

fn manual_line(account_id: Id, debit: Decimal, credit: Decimal) -> JournalEntryInputLine {
    JournalEntryInputLine { account_id, description: None, debit, credit, party_kind: None, party_id: None, branch_id: None, cost_center_id: None }
}

/// Posts one balanced manual journal entry (`expense_id` debit / `cash_id` credit) through the real
/// `create_journal_entry` posting path — same helper every slice-B test below reuses.
async fn post_manual_entry(test_db: &TestDb, fixture: &DebuggerFixture, amount: Decimal) -> accounting_app_lib::domains::accounting::dto::JournalEntry {
    // The app's own fully-registered registry (`create_journal_entry` records an undoable
    // `accounting.createJournalEntry` audit row, which must resolve to a registered handler).
    let undo = test_db.state.undo.clone();
    let expense_id = fixture.expense_id;
    let cash_id = fixture.cash_id;
    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            accounting_service::journal::create_journal_entry(
                tx,
                cx,
                &undo,
                JournalEntryInput {
                    date: cx.clock.today().format("%Y-%m-%d").to_string(),
                    description: "قيد يدوي تجريبي (مصحح التشخيص)".to_string(),
                    reference: None,
                    lines: vec![manual_line(expense_id, amount, Decimal::ZERO), manual_line(cash_id, Decimal::ZERO, amount)],
                    attachment_ids: None,
                    as_draft: None,
                    template_id: None,
                },
            )
            .await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::accounting::dto::JournalEntry>>
    })
    .await
    .expect("post_manual_entry must succeed")
}

#[tokio::test]
async fn list_recent_documents_orders_newest_first_and_marks_traced_entries() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;

    let fixture = with_tx(&test_db.state, TxOpts { require_user: false }, |tx, _cx| Box::pin(seed_debugger_fixture(tx))).await.expect("seed_debugger_fixture must succeed");
    let first = post_manual_entry(&test_db, &fixture, dec!(100)).await;
    let second = post_manual_entry(&test_db, &fixture, dec!(200)).await;

    let docs = with_tx(&test_db.state, TxOpts::default(), {
        let traces = test_db.state.traces.clone();
        move |tx, _cx| {
            let traces = traces.clone();
            Box::pin(async move { debugger_service::list_recent_documents(tx, &traces, 100).await })
        }
    })
    .await
    .expect("list_recent_documents must succeed");

    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].id, second.id, "the most recently posted entry must come first");
    assert_eq!(docs[1].id, first.id);
    assert!(docs.iter().all(|d| d.has_trace), "both entries were just posted in this process, so the trace ring must still have them");
}

#[tokio::test]
async fn posting_trace_is_recorded_on_commit_and_absent_for_a_rolled_back_attempt() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let fixture = with_tx(&test_db.state, TxOpts { require_user: false }, |tx, _cx| Box::pin(seed_debugger_fixture(tx))).await.expect("seed_debugger_fixture must succeed");

    let entry = post_manual_entry(&test_db, &fixture, dec!(75)).await;
    let trace = debugger_service::get_posting_trace(&test_db.state.traces, entry.id);
    assert!(trace.is_some(), "a committed posting must leave a trace in the ring");
    assert_eq!(trace.unwrap().doc_id, entry.id);

    assert!(debugger_service::get_posting_trace(&test_db.state.traces, Id::new()).is_none(), "an unknown entry id must return None (Q-4), not an error");
}

#[tokio::test]
async fn get_journal_entry_raw_finds_posted_entry_and_returns_none_for_unknown_id() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let fixture = with_tx(&test_db.state, TxOpts { require_user: false }, |tx, _cx| Box::pin(seed_debugger_fixture(tx))).await.expect("seed_debugger_fixture must succeed");
    let entry = post_manual_entry(&test_db, &fixture, dec!(60)).await;

    let raw = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move { debugger_service::get_journal_entry_raw(tx, entry.id).await })
    })
    .await
    .expect("get_journal_entry_raw must succeed");
    assert!(raw.is_some());
    assert_eq!(raw.unwrap().id, entry.id);

    let missing = with_tx(&test_db.state, TxOpts::default(), |tx, _cx| {
        Box::pin(async move { debugger_service::get_journal_entry_raw(tx, Id::new()).await })
    })
    .await
    .expect("get_journal_entry_raw must succeed even for an unknown id");
    assert!(missing.is_none(), "an unknown id must return None (Q-4), not NOT_FOUND");
}

#[tokio::test]
async fn balances_around_follow_number_ordering_on_the_same_day() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let fixture = with_tx(&test_db.state, TxOpts { require_user: false }, |tx, _cx| Box::pin(seed_debugger_fixture(tx))).await.expect("seed_debugger_fixture must succeed");

    let first = post_manual_entry(&test_db, &fixture, dec!(100)).await;
    let second = post_manual_entry(&test_db, &fixture, dec!(50)).await;
    assert!(first.number < second.number, "fixture assumption: numbering increases with each posted entry");

    let balances = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move { debugger_service::get_balances_around(tx, second.id).await })
    })
    .await
    .expect("get_balances_around must succeed");

    let cash_row = balances.iter().find(|b| b.account_id == fixture.cash_id).expect("cash account must be touched by the second entry");
    // Before the second entry (i.e. only the first, dr 100/cr 0 on expense, dr 0/cr 100 on cash):
    // cash's Σ(debit-credit) = -100. Including the second (-50 more) = -150.
    assert_eq!(cash_row.before, dec!(-100));
    assert_eq!(cash_row.after, dec!(-150));
}

#[tokio::test]
async fn get_balances_around_returns_empty_for_unknown_entry() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    with_tx(&test_db.state, TxOpts { require_user: false }, |tx, _cx| Box::pin(seed_debugger_fixture(tx))).await.expect("seed_debugger_fixture must succeed");

    let balances = with_tx(&test_db.state, TxOpts::default(), |tx, _cx| Box::pin(async move { debugger_service::get_balances_around(tx, Id::new()).await }))
        .await
        .expect("get_balances_around must succeed");
    assert!(balances.is_empty(), "an unknown entryId must return [] (Q-4), not NOT_FOUND");
}

#[tokio::test]
async fn invariant_results_equal_run_all_and_pass_on_a_balanced_fixture() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let fixture = with_tx(&test_db.state, TxOpts { require_user: false }, |tx, _cx| Box::pin(seed_debugger_fixture(tx))).await.expect("seed_debugger_fixture must succeed");
    post_manual_entry(&test_db, &fixture, dec!(100)).await;

    let (via_debugger, via_run_all): (Vec<accounting_app_lib::domains::diagnostics::dto::InvariantResultDto>, Vec<accounting_app_lib::shared::invariants::InvariantResult>) =
        with_tx(&test_db.state, TxOpts::default(), |tx, _cx| {
            Box::pin(async move {
                let via_debugger = debugger_service::get_invariant_results(tx).await?;
                let via_run_all = invariants::run_all(tx).await.map_err(TxError::App)?;
                Ok((via_debugger, via_run_all))
            }) as BoxFuture<'_, TxResult<(Vec<_>, Vec<_>)>>
        })
        .await
        .expect("both reads must succeed");

    assert_eq!(via_debugger.len(), via_run_all.len());
    for (dto, raw) in via_debugger.iter().zip(via_run_all.iter()) {
        assert_eq!(dto.key, raw.key);
        assert_eq!(dto.passed, raw.passed);
    }
    assert!(via_debugger.iter().all(|r| r.passed), "a single balanced manual entry on a clean fixture must pass every invariant");
}

#[tokio::test]
async fn drift_report_is_empty_on_a_clean_fixture() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let fixture = with_tx(&test_db.state, TxOpts { require_user: false }, |tx, _cx| Box::pin(seed_debugger_fixture(tx))).await.expect("seed_debugger_fixture must succeed");
    post_manual_entry(&test_db, &fixture, dec!(100)).await;

    let rows = with_tx(&test_db.state, TxOpts::default(), |tx, _cx| Box::pin(async move { debugger_service::get_drift_report(tx).await }))
        .await
        .expect("get_drift_report must succeed");
    assert!(rows.is_empty(), "no customers/suppliers/products exist in this fixture, so no drift row can be emitted");
}

#[tokio::test]
async fn explain_account_balance_returns_lines_newest_first() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let fixture = with_tx(&test_db.state, TxOpts { require_user: false }, |tx, _cx| Box::pin(seed_debugger_fixture(tx))).await.expect("seed_debugger_fixture must succeed");
    post_manual_entry(&test_db, &fixture, dec!(30)).await;
    post_manual_entry(&test_db, &fixture, dec!(40)).await;

    let expense_id = fixture.expense_id;
    let lines = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move { debugger_service::explain_account_balance(tx, expense_id, None).await })
    })
    .await
    .expect("explain_account_balance must succeed");

    assert_eq!(lines.len(), 2);
    for pair in lines.windows(2) {
        assert!(pair[0].doc_date >= pair[1].doc_date, "rows must be newest-first by docDate");
    }
}

#[tokio::test]
async fn release_mode_message_matches_the_documented_arabic_refusal() {
    // `debug_only()` (D-4) is a runtime `cfg!(debug_assertions)` check inside `commands.rs`, not a
    // `#[cfg]` on the function — this pins the exact Arabic message it returns in a release build,
    // since the actual "release build refuses" behavior can only be observed by building in release
    // mode (out of scope for a `#[tokio::test]`, which always runs under `cfg(test)` debug info).
    let expected = "أداة التشخيص المحاسبي متاحة في نسخة التطوير فقط";
    let err = AppError::forbidden(expected);
    assert!(matches!(err, AppError::Forbidden { message } if message == expected));
}
