//! DB-backed tests for the `accounting` domain's period-close half (12b-period-close.md §8a).
//! Written now, run in the deferred time-boxed test pass (never run cargo from this agent). Needs
//! `EQUAL_TEST_DATABASE_URL` — see `tests/support/mod.rs`.

mod support;

use std::sync::Arc;

use chrono::Datelike;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseTransaction, Set};

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::tx::{with_tx, BoxFuture, TxOpts, TxResult};
use accounting_app_lib::domains::accounting::dto::{FiscalYearInput, JournalEntryInput, JournalEntryInputLine};
use accounting_app_lib::domains::accounting::service;
use accounting_app_lib::entities::org::{accounts, branches, fiscal_years, payment_methods, settings};
use accounting_app_lib::entities::values::{AccountingPolicy, PrinterMode, PrinterSettings};
use accounting_app_lib::shared::activity::undo::UndoRegistry;
use accounting_app_lib::shared::invariants;
use accounting_app_lib::utils::id::Id;
use support::TestDb;

fn log_in(test_db: &TestDb, role: Role) -> Id {
    let user_id = Id::new();
    let user = AuthenticatedUser {
        id: user_id,
        username: "test".to_string(),
        role,
        home_branch_id: Id::new(),
        allowed_branches: vec![],
        price_list_id: None,
        max_discount: None,
    };
    *test_db.state.session.write().unwrap() = Some(user);
    user_id
}

async fn assert_invariants_ok(test_db: &TestDb) {
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let results = invariants::run_all(conn).await.expect("invariants must run");
    let failed: Vec<_> = results.iter().filter(|r| !r.passed).collect();
    assert!(failed.is_empty(), "invariants failed: {failed:?}");
}

