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

use crate::support;

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
use accounting_app_lib::core::tx::TxCtx;
use accounting_app_lib::entities::journal::journal_entries::JournalEntryType;
use accounting_app_lib::shared::invariants;
use accounting_app_lib::shared::ledger::accounts::SystemRole;
use accounting_app_lib::shared::ledger::post::{self as ledger_post, AccountRef, PostJournal, PostingLine};
use accounting_app_lib::shared::stock::{apply_change, lock_product, StockRef};
use accounting_app_lib::utils::dates::DocDate;
use accounting_app_lib::utils::id::Id;
use chrono::Datelike;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, Set};
use support::TestDb;

// --- Fixture -------------------------------------------------------------------------------------

#[allow(dead_code)] // fields kept for tests still to be written
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

/// The logged-in test user. A fixed id (each test has its own database) so `log_in` can run
/// before the fixture and the fixture can still insert the matching `users` row — `audit.user_id`
/// and the other actor columns are FKs to `users`, exactly like the real app, where a session
/// always belongs to an existing user.
fn test_user_id() -> Id {
    "01900000-0000-7000-8000-00000000a001".parse().unwrap()
}

async fn seed_test_user<C: sea_orm::ConnectionTrait>(conn: &C, branch_id: Id) {
    let now = chrono::Utc::now();
    accounting_app_lib::entities::org::users::ActiveModel {
        id: Set(test_user_id()),
        username: Set("test".to_string()),
        name: Set("test".to_string()),
        phone: Set(None),
        role: Set("admin".to_string()),
        max_discount: Set(rust_decimal::Decimal::ZERO),
        price_list_id: Set(None),
        active: Set(true),
        avatar: Set(None),
        allowed_branches: Set(None),
        home_branch: Set(Some(branch_id)),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    }
    .insert(conn)
    .await
    .unwrap();
}

