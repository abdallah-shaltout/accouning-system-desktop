//! `shared::ledger` DB-backed tests (phase-c-ledger.md "Tests"). Needs `EQUAL_TEST_DATABASE_URL`
//! (see `tests/support/mod.rs`) — never skipped, panics loudly instead per the entry file's own
//! instruction.

mod support;

use chrono::Datelike;

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::error::AppError;
use accounting_app_lib::core::events::ChangeCategory;
use accounting_app_lib::core::tx::{with_tx, BoxFuture, TxError, TxOpts};
use accounting_app_lib::entities::journal::journal_drafts::{Entity as DraftEntity, Model as DraftModel};
use accounting_app_lib::entities::journal::journal_entries::{Entity as EntryEntity, JournalEntryType, Model as JournalEntry};
use accounting_app_lib::entities::org::accounts::{ActiveModel as AccountActiveModel, Entity as AccountEntity};
use accounting_app_lib::entities::org::branches::ActiveModel as BranchActiveModel;
use accounting_app_lib::entities::org::fiscal_years::ActiveModel as FiscalYearActiveModel;
use accounting_app_lib::entities::org::settings::ActiveModel as SettingsActiveModel;
use accounting_app_lib::entities::values::{AccountingPolicy, PrinterMode, PrinterSettings};
use accounting_app_lib::shared::ledger::{
    post, reverse, AccountRef, MirrorDims, PartyRef, PostJournal, PostingLine, ReverseRequest, ReversalReason,
};
use accounting_app_lib::shared::ledger::accounts::SystemRole;
use accounting_app_lib::utils::id::Id;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
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

/// Minimal fixture: one branch, one settings row (pointing at it), and one live active account
/// per requested system role, in role-list order (so `ORDER BY created_at, id` gives back the
/// same order they were inserted in — every insert here is sequential and awaited, so
/// `created_at`'s `DEFAULT CURRENT_TIMESTAMP(3)` is monotonic in practice; ids are UUIDv7 so
/// insertion order and generation order agree too).
struct Fixture {
    pub branch_id: Id,
    pub accounts: std::collections::HashMap<&'static str, Id>,
}

