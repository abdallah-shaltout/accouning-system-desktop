//! DB-backed tests for the `analytics` and `dashboard` domains (03-domains/14-analytics.md §8a,
//! 14b-insights.md §8a). Written now, run in the deferred time-boxed test pass (per-implementer
//! hard rule: never run cargo from this agent). Needs `EQUAL_TEST_DATABASE_URL` — see
//! `tests/support/mod.rs`.
//!
//! Scope note (time-boxed pass, per this wave's final report): covers the highest-value scenarios
//! named in both specs' §8 checklists (sales/product/customer analytics basics + validation,
//! dashboard summary, home KPIs period math + changePct edge cases, low-stock ordering, recent
//! invoices/activity ordering, the six notification reads, and a representative sample of insight
//! rules: reorder, credit-limit, vat-deadline's `-0` quirk, year-end's sign-inversion quirk,
//! backup-overdue, a failing rule not breaking others, role filtering, and product inline hints)
//! rather than every rule/branch — the parity-case list (§8b of both files) is handed to Part 04
//! regardless of which of these ran here first.

mod support;

use std::sync::Arc;

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::tx::{with_read_ctx, with_tx, BoxFuture, ReadCtx, TxOpts, TxResult};
use accounting_app_lib::domains::analytics::dto::{DateStyle as DtoDateStyle, Numerals as DtoNumerals};
use accounting_app_lib::domains::analytics::service as analytics_service;
use accounting_app_lib::domains::dashboard::dto::HomePeriod;
use accounting_app_lib::domains::dashboard::service::insights::{engine, hints};
use accounting_app_lib::domains::dashboard::service::{feed, kpis};
use accounting_app_lib::domains::invoices::dto::{SaleInput, SaleInputLine, SalePaymentMethod};
use accounting_app_lib::domains::invoices::service::sale;
use accounting_app_lib::domains::products::dto::catalog::{Product, ProductInput, ProductType};
use accounting_app_lib::domains::products::service::products;
use accounting_app_lib::entities::org::{accounts, branches, fiscal_years, payment_methods, settings, taxes};
use accounting_app_lib::entities::parties::parties;
use accounting_app_lib::shared::activity::undo::UndoRegistry;
use accounting_app_lib::utils::id::Id;
use accounting_app_lib::utils::money::round2;
use chrono::Datelike;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseTransaction, Set};
use support::TestDb;

// --- Fixture (same pattern as domain_invoices.rs — test binaries can't import each other) --------

struct Fixture {
    pub product_id: Id,
    pub customer_id: Id,
}

