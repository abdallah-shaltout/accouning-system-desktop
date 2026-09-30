//! `shared::currency` DB-backed tests (phase-c-ledger.md "Tests"). Needs
//! `EQUAL_TEST_DATABASE_URL` (see `tests/support/mod.rs`) — never skipped.

use crate::support;

use accounting_app_lib::entities::org::currencies::ActiveModel as CurrencyActiveModel;
use accounting_app_lib::entities::org::exchange_rates::ActiveModel as RateActiveModel;
use accounting_app_lib::entities::org::settings::ActiveModel as SettingsActiveModel;
use accounting_app_lib::entities::values::{PrinterMode, PrinterSettings};
use accounting_app_lib::shared::currency::{base_currency, is_base_currency, latest_rate, require_rate};
use accounting_app_lib::utils::id::Id;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use support::TestDb;

async fn seed_settings<C: ConnectionTrait>(conn: &C, base: &str) {
    let default_branch_id = support::seed_settings_parents(conn, base).await;
    let settings = SettingsActiveModel {
        id: Set(Id::new()),
        singleton: Set(1),
        store_name: Set("متجر تجريبي".to_string()),
        logo: Set(None),
        stamp: Set(None),
        signature: Set(None),
        currency: Set(base.to_string()),
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
        default_branch_id: Set(default_branch_id),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    };
    settings.insert(conn).await.unwrap();
}

#[tokio::test]
async fn base_currency_and_is_base_currency() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    seed_settings(conn, "SAR").await;

    assert_eq!(base_currency(conn).await.unwrap(), "SAR");
    assert!(is_base_currency(conn, None).await.unwrap());
    assert!(is_base_currency(conn, Some("SAR")).await.unwrap());
    assert!(!is_base_currency(conn, Some("USD")).await.unwrap());
}

#[tokio::test]
async fn fixed_rate_wins_over_the_rate_table() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    seed_settings(conn, "SAR").await;

    let currency = CurrencyActiveModel {
        code: Set("USD".to_string()),
        name_ar: Set("دولار أمريكي".to_string()),
        symbol: Set("$".to_string()),
        decimals: Set(2),
        active: Set(true),
        fixed: Set(Some(true)),
        fixed_rate: Set(Some(dec!(3.75))),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    };
    currency.insert(conn).await.unwrap();

    // Even with a rate-table row present, the fixed rate wins.
    let rate_row = RateActiveModel {
        id: Set(Id::new()),
        currency: Set("USD".to_string()),
        date: Set(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
        rate: Set(dec!(3.80)),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    };
    rate_row.insert(conn).await.unwrap();

    let rate = latest_rate(conn, "USD", chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()).await.unwrap();
    assert_eq!(rate, Some(dec!(3.75)));
}

#[tokio::test]
async fn latest_rate_respects_the_cutoff_date() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    seed_settings(conn, "SAR").await;

    let currency = CurrencyActiveModel {
        code: Set("EUR".to_string()),
        name_ar: Set("يورو".to_string()),
        symbol: Set("€".to_string()),
        decimals: Set(2),
        active: Set(true),
        fixed: Set(Some(false)),
        fixed_rate: Set(None),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    };
    currency.insert(conn).await.unwrap();

    for (date, rate) in [("2026-01-01", dec!(4.0)), ("2026-03-01", dec!(4.2)), ("2026-09-01", dec!(4.5))] {
        let row = RateActiveModel {
            id: Set(Id::new()),
            currency: Set("EUR".to_string()),
            date: Set(chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap()),
            rate: Set(rate),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };
        row.insert(conn).await.unwrap();
    }

    let cutoff = chrono::NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
    let rate = latest_rate(conn, "EUR", cutoff).await.unwrap();
    assert_eq!(rate, Some(dec!(4.2)), "must pick the latest rate on or before the cutoff, not after it");

    let before_any = chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let rate_before_any = latest_rate(conn, "EUR", before_any).await.unwrap();
    assert_eq!(rate_before_any, None);
}

#[tokio::test]
async fn require_rate_message_when_missing() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    seed_settings(conn, "SAR").await;

    let currency = CurrencyActiveModel {
        code: Set("GBP".to_string()),
        name_ar: Set("جنيه إسترليني".to_string()),
        symbol: Set("£".to_string()),
        decimals: Set(2),
        active: Set(true),
        fixed: Set(Some(false)),
        fixed_rate: Set(None),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    };
    currency.insert(conn).await.unwrap();

    let err = require_rate(conn, "GBP", chrono::Utc::now().date_naive()).await.unwrap_err();
    assert_eq!(err.to_string(), "لا يوجد سعر صرف لعملة GBP");
}
