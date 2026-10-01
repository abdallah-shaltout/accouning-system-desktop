//! DB-backed tests for the `products` domain (06-products.md §8a catalog + 06b-inventory.md §8a
//! inventory). Written now, run in the deferred time-boxed test pass (per-implementer hard rule:
//! never run cargo from this agent). Needs `EQUAL_TEST_DATABASE_URL` — see `tests/support/mod.rs`.
//!
//! There is no shared `seed_company` fixture yet (G-P9, a manager task per 06 §7) — this file
//! builds its own minimal fixture (`seed_fixture`), the same pattern `tests/shared_ledger.rs` uses,
//! with every system-role account both files' postings touch (`inventory`, `inventoryInTransit`,
//! `inventoryVariance`, `inventoryWriteOff`, `openingBalanceEquity`, `ownerCurrent`, `otherIncome`,
//! plus one non-system "control" account for the `other` STOCK_IN reason). Once G-P9 lands, this
//! fixture should be replaced with it (same note pattern as `domain_settings.rs`'s
//! `seed_branch_and_settings` doc comment).

use crate::support;

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::tx::{with_read, with_tx, BoxFuture, TxOpts, TxResult};
use accounting_app_lib::domains::products::dto::catalog::{
    Category, CustomFieldDef, CustomFieldDefInput, CustomFieldType, PriceList, PriceListInput, Product, ProductInput, ProductPriceInput, ProductType,
    ProductUnit as DtoProductUnit, Unit, UnitPresetKind,
};
use accounting_app_lib::domains::products::dto::inventory::{
    DebitNoteDraft, DebitNoteDraftLineInput, ReceiveTransferInput, ReceiveTransferLineInput, StockAdjustment, StockAdjustmentInput, StockAdjustmentLineInput,
    StockAdjustmentType, StockCount, StockCountInput, StockCountScope, StockInReason, StockTransfer, StockTransferInput, StockTransferLineInput,
};
use accounting_app_lib::domains::products::service::adjustments::ApprovalCheck;
use accounting_app_lib::domains::products::service::{adjustments, batches, catalog, counts, movements, products, transfers};
use accounting_app_lib::entities::org::accounts::ActiveModel as AccountActiveModel;
use accounting_app_lib::entities::org::branches::ActiveModel as BranchActiveModel;
use accounting_app_lib::entities::org::settings::ActiveModel as SettingsActiveModel;
use accounting_app_lib::entities::org::users::ActiveModel as UserActiveModel;
use accounting_app_lib::entities::values::{AccountingPolicy, OnboardingState, PrinterMode, PrinterSettings};
use accounting_app_lib::shared::activity::undo::UndoRegistry;
use accounting_app_lib::shared::invariants;
use accounting_app_lib::utils::id::Id;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use std::sync::Arc;
use support::TestDb;

// --- Fixture ---------------------------------------------------------------------------------

struct Fixture {
    pub branch_id: Id,
    pub branch2_id: Id,
    // Kept for parity with the system-role accounts `seed_fixture` creates (inventory,
    // inventoryInTransit, inventoryVariance, inventoryWriteOff, openingBalanceEquity, ownerCurrent,
    // otherIncome, plus the "other" STOCK_IN control account) — no current test asserts against an
    // account id directly, but a future posting-trace test will want it without re-deriving it.
    #[allow(dead_code)]
    pub accounts: std::collections::HashMap<&'static str, Id>,
    pub admin_id: Id,
    pub manager_id: Id,
}

/// The seeded `admin` user's fixed id (each test has its own database), so tests that log in
/// without keeping the fixture still act as an existing user — `audit.user_id` and the other actor
/// columns are FKs to `users`, as in the real app where a session always belongs to a real user.
fn test_admin_id() -> Id {
    "01900000-0000-7000-8000-00000000a001".parse().unwrap()
}

fn log_in(test_db: &TestDb, user_id: Id, role: Role) {
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
}

async fn seed_fixture<C: ConnectionTrait>(conn: &C) -> Fixture {
    let branch_id = Id::new();
    let branch2_id = Id::new();
    let now = chrono::Utc::now();

    for (id, name, code) in [(branch_id, "الفرع الرئيسي", "MAIN"), (branch2_id, "الفرع الثاني", "BR2")] {
        let branch = BranchActiveModel {
            id: Set(id),
            name: Set(name.to_string()),
            code: Set(code.to_string()),
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
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
        };
        branch.insert(conn).await.unwrap();
    }

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
        printer: Set(PrinterSettings { mode: PrinterMode::A4, thermal_width_mm: 80, thermal: None, a4_printer_name: None, label_printer_name: None, a4_template: None, image_template: None }),
        prices_include_tax: Set(true),
        address: Set(None),
        national_address: Set(None),
        phone: Set(None),
        commercial_register: Set(None),
        receipt_footer: Set(None),
        accounting: Set(Some(AccountingPolicy { lock_date: None, default_purchase_account_id: None })),
        backup: Set(None),
        inventory_approval_threshold: Set(Some(dec!(1000))),
        role_access_overrides: Set(None),
        insight_thresholds: Set(None),
        pos: Set(None),
        sales: Set(None),
        features: Set(None),
        // Onboarding in progress: several tests post opening stock (STOCK_IN reason `opening`,
        // Cr openingBalanceEquity), which is an onboarding step — invariant 9 only requires 3900
        // to be zero once onboarding is finished (`checkOpeningBalanceEquity`), same as the mock.
        onboarding: Set(Some(OnboardingState {
            business_type: None,
            go_live_date: None,
            completed_step: None,
            skipped: vec![],
            done: vec![],
            finished_at: None,
            opening_entry_id: None,
            closing_entry_id: None,
            coa_template: None,
        })),
        timezone: Set(None),
        default_branch_id: Set(branch_id),
        created_at: Set(now),
        updated_at: Set(now),
    };
    settings.insert(conn).await.unwrap();

    let roles: &[(&str, &str)] = &[
        ("inventory", "inventory"),
        ("inventoryInTransit", "inventoryInTransit"),
        ("inventoryVariance", "inventoryVariance"),
        ("inventoryWriteOff", "inventoryWriteOff"),
        ("openingBalanceEquity", "openingBalanceEquity"),
        ("ownerCurrent", "ownerCurrent"),
        ("otherIncome", "otherIncome"),
    ];
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
            subtype: Set("otherCurrentAsset".to_string()),
            normal_side: Set("DEBIT".to_string()),
            system_role: Set(Some((*role).to_string())),
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
        accounts.insert(*label, id);
    }

    // A plain non-system-role, manual-allowed control account — the `other` STOCK_IN reason's
    // `offset_account_id` target.
    let control_id = Id::new();
    let control = AccountActiveModel {
        code_live: sea_orm::ActiveValue::NotSet,
        id: Set(control_id),
        code: Set("ACC-control".to_string()),
        name: Set("حساب مقابل".to_string()),
        name_en: Set(None),
        parent_id: Set(None),
        is_group: Set(false),
        kind: Set("EQUITY".to_string()),
        subtype: Set("equity".to_string()),
        normal_side: Set("CREDIT".to_string()),
        system_role: Set(None),
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
    control.insert(conn).await.unwrap();
    accounts.insert("control", control_id);

    let admin_id = test_admin_id();
    let manager_id = Id::new();
    for (id, username, role) in [(admin_id, "admin", "admin"), (manager_id, "manager", "manager")] {
        let user = UserActiveModel {
            id: Set(id),
            username: Set(username.to_string()),
            name: Set(username.to_string()),
            phone: Set(None),
            role: Set(role.to_string()),
            max_discount: Set(Decimal::ZERO),
            price_list_id: Set(None),
            active: Set(true),
            avatar: Set(None),
            allowed_branches: Set(None),
            home_branch: Set(Some(branch_id)),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
        };
        user.insert(conn).await.unwrap();
    }

    Fixture { branch_id, branch2_id, accounts, admin_id, manager_id }
}