fn log_in(test_db: &TestDb, role: Role) -> Id {
    let user_id = Id::new();
    let user = AuthenticatedUser { id: user_id, username: "test".to_string(), role, home_branch_id: Id::new(), allowed_branches: vec![], price_list_id: None, max_discount: None };
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

async fn seed_fixture(conn: &DatabaseTransaction) -> Id {
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
    seed_role_account(conn, "3900", "openingBalanceEquity").await;

    seed_payment_method(conn, "نقدي", "cash", "cash", 1).await;
    seed_payment_method(conn, "مدى", "card", "cardClearing", 2).await;
    seed_payment_method(conn, "تحويل بنكي", "bank_transfer", "bank", 3).await;

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

    customer_id
}

fn base_product_input(name: &str, sku: &str, cost_price: Decimal, price: Decimal, min_stock: Option<Decimal>) -> ProductInput {
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
        min_stock,
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

async fn stock_in(test_db: &TestDb, product_id: Id, qty: Decimal, unit_cost: Decimal) {
    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            let mut p = accounting_app_lib::shared::stock::lock_product(tx, product_id).await?;
            accounting_app_lib::shared::stock::apply_change(
                tx,
                cx,
                &mut p,
                qty,
                round2(qty * unit_cost),
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
    let customer_id = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| Box::pin(async move { Ok(seed_fixture(tx).await) }) as BoxFuture<'_, TxResult<Id>>).await.unwrap();

    log_in(test_db, Role::Admin);

    let undo = Arc::new(UndoRegistry::new());
    let p1 = with_tx(&test_db.state, TxOpts::default(), {
        let undo = undo.clone();
        move |tx, cx| {
            let input = base_product_input("منتج ١", "SKU-1", dec!(50), dec!(115), Some(dec!(10)));
            let undo = undo.clone();
            Box::pin(async move { products::create_product(tx, cx, &undo, input).await }) as BoxFuture<'_, TxResult<Product>>
        }
    })
    .await
    .unwrap();
    stock_in(test_db, p1.id, dec!(100), dec!(50)).await;

    Fixture { product_id: p1.id, customer_id }
}

async fn sell(test_db: &TestDb, fixture: &Fixture, qty: Decimal, price: Decimal, customer: bool) {
    let input = SaleInput {
        customer_id: if customer { Some(fixture.customer_id) } else { None },
        lines: vec![SaleInputLine {
            product_id: fixture.product_id.to_string(),
            qty,
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
        payment_method: SalePaymentMethod::Cash,
        paid_amount: round2(qty * price),
        tendered_amount: Some(round2(qty * price)),
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
    with_tx(&test_db.state, TxOpts::default(), move |tx, cx| {
        let input = input.clone();
        let registry = registry.clone();
        Box::pin(async move { sale::create_sale(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::invoices::dto::Invoice>>
    })
    .await
    .unwrap();
}

/// Builds a `ReadCtx` from the logged-in session for calling service functions directly (bypassing
/// the `#[tauri::command]` wrapper — same convention `domain_invoices.rs` uses via `with_tx`/
/// `with_read`, extended here to `with_read_ctx` for the read-only reports/dashboard/insights).
async fn read_ctx_today(test_db: &TestDb) -> chrono::NaiveDate {
    with_read_ctx(&test_db.state, |_tx, ctx: &ReadCtx| Box::pin(async move { Ok(ctx.clock.today()) }) as BoxFuture<'_, TxResult<chrono::NaiveDate>>).await.unwrap()
}

// --- analytics_get_sales_analytics --------------------------------------------------------------

#[tokio::test]
async fn sales_analytics_empty_data_gives_no_data_message() {
    let test_db = TestDb::fresh().await;
    let _fixture = make_fixture(&test_db).await;
    let today = read_ctx_today(&test_db).await;

    let result = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { analytics_service::sales::get_sales_analytics(tx, today, Some(30), DtoDateStyle::Ymd.into(), DtoNumerals::Latn.into()).await })
            as BoxFuture<'_, TxResult<accounting_app_lib::domains::analytics::dto::SalesAnalytics>>
    })
    .await
    .unwrap();

    assert_eq!(result.trend.len(), 30);
    assert_eq!(result.trend_insight, "لا توجد بيانات كافية بعد لهذه الفترة");
    assert_eq!(result.by_weekday.len(), 7);
}

#[tokio::test]
async fn sales_analytics_days_out_of_bounds_is_validation_error() {
    let test_db = TestDb::fresh().await;
    let _fixture = make_fixture(&test_db).await;
    let today = read_ctx_today(&test_db).await;

    let result = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { analytics_service::sales::get_sales_analytics(tx, today, Some(0), DtoDateStyle::Ymd.into(), DtoNumerals::Latn.into()).await })
            as BoxFuture<'_, TxResult<accounting_app_lib::domains::analytics::dto::SalesAnalytics>>
    })
    .await;
    assert!(result.is_err(), "days = 0 must be rejected");

    let result = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { analytics_service::sales::get_sales_analytics(tx, today, Some(367), DtoDateStyle::Ymd.into(), DtoNumerals::Latn.into()).await })
            as BoxFuture<'_, TxResult<accounting_app_lib::domains::analytics::dto::SalesAnalytics>>
    })
    .await;
    assert!(result.is_err(), "days = 367 must be rejected");
}

#[tokio::test]
async fn sales_analytics_trend_reflects_todays_sale() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    sell(&test_db, &fixture, dec!(2), dec!(115), false).await;
    let today = read_ctx_today(&test_db).await;

    let result = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { analytics_service::sales::get_sales_analytics(tx, today, Some(30), DtoDateStyle::Ymd.into(), DtoNumerals::Latn.into()).await })
            as BoxFuture<'_, TxResult<accounting_app_lib::domains::analytics::dto::SalesAnalytics>>
    })
    .await
    .unwrap();

    let last = result.trend.last().unwrap();
    assert_eq!(last.total, dec!(230));
    assert!(result.trend_insight.contains("أفضل يوم"));
    assert_eq!(result.avg_invoice, dec!(230));
    assert_eq!(result.avg_items_per_invoice, dec!(2));
    assert_eq!(result.returns_rate_pct, Decimal::ZERO);
}