async fn seed_fixture<C: ConnectionTrait>(conn: &C, roles: &[(&'static str, &str)]) -> Fixture {
    let branch_id = Id::new();
    let branch = BranchActiveModel {
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
        can_delete: Set(true),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    branch.insert(conn).await.unwrap();

    let settings = SettingsActiveModel {
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
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    };
    settings.insert(conn).await.unwrap();

    let mut accounts = std::collections::HashMap::new();
    for (label, role) in roles {
        let id = Id::new();
        let account = AccountActiveModel {
            code_live: sea_orm::ActiveValue::NotSet,
            id: Set(id),
            code: Set(format!("ACC-{label}")),
            name: Set(format!("حساب {label}")),
            name_en: Set(None),
            parent_id: Set(None),
            is_group: Set(false),
            kind: Set("ASSET".to_string()),
            subtype: Set("cash".to_string()),
            normal_side: Set("DEBIT".to_string()),
            system_role: Set(Some((*role).to_string())),
            currency: Set(None),
            branch_id: Set(None),
            requires_party: Set(Some(false)),
            allow_manual: Set(true),
            requires_cost_center: Set(Some(false)),
            active: Set(true),
            can_delete: Set(false),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
        };
        account.insert(conn).await.unwrap();
        accounts.insert(*label, id);
    }

    Fixture { branch_id, accounts }
}

async fn seed_group_account<C: ConnectionTrait>(conn: &C) -> Id {
    let id = Id::new();
    let account = AccountActiveModel {
        code_live: sea_orm::ActiveValue::NotSet,
        id: Set(id),
        code: Set("1000".to_string()),
        name: Set("الأصول".to_string()),
        name_en: Set(None),
        parent_id: Set(None),
        is_group: Set(true),
        kind: Set("ASSET".to_string()),
        subtype: Set("group".to_string()),
        normal_side: Set("DEBIT".to_string()),
        system_role: Set(None),
        currency: Set(None),
        branch_id: Set(None),
        requires_party: Set(Some(false)),
        allow_manual: Set(false),
        requires_cost_center: Set(Some(false)),
        active: Set(true),
        can_delete: Set(false),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    account.insert(conn).await.unwrap();
    id
}

fn cash_role() -> SystemRole {
    SystemRole::Cash
}
fn sales_role() -> SystemRole {
    SystemRole::Sales
}

#[tokio::test]
async fn balanced_post_persists_entry_and_lines_with_sequential_numbers_and_defaults() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let fixture = seed_fixture(conn, &[("cash", "cash"), ("sales", "sales")]).await;
    drop(db_guard);

    for expected_number in ["JE-000001", "JE-000002"] {
        let result: Result<JournalEntry, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
            Box::pin(async move {
                let req = PostJournal::new(
                    chrono::Utc::now().date_naive(),
                    "قيد اختبار",
                    JournalEntryType::Manual,
                    vec![
                        PostingLine::debit(AccountRef::Role(cash_role()), dec!(10.005)),
                        PostingLine::credit(AccountRef::Role(sales_role()), dec!(10.005)),
                    ],
                );
                post(txn, cx, req).await
            }) as BoxFuture<'_, Result<_, TxError>>
        })
        .await;
        let entry = result.unwrap();
        assert_eq!(entry.number, expected_number);
        assert_eq!(entry.total_debit, dec!(10.01), "10.005 debit must be stored as 10.01 (half away from zero)");
        assert_eq!(entry.total_credit, dec!(10.01));
    }

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let entries = EntryEntity::find().all(conn).await.unwrap();
    assert_eq!(entries.len(), 2);

    use accounting_app_lib::entities::journal::journal_lines::{Column as LineColumn, Entity as LineEntity};
    let lines = LineEntity::find().filter(LineColumn::JournalEntryId.eq(entries[0].id)).all(conn).await.unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].branch_id, Some(fixture.branch_id));
    assert_eq!(lines[0].currency.as_deref(), Some("SAR"));
}

#[tokio::test]
async fn zero_lines_are_dropped_and_one_kept_line_gives_the_exact_message() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    seed_fixture(conn, &[("cash", "cash"), ("sales", "sales")]).await;
    drop(db_guard);

    let result: Result<_, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
        Box::pin(async move {
            let req = PostJournal::new(
                chrono::Utc::now().date_naive(),
                "قيد سطر واحد",
                JournalEntryType::Manual,
                vec![
                    PostingLine::debit(AccountRef::Role(cash_role()), dec!(10)),
                    PostingLine::credit(AccountRef::Role(sales_role()), Decimal::ZERO),
                ],
            );
            post(txn, cx, req).await
        }) as BoxFuture<'_, Result<_, TxError>>
    })
    .await;

    let err = result.unwrap_err();
    assert_eq!(err.to_string(), "يجب أن يحتوي القيد على سطرين على الأقل");
}

#[tokio::test]
async fn unbalanced_post_gives_the_exact_arabic_message() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    seed_fixture(conn, &[("cash", "cash"), ("sales", "sales")]).await;
    drop(db_guard);

    let result: Result<_, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
        Box::pin(async move {
            let req = PostJournal::new(
                chrono::Utc::now().date_naive(),
                "قيد غير متوازن",
                JournalEntryType::Manual,
                vec![
                    PostingLine::debit(AccountRef::Role(cash_role()), dec!(100)),
                    PostingLine::credit(AccountRef::Role(sales_role()), dec!(99.99)),
                ],
            );
            post(txn, cx, req).await
        }) as BoxFuture<'_, Result<_, TxError>>
    })
    .await;

    let err = result.unwrap_err();
    assert_eq!(err.to_string(), "القيد غير متوازن: المدين 100 ≠ الدائن 99.99");
}