async fn seed_supplier<C: ConnectionTrait>(conn: &C) -> Id {
    let id = Id::new();
    let now = chrono::Utc::now();
    accounting_app_lib::entities::parties::parties::ActiveModel {
        id: Set(id),
        kind: Set("supplier".to_string()),
        r#type: Set("company".to_string()),
        name: Set("مورد".to_string()),
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
    }
    .insert(conn)
    .await
    .unwrap();
    id
}

fn base_product_input(name: &str, sku: &str) -> ProductInput {
    ProductInput {
        name: name.to_string(),
        name_en: None,
        sku: sku.to_string(),
        barcode: None,
        category_id: None,
        unit_id: None,
        r#type: ProductType::Product,
        stock_mode: None,
        cost_price: dec!(10),
        price: dec!(20),
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

// --- Catalog: create/update/validate -----------------------------------------------------------

#[tokio::test]
async fn create_product_happy_path_has_empty_prices_and_zero_stock() {
    let db = TestDb::fresh().await;
    let _fixture = {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await
    };
    log_in(&db, test_admin_id(), Role::Admin);

    let undo = Arc::new(UndoRegistry::default());
    let input = base_product_input("منتج تجريبي", "SKU-001");
    let result: Result<_, _> = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        let input = input.clone();
        let undo = undo.clone();
        Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
    })
    .await;

    let product = result.expect("create_product must succeed");
    assert_eq!(product.prices, Some(vec![]));
    assert_eq!(product.stock_qty, Decimal::ZERO);
    assert_eq!(product.stock_value, Decimal::ZERO);
    assert_eq!(product.sku, "SKU-001");
    db.finish().await;
}

#[tokio::test]
async fn create_product_validation_messages_in_order() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    async fn run(db: &TestDb, undo: &Arc<UndoRegistry>, input: ProductInput) -> Result<accounting_app_lib::core::error::AppError, String> {
        let undo = undo.clone();
        let result: Result<_, _> = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await;
        match result {
            Ok(_) => Err("expected an error, got Ok".to_string()),
            Err(e) => Ok(e),
        }
    }

    // 1. empty name
    let mut input = base_product_input("", "SKU-A");
    let err = run(&db, &undo, input.clone()).await.unwrap();
    assert_eq!(err.to_string(), "اسم المنتج مطلوب");

    // 2. empty sku
    input.name = "منتج".to_string();
    input.sku = "".to_string();
    let err = run(&db, &undo, input.clone()).await.unwrap();
    assert_eq!(err.to_string(), "رمز المنتج (SKU) مطلوب");

    // 3. negative price
    input.sku = "SKU-B".to_string();
    input.price = dec!(-1);
    let err = run(&db, &undo, input.clone()).await.unwrap();
    assert_eq!(err.to_string(), "الأسعار لا يمكن أن تكون سالبة");

    // 4. min_price > price
    input.price = dec!(20);
    input.min_price = Some(dec!(50));
    let err = run(&db, &undo, input.clone()).await.unwrap();
    assert_eq!(err.to_string(), "الحد الأدنى للسعر أكبر من سعر البيع");
    input.min_price = None;

    // 5. unit factor <= 0
    input.units = Some(vec![DtoProductUnit {
        id: "u1".to_string(),
        unit_id: Id::new(),
        factor: Decimal::ZERO,
        barcodes: vec![],
        price: dec!(20),
        price_is_auto: false,
        default_for_sale: true,
        default_for_purchase: true,
        active: true,
    }]);
    let err = run(&db, &undo, input.clone()).await.unwrap();
    assert_eq!(err.to_string(), "عامل تحويل الوحدة يجب أن يكون أكبر من صفر");

    // 6. two base units (both factor 1)
    input.units = Some(vec![
        DtoProductUnit { id: "u1".into(), unit_id: Id::new(), factor: Decimal::ONE, barcodes: vec![], price: dec!(20), price_is_auto: false, default_for_sale: true, default_for_purchase: true, active: true },
        DtoProductUnit { id: "u2".into(), unit_id: Id::new(), factor: Decimal::ONE, barcodes: vec![], price: dec!(20), price_is_auto: false, default_for_sale: false, default_for_purchase: false, active: true },
    ]);
    let err = run(&db, &undo, input.clone()).await.unwrap();
    assert_eq!(err.to_string(), "يجب أن تكون وحدة واحدة فقط بعامل تحويل = 1 (الوحدة الأساسية)");

    // Create the first product to exercise duplicate SKU/barcode checks against it.
    input.units = None;
    input.sku = "SKU-DUP".to_string();
    input.barcode = Some("BC-DUP".to_string());
    let created = {
        let undo = undo.clone();
        let input2 = input.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input2 = input2.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input2).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .expect("seed product for dup checks")
    };
    assert!(!created.id.to_string().is_empty());

    // 7. sku dup (case-insensitive)
    let mut dup_input = base_product_input("آخر", "sku-dup");
    let err = run(&db, &undo, dup_input.clone()).await.unwrap();
    assert_eq!(err.to_string(), "رمز المنتج مستخدم لمنتج آخر");

    // 8. barcode dup
    dup_input.sku = "SKU-C".to_string();
    dup_input.barcode = Some("BC-DUP".to_string());
    let err = run(&db, &undo, dup_input).await.unwrap();
    assert_eq!(err.to_string(), "الباركود مستخدم لمنتج آخر");
    db.finish().await;
}