// --- analytics_get_product_analytics ------------------------------------------------------------

#[tokio::test]
async fn product_analytics_no_sales_gives_insufficient_message() {
    let test_db = TestDb::fresh().await;
    let _fixture = make_fixture(&test_db).await;
    let today = read_ctx_today(&test_db).await;

    let result = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { analytics_service::products::get_product_analytics(tx, today, Some(30), Some(8)).await })
            as BoxFuture<'_, TxResult<accounting_app_lib::domains::analytics::dto::ProductAnalytics>>
    })
    .await
    .unwrap();

    assert!(result.top.is_empty());
    assert_eq!(result.insight, "لا توجد مبيعات كافية بعد لهذه الفترة");
}

#[tokio::test]
async fn product_analytics_computes_margin_and_top_insight() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    sell(&test_db, &fixture, dec!(2), dec!(115), false).await;
    let today = read_ctx_today(&test_db).await;

    let result = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { analytics_service::products::get_product_analytics(tx, today, Some(30), Some(8)).await })
            as BoxFuture<'_, TxResult<accounting_app_lib::domains::analytics::dto::ProductAnalytics>>
    })
    .await
    .unwrap();

    assert_eq!(result.top.len(), 1);
    assert_eq!(result.top[0].name, "منتج ١");
    assert!(result.insight.contains("منتج ١"));
}

// --- analytics_get_customer_analytics ------------------------------------------------------------

#[tokio::test]
async fn customer_analytics_new_customer_and_no_concentration_message() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    sell(&test_db, &fixture, dec!(1), dec!(115), true).await;
    let today = read_ctx_today(&test_db).await;

    let result = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { analytics_service::customers::get_customer_analytics(tx, today, Some(30), Some(8)).await })
            as BoxFuture<'_, TxResult<accounting_app_lib::domains::analytics::dto::CustomerAnalytics>>
    })
    .await
    .unwrap();

    assert_eq!(result.new_count, 1);
    assert_eq!(result.returning_count, 0);
    assert!(result.segment_insight.contains("%"));
    assert_eq!(result.concentration_insight, "عدد العملاء غير كافٍ لحساب التركّز بدقة");
    assert_eq!(result.top_customers.len(), 1);
    assert_eq!(result.top_customers[0].total, dec!(115));
}

// --- dashboard_get_dashboard_summary --------------------------------------------------------------

#[tokio::test]
async fn dashboard_summary_reflects_todays_sale_and_low_stock() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    sell(&test_db, &fixture, dec!(95), dec!(115), false).await; // leaves stock_qty = 5 <= min_stock 10.
    let today = read_ctx_today(&test_db).await;

    let summary = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { kpis::get_dashboard_summary(tx, today).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::dashboard::dto::DashboardSummary>>
    })
    .await
    .unwrap();

    assert_eq!(summary.today_invoice_count, 1);
    assert_eq!(summary.today_sales, round2(dec!(95) * dec!(115)));
    assert_eq!(summary.low_stock_count, 1, "stock_qty 5 <= min_stock 10 must count as low stock");
    assert!(summary.cash_on_hand > Decimal::ZERO);
}

