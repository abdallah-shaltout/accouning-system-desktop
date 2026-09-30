//! DB-backed tests for the `expenses` domain (03-domains/11-expenses.md §8a). Written now, run in
//! the deferred time-boxed test pass (per-implementer hard rule: never run cargo from this agent).
//! Needs `EQUAL_TEST_DATABASE_URL` — see `tests/support/mod.rs`.

use crate::support;

use std::sync::Arc;

use chrono::Datelike;

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::tx::{with_tx, BoxFuture, TxOpts};
use accounting_app_lib::domains::expenses::dto::{ExpenseCategoryInput, ExpenseFilter, ExpenseInput, ExpensePaidFrom, RecurringExpenseInput};
use accounting_app_lib::domains::expenses::service;
use accounting_app_lib::entities::org::{accounts, branches, fiscal_years, payment_methods, settings, taxes};
use accounting_app_lib::entities::parties::parties;
use accounting_app_lib::entities::values::{AccountingPolicy, PrinterMode, PrinterSettings};
use accounting_app_lib::shared::activity::undo::UndoRegistry;
use accounting_app_lib::shared::invariants;
use accounting_app_lib::utils::id::Id;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseTransaction, Set};
use support::TestDb;

/// Logs in an admin session whose `users` row exists (FK target of `journal_entries.created_by`,
/// `audit.user_id`, documents' `created_by`/`cashier_id`, …).
async fn log_in(test_db: &TestDb) -> Id {
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
    let connection = test_db.state.db.read().unwrap().as_ref().expect("test db connected").connection.clone();
    support::seed_user(&connection, user_id, "admin").await;
    *test_db.state.session.write().unwrap() = Some(user);
    user_id
}

/// Fixture: one branch + settings row, an open fiscal year covering today, the role accounts
/// `vatInput`/`payable`/`cash`, one plain expense account, a 15% active tax, a cash payment
/// method, and one supplier party.
struct Fixture {
    pub expense_account_id: Id,
    pub tax_id: Id,
    pub payment_method_id: Id,
    pub supplier_id: Id,
}