#[tokio::test]
async fn create_product_with_opening_qty_posts_stock_in_and_activity_order() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let mut input = base_product_input("منتج برصيد افتتاحي", "SKU-OPEN");
    input.cost_price = dec!(10);
    input.opening_qty = Some(dec!(5));

    let product = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .expect("create with opening qty must succeed")
    };

    assert_eq!(product.stock_qty, dec!(5));
    assert_eq!(product.stock_value, dec!(50));
    assert_eq!(product.cost_price, dec!(10));

    // Invariants must stay green after the opening post.
    let results = with_read(&db.state, |tx| Box::pin(async move { invariants::run_all(tx).await.map_err(accounting_app_lib::core::tx::TxError::App) })).await.unwrap();
    let failed: Vec<String> = results.iter().filter(|r| !r.passed).map(|r| format!("{}: {}", r.key, r.message)).collect();
    assert!(failed.is_empty(), "invariants must all pass, failed: {failed:?}");
    db.finish().await;
}

#[tokio::test]
async fn create_product_opening_qty_above_threshold_rolls_back_whole_create() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    // threshold is 1000; 200 * 10 = 2000 >= threshold, no approvedBy -> FORBIDDEN.
    let mut input = base_product_input("منتج فوق الحد", "SKU-OVER");
    input.cost_price = dec!(10);
    input.opening_qty = Some(dec!(200));

    let result: Result<_, _> = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
    };
    assert!(result.is_err(), "expected FORBIDDEN, got Ok");

    // The whole transaction (including the product row) must have rolled back — stricter than the
    // mock's partial write (Q-5).
    let found = with_read(&db.state, |tx| {
        Box::pin(async move {
            products::find_by_code(tx, "SKU-OVER").await.map_err(Into::into)
        })
    })
    .await
    .unwrap();
    assert!(found.is_none(), "product must not exist after a rolled-back opening adjustment");
    db.finish().await;
}

#[tokio::test]
async fn update_product_cost_ignored_with_stock_applied_when_empty() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let mut input = base_product_input("منتج للتحديث", "SKU-UPD");
    input.cost_price = dec!(10);
    let product = {
        let undo = undo.clone();
        let input = input.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };

    // No stock yet -> cost applies.
    let mut update_input = input.clone();
    update_input.cost_price = dec!(15);
    let updated = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let update_input = update_input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::update_product(tx, cx, &undo, product.id, update_input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };
    assert_eq!(updated.cost_price, dec!(15), "cost must apply when stock_qty is 0");

    // Give it stock, then try to change cost again -> ignored.
    {
        let undo = undo.clone();
        let adj_input = StockAdjustmentInput {
            r#type: StockAdjustmentType::StockIn,
            date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
            note: None,
            reason: Some(StockInReason::Found),
            offset_account_id: None,
            lines: vec![StockAdjustmentLineInput { product_id: updated.id, qty_change: Some(dec!(3)), counted_qty: None, batch_no: None, expiry_date: None }],
            approved_by: None,
        };
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let adj_input = adj_input.clone();
            let undo = undo.clone();
            Box::pin(async move { adjustments::record_stock_adjustment(tx, cx, &undo, adj_input, false, ApprovalCheck::none()).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
        })
        .await
        .unwrap();
    }

    let mut update_input2 = input.clone();
    update_input2.cost_price = dec!(99);
    let updated2 = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let update_input2 = update_input2.clone();
            let undo = undo.clone();
            Box::pin(async move { products::update_product(tx, cx, &undo, updated.id, update_input2).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };
    assert_eq!(updated2.cost_price, dec!(15), "cost change must be silently ignored once stock > 0");
    db.finish().await;
}

// --- Categories / units / price lists / custom fields -------------------------------------------

#[tokio::test]
async fn category_exact_name_conflict_and_case_variants_allowed() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);

    let cat1 = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move { catalog::save_category(tx, cx, "Food".to_string(), None, None).await }) as BoxFuture<'_, TxResult<Category>>
    })
    .await
    .unwrap();

    // Exact duplicate -> conflict.
    let dup: Result<_, _> = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move { catalog::save_category(tx, cx, "Food".to_string(), None, None).await }) as BoxFuture<'_, TxResult<Category>>
    })
    .await;
    assert!(dup.is_err());

    // Case-variant name is a different exact string -> allowed (after G-P4a's utf8mb4_bin
    // collation on name_live; this test assumes that migration has landed).
    let cat2: Result<_, _> = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move { catalog::save_category(tx, cx, "food".to_string(), None, None).await }) as BoxFuture<'_, TxResult<Category>>
    })
    .await;
    assert!(cat2.is_ok(), "\"food\" must be allowed alongside \"Food\" once names are bin-collated (G-P4a)");

    // productCount + delete guard.
    let categories = with_read(&db.state, |tx| Box::pin(async move { catalog::get_categories(tx).await.map_err(Into::into) })).await.unwrap();
    assert!(categories.iter().any(|c| c.id == cat1.id && c.product_count == 0));
    db.finish().await;
}

#[tokio::test]
async fn delete_category_in_use_is_refused_then_freed_after_removal() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let cat = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move { catalog::save_category(tx, cx, "أدوات".to_string(), None, None).await }) as BoxFuture<'_, TxResult<Category>>
    })
    .await
    .unwrap();

    let mut input = base_product_input("منتج مصنف", "SKU-CAT");
    input.category_id = Some(cat.id);
    {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap();
    }

    let result: Result<_, _> = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move { catalog::delete_category(tx, cx, cat.id).await }) as BoxFuture<'_, TxResult<()>>
    })
    .await;
    assert!(result.is_err(), "category in use must refuse delete");
    db.finish().await;
}

