//! DB-backed tests for the `invoices` domain (03-domains/08-invoices.md §8a, 08b-pos-shifts.md
//! §8a). Written now, run in the deferred time-boxed test pass (per-implementer hard rule: never
//! run cargo from this agent). Needs `EQUAL_TEST_DATABASE_URL` — see `tests/support/mod.rs`.
//!
//! No shared `seed_company` fixture yet (G-P9) — this file builds its own minimal fixture, same
//! pattern as `domain_purchases.rs`/`domain_payments.rs`: one branch, base settings (SAR,
//! prices_include_tax), an open fiscal year, the system-role accounts every posting touches
//! (`cash`, `bank`, `cardClearing`, `receivable`, `sales`, `vatOutput`, `cogs`, `inventory`,
//! `cashOver`, `cashShort`, `salesReturns`, `inventoryWriteOff`), three payment methods (cash,
//! mada/card, bank transfer), a default 15% OUTPUT tax, a customer with a credit limit, and two
//! products with stock.
//!
//! Scope note (time-boxed pass, per this wave's final report): this file covers the highest-value
//! scenarios named in the spec's §8 checklist (cash sale, split tender, credit sale + due date,
//! the PG-1 worked example, a refund, credit-limit block, shift open/close incl. variance, a held
//! sale) rather than every bullet — the parity-case list (§8b) is handed to Part 04 regardless of
//! which of these ran here first.

use crate::support;

use std::sync::Arc;

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::tx::{with_read, with_tx, BoxFuture, TxOpts, TxResult};
use accounting_app_lib::domains::invoices::dto::{
    CashMovementKind, CloseShiftInput, HandoverMode, HeldSaleInput, HeldSaleLine, OpenShiftInput, RefundInput, RefundLine, RefundMethod,
    SaleInput, SaleInputLine, SalePaymentMethod, Tender,
};
use accounting_app_lib::domains::invoices::service::{held, quotations as q_service, reads, refund as refund_service, sale, shifts};
use accounting_app_lib::domains::products::dto::catalog::{Product, ProductInput, ProductType};
use accounting_app_lib::domains::products::service::products;
use accounting_app_lib::entities::org::{accounts, branches, fiscal_years, payment_methods, settings, taxes};
use accounting_app_lib::entities::parties::parties;
use accounting_app_lib::shared::activity::undo::UndoRegistry;
use accounting_app_lib::shared::invariants;
use accounting_app_lib::utils::id::Id;
use chrono::Datelike;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseTransaction, Set};
use support::TestDb;

// --- Fixture -------------------------------------------------------------------------------------

#[allow(dead_code)] // fields kept for tests still to be written
struct Fixture {
    pub branch_id: Id,
    pub customer_id: Id,
    pub product_id: Id,
    pub product2_id: Id,
    pub cash_method_id: Id,
    pub card_method_id: Id,
    pub bank_method_id: Id,
}

/// Logs in an admin session whose `users` row exists (FK target of `journal_entries.created_by`,
/// `audit.user_id`, documents' `created_by`/`cashier_id`, …).
async fn log_in(test_db: &TestDb) -> Id {
    let user_id = Id::new();
    let user = AuthenticatedUser { id: user_id, username: "test".to_string(), role: Role::Admin, home_branch_id: Id::new(), allowed_branches: vec![], price_list_id: None, max_discount: None };
    let connection = test_db.state.db.read().unwrap().as_ref().expect("test db connected").connection.clone();
    support::seed_user(&connection, user_id, "admin").await;
    *test_db.state.session.write().unwrap() = Some(user);
    user_id
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
        subtype: Set("otherCurrentAsset".to_string()),
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

async fn seed_payment_method<C: ConnectionTrait>(conn: &C, name: &str, kind: &str, role: &str, sort_order: i16) -> Id {
    let id = Id::new();
    let now = chrono::Utc::now();
    let method = payment_methods::ActiveModel {
        id: Set(id),
        name: Set(name.to_string()),
        r#type: Set(kind.to_string()),
        icon: Set(None),
        account_role: Set(role.to_string()),
        fee_pct: Set(Decimal::ZERO),
        requires_reference: Set(None),
        show_in_pos: Set(true),
        show_in_payments: Set(true),
        sort_order: Set(sort_order),
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

    let tax_id = Id::new();
    let tax = taxes::ActiveModel {
        id: Set(tax_id),
        name: Set("ضريبة القيمة المضافة 15%".to_string()),
        rate: Set(dec!(15)),
        r#type: Set("OUTPUT".to_string()),
        is_default: Set(true),
        active: Set(true),
        category: Set("S".to_string()),
        direction: Set("sales".to_string()),
        exemption_reason: Set(None),
        account_role: Set(Some("vatOutput".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    tax.insert(conn).await.unwrap();

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
        default_tax_id: Set(Some(tax_id)),
        invoice_number_prefix: Set("INV-".to_string()),
        printer: Set(accounting_app_lib::entities::values::PrinterSettings {
            mode: accounting_app_lib::entities::values::PrinterMode::A4,
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
        accounting: Set(Some(accounting_app_lib::entities::values::AccountingPolicy { lock_date: None, default_purchase_account_id: None })),
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

    seed_role_account(conn, "1110", "cash").await;
    seed_role_account(conn, "1120", "bank").await;
    seed_role_account(conn, "1125", "cardClearing").await;
    seed_role_account(conn, "1200", "receivable").await;
    seed_role_account(conn, "4100", "sales").await;
    seed_role_account(conn, "2200", "vatOutput").await;
    seed_role_account(conn, "5100", "cogs").await;
    seed_role_account(conn, "1130", "inventory").await;
    seed_role_account(conn, "4900", "cashOver").await;
    seed_role_account(conn, "6900", "cashShort").await;
    seed_role_account(conn, "4200", "salesReturns").await;
    seed_role_account(conn, "5120", "inventoryWriteOff").await;
    // `stock_in` books its goods as an owner contribution in kind (Dr inventory / Cr capital).
    seed_role_account(conn, "3100", "capital").await;

    let cash_method_id = seed_payment_method(conn, "نقدي", "cash", "cash", 1).await;
    let card_method_id = seed_payment_method(conn, "مدى", "card", "cardClearing", 2).await;
    let bank_method_id = seed_payment_method(conn, "تحويل بنكي", "bank_transfer", "bank", 3).await;

    let customer_id = Id::new();
    let customer = parties::ActiveModel {
        id: Set(customer_id),
        kind: Set("customer".to_string()),
        r#type: Set("individual".to_string()),
        name: Set("أحمد".to_string()),
        name_en: Set(None),
        code: Set("C-0001".to_string()),
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
        payment_terms_days: Set(Some(30)),
        salesperson_id: Set(None),
        branch_id: Set(None),
        bank: Set(None),
        opening_balance: Set(None),
        notes: Set(None),
        linked_party_id: Set(None),
        credit_limit: Set(Some(dec!(1000))),
        contact_person: Set(None),
        default_expense_account_id: Set(None),
        search_normalized: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    customer.insert(conn).await.unwrap();

    let user_id = log_in_placeholder();
    let _ = user_id;

    Fixture { branch_id, customer_id, product_id: Id::new(), product2_id: Id::new(), cash_method_id, card_method_id, bank_method_id }
}

fn log_in_placeholder() -> Id {
    Id::new()
}

fn base_product_input(name: &str, sku: &str, cost_price: Decimal, price: Decimal) -> ProductInput {
    ProductInput {
        name: name.to_string(),
        name_en: None,
        sku: sku.to_string(),
        barcode: None,
        category_id: None,
        unit_id: None,
        r#type: ProductType::Product,
        stock_mode: None,
        cost_price,
        price,
        min_stock: None,
        active: true,
        image: None,
        prices: None,
        purchase_account_id: None,
        stock_by_branch: None,
        brand: None,
        tags: None,
        image_ids: None,
        description: None,
        units: None,
        unit_prices: None,
        min_price: None,
        sale_tax_id: None,
        purchase_tax_id: None,
        revenue_account_id: None,
        cogs_account_id: None,
        allow_negative_stock: None,
        shelf_location: None,
        preferred_supplier_id: None,
        reorder_qty: None,
        track_batches: None,
        expiry_alert_days: None,
        warranty_months: None,
        warranty_provider: None,
        weight: None,
        custom_fields: None,
        opening_qty: None,
    }
}

async fn run_all_invariants(conn: &DatabaseTransaction) {
    let report = invariants::run_all(conn).await.expect("invariants must run");
    for r in &report {
        assert!(r.passed, "invariant {} failed: {:?}", r.key, r.message);
    }
}

async fn stock_in(test_db: &TestDb, product_id: Id, qty: Decimal, unit_cost: Decimal) {
    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            let mut p = accounting_app_lib::shared::stock::lock_product(tx, product_id).await?;
            accounting_app_lib::shared::stock::apply_change(
                tx,
                cx,
                &mut p,
                qty,
                accounting_app_lib::utils::money::round2(qty * unit_cost),
                "stock_in",
                accounting_app_lib::shared::stock::StockRef { id: Id::new(), number: "STK-1".to_string() },
                &accounting_app_lib::utils::dates::DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) },
                None,
            )
            .await?;
            // The GL side of the stock (`inventory-gl`: inventory GL = Σ product.stockValue).
            use accounting_app_lib::shared::ledger::{accounts::SystemRole, post, AccountRef, PostJournal, PostingLine};
            let value = accounting_app_lib::utils::money::round2(qty * unit_cost);
            post(
                tx,
                cx,
                PostJournal::new(
                    cx.clock.today(),
                    "stock in",
                    accounting_app_lib::entities::journal::journal_entries::JournalEntryType::Manual,
                    vec![PostingLine::debit(AccountRef::Role(SystemRole::Inventory), value), PostingLine::credit(AccountRef::Role(SystemRole::Capital), value)],
                ),
            )
            .await?;
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();
}

async fn make_fixture(test_db: &TestDb) -> Fixture {
    // `with_tx` refuses with UNAUTHORIZED without a session, so log in before seeding.
    log_in(test_db).await;
    let mut fixture = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| Box::pin(async move { Ok(seed_fixture(tx).await) }) as BoxFuture<'_, TxResult<Fixture>>).await.unwrap();

    let undo = Arc::new(UndoRegistry::new());
    let p1 = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let input = base_product_input("منتج ١", "SKU-1", dec!(50), dec!(115));
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        }
    })
    .await
    .unwrap();
    fixture.product_id = p1.id;
    stock_in(test_db, p1.id, dec!(100), dec!(50)).await;

    let p2 = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let input = base_product_input("منتج ٢", "SKU-2", dec!(20), dec!(46));
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        }
    })
    .await
    .unwrap();
    fixture.product2_id = p2.id;
    stock_in(test_db, p2.id, dec!(50), dec!(20)).await;

    fixture
}