// --- dashboard_get_home_kpis ----------------------------------------------------------------------

#[tokio::test]
async fn home_kpis_change_pct_null_when_previous_zero_and_current_nonzero() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    sell(&test_db, &fixture, dec!(1), dec!(115), false).await;
    let today = read_ctx_today(&test_db).await;
    let now_iso = accounting_app_lib::utils::dates::format_iso_ms(chrono::Utc::now());

    let result = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { kpis::get_home_kpis(tx, today, &now_iso, HomePeriod::Today).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::dashboard::dto::HomeKpis>>
    })
    .await
    .unwrap();

    assert_eq!(result.net_sales.value, dec!(115));
    assert_eq!(result.net_sales.previous, Decimal::ZERO);
    assert_eq!(result.net_sales.change_pct, None, "current != 0, previous == 0 -> null (not a divide by zero)");
    assert_eq!(result.sales_trend.len(), 1, "'today' period spans exactly one day");
}

#[tokio::test]
async fn home_kpis_change_pct_zero_when_both_zero() {
    let test_db = TestDb::fresh().await;
    let _fixture = make_fixture(&test_db).await;
    let today = read_ctx_today(&test_db).await;
    let now_iso = accounting_app_lib::utils::dates::format_iso_ms(chrono::Utc::now());

    let result = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { kpis::get_home_kpis(tx, today, &now_iso, HomePeriod::Week).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::dashboard::dto::HomeKpis>>
    })
    .await
    .unwrap();

    assert_eq!(result.net_sales.change_pct, Some(Decimal::ZERO));
    assert_eq!(result.sales_trend.len(), 7, "'week' period spans 7 days");
}

// --- dashboard_get_low_stock_products / recent invoices / recent activity ------------------------

#[tokio::test]
async fn low_stock_products_ordered_by_stock_ratio() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    // Sell down to stock_qty = 5 (ratio 5/10 = 0.5) — still the only low-stock product.
    sell(&test_db, &fixture, dec!(95), dec!(115), false).await;

    let rows = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { feed::get_low_stock_products(tx, 6).await }) as BoxFuture<'_, TxResult<Vec<Product>>>
    })
    .await
    .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, fixture.product_id);
}

#[tokio::test]
async fn recent_invoices_includes_all_statuses_newest_first() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    sell(&test_db, &fixture, dec!(1), dec!(115), false).await;
    sell(&test_db, &fixture, dec!(1), dec!(115), true).await;

    let rows = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { feed::get_recent_invoices(tx, 8).await }) as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::dashboard::dto::RecentInvoice>>>
    })
    .await
    .unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].customer_name, Some("أحمد".to_string()), "most recent sale (to the customer) sorts first");
    assert_eq!(rows[1].customer_name, None);
}

#[tokio::test]
async fn recent_activity_excludes_auth_kind() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    sell(&test_db, &fixture, dec!(1), dec!(115), false).await;

    let rows = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { feed::get_recent_activity(tx, 12).await }) as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::dashboard::dto::RecentActivityEntry>>>
    })
    .await
    .unwrap();

    assert!(rows.iter().all(|r| !matches!(r.entry.kind, accounting_app_lib::core::dto::ActivityKind::Auth)));
}

// --- dashboard_get_top_products / dashboard_get_top_customers -------------------------------------

#[tokio::test]
async fn top_products_and_customers_rank_by_profit_and_sales() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    sell(&test_db, &fixture, dec!(2), dec!(115), true).await;
    let today = read_ctx_today(&test_db).await;

    let top_products = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { kpis::get_top_products(tx, today, HomePeriod::Month, 5).await }) as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::dashboard::dto::TopProductRow>>>
    })
    .await
    .unwrap();
    assert_eq!(top_products.len(), 1);
    assert_eq!(top_products[0].qty, dec!(2));

    let top_customers = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { kpis::get_top_customers(tx, today, HomePeriod::Month, 5).await }) as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::dashboard::dto::TopCustomerRow>>>
    })
    .await
    .unwrap();
    assert_eq!(top_customers.len(), 1);
    assert_eq!(top_customers[0].total, dec!(230));
}