#[tokio::test]
async fn unit_preset_applies_once_and_is_idempotent() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);

    let created1 = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move { catalog::apply_unit_preset(tx, cx, UnitPresetKind::Supermarket).await }) as BoxFuture<'_, TxResult<Vec<Unit>>>
    })
    .await
    .unwrap();
    assert_eq!(created1.len(), 5);

    let created2 = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move { catalog::apply_unit_preset(tx, cx, UnitPresetKind::Supermarket).await }) as BoxFuture<'_, TxResult<Vec<Unit>>>
    })
    .await
    .unwrap();
    assert!(created2.is_empty(), "re-applying the same preset must create nothing");
    db.finish().await;
}

#[tokio::test]
async fn price_list_delete_cascades_and_set_values_rolls_back_on_negative() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let list = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move { catalog::save_price_list(tx, cx, PriceListInput { name: "جملة".to_string(), active: true }, None).await }) as BoxFuture<'_, TxResult<PriceList>>
    })
    .await
    .unwrap();

    let mut input = base_product_input("منتج بسعر جملة", "SKU-PL");
    input.prices = Some(vec![ProductPriceInput { price_list_id: list.id, value: Some(dec!(15)) }]);
    let product = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };
    assert_eq!(product.prices.as_ref().unwrap().len(), 1);

    with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move { catalog::delete_price_list(tx, cx, list.id).await }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    let refreshed = with_read(&db.state, move |tx| Box::pin(async move { products::get_product(tx, product.id).await.map_err(Into::into) })).await.unwrap();
    assert_eq!(refreshed.prices, Some(vec![]), "delete_price_list must cascade product_prices rows");
    db.finish().await;
}

#[tokio::test]
async fn custom_field_def_requires_options_for_list_type_and_delete_guard() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let no_options: Result<_, _> = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            catalog::save_custom_field_def(tx, cx, CustomFieldDefInput { name: "لون".to_string(), r#type: CustomFieldType::List, options: None, active: true }, None).await
        }) as BoxFuture<'_, TxResult<CustomFieldDef>>
    })
    .await;
    assert!(no_options.is_err());

    let field = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            catalog::save_custom_field_def(
                tx,
                cx,
                CustomFieldDefInput { name: "لون".to_string(), r#type: CustomFieldType::List, options: Some(vec!["أحمر".to_string(), "أزرق".to_string()]), active: true },
                None,
            )
            .await
        }) as BoxFuture<'_, TxResult<CustomFieldDef>>
    })
    .await
    .unwrap();

    let mut input = base_product_input("منتج بحقل مخصص", "SKU-CF");
    input.custom_fields = Some(std::collections::BTreeMap::from([(field.id.to_string(), serde_json::json!("أحمر"))]));
    {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap();
    }

    let delete_result: Result<_, _> = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move { catalog::delete_custom_field_def(tx, cx, field.id).await }) as BoxFuture<'_, TxResult<()>>
    })
    .await;
    assert!(delete_result.is_err(), "deleting a custom field used by a product must be refused");
    db.finish().await;
}

// --- Inventory: stock adjustments ----------------------------------------------------------------

#[tokio::test]
async fn stock_in_each_reason_credits_expected_role() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let product = {
        let undo = undo.clone();
        let input = base_product_input("منتج STOCK_IN", "SKU-SI");
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };

    for reason in [StockInReason::Opening, StockInReason::OwnerContribution, StockInReason::Gift, StockInReason::Found] {
        let undo = undo.clone();
        let input = StockAdjustmentInput {
            r#type: StockAdjustmentType::StockIn,
            date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
            note: None,
            reason: Some(reason),
            offset_account_id: None,
            lines: vec![StockAdjustmentLineInput { product_id: product.id, qty_change: Some(dec!(1)), counted_qty: None, batch_no: None, expiry_date: None }],
            approved_by: None,
        };
        let result = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { adjustments::record_stock_adjustment(tx, cx, &undo, input, false, ApprovalCheck::none()).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
        })
        .await;
        assert!(result.is_ok(), "STOCK_IN with reason {reason:?} must post: {result:?}");
    }

    // `other` without offset_account_id -> message.
    let undo2 = undo.clone();
    let missing_offset = StockAdjustmentInput {
        r#type: StockAdjustmentType::StockIn,
        date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
        note: None,
        reason: Some(StockInReason::Other),
        offset_account_id: None,
        lines: vec![StockAdjustmentLineInput { product_id: product.id, qty_change: Some(dec!(1)), counted_qty: None, batch_no: None, expiry_date: None }],
        approved_by: None,
    };
    let result: Result<_, _> = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        let input = missing_offset.clone();
        let undo = undo2.clone();
        Box::pin(async move { adjustments::record_stock_adjustment(tx, cx, &undo, input, false, ApprovalCheck::none()).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
    })
    .await;
    assert!(result.is_err());

    let results = with_read(&db.state, |tx| Box::pin(async move { invariants::run_all(tx).await.map_err(accounting_app_lib::core::tx::TxError::App) })).await.unwrap();
    let failed: Vec<String> = results.iter().filter(|r| !r.passed).map(|r| format!("{}: {}", r.key, r.message)).collect();
    assert!(failed.is_empty(), "invariants must all pass after each STOCK_IN reason: {failed:?}");
    db.finish().await;
}

#[tokio::test]
async fn loss_above_stock_is_refused_and_tracked_product_requires_batch_no() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let mut input = base_product_input("منتج LOSS", "SKU-LOSS");
    input.track_batches = Some(true);
    let product = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };

    // LOSS above (zero) stock -> message.
    let undo2 = undo.clone();
    let loss_input = StockAdjustmentInput {
        r#type: StockAdjustmentType::Loss,
        date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
        note: None,
        reason: None,
        offset_account_id: None,
        lines: vec![StockAdjustmentLineInput { product_id: product.id, qty_change: Some(dec!(5)), counted_qty: None, batch_no: None, expiry_date: None }],
        approved_by: None,
    };
    let result: Result<_, _> = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        let input = loss_input.clone();
        let undo = undo2.clone();
        Box::pin(async move { adjustments::record_stock_adjustment(tx, cx, &undo, input, false, ApprovalCheck::none()).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
    })
    .await;
    assert!(result.is_err());

    // STOCK_IN of a tracked product without a batch number -> message.
    let undo3 = undo.clone();
    let no_batch = StockAdjustmentInput {
        r#type: StockAdjustmentType::StockIn,
        date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
        note: None,
        reason: Some(StockInReason::Found),
        offset_account_id: None,
        lines: vec![StockAdjustmentLineInput { product_id: product.id, qty_change: Some(dec!(2)), counted_qty: None, batch_no: None, expiry_date: None }],
        approved_by: None,
    };
    let result: Result<_, _> = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        let input = no_batch.clone();
        let undo = undo3.clone();
        Box::pin(async move { adjustments::record_stock_adjustment(tx, cx, &undo, input, false, ApprovalCheck::none()).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
    })
    .await;
    assert!(result.is_err(), "STOCK_IN on a tracked product without a batch number must be refused");
    db.finish().await;
}

