//! DB-backed tests for the `accounting` domain (03-domains/12-accounting.md §8a). Written now, run
//! in the deferred time-boxed test pass (per-implementer hard rule: never run cargo from this
//! agent). Needs `EQUAL_TEST_DATABASE_URL` — see `tests/support/mod.rs`.
//!
//! Period-close (12b) tests are in `domain_accounting_period.rs`.

use crate::support;

use std::sync::Arc;

use chrono::Datelike;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, Set};

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::tx::{with_tx, BoxFuture, TxOpts, TxResult};
use accounting_app_lib::domains::accounting::dto::{
    AccountInput, AccountKind, AccountSubtype, JournalEntryInput, JournalEntryInputLine, JournalTemplateInput, NormalSide,
};
use accounting_app_lib::domains::accounting::service;
use accounting_app_lib::entities::org::{accounts, branches, fiscal_years, payment_methods, settings};
use accounting_app_lib::entities::values::{AccountingPolicy, PrinterMode, PrinterSettings};
use accounting_app_lib::shared::activity::undo::{self, UndoRegistry};
use accounting_app_lib::shared::invariants;
use accounting_app_lib::utils::id::Id;
use support::TestDb;

async fn log_in(test_db: &TestDb, role: Role) -> Id {
    let user_id = Id::new();
    // The session user must exist: `journal_entries.created_by`, `audit.user_id`, … FK to `users`.
    let conn = test_db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    support::seed_user(&conn, user_id, &format!("{role:?}").to_lowercase()).await;
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

async fn seed_plain_account<C: ConnectionTrait>(
    conn: &C,
    code: &str,
    name: &str,
    kind: &str,
    normal_side: &str,
    allow_manual: bool,
    is_group: bool,
    parent_id: Option<Id>,
    requires_party: bool,
) -> Id {
    let id = Id::new();
    let now = chrono::Utc::now();
    let account = accounts::ActiveModel {
        code_live: sea_orm::ActiveValue::NotSet,
        id: Set(id),
        code: Set(code.to_string()),
        name: Set(name.to_string()),
        name_en: Set(None),
        parent_id: Set(parent_id),
        is_group: Set(is_group),
        kind: Set(kind.to_string()),
        subtype: Set("otherCurrentAsset".to_string()),
        normal_side: Set(normal_side.to_string()),
        system_role: Set(None),
        currency: Set(None),
        branch_id: Set(None),
        requires_party: Set(Some(requires_party)),
        allow_manual: Set(allow_manual),
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

#[allow(dead_code)] // fields kept for tests still to be written
struct Fixture {
    pub cash_id: Id,
    pub bank_id: Id,
    pub receivable_id: Id, // requires_party
    pub inventory_id: Id,  // allow_manual = false (B1)
    pub expense_group_id: Id,
    pub expense_leaf_id: Id,
    pub revenue_leaf_id: Id,
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

    support::seed_currency(conn, "SAR").await;
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
    let fiscal_year = fiscal_years::ActiveModel {
        id: Set(Id::new()),
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
    let bank_id = seed_role_account(conn, "1120", "bank", "ASSET", "DEBIT").await;

    let receivable_id = Id::new();
    let receivable = accounts::ActiveModel {
        code_live: sea_orm::ActiveValue::NotSet,
        id: Set(receivable_id),
        code: Set("1200".to_string()),
        name: Set("العملاء".to_string()),
        name_en: Set(None),
        parent_id: Set(None),
        is_group: Set(false),
        kind: Set("ASSET".to_string()),
        subtype: Set("receivable".to_string()),
        normal_side: Set("DEBIT".to_string()),
        system_role: Set(Some("receivable".to_string())),
        currency: Set(None),
        branch_id: Set(None),
        requires_party: Set(Some(true)),
        allow_manual: Set(true),
        requires_cost_center: Set(Some(false)),
        active: Set(true),
        can_delete: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    receivable.insert(conn).await.unwrap();

    let inventory_id = Id::new();
    let inventory = accounts::ActiveModel {
        code_live: sea_orm::ActiveValue::NotSet,
        id: Set(inventory_id),
        code: Set("1300".to_string()),
        name: Set("المخزون".to_string()),
        name_en: Set(None),
        parent_id: Set(None),
        is_group: Set(false),
        kind: Set("ASSET".to_string()),
        subtype: Set("inventory".to_string()),
        normal_side: Set("DEBIT".to_string()),
        system_role: Set(Some("inventory".to_string())),
        currency: Set(None),
        branch_id: Set(None),
        requires_party: Set(Some(false)),
        allow_manual: Set(false),
        requires_cost_center: Set(Some(false)),
        active: Set(true),
        can_delete: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    inventory.insert(conn).await.unwrap();

    let expense_group_id = seed_plain_account(conn, "5000", "المصروفات", "EXPENSE", "DEBIT", false, true, None, false).await;
    let expense_leaf_id = seed_plain_account(conn, "5100", "مصروفات إيجار", "EXPENSE", "DEBIT", true, false, Some(expense_group_id), false).await;
    let revenue_leaf_id = seed_plain_account(conn, "4100", "مبيعات", "REVENUE", "CREDIT", true, false, None, false).await;

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

    Fixture { cash_id, bank_id, receivable_id, inventory_id, expense_group_id, expense_leaf_id, revenue_leaf_id }
}

fn line(account_id: Id, debit: Decimal, credit: Decimal) -> JournalEntryInputLine {
    JournalEntryInputLine { account_id, description: None, debit, credit, party_kind: None, party_id: None, branch_id: None, cost_center_id: None }
}

// --- Accounts CRUD --------------------------------------------------------------------------

#[tokio::test]
async fn save_account_creates_and_validates_code_format() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;

    let result = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            let _fixture = seed_fixture(tx).await;
            let bad = service::accounts::save_account(
                tx,
                cx,
                AccountInput {
                    code: "12a".to_string(),
                    name: "حساب".to_string(),
                    name_en: None,
                    parent_id: None,
                    is_group: false,
                    kind: AccountKind::Asset,
                    subtype: AccountSubtype::OtherCurrentAsset,
                    normal_side: NormalSide::Debit,
                    requires_party: None,
                    allow_manual: true,
                    active: true,
                },
                None,
            )
            .await;
            assert!(bad.is_err(), "non-digit code must be refused");

            let created = service::accounts::save_account(
                tx,
                cx,
                AccountInput {
                    code: "6000".to_string(),
                    name: "حساب جديد".to_string(),
                    name_en: None,
                    parent_id: None,
                    is_group: false,
                    kind: AccountKind::Asset,
                    subtype: AccountSubtype::OtherCurrentAsset,
                    normal_side: NormalSide::Debit,
                    requires_party: None,
                    allow_manual: true,
                    active: true,
                },
                None,
            )
            .await?;
            Ok(created)
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::accounting::dto::Account>>
    })
    .await
    .expect("save_account must succeed");

    assert_eq!(result.code, "6000");
    assert!(result.can_delete);
    test_db.finish().await;
}

#[tokio::test]
async fn save_account_refuses_duplicate_code() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;

    let outcome = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let _ = fixture;
            let dup = service::accounts::save_account(
                tx,
                cx,
                AccountInput {
                    code: "1110".to_string(),
                    name: "تكرار".to_string(),
                    name_en: None,
                    parent_id: None,
                    is_group: false,
                    kind: AccountKind::Asset,
                    subtype: AccountSubtype::Cash,
                    normal_side: NormalSide::Debit,
                    requires_party: None,
                    allow_manual: true,
                    active: true,
                },
                None,
            )
            .await;
            Ok(dup.is_err())
        }) as BoxFuture<'_, TxResult<bool>>
    })
    .await
    .expect("closure must run");

    assert!(outcome, "duplicate code must be refused with CONFLICT");
    test_db.finish().await;
}

