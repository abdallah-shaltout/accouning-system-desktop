//! DB-backed tests for the `vouchers` domain (03-domains/10-vouchers.md §8a). Written now, run in
//! the deferred time-boxed test pass (per-implementer hard rule: never run cargo from this agent).
//! Needs `EQUAL_TEST_DATABASE_URL` — see `tests/support/mod.rs`. Fixture pattern copied from
//! `tests/domain_expenses.rs` (tests can't import each other's fixtures).

use crate::support;

use std::sync::Arc;

use chrono::Datelike;

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::error::AppError;
use accounting_app_lib::core::tx::{with_tx, BoxFuture, TxCtx, TxOpts, TxResult};
use accounting_app_lib::entities::journal::journal_entries::JournalEntryType;
use accounting_app_lib::shared::ledger::accounts::SystemRole;
use accounting_app_lib::shared::ledger::{post, AccountRef, PostJournal, PostingLine, SourceRef};
use accounting_app_lib::domains::vouchers::dto::{
    CardSettlementGroupRef, CardSettlementInput, OwnerDirection, OwnerVoucherInput, PaymentVoucherInput, ReceiptVoucherInput, TransferVoucherInput,
    UnsettledTenderGroup, Voucher, VoucherFilter,
};
use accounting_app_lib::domains::vouchers::service;
use accounting_app_lib::entities::org::{accounts, branches, fiscal_years, payment_methods, settings};
use accounting_app_lib::entities::sales::invoice_tenders;
use accounting_app_lib::entities::sales::invoices;
use accounting_app_lib::entities::values::{AccountingPolicy, PrinterMode, PrinterSettings};
use accounting_app_lib::shared::activity::undo::UndoRegistry;
use accounting_app_lib::shared::invariants;
use accounting_app_lib::utils::id::Id;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, Set};
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
/// `cash`/`bank`/`cardClearing`/`walletClearing`/`drawings`/`cardFees`, two plain (non-role, manual)
/// accounts for receipt/payment/transfer targets, a cash payment method, and a card + wallet payment
/// method (both clearing-role, non-zero fee %).
struct Fixture {
    pub cash_role_account_id: Id,
    pub bank_role_account_id: Id,
    // Kept for parity with the full set of role accounts `seed_fixture` creates — not every test
    // asserts against these ids directly (the card/wallet payment methods resolve them internally),
    // but a future posting-trace test will want them without re-deriving the fixture.
    #[allow(dead_code)]
    pub card_clearing_account_id: Id,
    #[allow(dead_code)]
    pub wallet_clearing_account_id: Id,
    #[allow(dead_code)]
    pub drawings_account_id: Id,
    pub plain_account_id: Id,
    #[allow(dead_code)]
    pub plain_account_2_id: Id,
    pub cash_payment_method_id: Id,
    pub card_payment_method_id: Id,
    pub wallet_payment_method_id: Id,
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

async fn seed_plain_account<C: ConnectionTrait>(conn: &C, code: &str, allow_manual: bool) -> Id {
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
        subtype: Set("otherCurrentAsset".to_string()),
        normal_side: Set("DEBIT".to_string()),
        system_role: Set(None),
        currency: Set(None),
        branch_id: Set(None),
        requires_party: Set(Some(false)),
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

async fn seed_payment_method<C: ConnectionTrait>(conn: &C, name: &str, r#type: &str, role: &str, fee_pct: Decimal) -> Id {
    let id = Id::new();
    let now = chrono::Utc::now();
    let method = payment_methods::ActiveModel {
        id: Set(id),
        name: Set(name.to_string()),
        r#type: Set(r#type.to_string()),
        icon: Set(None),
        account_role: Set(role.to_string()),
        fee_pct: Set(fee_pct),
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
    method.insert(conn).await.unwrap();
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
        start_date: Set(today.with_month(1).unwrap().with_day(1).unwrap()),
        end_date: Set(today.with_month(12).unwrap().with_day(31).unwrap_or(today)),
        is_closed: Set(false),
        closing_entry_id: Set(None),
        closed_at: Set(None),
        closed_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    fiscal_year.insert(conn).await.unwrap();

    let cash_role_account_id = seed_role_account(conn, "1110", "cash").await;
    let bank_role_account_id = seed_role_account(conn, "1120", "bank").await;
    let card_clearing_account_id = seed_role_account(conn, "1130", "cardClearing").await;
    let wallet_clearing_account_id = seed_role_account(conn, "1140", "walletClearing").await;
    let drawings_account_id = seed_role_account(conn, "3200", "drawings").await;
    // `cardFees` role account, needed by the settlement's fee line.
    let _card_fees_account_id = seed_role_account(conn, "5300", "cardFees").await;
    // The seeded completed invoices post their own tender entries (Dr method account / Cr sales).
    let _sales_account_id = seed_role_account(conn, "4100", "sales").await;

    let plain_account_id = seed_plain_account(conn, "6100", true).await;
    let plain_account_2_id = seed_plain_account(conn, "6200", true).await;

    let cash_payment_method_id = seed_payment_method(conn, "نقدًا", "cash", "cash", Decimal::ZERO).await;
    let card_payment_method_id = seed_payment_method(conn, "مدى", "card", "cardClearing", dec!(2.5)).await;
    let wallet_payment_method_id = seed_payment_method(conn, "STC Pay", "wallet", "walletClearing", dec!(1)).await;

    Fixture {
        cash_role_account_id,
        bank_role_account_id,
        card_clearing_account_id,
        wallet_clearing_account_id,
        drawings_account_id,
        plain_account_id,
        plain_account_2_id,
        cash_payment_method_id,
        card_payment_method_id,
        wallet_payment_method_id,
    }
}

/// A minimal `COMPLETED` invoice with one tender, for the settlement tests — only the columns the
/// `unsettled_tender_groups` query reads are meaningful; the rest are filled with harmless
/// defaults.
async fn seed_completed_invoice_with_tender<C: ConnectionTrait>(conn: &C, cx: &TxCtx, day: chrono::NaiveDate, payment_method_id: Id, amount: Decimal) -> Id {
    let now = chrono::Utc::now();
    // `invoices.cashier_id` FKs to `users`.
    let cashier_id = Id::new();
    support::seed_user(conn, cashier_id, "cashier").await;
    let invoice_id = Id::new();
    let number = format!("INV-{}", support::unique_tail(invoice_id, 8));
    let invoice = invoices::ActiveModel {
        id: Set(invoice_id),
        number: Set(number.clone()),
        date_day: Set(day),
        date_instant: Set(None),
        customer_id: Set(None),
        cashier_id: Set(cashier_id),
        status: Set(invoices::InvoiceStatus::Completed),
        payment_status: Set(invoices::PaymentStatus::Paid),
        sub_total: Set(amount),
        discount_rate: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        tax_rate: Set(Decimal::ZERO),
        tax_amount: Set(Decimal::ZERO),
        grand_total: Set(amount),
        payment_method: Set(invoices::SalePaymentMethod::Card),
        paid_amount: Set(amount),
        refunded_amount: Set(Decimal::ZERO),
        tendered_amount: Set(Some(amount)),
        due_date_day: Set(None),
        due_date_instant: Set(None),
        note: Set(None),
        source: Set(Some(invoices::InvoiceSource::Pos)),
        shift_id: Set(None),
        branch_id: Set(None),
        invoice_type: Set(Some(invoices::InvoiceType::Simplified)),
        po_reference: Set(None),
        terms: Set(None),
        attachment_ids: Set(None),
        currency: Set(None),
        exchange_rate: Set(None),
        cost_center_id: Set(None),
        search_normalized: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(invoices::SyncStatus::Local),
    };
    invoice.insert(conn).await.unwrap();

    let tender = invoice_tenders::ActiveModel {
        id: Set(Id::new()),
        invoice_id: Set(invoice_id),
        position: Set(0),
        payment_method_id: Set(payment_method_id),
        amount: Set(amount),
        reference: Set(None),
    };
    tender.insert(conn).await.unwrap();

    // The sale's own entry: Dr the tender's settlement account (its method's `account_role`) / Cr
    // sales. Without it a settlement credits a clearing account nothing ever debited
    // (`card-clearing`/`wallet-clearing` would read negative).
    let method = payment_methods::Entity::find_by_id(payment_method_id).one(conn).await.unwrap().expect("seeded payment method");
    let role: SystemRole = method.account_role.parse().expect("known account role");
    let mut req = PostJournal::new(
        day,
        format!("invoice {number}"),
        JournalEntryType::System,
        vec![PostingLine::debit(AccountRef::Role(role), amount), PostingLine::credit(AccountRef::Role(SystemRole::Sales), amount)],
    );
    req.source = Some(SourceRef { kind: "invoice".to_string(), id: invoice_id, number: Some(number) });
    post(conn, cx, req).await.unwrap();

    invoice_id
}

async fn assert_invariants_ok(test_db: &TestDb) {
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let results = invariants::run_all(conn).await.expect("invariants must run");
    let failed: Vec<_> = results.iter().filter(|r| !r.passed).collect();
    assert!(failed.is_empty(), "invariants failed: {failed:?}");
}

fn today_str(cx_today: chrono::NaiveDate) -> String {
    cx_today.format("%Y-%m-%d").to_string()
}

// --- Receipt / payment / transfer / owner --------------------------------------------------------

#[tokio::test]
async fn receipt_voucher_posts_dr_cash_cr_manual_account() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let voucher = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let input = ReceiptVoucherInput {
                date: today_str(cx.clock.today()),
                amount: dec!(100),
                description: "بيع خردة".to_string(),
                note: None,
                attachment_ids: None,
                cost_center_id: None,
                payment_method_id: fixture.cash_payment_method_id,
                credit_account_id: fixture.plain_account_id,
            };
            service::general::create_receipt_voucher(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<Voucher>>
    })
    .await
    .expect("receipt voucher must succeed");

    match voucher {
        Voucher::Receipt { base, .. } => {
            assert_eq!(base.number, "VCH-000001");
            assert_eq!(base.amount, dec!(100));
        }
        _ => panic!("expected a RECEIPT voucher"),
    }

    assert_invariants_ok(&test_db).await;
    test_db.finish().await;
}

#[tokio::test]
async fn payment_voucher_posts_dr_manual_account_cr_cash() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let voucher = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let input = PaymentVoucherInput {
                date: today_str(cx.clock.today()),
                amount: dec!(50),
                description: "رسوم حكومية".to_string(),
                note: None,
                attachment_ids: None,
                cost_center_id: None,
                payment_method_id: fixture.cash_payment_method_id,
                debit_account_id: fixture.plain_account_id,
            };
            service::general::create_payment_voucher(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<Voucher>>
    })
    .await
    .expect("payment voucher must succeed");

    match voucher {
        Voucher::Payment { base, .. } => assert_eq!(base.amount, dec!(50)),
        _ => panic!("expected a PAYMENT voucher"),
    }
    assert_invariants_ok(&test_db).await;
    test_db.finish().await;
}

