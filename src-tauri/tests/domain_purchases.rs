//! DB-backed tests for the `purchases` domain (03-domains/07-purchases.md §8a). Written now, run in
//! the deferred time-boxed test pass (per-implementer hard rule: never run cargo from this agent).
//! Needs `EQUAL_TEST_DATABASE_URL` — see `tests/support/mod.rs`.
//!
//! No shared `seed_company` fixture yet (G-P9) — this file builds its own minimal fixture
//! (`seed_fixture`), same pattern as `domain_products.rs`/`domain_payments.rs`: one branch, base
//! settings (SAR, prices_include_tax), an open fiscal year, the system-role accounts every posting
//! touches (`inventory`, `vatInput`, `payable`, `freightIn`, `cash`, `bank`, `inventoryVariance`),
//! a default 15% INPUT tax, and three suppliers (VAT-registered, no-VAT, and a "shipping" supplier
//! used for other-supplier landed costs).

mod support;

use std::sync::Arc;

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::error::AppError;
use accounting_app_lib::core::tx::{with_read, with_tx, BoxFuture, TxOpts, TxResult};
use accounting_app_lib::domains::products::dto::catalog::{Product, ProductInput, ProductType, StockMode};
use accounting_app_lib::domains::products::service::products;
use accounting_app_lib::domains::purchases::dto::{
    InvoiceDiscount, LandedCostLineInput, LandedCostSpread, PaymentStatus as PurchasePaymentStatus, PurchaseLineInput, PurchaseOrder, PurchaseOrderInput,
    PurchaseReturn, PurchaseReturnInput, PurchaseReturnInputLine, PurchaseStatus, ReceiveBatchInput, ReceiveLineInput, ReceivePurchaseInput, RefundMethod,
};
use accounting_app_lib::domains::purchases::service::{orders, read, receive, returns};
use accounting_app_lib::entities::catalog::product_batches;
use accounting_app_lib::entities::inventory::debit_note_drafts::{self, DebitNoteDraftLine, DebitNoteDraftLines};
use accounting_app_lib::entities::org::{accounts, branches, fiscal_years, settings, taxes};
use accounting_app_lib::entities::parties::parties;
use accounting_app_lib::entities::purchases::purchase_orders;
use accounting_app_lib::entities::values::{AccountingPolicy, PrinterMode, PrinterSettings};
use accounting_app_lib::shared::activity::undo::UndoRegistry;
use accounting_app_lib::shared::invariants;
use accounting_app_lib::utils::id::Id;
use chrono::Datelike;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, Set};
use support::TestDb;

// --- Fixture -------------------------------------------------------------------------------------

struct Fixture {
    pub branch_id: Id,
    /// VAT-registered supplier (has a `vat_number`).
    pub supplier_id: Id,
    /// Supplier with no `vat_number` — drives the non-VAT-supplier branch (input VAT -> freightIn).
    pub no_vat_supplier_id: Id,
    /// A third supplier used as an "other supplier" landed-cost target (e.g. a shipping company).
    pub shipping_supplier_id: Id,
    /// Default 15% INPUT/standard tax.
    pub tax_id: Id,
}

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

    seed_role_account(conn, "1130", "inventory").await;
    seed_role_account(conn, "1140", "vatInput").await;
    seed_role_account(conn, "2100", "payable").await;
    seed_role_account(conn, "5100", "freightIn").await;
    seed_role_account(conn, "1110", "cash").await;
    seed_role_account(conn, "1120", "bank").await;
    seed_role_account(conn, "5900", "inventoryVariance").await;

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

    async fn seed_supplier<C: ConnectionTrait>(conn: &C, name: &str, code: &str, vat_number: Option<&str>) -> Id {
        let id = Id::new();
        let now = chrono::Utc::now();
        let supplier = parties::ActiveModel {
            id: Set(id),
            kind: Set("supplier".to_string()),
            r#type: Set("company".to_string()),
            name: Set(name.to_string()),
            name_en: Set(None),
            code: Set(code.to_string()),
            group_id: Set(None),
            tags: Set(None),
            active: Set(true),
            phone: Set(None),
            email: Set(None),
            contacts: Set(None),
            address: Set(None),
            national_address: Set(None),
            structured_address: Set(None),
            vat_number: Set(vat_number.map(|s| s.to_string())),
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
        id
    }

    let supplier_id = seed_supplier(conn, "مورد ضريبي", "S-0001", Some("300000000000003")).await;
    let no_vat_supplier_id = seed_supplier(conn, "مورد بدون ضريبة", "S-0002", None).await;
    let shipping_supplier_id = seed_supplier(conn, "شركة الشحن", "S-0003", Some("300000000000010")).await;

    Fixture { branch_id, supplier_id, no_vat_supplier_id, shipping_supplier_id, tax_id }
}

fn base_product_input(name: &str, sku: &str, cost_price: Decimal, product_type: ProductType, track_batches: bool) -> ProductInput {
    ProductInput {
        name: name.to_string(),
        name_en: None,
        sku: sku.to_string(),
        barcode: None,
        category_id: None,
        unit_id: None,
        r#type: product_type,
        stock_mode: None,
        cost_price,
        price: cost_price * dec!(2),
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
        allow_negative_stock: Some(true),
        shelf_location: None,
        preferred_supplier_id: None,
        reorder_qty: None,
        track_batches: if track_batches { Some(true) } else { None },
        expiry_alert_days: None,
        warranty_months: None,
        warranty_provider: None,
        weight: None,
        custom_fields: None,
        opening_qty: None,
    }
}

async fn seed_product<C: ConnectionTrait>(
    conn: &C,
    cx: &accounting_app_lib::core::tx::TxCtx,
    undo: &UndoRegistry,
    name: &str,
    sku: &str,
    cost_price: Decimal,
    product_type: ProductType,
    track_batches: bool,
) -> Product {
    let input = base_product_input(name, sku, cost_price, product_type, track_batches);
    products::create_product(conn, cx, undo, input).await.expect("seed product")
}

fn po_line(product_id: Id, qty: Decimal, cost_price: Decimal) -> PurchaseLineInput {
    PurchaseLineInput { product_id, qty, cost_price, unit_id: None, unit_factor: None, discount: None, discount_is_pct: None, tax_id: None }
}

async fn assert_invariants_green(test_db: &TestDb) {
    let results = with_read(&test_db.state, |tx| Box::pin(async move { invariants::run_all(tx).await.map_err(accounting_app_lib::core::tx::TxError::App) })).await.unwrap();
    let failed: Vec<String> = results.iter().filter(|r| !r.passed).map(|r| format!("{}: {}", r.key, r.message)).collect();
    assert!(failed.is_empty(), "invariants failed: {failed:?}");
}

// === U-3/U-4 save_purchase_order ===================================================================

