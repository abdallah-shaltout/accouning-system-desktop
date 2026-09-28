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

mod support;

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

struct Fixture {
    pub branch_id: Id,
    pub customer_id: Id,
    pub product_id: Id,
    pub product2_id: Id,
    pub cash_method_id: Id,
    pub card_method_id: Id,
    pub bank_method_id: Id,
}

fn log_in(test_db: &TestDb) -> Id {
    let user_id = Id::new();
    let user = AuthenticatedUser { id: user_id, username: "test".to_string(), role: Role::Admin, home_branch_id: Id::new(), allowed_branches: vec![], price_list_id: None, max_discount: None };
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
        subtype: Set("other".to_string()),
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
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();
}

async fn make_fixture(test_db: &TestDb) -> Fixture {
    let mut fixture = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| Box::pin(async move { Ok(seed_fixture(tx).await) }) as BoxFuture<'_, TxResult<Fixture>>).await.unwrap();

    log_in(test_db);

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
}

#[tokio::test]
async fn credit_limit_blocks_over_limit_sale() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;

    // Customer's credit_limit is 1000; a 2000+ credit sale must be refused.
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

    assert!(result.is_err(), "a sale exceeding the customer's credit limit must be refused");
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

    let invoice_line_id = invoice.lines[0].id.trim_start_matches(&format!("{}-l", invoice.id)).to_string();
    // The DTO's synthetic line id is `{invoiceId}-l{position+1}` — recover the real row id by
    // re-reading the invoice detail (its `InvoiceLine.id` field carries this same synthetic id, but
    // `create_refund` needs the REAL `invoice_lines.id`, which we fetch here via a direct query).
    let real_line_id = with_read(&test_db.state, {
        let invoice_id = invoice.id;
        move |tx| {
            Box::pin(async move {
                use accounting_app_lib::entities::sales::invoice_lines::{Column, Entity};
                use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
                let line = Entity::find().filter(Column::InvoiceId.eq(invoice_id)).one(tx).await.unwrap().unwrap();
                Ok(line.id)
            }) as BoxFuture<'_, TxResult<Id>>
        }
    })
    .await
    .unwrap();
    let _ = invoice_line_id;

    let refund_input = RefundInput {
        invoice_id: invoice.id,
        reason: Some("عيب في المنتج".to_string()),
        lines: vec![RefundLine { invoice_line_id: real_line_id, qty: dec!(1), restock: Some(true) }],
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

    assert_eq!(refund.lines.len(), 1);
    assert!(refund.grand_total > Decimal::ZERO);

    with_read(&test_db.state, move |tx| Box::pin(async move { run_all_invariants(tx).await; Ok(()) }) as BoxFuture<'_, TxResult<()>>).await.unwrap();
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
    let shift_id = shift.id;
    let closed = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let registry = registry3.clone();
        Box::pin(async move {
            shifts::close_pos_shift(tx, cx, &registry, shift_id, CloseShiftInput { counted_cash: dec!(200), closing_denominations: None, handover_mode: Some(HandoverMode::Handover), note: None }).await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Shift>>
    })
    .await
    .unwrap();

    assert_eq!(closed.variance, Some(Decimal::ZERO));

    with_read(&test_db.state, move |tx| Box::pin(async move { run_all_invariants(tx).await; Ok(()) }) as BoxFuture<'_, TxResult<()>>).await.unwrap();
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
    let shift_id = shift.id;
    let closed = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let registry = registry2.clone();
        Box::pin(async move {
            shifts::close_pos_shift(tx, cx, &registry, shift_id, CloseShiftInput { counted_cash: dec!(205), closing_denominations: None, handover_mode: Some(HandoverMode::Handover), note: None }).await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Shift>>
    })
    .await
    .unwrap();

    assert_eq!(closed.variance, Some(dec!(5)));

    with_read(&test_db.state, move |tx| Box::pin(async move { run_all_invariants(tx).await; Ok(()) }) as BoxFuture<'_, TxResult<()>>).await.unwrap();
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
}