#[tokio::test]
async fn transfer_voucher_with_fee_credits_source_amount_plus_fee() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let voucher = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let input = TransferVoucherInput {
                date: today_str(cx.clock.today()),
                amount: dec!(1000),
                description: "تحويل من الدرج للبنك".to_string(),
                note: None,
                attachment_ids: None,
                cost_center_id: None,
                source_account_id: fixture.cash_role_account_id,
                destination_account_id: fixture.bank_role_account_id,
                fee_amount: Some(dec!(5)),
                fee_account_id: Some(fixture.plain_account_id),
            };
            service::general::create_transfer_voucher(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<Voucher>>
    })
    .await
    .expect("transfer voucher must succeed");

    match voucher {
        Voucher::Transfer { base, fee_amount, .. } => {
            assert_eq!(base.amount, dec!(1000));
            assert_eq!(fee_amount, Some(dec!(5)));
        }
        _ => panic!("expected a TRANSFER voucher"),
    }
    assert_invariants_ok(&test_db).await;
    test_db.finish().await;
}

#[tokio::test]
async fn transfer_voucher_without_fee_stores_none() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let voucher = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let input = TransferVoucherInput {
                date: today_str(cx.clock.today()),
                amount: dec!(200),
                description: "تحويل بسيط".to_string(),
                note: None,
                attachment_ids: None,
                cost_center_id: None,
                source_account_id: fixture.bank_role_account_id,
                destination_account_id: fixture.cash_role_account_id,
                fee_amount: None,
                fee_account_id: None,
            };
            service::general::create_transfer_voucher(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<Voucher>>
    })
    .await
    .expect("transfer voucher must succeed");

    match voucher {
        Voucher::Transfer { fee_amount, .. } => assert_eq!(fee_amount, None),
        _ => panic!("expected a TRANSFER voucher"),
    }

    test_db.finish().await;
}