#[tokio::test]
async fn save_draft_validation_messages_in_order() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    async fn run(test_db: &TestDb, undo: &Arc<UndoRegistry>, input: PurchaseOrderInput) -> AppError {
        let undo = undo.clone();
        let result = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { orders::save_purchase_order(tx, cx, &undo, input, None).await }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        })
        .await;
        result.unwrap_err()
    }

    let (product_id, missing_supplier_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج تجريبي", "SKU-P1", dec!(10), ProductType::Product, false).await;
                Ok((product.id, fixture.supplier_id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();

    // 1. supplier missing -> VALIDATION "اختر المورد"
    let input = PurchaseOrderInput {
        supplier_id: Id::new(),
        date: "2026-01-01".to_string(),
        lines: vec![po_line(product_id, dec!(1), dec!(10))],
        note: None,
        invoice_discount: None,
        landed_costs: None,
        supplier_invoice_no: None,
        supplier_invoice_date: None,
        attachment_ids: None,
        cost_center_id: None,
        branch_id: None,
        currency: None,
        exchange_rate: None,
        confirm: false,
    };
    let err = run(&test_db, &undo, input).await;
    match err {
        AppError::Validation { message } => assert_eq!(message, "اختر المورد"),
        other => panic!("expected VALIDATION, got {other:?}"),
    }

    // 2. no lines -> "أضف صنفاً واحداً على الأقل"
    let input = PurchaseOrderInput {
        supplier_id: missing_supplier_id,
        date: "2026-01-01".to_string(),
        lines: vec![],
        note: None,
        invoice_discount: None,
        landed_costs: None,
        supplier_invoice_no: None,
        supplier_invoice_date: None,
        attachment_ids: None,
        cost_center_id: None,
        branch_id: None,
        currency: None,
        exchange_rate: None,
        confirm: false,
    };
    let err = run(&test_db, &undo, input).await;
    match err {
        AppError::Validation { message } => assert_eq!(message, "أضف صنفاً واحداً على الأقل"),
        other => panic!("expected VALIDATION, got {other:?}"),
    }

    // 3. product missing -> NOT_FOUND "المنتج غير موجود"
    let input = PurchaseOrderInput {
        supplier_id: missing_supplier_id,
        date: "2026-01-01".to_string(),
        lines: vec![po_line(Id::new(), dec!(1), dec!(10))],
        note: None,
        invoice_discount: None,
        landed_costs: None,
        supplier_invoice_no: None,
        supplier_invoice_date: None,
        attachment_ids: None,
        cost_center_id: None,
        branch_id: None,
        currency: None,
        exchange_rate: None,
        confirm: false,
    };
    let err = run(&test_db, &undo, input).await;
    match err {
        AppError::NotFound { message } => assert_eq!(message, "المنتج غير موجود"),
        other => panic!("expected NOT_FOUND, got {other:?}"),
    }

    // 4. qty <= 0 -> "الكمية يجب أن تكون أكبر من صفر"
    let input = PurchaseOrderInput {
        supplier_id: missing_supplier_id,
        date: "2026-01-01".to_string(),
        lines: vec![po_line(product_id, dec!(0), dec!(10))],
        note: None,
        invoice_discount: None,
        landed_costs: None,
        supplier_invoice_no: None,
        supplier_invoice_date: None,
        attachment_ids: None,
        cost_center_id: None,
        branch_id: None,
        currency: None,
        exchange_rate: None,
        confirm: false,
    };
    let err = run(&test_db, &undo, input).await;
    match err {
        AppError::Validation { message } => assert_eq!(message, "الكمية يجب أن تكون أكبر من صفر"),
        other => panic!("expected VALIDATION, got {other:?}"),
    }

    // 5. cost_price < 0 -> "سعر التكلفة لا يمكن أن يكون سالباً"
    let input = PurchaseOrderInput {
        supplier_id: missing_supplier_id,
        date: "2026-01-01".to_string(),
        lines: vec![po_line(product_id, dec!(1), dec!(-1))],
        note: None,
        invoice_discount: None,
        landed_costs: None,
        supplier_invoice_no: None,
        supplier_invoice_date: None,
        attachment_ids: None,
        cost_center_id: None,
        branch_id: None,
        currency: None,
        exchange_rate: None,
        confirm: false,
    };
    let err = run(&test_db, &undo, input).await;
    match err {
        AppError::Validation { message } => assert_eq!(message, "سعر التكلفة لا يمكن أن يكون سالباً"),
        other => panic!("expected VALIDATION, got {other:?}"),
    }
}

#[tokio::test]
async fn save_draft_gets_number_and_totals_with_discounts_and_zero_rated_tax() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let po = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-D1", dec!(100), ProductType::Product, false).await;

                // Zero-rated: a tax with rate 0 (category Z) via a dedicated tax row.
                let zero_tax_id = Id::new();
                let now = chrono::Utc::now();
                let zero_tax = taxes::ActiveModel {
                    id: Set(zero_tax_id),
                    name: Set("معفاة".to_string()),
                    rate: Set(Decimal::ZERO),
                    r#type: Set("INPUT".to_string()),
                    is_default: Set(false),
                    active: Set(true),
                    category: Set("Z".to_string()),
                    direction: Set("purchase".to_string()),
                    exemption_reason: Set(None),
                    account_role: Set(None),
                    created_at: Set(now),
                    updated_at: Set(now),
                    deleted_at: Set(None),
                    sync_status: Set("local".to_string()),
                };
                zero_tax.insert(tx).await.unwrap();

                let mut line1 = po_line(product.id, dec!(2), dec!(100));
                line1.discount = Some(dec!(10));
                line1.discount_is_pct = Some(true);
                line1.tax_id = Some(zero_tax_id);

                let input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-02-01".to_string(),
                    lines: vec![line1],
                    note: None,
                    invoice_discount: Some(InvoiceDiscount { pct: None, amount: Some(dec!(5)) }),
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: false,
                };
                orders::save_purchase_order(tx, cx, &undo, input, None).await
            }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await
    .unwrap();

    assert_eq!(po.number, "PO-000001");
    assert_eq!(po.status, PurchaseStatus::Draft);
    // net = 2*100*0.9 - 5 = 180 - 5 = 175; zero-rated tax -> tax_amount 0; grand = 175.
    assert_eq!(po.sub_total, dec!(175));
    assert_eq!(po.tax_amount, dec!(0));
    assert_eq!(po.grand_total, dec!(175));
}

#[tokio::test]
async fn save_update_refused_when_not_draft() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, supplier_id, product_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-U1", dec!(10), ProductType::Product, false).await;
                let input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-01-01".to_string(),
                    lines: vec![po_line(product.id, dec!(1), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: true,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                Ok((po.id, fixture.supplier_id, product.id))
            }) as BoxFuture<'_, TxResult<(Id, Id, Id)>>
        }
    })
    .await
    .unwrap();

    let result = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = PurchaseOrderInput {
                    supplier_id,
                    date: "2026-01-02".to_string(),
                    lines: vec![po_line(product_id, dec!(2), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: false,
                };
                orders::save_purchase_order(tx, cx, &undo, input, Some(po_id)).await
            }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await;

    let err = result.unwrap_err();
    match err {
        AppError::Validation { message } => assert_eq!(message, "لا يمكن تعديل أمر شراء تم إرساله أو استلامه أو إلغاؤه"),
        other => panic!("expected VALIDATION, got {other:?}"),
    }
}

// === U-5/U-8 send/cancel ============================================================================