fn log_in(test_db: &TestDb) -> Id {
    let user_id = test_user_id();
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
    seed_test_user(conn, branch_id).await;

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

/// The fixture's VAT-registered supplier (`S-0001`), for tests that insert a `debit_note_drafts`
/// row by hand outside the fixture's transaction (`supplier_id` is an FK to `parties`).
async fn fixture_supplier_id(tx: &DatabaseTransaction) -> Id {
    use sea_orm::{ColumnTrait, QueryFilter};
    parties::Entity::find().filter(parties::Column::Code.eq("S-0001")).one(tx).await.unwrap().expect("fixture supplier S-0001").id
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

fn day(date: &str) -> DocDate {
    DocDate { day: chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(), instant: None }
}

/// A stand-in for stock that left at its carrying value outside this domain (a sale's COGS, a
/// write-down): `Dr <debit_role> amount / Cr inventory amount`, so GL(inventory) keeps matching
/// Σ stock_value when a test moves stock by hand.
async fn post_stand_in_entry(tx: &DatabaseTransaction, cx: &TxCtx, date: &str, debit_role: SystemRole, amount: Decimal) -> TxResult<()> {
    let lines = vec![PostingLine::debit(AccountRef::Role(debit_role), amount), PostingLine::credit(AccountRef::Role(SystemRole::Inventory), amount)];
    ledger_post::post(tx, cx, PostJournal::new(day(date), "حركة مخزون خارج المشتريات", JournalEntryType::System, lines)).await?;
    Ok(())
}

/// (Σ debit, Σ credit) of every journal line on the account holding `role`.
async fn role_totals(tx: &DatabaseTransaction, role: &str) -> (Decimal, Decimal) {
    use accounting_app_lib::entities::journal::journal_lines;
    use sea_orm::{ColumnTrait, QueryFilter};
    let account = accounts::Entity::find().filter(accounts::Column::SystemRole.eq(role)).one(tx).await.unwrap().unwrap();
    let rows = journal_lines::Entity::find().filter(journal_lines::Column::AccountId.eq(account.id)).all(tx).await.unwrap();
    rows.iter().fold((Decimal::ZERO, Decimal::ZERO), |(d, c), l| (d + l.debit, c + l.credit))
}

fn po_input(supplier_id: Id, date: &str, lines: Vec<PurchaseLineInput>, confirm: bool) -> PurchaseOrderInput {
    PurchaseOrderInput {
        supplier_id,
        date: date.to_string(),
        lines,
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
        confirm,
    }
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
    test_db.finish().await;
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
    test_db.finish().await;
}

/// m0020 regression (Part 04 parity L2, `p-p6-tracked-batches`): a line's `unitId` is the product's
/// own `ProductUnit.id` (a free string like `pu-panadol-box`), not a `units` row — it used to hit
/// `fk_purchase_order_lines_unit_id` (CONFLICT). It must be stored and read back verbatim.
#[tokio::test]
async fn non_base_unit_line_keeps_its_product_unit_id() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let po = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج بالعلبة", "SKU-U1", dec!(6), ProductType::Product, false).await;
                let mut line = po_line(product.id, dec!(10), dec!(18.5));
                line.unit_id = Some("pu-test-box".to_string());
                line.unit_factor = Some(dec!(3));
                let input = PurchaseOrderInput {
                    supplier_id: fixture.supplier_id,
                    date: "2026-02-01".to_string(),
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
                orders::save_purchase_order(tx, cx, &undo, input, None).await
            }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await
    .expect("a non-base-unit line must save");

    assert_eq!(po.lines[0].unit_id.as_deref(), Some("pu-test-box"));
    let id = po.id;
    let detail = with_read(&test_db.state, move |tx| Box::pin(async move { read::get_purchase_order(tx, id).await })).await.unwrap();
    assert_eq!(detail.lines[0].unit_id.as_deref(), Some("pu-test-box"));
    assert_eq!(detail.lines[0].unit_factor, Some(dec!(3)));
    test_db.finish().await;
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
    test_db.finish().await;
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
    test_db.finish().await;
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
                    supplier_invoice_date: Some("2026-03-02".to_string()),
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
    test_db.finish().await;
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

    // Four purchase activity rows in order: draft save, receipt, backorder draft save, backorder
    // creation. Filtered to
    // `kind = purchase` (the fixture's `create_product` logs its own `product` row first) and
    // ordered by (created_at, id) — the codebase's insertion order (UUIDv7 ids), since the receipt
    // and the backorder share one transaction clock.
    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            use accounting_app_lib::entities::platform::activity;
            use sea_orm::{ColumnTrait, QueryFilter, QueryOrder};
            let rows = activity::Entity::find()
                .filter(activity::Column::Kind.eq(activity::ActivityKind::Purchase))
                .order_by_asc(activity::Column::CreatedAt)
                .order_by_asc(activity::Column::Id)
                .all(tx)
                .await
                .unwrap();
            let messages: Vec<&str> = rows.iter().map(|r| r.message.as_str()).collect();
            // The backorder is created through the ordinary draft save (the mock's
            // `savePurchase(...)` inside `applyReceipt`), so its own "saved as draft" row comes
            // right before the "backorder created" row.
            assert_eq!(rows.len(), 4, "{messages:?}");
            assert!(rows[0].message.contains("حفظ أمر الشراء PO-000001"), "{messages:?}");
            assert!(rows[1].message.contains("استلام أمر الشراء PO-000001"), "{messages:?}");
            assert!(rows[2].message.contains("حفظ أمر الشراء PO-000002"), "{messages:?}");
            assert!(rows[3].message.contains("إنشاء أمر متبقٍ PO-000002"), "{messages:?}");
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    assert_invariants_green(&test_db).await;
    test_db.finish().await;
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
    test_db.finish().await;
}

#[tokio::test]
async fn landed_costs_split_100_over_three_equal_lines_puts_remainder_on_first_largest_line() {
    // ACC-0004: round2'd per-line shares of a 100/3 landed cost (33.33 * 3 = 99.99) used to leave
    // the receipt unbalanced against AP (100.00) and `ledger::post` refused it. The remainder now
    // goes to the largest-weight line (ties -> first), so the shares are 33.34 / 33.33 / 33.33.
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

    let po = result.expect("a 100/3 landed-cost spread must post balanced (ACC-0004)");
    assert_eq!(po.status, PurchaseStatus::Received);
    let shares: Vec<Option<Decimal>> = [p1, p2, p3].iter().map(|pid| po.lines.iter().find(|l| l.product_id == *pid).and_then(|l| l.landed_cost_share)).collect();
    assert_eq!(shares, vec![Some(dec!(33.34)), Some(dec!(33.33)), Some(dec!(33.33))], "remainder on the first of the tied largest lines");

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let mut total = Decimal::ZERO;
            for (pid, expected) in [(p1, dec!(43.34)), (p2, dec!(43.33)), (p3, dec!(43.33))] {
                let product = accounting_app_lib::entities::catalog::products::Entity::find_by_id(pid).one(tx).await.unwrap().unwrap();
                assert_eq!(product.stock_value, expected);
                total += product.stock_value;
            }
            // Stock value == order value (30) + the whole landed cost (100).
            assert_eq!(total, dec!(130));
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    assert_invariants_green(&test_db).await;
    test_db.finish().await;
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
    test_db.finish().await;
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
    test_db.finish().await;
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
    test_db.finish().await;
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
    test_db.finish().await;
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
    // GL/subledger relationship is deliberately out of sync at this point in the test. Note:
    // `finish()` still runs `shared::invariants::run_all` — the Rust port's only inventory-related
    // check (`inventory-gl`, §4.4) compares GL inventory balance against Σ product.stock_value, which
    // this test never touches (only stock_qty was hand-edited), so it is expected to stay green.
    test_db.finish().await;
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
    // 2 x 10 = 20 net + 15% input VAT at the PO's rate = 23 (the mock's
    // `computePurchaseTotals(lines, undefined, po.taxRate)` — purchase returns carry VAT).
    assert_eq!(ret_credit.cash_back, dec!(0));
    assert_eq!(ret_credit.settled_to_payable, dec!(23));

    // cash: cash_back paid out via cash (outstanding is fully paid down already by the credit
    // return's settled_to_payable, so a further return once the PO owes nothing produces cash_back
    // == full amount).
    with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                // Fully settle the remaining payable with a real supplier payment allocated to the
                // PO (not a hand-set `paid_amount`, which would leave GL(payable) and the PO's
                // outstanding disagreeing — the supplier-allocation invariant), so the next
                // return's outstanding is 0 and cash_back == its grand total.
                use accounting_app_lib::domains::payments::dto::{
                    AllocationInputTargetKind, PartyKind as PaymentPartyKind, PaymentAllocationInput, PaymentInput, PaymentTenderKind, PaymentTypeDto,
                };
                let po = purchase_orders::Entity::find_by_id(po_id).one(tx).await.unwrap().unwrap();
                let outstanding = po.grand_total - po.returned_amount - po.paid_amount;
                let input = PaymentInput {
                    date: "2026-09-01".to_string(),
                    r#type: PaymentTypeDto::Paid,
                    target_type: PaymentPartyKind::Supplier,
                    target_id: po.supplier_id,
                    amount: outstanding,
                    method: PaymentTenderKind::Cash,
                    note: None,
                    allocations: Some(vec![PaymentAllocationInput { target_kind: AllocationInputTargetKind::PurchaseOrder, target_id: po_id, amount: outstanding }]),
                    branch_id: None,
                    currency: None,
                    amount_fc: None,
                    rate: None,
                };
                accounting_app_lib::domains::payments::service::create::create_payment(tx, cx, &undo, input).await?;
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
    assert_eq!(ret_cash.cash_back, dec!(23));

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
    test_db.finish().await;
}

#[tokio::test]
async fn return_variance_guard_when_stock_value_is_below_what_return_would_remove() {
    // `cost_out_at_price`'s variance guard: returning at a cost_price higher than the current
    // weighted-average means the "value out" at that price exceeds what's actually on the books,
    // producing a positive variance — credited to inventoryVariance (ACC-0011: the supplier owes
    // back more than the stock is carried at) — rather than an inventory value going negative.
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

                // Write the stock down by 40 (simulating a weighted-average drift from other activity
                // outside this PO) so the return-time value-out at the PO's cost_price exceeds what's
                // on the books, forcing a variance line. The write-down posts its own GL entry so
                // GL(inventory) still equals Σ stock_value before the return.
                let mut am: accounting_app_lib::entities::catalog::products::ActiveModel =
                    accounting_app_lib::entities::catalog::products::Entity::find_by_id(product.id).one(tx).await.unwrap().unwrap().into();
                am.stock_value = Set(dec!(10)); // was 50 (5 * 10); now understated.
                am.update(tx).await.unwrap();
                post_stand_in_entry(tx, cx, "2026-09-10", SystemRole::InventoryVariance, dec!(40)).await?;

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
    // 5 × 10 net + the fixture's default 15% INPUT tax (lines carry no tax id, so the PO's own rate
    // applies to the return too).
    assert_eq!(ret.sub_total, dec!(50));
    assert_eq!(ret.grand_total, dec!(57.5));

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let product = accounting_app_lib::entities::catalog::products::Entity::find_by_id(product_id).one(tx).await.unwrap().unwrap();
            // Stock is fully drawn down (qty and value can't go negative given the guard clamps the
            // value-out to what's on hand) — value ends at 0, not negative.
            assert_eq!(product.stock_value, Decimal::ZERO);
            // Dr payable 50 / Cr inventory 10 / Cr inventoryVariance 40 (the write-down above
            // debited inventoryVariance 40 — nothing else touches it).
            assert_eq!(role_totals(tx, "inventoryVariance").await, (dec!(40), dec!(40)));
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    // Invariants stay green: the variance line balances the posting, and the write-down's own GL
    // entry kept GL(inventory) == Σ stock_value throughout.
    assert_invariants_green(&test_db).await;
    test_db.finish().await;
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
            // (3 + 2) * 10 = 50 net + 15% input VAT (the PO's tax rate) = 57.50 — the mock adds the
            // return's grandTotal (`po.returnedAmount += totals.grandTotal`), VAT included.
            assert_eq!(po.returned_amount, dec!(57.50));
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    assert_invariants_green(&test_db).await;
    test_db.finish().await;
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
                supplier_id: Set(fixture_supplier_id(tx).await),
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
                supplier_id: Set(fixture_supplier_id(tx).await),
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
                supplier_id: Set(fixture_supplier_id(tx).await),
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
                supplier_id: Set(fixture_supplier_id(tx).await),
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

    // The synthetic batches inserted above (`fake_batch_id` etc.) never touched `products.stock_value`
    // or posted a GL entry, and `inventory-gl` (the Rust port's only inventory-related invariant)
    // compares GL(inventory) against Σ products.stock_value, not `product_batches` rows — so the
    // books are still expected to be clean here.
    test_db.finish().await;
}

// === ACC-0005: non-stock item receipt is expensed, not debited to inventory ==========================

#[tokio::test]
async fn non_stock_product_receipt_debits_purchase_account_not_inventory() {
    // ACC-0005: a `type: 'product'` item with `stockMode: 'none'` used to debit `inventory` on
    // receipt while `apply_change` ignored it, leaving GL(inventory) != Sum(stockValue) by the
    // line's value. It now takes the service-line path: its purchase account (product -> category
    // -> settings default -> `freightIn`; this fixture has none of the first three, so `freightIn`).
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
    let _ = po_id;

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            use accounting_app_lib::entities::journal::journal_lines;
            use sea_orm::{ColumnTrait, QueryFilter};
            let product = accounting_app_lib::entities::catalog::products::Entity::find_by_id(product_id).one(tx).await.unwrap().unwrap();
            assert_eq!(product.stock_value, Decimal::ZERO, "a non-stock item carries no stock value");
            let debit_on = |role: &'static str| async move {
                let account = accounts::Entity::find().filter(accounts::Column::SystemRole.eq(role)).one(tx).await.unwrap().unwrap();
                journal_lines::Entity::find().filter(journal_lines::Column::AccountId.eq(account.id)).all(tx).await.unwrap().iter().fold(Decimal::ZERO, |a, l| a + l.debit)
            };
            assert_eq!(debit_on("inventory").await, Decimal::ZERO, "a non-stock receipt must not debit inventory");
            assert_eq!(debit_on("freightIn").await, dec!(30), "the non-stock line (3 * 10) is expensed to its purchase account");
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    assert_invariants_green(&test_db).await;
    test_db.finish().await;
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
    Arc::try_unwrap(test_db).unwrap_or_else(|_| panic!("test_db Arc must be uniquely owned at the end of the test")).finish().await;
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
    Arc::try_unwrap(test_db).unwrap_or_else(|_| panic!("test_db Arc must be uniquely owned at the end of the test")).finish().await;
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

    // Every spawned clone of the `Arc<TestDb>` has been joined (and dropped) by the loop above, so
    // `test_db` is uniquely owned again here.
    assert_invariants_green(&test_db).await;
    Arc::try_unwrap(test_db).unwrap_or_else(|_| panic!("test_db Arc must be uniquely owned at the end of the test")).finish().await;
}