#[tokio::test]
async fn transfer_voucher_same_accounts_rejected_before_lookup() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let err = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let input = TransferVoucherInput {
                date: today_str(cx.clock.today()),
                amount: dec!(10),
                description: "خطأ".to_string(),
                note: None,
                attachment_ids: None,
                cost_center_id: None,
                source_account_id: fixture.cash_role_account_id,
                destination_account_id: fixture.cash_role_account_id,
                fee_amount: None,
                fee_account_id: None,
            };
            service::general::create_transfer_voucher(tx, cx, &undo, input).await.map(|_| ())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect_err("same source/destination must be refused");
    assert!(matches!(err, AppError::Validation { message } if message == "اختر حسابين مختلفين للتحويل"));

    test_db.finish().await;
}

#[tokio::test]
async fn transfer_voucher_fee_without_account_rejected() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let err = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let input = TransferVoucherInput {
                date: today_str(cx.clock.today()),
                amount: dec!(10),
                description: "بلا حساب عمولة".to_string(),
                note: None,
                attachment_ids: None,
                cost_center_id: None,
                source_account_id: fixture.cash_role_account_id,
                destination_account_id: fixture.bank_role_account_id,
                fee_amount: Some(dec!(1)),
                fee_account_id: None,
            };
            service::general::create_transfer_voucher(tx, cx, &undo, input).await.map(|_| ())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect_err("fee without an account must be refused");
    assert!(matches!(err, AppError::Validation { message } if message == "اختر حساب العمولة"));

    test_db.finish().await;
}

#[tokio::test]
async fn record_transfer_voucher_with_preallocated_number_uses_it() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let voucher = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let input = TransferVoucherInput {
                date: today_str(cx.clock.today()),
                amount: dec!(300),
                description: "سحب نهاية الوردية".to_string(),
                note: None,
                attachment_ids: None,
                cost_center_id: None,
                source_account_id: fixture.cash_role_account_id,
                destination_account_id: fixture.bank_role_account_id,
                fee_amount: None,
                fee_account_id: None,
            };
            service::general::record_transfer_voucher(tx, cx, &undo, input, Some("VCH-PRESET-01".to_string())).await
        }) as BoxFuture<'_, TxResult<Voucher>>
    })
    .await
    .expect("preallocated-number transfer must succeed");

    match voucher {
        Voucher::Transfer { base, .. } => assert_eq!(base.number, "VCH-PRESET-01"),
        _ => panic!("expected a TRANSFER voucher"),
    }

    test_db.finish().await;
}

#[tokio::test]
async fn owner_voucher_drawings_and_contribution_both_directions() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let (drawings, contribution) = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let drawings = service::general::create_owner_voucher(
                tx,
                cx,
                &undo,
                OwnerVoucherInput {
                    date: today_str(cx.clock.today()),
                    amount: dec!(500),
                    description: "سحب شخصي".to_string(),
                    note: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    direction: OwnerDirection::Drawings,
                    cash_account_id: fixture.cash_role_account_id,
                },
            )
            .await?;
            let contribution = service::general::create_owner_voucher(
                tx,
                cx,
                &undo,
                OwnerVoucherInput {
                    date: today_str(cx.clock.today()),
                    amount: dec!(2000),
                    description: "إضافة رأس مال".to_string(),
                    note: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    direction: OwnerDirection::Contribution,
                    cash_account_id: fixture.cash_role_account_id,
                },
            )
            .await?;
            Ok((drawings, contribution))
        }) as BoxFuture<'_, TxResult<(Voucher, Voucher)>>
    })
    .await
    .expect("owner vouchers must succeed");

    match drawings {
        Voucher::Owner { direction, .. } => assert!(matches!(direction, OwnerDirection::Drawings)),
        _ => panic!("expected an OWNER voucher"),
    }
    match contribution {
        Voucher::Owner { direction, .. } => assert!(matches!(direction, OwnerDirection::Contribution)),
        _ => panic!("expected an OWNER voucher"),
    }
    assert_invariants_ok(&test_db).await;
    test_db.finish().await;
}

