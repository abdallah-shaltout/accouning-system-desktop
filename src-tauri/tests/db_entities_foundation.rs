//! Phase B (owner B1) DB-backed tests: constraint/round-trip/collation tests for the master-data
//! tables (`m0002…m0007`). Needs `EQUAL_TEST_DATABASE_URL` (see `tests/support/mod.rs`) — never
//! skipped when absent, panics with a clear message instead.

mod support;

use chrono::{TimeZone, Utc};
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DbErr, EntityTrait};

use accounting_app_lib::entities::catalog::{categories, products};
use accounting_app_lib::entities::org::{settings, taxes};
use accounting_app_lib::entities::soft_delete::SoftDelete;
use accounting_app_lib::entities::values::{Address, FeatureFlags, PrinterConnectionType, PrinterMode, PrinterSettings, ThermalPrinterSettings};
use accounting_app_lib::utils::id::Id;
use support::TestDb;

fn is_duplicate_key_error(err: &DbErr) -> bool {
    // MariaDB/MySQL errno 1062 = ER_DUP_ENTRY, surfaced by sqlx as part of the error message.
    format!("{err}").contains("1062") || format!("{err}").to_lowercase().contains("duplicate")
}

/// A minimal, valid `settings` row (the columns `NOT NULL` requires), for tests that need *some*
/// branch/default-tax/etc. FK targets to exist — inserted via raw SQL rather than the entity, since
/// this file's job is testing constraints/round-trips, not exercising a full setup-wizard flow.
async fn seed_minimal_branch<C: ConnectionTrait>(conn: &C) -> Id {
    let branch_id = Id::new();
    let stmt = sea_orm::Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO branches (id, name, code, active, can_delete) VALUES (?, ?, ?, 1, 1)",
        [branch_id.to_string().into(), "الفرع الرئيسي".into(), "MAIN".into()],
    );
    conn.execute(stmt).await.unwrap();
    branch_id
}

// --- Soft-delete re-create (categories) -----------------------------------------------------------

#[tokio::test]
async fn soft_deleted_category_name_can_be_recreated() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let first_id = Id::new();
    let am = categories::ActiveModel {
        id: Set(first_id),
        name: Set("مواد غذائية".to_string()),
        parent_id: Set(None),
        purchase_account_id: Set(None),
        revenue_account_id: Set(None),
        cogs_account_id: Set(None),
        sale_tax_id: Set(None),
        purchase_tax_id: Set(None),
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: sea_orm::ActiveValue::NotSet,
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
        name_live: sea_orm::ActiveValue::NotSet,
    };
    am.insert(conn).await.unwrap();

    // Soft-delete it.
    categories::Entity::soft_delete(conn, first_id, Utc::now()).await.unwrap();

    // Re-create with the exact same name — must succeed (the generated `_live` unique excludes
    // soft-deleted rows).
    let second_id = Id::new();
    let am2 = categories::ActiveModel {
        id: Set(second_id),
        name: Set("مواد غذائية".to_string()),
        parent_id: Set(None),
        purchase_account_id: Set(None),
        revenue_account_id: Set(None),
        cogs_account_id: Set(None),
        sale_tax_id: Set(None),
        purchase_tax_id: Set(None),
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: sea_orm::ActiveValue::NotSet,
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
        name_live: sea_orm::ActiveValue::NotSet,
    };
    am2.insert(conn).await.expect("re-creating a soft-deleted category's name must succeed");

    // `find_live()` only returns the live one.
    let live = categories::Entity::find_live().all(conn).await.unwrap();
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].id, second_id);
}

// --- Case-sensitivity: SKU vs. category name (products.md / B-1 collation rules) -------------------

#[tokio::test]
async fn sku_conflicts_case_insensitively_on_products() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    insert_minimal_product(conn, "SKU-1").await.unwrap();
    let err = insert_minimal_product(conn, "sku-1").await.expect_err("SKU-1 vs sku-1 must conflict (ci collation)");
    assert!(is_duplicate_key_error(&err), "expected a duplicate-key error, got: {err}");
}

#[tokio::test]
async fn category_name_does_not_conflict_case_sensitively() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let a = categories::ActiveModel {
        id: Set(Id::new()),
        name: Set("Food".to_string()),
        parent_id: Set(None),
        purchase_account_id: Set(None),
        revenue_account_id: Set(None),
        cogs_account_id: Set(None),
        sale_tax_id: Set(None),
        purchase_tax_id: Set(None),
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: sea_orm::ActiveValue::NotSet,
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
        name_live: sea_orm::ActiveValue::NotSet,
    };
    a.insert(conn).await.unwrap();

    let b = categories::ActiveModel {
        id: Set(Id::new()),
        name: Set("food".to_string()),
        parent_id: Set(None),
        purchase_account_id: Set(None),
        revenue_account_id: Set(None),
        cogs_account_id: Set(None),
        sale_tax_id: Set(None),
        purchase_tax_id: Set(None),
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: sea_orm::ActiveValue::NotSet,
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
        name_live: sea_orm::ActiveValue::NotSet,
    };
    b.insert(conn).await.expect("\"Food\" vs \"food\" must NOT conflict on categories (case-sensitive collation)");
}