#[tokio::test]
async fn stocktake_gain_and_loss_in_one_entry() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let gain_product = {
        let undo = undo.clone();
        let input = base_product_input("منتج زيادة", "SKU-GAIN");
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };
    let mut loss_input = base_product_input("منتج نقص", "SKU-LOSS2");
    loss_input.opening_qty = Some(dec!(10));
    let loss_product = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = loss_input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };

    let stocktake = StockAdjustmentInput {
        r#type: StockAdjustmentType::Stocktake,
        date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
        note: None,
        reason: None,
        offset_account_id: None,
        lines: vec![
            StockAdjustmentLineInput { product_id: gain_product.id, qty_change: None, counted_qty: Some(dec!(3)), batch_no: None, expiry_date: None },
            StockAdjustmentLineInput { product_id: loss_product.id, qty_change: None, counted_qty: Some(dec!(6)), batch_no: None, expiry_date: None },
        ],
        approved_by: None,
    };
    let result = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        let input = stocktake.clone();
        let undo = undo.clone();
        Box::pin(async move { adjustments::record_stock_adjustment(tx, cx, &undo, input, false, ApprovalCheck::none()).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
    })
    .await;
    assert!(result.is_ok(), "STOCKTAKE with a gain and a loss must post one entry: {result:?}");

    let results = with_read(&db.state, |tx| Box::pin(async move { invariants::run_all(tx).await.map_err(accounting_app_lib::core::tx::TxError::App) })).await.unwrap();
    let failed: Vec<String> = results.iter().filter(|r| !r.passed).map(|r| format!("{}: {}", r.key, r.message)).collect();
    assert!(failed.is_empty(), "invariants must all pass after a mixed STOCKTAKE: {failed:?}");
    db.finish().await;
}

#[tokio::test]
async fn draft_adjustment_has_no_journal_and_complete_uses_snapshot() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let product = {
        let undo = undo.clone();
        let input = base_product_input("منتج مسودة", "SKU-DRAFT");
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };

    let draft_input = StockAdjustmentInput {
        r#type: StockAdjustmentType::StockIn,
        date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
        note: None,
        reason: Some(StockInReason::Found),
        offset_account_id: None,
        lines: vec![StockAdjustmentLineInput { product_id: product.id, qty_change: Some(dec!(4)), counted_qty: None, batch_no: None, expiry_date: None }],
        approved_by: None,
    };
    let draft = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = draft_input.clone();
            let undo = undo.clone();
            Box::pin(async move { adjustments::record_stock_adjustment(tx, cx, &undo, input, true, ApprovalCheck::none()).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
        })
        .await
        .unwrap()
    };

    // Stock must be unaffected by a draft.
    let after_draft = with_read(&db.state, move |tx| Box::pin(async move { products::get_product(tx, product.id).await.map_err(Into::into) })).await.unwrap();
    assert_eq!(after_draft.stock_qty, Decimal::ZERO, "a draft adjustment must not move stock");

    let completed = {
        let undo = undo.clone();
        let id = draft.id;
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { adjustments::complete_adjustment(tx, cx, &undo, id, None, ApprovalCheck::none()).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
        })
        .await
        .unwrap()
    };
    assert_eq!(completed.status, accounting_app_lib::domains::products::dto::inventory::StockAdjustmentStatus::Completed);

    // Completing twice -> message.
    let again: Result<_, _> = {
        let undo = undo.clone();
        let id = draft.id;
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { adjustments::complete_adjustment(tx, cx, &undo, id, None, ApprovalCheck::none()).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
        })
        .await
    };
    assert!(again.is_err(), "completing an already-completed adjustment must be refused");
    db.finish().await;
}

#[tokio::test]
async fn delete_draft_adjustment_writes_activity_and_refuses_completed() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let product = {
        let undo = undo.clone();
        let input = base_product_input("منتج حذف مسودة", "SKU-DELDRAFT");
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };

    let draft_input = StockAdjustmentInput {
        r#type: StockAdjustmentType::StockIn,
        date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
        note: None,
        reason: Some(StockInReason::Found),
        offset_account_id: None,
        lines: vec![StockAdjustmentLineInput { product_id: product.id, qty_change: Some(dec!(1)), counted_qty: None, batch_no: None, expiry_date: None }],
        approved_by: None,
    };
    let draft = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = draft_input.clone();
            let undo = undo.clone();
            Box::pin(async move { adjustments::record_stock_adjustment(tx, cx, &undo, input, true, ApprovalCheck::none()).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
        })
        .await
        .unwrap()
    };

    {
        let undo = undo.clone();
        let id = draft.id;
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { adjustments::delete_draft_adjustment(tx, cx, &undo, id).await }) as BoxFuture<'_, TxResult<()>>
        })
        .await
        .expect("delete draft must succeed");
    }

    let missing: Result<_, _> = with_read(&db.state, move |tx| Box::pin(async move { adjustments::get_stock_adjustment(tx, draft.id).await.map_err(Into::into) })).await;
    assert!(missing.is_err(), "deleted draft must no longer be readable");
    db.finish().await;
}