#[tokio::test]
async fn voucher_amount_and_method_and_account_validation_messages() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    // amount <= 0
    let err = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            service::general::create_receipt_voucher(
                tx,
                cx,
                &undo,
                ReceiptVoucherInput {
                    date: today_str(cx.clock.today()),
                    amount: Decimal::ZERO,
                    description: "x".to_string(),
                    note: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    payment_method_id: fixture.cash_payment_method_id,
                    credit_account_id: fixture.plain_account_id,
                },
            )
            .await
            .map(|_| ())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect_err("zero amount must be refused");
    assert!(matches!(err, AppError::Validation { message } if message == "المبلغ يجب أن يكون أكبر من صفر"));

    // unknown/inactive payment method
    let undo2 = Arc::new(UndoRegistry::new());
    let err = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo2.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            service::general::create_receipt_voucher(
                tx,
                cx,
                &undo,
                ReceiptVoucherInput {
                    date: today_str(cx.clock.today()),
                    amount: dec!(10),
                    description: "x".to_string(),
                    note: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    payment_method_id: Id::new(),
                    credit_account_id: fixture.plain_account_id,
                },
            )
            .await
            .map(|_| ())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect_err("unknown payment method must be refused");
    assert!(matches!(err, AppError::Validation { message } if message == "اختر طريقة الدفع"));

    // unknown account
    let undo3 = Arc::new(UndoRegistry::new());
    let err = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo3.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            service::general::create_receipt_voucher(
                tx,
                cx,
                &undo,
                ReceiptVoucherInput {
                    date: today_str(cx.clock.today()),
                    amount: dec!(10),
                    description: "x".to_string(),
                    note: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    payment_method_id: fixture.cash_payment_method_id,
                    credit_account_id: Id::new(),
                },
            )
            .await
            .map(|_| ())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect_err("unknown account must be refused");
    assert!(matches!(err, AppError::NotFound { message } if message == "الحساب غير موجود في شجرة الحسابات"));

    // non-manual account (both wordings: receipt "إلى", payment "من")
    let undo4 = Arc::new(UndoRegistry::new());
    let err = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo4.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let locked_account_id = seed_plain_account(tx, "9999", false).await;
            service::general::create_receipt_voucher(
                tx,
                cx,
                &undo,
                ReceiptVoucherInput {
                    date: today_str(cx.clock.today()),
                    amount: dec!(10),
                    description: "x".to_string(),
                    note: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    payment_method_id: fixture.cash_payment_method_id,
                    credit_account_id: locked_account_id,
                },
            )
            .await
            .map(|_| ())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect_err("non-manual account must be refused on receipt");
    assert!(matches!(err, AppError::Validation { ref message } if message.contains("إلى")));

    let undo5 = Arc::new(UndoRegistry::new());
    let err = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo5.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let locked_account_id = seed_plain_account(tx, "9998", false).await;
            service::general::create_payment_voucher(
                tx,
                cx,
                &undo,
                PaymentVoucherInput {
                    date: today_str(cx.clock.today()),
                    amount: dec!(10),
                    description: "x".to_string(),
                    note: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    payment_method_id: fixture.cash_payment_method_id,
                    debit_account_id: locked_account_id,
                },
            )
            .await
            .map(|_| ())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect_err("non-manual account must be refused on payment");
    assert!(matches!(err, AppError::Validation { ref message } if message.contains("من")));

    test_db.finish().await;
}

#[tokio::test]
async fn owner_voucher_missing_drawings_role_rejected() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    // Fixture without seeding a `drawings` role account: build a stripped-down version inline.
    let err = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let now = chrono::Utc::now();
            let branch_id = Id::new();
            branches::ActiveModel {
                id: Set(branch_id),
                name: Set("فرع".to_string()),
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
            }
            .insert(tx)
            .await
            .unwrap();
            support::seed_currency(tx, "SAR").await;
            settings::ActiveModel {
                id: Set(Id::new()),
                singleton: Set(1),
                store_name: Set("متجر".to_string()),
                logo: Set(None),
                stamp: Set(None),
                signature: Set(None),
                currency: Set("SAR".to_string()),
                country: Set(Some("SA".to_string())),
                vat_number: Set(None),
                default_tax_id: Set(None),
                invoice_number_prefix: Set("INV-".to_string()),
                printer: Set(PrinterSettings { mode: PrinterMode::A4, thermal_width_mm: 80, thermal: None, a4_printer_name: None, label_printer_name: None, a4_template: None, image_template: None }),
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
            }
            .insert(tx)
            .await
            .unwrap();
            let today = chrono::Utc::now().date_naive();
            fiscal_years::ActiveModel {
                id: Set(Id::new()),
                name: Set("2026".to_string()),
                start_date: Set(today.with_month(1).unwrap().with_day(1).unwrap()),
                end_date: Set(today.with_month(12).unwrap().with_day(31).unwrap_or(today)),
                is_closed: Set(false),
                closing_entry_id: Set(None),
                closed_at: Set(None),
                closed_by: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(tx)
            .await
            .unwrap();
            let cash_account_id = seed_role_account(tx, "1110", "cash").await;
            // No `drawings` role account seeded on purpose.

            service::general::create_owner_voucher(
                tx,
                cx,
                &undo,
                OwnerVoucherInput {
                    date: today_str(cx.clock.today()),
                    amount: dec!(10),
                    description: "x".to_string(),
                    note: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    direction: OwnerDirection::Drawings,
                    cash_account_id,
                },
            )
            .await
            .map(|_| ())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect_err("missing drawings role account must be refused");
    assert!(matches!(err, AppError::NotFound { ref message } if message.contains("المسحوبات الشخصية")));

    test_db.finish().await;
}