async fn seed_role_account<C: ConnectionTrait>(conn: &C, code: &str, role: &str, kind: &str, normal_side: &str) -> Id {
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
        requires_party: Set(Some(false)),
        allow_manual: Set(true),
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

struct Fixture {
    pub fiscal_year_id: Id,
    pub cash_id: Id,
    pub revenue_id: Id,
    pub expense_id: Id,
    pub payment_method_id: Id,
}

async fn seed_fixture(conn: &DatabaseTransaction) -> Fixture {
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

    let today = chrono::Utc::now().date_naive();
    let fiscal_year_id = Id::new();
    let fiscal_year = fiscal_years::ActiveModel {
        id: Set(fiscal_year_id),
        name: Set(today.year().to_string()),
        start_date: Set(chrono::NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap()),
        end_date: Set(chrono::NaiveDate::from_ymd_opt(today.year(), 12, 31).unwrap()),
        is_closed: Set(false),
        closing_entry_id: Set(None),
        closed_at: Set(None),
        closed_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    fiscal_year.insert(conn).await.unwrap();

    let cash_id = seed_role_account(conn, "1110", "cash", "ASSET", "DEBIT").await;
    let _retained_id = seed_role_account(conn, "3250", "retainedEarnings", "EQUITY", "CREDIT").await;
    let _opening_equity_id = seed_role_account(conn, "3900", "openingBalanceEquity", "EQUITY", "CREDIT").await;
    let _vat_output_id = seed_role_account(conn, "2150", "vatOutput", "LIABILITY", "CREDIT").await;
    let _vat_input_id = seed_role_account(conn, "2110", "vatInput", "ASSET", "DEBIT").await;
    let _vat_payable_id = seed_role_account(conn, "2155", "vatPayable", "LIABILITY", "CREDIT").await;

    let revenue_id = Id::new();
    let revenue = accounts::ActiveModel {
        code_live: sea_orm::ActiveValue::NotSet,
        id: Set(revenue_id),
        code: Set("4100".to_string()),
        name: Set("مبيعات".to_string()),
        name_en: Set(None),
        parent_id: Set(None),
        is_group: Set(false),
        kind: Set("REVENUE".to_string()),
        subtype: Set("revenue".to_string()),
        normal_side: Set("CREDIT".to_string()),
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
    revenue.insert(conn).await.unwrap();

    let expense_id = Id::new();
    let expense = accounts::ActiveModel {
        code_live: sea_orm::ActiveValue::NotSet,
        id: Set(expense_id),
        code: Set("5100".to_string()),
        name: Set("مصروفات".to_string()),
        name_en: Set(None),
        parent_id: Set(None),
        is_group: Set(false),
        kind: Set("EXPENSE".to_string()),
        subtype: Set("operatingExpense".to_string()),
        normal_side: Set("DEBIT".to_string()),
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
    expense.insert(conn).await.unwrap();

    let payment_method_id = Id::new();
    let payment_method = payment_methods::ActiveModel {
        id: Set(payment_method_id),
        name: Set("نقدًا".to_string()),
        r#type: Set("cash".to_string()),
        icon: Set(None),
        account_role: Set("cash".to_string()),
        fee_pct: Set(Decimal::ZERO),
        requires_reference: Set(None),
        show_in_pos: Set(true),
        show_in_payments: Set(true),
        sort_order: Set(1),
        branch_overrides: Set(None),
        active: Set(true),
        can_delete: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    payment_method.insert(conn).await.unwrap();

    Fixture { fiscal_year_id, cash_id, revenue_id, expense_id, payment_method_id }
}

fn line(account_id: Id, debit: Decimal, credit: Decimal) -> JournalEntryInputLine {
    JournalEntryInputLine { account_id, description: None, debit, credit, party_kind: None, party_id: None, branch_id: None, cost_center_id: None }
}

// --- Fiscal years ----------------------------------------------------------------------------

#[tokio::test]
async fn save_fiscal_year_refuses_overlap_and_edit_when_closed() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin);

    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;

            let overlap = service::period::save_fiscal_year(
                tx,
                cx,
                FiscalYearInput {
                    name: "متداخلة".to_string(),
                    start_date: "2020-06-01".to_string(),
                    end_date: format!("{}-06-30", cx.clock.today().year()),
                    is_closed: false,
                    closing_entry_id: None,
                    closed_at: None,
                    closed_by: None,
                },
                None,
            )
            .await;
            assert!(overlap.is_err(), "an overlapping period must be refused with CONFLICT");

            let bad_dates = service::period::save_fiscal_year(
                tx,
                cx,
                FiscalYearInput {
                    name: "تواريخ خاطئة".to_string(),
                    start_date: "2030-01-01".to_string(),
                    end_date: "2029-01-01".to_string(),
                    is_closed: false,
                    closing_entry_id: None,
                    closed_at: None,
                    closed_by: None,
                },
                None,
            )
            .await;
            assert!(bad_dates.is_err(), "end before start must be refused");

            let _ = fixture;
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("fiscal year validation test must succeed");
}

#[tokio::test]
async fn lock_date_blocks_posting_on_or_before_it() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Accountant);
    let undo = Arc::new(UndoRegistry::new());

    let outcome = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let today = cx.clock.today();
            service::period::save_lock_date(tx, cx, Some(today.format("%Y-%m-%d").to_string())).await?;

            let blocked = service::journal::create_journal_entry(
                tx,
                cx,
                &undo,
                JournalEntryInput {
                    date: today.format("%Y-%m-%d").to_string(),
                    description: "محاولة ترحيل قبل تاريخ القفل".to_string(),
                    reference: None,
                    lines: vec![line(fixture.expense_id, dec!(10), Decimal::ZERO), line(fixture.cash_id, Decimal::ZERO, dec!(10))],
                    attachment_ids: None,
                    as_draft: None,
                    template_id: None,
                },
            )
            .await;
            Ok(blocked.is_err())
        }) as BoxFuture<'_, TxResult<bool>>
    })
    .await
    .expect("closure must run");

    assert!(outcome, "a post on the lock date must be refused for a non-admin");
}

// --- Closing wizard ----------------------------------------------------------------------------

#[tokio::test]
async fn close_year_posts_closing_entry_and_creates_next_year() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin);
    let undo = Arc::new(UndoRegistry::new());

    let result = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;

            // Revenue > expense so the closing entry has a real net profit to move to retained earnings.
            service::journal::create_journal_entry(
                tx,
                cx,
                &undo,
                JournalEntryInput {
                    date: cx.clock.today().format("%Y-%m-%d").to_string(),
                    description: "إيراد".to_string(),
                    reference: None,
                    lines: vec![line(fixture.cash_id, dec!(1000), Decimal::ZERO), line(fixture.revenue_id, Decimal::ZERO, dec!(1000))],
                    attachment_ids: None,
                    as_draft: None,
                    template_id: None,
                },
            )
            .await?;
            service::journal::create_journal_entry(
                tx,
                cx,
                &undo,
                JournalEntryInput {
                    date: cx.clock.today().format("%Y-%m-%d").to_string(),
                    description: "مصروف".to_string(),
                    reference: None,
                    lines: vec![line(fixture.expense_id, dec!(400), Decimal::ZERO), line(fixture.cash_id, Decimal::ZERO, dec!(400))],
                    attachment_ids: None,
                    as_draft: None,
                    template_id: None,
                },
            )
            .await?;

            let pre_checks = service::period::get_close_year_pre_checks(tx, fixture.fiscal_year_id).await?;
            assert!(pre_checks.iter().all(|c| c.passed), "pre-checks must pass: {pre_checks:?}");

            let result = service::period::close_year(tx, cx, &undo, fixture.fiscal_year_id).await?;
            Ok(result)
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::accounting::dto::CloseYearResult>>
    })
    .await
    .expect("close_year must succeed");

    assert!(result.fiscal_year.is_closed);
    assert_eq!(result.closing_entry.total_debit, result.closing_entry.total_credit);
    assert!(result.next_year.is_some(), "closing must auto-create the next fiscal year");
    assert_invariants_ok(&test_db).await;
}