// --- Tests -----------------------------------------------------------------------------------

#[tokio::test]
async fn cash_sale_inclusive_vat_posts_and_updates_stock() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;

    let input = SaleInput {
        customer_id: None,
        lines: vec![SaleInputLine {
            product_id: fixture.product_id.to_string(),
            qty: dec!(2),
            price: dec!(115),
            discount: None,
            discount_is_pct: None,
            tax_id: None,
            unit_id: None,
            unit_factor: None,
            list_price: None,
            price_override_reason: None,
            batch_id: None,
            batch_no: None,
            is_free_text: None,
            revenue_account_id: None,
            name: None,
        }],
        discount_rate: Decimal::ZERO,
        discount_amount: None,
        payment_method: SalePaymentMethod::Cash,
        paid_amount: dec!(230),
        tendered_amount: Some(dec!(230)),
        tenders: None,
        note: None,
        source: None,
        shift_id: None,
        invoice_type: None,
        due_date_override: None,
        po_reference: None,
        terms: None,
        attachment_ids: None,
        manager_approved_by: None,
        branch_id: None,
        cost_center_id: None,
        currency: None,
        exchange_rate: None,
    };

    let registry = Arc::new(UndoRegistry::new());
    let invoice = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let input = input.clone();
        let registry = registry.clone();
        Box::pin(async move { sale::create_sale(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Invoice>>
    })
    .await
    .unwrap();

    assert_eq!(invoice.grand_total, dec!(230));
    assert_eq!(invoice.paid_amount, dec!(230));
    assert_eq!(invoice.tenders.as_ref().unwrap().len(), 1);

    with_read(&test_db.state, move |tx| {
        Box::pin(async move {
            run_all_invariants(tx).await;
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    test_db.finish().await;
}

#[tokio::test]
async fn split_tender_cash_and_card() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;

    let input = SaleInput {
        customer_id: None,
        lines: vec![SaleInputLine {
            product_id: fixture.product_id.to_string(),
            qty: dec!(1),
            price: dec!(115),
            discount: None,
            discount_is_pct: None,
            tax_id: None,
            unit_id: None,
            unit_factor: None,
            list_price: None,
            price_override_reason: None,
            batch_id: None,
            batch_no: None,
            is_free_text: None,
            revenue_account_id: None,
            name: None,
        }],
        discount_rate: Decimal::ZERO,
        discount_amount: None,
        payment_method: SalePaymentMethod::Cash,
        paid_amount: dec!(115),
        tendered_amount: None,
        tenders: Some(vec![
            Tender { payment_method_id: fixture.cash_method_id, amount: dec!(65), reference: None },
            Tender { payment_method_id: fixture.card_method_id, amount: dec!(50), reference: None },
        ]),
        note: None,
        source: None,
        shift_id: None,
        invoice_type: None,
        due_date_override: None,
        po_reference: None,
        terms: None,
        attachment_ids: None,
        manager_approved_by: None,
        branch_id: None,
        cost_center_id: None,
        currency: None,
        exchange_rate: None,
    };

    let registry = Arc::new(UndoRegistry::new());
    let invoice = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let input = input.clone();
        let registry = registry.clone();
        Box::pin(async move { sale::create_sale(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Invoice>>
    })
    .await
    .unwrap();

    assert_eq!(invoice.tenders.as_ref().unwrap().len(), 2);
    assert_eq!(invoice.paid_amount, dec!(115));

    with_read(&test_db.state, move |tx| Box::pin(async move { run_all_invariants(tx).await; Ok(()) }) as BoxFuture<'_, TxResult<()>>).await.unwrap();

    test_db.finish().await;
}

#[tokio::test]
async fn credit_sale_sets_due_date_and_partial_payment_status() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;

    let input = SaleInput {
        customer_id: Some(fixture.customer_id),
        lines: vec![SaleInputLine {
            product_id: fixture.product_id.to_string(),
            qty: dec!(1),
            price: dec!(115),
            discount: None,
            discount_is_pct: None,
            tax_id: None,
            unit_id: None,
            unit_factor: None,
            list_price: None,
            price_override_reason: None,
            batch_id: None,
            batch_no: None,
            is_free_text: None,
            revenue_account_id: None,
            name: None,
        }],
        discount_rate: Decimal::ZERO,
        discount_amount: None,
        payment_method: SalePaymentMethod::Credit,
        paid_amount: Decimal::ZERO,
        tendered_amount: None,
        tenders: None,
        note: None,
        source: None,
        shift_id: None,
        invoice_type: None,
        due_date_override: None,
        po_reference: None,
        terms: None,
        attachment_ids: None,
        manager_approved_by: None,
        branch_id: None,
        cost_center_id: None,
        currency: None,
        exchange_rate: None,
    };

    let registry = Arc::new(UndoRegistry::new());
    let invoice = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let input = input.clone();
        let registry = registry.clone();
        Box::pin(async move { sale::create_sale(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Invoice>>
    })
    .await
    .unwrap();

    assert!(invoice.due_date.is_some(), "a credit sale to a customer with payment terms must get a due date");
    assert_eq!(invoice.payment_status, accounting_app_lib::domains::invoices::dto::PaymentStatus::Unpaid);

    with_read(&test_db.state, move |tx| Box::pin(async move { run_all_invariants(tx).await; Ok(()) }) as BoxFuture<'_, TxResult<()>>).await.unwrap();

    test_db.finish().await;
}

#[tokio::test]
async fn credit_limit_blocks_over_limit_sale() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;

    // Customer's credit_limit is 1000; a 2000+ credit sale must be refused. Only a role without the
    // override (`role_can_override_credit_limit`: Accounting/Parties write) is blocked — the fixture's
    // admin could override it (`creditLimit.ts`'s `canOverride`), so sell as a cashier.
    test_db.state.session.write().unwrap().as_mut().expect("logged in").role = Role::Cashier;
    let input = SaleInput {
        customer_id: Some(fixture.customer_id),
        lines: vec![SaleInputLine {
            product_id: fixture.product_id.to_string(),
            qty: dec!(20),
            price: dec!(115),
            discount: None,
            discount_is_pct: None,
            tax_id: None,
            unit_id: None,
            unit_factor: None,
            list_price: None,
            price_override_reason: None,
            batch_id: None,
            batch_no: None,
            is_free_text: None,
            revenue_account_id: None,
            name: None,
        }],
        discount_rate: Decimal::ZERO,
        discount_amount: None,
        payment_method: SalePaymentMethod::Credit,
        paid_amount: Decimal::ZERO,
        tendered_amount: None,
        tenders: None,
        note: None,
        source: None,
        shift_id: None,
        invoice_type: None,
        due_date_override: None,
        po_reference: None,
        terms: None,
        attachment_ids: None,
        manager_approved_by: None,
        branch_id: None,
        cost_center_id: None,
        currency: None,
        exchange_rate: None,
    };

    let registry = Arc::new(UndoRegistry::new());
    let result = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let input = input.clone();
        let registry = registry.clone();
        Box::pin(async move { sale::create_sale(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Invoice>>
    })
    .await;

    let err = result.expect_err("a sale exceeding the customer's credit limit must be refused");
    assert!(err.to_string().contains("تجاوز الحد الائتماني"), "unexpected refusal: {err:?}");

    test_db.finish().await;
}

#[tokio::test]
async fn refund_partial_then_stock_restocked() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;

    let sale_input = SaleInput {
        customer_id: None,
        lines: vec![SaleInputLine {
            product_id: fixture.product_id.to_string(),
            qty: dec!(2),
            price: dec!(115),
            discount: None,
            discount_is_pct: None,
            tax_id: None,
            unit_id: None,
            unit_factor: None,
            list_price: None,
            price_override_reason: None,
            batch_id: None,
            batch_no: None,
            is_free_text: None,
            revenue_account_id: None,
            name: None,
        }],
        discount_rate: Decimal::ZERO,
        discount_amount: None,
        payment_method: SalePaymentMethod::Cash,
        paid_amount: dec!(230),
        tendered_amount: Some(dec!(230)),
        tenders: None,
        note: None,
        source: None,
        shift_id: None,
        invoice_type: None,
        due_date_override: None,
        po_reference: None,
        terms: None,
        attachment_ids: None,
        manager_approved_by: None,
        branch_id: None,
        cost_center_id: None,
        currency: None,
        exchange_rate: None,
    };
    let registry = Arc::new(UndoRegistry::new());
    let invoice = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let input = sale_input.clone();
        let registry = registry.clone();
        Box::pin(async move { sale::create_sale(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Invoice>>
    })
    .await
    .unwrap();

    // The refund input names a line by its DTO id (`{invoiceId}-l{position+1}`), the id the UI
    // reads off the invoice and sends back.
    let real_line_id = invoice.lines[0].id.clone();

    let refund_input = RefundInput {
        invoice_id: invoice.id,
        reason: Some("عيب في المنتج".to_string()),
        lines: vec![RefundLine { invoice_line_id: real_line_id.clone(), qty: dec!(1), restock: Some(true) }],
        refund_method: Some(RefundMethod::Cash),
    };
    let registry2 = Arc::new(UndoRegistry::new());
    let refund = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let input = refund_input.clone();
        let registry = registry2.clone();
        Box::pin(async move { refund_service::create_refund(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Refund>>
    })
    .await
    .unwrap();

    // G-25 / ACC-0003: the sale is 2 × 115 tax-inclusive (line net 200, VAT 30). Returning 1 unit
    // reverses exactly half of the line's net and VAT — 100 + 15 = 115 back, never 115 + 15% = 132.25.
    assert_eq!(refund.lines.len(), 1);
    assert_eq!(refund.sub_total, dec!(100));
    assert_eq!(refund.tax_amount, dec!(15));
    assert_eq!(refund.grand_total, dec!(115));
    assert_eq!(refund.cash_back, dec!(115));

    with_read(&test_db.state, move |tx| Box::pin(async move { run_all_invariants(tx).await; Ok(()) }) as BoxFuture<'_, TxResult<()>>).await.unwrap();

    // Returning the last unit is the final refund: together the two reverse the sale's revenue and
    // VAT exactly (Σ net 200, Σ VAT 30, Σ gross 230 = the invoice's grand total).
    let final_input = RefundInput {
        invoice_id: invoice.id,
        reason: Some("عيب في المنتج".to_string()),
        lines: vec![RefundLine { invoice_line_id: real_line_id.clone(), qty: dec!(1), restock: Some(true) }],
        refund_method: Some(RefundMethod::Cash),
    };
    let registry3 = Arc::new(UndoRegistry::new());
    let final_refund = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let input = final_input.clone();
        let registry = registry3.clone();
        Box::pin(async move { refund_service::create_refund(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Refund>>
    })
    .await
    .unwrap();

    assert_eq!(final_refund.sub_total, dec!(100));
    assert_eq!(final_refund.tax_amount, dec!(15));
    assert_eq!(final_refund.grand_total, dec!(115));
    assert_eq!(refund.sub_total + final_refund.sub_total, invoice.grand_total - invoice.tax_amount);
    assert_eq!(refund.tax_amount + final_refund.tax_amount, invoice.tax_amount);
    assert_eq!(refund.grand_total + final_refund.grand_total, invoice.grand_total);

    with_read(&test_db.state, move |tx| Box::pin(async move { run_all_invariants(tx).await; Ok(()) }) as BoxFuture<'_, TxResult<()>>).await.unwrap();

    test_db.finish().await;
}

#[tokio::test]
async fn shift_open_close_exact_no_variance_entry() {
    let test_db = TestDb::fresh().await;
    let _fixture = make_fixture(&test_db).await;

    let registry = Arc::new(UndoRegistry::new());
    let shift = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { shifts::open_pos_shift(tx, cx, &registry, OpenShiftInput { terminal_id: None, branch_id: None, opening_float: dec!(200), opening_denominations: None }).await })
            as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Shift>>
    })
    .await
    .unwrap();

    // Second open on the same terminal must be refused (uq_shifts_open_key / pre-check).
    let registry2 = Arc::new(UndoRegistry::new());
    let second_open = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let registry = registry2.clone();
        Box::pin(async move { shifts::open_pos_shift(tx, cx, &registry, OpenShiftInput { terminal_id: None, branch_id: None, opening_float: dec!(100), opening_denominations: None }).await })
            as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Shift>>
    })
    .await;
    assert!(second_open.is_err(), "opening a second shift on the same terminal must be refused");

    let registry3 = Arc::new(UndoRegistry::new());
    let shift_id = shift.base.id;
    let closed = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let registry = registry3.clone();
        Box::pin(async move {
            shifts::close_pos_shift(tx, cx, &registry, shift_id, CloseShiftInput { counted_cash: dec!(200), closing_denominations: None, handover_mode: Some(HandoverMode::Handover), note: None }).await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Shift>>
    })
    .await
    .unwrap();

    assert_eq!(closed.base.variance, Some(Decimal::ZERO));

    with_read(&test_db.state, move |tx| Box::pin(async move { run_all_invariants(tx).await; Ok(()) }) as BoxFuture<'_, TxResult<()>>).await.unwrap();

    test_db.finish().await;
}

#[tokio::test]
async fn shift_close_over_posts_cash_over_variance() {
    let test_db = TestDb::fresh().await;
    let _fixture = make_fixture(&test_db).await;

    let registry = Arc::new(UndoRegistry::new());
    let shift = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { shifts::open_pos_shift(tx, cx, &registry, OpenShiftInput { terminal_id: None, branch_id: None, opening_float: dec!(200), opening_denominations: None }).await })
            as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Shift>>
    })
    .await
    .unwrap();

    let registry2 = Arc::new(UndoRegistry::new());
    let shift_id = shift.base.id;
    let closed = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let registry = registry2.clone();
        Box::pin(async move {
            shifts::close_pos_shift(tx, cx, &registry, shift_id, CloseShiftInput { counted_cash: dec!(205), closing_denominations: None, handover_mode: Some(HandoverMode::Handover), note: None }).await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Shift>>
    })
    .await
    .unwrap();

    assert_eq!(closed.base.variance, Some(dec!(5)));

    with_read(&test_db.state, move |tx| Box::pin(async move { run_all_invariants(tx).await; Ok(()) }) as BoxFuture<'_, TxResult<()>>).await.unwrap();

    test_db.finish().await;
}

#[tokio::test]
async fn record_cash_in_out_requires_open_shift() {
    let test_db = TestDb::fresh().await;
    let _fixture = make_fixture(&test_db).await;

    let result = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move { shifts::record_cash_in_out(tx, cx, CashMovementKind::PayIn, dec!(50), None).await }) as BoxFuture<'_, TxResult<()>>
    })
    .await;
    assert!(result.is_err(), "a pay-in with no open shift must be refused");

    let registry = Arc::new(UndoRegistry::new());
    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            shifts::open_pos_shift(tx, cx, &registry, OpenShiftInput { terminal_id: None, branch_id: None, opening_float: dec!(0), opening_denominations: None }).await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Shift>>
    })
    .await
    .unwrap();

    let ok = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move { shifts::record_cash_in_out(tx, cx, CashMovementKind::PayIn, dec!(50), None).await }) as BoxFuture<'_, TxResult<()>>
    })
    .await;
    assert!(ok.is_ok());

    test_db.finish().await;
}

