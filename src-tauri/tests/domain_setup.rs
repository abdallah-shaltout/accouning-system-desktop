//! DB-backed tests for the `setup` domain (03-domains/02-setup.md §8a). Written now, run in the
//! deferred time-boxed test pass (per-implementer hard rule: never run cargo from this agent).
//! Needs `EQUAL_TEST_DATABASE_URL` — see `tests/support/mod.rs`.
//!
//! Device (§3.1) commands are Windows-only bodies; this file's `device_*` tests only exercise the
//! non-Windows `FORBIDDEN` stub path (real provision/pair runs are in the final testing plan, per
//! the entry file's §8a note).

use crate::support;

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::tx::{with_read, with_tx, BoxFuture, TxOpts, TxResult};
use accounting_app_lib::domains::setup::dto::{
    CloseTarget, CountryTaxInput, ExtraCurrency, OpeningCashLine, OpeningEntryInput, OpeningStockLine, PartyKindWire, PartyOpeningInput, PostingSide,
    WizardBranchInput, WizardPaymentMethodInput,
};
use accounting_app_lib::domains::setup::service;
use accounting_app_lib::domains::settings::dto::{PaymentMethodAccountRole, PaymentMethodType};
use accounting_app_lib::shared::activity::undo::{UndoRegistry, UndoRequest};
use accounting_app_lib::shared::invariants;
use accounting_app_lib::utils::id::Id;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, PaginatorTrait, QueryFilter, Set};
use std::sync::Arc;
use support::TestDb;

/// Signs in as the shell's own seeded `admin` user (a real `users` row — every audit/activity row
/// references its actor through `fk_audit_user_id`), with `role` for the session and the shell's
/// main branch as home — the same identity the wizard's bootstrap session runs under.
async fn log_in(db: &TestDb, role: Role) -> Id {
    use accounting_app_lib::entities::org::users::{Column as UserColumn, Entity as UserEntity};
    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    let admin = UserEntity::find().filter(UserColumn::Username.eq("admin")).one(&conn).await.unwrap().expect("seed_shell must run before log_in");
    let home_branch_id = default_branch_id(db).await;
    let user = AuthenticatedUser {
        id: admin.id,
        username: admin.username.clone(),
        role,
        home_branch_id,
        allowed_branches: vec![],
        price_list_id: None,
        max_discount: None,
    };
    *db.state.session.write().unwrap() = Some(user);
    admin.id
}