async fn insert_minimal_product<C: ConnectionTrait>(conn: &C, sku: &str) -> Result<products::Model, DbErr> {
    let am = products::ActiveModel {
        id: Set(Id::new()),
        name: Set("منتج تجريبي".to_string()),
        name_en: Set(None),
        sku: Set(sku.to_string()),
        barcode: Set(None),
        category_id: Set(None),
        unit_id: Set(None),
        r#type: Set("product".to_string()),
        stock_mode: Set(None),
        cost_price: Set(dec!(0)),
        price: Set(dec!(0)),
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
        track_batches: Set(None),
        expiry_alert_days: Set(None),
        warranty_months: Set(None),
        warranty_provider: Set(None),
        weight: Set(None),
        custom_fields: Set(None),
        search_normalized: Set(None),
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: sea_orm::ActiveValue::NotSet,
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
        sku_live: sea_orm::ActiveValue::NotSet,
    };
    am.insert(conn).await
}

// --- Settings row: JSON policies + Decimal scale round-trip ----------------------------------------

#[tokio::test]
async fn settings_row_with_json_policies_round_trips_field_equal() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let branch_id = seed_minimal_branch(conn).await;

    let printer = PrinterSettings {
        mode: PrinterMode::Thermal,
        thermal_width_mm: 80,
        thermal: Some(ThermalPrinterSettings {
            printer_name: Some("EPSON-TM88".to_string()),
            connection: PrinterConnectionType::Windows,
            host: None,
            dpi: 203,
            cut: true,
            open_drawer: true,
            copies: 1,
        }),
        a4_printer_name: None,
        label_printer_name: None,
        a4_template: None,
        image_template: None,
    };
    let features = FeatureFlags { branches: Some(true), currencies: Some(false), cost_centers: Some(false) };
    let national_address = Address {
        country: "EG".to_string(),
        region_id: None,
        region_name: None,
        city_id: None,
        city_name: None,
        district_id: None,
        district_name: None,
        region_free_text: None,
        city_free_text: None,
        district_free_text: None,
        street: Some("شارع التحرير".to_string()),
        building_no: Some("10".to_string()),
        floor: None,
        apartment: None,
        landmark: None,
        postal_code: None,
        sa_building_no: None,
        sa_additional_no: None,
        sa_postal_code: None,
        sa_unit_no: None,
        sa_short_address: None,
    };

    let am = settings::ActiveModel {
        id: Set(Id::new()),
        singleton: Set(1),
        store_name: Set("متجر إيكوال".to_string()),
        logo: Set(None),
        stamp: Set(None),
        signature: Set(None),
        currency: Set("EGP".to_string()),
        country: Set(Some("EG".to_string())),
        vat_number: Set(None),
        default_tax_id: Set(None),
        invoice_number_prefix: Set("INV".to_string()),
        printer: Set(printer.clone()),
        prices_include_tax: Set(true),
        address: Set(None),
        national_address: Set(Some(national_address.clone())),
        phone: Set(None),
        commercial_register: Set(None),
        receipt_footer: Set(None),
        accounting: Set(None),
        backup: Set(None),
        inventory_approval_threshold: Set(Some(dec!(500.25))),
        role_access_overrides: Set(None),
        insight_thresholds: Set(None),
        pos: Set(None),
        sales: Set(None),
        features: Set(Some(features.clone())),
        onboarding: Set(None),
        timezone: Set(Some("Asia/Riyadh".to_string())),
        default_branch_id: Set(branch_id),
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: sea_orm::ActiveValue::NotSet,
    };
    am.insert(conn).await.unwrap();

    let loaded = accounting_app_lib::core::settings::load(conn).await.unwrap();
    assert_eq!(loaded.store_name, "متجر إيكوال");
    assert_eq!(loaded.printer, printer);
    assert_eq!(loaded.features, Some(features));
    assert_eq!(loaded.national_address, Some(national_address));
    assert_eq!(loaded.inventory_approval_threshold, Some(dec!(500.25)), "Decimal scale must round-trip exactly");
    assert_eq!(loaded.timezone.as_deref(), Some("Asia/Riyadh"));
    assert_eq!(loaded.default_branch_id, branch_id);

    // Singleton CHECK: a second row must be rejected (both by the generated unique on `singleton`
    // and the `ck_settings_singleton` CHECK).
    let second = settings::ActiveModel {
        id: Set(Id::new()),
        singleton: Set(1),
        store_name: Set("متجر آخر".to_string()),
        logo: Set(None),
        stamp: Set(None),
        signature: Set(None),
        currency: Set("EGP".to_string()),
        country: Set(None),
        vat_number: Set(None),
        default_tax_id: Set(None),
        invoice_number_prefix: Set("INV".to_string()),
        printer: Set(printer),
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
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: sea_orm::ActiveValue::NotSet,
    };
    let err = second.insert(conn).await.expect_err("a second settings row must be rejected");
    assert!(is_duplicate_key_error(&err), "expected a duplicate-key error on the singleton unique, got: {err}");
}