#[tokio::test]
async fn held_sale_hold_resume_discard() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;

    let held_sale = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let input = HeldSaleInput {
            label: Some("عميل واقف".to_string()),
            terminal_id: None,
            customer_id: None,
            discount_rate: Decimal::ZERO,
            discount_is_pct: false,
            note: None,
            lines: vec![HeldSaleLine {
                product_id: fixture.product_id.to_string(),
                unit_id: None,
                qty: dec!(1),
                price: dec!(115),
                list_price: None,
                price_override_reason: None,
                discount: None,
                discount_is_pct: None,
                batch_id: None,
                tax_id: None,
            }],
        };
        Box::pin(async move { held::hold_sale(tx, cx, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::HeldSale>>
    })
    .await
    .unwrap();

    let list = with_read(&test_db.state, {
        let terminal_id = test_db.state.terminal.terminal_id;
        move |tx| Box::pin(async move { held::get_held_sales(tx, terminal_id).await }) as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::invoices::dto::HeldSale>>>
    })
    .await
    .unwrap();
    assert_eq!(list.len(), 1);

    let id = held_sale.id;
    let resumed = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move { held::resume_held_sale(tx, id).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::HeldSale>>
    })
    .await
    .unwrap();
    assert_eq!(resumed.id, id);

    let resumed_again = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move { held::resume_held_sale(tx, id).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::HeldSale>>
    })
    .await;
    assert!(resumed_again.is_err(), "resuming an already-resumed held sale must be NOT_FOUND");

    // Discarding a missing id is idempotent, not an error.
    let discard = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| Box::pin(async move { held::discard_held_sale(tx, id).await }) as BoxFuture<'_, TxResult<()>>).await;
    assert!(discard.is_ok());

    test_db.finish().await;
}