/// Seeds the empty-company shell via the real `shell::seed_company_shell` (not a hand-built
/// fixture) — every other test in this file builds on top of it, exactly like
/// `get_onboarding_progress`'s command body does on a fresh DB.
async fn seed_shell(db: &TestDb) {
    with_tx(&db.state, TxOpts { require_user: false }, |tx, cx| {
        Box::pin(async move { service::shell::seed_company_shell(tx, cx).await }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("seed_company_shell must succeed");
}

/// Marks `settings.onboarding` as "wizard in progress" (no `finished_at`) — the real wizard's state
/// the instant it seeds the shell and before it has posted+closed opening balances. A test that
/// exercises one wizard step in isolation (e.g. `post_opening_stock`/`post_party_opening` alone,
/// without also calling `post_opening_balances`/`reclose_opening_balance_equity` to close 3900) needs
/// this so `shared::invariants::check_opening_balance_equity` (ACC-0023) doesn't enforce 3900 = 0
/// mid-wizard — `seed_company_shell` itself leaves `onboarding: None`, which the check (correctly)
/// reads as "no wizard ever ran" and enforces immediately, per its own doc comment.
async fn mark_onboarding_in_progress(db: &TestDb) {
    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    conn.execute(sea_orm::Statement::from_sql_and_values(
        conn.get_database_backend(),
        "UPDATE settings SET onboarding = ?",
        [serde_json::json!({ "goLiveDate": "2026-01-01" }).to_string().into()],
    ))
    .await
    .expect("marking onboarding in progress must succeed");
}

async fn default_branch_id(db: &TestDb) -> Id {
    use accounting_app_lib::entities::org::settings::Entity as SettingsEntity;
    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    SettingsEntity::find().one(&conn).await.unwrap().unwrap().default_branch_id
}

async fn account_id_by_code(db: &TestDb, code: &str) -> Id {
    use accounting_app_lib::entities::org::accounts::{Column as AccountColumn, Entity as AccountEntity};
    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    AccountEntity::find().filter(AccountColumn::Code.eq(code)).one(&conn).await.unwrap().unwrap_or_else(|| panic!("account {code} not seeded")).id
}

/// Minimal live customer/supplier — just enough for `party_opening` tests (D-9's `NOT_FOUND` path
/// needs a row that does NOT match, so this also proves the kind filter works).
async fn seed_party(db: &TestDb, kind: &str, name: &str) -> Id {
    use accounting_app_lib::entities::parties::parties::ActiveModel as PartyActiveModel;
    let id = Id::new();
    with_tx(&db.state, TxOpts { require_user: false }, move |tx: &DatabaseTransaction, _cx| {
        let name = name.to_string();
        let kind = kind.to_string();
        Box::pin(async move {
            let now = chrono::Utc::now();
            let model = PartyActiveModel {
                id: Set(id),
                kind: Set(kind),
                r#type: Set("individual".to_string()),
                name: Set(name.clone()),
                name_en: Set(None),
                code: Set(format!("P-{id}")),
                group_id: Set(None),
                tags: Set(None),
                active: Set(true),
                phone: Set(None),
                email: Set(None),
                contacts: Set(None),
                address: Set(None),
                national_address: Set(None),
                structured_address: Set(None),
                vat_number: Set(None),
                cr_number: Set(None),
                national_id: Set(None),
                currency: Set(None),
                price_list_id: Set(None),
                payment_terms_days: Set(None),
                salesperson_id: Set(None),
                branch_id: Set(None),
                bank: Set(None),
                opening_balance: Set(None),
                notes: Set(None),
                linked_party_id: Set(None),
                credit_limit: Set(None),
                contact_person: Set(None),
                default_expense_account_id: Set(None),
                search_normalized: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                deleted_at: Set(None),
                sync_status: Set("local".to_string()),
            };
            model.insert(tx).await?;
            Ok(())
        })
    })
    .await
    .expect("seed_party must succeed");
    id
}

// --- Shell (§3.3) -----------------------------------------------------------------------------

#[tokio::test]
async fn shell_seeds_expected_rows_exactly_once() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;

    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();

    use accounting_app_lib::entities::org::{branches, fiscal_years, payment_methods, settings, taxes, users};

    let tax_rows = taxes::Entity::find().all(&conn).await.unwrap();
    assert_eq!(tax_rows.len(), 2, "exactly 2 taxes at 14% (EG)");
    for t in &tax_rows {
        assert_eq!(t.rate, dec!(14));
    }
    assert!(tax_rows.iter().any(|t| t.account_role.as_deref() == Some("vatOutput") && t.name.contains("مبيعات")));
    assert!(tax_rows.iter().any(|t| t.account_role.as_deref() == Some("vatInput") && t.name.contains("مشتريات")));

    let methods = payment_methods::Entity::find().all(&conn).await.unwrap();
    assert_eq!(methods.len(), 2, "2 seeded payment methods");
    assert!(methods.iter().any(|m| m.name == "نقداً" && m.sort_order == 1));
    assert!(methods.iter().any(|m| m.name == "آجل" && m.sort_order == 2));

    let branch = branches::Entity::find().one(&conn).await.unwrap().expect("MAIN branch");
    assert_eq!(branch.code, "MAIN");
    assert!(branch.cost_center_id.is_some(), "branch's own cost_center_id set after the two-step wiring");
    assert!(branch.cash_account_id.is_some());

    let cash_account_id = account_id_by_code(&db, "1110").await;
    assert_eq!(branch.cash_account_id, Some(cash_account_id));

    let fys = fiscal_years::Entity::find().all(&conn).await.unwrap();
    assert_eq!(fys.len(), 2, "[y-1 closed, y open]");
    assert_eq!(fys.iter().filter(|f| f.is_closed).count(), 1);
    assert_eq!(fys.iter().filter(|f| !f.is_closed).count(), 1);

    let settings_row = settings::Entity::find().one(&conn).await.unwrap().expect("settings row");
    assert_eq!(settings_row.timezone.as_deref(), Some("Africa/Cairo"));
    assert_eq!(settings_row.currency, "EGP");
    assert_eq!(settings_row.country.as_deref(), Some("EG"));
    assert!(settings_row.prices_include_tax);

    let admin = users::Entity::find().one(&conn).await.unwrap().expect("admin user");
    assert_eq!(admin.username, "admin");
    assert_eq!(admin.role, "admin");

    use accounting_app_lib::entities::org::credentials;
    let creds = credentials::Entity::find_by_id(admin.id).one(&conn).await.unwrap().expect("admin credentials");
    assert!(accounting_app_lib::core::auth::verify_password("admin123", &creds.password_hash).unwrap_or(false), "admin123 must verify");

    // Called twice: no second seed (idempotent).
    seed_shell(&db).await;
    let admin_count = users::Entity::find().count(&conn).await.unwrap();
    assert_eq!(admin_count, 1, "seeding twice must not duplicate the admin user");
    db.finish().await;
}

#[tokio::test]
async fn bootstrap_session_set_then_cleared_by_finish() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;

    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    assert!(db.state.session.read().unwrap().is_none());

    service::session::ensure_bootstrap_session(&db.state, &conn).await.expect("ensure_bootstrap_session must succeed");
    assert!(db.state.session.read().unwrap().is_some(), "bootstrap session must be set once a shell + admin exist");

    with_tx(&db.state, TxOpts::default(), |tx, cx| Box::pin(async move { service::progress::finish(tx, cx).await }) as BoxFuture<'_, TxResult<()>>)
        .await
        .expect("finish must succeed");
    service::progress::clear_bootstrap_session_after_finish(&db.state);

    assert!(db.state.session.read().unwrap().is_none(), "finish_onboarding must clear the bootstrap session");
    db.finish().await;
}

// --- coa (build_accounts) -----------------------------------------------------------------------

#[tokio::test]
async fn coa_build_accounts_sa_and_pharmacy_addons() {
    use accounting_app_lib::domains::setup::service::coa::{build_accounts, AccountTemplateKind};

    for template in [AccountTemplateKind::Basic, AccountTemplateKind::Standard, AccountTemplateKind::Detailed] {
        let eg = build_accounts(template, Some("EG"), None);
        let sa = build_accounts(template, Some("SA"), None);
        assert!(!eg.iter().any(|r| r.code == "6910"), "no zakat account for EG");
        assert!(sa.iter().any(|r| r.code == "6910"), "zakat account appended for SA");

        let pharmacy = build_accounts(template, Some("EG"), Some("pharmacy"));
        assert!(pharmacy.iter().any(|r| r.code == "4120"), "pharmacy addon appended");

        // Every non-root row's parent code (when present) resolves to a code that exists.
        let codes: std::collections::HashSet<&str> = eg.iter().map(|r| r.code.as_str()).collect();
        for r in &eg {
            if let Some(pc) = &r.parent_code {
                assert!(codes.contains(pc.as_str()), "{}'s parent {} must exist", r.code, pc);
            }
        }
    }
}

// --- country tax --------------------------------------------------------------------------------

#[tokio::test]
async fn country_tax_eg_to_sa_switches_currency_rate_and_timezone() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    log_in(&db, Role::Admin).await;
    let registry = db.state.undo.clone();

    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let input = CountryTaxInput {
                country: "SA".to_string(),
                currency: "SAR".to_string(),
                vat_registered: Some(true),
                prices_include_tax: true,
                extra_currencies: vec![ExtraCurrency { code: "USD".to_string(), rate: dec!(3.75) }],
            };
            service::country_tax::apply(tx, cx, &registry, input).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("apply must succeed");

    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    use accounting_app_lib::entities::org::{currencies, settings, taxes};
    let settings_row = settings::Entity::find().one(&conn).await.unwrap().unwrap();
    assert_eq!(settings_row.currency, "SAR");
    assert_eq!(settings_row.timezone.as_deref(), Some("Asia/Riyadh"));
    assert_eq!(settings_row.country.as_deref(), Some("SA"));
    assert!(settings_row.features.as_ref().and_then(|f| f.currencies).unwrap_or(false));

    let tax_rows = taxes::Entity::find().all(&conn).await.unwrap();
    for t in &tax_rows {
        assert_eq!(t.rate, dec!(15), "SA VAT rate is 15%");
    }
    assert!(tax_rows.iter().any(|t| t.name.contains("مبيعات")));

    let usd = currencies::Entity::find_by_id("USD".to_string()).one(&conn).await.unwrap();
    assert!(usd.is_some(), "extra currency created once");

    // Adding the same code again is a no-op (exact match check).
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let input = CountryTaxInput {
                country: "SA".to_string(),
                currency: "SAR".to_string(),
                vat_registered: Some(true),
                prices_include_tax: true,
                extra_currencies: vec![ExtraCurrency { code: "USD".to_string(), rate: dec!(3.8) }],
            };
            service::country_tax::apply(tx, cx, &registry, input).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("second apply must succeed");
    let usd_count = currencies::Entity::find_by_id("USD".to_string()).count(&conn).await.unwrap();
    assert_eq!(usd_count, 1, "existing extra currency is never re-created");
    db.finish().await;
}

#[tokio::test]
async fn country_tax_locked_after_a_posted_journal_entry() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    log_in(&db, Role::Admin).await;
    let registry = db.state.undo.clone();

    post_trivial_journal_entry(&db).await;

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let input = CountryTaxInput { country: "SA".to_string(), currency: "SAR".to_string(), vat_registered: Some(true), prices_include_tax: true, extra_currencies: vec![] };
            service::country_tax::apply(tx, cx, &registry, input).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "لا يمكن تغيير الدولة أو العملة الأساسية بعد أول ترحيل");
    db.finish().await;
}