#[tokio::test]
async fn group_account_is_refused() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    seed_fixture(conn, &[("sales", "sales")]).await;
    let group_id = seed_group_account(conn).await;
    drop(db_guard);

    let result: Result<_, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
        Box::pin(async move {
            let req = PostJournal::new(
                chrono::Utc::now().date_naive(),
                "قيد على حساب تجميعي",
                JournalEntryType::Manual,
                vec![
                    PostingLine::debit(AccountRef::Id(group_id), dec!(10)),
                    PostingLine::credit(AccountRef::Role(sales_role()), dec!(10)),
                ],
            );
            post(txn, cx, req).await
        }) as BoxFuture<'_, Result<_, TxError>>
    })
    .await;

    let err = result.unwrap_err();
    assert!(err.to_string().contains("حساب رئيسي (تجميعي) ولا يقبل الترحيل المباشر"));
}

#[tokio::test]
async fn missing_role_gives_the_label_message() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    seed_fixture(conn, &[("sales", "sales")]).await; // no "cash" role account seeded
    drop(db_guard);

    let result: Result<_, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
        Box::pin(async move {
            let req = PostJournal::new(
                chrono::Utc::now().date_naive(),
                "قيد بدون حساب صندوق",
                JournalEntryType::Manual,
                vec![
                    PostingLine::debit(AccountRef::Role(cash_role()), dec!(10)),
                    PostingLine::credit(AccountRef::Role(sales_role()), dec!(10)),
                ],
            );
            post(txn, cx, req).await
        }) as BoxFuture<'_, Result<_, TxError>>
    })
    .await;

    let err = result.unwrap_err();
    assert!(err.to_string().contains("الصندوق"), "expected the cash role's Arabic label in: {err}");
}

#[tokio::test]
async fn period_lock_date_and_closed_year_are_refused_unless_allowed() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    seed_fixture(conn, &[("cash", "cash"), ("sales", "sales")]).await;

    let closed_year_start = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let closed_year_end = chrono::NaiveDate::from_ymd_opt(2020, 12, 31).unwrap();
    let fy = FiscalYearActiveModel {
        id: Set(Id::new()),
        name: Set("2020".to_string()),
        start_date: Set(closed_year_start),
        end_date: Set(closed_year_end),
        is_closed: Set(true),
        closing_entry_id: Set(None),
        closed_at: Set(None),
        closed_by: Set(None),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    };
    fy.insert(conn).await.unwrap();
    drop(db_guard);

    let closed_year_date = chrono::NaiveDate::from_ymd_opt(2020, 6, 15).unwrap();

    // Closed-year refusal.
    let result: Result<_, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
        Box::pin(async move {
            let mut req = PostJournal::new(
                closed_year_date,
                "قيد في سنة مقفلة",
                JournalEntryType::Manual,
                vec![
                    PostingLine::debit(AccountRef::Role(cash_role()), dec!(10)),
                    PostingLine::credit(AccountRef::Role(sales_role()), dec!(10)),
                ],
            );
            req.allow_closed_period = false;
            post(txn, cx, req).await
        }) as BoxFuture<'_, Result<_, TxError>>
    })
    .await;
    let err = result.unwrap_err();
    assert!(err.to_string().contains("مقفلة"), "expected a closed-year message, got: {err}");

    // allow_closed_period bypasses it.
    let result: Result<_, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
        Box::pin(async move {
            let mut req = PostJournal::new(
                closed_year_date,
                "قيد بصلاحية تجاوز الإغلاق",
                JournalEntryType::Manual,
                vec![
                    PostingLine::debit(AccountRef::Role(cash_role()), dec!(10)),
                    PostingLine::credit(AccountRef::Role(sales_role()), dec!(10)),
                ],
            );
            req.allow_closed_period = true;
            post(txn, cx, req).await
        }) as BoxFuture<'_, Result<_, TxError>>
    })
    .await;
    assert!(result.is_ok(), "allow_closed_period must bypass the closed-year refusal");

    // No covering fiscal year at all is allowed (mock's own behaviour, core.ts:112).
    let uncovered_date = chrono::NaiveDate::from_ymd_opt(2099, 1, 1).unwrap();
    let result: Result<_, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
        Box::pin(async move {
            let req = PostJournal::new(
                uncovered_date,
                "قيد بدون سنة مالية مغطية",
                JournalEntryType::Manual,
                vec![
                    PostingLine::debit(AccountRef::Role(cash_role()), dec!(10)),
                    PostingLine::credit(AccountRef::Role(sales_role()), dec!(10)),
                ],
            );
            post(txn, cx, req).await
        }) as BoxFuture<'_, Result<_, TxError>>
    })
    .await;
    assert!(result.is_ok(), "no covering fiscal year must be allowed, matching the mock");
}