// === ACC-0011 / ACC-0012 / ACC-0013 ================================================================

#[tokio::test]
async fn acc_0011_return_variance_is_credited_when_the_supplier_owes_more_than_the_stock_is_carried_at() {
    // Parity case purchases/p-p7-return-variance-guard: buy 2 @50, one unit leaves at its carrying
    // value (50), buy 1 @10 — on hand 2 units carried at 60. Returning both 50-units gives the
    // supplier back 100 net (+ VAT) but only 60 leaves inventory, so the entry is Dr payable / Cr
    // inventory 60 / Cr VAT / Cr inventoryVariance 40. The old rule posted Dr inventoryVariance 40,
    // and the entry was refused as unbalanced.
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_a, product_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "حزام قماش", "SKU-ACC11", dec!(50), ProductType::Product, false).await;
                let po_a = orders::save_purchase_order(tx, cx, &undo, po_input(fixture.supplier_id, "2026-09-01", vec![po_line(product.id, dec!(2), dec!(50))], true), None).await?;
                // One unit leaves at its carrying value (a sale's COGS).
                let mut locked = lock_product(tx, product.id).await?;
                apply_change(tx, cx, &mut locked, dec!(-1), dec!(-50), "sale", StockRef { id: Id::new(), number: "INV-ACC11".to_string() }, &day("2026-09-01"), Some(fixture.branch_id))
                    .await?;
                post_stand_in_entry(tx, cx, "2026-09-01", SystemRole::FreightIn, dec!(50)).await?;
                orders::save_purchase_order(tx, cx, &undo, po_input(fixture.supplier_id, "2026-09-01", vec![po_line(product.id, dec!(1), dec!(10))], true), None).await?;
                Ok((po_a.id, product.id))
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
                    purchase_order_id: po_a,
                    reason: Some("إرجاع كامل".to_string()),
                    refund_method: Some(RefundMethod::Credit),
                    lines: vec![PurchaseReturnInputLine { product_id, qty: dec!(2), batch_id: None }],
                    from_draft_id: None,
                };
                returns::create_purchase_return(tx, cx, &undo, input).await
            }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await
    .expect("a return with a positive variance must post (balanced)");
    assert_eq!(ret.sub_total, dec!(100));

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let product = accounting_app_lib::entities::catalog::products::Entity::find_by_id(product_id).one(tx).await.unwrap().unwrap();
            assert_eq!(product.stock_qty, Decimal::ZERO);
            assert_eq!(product.stock_value, Decimal::ZERO);
            assert_eq!(role_totals(tx, "inventoryVariance").await, (Decimal::ZERO, dec!(40)), "positive variance is a CREDIT to inventoryVariance");
            // Inventory: +100 (PO A) − 50 (the unit that left) + 10 (PO B) − 60 (the return).
            assert_eq!(role_totals(tx, "inventory").await, (dec!(110), dec!(110)));
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    assert_invariants_green(&test_db).await;
    test_db.finish().await;
}