/// Posts one trivial manual journal entry through the real posting path — used only to flip the
/// various "already past go-live" guards (`is_base_currency_locked`, the fiscal-year/CoA/payment
/// method "any journal entry" checks all key off `journal_entries` being non-empty).
async fn post_trivial_journal_entry(db: &TestDb) {
    use accounting_app_lib::entities::journal::journal_entries::JournalEntryType;
    use accounting_app_lib::shared::ledger::accounts::SystemRole;
    use accounting_app_lib::shared::ledger::post::{AccountRef, PostJournal, PostingLine};
    use accounting_app_lib::utils::dates::DocDate;

    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            accounting_app_lib::shared::ledger::post::post(
                tx,
                cx,
                PostJournal {
                    date: DocDate::from(chrono::Utc::now().date_naive()),
                    description: "قيد اختبار".to_string(),
                    entry_type: JournalEntryType::Manual,
                    source: None,
                    lines: vec![
                        PostingLine::debit(AccountRef::Role(SystemRole::Cash), dec!(10)),
                        PostingLine::credit(AccountRef::Role(SystemRole::Capital), dec!(10)),
                    ],
                    allow_closed_period: true,
                    attachment_ids: Vec::new(),
                    template_id: None,
                },
            )
            .await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::entities::journal::journal_entries::Model>>
    })
    .await
    .expect("trivial journal entry must post");
}

// --- fiscal year -----------------------------------------------------------------------------

#[tokio::test]
async fn fiscal_year_start_jan_1_and_jul_1_and_feb_30_normalization() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    log_in(&db, Role::Admin).await;

    let go_live = chrono::NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
    let fy = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move { service::fiscal_year::apply(tx, cx, 1, 1, go_live).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::accounting::dto::FiscalYear>>
    })
    .await
    .expect("apply must succeed");
    assert_eq!(fy.start_date, "2026-01-01");
    assert_eq!(fy.end_date, "2026-12-31");

    let first_id = fy.id.clone();

    let fy2 = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move { service::fiscal_year::apply(tx, cx, 7, 1, go_live).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::accounting::dto::FiscalYear>>
    })
    .await
    .expect("second apply must succeed");
    assert_eq!(fy2.start_date, "2025-07-01");
    assert_eq!(fy2.end_date, "2026-06-30");
    assert_eq!(fy2.id, first_id, "the first row's id is kept across a wholesale replace");

    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    use accounting_app_lib::entities::org::fiscal_years::Entity as FyEntity;
    let count = FyEntity::find().count(&conn).await.unwrap();
    assert_eq!(count, 1, "every other row deleted, exactly one remains");
    db.finish().await;
}

#[tokio::test]
async fn fiscal_year_locked_after_a_posted_journal_entry() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    log_in(&db, Role::Admin).await;
    post_trivial_journal_entry(&db).await;

    let go_live = chrono::NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
    let err = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move { service::fiscal_year::apply(tx, cx, 1, 1, go_live).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::accounting::dto::FiscalYear>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "لا يمكن تغيير السنة المالية بعد بدء الترحيل");
    db.finish().await;
}

// --- branches --------------------------------------------------------------------------------