// --- get_vouchers / get_voucher -------------------------------------------------------------------

#[tokio::test]
async fn get_vouchers_filters_by_kind_date_and_arabic_search_and_orders_desc() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let today = cx.clock.today();

            service::general::create_receipt_voucher(
                tx,
                cx,
                &undo,
                ReceiptVoucherInput {
                    date: today_str(today),
                    amount: dec!(10),
                    description: "من أحمد".to_string(),
                    note: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    payment_method_id: fixture.cash_payment_method_id,
                    credit_account_id: fixture.plain_account_id,
                },
            )
            .await?;
            service::general::create_payment_voucher(
                tx,
                cx,
                &undo,
                PaymentVoucherInput {
                    date: today_str(today),
                    amount: dec!(20),
                    description: "دفعة عادية".to_string(),
                    note: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    payment_method_id: fixture.cash_payment_method_id,
                    debit_account_id: fixture.plain_account_id,
                },
            )
            .await?;

            let all = service::read::get_vouchers(tx, None).await?;
            assert_eq!(all.len(), 2);

            let by_kind = service::read::get_vouchers(tx, Some(VoucherFilter { kind: Some(accounting_app_lib::domains::vouchers::dto::VoucherKind::Payment), from: None, to: None, search: None })).await?;
            assert_eq!(by_kind.len(), 1);

            let by_date = service::read::get_vouchers(tx, Some(VoucherFilter { kind: None, from: Some(today_str(today)), to: Some(today_str(today)), search: None })).await?;
            assert_eq!(by_date.len(), 2);

            // Arabic-normalized search: "احمد" (no hamza) must match "أحمد" (with hamza).
            let by_search = service::read::get_vouchers(tx, Some(VoucherFilter { kind: None, from: None, to: None, search: Some("احمد".to_string()) })).await?;
            assert_eq!(by_search.len(), 1, "Arabic-normalized search must match أ/ا variants");

            let get_unknown = service::read::get_voucher(tx, Id::new()).await;
            assert!(get_unknown.is_err());

            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("list/search assertions must pass");

    test_db.finish().await;
}

// --- Card/wallet settlement ------------------------------------------------------------------------

#[tokio::test]
async fn unsettled_groups_split_by_day_and_method_excluding_cash_and_settled() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;

    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let today = chrono::Utc::now().date_naive();
            let yesterday = today.pred_opt().unwrap();

            // Two card tenders today (same group), one wallet tender today, one card tender
            // yesterday, and a cash tender today (must never appear).
            seed_completed_invoice_with_tender(tx, cx, today, fixture.card_payment_method_id, dec!(100)).await;
            seed_completed_invoice_with_tender(tx, cx, today, fixture.card_payment_method_id, dec!(50)).await;
            seed_completed_invoice_with_tender(tx, cx, today, fixture.wallet_payment_method_id, dec!(30)).await;
            seed_completed_invoice_with_tender(tx, cx, yesterday, fixture.card_payment_method_id, dec!(75)).await;
            seed_completed_invoice_with_tender(tx, cx, today, fixture.cash_payment_method_id, dec!(999)).await;

            let groups = service::settlements::unsettled_tender_groups(tx).await?;
            assert_eq!(groups.len(), 3, "cash tenders must never be grouped: {groups:?}");

            let today_card = groups.iter().find(|g| g.date == today.format("%Y-%m-%d").to_string() && g.payment_method_id == fixture.card_payment_method_id).expect("today's card group");
            assert_eq!(today_card.total, dec!(150));
            assert_eq!(today_card.tender_count, 2);

            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("unsettled groups assertions must pass");

    test_db.finish().await;
}

#[tokio::test]
async fn refunded_invoice_tenders_are_excluded_from_unsettled_groups() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;

    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let today = chrono::Utc::now().date_naive();
            let invoice_id = seed_completed_invoice_with_tender(tx, cx, today, fixture.card_payment_method_id, dec!(40)).await;

            // Flip it to REFUNDED (Q-V2 kept: it must drop out of the unsettled list).
            use accounting_app_lib::entities::sales::invoices::{ActiveModel as InvoiceActiveModel, Entity as InvoiceEntity, InvoiceStatus};
            use sea_orm::EntityTrait;
            let existing = InvoiceEntity::find_by_id(invoice_id).one(tx).await.unwrap().unwrap();
            let mut model: InvoiceActiveModel = existing.into();
            model.status = Set(InvoiceStatus::Refunded);
            model.update(tx).await.unwrap();

            let groups = service::settlements::unsettled_tender_groups(tx).await?;
            assert!(groups.is_empty(), "a REFUNDED invoice's tenders must not appear: {groups:?}");
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("refunded exclusion assertion must pass");

    test_db.finish().await;
}