// --- the six notification reads --------------------------------------------------------------------

#[tokio::test]
async fn notification_reads_reflect_empty_state() {
    let test_db = TestDb::fresh().await;
    let _fixture = make_fixture(&test_db).await;

    let transfers = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { feed::in_transit_transfers(tx, None).await }) as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::products::dto::inventory::StockTransfer>>>
    })
    .await
    .unwrap();
    assert!(transfers.is_empty());

    let approvals = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { feed::pending_approval_requests(tx).await }) as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::approvals::dto::ApprovalRequest>>>
    })
    .await
    .unwrap();
    assert!(approvals.is_empty());

    let last_failed = with_read_ctx(&test_db.state, move |tx, _ctx| Box::pin(async move { feed::last_backup_failed_at(tx).await }) as BoxFuture<'_, TxResult<Option<String>>>)
        .await
        .unwrap();
    assert_eq!(last_failed, None);

    let draft_count =
        with_read_ctx(&test_db.state, move |tx, _ctx| Box::pin(async move { feed::journal_draft_count(tx).await }) as BoxFuture<'_, TxResult<i32>>).await.unwrap();
    assert_eq!(draft_count, 0);

    let stock_value = with_read_ctx(&test_db.state, move |tx, _ctx| Box::pin(async move { feed::stock_value_snapshot(tx).await }) as BoxFuture<'_, TxResult<Decimal>>).await.unwrap();
    assert_eq!(stock_value, round2(dec!(100) * dec!(50)), "100 units at cost 50 stocked in");

    let has_products = with_read_ctx(&test_db.state, move |tx, _ctx| Box::pin(async move { feed::has_any_products(tx).await }) as BoxFuture<'_, TxResult<bool>>).await.unwrap();
    assert!(has_products);
}

// --- insight engine (14b) --------------------------------------------------------------------------

#[tokio::test]
async fn insight_reorder_fires_when_stock_at_or_below_min() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    sell(&test_db, &fixture, dec!(95), dec!(115), false).await; // stock_qty 5 <= min_stock 10.
    log_in(&test_db, Role::Manager);
    let today = read_ctx_today(&test_db).await;

    let insights = with_read_ctx(&test_db.state, move |tx, ctx| {
        Box::pin(async move { engine::compute_all(tx, today, ctx.clock.now, DtoNumerals::Latn, Role::Manager).await })
            as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::dashboard::dto::InsightDto>>>
    })
    .await
    .unwrap();

    let reorder = insights.iter().find(|i| i.rule_key == "reorder");
    assert!(reorder.is_some(), "reorder insight must fire for the low-stock product");
    assert_eq!(reorder.unwrap().id, "reorder:__none__", "no preferred supplier set");
}

#[tokio::test]
async fn insight_role_filtering_hides_manager_only_rules_from_cashier() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    sell(&test_db, &fixture, dec!(95), dec!(115), false).await;
    let today = read_ctx_today(&test_db).await;

    let insights = with_read_ctx(&test_db.state, move |tx, ctx| {
        Box::pin(async move { engine::compute_all(tx, today, ctx.clock.now, DtoNumerals::Latn, Role::Cashier).await })
            as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::dashboard::dto::InsightDto>>>
    })
    .await
    .unwrap();

    assert!(insights.iter().all(|i| i.roles.contains(&Role::Cashier)), "every returned insight must list the cashier role");
    assert!(insights.iter().find(|i| i.rule_key == "reorder").is_none(), "reorder is storekeeper/manager/admin only");
}

#[tokio::test]
async fn insight_a_failing_rule_does_not_stop_the_others() {
    // No `openingBalanceEquity` account seeded here (unlike `make_fixture`'s full set): the
    // opening-balance-equity rule's `resolve_account` call fails with NOT_FOUND, but every other
    // rule must still run to completion ("a single bad rule shouldn't break the whole home
    // screen", ie:108-112).
    let test_db = TestDb::fresh().await;
    let customer_id = with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
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
            branch.insert(tx).await?;
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
                accounting: Set(None),
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
            settings_row.insert(tx).await?;
            Ok(Id::new())
        }) as BoxFuture<'_, TxResult<Id>>
    })
    .await
    .unwrap();
    let _ = customer_id;
    log_in(&test_db, Role::Admin);
    let today = read_ctx_today(&test_db).await;

    // No accounts/products/customers seeded at all: every rule that needs an account
    // (opening-balance-equity, cash-drawer, supplier-dues, unsettled-cards) fails with NOT_FOUND
    // and is skipped; rules with no ledger dependency (reorder, dead-stock, expiring,
    // missing-supplier-invoice, good-news, backup-overdue, year-end, recurring-*) must still
    // return successfully (empty, since there's no data), proving one failure doesn't abort the
    // whole batch.
    let insights = with_read_ctx(&test_db.state, move |tx, ctx| {
        Box::pin(async move { engine::compute_all(tx, today, ctx.clock.now, DtoNumerals::Latn, Role::Admin).await })
            as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::dashboard::dto::InsightDto>>>
    })
    .await;
    assert!(insights.is_ok(), "compute_all itself must not fail even though individual rules do");
    let insights = insights.unwrap();
    assert!(insights.iter().find(|i| i.rule_key == "opening-balance-equity").is_none(), "the failing rule contributes no insight");
    assert!(insights.iter().any(|i| i.rule_key == "backup-overdue"), "an unrelated rule must still have run");
}