#[tokio::test]
async fn branches_rename_main_and_create_two_more() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    log_in(&db, Role::Admin).await;
    let registry = db.state.undo.clone();

    let branches = vec![
        WizardBranchInput { name: "الفرع الرئيسي المعدل".to_string(), code: "main2".to_string(), address: None },
        WizardBranchInput { name: "فرع 1".to_string(), code: "b1".to_string(), address: None },
        WizardBranchInput { name: "فرع 2".to_string(), code: "b2".to_string(), address: None },
    ];

    let result = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let branches = branches.clone();
        Box::pin(async move { service::branches::apply(tx, cx, &registry, branches).await }) as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::settings::dto::Branch>>>
    })
    .await
    .expect("apply must succeed");

    assert_eq!(result.len(), 3);
    assert_eq!(result[0].code, "MAIN2", "code upper-cased");
    assert_eq!(result[0].name, "الفرع الرئيسي المعدل");

    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    use accounting_app_lib::entities::org::branches::Entity as BranchEntity;
    let count = BranchEntity::find().count(&conn).await.unwrap();
    assert_eq!(count, 3);

    let settings_row = accounting_app_lib::entities::org::settings::Entity::find().one(&conn).await.unwrap().unwrap();
    assert!(settings_row.features.as_ref().and_then(|f| f.branches).unwrap_or(false));
    db.finish().await;
}

#[tokio::test]
async fn branches_duplicate_code_and_empty_list_texts() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    log_in(&db, Role::Admin).await;
    let registry = db.state.undo.clone();

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::branches::apply(tx, cx, &registry, vec![]).await }) as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::settings::dto::Branch>>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "أضف فرعاً واحداً على الأقل");

    let branches = vec![
        WizardBranchInput { name: "رئيسي".to_string(), code: "MAIN".to_string(), address: None },
        WizardBranchInput { name: "فرع مكرر".to_string(), code: "main".to_string(), address: None },
    ];
    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let branches = branches.clone();
        Box::pin(async move { service::branches::apply(tx, cx, &registry, branches).await }) as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::settings::dto::Branch>>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "رمز الفرع \"main\" مستخدم بالفعل");
    db.finish().await;
}

// --- coa apply -----------------------------------------------------------------------------------

#[tokio::test]
async fn coa_apply_keeps_ids_of_shared_codes_and_soft_deletes_the_rest() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    log_in(&db, Role::Admin).await;

    let cash_id_before = account_id_by_code(&db, "1110").await;

    let accounts = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move { service::coa::apply(tx, cx, service::coa::AccountTemplateKind::Basic, Some("EG"), None).await })
            as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::accounting::dto::Account>>>
    })
    .await
    .expect("apply must succeed");

    let cash_id_after = account_id_by_code(&db, "1110").await;
    assert_eq!(cash_id_before, cash_id_after, "shared code 1110 keeps its id (D-7)");

    // Order equals template order.
    let codes: Vec<&str> = accounts.iter().map(|a| a.code.as_str()).collect();
    let basic_codes: Vec<String> = service::coa::build_accounts(service::coa::AccountTemplateKind::Basic, Some("EG"), None).into_iter().map(|r| r.code).collect();
    assert_eq!(codes, basic_codes.iter().map(|s| s.as_str()).collect::<Vec<_>>());

    // A standard-only code (e.g. 1145, "بضاعة بالطريق بين الفروع") is soft-deleted, not in the live set.
    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    use accounting_app_lib::entities::org::accounts::{Column as AccountColumn, Entity as AccountEntity};
    use accounting_app_lib::entities::soft_delete::SoftDelete;
    let dangling = AccountEntity::find_live().filter(AccountColumn::Code.eq("1145")).one(&conn).await.unwrap();
    assert!(dangling.is_none(), "1145 (standard-only) must not exist in the basic template's live set");
    let soft_deleted = AccountEntity::find_including_deleted().filter(AccountColumn::Code.eq("1145")).one(&conn).await.unwrap().expect("1145 row is kept, soft-deleted");
    assert!(soft_deleted.deleted_at.is_some(), "1145 is soft-deleted, not hard-deleted");
    db.finish().await;
}

#[tokio::test]
async fn coa_apply_locked_after_a_posted_journal_entry() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    log_in(&db, Role::Admin).await;
    post_trivial_journal_entry(&db).await;

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move { service::coa::apply(tx, cx, service::coa::AccountTemplateKind::Basic, Some("EG"), None).await })
            as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::accounting::dto::Account>>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "لا يمكن تغيير شجرة الحسابات بعد بدء الترحيل");
    db.finish().await;
}

// --- payment methods -----------------------------------------------------------------------------

#[tokio::test]
async fn payment_methods_replace_and_locked_text() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    log_in(&db, Role::Admin).await;

    let methods = vec![
        WizardPaymentMethodInput { name: "نقدي".to_string(), kind: PaymentMethodType::Cash, account_role: PaymentMethodAccountRole::Cash, active: true },
        WizardPaymentMethodInput { name: "فيزا".to_string(), kind: PaymentMethodType::Card, account_role: PaymentMethodAccountRole::CardClearing, active: true },
    ];
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let methods = methods.clone();
        Box::pin(async move { service::payment_methods::apply(tx, cx, methods).await }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("apply must succeed");

    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    use accounting_app_lib::entities::org::payment_methods::Entity as PmEntity;
    use accounting_app_lib::entities::soft_delete::SoftDelete;
    let live = PmEntity::find_live().all(&conn).await.unwrap();
    assert_eq!(live.len(), 2, "the shell's 2 methods are soft-deleted, the wizard's 2 are the live set");
    assert!(live.iter().all(|m| m.can_delete), "wizard-created methods are deletable (Q-9)");

    post_trivial_journal_entry(&db).await;
    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let methods = methods.clone();
        Box::pin(async move { service::payment_methods::apply(tx, cx, methods).await }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "لا يمكن تغيير طرق الدفع بعد بدء الترحيل");
    db.finish().await;
}

// --- opening balances ------------------------------------------------------------------------