#[tokio::test]
async fn settlement_with_fee_posts_bank_and_card_fees_debit_and_clearing_credit() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let settlement = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let today = cx.clock.today();
            seed_completed_invoice_with_tender(tx, cx, today, fixture.card_payment_method_id, dec!(100)).await;

            let input = CardSettlementInput {
                date: today_str(today),
                groups: vec![CardSettlementGroupRef { date: today_str(today), payment_method_id: fixture.card_payment_method_id }],
                deposit_amount: Some(dec!(97)),
                note: None,
            };
            service::settlements::create_card_settlement(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::vouchers::dto::CardSettlement>>
    })
    .await
    .expect("settlement must succeed");

    assert_eq!(settlement.number, "STL-000001");
    assert_eq!(settlement.gross_amount, dec!(100));
    assert_eq!(settlement.deposit_amount, dec!(97));
    assert_eq!(settlement.fee_amount, dec!(3));
    assert_eq!(settlement.groups.len(), 1);
    assert_invariants_ok(&test_db).await;
    test_db.finish().await;
}

#[tokio::test]
async fn settlement_deposit_slightly_over_gross_is_allowed() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let settlement = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let today = cx.clock.today();
            seed_completed_invoice_with_tender(tx, cx, today, fixture.card_payment_method_id, dec!(100)).await;

            let input = CardSettlementInput {
                date: today_str(today),
                groups: vec![CardSettlementGroupRef { date: today_str(today), payment_method_id: fixture.card_payment_method_id }],
                // gross 100, deposit 100.003 -> fee = -0.003, within the -0.005 tolerance.
                deposit_amount: Some(dec!(100.003)),
                note: None,
            };
            service::settlements::create_card_settlement(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::vouchers::dto::CardSettlement>>
    })
    .await
    .expect("slightly-over deposit must be allowed");

    // round2's negative-zero normalization (master rule: money rounding) turns -0.003 into 0, not
    // -0.00 — `Decimal::eq` would accept either spelling, but asserting the normalized form keeps
    // this test honest about that rule too.
    assert_eq!(settlement.fee_amount, Decimal::ZERO);

    test_db.finish().await;
}

#[tokio::test]
async fn settlement_deposit_over_gross_by_a_lot_rejected() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let err = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let today = cx.clock.today();
            seed_completed_invoice_with_tender(tx, cx, today, fixture.card_payment_method_id, dec!(100)).await;

            let input = CardSettlementInput {
                date: today_str(today),
                groups: vec![CardSettlementGroupRef { date: today_str(today), payment_method_id: fixture.card_payment_method_id }],
                deposit_amount: Some(dec!(101)),
                note: None,
            };
            service::settlements::create_card_settlement(tx, cx, &undo, input).await.map(|_| ())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect_err("deposit far over gross must be refused");
    assert!(matches!(err, AppError::Validation { message } if message == "مبلغ الإيداع أكبر من إجمالي العمليات المختارة"));

    test_db.finish().await;
}

#[tokio::test]
async fn settlement_mixed_card_and_wallet_posts_two_credit_lines() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let settlement = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let today = cx.clock.today();
            seed_completed_invoice_with_tender(tx, cx, today, fixture.card_payment_method_id, dec!(100)).await;
            seed_completed_invoice_with_tender(tx, cx, today, fixture.wallet_payment_method_id, dec!(50)).await;

            let input = CardSettlementInput {
                date: today_str(today),
                groups: vec![
                    CardSettlementGroupRef { date: today_str(today), payment_method_id: fixture.card_payment_method_id },
                    CardSettlementGroupRef { date: today_str(today), payment_method_id: fixture.wallet_payment_method_id },
                ],
                deposit_amount: Some(dec!(150)),
                note: None,
            };
            service::settlements::create_card_settlement(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::vouchers::dto::CardSettlement>>
    })
    .await
    .expect("mixed settlement must succeed");

    assert_eq!(settlement.gross_amount, dec!(150));
    assert_eq!(settlement.groups.len(), 2);
    assert_invariants_ok(&test_db).await;
    test_db.finish().await;
}

#[tokio::test]
async fn settlement_of_already_settled_group_is_conflict() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let err = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let today = cx.clock.today();
            seed_completed_invoice_with_tender(tx, cx, today, fixture.card_payment_method_id, dec!(100)).await;

            let group_ref = CardSettlementGroupRef { date: today_str(today), payment_method_id: fixture.card_payment_method_id };
            let first = CardSettlementInput { date: today_str(today), groups: vec![group_ref.clone()], deposit_amount: Some(dec!(100)), note: None };
            service::settlements::create_card_settlement(tx, cx, &undo, first).await?;

            // The same (day, method) is no longer in the unsettled list, so re-selecting it must
            // fail the stale-list `CONFLICT` (step 3), not the DB constraint.
            let second = CardSettlementInput { date: today_str(today), groups: vec![group_ref], deposit_amount: Some(dec!(100)), note: None };
            service::settlements::create_card_settlement(tx, cx, &undo, second).await.map(|_| ())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect_err("re-settling an already-settled group must be refused");
    assert!(matches!(err, AppError::Conflict { message } if message == "أحد العناصر المختارة غير متاح للتسوية (ربما تمت تسويته بالفعل)"));

    test_db.finish().await;
}

#[tokio::test]
async fn settlement_no_groups_or_negative_deposit_rejected() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let err = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let _fixture = seed_fixture(tx).await;
            let input = CardSettlementInput { date: today_str(cx.clock.today()), groups: vec![], deposit_amount: Some(dec!(10)), note: None };
            service::settlements::create_card_settlement(tx, cx, &undo, input).await.map(|_| ())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect_err("no groups must be refused");
    assert!(matches!(err, AppError::Validation { message } if message == "اختر يوماً واحداً على الأقل للتسوية"));

    test_db.finish().await;
}