#[tokio::test]
async fn delete_account_refuses_when_postings_exist() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let undo = test_db.state.undo.clone();

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
                    description: "قيد اختبار".to_string(),
                    reference: None,
                    lines: vec![line(fixture.expense_leaf_id, dec!(100), Decimal::ZERO), line(fixture.cash_id, Decimal::ZERO, dec!(100))],
                    attachment_ids: None,
                    as_draft: None,
                    template_id: None,
                },
            )
            .await?;

            let deleted = service::accounts::delete_account(tx, cx, fixture.expense_leaf_id).await;
            Ok(deleted.is_err())
        }) as BoxFuture<'_, TxResult<bool>>
    })
    .await
    .expect("closure must run");

    assert!(outcome, "an account with postings must refuse delete with CONFLICT");
    test_db.finish().await;
}

// --- Manual journal --------------------------------------------------------------------------

#[tokio::test]
async fn create_journal_entry_posts_balanced_manual_entry() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let undo = test_db.state.undo.clone();

    let entry = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            service::journal::create_journal_entry(
                tx,
                cx,
                &undo,
                JournalEntryInput {
                    date: cx.clock.today().format("%Y-%m-%d").to_string(),
                    description: "قيد يدوي تجريبي".to_string(),
                    reference: None,
                    lines: vec![line(fixture.expense_leaf_id, dec!(250), Decimal::ZERO), line(fixture.cash_id, Decimal::ZERO, dec!(250))],
                    attachment_ids: None,
                    as_draft: None,
                    template_id: None,
                },
            )
            .await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::accounting::dto::JournalEntry>>
    })
    .await
    .expect("create_journal_entry must succeed");

    assert!(entry.number.starts_with("JE-"));
    assert_eq!(entry.total_debit, dec!(250));
    assert_eq!(entry.total_credit, dec!(250));
    assert_invariants_ok(&test_db).await;
    test_db.finish().await;
}