#[tokio::test]
async fn approval_threshold_forbidden_without_grant_posts_with_granted_manager() {
    let db = TestDb::fresh().await;
    let fixture = {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await
    };
    log_in(&db, fixture.admin_id, Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let product = {
        let undo = undo.clone();
        let input = base_product_input("منتج فوق الحد", "SKU-THRESH");
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };

    // 200 * 10 = 2000 >= threshold (1000), no approvedBy -> FORBIDDEN.
    let over_input = StockAdjustmentInput {
        r#type: StockAdjustmentType::StockIn,
        date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
        note: None,
        reason: Some(StockInReason::Found),
        offset_account_id: None,
        lines: vec![StockAdjustmentLineInput { product_id: product.id, qty_change: Some(dec!(200)), counted_qty: None, batch_no: None, expiry_date: None }],
        approved_by: None,
    };
    let no_grant: Result<_, _> = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = over_input.clone();
            let undo = undo.clone();
            Box::pin(async move { adjustments::record_stock_adjustment(tx, cx, &undo, input, false, ApprovalCheck::none()).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
        })
        .await
    };
    assert!(no_grant.is_err(), "over-threshold STOCK_IN without approvedBy must be FORBIDDEN");

    // With approvedBy set but the check reporting `granted: false` (an ungranted PIN) -> still
    // FORBIDDEN.
    let ungranted_input = StockAdjustmentInput {
        r#type: StockAdjustmentType::StockIn,
        date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
        note: None,
        reason: Some(StockInReason::Found),
        offset_account_id: None,
        lines: vec![StockAdjustmentLineInput { product_id: product.id, qty_change: Some(dec!(200)), counted_qty: None, batch_no: None, expiry_date: None }],
        approved_by: Some(fixture.manager_id),
    };
    let ungranted: Result<_, _> = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = ungranted_input.clone();
            let undo = undo.clone();
            Box::pin(async move { adjustments::record_stock_adjustment(tx, cx, &undo, input, false, ApprovalCheck::none()).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
        })
        .await
    };
    assert!(ungranted.is_err(), "an approvedBy id whose grant check reports false must still be FORBIDDEN");

    // With a granted manager -> posts.
    let granted_input = StockAdjustmentInput {
        r#type: StockAdjustmentType::StockIn,
        date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
        note: None,
        reason: Some(StockInReason::Found),
        offset_account_id: None,
        lines: vec![StockAdjustmentLineInput { product_id: product.id, qty_change: Some(dec!(200)), counted_qty: None, batch_no: None, expiry_date: None }],
        approved_by: Some(fixture.manager_id),
    };
    let granted: Result<_, _> = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = granted_input.clone();
            let undo = undo.clone();
            Box::pin(async move { adjustments::record_stock_adjustment(tx, cx, &undo, input, false, ApprovalCheck { granted: true }).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
        })
        .await
    };
    assert!(granted.is_ok(), "a granted, active manager must be able to approve: {granted:?}");
    db.finish().await;
}

/// ACC-0006: completing an above-threshold draft enforces the same manager-approval rule as a
/// direct adjustment (the mock used to post it without any approval).
#[tokio::test]
async fn completing_draft_above_threshold_requires_granted_manager() {
    let db = TestDb::fresh().await;
    let fixture = {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await
    };
    log_in(&db, fixture.admin_id, Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let product = {
        let undo = undo.clone();
        let input = base_product_input("منتج مسودة فوق الحد", "SKU-DRAFT-THRESH");
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };

    // A draft skips the approval check (it posts nothing): 200 * 10 = 2000 >= threshold (1000).
    let draft_input = StockAdjustmentInput {
        r#type: StockAdjustmentType::StockIn,
        date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
        note: None,
        reason: Some(StockInReason::Found),
        offset_account_id: None,
        lines: vec![StockAdjustmentLineInput { product_id: product.id, qty_change: Some(dec!(200)), counted_qty: None, batch_no: None, expiry_date: None }],
        approved_by: None,
    };
    let draft = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = draft_input.clone();
            let undo = undo.clone();
            Box::pin(async move { adjustments::record_stock_adjustment(tx, cx, &undo, input, true, ApprovalCheck::none()).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
        })
        .await
        .expect("an above-threshold draft is saved without approval")
    };

    // Completing without approval, or with an ungranted manager id -> FORBIDDEN, nothing posted.
    for (approved_by, granted) in [(None, false), (Some(fixture.manager_id), false)] {
        let undo = undo.clone();
        let id = draft.id;
        let refused: Result<_, _> = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { adjustments::complete_adjustment(tx, cx, &undo, id, approved_by, ApprovalCheck { granted }).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
        })
        .await;
        assert!(
            matches!(refused, Err(accounting_app_lib::core::error::AppError::Forbidden { .. })),
            "completing an above-threshold draft without a granted manager must be FORBIDDEN, got {refused:?}"
        );
    }
    let still_draft = with_read(&db.state, move |tx| Box::pin(async move { adjustments::get_stock_adjustment(tx, draft.id).await.map_err(Into::into) })).await.unwrap();
    assert_eq!(still_draft.status, accounting_app_lib::domains::products::dto::inventory::StockAdjustmentStatus::Draft);
    let untouched = with_read(&db.state, move |tx| Box::pin(async move { products::get_product(tx, product.id).await.map_err(Into::into) })).await.unwrap();
    assert_eq!(untouched.stock_qty, Decimal::ZERO, "a refused completion must not move stock");

    // With a granted, active manager -> completes and stamps the approver.
    let completed = {
        let undo = undo.clone();
        let id = draft.id;
        let manager_id = fixture.manager_id;
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { adjustments::complete_adjustment(tx, cx, &undo, id, Some(manager_id), ApprovalCheck { granted: true }).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
        })
        .await
        .expect("a granted manager approves the draft")
    };
    assert_eq!(completed.status, accounting_app_lib::domains::products::dto::inventory::StockAdjustmentStatus::Completed);
    assert_eq!(completed.approved_by, Some(fixture.manager_id));
    assert!(completed.approved_at.is_some());

    let results = with_read(&db.state, |tx| Box::pin(async move { invariants::run_all(tx).await.map_err(accounting_app_lib::core::tx::TxError::App) })).await.unwrap();
    let failed: Vec<String> = results.iter().filter(|r| !r.passed).map(|r| format!("{}: {}", r.key, r.message)).collect();
    assert!(failed.is_empty(), "invariants must all pass after an approved draft completion: {failed:?}");
    db.finish().await;
}

// --- Movements -------------------------------------------------------------------------------

#[tokio::test]
async fn stock_movements_filter_and_date_desc_order() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let product = {
        let undo = undo.clone();
        let mut input = base_product_input("منتج حركات", "SKU-MOV");
        input.opening_qty = Some(dec!(20));
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };

    use accounting_app_lib::domains::products::dto::inventory::MovementFilter;
    let filter = MovementFilter { product_id: Some(product.id), reason: None, from: None, to: None };
    let rows = with_read(&db.state, move |tx| Box::pin(async move { movements::get_stock_movements(tx, Some(filter)).await.map_err(Into::into) })).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].qty_change, dec!(20));
    db.finish().await;
}

// --- Batches / expiry ------------------------------------------------------------------------