#[tokio::test]
async fn reverse_keep_vs_default_dimensions_and_both_reason_variants_and_double_reversal_refused() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let fixture = seed_fixture(conn, &[("cash", "cash"), ("sales", "sales")]).await;
    drop(db_guard);

    let original_id = std::sync::Arc::new(std::sync::Mutex::new(None::<Id>));
    let original_id_clone = original_id.clone();
    let branch_id = fixture.branch_id;

    let result: Result<JournalEntry, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
        let original_id_clone = original_id_clone.clone();
        Box::pin(async move {
            let mut req = PostJournal::new(
                chrono::Utc::now().date_naive(),
                "قيد أصلي",
                JournalEntryType::Manual,
                vec![
                    PostingLine { branch_id: Some(branch_id), ..PostingLine::debit(AccountRef::Role(cash_role()), dec!(20)) },
                    PostingLine::credit(AccountRef::Role(sales_role()), dec!(20)),
                ],
            );
            req.allow_closed_period = false;
            let entry = post(txn, cx, req).await?;
            *original_id_clone.lock().unwrap() = Some(entry.id);
            Ok(entry)
        }) as BoxFuture<'_, Result<_, TxError>>
    })
    .await;
    let original = result.unwrap();
    let original_id = original.id;

    // Keep dimensions: mirror carries the same branch id.
    let result: Result<JournalEntry, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
        Box::pin(async move {
            reverse(
                txn,
                cx,
                ReverseRequest {
                    original_id,
                    date: chrono::Utc::now().date_naive().into(),
                    description: "عكس بالإبقاء على الأبعاد".to_string(),
                    entry_type: JournalEntryType::Manual,
                    allow_closed_period: false,
                    reason: Some(ReversalReason { text: "سبب العكس".to_string(), stamp_original: true }),
                    dims: MirrorDims::Keep,
                },
            )
            .await
        }) as BoxFuture<'_, Result<_, TxError>>
    })
    .await;
    let mirror = result.unwrap();
    assert_eq!(mirror.reversal_of_id, Some(original_id));
    assert_eq!(mirror.reversal_reason.as_deref(), Some("سبب العكس"));

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let original_after = EntryEntity::find_by_id(original_id).one(conn).await.unwrap().unwrap();
    assert!(original_after.reversed);
    assert_eq!(original_after.reversal_reason.as_deref(), Some("سبب العكس"), "stamp_original must set the original's reason too");
    drop(db_guard);

    // A second reversal of the same original is refused.
    let result: Result<_, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
        Box::pin(async move {
            reverse(
                txn,
                cx,
                ReverseRequest {
                    original_id,
                    date: chrono::Utc::now().date_naive().into(),
                    description: "عكس ثانٍ".to_string(),
                    entry_type: JournalEntryType::Manual,
                    allow_closed_period: false,
                    reason: Some(ReversalReason { text: "سبب آخر".to_string(), stamp_original: false }),
                    dims: MirrorDims::Default,
                },
            )
            .await
        }) as BoxFuture<'_, Result<_, TxError>>
    })
    .await;
    let err = result.unwrap_err();
    assert_eq!(err.to_string(), "هذا القيد معكوس بالفعل");
}