async fn seed_role_account<C: ConnectionTrait>(conn: &C, code: &str, role: &str) -> Id {
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
        kind: Set("ASSET".to_string()),
        subtype: Set("cash".to_string()),
        normal_side: Set("DEBIT".to_string()),
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
        name: Set("2026".to_string()),
        start_date: Set(today.with_day(1).unwrap().with_month(1).unwrap()),
        end_date: Set(today.with_day(31).unwrap_or(today).with_month(12).unwrap_or(today)),
        is_closed: Set(false),
        closing_entry_id: Set(None),
        closed_at: Set(None),
        closed_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    fiscal_year.insert(conn).await.unwrap();

    let _vat_input_id = seed_role_account(conn, "2110", "vatInput").await;
    let _payable_id = seed_role_account(conn, "2100", "payable").await;
    let _cash_role_id = seed_role_account(conn, "1110", "cash").await;

    // A plain (non-role) expense account — `save_expense_category`/`record_expense` resolve this
    // by explicit id (`AccountRef::Id`), not by role.
    let expense_account_id = Id::new();
    let expense_account = accounts::ActiveModel {
        code_live: sea_orm::ActiveValue::NotSet,
        id: Set(expense_account_id),
        code: Set("5100".to_string()),
        name: Set("مصروفات إيجار".to_string()),
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
    expense_account.insert(conn).await.unwrap();

    let tax_id = Id::new();
    let tax = taxes::ActiveModel {
        id: Set(tax_id),
        name: Set("ضريبة القيمة المضافة 15%".to_string()),
        rate: Set(dec!(15)),
        r#type: Set("INPUT".to_string()),
        is_default: Set(true),
        active: Set(true),
        category: Set("S".to_string()),
        direction: Set("purchase".to_string()),
        exemption_reason: Set(None),
        account_role: Set(Some("vatInput".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    tax.insert(conn).await.unwrap();

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

    let supplier_id = Id::new();
    let supplier = parties::ActiveModel {
        id: Set(supplier_id),
        kind: Set("supplier".to_string()),
        r#type: Set("company".to_string()),
        name: Set("مورد تجريبي".to_string()),
        name_en: Set(None),
        code: Set("S-0001".to_string()),
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
    supplier.insert(conn).await.unwrap();

    Fixture { expense_account_id, tax_id, payment_method_id, supplier_id }
}

fn method_paid_from(id: Id) -> ExpensePaidFrom {
    ExpensePaidFrom::Method { payment_method_id: id }
}

fn credit_paid_from(id: Id) -> ExpensePaidFrom {
    ExpensePaidFrom::Credit { supplier_id: id }
}

#[tokio::test]
async fn create_expense_cash_no_vat_posts_balanced_entry() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let (category_id, expense) = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let category = service::categories::save_expense_category(
                tx,
                cx,
                ExpenseCategoryInput { name: "إيجار".to_string(), icon: None, account_id: fixture.expense_account_id, default_tax_id: None, default_cost_center_id: None, active: true },
                None,
            )
            .await?;

            let input = ExpenseInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                category_id: category.id,
                amount: dec!(100),
                is_tax_invoice: false,
                tax_id: None,
                supplier_vat_number: None,
                supplier_invoice_no: None,
                cost_center_id: None,
                paid_from: method_paid_from(fixture.payment_method_id),
                description: None,
                attachment_ids: None,
                repeat_monthly: None,
                recurring_template_id: None,
            };
            let expense = service::expenses::create_expense(tx, cx, &undo, input).await?;
            Ok((category.id, expense))
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<(Id, accounting_app_lib::domains::expenses::dto::Expense)>>
    })
    .await
    .expect("create_expense must succeed");

    assert_eq!(expense.number, "EXP-000001");
    assert_eq!(expense.net_amount, dec!(100));
    assert_eq!(expense.tax_amount, Decimal::ZERO);
    assert_eq!(expense.category_id, category_id);

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let results = invariants::run_all(conn).await.expect("invariants must run");
    let failed: Vec<_> = results.iter().filter(|r| !r.passed).collect();
    assert!(failed.is_empty(), "invariants failed: {failed:?}");
}

#[tokio::test]
async fn create_expense_tax_invoice_splits_net_and_vat() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let expense = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let category = service::categories::save_expense_category(
                tx,
                cx,
                ExpenseCategoryInput { name: "صيانة".to_string(), icon: None, account_id: fixture.expense_account_id, default_tax_id: None, default_cost_center_id: None, active: true },
                None,
            )
            .await?;
            let input = ExpenseInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                category_id: category.id,
                amount: dec!(115),
                is_tax_invoice: true,
                tax_id: Some(fixture.tax_id),
                supplier_vat_number: None,
                supplier_invoice_no: None,
                cost_center_id: None,
                paid_from: method_paid_from(fixture.payment_method_id),
                description: None,
                attachment_ids: None,
                repeat_monthly: None,
                recurring_template_id: None,
            };
            service::expenses::create_expense(tx, cx, &undo, input).await
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<accounting_app_lib::domains::expenses::dto::Expense>>
    })
    .await
    .expect("create_expense must succeed");

    assert_eq!(expense.net_amount, dec!(100));
    assert_eq!(expense.tax_amount, dec!(15));
}

#[tokio::test]
async fn create_expense_credit_posts_to_payable_with_supplier_party() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let expense = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let category = service::categories::save_expense_category(
                tx,
                cx,
                ExpenseCategoryInput { name: "شحن".to_string(), icon: None, account_id: fixture.expense_account_id, default_tax_id: None, default_cost_center_id: None, active: true },
                None,
            )
            .await?;
            let input = ExpenseInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                category_id: category.id,
                amount: dec!(200),
                is_tax_invoice: false,
                tax_id: None,
                supplier_vat_number: None,
                supplier_invoice_no: None,
                cost_center_id: None,
                paid_from: credit_paid_from(fixture.supplier_id),
                description: None,
                attachment_ids: None,
                repeat_monthly: None,
                recurring_template_id: None,
            };
            service::expenses::create_expense(tx, cx, &undo, input).await
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<accounting_app_lib::domains::expenses::dto::Expense>>
    })
    .await
    .expect("create_expense must succeed");

    match expense.paid_from {
        ExpensePaidFrom::Credit { supplier_id } => assert!(!supplier_id.to_string().is_empty()),
        _ => panic!("expected a credit paid_from"),
    }
}