#[tokio::test]
async fn acc_0012_other_supplier_landed_cost_becomes_an_open_payable_document() {
    // Parity case purchases/p-p4-other-supplier-landed: a landed cost billed by a shipper posts a
    // `Cr payable` tagged with the shipper. That liability is now a RECEIVED purchase order of the
    // shipper's own (no stock lines, total = the landed amount, numbered before the journal), so it
    // is an open document a payment can be allocated to, and `supplier-allocation` holds.
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, product_id, supplier_id, shipper_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-ACC12", dec!(100), ProductType::Product, false).await;
                let po = orders::save_purchase_order(tx, cx, &undo, po_input(fixture.supplier_id, "2026-09-01", vec![po_line(product.id, dec!(2), dec!(100))], false), None).await?;
                Ok((po.id, product.id, fixture.supplier_id, fixture.shipping_supplier_id))
            }) as BoxFuture<'_, TxResult<(Id, Id, Id, Id)>>
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
                    date: "2026-09-02".to_string(),
                    lines: vec![ReceiveLineInput { product_id, received_qty: dec!(2), batches: None }],
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    vat_not_recoverable: None,
                    landed_costs: Some(vec![
                        LandedCostLineInput { label: "شحن المورد".to_string(), amount: dec!(10), supplier_id: None, spread_by: LandedCostSpread::Value },
                        LandedCostLineInput { label: "تخليص جمركي".to_string(), amount: dec!(25), supplier_id: Some(shipper_id), spread_by: LandedCostSpread::Qty },
                    ]),
                    create_backorder: None,
                };
                receive::receive_purchase(tx, cx, &undo, po_id, input).await
            }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await
    .unwrap();
    // The PO's own AP excludes the shipper's 25: 200 net + 30 VAT + 10 own landed cost.
    assert_eq!(po.grand_total, dec!(240));
    let po_number = po.number.clone();

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        let po_number = po_number.clone();
        Box::pin(async move {
            use accounting_app_lib::entities::journal::journal_lines;
            use accounting_app_lib::entities::purchases::purchase_order_lines;
            use sea_orm::{ColumnTrait, QueryFilter};

            let bills = purchase_orders::Entity::find().filter(purchase_orders::Column::SupplierId.eq(shipper_id)).all(tx).await.unwrap();
            assert_eq!(bills.len(), 1, "one payable document for the shipper");
            let bill = &bills[0];
            assert_eq!(bill.status, purchase_orders::PurchaseStatus::Received);
            assert_eq!(bill.grand_total, dec!(25));
            assert_eq!(bill.sub_total, dec!(25));
            assert_eq!(bill.tax_amount, Decimal::ZERO);
            assert_eq!(bill.paid_amount, Decimal::ZERO);
            assert_eq!(bill.returned_amount, Decimal::ZERO);
            assert!(bill.received_date_day.is_some());
            assert_eq!(bill.note.as_deref(), Some(format!("تكلفة إضافية \"تخليص جمركي\" على أمر الشراء {}", po_number).as_str()));
            let bill_lines = purchase_order_lines::Entity::find().filter(purchase_order_lines::Column::PurchaseOrderId.eq(bill.id)).all(tx).await.unwrap();
            assert!(bill_lines.is_empty(), "the shipper's document carries only the liability — no stock lines");

            // Its `Cr payable` line (in the receipt's own entry) names the document.
            let shipper_lines = journal_lines::Entity::find().filter(journal_lines::Column::PartyId.eq(shipper_id)).all(tx).await.unwrap();
            assert_eq!(shipper_lines.len(), 1);
            assert_eq!(shipper_lines[0].credit, dec!(25));
            assert_eq!(shipper_lines[0].description.as_deref(), Some(format!("تكلفة إضافية \"تخليص جمركي\" — {}", bill.number).as_str()));

            // The PO's own supplier still has exactly one document.
            let own = purchase_orders::Entity::find().filter(purchase_orders::Column::SupplierId.eq(supplier_id)).all(tx).await.unwrap();
            assert_eq!(own.len(), 1);

            // The whole landed cost (10 + 25) is in the stock value: 200 + 35.
            let product = accounting_app_lib::entities::catalog::products::Entity::find_by_id(product_id).one(tx).await.unwrap().unwrap();
            assert_eq!(product.stock_value, dec!(235));
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    // `supplier-allocation`: the shipper's balance (25) = Σ its open documents (25).
    assert_invariants_green(&test_db).await;
    test_db.finish().await;
}

#[tokio::test]
async fn acc_0012_other_supplier_landed_cost_refused_when_that_supplier_does_not_exist() {
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_id, product_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-ACC12B", dec!(100), ProductType::Product, false).await;
                let po = orders::save_purchase_order(tx, cx, &undo, po_input(fixture.supplier_id, "2026-09-01", vec![po_line(product.id, dec!(1), dec!(100))], false), None).await?;
                Ok((po.id, product.id))
            }) as BoxFuture<'_, TxResult<(Id, Id)>>
        }
    })
    .await
    .unwrap();

    let err = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = ReceivePurchaseInput {
                    date: "2026-09-02".to_string(),
                    lines: vec![ReceiveLineInput { product_id, received_qty: dec!(1), batches: None }],
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    vat_not_recoverable: None,
                    landed_costs: Some(vec![LandedCostLineInput { label: "شحن".to_string(), amount: dec!(5), supplier_id: Some(Id::new()), spread_by: LandedCostSpread::Value }]),
                    create_backorder: None,
                };
                receive::receive_purchase(tx, cx, &undo, po_id, input).await
            }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await
    .unwrap_err();
    match err {
        AppError::Validation { message } => assert_eq!(message, "مورد التكلفة الإضافية غير موجود"),
        other => panic!("expected VALIDATION, got {other:?}"),
    }

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let po = purchase_orders::Entity::find_by_id(po_id).one(tx).await.unwrap().unwrap();
            assert_eq!(po.status, purchase_orders::PurchaseStatus::Draft, "a refused receipt leaves the order as it was");
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();
    assert_invariants_green(&test_db).await;
    test_db.finish().await;
}