#[tokio::test]
async fn manual_journal_refuses_posting_to_non_manual_account() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let undo = test_db.state.undo.clone();

    let outcome = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let result = service::journal::create_journal_entry(
                tx,
                cx,
                &undo,
                JournalEntryInput {
                    date: cx.clock.today().format("%Y-%m-%d").to_string(),
                    description: "محاولة ترحيل على المخزون".to_string(),
                    reference: None,
                    lines: vec![line(fixture.inventory_id, dec!(50), Decimal::ZERO), line(fixture.cash_id, Decimal::ZERO, dec!(50))],
                    attachment_ids: None,
                    as_draft: None,
                    template_id: None,
                },
            )
            .await;
            Ok(result.is_err())
        }) as BoxFuture<'_, TxResult<bool>>
    })
    .await
    .expect("closure must run");

    assert!(outcome, "B1: non-manual account must refuse with FORBIDDEN");
    test_db.finish().await;
}

#[tokio::test]
async fn manual_journal_requires_party_on_control_account() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let undo = test_db.state.undo.clone();

    let outcome = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let result = service::journal::create_journal_entry(
                tx,
                cx,
                &undo,
                JournalEntryInput {
                    date: cx.clock.today().format("%Y-%m-%d").to_string(),
                    description: "بدون طرف".to_string(),
                    reference: None,
                    lines: vec![line(fixture.receivable_id, dec!(80), Decimal::ZERO), line(fixture.cash_id, Decimal::ZERO, dec!(80))],
                    attachment_ids: None,
                    as_draft: None,
                    template_id: None,
                },
            )
            .await;
            Ok(result.is_err())
        }) as BoxFuture<'_, TxResult<bool>>
    })
    .await
    .expect("closure must run");

    assert!(outcome, "requiresParty account without a party must be refused");
    test_db.finish().await;
}

#[tokio::test]
async fn draft_lifecycle_save_update_post_delete() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let undo = test_db.state.undo.clone();

    let (draft_id, posted) = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let draft = service::journal::create_journal_entry(
                tx,
                cx,
                &undo,
                JournalEntryInput {
                    date: cx.clock.today().format("%Y-%m-%d").to_string(),
                    description: "مسودة".to_string(),
                    reference: None,
                    lines: vec![line(fixture.expense_leaf_id, dec!(60), Decimal::ZERO), line(fixture.cash_id, Decimal::ZERO, dec!(60))],
                    attachment_ids: None,
                    as_draft: Some(true),
                    template_id: None,
                },
            )
            .await?;
            assert_eq!(draft.status, accounting_app_lib::domains::accounting::dto::JournalEntryStatus::Draft);

            let updated = service::journal::update_journal_draft(
                tx,
                cx,
                draft.id,
                JournalEntryInput {
                    date: cx.clock.today().format("%Y-%m-%d").to_string(),
                    description: "مسودة محدّثة".to_string(),
                    reference: None,
                    lines: vec![line(fixture.expense_leaf_id, dec!(70), Decimal::ZERO), line(fixture.cash_id, Decimal::ZERO, dec!(70))],
                    attachment_ids: None,
                    as_draft: Some(true),
                    template_id: None,
                },
            )
            .await?;
            assert_eq!(updated.total_debit, dec!(70));

            let posted = service::journal::post_journal_draft(tx, cx, &undo, draft.id).await?;
            assert_eq!(posted.id, draft.id);
            assert_eq!(posted.number, draft.number);
            assert_eq!(posted.status, accounting_app_lib::domains::accounting::dto::JournalEntryStatus::Posted);

            Ok((draft.id, posted))
        }) as BoxFuture<'_, TxResult<(Id, accounting_app_lib::domains::accounting::dto::JournalEntry)>>
    })
    .await
    .expect("draft lifecycle must succeed");

    assert_eq!(posted.id, draft_id);
    assert_invariants_ok(&test_db).await;
    test_db.finish().await;
}