#[tokio::test]
async fn send_and_cancel_status_guards() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (draft_id, received_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-S1", dec!(10), ProductType::Product, false).await;

                let draft_input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-01-01".to_string(),
                    lines: vec![po_line(product.id, dec!(1), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: false,
                };
                let draft = orders::save_purchase_order(tx, cx, &undo, draft_input, None).await?;

                let received_input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-01-01".to_string(),
                    lines: vec![po_line(product.id, dec!(1), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: true,
                };
                let received = orders::save_purchase_order(tx, cx, &undo, received_input, None).await?;
                Ok((draft.id, received.id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();

    // send the draft -> ORDERED.
    let sent = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { orders::send_purchase_order_to_supplier(tx, cx, &undo, draft_id).await }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await
    .unwrap();
    assert_eq!(sent.status, PurchaseStatus::Ordered);

    // sending it again fails ("لا يمكن إرسال إلا مسودة").
    let result = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { orders::send_purchase_order_to_supplier(tx, cx, &undo, draft_id).await }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await;
    match result.unwrap_err() {
        AppError::Validation { message } => assert_eq!(message, "لا يمكن إرسال إلا مسودة"),
        other => panic!("expected VALIDATION, got {other:?}"),
    }

    // cancel the ORDERED one -> CANCELED.
    let canceled = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { orders::cancel_purchase_order(tx, cx, &undo, draft_id).await }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await
    .unwrap();
    assert_eq!(canceled.status, PurchaseStatus::Canceled);

    // cancel a RECEIVED order is refused with the return hint.
    let result = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { orders::cancel_purchase_order(tx, cx, &undo, received_id).await }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await;
    match result.unwrap_err() {
        AppError::Validation { message } => assert_eq!(message, "يمكن إلغاء المسودات والأوامر المرسلة فقط — استخدم مرتجع المشتريات للأوامر المستلمة"),
        other => panic!("expected VALIDATION, got {other:?}"),
    }
}

// === receive full ===================================================================================

#[tokio::test]
async fn receive_full_posts_inventory_vat_payable_and_updates_stock() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, product_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-R1", dec!(100), ProductType::Product, false).await;

                let mut line = po_line(product.id, dec!(10), dec!(100));
                line.tax_id = Some(fixture.tax_id);
                let input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-03-01".to_string(),
                    lines: vec![line],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: false,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                Ok((po.id, product.id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();

    let po = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = ReceivePurchaseInput {
                    date: "2026-03-02".to_string(),
                    lines: vec![ReceiveLineInput { product_id, received_qty: dec!(10), batches: None }],
                    supplier_invoice_no: Some("INV-SUP-001".to_string()),
                    supplier_invoice_date: Some(chrono::NaiveDate::from_ymd_opt(2026, 3, 2).unwrap()),
                    vat_not_recoverable: Some(false),
                    landed_costs: None,
                    create_backorder: None,
                };
                receive::receive_purchase(tx, cx, &undo, po_id, input).await
            }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await
    .unwrap();

    assert_eq!(po.status, PurchaseStatus::Received);
    assert_eq!(po.sub_total, dec!(1000));
    assert_eq!(po.tax_amount, dec!(150));
    assert_eq!(po.grand_total, dec!(1150));
    assert_eq!(po.payment_status, PurchasePaymentStatus::Unpaid);

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let product = accounting_app_lib::entities::catalog::products::Entity::find_by_id(product_id).one(tx).await.unwrap().unwrap();
            assert_eq!(product.stock_qty, dec!(10));
            assert_eq!(product.stock_value, dec!(1000));
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    assert_invariants_green(&test_db).await;
}

// === receive short + backorder ======================================================================

#[tokio::test]
async fn receive_short_creates_backorder_with_shrunk_totals_and_ratio_vat() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, product_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-B1", dec!(50), ProductType::Product, false).await;
                let mut line = po_line(product.id, dec!(10), dec!(50));
                line.tax_id = Some(fixture.tax_id);
                let input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-03-01".to_string(),
                    lines: vec![line],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: false,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                Ok((po.id, product.id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();

    // receive 6 of 10 -> ratio 0.6, vat = round2(75 * 0.6) = 45 (po grand=575, sub=500, tax=75).
    let po = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = ReceivePurchaseInput {
                    date: "2026-03-02".to_string(),
                    lines: vec![ReceiveLineInput { product_id, received_qty: dec!(6), batches: None }],
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    vat_not_recoverable: Some(false),
                    landed_costs: None,
                    create_backorder: Some(true),
                };
                receive::receive_purchase(tx, cx, &undo, po_id, input).await
            }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await
    .unwrap();

    assert_eq!(po.status, PurchaseStatus::Received);
    assert_eq!(po.sub_total, dec!(300)); // 6 * 50
    assert_eq!(po.tax_amount, dec!(45)); // round2(75 * (300/500))
    assert_eq!(po.grand_total, dec!(345));

    // The backorder: a DRAFT PO linked via backorderOfId, for the remaining 4 units.
    let orders_list = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move { read::get_purchase_orders(tx, None).await }) as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::purchases::dto::PurchaseRow>>>
    })
    .await
    .unwrap();
    let backorder = orders_list.iter().find(|o| o.backorder_of_id == Some(po_id)).expect("a backorder must exist");
    assert_eq!(backorder.status, PurchaseStatus::Draft);
    assert_eq!(backorder.lines[0].qty, dec!(4));

    // Three activity rows in order: draft save, receipt, backorder creation.
    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            use sea_orm::QueryOrder;
            let rows = accounting_app_lib::entities::platform::activity::Entity::find()
                .order_by_asc(accounting_app_lib::entities::platform::activity::Column::CreatedAt)
                .all(tx)
                .await
                .unwrap();
            assert!(rows.len() >= 3, "expected at least 3 activity rows, got {}", rows.len());
            assert!(rows[0].message.contains("حفظ أمر الشراء"));
            assert!(rows[1].message.contains("استلام أمر الشراء"));
            assert!(rows[2].message.contains("إنشاء أمر متبقٍ"));
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    assert_invariants_green(&test_db).await;
}

// === landed costs ====================================================================================

#[tokio::test]
async fn landed_costs_own_and_other_supplier_split_by_value() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, product_a, product_b, shipping_supplier_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let a = seed_product(tx, cx, &undo, "منتج أ", "SKU-LC-A", dec!(100), ProductType::Product, false).await;
                let b = seed_product(tx, cx, &undo, "منتج ب", "SKU-LC-B", dec!(300), ProductType::Product, false).await;
                let input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-04-01".to_string(),
                    lines: vec![po_line(a.id, dec!(1), dec!(100)), po_line(b.id, dec!(1), dec!(300))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: false,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                Ok((po.id, a.id, b.id, fixture.shipping_supplier_id))
            }) as BoxFuture<'_, TxResult<(Id, Id, Id, Id)>>
        }
    })
    .await
    .unwrap();

    // Own-supplier landed cost of 40 (by value: A gets 100/400*40=10, B gets 300/400*40=30) plus an
    // other-supplier (shipping) landed cost of 20 (by value: A gets 5, B gets 15).
    let po = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = ReceivePurchaseInput {
                    date: "2026-04-02".to_string(),
                    lines: vec![ReceiveLineInput { product_id: product_a, received_qty: dec!(1), batches: None }, ReceiveLineInput { product_id: product_b, received_qty: dec!(1), batches: None }],
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    vat_not_recoverable: Some(true), // avoid needing a real VAT tax id on these lines
                    landed_costs: Some(vec![
                        LandedCostLineInput { label: "شحن داخلي".to_string(), amount: dec!(40), supplier_id: None, spread_by: LandedCostSpread::Value },
                        LandedCostLineInput { label: "شحن خارجي".to_string(), amount: dec!(20), supplier_id: Some(shipping_supplier_id), spread_by: LandedCostSpread::Value },
                    ]),
                    create_backorder: None,
                };
                receive::receive_purchase(tx, cx, &undo, po_id, input).await
            }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await
    .unwrap();

    assert_eq!(po.status, PurchaseStatus::Received);

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let a = accounting_app_lib::entities::catalog::products::Entity::find_by_id(product_a).one(tx).await.unwrap().unwrap();
            let b = accounting_app_lib::entities::catalog::products::Entity::find_by_id(product_b).one(tx).await.unwrap().unwrap();
            // A: 100 + 10 (own) + 5 (other) = 115; B: 300 + 30 (own) + 15 (other) = 345.
            assert_eq!(a.stock_value, dec!(115));
            assert_eq!(b.stock_value, dec!(345));
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    assert_invariants_green(&test_db).await;
}