#[tokio::test]
async fn opening_balances_posts_balanced_entry_with_3900_line_and_closes_to_capital() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    log_in(&db, Role::Admin).await;
    let registry = db.state.undo.clone();

    let cash_id = account_id_by_code(&db, "1110").await;
    let input = OpeningEntryInput {
        date: "2026-01-01".to_string(),
        cash: vec![OpeningCashLine { account_id: cash_id, amount: dec!(1000), currency: None, amount_fc: None, rate: None }],
        customers: vec![],
        suppliers: vec![],
        other: vec![],
    };

    let result = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let input = input.clone();
        Box::pin(async move { service::opening::post_opening_balances(tx, cx, &registry, input, CloseTarget::Capital).await })
            as BoxFuture<'_, TxResult<accounting_app_lib::domains::setup::dto::PostOpeningBalancesResult>>
    })
    .await
    .expect("post_opening_balances must succeed");

    assert!(result.closing_entry_id.is_some(), "3900 net != 0, closing entry must exist");

    let net_after = with_read(&db.state, |conn| Box::pin(async move { service::opening::get_opening_balance_equity_net(conn).await }) as BoxFuture<'_, TxResult<rust_decimal::Decimal>>)
        .await
        .unwrap();
    assert_eq!(net_after, dec!(0), "3900 must be zero after closing");

    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    let settings_row = accounting_app_lib::entities::org::settings::Entity::find().one(&conn).await.unwrap().unwrap();
    let onboarding = settings_row.onboarding.expect("onboarding state");
    assert_eq!(onboarding.opening_entry_id, Some(result.opening_entry_id));
    assert_eq!(onboarding.closing_entry_id, result.closing_entry_id);

    // Reclose is idempotent (no-op) once already zero.
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::opening::reclose_opening_balance_equity(tx, cx, &registry, chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), CloseTarget::Capital).await })
            as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("reclose must be a safe no-op");

    let results = with_read(&db.state, |tx| Box::pin(async move { invariants::run_all(tx).await.map_err(accounting_app_lib::core::tx::TxError::App) })).await.unwrap();
    let failed: Vec<String> = results.iter().filter(|r| !r.passed).map(|r| format!("{}: {}", r.key, r.message)).collect();
    assert!(failed.is_empty(), "invariants must be green after opening balances: {failed:?}");
    db.finish().await;
}

#[tokio::test]
async fn opening_stock_per_branch_posts_movement_and_inventory_entry() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    mark_onboarding_in_progress(&db).await;
    log_in(&db, Role::Admin).await;
    let registry = db.state.undo.clone();

    let branch_id = default_branch_id(&db).await;
    let product_id = seed_product(&db, dec!(10), false).await;

    let lines = vec![OpeningStockLine { product_id, qty: dec!(5), unit_cost: dec!(12), batch_no: None, expiry_date: None }];

    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let lines = lines.clone();
        Box::pin(async move { service::opening::post_opening_stock(tx, cx, &registry, branch_id, chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), lines).await })
            as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("post_opening_stock must succeed");

    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    use accounting_app_lib::entities::journal::journal_entries::{Column as EntryColumn, Entity as EntryEntity};
    let entry = EntryEntity::find().filter(EntryColumn::SourceKind.eq("opening")).filter(EntryColumn::SourceNumber.eq("OPENING-STOCK")).one(&conn).await.unwrap();
    assert!(entry.is_some(), "opening-stock journal entry must exist (total = 60 > 0)");
    db.finish().await;
}

#[tokio::test]
async fn opening_stock_unknown_branch_is_not_found() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    log_in(&db, Role::Admin).await;
    let registry = db.state.undo.clone();

    let product_id = seed_product(&db, dec!(10), false).await;
    let lines = vec![OpeningStockLine { product_id, qty: dec!(5), unit_cost: dec!(12), batch_no: None, expiry_date: None }];

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let lines = lines.clone();
        Box::pin(async move { service::opening::post_opening_stock(tx, cx, &registry, Id::new(), chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), lines).await })
            as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "الفرع غير موجود");
    db.finish().await;
}

/// A minimal live product — `track_batches` optional (used by the batch-receiving opening-stock test).
async fn seed_product(db: &TestDb, cost_price: rust_decimal::Decimal, track_batches: bool) -> Id {
    use accounting_app_lib::entities::catalog::products::ActiveModel as ProductActiveModel;
    let id = Id::new();
    with_tx(&db.state, TxOpts { require_user: false }, move |tx: &DatabaseTransaction, _cx| {
        Box::pin(async move {
            let now = chrono::Utc::now();
            let model = ProductActiveModel {
                id: Set(id),
                name: Set("منتج اختبار".to_string()),
                name_en: Set(None),
                sku: Set(format!("SKU-{id}")),
                barcode: Set(None),
                category_id: Set(None),
                unit_id: Set(None),
                r#type: Set("product".to_string()),
                stock_mode: Set(None),
                cost_price: Set(cost_price),
                price: Set(cost_price * dec!(2)),
                stock_qty: Set(dec!(0)),
                min_stock: Set(None),
                active: Set(true),
                image: Set(None),
                purchase_account_id: Set(None),
                stock_value: Set(dec!(0)),
                stock_by_branch: Set(None),
                brand: Set(None),
                tags: Set(None),
                image_ids: Set(None),
                description: Set(None),
                units: Set(None),
                unit_prices: Set(None),
                min_price: Set(None),
                sale_tax_id: Set(None),
                purchase_tax_id: Set(None),
                revenue_account_id: Set(None),
                cogs_account_id: Set(None),
                allow_negative_stock: Set(None),
                shelf_location: Set(None),
                preferred_supplier_id: Set(None),
                reorder_qty: Set(None),
                track_batches: Set(Some(track_batches)),
                expiry_alert_days: Set(None),
                warranty_months: Set(None),
                warranty_provider: Set(None),
                weight: Set(None),
                custom_fields: Set(None),
                search_normalized: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                deleted_at: Set(None),
                sync_status: Set("local".to_string()),
                sku_live: Set(None),
                barcode_live: sea_orm::ActiveValue::NotSet,
            };
            model.insert(tx).await?;
            Ok(())
        })
    })
    .await
    .expect("seed_product must succeed");
    id
}

// --- party opening ---------------------------------------------------------------------------