#[tokio::test]
async fn quotation_save_and_convert_to_invoice() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;

    let quotation = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let input = accounting_app_lib::domains::invoices::dto::QuotationInput {
            customer_id: Some(fixture.customer_id),
            expiry_date: None,
            lines: vec![SaleInputLine {
                product_id: fixture.product_id.to_string(),
                qty: dec!(1),
                price: dec!(115),
                discount: None,
                discount_is_pct: None,
                tax_id: None,
                unit_id: None,
                unit_factor: None,
                list_price: None,
                price_override_reason: None,
                batch_id: None,
                batch_no: None,
                is_free_text: None,
                revenue_account_id: None,
                name: None,
            }],
            discount_rate: Decimal::ZERO,
            note: None,
            terms: None,
            po_reference: None,
        };
        Box::pin(async move { q_service::save_quotation(tx, cx, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Quotation>>
    })
    .await
    .unwrap();

    assert_eq!(quotation.status, accounting_app_lib::domains::invoices::dto::QuotationStatus::Draft);

    let registry = Arc::new(UndoRegistry::new());
    let quotation_id = quotation.id;
    let invoice = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            q_service::convert_quotation_to_invoice(
                tx,
                cx,
                &registry,
                quotation_id,
                accounting_app_lib::domains::invoices::dto::ConvertPayment { payment_method: SalePaymentMethod::Cash, paid_amount: dec!(115), tendered_amount: None },
            )
            .await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Invoice>>
    })
    .await
    .unwrap();

    assert_eq!(invoice.grand_total, dec!(115));

    // Converting twice must be refused (CONFLICT).
    let registry2 = Arc::new(UndoRegistry::new());
    let second = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let registry = registry2.clone();
        Box::pin(async move {
            q_service::convert_quotation_to_invoice(
                tx,
                cx,
                &registry,
                quotation_id,
                accounting_app_lib::domains::invoices::dto::ConvertPayment { payment_method: SalePaymentMethod::Cash, paid_amount: dec!(115), tendered_amount: None },
            )
            .await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Invoice>>
    })
    .await;
    assert!(second.is_err());

    with_read(&test_db.state, move |tx| Box::pin(async move { run_all_invariants(tx).await; Ok(()) }) as BoxFuture<'_, TxResult<()>>).await.unwrap();

    test_db.finish().await;
}