#[tokio::test]
async fn reverse_journal_entry_requires_reason_and_blocks_double_reversal() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let undo = test_db.state.undo.clone();

    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let entry = service::journal::create_journal_entry(
                tx,
                cx,
                &undo,
                JournalEntryInput {
                    date: cx.clock.today().format("%Y-%m-%d").to_string(),
                    description: "قيد للعكس".to_string(),
                    reference: None,
                    lines: vec![line(fixture.expense_leaf_id, dec!(40), Decimal::ZERO), line(fixture.cash_id, Decimal::ZERO, dec!(40))],
                    attachment_ids: None,
                    as_draft: None,
                    template_id: None,
                },
            )
            .await?;

            let empty_reason = service::journal::reverse_journal_entry(
                tx,
                cx,
                &undo,
                entry.id,
                accounting_app_lib::utils::dates::DocDate { day: cx.clock.today(), instant: None },
                "",
            )
            .await;
            assert!(empty_reason.is_err(), "empty reason must be refused");

            let (_reversal, _audit_id) = service::journal::reverse_journal_entry(
                tx,
                cx,
                &undo,
                entry.id,
                accounting_app_lib::utils::dates::DocDate { day: cx.clock.today(), instant: None },
                "خطأ في الإدخال",
            )
            .await?;

            let double = service::journal::reverse_journal_entry(
                tx,
                cx,
                &undo,
                entry.id,
                accounting_app_lib::utils::dates::DocDate { day: cx.clock.today(), instant: None },
                "مرة أخرى",
            )
            .await;
            assert!(double.is_err(), "a second reversal of the same entry must be refused");

            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("reversal test must succeed");

    test_db.finish().await;
}

// --- Templates and recurring ------------------------------------------------------------------

#[tokio::test]
async fn save_and_post_recurring_template_advances_next_date() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let undo = test_db.state.undo.clone();

    let (first_next, second_next) = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let template = service::templates::save_journal_template(
                tx,
                cx,
                JournalTemplateInput {
                    name: "إيجار شهري".to_string(),
                    description: "إيجار المكتب".to_string(),
                    lines: vec![
                        accounting_app_lib::domains::accounting::dto::JournalTemplateLine {
                            account_id: fixture.expense_leaf_id,
                            description: None,
                            debit: dec!(500),
                            credit: Decimal::ZERO,
                            party_kind: None,
                            party_id: None,
                        },
                        accounting_app_lib::domains::accounting::dto::JournalTemplateLine {
                            account_id: fixture.cash_id,
                            description: None,
                            debit: Decimal::ZERO,
                            credit: dec!(500),
                            party_kind: None,
                            party_id: None,
                        },
                    ],
                    recurrence: Some(accounting_app_lib::domains::accounting::dto::JournalTemplateRecurrence {
                        every: accounting_app_lib::domains::accounting::dto::RecurrenceEvery::Month,
                        day: 1,
                        next_date: cx.clock.today().format("%Y-%m-%d").to_string(),
                        auto_post: false,
                    }),
                },
                None,
            )
            .await?;

            let first_next = template.recurrence.as_ref().unwrap().next_date.clone();
            let _entry = service::templates::post_recurring_template(tx, cx, &undo, template.id).await?;
            let reloaded = service::templates::get_journal_template(tx, template.id).await?;
            let second_next = reloaded.recurrence.as_ref().unwrap().next_date.clone();

            Ok((first_next, second_next))
        }) as BoxFuture<'_, TxResult<(String, String)>>
    })
    .await
    .expect("recurring template post must succeed");

    assert_ne!(first_next, second_next, "next_date must advance after posting");
    assert_invariants_ok(&test_db).await;
    test_db.finish().await;
}

// --- Undo ---------------------------------------------------------------------------------

#[tokio::test]
async fn undo_create_journal_entry_reverses_it_via_registry() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db, Role::Admin).await;
    let mut registry = UndoRegistry::new();
    accounting_app_lib::domains::accounting::register_undo(&mut registry);
    let registry = Arc::new(registry);

    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let entry = service::journal::create_journal_entry(
                tx,
                cx,
                &registry,
                JournalEntryInput {
                    date: cx.clock.today().format("%Y-%m-%d").to_string(),
                    description: "قيد قابل للتراجع".to_string(),
                    reference: None,
                    lines: vec![line(fixture.expense_leaf_id, dec!(30), Decimal::ZERO), line(fixture.cash_id, Decimal::ZERO, dec!(30))],
                    attachment_ids: None,
                    as_draft: None,
                    template_id: None,
                },
            )
            .await?;

            // Find the audit row `record_manual_journal` wrote for this entry.
            let audit_row = accounting_app_lib::entities::platform::audit::Entity::find()
                .all(tx)
                .await
                .map_err(accounting_app_lib::core::tx::TxError::from)?
                .into_iter()
                .find(|a| a.action_type.as_deref() == Some("accounting.createJournalEntry"))
                .expect("an audit row for accounting.createJournalEntry must exist");

            let comp_id = undo::undo(tx, cx, &registry, audit_row.id, undo::UndoRequest { reason: "تراجع اختبار".to_string(), date: None }).await?;
            assert_ne!(comp_id, entry.id);

            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("undo must succeed");

    assert_invariants_ok(&test_db).await;
    test_db.finish().await;
}