#[tokio::test]
async fn draft_post_draft_keeps_id_and_number_and_removes_the_draft() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    seed_fixture(conn, &[("cash", "cash"), ("sales", "sales")]).await;
    drop(db_guard);

    let draft_id = std::sync::Arc::new(std::sync::Mutex::new(None::<Id>));
    let draft_id_clone = draft_id.clone();
    let result: Result<DraftModel, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
        let draft_id_clone = draft_id_clone.clone();
        Box::pin(async move {
            let draft = accounting_app_lib::shared::ledger::save_draft(
                txn,
                cx,
                chrono::Utc::now().date_naive().into(),
                "مسودة اختبار".to_string(),
                vec![
                    PostingLine::debit(AccountRef::Role(cash_role()), dec!(15)),
                    PostingLine::credit(AccountRef::Role(sales_role()), dec!(15)),
                ],
                vec![],
                None,
            )
            .await?;
            *draft_id_clone.lock().unwrap() = Some(draft.id);
            Ok(draft)
        }) as BoxFuture<'_, Result<_, TxError>>
    })
    .await;
    let draft = result.unwrap();
    let draft_id = draft.id;
    let draft_number = draft.number.clone();

    let result: Result<JournalEntry, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
        Box::pin(async move { accounting_app_lib::shared::ledger::post_draft(txn, cx, draft_id, false).await }) as BoxFuture<'_, Result<_, TxError>>
    })
    .await;
    let posted = result.unwrap();
    assert_eq!(posted.id, draft_id, "post_draft must keep the same id");
    assert_eq!(Some(posted.number), draft_number, "post_draft must keep the same number");

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let remaining_draft = DraftEntity::find_by_id(draft_id).one(conn).await.unwrap();
    assert!(remaining_draft.is_none(), "the draft row must be gone after post_draft");
}

#[tokio::test]
async fn effects_ledger_always_touched_parties_only_with_a_party_line() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    seed_fixture(conn, &[("cash", "cash"), ("sales", "sales"), ("receivable", "receivable")]).await;
    drop(db_guard);

    // No party line: only Ledger touched.
    let _: Result<_, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
        Box::pin(async move {
            let req = PostJournal::new(
                chrono::Utc::now().date_naive(),
                "بدون طرف",
                JournalEntryType::Manual,
                vec![
                    PostingLine::debit(AccountRef::Role(cash_role()), dec!(5)),
                    PostingLine::credit(AccountRef::Role(sales_role()), dec!(5)),
                ],
            );
            post(txn, cx, req).await
        }) as BoxFuture<'_, Result<_, TxError>>
    })
    .await;
    assert_eq!(test_db.state.change_seen.lock().unwrap().get(&ChangeCategory::Parties).copied(), None);
    let ledger_after_first = test_db.state.change_seen.lock().unwrap().get(&ChangeCategory::Ledger).copied();
    assert_eq!(ledger_after_first, Some(1));

    // A party line touches Parties too.
    let party_id = Id::new();
    let _: Result<_, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
        Box::pin(async move {
            let req = PostJournal::new(
                chrono::Utc::now().date_naive(),
                "بطرف",
                JournalEntryType::Manual,
                vec![
                    PostingLine {
                        party: Some(PartyRef { kind: accounting_app_lib::entities::journal::journal_lines::PartyKind::Customer, id: party_id }),
                        ..PostingLine::debit(AccountRef::Role(SystemRole::Receivable), dec!(5))
                    },
                    PostingLine::credit(AccountRef::Role(sales_role()), dec!(5)),
                ],
            );
            post(txn, cx, req).await
        }) as BoxFuture<'_, Result<_, TxError>>
    })
    .await;
    assert_eq!(test_db.state.change_seen.lock().unwrap().get(&ChangeCategory::Parties).copied(), Some(1));
    let ledger_after_second = test_db.state.change_seen.lock().unwrap().get(&ChangeCategory::Ledger).copied();
    assert_eq!(ledger_after_second, Some(2));
}