#[tokio::test]
async fn expense_category_validation_messages_in_order() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;

    let err = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            let _fixture = seed_fixture(tx).await;
            service::categories::save_expense_category(
                tx,
                cx,
                ExpenseCategoryInput { name: "   ".to_string(), icon: None, account_id: Id::new(), default_tax_id: None, default_cost_center_id: None, active: true },
                None,
            )
            .await
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<accounting_app_lib::domains::expenses::dto::ExpenseCategory>>
    })
    .await
    .expect_err("empty name must be refused");
    assert!(matches!(err, accounting_app_lib::core::error::AppError::Validation { message } if message == "الاسم مطلوب"));

    let err = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            let _fixture = seed_fixture(tx).await;
            service::categories::save_expense_category(
                tx,
                cx,
                ExpenseCategoryInput { name: "فئة".to_string(), icon: None, account_id: Id::new(), default_tax_id: None, default_cost_center_id: None, active: true },
                None,
            )
            .await
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<accounting_app_lib::domains::expenses::dto::ExpenseCategory>>
    })
    .await
    .expect_err("unknown account must be refused");
    assert!(matches!(err, accounting_app_lib::core::error::AppError::NotFound { message } if message == "الحساب غير موجود في شجرة الحسابات"));
}

#[tokio::test]
async fn expense_category_delete_protected_and_in_use_refusals() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let category_id = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let category = service::categories::save_expense_category(
                tx,
                cx,
                ExpenseCategoryInput { name: "إيجار".to_string(), icon: None, account_id: fixture.expense_account_id, default_tax_id: None, default_cost_center_id: None, active: true },
                None,
            )
            .await?;
            Ok((category.id, fixture))
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<(Id, Fixture)>>
    })
    .await
    .expect("setup must succeed");

    let (category_id, fixture) = category_id;

    // Mark it protected (can_delete = false) directly, then confirm FORBIDDEN.
    {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        use accounting_app_lib::entities::expenses::expense_categories::{ActiveModel as CategoryActiveModel, Entity as CategoryEntity};
        use sea_orm::EntityTrait;
        let existing = CategoryEntity::find_by_id(category_id).one(conn).await.unwrap().unwrap();
        let mut model: CategoryActiveModel = existing.into();
        model.can_delete = Set(false);
        model.update(conn).await.unwrap();
    }

    let err = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move { service::categories::delete_expense_category(tx, cx, category_id).await.map(|_| ()) }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>>
    })
    .await
    .expect_err("protected category delete must be refused");
    assert!(matches!(err, accounting_app_lib::core::error::AppError::Forbidden { .. }));

    // Un-protect, record an expense against it, then confirm CONFLICT on delete.
    {
        let db_guard = test_db.state.db.read().unwrap();
        let conn = &db_guard.as_ref().unwrap().connection;
        use accounting_app_lib::entities::expenses::expense_categories::{ActiveModel as CategoryActiveModel, Entity as CategoryEntity};
        use sea_orm::EntityTrait;
        let existing = CategoryEntity::find_by_id(category_id).one(conn).await.unwrap().unwrap();
        let mut model: CategoryActiveModel = existing.into();
        model.can_delete = Set(true);
        model.update(conn).await.unwrap();
    }

    let undo3 = undo.clone();
    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo3.clone();
        let fixture_method = fixture.payment_method_id;
        Box::pin(async move {
            let input = ExpenseInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                category_id,
                amount: dec!(50),
                is_tax_invoice: false,
                tax_id: None,
                supplier_vat_number: None,
                supplier_invoice_no: None,
                cost_center_id: None,
                paid_from: method_paid_from(fixture_method),
                description: None,
                attachment_ids: None,
                repeat_monthly: None,
                recurring_template_id: None,
            };
            service::expenses::create_expense(tx, cx, &undo, input).await
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<accounting_app_lib::domains::expenses::dto::Expense>>
    })
    .await
    .expect("recording the expense must succeed");

    let err = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move { service::categories::delete_expense_category(tx, cx, category_id).await }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>>
    })
    .await
    .expect_err("category with an expense must be refused");
    assert!(matches!(err, accounting_app_lib::core::error::AppError::Conflict { .. }));
}