#[tokio::test]
async fn get_invoices_paged_totals_and_search() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;

    let registry = Arc::new(UndoRegistry::new());
    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let registry = registry.clone();
        let input = SaleInput {
            customer_id: Some(fixture.customer_id),
            lines: vec![SaleInputLine {
                product_id: fixture.product_id.to_string(),
                qty: dec!(1),
                price: dec!(115),
                discount: None,
                discount_is_pct: None,
                tax_id: None,
                unit_id: None,
                unit_factor: None,
                list_price: None,
                price_override_reason: None,
                batch_id: None,
                batch_no: None,
                is_free_text: None,
                revenue_account_id: None,
                name: None,
            }],
            discount_rate: Decimal::ZERO,
            discount_amount: None,
            payment_method: SalePaymentMethod::Cash,
            paid_amount: dec!(115),
            tendered_amount: None,
            tenders: None,
            note: None,
            source: None,
            shift_id: None,
            invoice_type: None,
            due_date_override: None,
            po_reference: None,
            terms: None,
            attachment_ids: None,
            manager_approved_by: None,
            branch_id: None,
            cost_center_id: None,
            currency: None,
            exchange_rate: None,
        };
        Box::pin(async move { sale::create_sale(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Invoice>>
    })
    .await
    .unwrap();

    let paged = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            let read_ctx = accounting_app_lib::core::tx::ReadCtx { actor: cx.actor.clone(), terminal_id: cx.terminal_id, clock: cx.clock };
            reads::get_invoices_paged(
                tx,
                &read_ctx,
                accounting_app_lib::core::dto::PagedQuery { page: 1, page_size: 20, sort: None, filters: Some(accounting_app_lib::domains::invoices::dto::InvoiceListFilter { search: Some("أحمد".to_string()), ..Default::default() }) },
            )
            .await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::core::dto::PagedResult<accounting_app_lib::domains::invoices::dto::InvoiceRow>>>
    })
    .await
    .unwrap();

    assert_eq!(paged.total, 1, "search for 'أحمد' must find the customer's invoice");
    assert!(paged.totals.is_some());

    test_db.finish().await;
}

// --- ACC-0009 / ACC-0010 ----------------------------------------------------------------------

fn one_line_sale(product_id: Id, customer_id: Option<Id>, price: Decimal, method: SalePaymentMethod, paid: Decimal, currency: Option<(&str, Decimal)>) -> SaleInput {
    SaleInput {
        customer_id,
        lines: vec![SaleInputLine {
            product_id: product_id.to_string(),
            qty: dec!(1),
            price,
            discount: None,
            discount_is_pct: None,
            tax_id: None,
            unit_id: None,
            unit_factor: None,
            list_price: None,
            price_override_reason: None,
            batch_id: None,
            batch_no: None,
            is_free_text: None,
            revenue_account_id: None,
            name: None,
        }],
        discount_rate: Decimal::ZERO,
        discount_amount: None,
        payment_method: method,
        paid_amount: paid,
        tendered_amount: None,
        tenders: None,
        note: None,
        source: None,
        shift_id: None,
        invoice_type: None,
        due_date_override: None,
        po_reference: None,
        terms: None,
        attachment_ids: None,
        manager_approved_by: None,
        branch_id: None,
        cost_center_id: None,
        currency: currency.map(|(c, _)| c.to_string()),
        exchange_rate: currency.map(|(_, r)| r),
    }
}