#[tokio::test]
async fn trace_ring_gets_the_entry_after_commit_and_nothing_after_a_rollback() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    seed_fixture(conn, &[("cash", "cash"), ("sales", "sales")]).await;
    drop(db_guard);

    // A rolled-back post leaves no trace.
    let _: Result<_, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
        Box::pin(async move {
            let req = PostJournal::new(
                chrono::Utc::now().date_naive(),
                "قيد سيتم التراجع عنه",
                JournalEntryType::Manual,
                vec![
                    PostingLine::debit(AccountRef::Role(cash_role()), dec!(5)),
                    PostingLine::credit(AccountRef::Role(sales_role()), dec!(5)),
                ],
            );
            let _entry = post(txn, cx, req).await?;
            Err(TxError::from(AppError::validation("deliberate rollback")))
        }) as BoxFuture<'_, Result<accounting_app_lib::entities::journal::journal_entries::Model, TxError>>
    })
    .await;
    assert_eq!(test_db.state.traces.recent(10).len(), 0, "a rolled-back post must leave no trace");

    // A committed post lands in the ring, findable by entry id.
    let result: Result<JournalEntry, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
        Box::pin(async move {
            let req = PostJournal::new(
                chrono::Utc::now().date_naive(),
                "قيد سيُرحّل",
                JournalEntryType::Manual,
                vec![
                    PostingLine::debit(AccountRef::Role(cash_role()), dec!(5)),
                    PostingLine::credit(AccountRef::Role(sales_role()), dec!(5)),
                ],
            );
            post(txn, cx, req).await
        }) as BoxFuture<'_, Result<_, TxError>>
    })
    .await;
    let entry = result.unwrap();
    assert!(test_db.state.traces.for_entry(entry.id).is_some(), "a committed post's trace must be findable by entry id");
}