#[tokio::test]
async fn list_expenses_filters_by_category_and_search() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let category = service::categories::save_expense_category(
                tx,
                cx,
                ExpenseCategoryInput { name: "إيجار".to_string(), icon: None, account_id: fixture.expense_account_id, default_tax_id: None, default_cost_center_id: None, active: true },
                None,
            )
            .await?;
            let input = ExpenseInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                category_id: category.id,
                amount: dec!(100),
                is_tax_invoice: false,
                tax_id: None,
                supplier_vat_number: None,
                supplier_invoice_no: None,
                cost_center_id: None,
                paid_from: method_paid_from(fixture.payment_method_id),
                description: None,
                attachment_ids: None,
                repeat_monthly: None,
                recurring_template_id: None,
            };
            service::expenses::create_expense(tx, cx, &undo, input).await?;

            let all = service::expenses::get_expenses(tx, None).await?;
            assert_eq!(all.len(), 1);

            let by_category = service::expenses::get_expenses(tx, Some(ExpenseFilter { category_id: Some(category.id), from: None, to: None, search: None })).await?;
            assert_eq!(by_category.len(), 1);

            let by_search = service::expenses::get_expenses(tx, Some(ExpenseFilter { category_id: None, from: None, to: None, search: Some("ايجار".to_string()) })).await?;
            assert_eq!(by_search.len(), 1, "Arabic-normalized search must match أ/ا variants");

            let get_unknown = service::expenses::get_expense(tx, Id::new()).await;
            assert!(get_unknown.is_err());

            Ok(())
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>>
    })
    .await
    .expect("list/search assertions must pass");
}

#[tokio::test]
async fn recurring_expense_crud_and_due_list_and_post_due() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let category = service::categories::save_expense_category(
                tx,
                cx,
                ExpenseCategoryInput { name: "اشتراكات".to_string(), icon: None, account_id: fixture.expense_account_id, default_tax_id: None, default_cost_center_id: None, active: true },
                None,
            )
            .await?;

            // Day out of range.
            let bad_day = service::recurring::save_recurring_expense(
                tx,
                cx,
                RecurringExpenseInput {
                    name: "اشتراك شهري".to_string(),
                    category_id: category.id,
                    amount: dec!(50),
                    is_tax_invoice: false,
                    tax_id: None,
                    paid_from: method_paid_from(fixture.payment_method_id),
                    description: None,
                    day: 29,
                    next_date: cx.clock.today().format("%Y-%m-%d").to_string(),
                    auto_post: false,
                    active: true,
                },
                None,
            )
            .await;
            assert!(bad_day.is_err());

            let today = cx.clock.today();
            let template = service::recurring::save_recurring_expense(
                tx,
                cx,
                RecurringExpenseInput {
                    name: "اشتراك شهري".to_string(),
                    category_id: category.id,
                    amount: dec!(50),
                    is_tax_invoice: false,
                    tax_id: None,
                    paid_from: method_paid_from(fixture.payment_method_id),
                    description: None,
                    day: 5,
                    next_date: today.format("%Y-%m-%d").to_string(),
                    auto_post: false,
                    active: true,
                },
                None,
            )
            .await?;

            let due = service::recurring::due_recurring_expenses(tx, today).await?;
            assert_eq!(due.len(), 1);

            let posted = service::recurring::post_due_recurring_expense(tx, cx, &undo, template.id).await?;
            assert!(posted.repeat_monthly);
            assert_eq!(posted.recurring_template_id, Some(template.id));

            let due_after = service::recurring::due_recurring_expenses(tx, today).await?;
            assert!(due_after.is_empty(), "next_date must have advanced past today");

            // A second post-due on the same (now not-due) template must be refused.
            let second = service::recurring::post_due_recurring_expense(tx, cx, &undo, template.id).await;
            assert!(second.is_err());

            service::recurring::delete_recurring_expense(tx, cx, template.id).await?;
            let after_delete = service::recurring::get_recurring_expenses(tx).await?;
            assert!(after_delete.iter().all(|r| r.id != template.id));

            Ok(())
        }) as BoxFuture<'_, accounting_app_lib::core::tx::TxResult<()>>
    })
    .await
    .expect("recurring expense assertions must pass");
}