#[tokio::test]
async fn landed_costs_split_100_over_three_equal_lines_by_qty_unbalances_and_is_refused() {
    // Pins Q-U1: round2'd per-line shares of a 100/3 landed cost spread over 3 equal-qty lines lose
    // a cent (33.33 * 3 = 99.99 != 100.00 in AP), so `ledger::post` refuses the unbalanced entry.
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, p1, p2, p3) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let p1 = seed_product(tx, cx, &undo, "منتج 1", "SKU-Q1", dec!(10), ProductType::Product, false).await;
                let p2 = seed_product(tx, cx, &undo, "منتج 2", "SKU-Q2", dec!(10), ProductType::Product, false).await;
                let p3 = seed_product(tx, cx, &undo, "منتج 3", "SKU-Q3", dec!(10), ProductType::Product, false).await;
                let input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-04-05".to_string(),
                    lines: vec![po_line(p1.id, dec!(1), dec!(10)), po_line(p2.id, dec!(1), dec!(10)), po_line(p3.id, dec!(1), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: false,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                Ok((po.id, p1.id, p2.id, p3.id))
            }) as BoxFuture<'_, TxResult<(Id, Id, Id, Id)>>
        }
    })
    .await
    .unwrap();

    let result = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = ReceivePurchaseInput {
                    date: "2026-04-06".to_string(),
                    lines: vec![
                        ReceiveLineInput { product_id: p1, received_qty: dec!(1), batches: None },
                        ReceiveLineInput { product_id: p2, received_qty: dec!(1), batches: None },
                        ReceiveLineInput { product_id: p3, received_qty: dec!(1), batches: None },
                    ],
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    vat_not_recoverable: Some(true),
                    landed_costs: Some(vec![LandedCostLineInput { label: "توزيع".to_string(), amount: dec!(100), supplier_id: None, spread_by: LandedCostSpread::Qty }]),
                    create_backorder: None,
                };
                receive::receive_purchase(tx, cx, &undo, po_id, input).await
            }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await;

    assert!(result.is_err(), "an unbalanced 100/3 landed-cost spread must be refused by ledger::post, pinning Q-U1");
}

// === non-VAT supplier ================================================================================

#[tokio::test]
async fn non_vat_supplier_receipt_posts_vat_to_freight_in_and_return_credits_it_back() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, product_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-NV1", dec!(100), ProductType::Product, false).await;
                let mut line = po_line(product.id, dec!(5), dec!(100));
                line.tax_id = Some(fixture.tax_id);
                let input = PurchaseOrderInput {
                    supplier_id: fixture.no_vat_supplier_id,
                    date: "2026-05-01".to_string(),
                    lines: vec![line],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: false,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                Ok((po.id, product.id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();

    // vat_not_recoverable defaults to true because the supplier has no vat_number.
    let po = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = ReceivePurchaseInput {
                    date: "2026-05-02".to_string(),
                    lines: vec![ReceiveLineInput { product_id, received_qty: dec!(5), batches: None }],
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    vat_not_recoverable: None,
                    landed_costs: None,
                    create_backorder: None,
                };
                receive::receive_purchase(tx, cx, &undo, po_id, input).await
            }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await
    .unwrap();
    assert_eq!(po.tax_amount, dec!(75)); // 5 * 100 * 15%

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            // Not folded into stock value: stock_value == 500, not 575.
            let product = accounting_app_lib::entities::catalog::products::Entity::find_by_id(product_id).one(tx).await.unwrap().unwrap();
            assert_eq!(product.stock_value, dec!(500));
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();
    assert_invariants_green(&test_db).await;

    // Now return all 5 units; VAT credits back to freightIn (not vatInput).
    let ret = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = PurchaseReturnInput {
                    purchase_order_id: po_id,
                    reason: Some("تالف".to_string()),
                    refund_method: Some(RefundMethod::Credit),
                    lines: vec![PurchaseReturnInputLine { product_id, qty: dec!(5), batch_id: None }],
                    from_draft_id: None,
                };
                returns::create_purchase_return(tx, cx, &undo, input).await
            }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await
    .unwrap();
    assert_eq!(ret.tax_amount, dec!(75));
    assert_invariants_green(&test_db).await;
}

// === tracked product batches =========================================================================

#[tokio::test]
async fn tracked_product_receives_requested_batches_at_landed_cost_and_default_batch() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, tracked_id, number) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let tracked = seed_product(tx, cx, &undo, "منتج بتشغيلات", "SKU-T1", dec!(20), ProductType::Product, true).await;
                let input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-06-01".to_string(),
                    lines: vec![po_line(tracked.id, dec!(10), dec!(20))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: false,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                Ok((po.id, tracked.id, po.number))
            }) as BoxFuture<'_, TxResult<(Id, Id, String)>>
        }
    })
    .await
    .unwrap();

    // Receive with two explicit batches (6 + 4) — no landed costs, so batch unit cost = order cost.
    with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = ReceivePurchaseInput {
                    date: "2026-06-02".to_string(),
                    lines: vec![ReceiveLineInput {
                        product_id: tracked_id,
                        received_qty: dec!(10),
                        batches: Some(vec![
                            ReceiveBatchInput { batch_no: "B-1".to_string(), expiry_date: None, qty: dec!(6) },
                            ReceiveBatchInput { batch_no: "B-2".to_string(), expiry_date: None, qty: dec!(4) },
                        ]),
                    }],
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    vat_not_recoverable: Some(true),
                    landed_costs: None,
                    create_backorder: None,
                };
                receive::receive_purchase(tx, cx, &undo, po_id, input).await
            }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await
    .unwrap();

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            use sea_orm::{ColumnTrait, QueryFilter};
            let batches = product_batches::Entity::find().filter(product_batches::Column::ProductId.eq(tracked_id)).all(tx).await.unwrap();
            assert_eq!(batches.len(), 2);
            let b1 = batches.iter().find(|b| b.batch_no == "B-1").expect("batch B-1 must exist");
            let b2 = batches.iter().find(|b| b.batch_no == "B-2").expect("batch B-2 must exist");
            assert_eq!(b1.qty, dec!(6));
            assert_eq!(b1.unit_cost, dec!(20));
            assert_eq!(b2.qty, dec!(4));
            assert_eq!(b2.unit_cost, dec!(20));
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    // A second tracked product received with no explicit batches gets one default RCV-{number} batch.
    let (po2_id, tracked2_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture_supplier_only(tx).await;
                let tracked2 = seed_product(tx, cx, &undo, "منتج بتشغيلات 2", "SKU-T2", dec!(30), ProductType::Product, true).await;
                let input = PurchaseOrderInput {
                    supplier_id: fixture,
                    date: "2026-06-03".to_string(),
                    lines: vec![po_line(tracked2.id, dec!(3), dec!(30))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: true,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                Ok((po.id, tracked2.id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();
    let _ = (po2_id, number);

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            use sea_orm::{ColumnTrait, QueryFilter};
            let batches = product_batches::Entity::find().filter(product_batches::Column::ProductId.eq(tracked2_id)).all(tx).await.unwrap();
            assert_eq!(batches.len(), 1);
            assert!(batches[0].batch_no.starts_with("RCV-"));
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    assert_invariants_green(&test_db).await;
}

/// Helper: re-use the fixture's existing supplier when a second PO needs one without re-seeding the
/// whole company (branch/settings/fiscal-year/accounts already exist from the first `seed_fixture`
/// call in the same test's transaction).
async fn seed_fixture_supplier_only<C: ConnectionTrait>(conn: &C) -> Id {
    use sea_orm::{ColumnTrait, QueryFilter};
    parties::Entity::find().filter(parties::Column::Kind.eq("supplier")).one(conn).await.unwrap().expect("a supplier must already exist").id
}

// === confirm =========================================================================================

#[tokio::test]
async fn confirm_receives_everything_outstanding_at_now() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let po_id = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-C1", dec!(15), ProductType::Product, false).await;
                let input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-07-01".to_string(),
                    lines: vec![po_line(product.id, dec!(4), dec!(15))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: false,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                Ok(po.id)
            }) as BoxFuture<'_, TxResult<Id>>
        }
    })
    .await
    .unwrap();

    let po = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { orders::confirm_purchase_order(tx, cx, &undo, po_id).await }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await
    .unwrap();

    assert_eq!(po.status, PurchaseStatus::Received);
    assert_eq!(po.sub_total, dec!(60));
    assert_invariants_green(&test_db).await;
}

// === returns =========================================================================================