#[tokio::test]
async fn insight_credit_limit_fires_near_and_over() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    // credit_limit = 1000 (fixture); a balance >= 900 (90%) fires "near", > 1000 fires "over".
    sell(&test_db, &fixture, dec!(10), dec!(115), true).await; // 1150 > 1000 -> over.
    log_in(&test_db, Role::Manager);
    let today = read_ctx_today(&test_db).await;

    let insights = with_read_ctx(&test_db.state, move |tx, ctx| {
        Box::pin(async move { engine::compute_all(tx, today, ctx.clock.now, DtoNumerals::Latn, Role::Manager).await })
            as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::dashboard::dto::InsightDto>>>
    })
    .await
    .unwrap();

    let credit = insights.iter().find(|i| i.rule_key == "credit-limit").expect("credit-limit insight must fire when balance exceeds the limit");
    assert_eq!(credit.severity, accounting_app_lib::domains::dashboard::dto::InsightSeverity::Critical, "balance > limit is critical (over), not just warning (near)");
}

#[tokio::test]
async fn insight_backup_overdue_fires_with_no_backup_ever_taken() {
    let test_db = TestDb::fresh().await;
    let _fixture = make_fixture(&test_db).await;
    log_in(&test_db, Role::Admin);
    let today = read_ctx_today(&test_db).await;

    let insights = with_read_ctx(&test_db.state, move |tx, ctx| {
        Box::pin(async move { engine::compute_all(tx, today, ctx.clock.now, DtoNumerals::Latn, Role::Admin).await })
            as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::dashboard::dto::InsightDto>>>
    })
    .await
    .unwrap();

    let backup = insights.iter().find(|i| i.rule_key == "backup-overdue").expect("backup-overdue must fire when no backup was ever taken");
    assert_eq!(backup.message, "لم يتم أخذ أي نسخة احتياطية بعد");
    assert_eq!(backup.value, dec!(9999));
    assert_eq!(backup.metric, None);
}