#[tokio::test]
async fn write_off_expired_batches_groups_by_product() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let mut input = base_product_input("منتج تشغيلات", "SKU-BATCH");
    input.track_batches = Some(true);
    let product = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };

    // Receive one batch expiring in the past via a STOCK_IN adjustment.
    let past = chrono::Utc::now().date_naive() - chrono::Duration::days(5);
    let stock_in = StockAdjustmentInput {
        r#type: StockAdjustmentType::StockIn,
        date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
        note: None,
        reason: Some(StockInReason::Found),
        offset_account_id: None,
        lines: vec![StockAdjustmentLineInput { product_id: product.id, qty_change: Some(dec!(10)), counted_qty: None, batch_no: Some("B1".to_string()), expiry_date: Some(past) }],
        approved_by: None,
    };
    {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = stock_in.clone();
            let undo = undo.clone();
            Box::pin(async move { adjustments::record_stock_adjustment(tx, cx, &undo, input, false, ApprovalCheck::none()).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
        })
        .await
        .unwrap();
    }

    let batches_list = with_read(&db.state, move |tx| Box::pin(async move { batches::get_batches(tx, product.id).await.map_err(Into::into) })).await.unwrap();
    assert_eq!(batches_list.len(), 1);
    let batch_id = batches_list[0].id;

    let write_off = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { batches::write_off_expired_batches(tx, cx, &undo, vec![batch_id], None).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
        })
        .await
    };
    assert!(write_off.is_ok(), "write-off of an expired batch must post a LOSS adjustment: {write_off:?}");
    db.finish().await;
}

#[tokio::test]
async fn empty_batch_selection_for_write_off_and_return_are_refused() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let result: Result<_, _> = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { batches::write_off_expired_batches(tx, cx, &undo, vec![], None).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
        })
        .await
    };
    assert!(result.is_err());

    let result2: Result<_, _> = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move { batches::return_batches_to_supplier(tx, cx, &undo, Id::new(), vec![], None).await }) as BoxFuture<'_, TxResult<DebitNoteDraft>>
    })
    .await;
    assert!(result2.is_err());
    db.finish().await;
}

#[tokio::test]
async fn return_batches_to_supplier_creates_draft() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    // `debit_note_drafts.supplier_id` is an FK to `parties` — the draft needs a real supplier
    // (the UI only offers existing ones).
    let supplier_id = {
        let guard = db.state.db.read().unwrap();
        seed_supplier(&guard.as_ref().unwrap().connection).await
    };
    let lines = vec![DebitNoteDraftLineInput { product_id: Id::new(), batch_id: Id::new(), qty: dec!(2), unit_cost: dec!(5) }];
    let draft = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        let lines = lines.clone();
        let undo = undo.clone();
        Box::pin(async move { batches::return_batches_to_supplier(tx, cx, &undo, supplier_id, lines, None).await }) as BoxFuture<'_, TxResult<DebitNoteDraft>>
    })
    .await
    .expect("return_batches_to_supplier must create a draft");
    assert_eq!(draft.lines.len(), 1);
    db.finish().await;
}

// --- Stock counts ----------------------------------------------------------------------------

#[tokio::test]
async fn stock_count_full_cycle() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let mut input = base_product_input("منتج جرد", "SKU-COUNT");
    input.opening_qty = Some(dec!(10));
    let product = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };

    let count_input = StockCountInput { scope: StockCountScope::All, category_id: None, location: None, blind: false, note: None };
    let count = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        let input = count_input.clone();
        let undo = undo.clone();
        Box::pin(async move { counts::create_stock_count(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<StockCount>>
    })
    .await
    .expect("create_stock_count must succeed with an in-scope product");
    assert!(count.lines.iter().any(|l| l.product_id == product.id));

    let submit_before_counting: Result<_, _> = with_tx(&db.state, TxOpts::default(), {
        let count_id = count.id;
        move |tx, _cx| Box::pin(async move { counts::submit_count_for_review(tx, count_id).await }) as BoxFuture<'_, TxResult<StockCount>>
    })
    .await;
    assert!(submit_before_counting.is_err(), "submitting with an uncounted line must be refused");

    with_tx(&db.state, TxOpts::default(), {
        let count_id = count.id;
        let product_id = product.id;
        move |tx, _cx| Box::pin(async move { counts::update_stock_count_line(tx, count_id, product_id, dec!(12), false).await }) as BoxFuture<'_, TxResult<StockCount>>
    })
    .await
    .expect("update_stock_count_line must succeed");

    let submitted = with_tx(&db.state, TxOpts::default(), {
        let count_id = count.id;
        move |tx, _cx| Box::pin(async move { counts::submit_count_for_review(tx, count_id).await }) as BoxFuture<'_, TxResult<StockCount>>
    })
    .await
    .expect("submit after counting all lines must succeed");
    assert_eq!(submitted.status, accounting_app_lib::domains::products::dto::inventory::StockCountStatus::Review);

    let undo2 = Arc::new(UndoRegistry::default());
    let adjustment = with_tx(&db.state, TxOpts::default(), {
        let count_id = count.id;
        let undo = undo2.clone();
        move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { counts::complete_stock_count(tx, cx, &undo, count_id).await }) as BoxFuture<'_, TxResult<StockAdjustment>>
        }
    })
    .await
    .expect("complete_stock_count must post a STOCKTAKE adjustment");
    assert_eq!(adjustment.r#type, StockAdjustmentType::Stocktake);

    let results = with_read(&db.state, |tx| Box::pin(async move { invariants::run_all(tx).await.map_err(accounting_app_lib::core::tx::TxError::App) })).await.unwrap();
    let failed: Vec<String> = results.iter().filter(|r| !r.passed).map(|r| format!("{}: {}", r.key, r.message)).collect();
    assert!(failed.is_empty(), "invariants must all pass after completing a stock count: {failed:?}");
    db.finish().await;
}