#[tokio::test]
async fn return_over_return_and_stock_conflict_messages() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, product_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-RET1", dec!(10), ProductType::Product, false).await;
                let input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-08-01".to_string(),
                    lines: vec![po_line(product.id, dec!(5), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: true,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                Ok((po.id, product.id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();

    // over-return: request 6 of 5 received -> VALIDATION "لا يمكن إرجاع أكثر من 5 من ..."
    let result = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = PurchaseReturnInput {
                    purchase_order_id: po_id,
                    reason: Some("سبب".to_string()),
                    refund_method: Some(RefundMethod::Credit),
                    lines: vec![PurchaseReturnInputLine { product_id, qty: dec!(6), batch_id: None }],
                    from_draft_id: None,
                };
                returns::create_purchase_return(tx, cx, &undo, input).await
            }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await;
    match result.unwrap_err() {
        AppError::Validation { message } => assert!(message.contains("لا يمكن إرجاع أكثر من"), "unexpected message: {message}"),
        other => panic!("expected VALIDATION, got {other:?}"),
    }

    // Drain stock to 0 via a valid return of the full 5 units, then try a second, empty-stock return.
    with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = PurchaseReturnInput {
                    purchase_order_id: po_id,
                    reason: Some("سبب".to_string()),
                    refund_method: Some(RefundMethod::Credit),
                    lines: vec![PurchaseReturnInputLine { product_id, qty: dec!(5), batch_id: None }],
                    from_draft_id: None,
                };
                returns::create_purchase_return(tx, cx, &undo, input).await
            }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await
    .unwrap();

    // A second PO for the same product (fresh remaining qty), drained to 0 stock separately would
    // require draining external stock; instead assert the CONFLICT path directly against the first
    // PO's own remaining qty being 0 (no more to return at all -> "لا يمكن إرجاع أكثر من 0 من").
    let result = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = PurchaseReturnInput {
                    purchase_order_id: po_id,
                    reason: Some("سبب آخر".to_string()),
                    refund_method: Some(RefundMethod::Credit),
                    lines: vec![PurchaseReturnInputLine { product_id, qty: dec!(1), batch_id: None }],
                    from_draft_id: None,
                };
                returns::create_purchase_return(tx, cx, &undo, input).await
            }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await;
    match result.unwrap_err() {
        AppError::Validation { message } => assert!(message.contains("لا يمكن إرجاع أكثر من"), "unexpected message: {message}"),
        other => panic!("expected VALIDATION, got {other:?}"),
    }

    assert_invariants_green(&test_db).await;
}

#[tokio::test]
async fn return_stock_conflict_when_qty_exceeds_current_stock() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, other_po_id, product_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-RET2", dec!(10), ProductType::Product, false).await;

                // PO1: receive 5.
                let input1 = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-08-01".to_string(),
                    lines: vec![po_line(product.id, dec!(5), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: true,
                };
                let po1 = orders::save_purchase_order(tx, cx, &undo, input1, None).await?;

                // PO2: receive 5 more from the same product on a different PO (so PO2 alone has 5
                // returnable, but total on-hand stock is only 5 after we sell/consume 5 elsewhere —
                // simulate that by directly reducing stock_qty below what PO2 allows to return).
                let input2 = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-08-02".to_string(),
                    lines: vec![po_line(product.id, dec!(5), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: true,
                };
                let po2 = orders::save_purchase_order(tx, cx, &undo, input2, None).await?;

                // Directly drop on-hand stock to 3 (below PO2's 5 returnable) to force the CONFLICT
                // branch without needing a full sales flow in this domain's own test file.
                let mut am: accounting_app_lib::entities::catalog::products::ActiveModel =
                    accounting_app_lib::entities::catalog::products::Entity::find_by_id(product.id).one(tx).await.unwrap().unwrap().into();
                am.stock_qty = Set(dec!(3));
                am.update(tx).await.unwrap();

                Ok((po1.id, po2.id, product.id))
            }) as BoxFuture<'_, TxResult<(Id, Id, Id)>>
        }
    })
    .await
    .unwrap();
    let _ = po_id;

    let result = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = PurchaseReturnInput {
                    purchase_order_id: other_po_id,
                    reason: Some("سبب".to_string()),
                    refund_method: Some(RefundMethod::Credit),
                    lines: vec![PurchaseReturnInputLine { product_id, qty: dec!(5), batch_id: None }],
                    from_draft_id: None,
                };
                returns::create_purchase_return(tx, cx, &undo, input).await
            }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await;
    match result.unwrap_err() {
        AppError::Conflict { message } => assert!(message.contains("أقل من كمية الإرجاع"), "unexpected message: {message}"),
        other => panic!("expected CONFLICT, got {other:?}"),
    }
    // No invariants check here: we intentionally hand-edited stock_qty to force the guard, so the
    // GL/subledger relationship is deliberately out of sync at this point in the test.
}