#[tokio::test]
async fn party_opening_before_and_after_go_live_counter_account() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    log_in(&db, Role::Admin).await;
    let registry = db.state.undo.clone();

    // Set a go-live date via `apply_fiscal_year` so `party_opening` has something to compare against.
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move { service::fiscal_year::apply(tx, cx, 1, 1, chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()).await })
            as BoxFuture<'_, TxResult<accounting_app_lib::domains::accounting::dto::FiscalYear>>
    })
    .await
    .expect("apply_fiscal_year must succeed");

    let customer_id = seed_party(&db, "customer", "عميل تجريبي").await;

    // Before go-live: counter is openingBalanceEquity (3900).
    let net_before = with_read(&db.state, |conn| Box::pin(async move { service::opening::get_opening_balance_equity_net(conn).await }) as BoxFuture<'_, TxResult<rust_decimal::Decimal>>)
        .await
        .unwrap();

    let entry_id = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let input = PartyOpeningInput { party_kind: PartyKindWire::Customer, party_id: customer_id, amount: dec!(500), side: PostingSide::Debit, as_of_date: "2026-01-01".to_string() };
            service::party_opening::post_party_opening(tx, cx, &registry, input).await
        }) as BoxFuture<'_, TxResult<Option<Id>>>
    })
    .await
    .expect("post_party_opening must succeed")
    .expect("non-zero amount must return an entry id");

    let net_after = with_read(&db.state, |conn| Box::pin(async move { service::opening::get_opening_balance_equity_net(conn).await }) as BoxFuture<'_, TxResult<rust_decimal::Decimal>>)
        .await
        .unwrap();
    // `openingBalanceEquityNet` is debit-positive (`opening.ts:41`): a customer opening debit of
    // 500 credits 3900 by 500, so its net moves by −500.
    assert_eq!(net_after - net_before, dec!(-500), "before go-live, the counter is 3900");

    // After go-live: counter is capital, 3900 must not move further.
    let net_before_2 = net_after;
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let input = PartyOpeningInput { party_kind: PartyKindWire::Customer, party_id: customer_id, amount: dec!(200), side: PostingSide::Debit, as_of_date: "2026-06-01".to_string() };
            service::party_opening::post_party_opening(tx, cx, &registry, input).await
        }) as BoxFuture<'_, TxResult<Option<Id>>>
    })
    .await
    .expect("post_party_opening (after go-live) must succeed");

    let net_after_2 = with_read(&db.state, |conn| Box::pin(async move { service::opening::get_opening_balance_equity_net(conn).await }) as BoxFuture<'_, TxResult<rust_decimal::Decimal>>)
        .await
        .unwrap();
    assert_eq!(net_after_2, net_before_2, "after go-live, 3900 stays put — the counter went to capital");

    let _ = entry_id;
    db.finish().await;
}

#[tokio::test]
async fn party_opening_zero_amount_is_none_and_unknown_party_is_not_found() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    log_in(&db, Role::Admin).await;
    let registry = db.state.undo.clone();

    let customer_id = seed_party(&db, "customer", "عميل").await;

    let zero = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let input = PartyOpeningInput { party_kind: PartyKindWire::Customer, party_id: customer_id, amount: dec!(0), side: PostingSide::Debit, as_of_date: "2026-01-01".to_string() };
            service::party_opening::post_party_opening(tx, cx, &registry, input).await
        }) as BoxFuture<'_, TxResult<Option<Id>>>
    })
    .await
    .expect("must succeed");
    assert!(zero.is_none(), "amount == 0 -> None, no write");

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let input = PartyOpeningInput { party_kind: PartyKindWire::Customer, party_id: Id::new(), amount: dec!(100), side: PostingSide::Debit, as_of_date: "2026-01-01".to_string() };
            service::party_opening::post_party_opening(tx, cx, &registry, input).await
        }) as BoxFuture<'_, TxResult<Option<Id>>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "العميل غير موجود");

    // Wrong kind (a supplier id passed as a customer) also NOT_FOUND (D-9's kind filter).
    let supplier_id = seed_party(&db, "supplier", "مورد").await;
    let err2 = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let input = PartyOpeningInput { party_kind: PartyKindWire::Customer, party_id: supplier_id, amount: dec!(100), side: PostingSide::Debit, as_of_date: "2026-01-01".to_string() };
            service::party_opening::post_party_opening(tx, cx, &registry, input).await
        }) as BoxFuture<'_, TxResult<Option<Id>>>
    })
    .await
    .unwrap_err();
    assert_eq!(err2.to_string(), "العميل غير موجود");
    db.finish().await;
}

#[tokio::test]
async fn party_opening_audit_row_is_undoable_with_correct_action_type() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    mark_onboarding_in_progress(&db).await;
    log_in(&db, Role::Admin).await;
    let registry = db.state.undo.clone();

    let customer_id = seed_party(&db, "customer", "عميل").await;
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let input = PartyOpeningInput { party_kind: PartyKindWire::Customer, party_id: customer_id, amount: dec!(300), side: PostingSide::Debit, as_of_date: "2026-01-01".to_string() };
            service::party_opening::post_party_opening(tx, cx, &registry, input).await
        }) as BoxFuture<'_, TxResult<Option<Id>>>
    })
    .await
    .expect("must succeed");

    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    use accounting_app_lib::entities::platform::audit::Entity as AuditEntity;
    let audit_row = AuditEntity::find().one(&conn).await.unwrap().expect("one audit row written");
    assert!(audit_row.is_undoable);
    assert_eq!(audit_row.action_type.as_deref(), Some("setup.postPartyOpening"));
    db.finish().await;
}

// --- reverse_party_opening --------------------------------------------------------------------