#[tokio::test]
async fn resolve_account_precedence_and_inactive_soft_deleted_excluded() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let fixture = seed_fixture(conn, &[("neutral", "cash")]).await;

    // A second, branch-specific cash account.
    let branch_specific_id = Id::new();
    let account = AccountActiveModel {
        code_live: sea_orm::ActiveValue::NotSet,
        id: Set(branch_specific_id),
        code: Set("ACC-branch".to_string()),
        name: Set("صندوق الفرع".to_string()),
        name_en: Set(None),
        parent_id: Set(None),
        is_group: Set(false),
        kind: Set("ASSET".to_string()),
        subtype: Set("cash".to_string()),
        normal_side: Set("DEBIT".to_string()),
        system_role: Set(Some("cash".to_string())),
        currency: Set(None),
        branch_id: Set(Some(fixture.branch_id)),
        requires_party: Set(Some(false)),
        allow_manual: Set(true),
        requires_cost_center: Set(Some(false)),
        active: Set(true),
        can_delete: Set(false),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    account.insert(conn).await.unwrap();

    // An inactive third candidate must never be picked.
    let inactive_id = Id::new();
    let inactive = AccountActiveModel {
        code_live: sea_orm::ActiveValue::NotSet,
        id: Set(inactive_id),
        code: Set("ACC-inactive".to_string()),
        name: Set("صندوق معطل".to_string()),
        name_en: Set(None),
        parent_id: Set(None),
        is_group: Set(false),
        kind: Set("ASSET".to_string()),
        subtype: Set("cash".to_string()),
        normal_side: Set("DEBIT".to_string()),
        system_role: Set(Some("cash".to_string())),
        currency: Set(None),
        branch_id: Set(None),
        requires_party: Set(Some(false)),
        allow_manual: Set(true),
        requires_cost_center: Set(Some(false)),
        active: Set(false),
        can_delete: Set(false),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    inactive.insert(conn).await.unwrap();

    let ctx_with_branch = accounting_app_lib::shared::ledger::accounts::AccountCtx { branch_id: Some(fixture.branch_id), currency: None };
    let resolved = accounting_app_lib::shared::ledger::accounts::resolve_account(conn, SystemRole::Cash, &ctx_with_branch).await.unwrap();
    assert_eq!(resolved.id, branch_specific_id, "a branch match must win over the neutral account");

    let ctx_no_branch = accounting_app_lib::shared::ledger::accounts::AccountCtx::default();
    let resolved_neutral = accounting_app_lib::shared::ledger::accounts::resolve_account(conn, SystemRole::Cash, &ctx_no_branch).await.unwrap();
    assert_eq!(resolved_neutral.id, fixture.accounts["neutral"], "with no context, the neutral (no branch/currency) account must win");

    // Soft-deleting the neutral account excludes it; only the inactive/branch ones remain, and
    // inactive is excluded by the `active` filter, so the branch-specific one (no ctx) becomes
    // the sole/first candidate instead.
    let mut neutral_model: AccountActiveModel = AccountEntity::find_by_id(fixture.accounts["neutral"]).one(conn).await.unwrap().unwrap().into();
    neutral_model.deleted_at = Set(Some(chrono::Utc::now()));
    neutral_model.update(conn).await.unwrap();

    let resolved_after_delete = accounting_app_lib::shared::ledger::accounts::resolve_account(conn, SystemRole::Cash, &ctx_no_branch).await.unwrap();
    assert_ne!(resolved_after_delete.id, fixture.accounts["neutral"], "a soft-deleted account must be excluded");
    assert_ne!(resolved_after_delete.id, inactive_id, "an inactive account must never be picked");
}

/// Concurrency (a): 20 concurrent posts against the SAME `TestDb` (one pooled connection, many
/// transactions) get 20 distinct sequential numbers with no gaps — `next_number`'s
/// `UPDATE ... WHERE kind = ?` X-locks the counter row until commit, serializing the increments.
#[tokio::test]
async fn twenty_concurrent_posts_get_distinct_sequential_numbers_with_no_gaps() {
    let test_db = std::sync::Arc::new(TestDb::fresh().await);
    log_in(&test_db);

    {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        seed_fixture(conn, &[("cash", "cash"), ("sales", "sales")]).await;
    }

    let mut handles = Vec::new();
    for _ in 0..20 {
        let test_db = test_db.clone();
        handles.push(tokio::spawn(async move {
            let result: Result<JournalEntry, AppError> = with_tx(&test_db.state, TxOpts::default(), |txn, cx| {
                Box::pin(async move {
                    let req = PostJournal::new(
                        chrono::Utc::now().date_naive(),
                        "قيد متزامن",
                        JournalEntryType::Manual,
                        vec![
                            PostingLine::debit(AccountRef::Role(cash_role()), dec!(1)),
                            PostingLine::credit(AccountRef::Role(sales_role()), dec!(1)),
                        ],
                    );
                    post(txn, cx, req).await
                }) as BoxFuture<'_, Result<_, TxError>>
            })
            .await;
            result.unwrap().number
        }));
    }

    let mut numbers = Vec::new();
    for handle in handles {
        numbers.push(tokio::time::timeout(std::time::Duration::from_secs(30), handle).await.expect("a concurrent post timed out").unwrap());
    }
    numbers.sort();
    let expected: Vec<String> = (1..=20).map(|n| format!("JE-{n:06}")).collect();
    assert_eq!(numbers, expected, "20 concurrent posts must get 20 distinct sequential numbers with no gaps");
}

/// Concurrency (b): transaction A holds the fiscal year's exclusive (X) lock via
/// `lock_fiscal_year_exclusive`, then B's post (which only needs the shared lock) blocks until A
/// commits (`is_closed = true`); B is then refused with the closed-year message. Timeout-guarded
/// so a lock-order bug hangs the test instead of the whole suite.
#[tokio::test]
async fn closing_a_year_blocks_a_concurrent_poster_then_refuses_it() {
    let test_db = std::sync::Arc::new(TestDb::fresh().await);
    log_in(&test_db);

    let fy_id = Id::new();
    let today = chrono::Utc::now().date_naive();
    let year_start = chrono::NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap();
    let year_end = chrono::NaiveDate::from_ymd_opt(today.year(), 12, 31).unwrap();

    {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        seed_fixture(conn, &[("cash", "cash"), ("sales", "sales")]).await;
        let fy = FiscalYearActiveModel {
            id: Set(fy_id),
            name: Set(today.year().to_string()),
            start_date: Set(year_start),
            end_date: Set(year_end),
            is_closed: Set(false),
            closing_entry_id: Set(None),
            closed_at: Set(None),
            closed_by: Set(None),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };
        fy.insert(conn).await.unwrap();
    }

    let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
    let (locked_tx, locked_rx) = tokio::sync::oneshot::channel::<()>();

    let closer_state = test_db.clone();
    let closer = tokio::spawn(async move {
        // `with_tx` takes an `Fn` (it may retry), so the one-shot channel ends live in shared
        // `Arc<Mutex<Option<_>>>` slots that the first attempt takes; the guard is dropped before
        // every `.await`, keeping the future `Send`.
        let locked_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(locked_tx)));
        let release_rx = std::sync::Arc::new(std::sync::Mutex::new(Some(release_rx)));
        let result: Result<(), AppError> = with_tx(&closer_state.state, TxOpts::default(), move |txn, _cx| {
            let locked_tx = locked_tx.clone();
            let release_rx = release_rx.clone();
            Box::pin(async move {
                accounting_app_lib::shared::ledger::lock_fiscal_year_exclusive(txn, fy_id).await?;
                let tx = locked_tx.lock().unwrap().take();
                if let Some(tx) = tx {
                    let _ = tx.send(());
                }
                let rx = release_rx.lock().unwrap().take();
                if let Some(rx) = rx {
                    let _ = rx.await;
                }
                use sea_orm::{ActiveModelTrait, EntityTrait, Set};
                let mut model: FiscalYearActiveModel = accounting_app_lib::entities::org::fiscal_years::Entity::find_by_id(fy_id)
                    .one(txn)
                    .await
                    .map_err(TxError::from)?
                    .unwrap()
                    .into();
                model.is_closed = Set(true);
                model.update(txn).await.map_err(TxError::from)?;
                Ok(())
            }) as BoxFuture<'_, Result<(), TxError>>
        })
        .await;
        result.unwrap();
    });

    tokio::time::timeout(std::time::Duration::from_secs(10), locked_rx).await.expect("closer never took the X lock").unwrap();

    let poster_state = test_db.clone();
    let mid_year_date = chrono::NaiveDate::from_ymd_opt(today.year(), 6, 15).unwrap();
    let poster = tokio::spawn(async move {
        with_tx(&poster_state.state, TxOpts::default(), move |txn, cx| {
            Box::pin(async move {
                let req = PostJournal::new(
                    mid_year_date,
                    "قيد أثناء الإغلاق",
                    JournalEntryType::Manual,
                    vec![
                        PostingLine::debit(AccountRef::Role(cash_role()), dec!(1)),
                        PostingLine::credit(AccountRef::Role(sales_role()), dec!(1)),
                    ],
                );
                post(txn, cx, req).await
            }) as BoxFuture<'_, Result<_, TxError>>
        })
        .await
    });

    // Give the poster a moment to actually reach and block on the shared lock before releasing
    // the closer — a fixed short sleep is acceptable here since the assertion itself is
    // timeout-guarded either way and doesn't depend on this delay for correctness.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let _ = release_tx.send(());

    tokio::time::timeout(std::time::Duration::from_secs(10), closer).await.expect("closer timed out").unwrap();
    let poster_result: Result<JournalEntry, AppError> =
        tokio::time::timeout(std::time::Duration::from_secs(10), poster).await.expect("poster timed out").unwrap();

    let err = poster_result.unwrap_err();
    assert!(err.to_string().contains("مقفلة"), "poster must be refused once the year is closed, got: {err}");
}