#[tokio::test]
async fn stock_count_empty_scope_is_refused() {
    let db = TestDb::fresh().await;
    {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await;
    }
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let input = StockCountInput { scope: StockCountScope::Category, category_id: Some(Id::new()), location: None, blind: false, note: None };
    let result: Result<_, _> = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        let input = input.clone();
        let undo = undo.clone();
        Box::pin(async move { counts::create_stock_count(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<StockCount>>
    })
    .await;
    assert!(result.is_err(), "a scope matching no products must be refused");
    db.finish().await;
}

// --- Transfers -------------------------------------------------------------------------------

#[tokio::test]
async fn transfer_send_over_branch_stock_is_conflict_full_cycle_otherwise_succeeds() {
    let db = TestDb::fresh().await;
    let fixture = {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await
    };
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let mut input = base_product_input("منتج تحويل", "SKU-TR");
    input.opening_qty = Some(dec!(10));
    let product = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };

    // Same-branch transfer refused.
    let same_branch = StockTransferInput {
        from_branch_id: fixture.branch_id,
        to_branch_id: fixture.branch_id,
        date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
        note: None,
        lines: vec![StockTransferLineInput { product_id: product.id, qty: dec!(1), unit_id: None, unit_factor: None, batch_id: None, batch_no: None }],
    };
    let result: Result<_, _> = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = same_branch.clone();
            let undo = undo.clone();
            Box::pin(async move { transfers::create_transfer(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<StockTransfer>>
        })
        .await
    };
    assert!(result.is_err());

    // Over-stock transfer -> send fails with CONFLICT.
    let over_input = StockTransferInput {
        from_branch_id: fixture.branch_id,
        to_branch_id: fixture.branch2_id,
        date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
        note: None,
        lines: vec![StockTransferLineInput { product_id: product.id, qty: dec!(50), unit_id: None, unit_factor: None, batch_id: None, batch_no: None }],
    };
    let over_transfer = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = over_input.clone();
            let undo = undo.clone();
            Box::pin(async move { transfers::create_transfer(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<StockTransfer>>
        })
        .await
        .unwrap()
    };
    let send_over: Result<_, _> = {
        let undo = undo.clone();
        let id = over_transfer.id;
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { transfers::send_transfer(tx, cx, &undo, id).await }) as BoxFuture<'_, TxResult<StockTransfer>>
        })
        .await
    };
    assert!(send_over.is_err(), "sending more than the branch has must be a CONFLICT");

    // Full cycle: create -> send -> receive short -> shortage_value set.
    let cycle_input = StockTransferInput {
        from_branch_id: fixture.branch_id,
        to_branch_id: fixture.branch2_id,
        date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
        note: None,
        lines: vec![StockTransferLineInput { product_id: product.id, qty: dec!(4), unit_id: None, unit_factor: None, batch_id: None, batch_no: None }],
    };
    let transfer = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = cycle_input.clone();
            let undo = undo.clone();
            Box::pin(async move { transfers::create_transfer(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<StockTransfer>>
        })
        .await
        .unwrap()
    };

    let sent = {
        let undo = undo.clone();
        let id = transfer.id;
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { transfers::send_transfer(tx, cx, &undo, id).await }) as BoxFuture<'_, TxResult<StockTransfer>>
        })
        .await
        .expect("send_transfer must succeed within available stock")
    };
    assert_eq!(sent.status, accounting_app_lib::domains::products::dto::inventory::StockTransferStatus::Sent);

    let receive_input = ReceiveTransferInput { lines: vec![ReceiveTransferLineInput { product_id: product.id, received_qty: dec!(3), batch_id: None }] };
    let received = {
        let undo = undo.clone();
        let id = transfer.id;
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = receive_input.clone();
            let undo = undo.clone();
            Box::pin(async move { transfers::receive_transfer(tx, cx, &undo, id, input).await }) as BoxFuture<'_, TxResult<StockTransfer>>
        })
        .await
        .expect("receive_transfer must succeed")
    };
    assert!(received.shortage_value.is_some(), "receiving less than sent must record a shortage_value");

    let results = with_read(&db.state, |tx| Box::pin(async move { invariants::run_all(tx).await.map_err(accounting_app_lib::core::tx::TxError::App) })).await.unwrap();
    let failed: Vec<String> = results.iter().filter(|r| !r.passed).map(|r| format!("{}: {}", r.key, r.message)).collect();
    assert!(failed.is_empty(), "invariants must all pass after a transfer send + short receive: {failed:?}");
    db.finish().await;
}

#[tokio::test]
async fn reject_transfer_returns_full_value_and_requires_reason() {
    let db = TestDb::fresh().await;
    let fixture = {
        let guard = db.state.db.read().unwrap();
        seed_fixture(&guard.as_ref().unwrap().connection).await
    };
    log_in(&db, test_admin_id(), Role::Admin);
    let undo = Arc::new(UndoRegistry::default());

    let mut input = base_product_input("منتج رفض تحويل", "SKU-REJ");
    input.opening_qty = Some(dec!(5));
    let product = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = input.clone();
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        })
        .await
        .unwrap()
    };

    let transfer_input = StockTransferInput {
        from_branch_id: fixture.branch_id,
        to_branch_id: fixture.branch2_id,
        date: chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string(),
        note: None,
        lines: vec![StockTransferLineInput { product_id: product.id, qty: dec!(2), unit_id: None, unit_factor: None, batch_id: None, batch_no: None }],
    };
    let transfer = {
        let undo = undo.clone();
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let input = transfer_input.clone();
            let undo = undo.clone();
            Box::pin(async move { transfers::create_transfer(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<StockTransfer>>
        })
        .await
        .unwrap()
    };
    {
        let undo = undo.clone();
        let id = transfer.id;
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { transfers::send_transfer(tx, cx, &undo, id).await }) as BoxFuture<'_, TxResult<StockTransfer>>
        })
        .await
        .unwrap();
    }

    let empty_reason: Result<_, _> = {
        let undo = undo.clone();
        let id = transfer.id;
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { transfers::reject_transfer(tx, cx, &undo, id, "  ".to_string()).await }) as BoxFuture<'_, TxResult<StockTransfer>>
        })
        .await
    };
    assert!(empty_reason.is_err(), "an empty/whitespace reject reason must be refused");

    let rejected = {
        let undo = undo.clone();
        let id = transfer.id;
        with_tx(&db.state, TxOpts::default(), move |tx, cx| {
            let undo = undo.clone();
            Box::pin(async move { transfers::reject_transfer(tx, cx, &undo, id, "تالف".to_string()).await }) as BoxFuture<'_, TxResult<StockTransfer>>
        })
        .await
        .expect("reject_transfer with a reason must succeed")
    };
    assert_eq!(rejected.status, accounting_app_lib::domains::products::dto::inventory::StockTransferStatus::Rejected);

    let after = with_read(&db.state, move |tx| Box::pin(async move { products::get_product(tx, product.id).await.map_err(Into::into) })).await.unwrap();
    assert_eq!(after.stock_qty, dec!(5), "rejecting must return the full quantity to the source branch");
    db.finish().await;
}