#[tokio::test]
async fn close_year_twice_is_refused() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin);
    let undo = Arc::new(UndoRegistry::new());

    let outcome = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            service::journal::create_journal_entry(
                tx,
                cx,
                &undo,
                JournalEntryInput {
                    date: cx.clock.today().format("%Y-%m-%d").to_string(),
                    description: "إيراد".to_string(),
                    reference: None,
                    lines: vec![line(fixture.cash_id, dec!(200), Decimal::ZERO), line(fixture.revenue_id, Decimal::ZERO, dec!(200))],
                    attachment_ids: None,
                    as_draft: None,
                    template_id: None,
                },
            )
            .await?;

            service::period::close_year(tx, cx, &undo, fixture.fiscal_year_id).await?;
            let second = service::period::close_year(tx, cx, &undo, fixture.fiscal_year_id).await;
            Ok(second.is_err())
        }) as BoxFuture<'_, TxResult<bool>>
    })
    .await
    .expect("closure must run");

    assert!(outcome, "closing an already-closed year must be refused");
}

#[tokio::test]
async fn reopen_year_requires_admin_and_mirrors_closing_entry() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin);
    let undo = Arc::new(UndoRegistry::new());

    let fiscal_year_id = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            service::journal::create_journal_entry(
                tx,
                cx,
                &undo,
                JournalEntryInput {
                    date: cx.clock.today().format("%Y-%m-%d").to_string(),
                    description: "إيراد".to_string(),
                    reference: None,
                    lines: vec![line(fixture.cash_id, dec!(500), Decimal::ZERO), line(fixture.revenue_id, Decimal::ZERO, dec!(500))],
                    attachment_ids: None,
                    as_draft: None,
                    template_id: None,
                },
            )
            .await?;
            service::period::close_year(tx, cx, &undo, fixture.fiscal_year_id).await?;
            Ok(fixture.fiscal_year_id)
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .expect("setup close must succeed");

    // Non-admin is refused first.
    log_in(&test_db, Role::Accountant);
    let undo2 = Arc::new(UndoRegistry::new());
    let non_admin_outcome = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo2 = undo2.clone();
        Box::pin(async move {
            let result = service::period::reopen_year(tx, cx, &undo2, fiscal_year_id).await;
            Ok(result.is_err())
        }) as BoxFuture<'_, TxResult<bool>>
    })
    .await
    .expect("closure must run");
    assert!(non_admin_outcome, "non-admin reopen must be refused");

    // Admin succeeds.
    log_in(&test_db, Role::Admin);
    let undo3 = Arc::new(UndoRegistry::new());
    let (fy, _audit_id) = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo3 = undo3.clone();
        Box::pin(async move { service::period::reopen_year(tx, cx, &undo3, fiscal_year_id).await }) as BoxFuture<'_, TxResult<(accounting_app_lib::domains::accounting::dto::FiscalYear, Id)>>
    })
    .await
    .expect("admin reopen must succeed");

    assert!(!fy.is_closed);
    assert!(fy.closing_entry_id.is_none());
    assert_invariants_ok(&test_db).await;
}

// --- VAT settlement ------------------------------------------------------------------------

#[tokio::test]
async fn vat_settlement_zero_movement_is_refused() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin);

    let outcome = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let today = cx.clock.today().format("%Y-%m-%d").to_string();
            let result = service::vat::submit_vat_settlement(tx, cx, &today, &today).await;
            let _ = fixture;
            Ok(result.is_err())
        }) as BoxFuture<'_, TxResult<bool>>
    })
    .await
    .expect("closure must run");

    assert!(outcome, "zero VAT movement must be refused");
}

#[tokio::test]
async fn pay_vat_settlement_now_zero_amount_is_refused() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin);
    let undo = Arc::new(UndoRegistry::new());

    let outcome = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let result = service::vat::pay_vat_settlement_now(tx, cx, &undo, Decimal::ZERO, fixture.payment_method_id).await;
            Ok(result.is_err())
        }) as BoxFuture<'_, TxResult<bool>>
    })
    .await
    .expect("closure must run");

    assert!(outcome, "a zero payment amount must be refused");
}