#[tokio::test]
async fn acc_0013_purchase_writes_refused_by_a_closed_period_leave_no_trace() {
    // The mock used to move stock / write rows before a refused posting (closed period), leaving
    // partial state; the Rust commands must (and do — one transaction) leave nothing behind.
    let test_db = TestDb::fresh().await;
    log_in(&test_db);
    let undo = Arc::new(UndoRegistry::new());

    let (po_a, draft_id, product_id, supplier_id) = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let fixture = seed_fixture(tx).await;
                let product = seed_product(tx, cx, &undo, "منتج", "SKU-ACC13", dec!(25), ProductType::Product, false).await;
                let po_a = orders::save_purchase_order(tx, cx, &undo, po_input(fixture.supplier_id, "2026-09-01", vec![po_line(product.id, dec!(4), dec!(25))], true), None).await?;
                let draft = orders::save_purchase_order(tx, cx, &undo, po_input(fixture.supplier_id, "2026-09-01", vec![po_line(product.id, dec!(3), dec!(30))], false), None).await?;
                // Close every fiscal year: from here on nothing may post.
                for fy in fiscal_years::Entity::find().all(tx).await.unwrap() {
                    let mut am: fiscal_years::ActiveModel = fy.into();
                    am.is_closed = Set(true);
                    am.update(tx).await.unwrap();
                }
                Ok((po_a.id, draft.id, product.id, fixture.supplier_id))
            }) as BoxFuture<'_, TxResult<(Id, Id, Id, Id)>>
        }
    })
    .await
    .unwrap();

    // 1. Receiving the draft is refused.
    let receive_result = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = ReceivePurchaseInput {
                    date: "2026-09-20".to_string(),
                    lines: vec![ReceiveLineInput { product_id, received_qty: dec!(3), batches: None }],
                    supplier_invoice_no: None,
                    supplier_invoice_date: None,
                    vat_not_recoverable: None,
                    landed_costs: None,
                    create_backorder: None,
                };
                receive::receive_purchase(tx, cx, &undo, draft_id, input).await
            }) as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await;
    assert!(receive_result.is_err(), "receiving into a closed period must be refused");

    // 2. Saving a new order with confirm is refused — and the order itself is not saved.
    let save_result = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { orders::save_purchase_order(tx, cx, &undo, po_input(supplier_id, "2026-09-20", vec![po_line(product_id, dec!(2), dec!(40))], true), None).await })
                as BoxFuture<'_, TxResult<PurchaseOrder>>
        }
    })
    .await;
    assert!(save_result.is_err(), "save-and-confirm into a closed period must be refused");

    // 3. A return (dated today) is refused.
    let return_result = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move {
                let input = PurchaseReturnInput {
                    purchase_order_id: po_a,
                    reason: Some("تالف".to_string()),
                    refund_method: None,
                    lines: vec![PurchaseReturnInputLine { product_id, qty: dec!(1), batch_id: None }],
                    from_draft_id: None,
                };
                returns::create_purchase_return(tx, cx, &undo, input).await
            }) as BoxFuture<'_, TxResult<PurchaseReturn>>
        }
    })
    .await;
    assert!(return_result.is_err(), "a return into a closed period must be refused");

    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let product = accounting_app_lib::entities::catalog::products::Entity::find_by_id(product_id).one(tx).await.unwrap().unwrap();
            assert_eq!(product.stock_qty, dec!(4), "stock untouched by the refused writes");
            assert_eq!(product.stock_value, dec!(100));
            let draft = purchase_orders::Entity::find_by_id(draft_id).one(tx).await.unwrap().unwrap();
            assert_eq!(draft.status, purchase_orders::PurchaseStatus::Draft);
            assert_eq!(purchase_orders::Entity::find().all(tx).await.unwrap().len(), 2, "the refused save-and-confirm left no order behind");
            let a = purchase_orders::Entity::find_by_id(po_a).one(tx).await.unwrap().unwrap();
            assert_eq!(a.returned_amount, Decimal::ZERO);
            assert!(accounting_app_lib::entities::purchases::purchase_returns::Entity::find().all(tx).await.unwrap().is_empty(), "no return row");
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    assert_invariants_green(&test_db).await;
    test_db.finish().await;
}