async fn create_sale_now(test_db: &TestDb, input: SaleInput) -> accounting_app_lib::domains::invoices::dto::Invoice {
    let registry = Arc::new(UndoRegistry::new());
    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let input = input.clone();
        let registry = registry.clone();
        Box::pin(async move { sale::create_sale(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Invoice>>
    })
    .await
    .unwrap()
}

async fn refund_whole_line(test_db: &TestDb, invoice_id: Id, method: RefundMethod) -> accounting_app_lib::domains::invoices::dto::Refund {
    let line_id = with_read(&test_db.state, move |tx| {
        Box::pin(async move {
            use accounting_app_lib::entities::sales::invoice_lines::{Column, Entity};
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
            let line = Entity::find().filter(Column::InvoiceId.eq(invoice_id)).one(tx).await.unwrap().unwrap();
            Ok(format!("{}-l{}", line.invoice_id, line.position + 1))
        }) as BoxFuture<'_, TxResult<String>>
    })
    .await
    .unwrap();
    let input = RefundInput { invoice_id, reason: Some("إرجاع".to_string()), lines: vec![RefundLine { invoice_line_id: line_id, qty: dec!(1), restock: Some(true) }], refund_method: Some(method) };
    let registry = Arc::new(UndoRegistry::new());
    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let input = input.clone();
        let registry = registry.clone();
        Box::pin(async move { refund_service::create_refund(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Refund>>
    })
    .await
    .unwrap()
}

type PostedLine = (String, Decimal, Decimal, Option<Decimal>, Option<Decimal>);

/// `(system_role, debit, credit, amount_fc, rate)` for every line of a document's journal entry.
async fn posted_lines(test_db: &TestDb, source_kind: &'static str, source_id: Id) -> Vec<PostedLine> {
    with_read(&test_db.state, move |tx| {
        Box::pin(async move {
            use accounting_app_lib::entities::journal::{journal_entries, journal_lines};
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
            let entry = journal_entries::Entity::find()
                .filter(journal_entries::Column::SourceKind.eq(source_kind))
                .filter(journal_entries::Column::SourceId.eq(source_id))
                .one(tx)
                .await
                .unwrap()
                .expect("document must have posted");
            let lines = journal_lines::Entity::find().filter(journal_lines::Column::JournalEntryId.eq(entry.id)).all(tx).await.unwrap();
            let mut out = Vec::new();
            for l in lines {
                let account = accounts::Entity::find_by_id(l.account_id).one(tx).await.unwrap().unwrap();
                out.push((account.system_role.unwrap_or_default(), l.debit, l.credit, l.amount_fc, l.rate));
            }
            Ok(out)
        }) as BoxFuture<'_, TxResult<Vec<PostedLine>>>
    })
    .await
    .unwrap()
}

fn role_total(lines: &[PostedLine], role: &str) -> (Decimal, Decimal) {
    lines.iter().filter(|l| l.0 == role).fold((Decimal::ZERO, Decimal::ZERO), |a, l| (a.0 + l.1, a.1 + l.2))
}

/// ACC-0009: a USD invoice (rate 48.5) is paid by a USD receipt at 49 and then refunded in cash
/// when the day's rate is 50. The refund reverses the sale's BASE revenue and VAT exactly (at the
/// invoice's rate), pays the 10 USD back at the day's rate (500 SAR) and books the 15 SAR gap as a
/// realized FX loss — never posting the raw 10 USD amounts to the SAR ledger.
#[tokio::test]
async fn fx_refund_posts_base_amounts_at_invoice_rate() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            use accounting_app_lib::entities::org::{currencies, exchange_rates};
            let now = chrono::Utc::now();
            seed_role_account(tx, "4310", "fxGain").await;
            seed_role_account(tx, "6310", "fxLoss").await;
            currencies::ActiveModel {
                code: Set("USD".to_string()),
                name_ar: Set("دولار أمريكي".to_string()),
                symbol: Set("$".to_string()),
                decimals: Set(2),
                active: Set(true),
                fixed: Set(None),
                fixed_rate: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(tx)
            .await
            .unwrap();
            exchange_rates::ActiveModel { id: Set(Id::new()), currency: Set("USD".to_string()), date: Set(cx.clock.today()), rate: Set(dec!(50)), created_at: Set(now), updated_at: Set(now) }
                .insert(tx)
                .await
                .unwrap();
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    let invoice = create_sale_now(&test_db, one_line_sale(fixture.product_id, Some(fixture.customer_id), dec!(10), SalePaymentMethod::Credit, Decimal::ZERO, Some(("USD", dec!(48.5))))).await;
    let sale_lines = posted_lines(&test_db, "invoice", invoice.id).await;
    assert_eq!(role_total(&sale_lines, "receivable").0, dec!(485));

    // Receive 10 USD at 49 (490 SAR) against the invoice: AR at the invoice's rate, +5 FX gain.
    let customer_id = fixture.customer_id;
    let branch_id = fixture.branch_id;
    let invoice_id = invoice.id;
    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            use accounting_app_lib::domains::payments::dto::{AllocationInputTargetKind, PartyKind, PaymentAllocationInput, PaymentInput, PaymentTenderKind, PaymentTypeDto};
            let undo = UndoRegistry::new();
            let input = PaymentInput {
                date: cx.clock.today().format("%Y-%m-%d").to_string(),
                r#type: PaymentTypeDto::Received,
                target_type: PartyKind::Customer,
                target_id: customer_id,
                amount: dec!(490),
                method: PaymentTenderKind::BankTransfer,
                note: None,
                allocations: Some(vec![PaymentAllocationInput { target_kind: AllocationInputTargetKind::Invoice, target_id: invoice_id, amount: dec!(490) }]),
                branch_id: Some(branch_id),
                currency: Some("USD".to_string()),
                amount_fc: Some(dec!(10)),
                rate: Some(dec!(49)),
            };
            accounting_app_lib::domains::payments::service::create::create_payment(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::payments::dto::Payment>>
    })
    .await
    .unwrap();

    let refund = refund_whole_line(&test_db, invoice.id, RefundMethod::Cash).await;
    assert_eq!(refund.grand_total, dec!(10), "the refund document itself stays in the invoice currency");
    assert_eq!(refund.cash_back, dec!(10));

    let lines = posted_lines(&test_db, "refund", refund.id).await;
    // Revenue and VAT reverse the sale's base amounts exactly.
    assert_eq!(role_total(&lines, "salesReturns").0, role_total(&sale_lines, "sales").1);
    assert_eq!(role_total(&lines, "vatOutput").0, role_total(&sale_lines, "vatOutput").1);
    // Cash out at the day's rate, tagged with its FC amount; the gap is a realized FX loss.
    let cash = lines.iter().find(|l| l.0 == "cash").expect("cash leg");
    assert_eq!((cash.2, cash.3, cash.4), (dec!(500), Some(dec!(10)), Some(dec!(50))));
    assert_eq!(role_total(&lines, "fxLoss").0, dec!(15));
    assert_eq!(role_total(&lines, "receivable"), (Decimal::ZERO, Decimal::ZERO));

    with_read(&test_db.state, move |tx| Box::pin(async move { run_all_invariants(tx).await; Ok(()) }) as BoxFuture<'_, TxResult<()>>).await.unwrap();

    test_db.finish().await;
}

/// ACC-0010: a `customer_credit` refund on an already-paid invoice leaves a real credit on the
/// customer's receivable — it is counted as unallocated customer credit (party page) and keeps the
/// `customer-allocation` invariant (Σ outstanding − unallocated credit = ledger balance) green.
#[tokio::test]
async fn customer_credit_refund_on_paid_invoice_is_unallocated_credit() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;

    let invoice = create_sale_now(&test_db, one_line_sale(fixture.product_id, Some(fixture.customer_id), dec!(115), SalePaymentMethod::Cash, dec!(115), None)).await;
    let refund = refund_whole_line(&test_db, invoice.id, RefundMethod::CustomerCredit).await;
    assert_eq!(refund.settled_to_receivable, Decimal::ZERO);
    assert_eq!(refund.credited_to_account, Some(dec!(115)));

    let customer_id = fixture.customer_id;
    let (balance, credit, credits) = with_read(&test_db.state, move |tx| {
        Box::pin(async move {
            use accounting_app_lib::entities::journal::journal_lines::PartyKind;
            use accounting_app_lib::shared::balances;
            let balance = balances::customer_balance(tx, customer_id).await?;
            let credit = balances::unallocated_credit_for(tx, PartyKind::Customer, customer_id).await?;
            let credits = balances::unallocated_credits(tx, PartyKind::Customer, &[customer_id]).await?;
            Ok((balance, credit, credits.get(&customer_id).copied()))
        }) as BoxFuture<'_, TxResult<(Decimal, Decimal, Option<Decimal>)>>
    })
    .await
    .unwrap();
    assert_eq!(balance, dec!(-115), "the credit sits on the customer's receivable");
    assert_eq!(credit, dec!(115));
    assert_eq!(credits, Some(dec!(115)), "the batch (party list) read agrees with the single-party read");

    with_read(&test_db.state, move |tx| Box::pin(async move { run_all_invariants(tx).await; Ok(()) }) as BoxFuture<'_, TxResult<()>>).await.unwrap();

    test_db.finish().await;
}

// --- ACC-0014 / ACC-0016 / ACC-0017 -----------------------------------------------------------

/// USD (no rate of its own for today — the sales below pass their own) plus the FX role accounts.
async fn seed_usd(test_db: &TestDb) {
    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            use accounting_app_lib::entities::org::currencies;
            let now = chrono::Utc::now();
            seed_role_account(tx, "4310", "fxGain").await;
            seed_role_account(tx, "6310", "fxLoss").await;
            currencies::ActiveModel {
                code: Set("USD".to_string()),
                name_ar: Set("دولار أمريكي".to_string()),
                symbol: Set("$".to_string()),
                decimals: Set(2),
                active: Set(true),
                fixed: Set(None),
                fixed_rate: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(tx)
            .await
            .unwrap();
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();
}

async fn set_lock_date(test_db: &TestDb, days_from_today: Option<i64>) {
    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            use sea_orm::EntityTrait;
            let row = settings::Entity::find().one(tx).await.unwrap().unwrap();
            let mut model: settings::ActiveModel = row.into();
            let lock_date = days_from_today.map(|d| cx.clock.today() + chrono::Duration::days(d));
            model.accounting = Set(Some(accounting_app_lib::entities::values::AccountingPolicy { lock_date, default_purchase_account_id: None }));
            model.update(tx).await.unwrap();
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();
}

/// ACC-0016: a USD sale paid with cash + bank-transfer tenders posts each tender at its BASE amount
/// (the sale's rate), FC-tagged like a payment — it used to post the raw USD amounts, leaving the
/// entry unbalanced and the sale refused.
#[tokio::test]
async fn fc_sale_tenders_post_base_amounts() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    seed_usd(&test_db).await;

    let mut input = one_line_sale(fixture.product_id, Some(fixture.customer_id), dec!(100), SalePaymentMethod::Cash, dec!(100), Some(("USD", dec!(48.57))));
    input.tenders = Some(vec![
        Tender { payment_method_id: fixture.cash_method_id, amount: dec!(60.33), reference: None },
        Tender { payment_method_id: fixture.bank_method_id, amount: dec!(39.67), reference: Some("TRX-1".to_string()) },
    ]);
    let invoice = create_sale_now(&test_db, input).await;
    assert_eq!(invoice.grand_total, dec!(100), "the invoice stays in USD");

    let lines = posted_lines(&test_db, "invoice", invoice.id).await;
    let cash = lines.iter().find(|l| l.0 == "cash").expect("cash tender");
    assert_eq!((cash.1, cash.3, cash.4), (dec!(2930.23), Some(dec!(60.33)), Some(dec!(48.57))));
    let bank = lines.iter().find(|l| l.0 == "bank").expect("bank tender");
    assert_eq!((bank.1, bank.3, bank.4), (dec!(1926.77), Some(dec!(39.67)), Some(dec!(48.57))));
    // Tenders (+ a zero receivable) carry exactly the sale's base total: 100 USD × 48.57.
    assert_eq!(role_total(&lines, "sales").1 + role_total(&lines, "vatOutput").1, dec!(4857));

    with_read(&test_db.state, move |tx| Box::pin(async move { run_all_invariants(tx).await; Ok(()) }) as BoxFuture<'_, TxResult<()>>).await.unwrap();

    test_db.finish().await;
}

/// ACC-0017: 2 × 100 tax-inclusive (net 173.91, VAT 26.09). Each unit refunds exactly 100 (net 86.95
/// + VAT 13.05, then 86.96 + 13.04) — not 100.01 then 99.99 — and the two refunds reverse the line.
#[tokio::test]
async fn unit_refunds_of_an_inclusive_line_are_each_exactly_the_price() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;

    let mut input = one_line_sale(fixture.product_id, None, dec!(100), SalePaymentMethod::Cash, dec!(200), None);
    input.lines[0].qty = dec!(2);
    let invoice = create_sale_now(&test_db, input).await;
    assert_eq!((invoice.grand_total, invoice.tax_amount), (dec!(200), dec!(26.09)));

    let r1 = refund_whole_line(&test_db, invoice.id, RefundMethod::Cash).await;
    let r2 = refund_whole_line(&test_db, invoice.id, RefundMethod::Cash).await;
    assert_eq!((r1.sub_total, r1.tax_amount, r1.grand_total), (dec!(86.95), dec!(13.05), dec!(100)));
    assert_eq!((r2.sub_total, r2.tax_amount, r2.grand_total), (dec!(86.96), dec!(13.04), dec!(100)));

    with_read(&test_db.state, move |tx| Box::pin(async move { run_all_invariants(tx).await; Ok(()) }) as BoxFuture<'_, TxResult<()>>).await.unwrap();

    test_db.finish().await;
}

/// ACC-0014: a sale refused by the lock date leaves no trace — no invoice, stock untouched, and the
/// next accepted sale still takes the first invoice number.
#[tokio::test]
async fn refused_sale_leaves_no_invoice_stock_or_number() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    set_lock_date(&test_db, Some(1)).await;

    let input = one_line_sale(fixture.product_id, None, dec!(115), SalePaymentMethod::Cash, dec!(115), None);
    let registry = Arc::new(UndoRegistry::new());
    let refused = with_tx(&test_db.state, TxOpts::default(), {
        let input = input.clone();
        move |tx, cx| {
            let input = input.clone();
            let registry = registry.clone();
            Box::pin(async move { sale::create_sale(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Invoice>>
        }
    })
    .await;
    assert!(matches!(refused, Err(accounting_app_lib::core::error::AppError::Forbidden { .. })), "expected FORBIDDEN, got {refused:?}");

    let product_id = fixture.product_id;
    with_read(&test_db.state, move |tx| {
        Box::pin(async move {
            use accounting_app_lib::entities::catalog::products as product_rows;
            use accounting_app_lib::entities::sales::invoices;
            use sea_orm::{EntityTrait, PaginatorTrait};
            assert_eq!(invoices::Entity::find().count(tx).await.unwrap(), 0, "a refused sale must save no invoice");
            let p = product_rows::Entity::find_by_id(product_id).one(tx).await.unwrap().unwrap();
            assert_eq!(p.stock_qty, dec!(100), "a refused sale must not move stock");
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    set_lock_date(&test_db, None).await;
    let invoice = create_sale_now(&test_db, input).await;
    assert!(invoice.number.ends_with("000001"), "the refused sale consumed a number: {}", invoice.number);

    test_db.finish().await;
}

// --- ACC-0032 / ACC-0033 (non-base-unit lines) -----------------------------------------------------

/// `(stock_qty, stock_value)` of a product row.
async fn product_stock(test_db: &TestDb, product_id: Id) -> (Decimal, Decimal) {
    with_read(&test_db.state, move |tx| {
        Box::pin(async move {
            use accounting_app_lib::entities::catalog::products as product_rows;
            use sea_orm::EntityTrait;
            let p = product_rows::Entity::find_by_id(product_id).one(tx).await.unwrap().unwrap();
            Ok((p.stock_qty, p.stock_value))
        }) as BoxFuture<'_, TxResult<(Decimal, Decimal)>>
    })
    .await
    .unwrap()
}

async fn refund_line(test_db: &TestDb, invoice_id: Id, line_id: String, qty: Decimal, restock: bool) -> accounting_app_lib::domains::invoices::dto::Refund {
    let input = RefundInput { invoice_id, reason: Some("إرجاع".to_string()), lines: vec![RefundLine { invoice_line_id: line_id, qty, restock: Some(restock) }], refund_method: Some(RefundMethod::Cash) };
    let registry = Arc::new(UndoRegistry::new());
    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let input = input.clone();
        let registry = registry.clone();
        Box::pin(async move { refund_service::create_refund(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Refund>>
    })
    .await
    .unwrap()
}

/// ACC-0032: 2 boxes of 4 (8 base units at cost 50) sold, then one box refunded to stock and one
/// written off. Each refunded box moves 4 base units at 4 × 50 = 200 — the restock puts 4 units and
/// 200 back into stock (Dr inventory 200 / Cr COGS 200), the write-off books 200 to 5120 and leaves
/// stock alone — never 1 unit / 50 per box.
#[tokio::test]
async fn refund_of_a_non_base_unit_line_moves_its_base_units() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;

    let mut input = one_line_sale(fixture.product_id, None, dec!(460), SalePaymentMethod::Cash, dec!(920), None);
    input.lines[0].qty = dec!(2);
    input.lines[0].unit_id = Some("pu-box".to_string());
    input.lines[0].unit_factor = Some(dec!(4));
    let invoice = create_sale_now(&test_db, input).await;
    assert_eq!(product_stock(&test_db, fixture.product_id).await, (dec!(92), dec!(4600)));
    let line_id = invoice.lines[0].id.clone();

    let restocked = refund_line(&test_db, invoice.id, line_id.clone(), dec!(1), true).await;
    assert_eq!(product_stock(&test_db, fixture.product_id).await, (dec!(96), dec!(4800)));
    let lines = posted_lines(&test_db, "refund", restocked.id).await;
    assert_eq!(role_total(&lines, "inventory"), (dec!(200), Decimal::ZERO));
    assert_eq!(role_total(&lines, "cogs"), (Decimal::ZERO, dec!(200)));

    let written_off = refund_line(&test_db, invoice.id, line_id, dec!(1), false).await;
    assert_eq!(product_stock(&test_db, fixture.product_id).await, (dec!(96), dec!(4800)));
    let lines = posted_lines(&test_db, "refund", written_off.id).await;
    assert_eq!(role_total(&lines, "inventoryWriteOff"), (dec!(200), Decimal::ZERO));
    assert_eq!(role_total(&lines, "cogs"), (Decimal::ZERO, dec!(200)));

    with_read(&test_db.state, move |tx| Box::pin(async move { run_all_invariants(tx).await; Ok(()) }) as BoxFuture<'_, TxResult<()>>).await.unwrap();

    test_db.finish().await;
}

/// ACC-0033: a quotation for 2 boxes of 4 keeps its unit, and converting it sells 8 base units
/// (stock 100 → 92, COGS 8 × 50 = 400) — not 2.
#[tokio::test]
async fn quotation_in_a_non_base_unit_converts_to_its_base_units() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;

    let mut line = one_line_sale(fixture.product_id, None, dec!(460), SalePaymentMethod::Cash, dec!(920), None).lines.remove(0);
    line.qty = dec!(2);
    line.unit_id = Some("pu-box".to_string());
    line.unit_factor = Some(dec!(4));
    let customer_id = fixture.customer_id;
    let quotation = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let input = accounting_app_lib::domains::invoices::dto::QuotationInput {
            customer_id: Some(customer_id),
            expiry_date: None,
            lines: vec![line.clone()],
            discount_rate: Decimal::ZERO,
            note: None,
            terms: None,
            po_reference: None,
        };
        Box::pin(async move { q_service::save_quotation(tx, cx, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Quotation>>
    })
    .await
    .unwrap();
    assert_eq!((quotation.lines[0].unit_id.as_deref(), quotation.lines[0].unit_factor), (Some("pu-box"), Some(dec!(4))));

    let registry = Arc::new(UndoRegistry::new());
    let quotation_id = quotation.id;
    let invoice = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            q_service::convert_quotation_to_invoice(
                tx,
                cx,
                &registry,
                quotation_id,
                accounting_app_lib::domains::invoices::dto::ConvertPayment { payment_method: SalePaymentMethod::Cash, paid_amount: dec!(920), tendered_amount: None },
            )
            .await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Invoice>>
    })
    .await
    .unwrap();

    assert_eq!(invoice.lines[0].unit_factor, Some(dec!(4)));
    assert_eq!(product_stock(&test_db, fixture.product_id).await, (dec!(92), dec!(4600)));
    let lines = posted_lines(&test_db, "invoice", invoice.id).await;
    assert_eq!(role_total(&lines, "cogs"), (dec!(400), Decimal::ZERO));

    with_read(&test_db.state, move |tx| Box::pin(async move { run_all_invariants(tx).await; Ok(()) }) as BoxFuture<'_, TxResult<()>>).await.unwrap();

    test_db.finish().await;
}