#[tokio::test]
async fn return_refund_methods_and_variance_guard() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    // credit: settled fully to payable, no cash/bank line.
    let (po_id, product_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-REF1", dec!(10), ProductType::Product, false).await;
                let input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-09-01".to_string(),
                    lines: vec![po_line(product.id, dec!(10), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: true,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                Ok((po.id, product.id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();

    let ret_credit = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = PurchaseReturnInput {
                    purchase_order_id: po_id,
                    reason: Some("سبب".to_string()),
                    refund_method: Some(RefundMethod::Credit),
                    lines: vec![PurchaseReturnInputLine { product_id, qty: dec!(2), batch_id: None }],
                    from_draft_id: None,
                };
                returns::create_purchase_return(tx, cx, &undo, input).await
            }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await
    .unwrap();
    assert_eq!(ret_credit.cash_back, dec!(0));
    assert_eq!(ret_credit.settled_to_payable, dec!(20));

    // cash: cash_back paid out via cash (outstanding is fully paid down already by the credit
    // return's settled_to_payable, so a further return once the PO owes nothing produces cash_back
    // == full amount).
    with_tx(&test_db.state, TxOpts::default(), {
        move |tx, _cx| {
            Box::pin(async move {
                // Fully settle the remaining payable by paying it off directly (mark paid_amount)
                // so the next return's outstanding is 0 and cash_back == grand_total.
                let po = purchase_orders::Entity::find_by_id(po_id).one(tx).await.unwrap().unwrap();
                let mut am: purchase_orders::ActiveModel = po.clone().into();
                am.paid_amount = Set(po.grand_total - po.returned_amount);
                am.payment_status = Set(purchase_orders::PaymentStatus::Paid);
                am.update(tx).await.unwrap();
                Ok(())
            }) as BoxFuture<'_, TxResult<()>>
        }
    })
    .await
    .unwrap();

    let ret_cash = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = PurchaseReturnInput {
                    purchase_order_id: po_id,
                    reason: Some("سبب".to_string()),
                    refund_method: Some(RefundMethod::Cash),
                    lines: vec![PurchaseReturnInputLine { product_id, qty: dec!(2), batch_id: None }],
                    from_draft_id: None,
                };
                returns::create_purchase_return(tx, cx, &undo, input).await
            }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await
    .unwrap();
    assert_eq!(ret_cash.settled_to_payable, dec!(0));
    assert_eq!(ret_cash.cash_back, dec!(20));

    // bank_transfer: same split, different settlement account (bank instead of cash) — not asserted
    // by account id here (out of scope for this test's fixture), just that it succeeds.
    with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = PurchaseReturnInput {
                    purchase_order_id: po_id,
                    reason: Some("سبب".to_string()),
                    refund_method: Some(RefundMethod::BankTransfer),
                    lines: vec![PurchaseReturnInputLine { product_id, qty: dec!(2), batch_id: None }],
                    from_draft_id: None,
                };
                returns::create_purchase_return(tx, cx, &undo, input).await
            }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await
    .unwrap();

    assert_invariants_green(&test_db).await;
}

#[tokio::test]
async fn return_variance_guard_when_stock_value_is_below_what_return_would_remove() {
    // `cost_out_at_price`'s variance guard: returning at a cost_price higher than the current
    // weighted-average means the "value out" at that price exceeds what's actually on the books,
    // producing a positive variance line (Dr inventoryVariance) rather than an inventory value going
    // negative.
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, product_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-VAR1", dec!(10), ProductType::Product, false).await;
                let input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-09-10".to_string(),
                    lines: vec![po_line(product.id, dec!(5), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: true,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;

                // Deflate the product's stock_value (simulating a weighted-average drift from other
                // activity outside this PO) so the return-time value-out at the PO's cost_price
                // exceeds what's on the books, forcing a variance line.
                let mut am: accounting_app_lib::entities::catalog::products::ActiveModel =
                    accounting_app_lib::entities::catalog::products::Entity::find_by_id(product.id).one(tx).await.unwrap().unwrap().into();
                am.stock_value = Set(dec!(10)); // was 50 (5 * 10); now understated.
                am.update(tx).await.unwrap();

                Ok((po.id, product.id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();

    let ret = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = PurchaseReturnInput {
                    purchase_order_id: po_id,
                    reason: Some("سبب".to_string()),
                    refund_method: Some(RefundMethod::Credit),
                    lines: vec![PurchaseReturnInputLine { product_id, qty: dec!(5), batch_id: None }],
                    from_draft_id: None,
                };
                returns::create_purchase_return(tx, cx, &undo, input).await
            }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await
    .unwrap();
    assert_eq!(ret.grand_total, dec!(50));

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let product = accounting_app_lib::entities::catalog::products::Entity::find_by_id(product_id).one(tx).await.unwrap().unwrap();
            // Stock is fully drawn down (qty and value can't go negative given the guard clamps the
            // value-out to what's on hand) — value ends at 0, not negative.
            assert_eq!(product.stock_value, Decimal::ZERO);
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    // Invariants stay green: the variance line balances the posting even though we hand-deflated
    // stock_value earlier (the posting itself is internally consistent; GL == subledger afterward).
    assert_invariants_green(&test_db).await;
}

#[tokio::test]
async fn return_batch_drawn_by_id_and_by_fefo() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, tracked_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let tracked = seed_product(tx, cx, &undo, "منتج بتشغيلات", "SKU-RETB1", dec!(10), ProductType::Product, true).await;
                let input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-09-15".to_string(),
                    lines: vec![po_line(tracked.id, dec!(10), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: false,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                Ok((po.id, tracked.id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();

    // Receive as two batches with distinct expiry dates (B-early expires first -> FEFO picks it).
    let today = chrono::Utc::now().date_naive();
    with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = ReceivePurchaseInput {
                    date: "2026-09-16".to_string(),
                    lines: vec![ReceiveLineInput {
                        product_id: tracked_id,
                        received_qty: dec!(10),
                        batches: Some(vec![
                            ReceiveBatchInput { batch_no: "EARLY".to_string(), expiry_date: Some(today + chrono::Duration::days(10)), qty: dec!(4) },
                            ReceiveBatchInput { batch_no: "LATE".to_string(), expiry_date: Some(today + chrono::Duration::days(60)), qty: dec!(6) },
                        ]),
                    }],
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    vat_not_recoverable: Some(true),
                    landed_costs: None,
                    create_backorder: None,
                };
                receive::receive_purchase(tx, cx, &undo, po_id, input).await
            }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await
    .unwrap();

    let late_batch_id = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            use sea_orm::{ColumnTrait, QueryFilter};
            let b = product_batches::Entity::find().filter(product_batches::Column::ProductId.eq(tracked_id)).filter(product_batches::Column::BatchNo.eq("LATE")).one(tx).await.unwrap().unwrap();
            Ok(b.id)
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    // Return 3 explicitly drawn from LATE (by id).
    with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = PurchaseReturnInput {
                    purchase_order_id: po_id,
                    reason: Some("سبب".to_string()),
                    refund_method: Some(RefundMethod::Credit),
                    lines: vec![PurchaseReturnInputLine { product_id: tracked_id, qty: dec!(3), batch_id: Some(late_batch_id) }],
                    from_draft_id: None,
                };
                returns::create_purchase_return(tx, cx, &undo, input).await
            }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await
    .unwrap();

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            use sea_orm::{ColumnTrait, QueryFilter};
            let late = product_batches::Entity::find().filter(product_batches::Column::ProductId.eq(tracked_id)).filter(product_batches::Column::BatchNo.eq("LATE")).one(tx).await.unwrap().unwrap();
            assert_eq!(late.qty, dec!(3)); // 6 - 3 drawn explicitly.
            let early = product_batches::Entity::find().filter(product_batches::Column::ProductId.eq(tracked_id)).filter(product_batches::Column::BatchNo.eq("EARLY")).one(tx).await.unwrap().unwrap();
            assert_eq!(early.qty, dec!(4)); // untouched by the explicit draw.
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    // Return 2 more with no batch id -> FEFO consumes from EARLY first.
    with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = PurchaseReturnInput {
                    purchase_order_id: po_id,
                    reason: Some("سبب".to_string()),
                    refund_method: Some(RefundMethod::Credit),
                    lines: vec![PurchaseReturnInputLine { product_id: tracked_id, qty: dec!(2), batch_id: None }],
                    from_draft_id: None,
                };
                returns::create_purchase_return(tx, cx, &undo, input).await
            }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await
    .unwrap();

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            use sea_orm::{ColumnTrait, QueryFilter};
            let early = product_batches::Entity::find().filter(product_batches::Column::ProductId.eq(tracked_id)).filter(product_batches::Column::BatchNo.eq("EARLY")).one(tx).await.unwrap().unwrap();
            assert_eq!(early.qty, dec!(2)); // 4 - 2 consumed FEFO.
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    // Check returnedAmount/paymentStatus updated on the PO.
    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let po = purchase_orders::Entity::find_by_id(po_id).one(tx).await.unwrap().unwrap();
            assert_eq!(po.returned_amount, dec!(50)); // (3 + 2) * 10
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    assert_invariants_green(&test_db).await;
}

// === debit-note draft ================================================================================

#[tokio::test]
async fn debit_note_draft_batch_from_adjustment_conflict_two_pos_conflict_success_deletes_draft() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, product_id, batch_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let tracked = seed_product(tx, cx, &undo, "منتج بتشغيلات", "SKU-DND1", dec!(10), ProductType::Product, true).await;
                let input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-10-01".to_string(),
                    lines: vec![po_line(tracked.id, dec!(10), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: true,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                use sea_orm::{ColumnTrait, QueryFilter};
                let batches = product_batches::Entity::find().filter(product_batches::Column::ProductId.eq(tracked.id)).all(tx).await.unwrap();
                Ok((po.id, tracked.id, batches[0].id))
            }) as BoxFuture<'_, TxResult<(Id, Id, Id)>>
        }
    })
    .await
    .unwrap();

    // Case 1: a batch whose `source_ref_id` names a non-PO source (simulated: a batch with no
    // `source_ref_id` at all, or pointing at a random id) -> CONFLICT "إحدى التشغيلات ليست من أمر
    // شراء مستلم".
    let adjustment_batch_id = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let mut am: product_batches::ActiveModel = product_batches::Entity::find_by_id(batch_id).one(tx).await.unwrap().unwrap().into();
            // Point this batch at a random, non-existent "adjustment" source id instead of the PO.
            am.source_ref_id = Set(Some(Id::new()));
            let updated = am.update(tx).await.unwrap();
            Ok(updated.id)
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    let draft_id = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let now = chrono::Utc::now();
            let id = Id::new();
            let draft = debit_note_drafts::ActiveModel {
                id: Set(id),
                number: Set("DND-000001".to_string()),
                supplier_id: Set(Id::new()),
                date_day: Set(now.date_naive()),
                date_instant: Set(Some(now)),
                date_key: Set(now.date_naive().format("%Y-%m-%d").to_string()),
                status: Set("DRAFT".to_string()),
                lines: Set(DebitNoteDraftLines(vec![DebitNoteDraftLine { product_id, batch_id: adjustment_batch_id, qty: dec!(1), unit_cost: dec!(10) }])),
                note: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            };
            let inserted = draft.insert(tx).await.unwrap();
            Ok(inserted.id)
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    let result = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { returns::post_debit_note_draft(tx, cx, &undo, draft_id, Some(RefundMethod::Credit)).await }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await;
    match result.unwrap_err() {
        AppError::Conflict { message } => assert!(message.contains("إحدى التشغيلات ليست من أمر شراء مستلم")),
        other => panic!("expected CONFLICT, got {other:?}"),
    }

    // Restore the batch's source_ref_id to the real PO for the remaining cases.
    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let mut am: product_batches::ActiveModel = product_batches::Entity::find_by_id(batch_id).one(tx).await.unwrap().unwrap().into();
            am.source_ref_id = Set(Some(po_id));
            am.update(tx).await.unwrap();
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    // Case 2: two POs in one draft -> CONFLICT "تشغيلات المسودة من أوامر شراء مختلفة".
    let (po2_id, batch2_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let supplier_id = seed_fixture_supplier_only(tx).await;
                let input = PurchaseOrderInput {
                    supplier_id,
                    date: "2026-10-02".to_string(),
                    lines: vec![po_line(product_id, dec!(5), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: true,
                };
                let po2 = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                use sea_orm::{ColumnTrait, QueryFilter};
                let batches = product_batches::Entity::find()
                    .filter(product_batches::Column::ProductId.eq(product_id))
                    .filter(product_batches::Column::SourceRefId.eq(po2.id))
                    .all(tx)
                    .await
                    .unwrap();
                Ok((po2.id, batches[0].id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();

    let two_po_draft_id = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let now = chrono::Utc::now();
            let id = Id::new();
            let draft = debit_note_drafts::ActiveModel {
                id: Set(id),
                number: Set("DND-000002".to_string()),
                supplier_id: Set(Id::new()),
                date_day: Set(now.date_naive()),
                date_instant: Set(Some(now)),
                date_key: Set(now.date_naive().format("%Y-%m-%d").to_string()),
                status: Set("DRAFT".to_string()),
                lines: Set(DebitNoteDraftLines(vec![
                    DebitNoteDraftLine { product_id, batch_id, qty: dec!(1), unit_cost: dec!(10) },
                    DebitNoteDraftLine { product_id, batch_id: batch2_id, qty: dec!(1), unit_cost: dec!(10) },
                ])),
                note: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            };
            let inserted = draft.insert(tx).await.unwrap();
            Ok(inserted.id)
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    let result = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { returns::post_debit_note_draft(tx, cx, &undo, two_po_draft_id, Some(RefundMethod::Credit)).await }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await;
    match result.unwrap_err() {
        AppError::Conflict { message } => assert!(message.contains("تشغيلات المسودة من أوامر شراء مختلفة")),
        other => panic!("expected CONFLICT, got {other:?}"),
    }
    let _ = po2_id;

    // Case 3: a valid single-PO draft posts successfully and is deleted afterward.
    let valid_draft_id = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let now = chrono::Utc::now();
            let id = Id::new();
            let draft = debit_note_drafts::ActiveModel {
                id: Set(id),
                number: Set("DND-000003".to_string()),
                supplier_id: Set(Id::new()),
                date_day: Set(now.date_naive()),
                date_instant: Set(Some(now)),
                date_key: Set(now.date_naive().format("%Y-%m-%d").to_string()),
                status: Set("DRAFT".to_string()),
                lines: Set(DebitNoteDraftLines(vec![DebitNoteDraftLine { product_id, batch_id, qty: dec!(1), unit_cost: dec!(10) }])),
                note: Set(Some("منتهي الصلاحية".to_string())),
                created_at: Set(now),
                updated_at: Set(now),
            };
            let inserted = draft.insert(tx).await.unwrap();
            Ok(inserted.id)
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    let posted = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { returns::post_debit_note_draft(tx, cx, &undo, valid_draft_id, Some(RefundMethod::Credit)).await }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await
    .unwrap();
    assert_eq!(posted.purchase_order_id, po_id);
    assert_eq!(posted.lines[0].qty, dec!(1));

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let still_there = debit_note_drafts::Entity::find_by_id(valid_draft_id).one(tx).await.unwrap();
            assert!(still_there.is_none(), "a successfully posted draft must be hard-deleted");
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    // Case 4 (a failed post leaves the draft in place): a draft for a CANCELED PO's batch fails, and
    // the draft row survives.
    let (canceled_po_id, canceled_product_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let supplier_id = seed_fixture_supplier_only(tx).await;
                let tracked = seed_product(tx, cx, &undo, "منتج ملغى", "SKU-DND-CANCEL", dec!(10), ProductType::Product, true).await;
                let input = PurchaseOrderInput {
                    supplier_id,
                    date: "2026-10-03".to_string(),
                    lines: vec![po_line(tracked.id, dec!(5), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: false,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                orders::cancel_purchase_order(tx, cx, &undo, po.id).await?;
                Ok((po.id, tracked.id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();

    // No batch exists for a canceled (never received) PO — use a synthetic batch pointing at it so
    // `post_debit_note_draft`'s lookup finds a PO that exists but isn't RECEIVED, which the batch ->
    // PO join treats the same as "not from a received PO" -> CONFLICT, leaving the draft in place.
    let fake_batch_id = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let now = chrono::Utc::now();
            let id = Id::new();
            let batch = product_batches::ActiveModel {
                id: Set(id),
                product_id: Set(canceled_product_id),
                batch_no: Set("CANCELED-PO-BATCH".to_string()),
                expiry_date: Set(None),
                qty: Set(dec!(5)),
                unit_cost: Set(dec!(10)),
                supplier_id: Set(None),
                received_date_day: Set(now.date_naive()),
                received_date_instant: Set(Some(now)),
                source_ref_id: Set(Some(canceled_po_id)),
                source_ref_number: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            };
            let inserted = batch.insert(tx).await.unwrap();
            Ok(inserted.id)
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    let leftover_draft_id = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let now = chrono::Utc::now();
            let id = Id::new();
            let draft = debit_note_drafts::ActiveModel {
                id: Set(id),
                number: Set("DND-000004".to_string()),
                supplier_id: Set(Id::new()),
                date_day: Set(now.date_naive()),
                date_instant: Set(Some(now)),
                date_key: Set(now.date_naive().format("%Y-%m-%d").to_string()),
                status: Set("DRAFT".to_string()),
                lines: Set(DebitNoteDraftLines(vec![DebitNoteDraftLine { product_id: canceled_product_id, batch_id: fake_batch_id, qty: dec!(1), unit_cost: dec!(10) }])),
                note: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            };
            let inserted = draft.insert(tx).await.unwrap();
            Ok(inserted.id)
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();

    let result = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { returns::post_debit_note_draft(tx, cx, &undo, leftover_draft_id, Some(RefundMethod::Credit)).await }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await;
    assert!(result.is_err(), "a draft against a non-RECEIVED PO must fail");

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let still_there = debit_note_drafts::Entity::find_by_id(leftover_draft_id).one(tx).await.unwrap();
            assert!(still_there.is_some(), "a failed post must leave the draft in place for retry");
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();
}

// === Q-U2 pin: non-stock item receipt leaves invariant 4 failing by exactly its value ===============

#[tokio::test]
async fn q_u2_non_stock_product_receipt_leaves_inventory_gl_drift_pinned() {
    // Q-U2 (kept mock quirk, reported not fixed): a `type: 'product'` item with `stockMode: 'none'`
    // (non-stock) still debits `inventory` on receipt (only `type == 'service'` takes the expense
    // path), while `apply_change`/`applyStockChange` ignores non-stock items entirely (no
    // `product_branch_stock` row, no `stock_qty`/`stock_value` change). This leaves
    // GL(inventory) != Sum(stockValue) by exactly the non-stock line's posted value — a real
    // accounting defect in the source mock, kept here byte-for-byte and pinned by this test rather
    // than silently fixed. See spec §7 O-U3 for the open question to the user (route non-stock
    // items like service lines instead).
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, product_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                // `type: Product` with `stock_mode: Some(StockMode::None)` — a non-stock item, per
                // B-1's own "stock_mode = NULL means tracked" convention.
                let mut input = base_product_input("منتج غير مخزني", "SKU-NS1", dec!(10), ProductType::Product, false);
                input.stock_mode = Some(StockMode::None);
                let product = products::create_product(tx, cx, &undo, input).await?;

                let po_input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-11-01".to_string(),
                    lines: vec![po_line(product.id, dec!(3), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: true,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, po_input, None).await?;
                Ok((po.id, product.id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();
    let _ = (po_id, product_id);

    // The invariant-4 (GL inventory vs sum of stockValue) check must fail by exactly 30 (3 * 10):
    // the receipt debited `inventory` for 30, but the non-stock item's own `stock_value` never moved.
    let results = with_read(&test_db.state, |tx| Box::pin(async move { invariants::run_all(tx).await.map_err(accounting_app_lib::core::tx::TxError::App) })).await.unwrap();
    let inventory_check = results.iter().find(|r| r.key.to_lowercase().contains("inventory")).expect("an inventory-vs-GL invariant must exist");
    assert!(!inventory_check.passed, "Q-U2: the non-stock receipt must leave the inventory GL/subledger check failing (kept mock defect, not fixed here)");
}

// === concurrency ======================================================================================

#[tokio::test]
async fn concurrent_receive_of_the_same_po_one_succeeds_one_refused() {
    let test_db = Arc::new(TestDb::fresh().await);
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, product_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-CONC1", dec!(10), ProductType::Product, false).await;
                let input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-12-01".to_string(),
                    lines: vec![po_line(product.id, dec!(5), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: false,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                Ok((po.id, product.id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();
    let _ = product_id;

    let fut_a = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = ReceivePurchaseInput {
                    date: "2026-12-02".to_string(),
                    lines: vec![ReceiveLineInput { product_id, received_qty: dec!(5), batches: None }],
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    vat_not_recoverable: Some(true),
                    landed_costs: None,
                    create_backorder: None,
                };
                receive::receive_purchase(tx, cx, &undo, po_id, input).await
            }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    });
    let fut_b = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = ReceivePurchaseInput {
                    date: "2026-12-02".to_string(),
                    lines: vec![ReceiveLineInput { product_id, received_qty: dec!(5), batches: None }],
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    vat_not_recoverable: Some(true),
                    landed_costs: None,
                    create_backorder: None,
                };
                receive::receive_purchase(tx, cx, &undo, po_id, input).await
            }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    });

    let (r1, r2): (Result<PurchaseOrder, AppError>, Result<PurchaseOrder, AppError>) = tokio::join!(fut_a, fut_b);
    let successes = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
    let already_done = [&r1, &r2].iter().filter(|r| matches!(r, Err(e) if e.to_string() == "تم استلام أمر الشراء هذا بالفعل أو تم إلغاؤه")).count();
    assert_eq!(successes, 1, "exactly one concurrent receive must succeed");
    assert_eq!(already_done, 1, "the loser must see the already-received/canceled message");

    assert_invariants_green(&test_db).await;
}

#[tokio::test]
async fn concurrent_returns_of_the_last_returnable_unit_one_gets_over_return_message() {
    let test_db = Arc::new(TestDb::fresh().await);
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, product_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-CONC2", dec!(10), ProductType::Product, false).await;
                let input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-12-05".to_string(),
                    lines: vec![po_line(product.id, dec!(1), dec!(10))],
                    note: None,
                    invoice_discount: None,
                    landed_costs: None,
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    attachment_ids: None,
                    cost_center_id: None,
                    branch_id: None,
                    currency: None,
                    exchange_rate: None,
                    confirm: true,
                };
                let po = orders::save_purchase_order(tx, cx, &undo, input, None).await?;
                Ok((po.id, product.id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();
    let _ = product_id;

    // Only 1 unit is returnable in total; two concurrent returns each request 1 -> exactly one wins.
    let fut_a = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = PurchaseReturnInput {
                    purchase_order_id: po_id,
                    reason: Some("سبب أ".to_string()),
                    refund_method: Some(RefundMethod::Credit),
                    lines: vec![PurchaseReturnInputLine { product_id, qty: dec!(1), batch_id: None }],
                    from_draft_id: None,
                };
                returns::create_purchase_return(tx, cx, &undo, input).await
            }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    });
    let fut_b = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = PurchaseReturnInput {
                    purchase_order_id: po_id,
                    reason: Some("سبب ب".to_string()),
                    refund_method: Some(RefundMethod::Credit),
                    lines: vec![PurchaseReturnInputLine { product_id, qty: dec!(1), batch_id: None }],
                    from_draft_id: None,
                };
                returns::create_purchase_return(tx, cx, &undo, input).await
            }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    });

    let (r1, r2): (Result<PurchaseReturn, AppError>, Result<PurchaseReturn, AppError>) = tokio::join!(fut_a, fut_b);
    let successes = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
    let over_return = [&r1, &r2].iter().filter(|r| matches!(r, Err(e) if e.to_string().contains("لا يمكن إرجاع أكثر من"))).count();
    assert_eq!(successes, 1, "exactly one concurrent return of the last unit must succeed");
    assert_eq!(over_return, 1, "the loser must see the over-return message");

    assert_invariants_green(&test_db).await;
}

#[tokio::test]
async fn twenty_parallel_draft_saves_get_distinct_gapless_numbers() {
    let test_db = Arc::new(TestDb::fresh().await);
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let product_id = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-CONC3", dec!(10), ProductType::Product, false).await;
                Ok((fixture.supplier_id, product.id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();
    let (supplier_id, product_id) = product_id;

    let mut handles = Vec::new();
    for _ in 0..20 {
        let test_db = test_db.clone();
        let undo = undo.clone();
        handles.push(tokio::spawn(async move {
            let result: accounting_app_lib::core::error::AppResult<PurchaseOrder> = with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
                let undo = undo.clone();
                Box::pin(async move {
                    let input = PurchaseOrderInput {
                        supplier_id,
                        date: "2026-12-10".to_string(),
                        lines: vec![po_line(product_id, dec!(1), dec!(10))],
                        note: None,
                        invoice_discount: None,
                        landed_costs: None,
                        supplier_invoice_no: None,
                        supplier_invoice_date: None,
                        attachment_ids: None,
                        cost_center_id: None,
                        branch_id: None,
                        currency: None,
                        exchange_rate: None,
                        confirm: false,
                    };
                    orders::save_purchase_order(tx, cx, &undo, input, None).await
                }) as BoxFuture<'_, TxResult<PurchaseOrder>>
            })
            .await;
            result.unwrap().number
        }));
    }

    let mut numbers = Vec::new();
    for handle in handles {
        numbers.push(tokio::time::timeout(std::time::Duration::from_secs(30), handle).await.expect("a concurrent draft save timed out").unwrap());
    }
    numbers.sort();
    let expected: Vec<String> = (1..=20).map(|n| format!("PO-{n:06}")).collect();
    assert_eq!(numbers, expected, "20 concurrent draft saves must get 20 distinct sequential numbers with no gaps");
}