#[tokio::test]
async fn reverse_party_opening_missing_not_opening_allocated_and_success() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    log_in(&db, Role::Admin).await;
    let registry = db.state.undo.clone();

    // Missing.
    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::party_opening::reverse_party_opening(tx, cx, &registry, Id::new(), true, None).await }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "القيد غير موجود");

    // Not a party opening: a plain manual entry.
    let manual_entry = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            use accounting_app_lib::entities::journal::journal_entries::JournalEntryType;
            use accounting_app_lib::shared::ledger::accounts::SystemRole;
            use accounting_app_lib::shared::ledger::post::{AccountRef, PostJournal, PostingLine};
            use accounting_app_lib::utils::dates::DocDate;
            accounting_app_lib::shared::ledger::post::post(
                tx,
                cx,
                PostJournal {
                    date: DocDate::from(chrono::Utc::now().date_naive()),
                    description: "قيد يدوي".to_string(),
                    entry_type: JournalEntryType::Manual,
                    source: None,
                    lines: vec![
                        PostingLine::debit(AccountRef::Role(SystemRole::Cash), dec!(10)),
                        PostingLine::credit(AccountRef::Role(SystemRole::Capital), dec!(10)),
                    ],
                    allow_closed_period: true,
                    attachment_ids: Vec::new(),
                    template_id: None,
                },
            )
            .await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::entities::journal::journal_entries::Model>>
    })
    .await
    .expect("manual entry must post");

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::party_opening::reverse_party_opening(tx, cx, &registry, manual_entry.id, true, None).await }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "هذا القيد ليس رصيداً افتتاحياً لطرف");

    // Success path: a real party opening entry.
    let customer_id = seed_party(&db, "customer", "عميل").await;
    let entry_id = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let input = PartyOpeningInput { party_kind: PartyKindWire::Customer, party_id: customer_id, amount: dec!(400), side: PostingSide::Debit, as_of_date: "2026-01-01".to_string() };
            service::party_opening::post_party_opening(tx, cx, &registry, input).await
        }) as BoxFuture<'_, TxResult<Option<Id>>>
    })
    .await
    .expect("must succeed")
    .expect("non-zero amount");

    // Allocated: insert a payment + allocation targeting this entry as 'opening'.
    seed_payment_allocation(&db, entry_id, customer_id).await;
    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::party_opening::reverse_party_opening(tx, cx, &registry, entry_id, true, None).await }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "لا يمكن التراجع عن رصيد افتتاحي له تخصيص دفعة — أزل التخصيص أولاً");

    // Remove the allocation, then a real reversal succeeds.
    remove_all_allocations(&db).await;
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::party_opening::reverse_party_opening(tx, cx, &registry, entry_id, true, None).await }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .expect("reversal must succeed once unallocated");

    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    use accounting_app_lib::entities::journal::journal_entries::Entity as EntryEntity;
    let original = EntryEntity::find_by_id(entry_id).one(&conn).await.unwrap().unwrap();
    assert!(original.reversed, "the original entry must be flagged reversed");
    db.finish().await;
}

/// `customer_id` must be a real customer: `payments (target_id, target_type)` is an FK to
/// `parties (id, kind)` (`fk_payments_target_party`).
async fn seed_payment_allocation(db: &TestDb, entry_id: Id, customer_id: Id) {
    use accounting_app_lib::entities::payments::payment_allocations::ActiveModel as AllocActiveModel;
    use accounting_app_lib::entities::payments::payments::ActiveModel as PaymentActiveModel;
    let payment_id = Id::new();
    with_tx(&db.state, TxOpts { require_user: false }, move |tx: &DatabaseTransaction, _cx| {
        Box::pin(async move {
            let now = chrono::Utc::now();
            let payment = PaymentActiveModel {
                id: Set(payment_id),
                number: Set("PAY-0001".to_string()),
                date_day: Set(now.date_naive()),
                date_instant: Set(Some(now)),
                r#type: Set(accounting_app_lib::entities::payments::payments::PaymentType::Received),
                target_type: Set(accounting_app_lib::entities::payments::payments::PaymentTargetType::Customer),
                target_id: Set(customer_id),
                target_ref: Set(None),
                target_ref_number: Set(None),
                amount: Set(dec!(400)),
                method: Set(accounting_app_lib::entities::payments::payments::PaymentMethodKind::Cash),
                note: Set(None),
                branch_id: Set(None),
                currency: Set(None),
                amount_fc: Set(None),
                rate: Set(None),
                fx_gain_loss: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                deleted_at: Set(None),
                sync_status: Set(accounting_app_lib::entities::payments::payments::SyncStatus::Local),
            };
            payment.insert(tx).await?;

            let alloc = AllocActiveModel {
                id: Set(Id::new()),
                payment_id: Set(payment_id),
                position: Set(0),
                target_kind: Set(accounting_app_lib::entities::payments::payment_allocations::PaymentAllocationTargetKind::Opening),
                target_id: Set(entry_id),
                target_number: Set("OPENING".to_string()),
                amount: Set(dec!(400)),
                date_day: Set(now.date_naive()),
                date_instant: Set(Some(now)),
                amount_fc: Set(None),
                fx_gain_loss: Set(None),
            };
            alloc.insert(tx).await?;
            Ok(())
        })
    })
    .await
    .expect("seed_payment_allocation must succeed");
}

async fn remove_all_allocations(db: &TestDb) {
    use accounting_app_lib::entities::payments::payment_allocations::Entity as AllocEntity;
    use accounting_app_lib::entities::payments::payments::Entity as PaymentEntity;
    with_tx(&db.state, TxOpts { require_user: false }, |tx: &DatabaseTransaction, _cx| {
        Box::pin(async move {
            AllocEntity::delete_many().exec(tx).await?;
            PaymentEntity::delete_many().exec(tx).await?;
            Ok(())
        })
    })
    .await
    .expect("remove_all_allocations must succeed");
}

// --- undo (registry) -------------------------------------------------------------------------