#[tokio::test]
async fn estimate_settlement_fee_matches_round2_of_sum() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let groups = vec![
                UnsettledTenderGroup {
                    date: "2026-01-01".to_string(),
                    payment_method_id: fixture.card_payment_method_id,
                    payment_method_name: "مدى".to_string(),
                    account_role: accounting_app_lib::domains::vouchers::dto::ClearingRole::CardClearing,
                    total: dec!(100),
                    tender_count: 1,
                },
                UnsettledTenderGroup {
                    date: "2026-01-01".to_string(),
                    payment_method_id: fixture.wallet_payment_method_id,
                    payment_method_name: "STC Pay".to_string(),
                    account_role: accounting_app_lib::domains::vouchers::dto::ClearingRole::WalletClearing,
                    total: dec!(50),
                    tender_count: 1,
                },
            ];
            // 100 * 2.5% + 50 * 1% = 2.5 + 0.5 = 3.00
            let fee = service::settlements::estimate_settlement_fee(tx, groups).await?;
            assert_eq!(fee.0, dec!(3.00));
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("fee estimate assertion must pass");

    test_db.finish().await;
}

#[tokio::test]
async fn get_card_settlements_and_get_card_settlement() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let settlement_id = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let today = cx.clock.today();
            seed_completed_invoice_with_tender(tx, cx, today, fixture.card_payment_method_id, dec!(20)).await;
            let input = CardSettlementInput {
                date: today_str(today),
                groups: vec![CardSettlementGroupRef { date: today_str(today), payment_method_id: fixture.card_payment_method_id }],
                deposit_amount: Some(dec!(20)),
                note: None,
            };
            let settlement = service::settlements::create_card_settlement(tx, cx, &undo, input).await?;

            let list = service::read::get_card_settlements(tx).await?;
            assert_eq!(list.len(), 1);

            Ok(settlement.id)
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .expect("settlement list/create must succeed");

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let found = service::read::get_card_settlement(tx, settlement_id).await?;
            assert_eq!(found.id, settlement_id);
            let missing = service::read::get_card_settlement(tx, Id::new()).await;
            assert!(missing.is_err());
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("settlement detail assertions must pass");

    test_db.finish().await;
}

// --- Period lock ------------------------------------------------------------------------------------

#[tokio::test]
async fn voucher_in_closed_period_is_forbidden_and_writes_nothing() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let err = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;

            // Close the period by setting `accounting.lock_date` to today (a date on or before the
            // lock date is closed — `assert_open_period`'s own rule).
            use accounting_app_lib::entities::org::settings::{ActiveModel as SettingsActiveModel, Entity as SettingsEntity};
            use sea_orm::EntityTrait;
            let existing = SettingsEntity::find().one(tx).await.unwrap().unwrap();
            let mut model: SettingsActiveModel = existing.into();
            model.accounting = Set(Some(AccountingPolicy { lock_date: Some(cx.clock.today()), default_purchase_account_id: None }));
            model.update(tx).await.unwrap();

            service::general::create_receipt_voucher(
                tx,
                cx,
                &undo,
                ReceiptVoucherInput {
                    date: today_str(cx.clock.today()),
                    amount: dec!(10),
                    description: "x".to_string(),
                    note: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    payment_method_id: fixture.cash_payment_method_id,
                    credit_account_id: fixture.plain_account_id,
                },
            )
            .await
            .map(|_| ())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect_err("a closed period must refuse the voucher");
    assert!(matches!(err, AppError::Forbidden { .. }));

    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let count = accounting_app_lib::entities::payments::vouchers::Entity::find()
        .all(conn)
        .await
        .unwrap()
        .len();
    drop(db_guard);
    assert_eq!(count, 0, "nothing must be written when the period is closed");

    test_db.finish().await;
}

// --- ACC-0014 / ACC-0016 ------------------------------------------------------------------------