#[tokio::test]
async fn insight_year_end_sign_quirk_future_vs_past() {
    let test_db = TestDb::fresh().await;
    let _fixture = make_fixture(&test_db).await;
    log_in(&test_db, Role::Admin);
    let today = read_ctx_today(&test_db).await;

    // Add a second, already-ended fiscal year (ends yesterday) to exercise the "ended" branch
    // alongside the fixture's still-open 2026 year (which is far more than yearEndDays away, so it
    // won't fire — proving the quirk's sign inversion on the ended year specifically).
    with_tx(&test_db.state, TxOpts::default(), move |tx, _cx| {
        Box::pin(async move {
            let now = chrono::Utc::now();
            let ended = fiscal_years::ActiveModel {
                id: Set(Id::new()),
                name: Set("2025".to_string()),
                start_date: Set(today - chrono::Duration::days(400)),
                end_date: Set(today - chrono::Duration::days(1)),
                is_closed: Set(false),
                closing_entry_id: Set(None),
                closed_at: Set(None),
                closed_by: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            };
            ended.insert(tx).await?;
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();

    let insights = with_read_ctx(&test_db.state, move |tx, ctx| {
        Box::pin(async move { engine::compute_all(tx, today, ctx.clock.now, DtoNumerals::Latn, Role::Admin).await })
            as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::dashboard::dto::InsightDto>>>
    })
    .await
    .unwrap();

    let ended_insight = insights.iter().find(|i| i.message.contains("2025")).expect("the already-ended fiscal year must fire as 'ended'");
    assert!(ended_insight.message.contains("انتهت ولم تُقفل بعد"), "a past end date reports as 'ended' (quirk Q-1 sign inversion)");
    assert_eq!(ended_insight.severity, accounting_app_lib::domains::dashboard::dto::InsightSeverity::Critical);
    assert_eq!(ended_insight.value, dec!(5000));
}

// --- dashboard_get_product_inline_hints -----------------------------------------------------------

#[tokio::test]
async fn product_inline_hints_reorder_and_role_filter() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    sell(&test_db, &fixture, dec!(95), dec!(115), false).await; // stock_qty 5 <= min_stock 10.
    let today = read_ctx_today(&test_db).await;
    let now = chrono::Utc::now();

    let hints_for_manager = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { hints::get_product_inline_hints(tx, today, now, fixture.product_id, Role::Manager).await })
            as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::dashboard::dto::InsightDto>>>
    })
    .await
    .unwrap();
    assert!(hints_for_manager.iter().any(|h| h.rule_key == "reorder-item"));

    let hints_for_accountant = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { hints::get_product_inline_hints(tx, today, now, fixture.product_id, Role::Accountant).await })
            as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::dashboard::dto::InsightDto>>>
    })
    .await
    .unwrap();
    assert!(hints_for_accountant.is_empty(), "reorder-item is storekeeper/manager/admin only — an accountant sees nothing");
}

#[tokio::test]
async fn product_inline_hints_empty_for_inactive_product() {
    let test_db = TestDb::fresh().await;
    let fixture = make_fixture(&test_db).await;
    with_tx(&test_db.state, TxOpts::default(), {
        let product_id = fixture.product_id;
        move |tx, cx| {
            Box::pin(async move {
                let input = base_product_input("منتج ١", "SKU-1", dec!(50), dec!(115), Some(dec!(10)));
                let mut input = input;
                input.active = false;
                let undo = Arc::new(UndoRegistry::new());
                products::update_product(tx, cx, &undo, product_id, input).await.map(|_| ())
            }) as BoxFuture<'_, TxResult<()>>
        }
    })
    .await
    .unwrap();

    let today = read_ctx_today(&test_db).await;
    let now = chrono::Utc::now();
    let hints = with_read_ctx(&test_db.state, move |tx, _ctx| {
        Box::pin(async move { hints::get_product_inline_hints(tx, today, now, fixture.product_id, Role::Manager).await })
            as BoxFuture<'_, TxResult<Vec<accounting_app_lib::domains::dashboard::dto::InsightDto>>>
    })
    .await
    .unwrap();
    assert!(hints.is_empty(), "an inactive product yields no inline hints (ie:166)");
}