#[tokio::test]
async fn undo_reverses_the_party_opening_via_the_registry() {
    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    log_in(&db, Role::Admin).await;

    let mut registry = UndoRegistry::new();
    accounting_app_lib::domains::setup::register_undo(&mut registry);
    let registry = Arc::new(registry);

    let customer_id = seed_party(&db, "customer", "عميل").await;
    let entry_id = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let input = PartyOpeningInput { party_kind: PartyKindWire::Customer, party_id: customer_id, amount: dec!(250), side: PostingSide::Debit, as_of_date: "2026-01-01".to_string() };
            service::party_opening::post_party_opening(tx, cx, &registry, input).await
        }) as BoxFuture<'_, TxResult<Option<Id>>>
    })
    .await
    .expect("must succeed")
    .expect("non-zero amount");

    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    use accounting_app_lib::entities::platform::audit::Entity as AuditEntity;
    let audit_row = AuditEntity::find().one(&conn).await.unwrap().expect("audit row for the party opening");

    let comp_id = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let audit_id = audit_row.id;
        Box::pin(async move {
            accounting_app_lib::shared::activity::undo::undo(tx, cx, &registry, audit_id, UndoRequest { reason: "خطأ في الإدخال".to_string(), date: None }).await
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .expect("undo must succeed");

    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    use accounting_app_lib::entities::journal::journal_entries::Entity as EntryEntity;
    let original = EntryEntity::find_by_id(entry_id).one(&conn).await.unwrap().unwrap();
    assert!(original.reversed, "undo must have reversed the mirror entry");

    let original_audit = AuditEntity::find_by_id(audit_row.id).one(&conn).await.unwrap().unwrap();
    assert_eq!(original_audit.undone_by, Some(comp_id), "original audit row linked to its compensation");

    let comp_audit = AuditEntity::find_by_id(comp_id).one(&conn).await.unwrap().unwrap();
    assert_eq!(comp_audit.undo_of, Some(audit_row.id), "compensation audit row linked back");
    db.finish().await;
}

// --- device (non-Windows stub path) ------------------------------------------------------------
//
// `get_device_setup_state`/`provision_main`/`pair_terminal` all take `&tauri::AppHandle`, and no
// test harness in this repo constructs one outside a running Tauri app (`TestDb` deliberately
// stays app-handle-free — see `tests/support/mod.rs`). The non-Windows `FORBIDDEN` stub bodies
// (`service::device::unsupported()`) are therefore exercised only by real desktop runs in the
// final testing plan (entry file §8a's own note), not here. `pairing::parse_code`'s own validator
// is a pure function with no app handle dependency — it is covered by `infrastructure::database`'s
// own unit tests, not duplicated here.

/// ACC-0030: the wizard closes 3900, something moves it again (opening stock, a changed opening
/// entry), and the wizard re-closes it. The re-close is a separate posting, so it gets its own source
/// id instead of reusing `ONBOARDING_CLOSE_SOURCE_ID` — two active entries on one source broke
/// `one-active-entry`.
#[tokio::test]
async fn reclose_after_3900_moves_again_uses_its_own_source() {
    use accounting_app_lib::entities::journal::journal_entries;
    use accounting_app_lib::shared::ledger::accounts::SystemRole;
    use accounting_app_lib::shared::ledger::post::{self as ledger_post, AccountRef, PostJournal, PostingLine};
    use accounting_app_lib::utils::dates::DocDate;

    let db = TestDb::fresh().await;
    seed_shell(&db).await;
    log_in(&db, Role::Admin).await;
    let registry = db.state.undo.clone();

    let cash_id = account_id_by_code(&db, "1110").await;
    let input = OpeningEntryInput {
        date: "2026-01-01".to_string(),
        cash: vec![OpeningCashLine { account_id: cash_id, amount: dec!(1000), currency: None, amount_fc: None, rate: None }],
        customers: vec![],
        suppliers: vec![],
        other: vec![],
    };
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let input = input.clone();
        Box::pin(async move { service::opening::post_opening_balances(tx, cx, &registry, input, CloseTarget::Capital).await })
            as BoxFuture<'_, TxResult<accounting_app_lib::domains::setup::dto::PostOpeningBalancesResult>>
    })
    .await
    .expect("post_opening_balances must succeed");

    // Something credits 3900 again (what opening stock does): Dr cash 400 / Cr 3900 400.
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            ledger_post::post(
                tx,
                cx,
                PostJournal {
                    date: DocDate { day: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), instant: None },
                    description: "رصيد افتتاحي إضافي".to_string(),
                    entry_type: journal_entries::JournalEntryType::Opening,
                    source: None,
                    lines: vec![PostingLine::debit(AccountRef::Role(SystemRole::Cash), dec!(400)), PostingLine::credit(AccountRef::Role(SystemRole::OpeningBalanceEquity), dec!(400))],
                    allow_closed_period: true,
                    attachment_ids: Vec::new(),
                    template_id: None,
                },
            )
            .await?;
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("extra opening posting must succeed");

    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::opening::reclose_opening_balance_equity(tx, cx, &registry, chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), CloseTarget::Capital).await })
            as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("reclose must succeed");

    let conn = db.state.db.read().unwrap().as_ref().unwrap().connection.clone();
    let closes = journal_entries::Entity::find()
        .filter(journal_entries::Column::SourceKind.eq("opening"))
        .filter(journal_entries::Column::Type.eq(journal_entries::JournalEntryType::Closing))
        .all(&conn)
        .await
        .unwrap();
    assert_eq!(closes.len(), 2, "the close and the re-close");
    assert_ne!(closes[0].source_id, closes[1].source_id, "the re-close must not reuse the first close's source");

    let results = with_read(&db.state, |tx| Box::pin(async move { invariants::run_all(tx).await.map_err(accounting_app_lib::core::tx::TxError::App) })).await.unwrap();
    let failed: Vec<String> = results.iter().filter(|r| !r.passed).map(|r| format!("{}: {}", r.key, r.message)).collect();
    assert!(failed.is_empty(), "invariants must be green after the re-close: {failed:?}");
    db.finish().await;
}