/// ACC-0014: every voucher kind refused by the lock date (committed in its own transaction, like a
/// real user's settings) leaves no trace — no voucher row, and the next accepted voucher still takes
/// the first number.
#[tokio::test]
async fn refused_vouchers_leave_no_row_and_consume_no_number() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    let (cash_account, bank_account, plain_account, cash_method) = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let row = settings::Entity::find().one(tx).await.unwrap().unwrap();
            let mut model: settings::ActiveModel = row.into();
            model.accounting = Set(Some(AccountingPolicy { lock_date: Some(cx.clock.today() + chrono::Duration::days(1)), default_purchase_account_id: None }));
            model.update(tx).await.unwrap();
            Ok((fixture.cash_role_account_id, fixture.bank_role_account_id, fixture.plain_account_id, fixture.cash_payment_method_id))
        }) as BoxFuture<'_, TxResult<(Id, Id, Id, Id)>>
    })
    .await
    .unwrap();

    let receipt = move |date: String| ReceiptVoucherInput {
        date,
        amount: dec!(100),
        description: "بيع خردة".to_string(),
        note: None,
        attachment_ids: None,
        cost_center_id: None,
        payment_method_id: cash_method,
        credit_account_id: plain_account,
    };

    for kind in 0..4 {
        let undo = undo.clone();
        let result = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
            let undo = undo.clone();
            let date = today_str(cx.clock.today());
            Box::pin(async move {
                match kind {
                    0 => service::general::create_receipt_voucher(tx, cx, &undo, receipt(date)).await,
                    1 => {
                        let input = PaymentVoucherInput {
                            date,
                            amount: dec!(50),
                            description: "رسوم".to_string(),
                            note: None,
                            attachment_ids: None,
                            cost_center_id: None,
                            payment_method_id: cash_method,
                            debit_account_id: plain_account,
                        };
                        service::general::create_payment_voucher(tx, cx, &undo, input).await
                    }
                    2 => {
                        let input = TransferVoucherInput {
                            date,
                            amount: dec!(500),
                            description: "إيداع".to_string(),
                            note: None,
                            attachment_ids: None,
                            cost_center_id: None,
                            source_account_id: cash_account,
                            destination_account_id: bank_account,
                            fee_amount: Some(dec!(5)),
                            fee_account_id: Some(plain_account),
                        };
                        service::general::create_transfer_voucher(tx, cx, &undo, input).await
                    }
                    _ => {
                        let input = OwnerVoucherInput {
                            date,
                            amount: dec!(300),
                            description: "مسحوبات".to_string(),
                            note: None,
                            attachment_ids: None,
                            cost_center_id: None,
                            direction: OwnerDirection::Drawings,
                            cash_account_id: cash_account,
                        };
                        service::general::create_owner_voucher(tx, cx, &undo, input).await
                    }
                }
            }) as BoxFuture<'_, TxResult<Voucher>>
        })
        .await;
        assert!(matches!(result, Err(AppError::Forbidden { .. })), "voucher kind {kind}: expected FORBIDDEN, got {result:?}");
    }

    // Unlock, then post one receipt: it is the only voucher and it takes the first number.
    let voucher = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        let date = today_str(cx.clock.today());
        Box::pin(async move {
            use sea_orm::PaginatorTrait;
            assert_eq!(accounting_app_lib::entities::payments::vouchers::Entity::find().count(tx).await.unwrap(), 0, "refused vouchers must save nothing");
            let row = settings::Entity::find().one(tx).await.unwrap().unwrap();
            let mut model: settings::ActiveModel = row.into();
            model.accounting = Set(Some(AccountingPolicy { lock_date: None, default_purchase_account_id: None }));
            model.update(tx).await.unwrap();
            service::general::create_receipt_voucher(tx, cx, &undo, receipt(date)).await
        }) as BoxFuture<'_, TxResult<Voucher>>
    })
    .await
    .expect("receipt voucher must succeed once unlocked");
    match voucher {
        Voucher::Receipt { base, .. } => assert_eq!(base.number, "VCH-000001", "a refused voucher consumed a number"),
        _ => panic!("expected a RECEIPT voucher"),
    }

    test_db.finish().await;
}

/// ACC-0016: a USD invoice's card tender posted its BASE amount to card clearing (the sale's rate),
/// so the settlement pick list offers that base amount — 100 USD × 48.57 = 4,857 — not "100".
#[tokio::test]
async fn unsettled_groups_read_fc_invoice_tenders_in_base() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;

    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            let fixture = seed_fixture(tx).await;
            let today = chrono::Utc::now().date_naive();
            let invoice_id = seed_completed_invoice_with_tender(tx, cx, today, fixture.card_payment_method_id, dec!(100)).await;
            let invoice = invoices::Entity::find_by_id(invoice_id).one(tx).await.unwrap().unwrap();
            let mut model: invoices::ActiveModel = invoice.into();
            model.currency = Set(Some("USD".to_string()));
            model.exchange_rate = Set(Some(dec!(48.57)));
            model.update(tx).await.unwrap();

            let groups = service::settlements::unsettled_tender_groups(tx).await?;
            let card = groups.iter().find(|g| g.payment_method_id == fixture.card_payment_method_id).expect("today's card group");
            assert_eq!(card.total, dec!(4857));
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("FC tender group assertions must pass");

    test_db.finish().await;
}

/// ACC-0031: a payment voucher paid with the business's card (or wallet) credits the BANK — the
/// method's clearing account only holds customer tenders awaiting the acquirer's deposit — the rule
/// ACC-0024 set for expenses. Before, it credited card clearing and broke `card-clearing`.
#[tokio::test]
async fn payment_voucher_by_card_or_wallet_credits_bank_not_clearing() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db).await;
    let undo = Arc::new(UndoRegistry::new());

    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            use accounting_app_lib::entities::journal::{journal_entries, journal_lines};
            use sea_orm::{ColumnTrait, QueryFilter};
            let fixture = seed_fixture(tx).await;
            for method in [fixture.card_payment_method_id, fixture.wallet_payment_method_id] {
                let input = PaymentVoucherInput {
                    date: today_str(cx.clock.today()),
                    amount: dec!(40),
                    description: "اشتراك مدفوع ببطاقة المتجر".to_string(),
                    note: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    payment_method_id: method,
                    debit_account_id: fixture.plain_account_id,
                };
                let voucher = service::general::create_payment_voucher(tx, cx, &undo, input).await?;
                let Voucher::Payment { base, .. } = voucher else { panic!("expected a PAYMENT voucher") };
                let entry = journal_entries::Entity::find()
                    .filter(journal_entries::Column::SourceKind.eq("voucher"))
                    .filter(journal_entries::Column::SourceId.eq(base.id))
                    .one(tx)
                    .await
                    .unwrap()
                    .expect("voucher entry");
                let credit = journal_lines::Entity::find()
                    .filter(journal_lines::Column::JournalEntryId.eq(entry.id))
                    .filter(journal_lines::Column::Credit.gt(Decimal::ZERO))
                    .one(tx)
                    .await
                    .unwrap()
                    .expect("credit line");
                assert_eq!(credit.account_id, fixture.bank_role_account_id, "a card/wallet payout must leave the bank");
            }
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("card/wallet payment vouchers must succeed");
    assert_invariants_ok(&test_db).await;
    test_db.finish().await;
}