// --- DocDate: both shapes round-trip (stock_adjustments.date) ---------------------------------------

#[tokio::test]
async fn doc_date_round_trips_both_shapes_on_stock_adjustments() {
    use accounting_app_lib::entities::inventory::stock_adjustments;

    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let day_only_id = Id::new();
    let day = chrono::NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
    let am_day = stock_adjustments::ActiveModel {
        id: Set(day_only_id),
        number: Set("ADJ-0001".to_string()),
        r#type: Set("STOCK_IN".to_string()),
        date_day: Set(day),
        date_instant: Set(None),
        date_key: sea_orm::ActiveValue::NotSet,
        status: Set("COMPLETED".to_string()),
        note: Set(None),
        reason: Set(Some("opening".to_string())),
        offset_account_id: Set(None),
        approved_by: Set(None),
        approved_at: Set(None),
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: sea_orm::ActiveValue::NotSet,
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    am_day.insert(conn).await.unwrap();

    let instant_id = Id::new();
    let instant = Utc.with_ymd_and_hms(2026, 9, 27, 10, 0, 0).unwrap() + chrono::Duration::milliseconds(120);
    let am_instant = stock_adjustments::ActiveModel {
        id: Set(instant_id),
        number: Set("ADJ-0002".to_string()),
        r#type: Set("STOCK_IN".to_string()),
        date_day: Set(day),
        date_instant: Set(Some(instant)),
        date_key: sea_orm::ActiveValue::NotSet,
        status: Set("COMPLETED".to_string()),
        note: Set(None),
        reason: Set(Some("opening".to_string())),
        offset_account_id: Set(None),
        approved_by: Set(None),
        approved_at: Set(None),
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: sea_orm::ActiveValue::NotSet,
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    am_instant.insert(conn).await.unwrap();

    let loaded_day = stock_adjustments::Entity::find_by_id(day_only_id).one(conn).await.unwrap().unwrap();
    assert_eq!(loaded_day.date_key, "2026-09-27");
    assert_eq!(loaded_day.date().key(), "2026-09-27");

    let loaded_instant = stock_adjustments::Entity::find_by_id(instant_id).one(conn).await.unwrap().unwrap();
    assert_eq!(loaded_instant.date_key, "2026-09-27T10:00:00.120Z");
    assert_eq!(loaded_instant.date().key(), "2026-09-27T10:00:00.120Z");
}

// --- taxes: soft-delete round-trip (no live-name unique on taxes, just the soft_delete mechanism) --

#[tokio::test]
async fn taxes_soft_delete_and_restore_round_trip() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let id = Id::new();
    let am = taxes::ActiveModel {
        id: Set(id),
        name: Set("ضريبة القيمة المضافة".to_string()),
        rate: Set(dec!(15.0000)),
        r#type: Set("OUTPUT".to_string()),
        is_default: Set(true),
        active: Set(true),
        category: Set("S".to_string()),
        direction: Set("sales".to_string()),
        exemption_reason: Set(None),
        account_role: Set(Some("vatOutput".to_string())),
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: sea_orm::ActiveValue::NotSet,
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    am.insert(conn).await.unwrap();

    taxes::Entity::soft_delete(conn, id, Utc::now()).await.unwrap();
    let live = taxes::Entity::find_live().all(conn).await.unwrap();
    assert!(live.is_empty());

    taxes::Entity::restore(conn, id).await.unwrap();
    let live_after_restore = taxes::Entity::find_live().all(conn).await.unwrap();
    assert_eq!(live_after_restore.len(), 1);
    assert_eq!(live_after_restore[0].rate, dec!(15.0000));
}

// --- Migrator up/down/up sanity (part of the phase-b gate) ------------------------------------------

#[tokio::test]
async fn migrator_up_down_up_succeeds() {
    // `TestDb::fresh()` already runs `Migrator::up` once; this test additionally proves `down` then
    // `up` again succeeds from that same fresh state.
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    use migration::MigratorTrait;
    migration::Migrator::down(conn, None).await.expect("migrator down must succeed");
    migration::Migrator::up(conn, None).await.expect("migrator up (again) must succeed");
}
