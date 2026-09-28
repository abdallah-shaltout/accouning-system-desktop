//! Phase B (owner B1) DB-backed tests for B-6/B-7: `search_normalized` is filled/refreshed by
//! `ActiveModelBehavior::before_save` on exactly the 4 tables named in P2-38 (`products`,
//! `parties`, `journal_entries`, `vouchers`). Needs `EQUAL_TEST_DATABASE_URL` (see
//! `tests/support/mod.rs`) — never skipped when absent, panics with a clear message instead.

mod support;

use accounting_app_lib::entities::catalog::products;
use accounting_app_lib::entities::journal::journal_entries;
use accounting_app_lib::entities::parties::parties;
use accounting_app_lib::entities::payments::vouchers;
use accounting_app_lib::utils::id::Id;
use accounting_app_lib::utils::text::normalize_arabic;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, EntityTrait, Statement};
use support::TestDb;

/// Seeds one minimal `users` row (raw SQL, same pattern as `db_entities_foundation.rs`'s
/// `seed_minimal_branch` — this file's job is testing `before_save`, not exercising the full
/// setup-wizard flow) for `journal_entries.created_by` / `vouchers.created_by`.
async fn seed_minimal_user<C: ConnectionTrait>(conn: &C) -> Id {
    let user_id = Id::new();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO users (id, username, name, role, max_discount, active) VALUES (?, ?, ?, ?, 0, 1)",
        [user_id.to_string().into(), "admin".into(), "المدير".into(), "admin".into()],
    );
    conn.execute(stmt).await.unwrap();
    user_id
}

fn haystack_of(fields: &[&str]) -> String {
    let opts: Vec<Option<&str>> = fields.iter().map(|s| Some(*s)).collect();
    accounting_app_lib::utils::text::search_haystack(&opts)
}

// --- products ----------------------------------------------------------------------------------

#[tokio::test]
async fn products_search_normalized_filled_on_insert_and_refreshed_on_update() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let id = Id::new();
    let am = products::ActiveModel {
        barcode_live: sea_orm::ActiveValue::NotSet,
        id: Set(id),
        name: Set("أحمد للمواد الغذائية".to_string()),
        name_en: Set(None),
        sku: Set("SKU-001".to_string()),
        barcode: Set(Some("1234567890".to_string())),
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
        // Deliberately NOT set — `before_save` must compute it, not require the caller to.
        search_normalized: sea_orm::ActiveValue::NotSet,
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: sea_orm::ActiveValue::NotSet,
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
        sku_live: sea_orm::ActiveValue::NotSet,
    };
    am.insert(conn).await.unwrap();

    let loaded = products::Entity::find_by_id(id).one(conn).await.unwrap().unwrap();
    assert_eq!(loaded.search_normalized.as_deref(), Some(haystack_of(&["أحمد للمواد الغذائية", "SKU-001", "1234567890"]).as_str()));
    // Arabic hamza-variant normalization applied (أحمد -> احمد).
    assert!(loaded.search_normalized.as_ref().unwrap().contains(&normalize_arabic(Some("احمد"))));

    // Partial update: only `sku` is `Set`; `name`/`barcode` are read back from the row's own state
    // (the strict "recompute from full row state" choice, not "only when every field is present").
    let update = products::ActiveModel { id: Set(id), sku: Set("SKU-002".to_string()), ..Default::default() };
    update.update(conn).await.unwrap();

    let reloaded = products::Entity::find_by_id(id).one(conn).await.unwrap().unwrap();
    assert_eq!(reloaded.sku, "SKU-002");
    assert_eq!(reloaded.search_normalized.as_deref(), Some(haystack_of(&["أحمد للمواد الغذائية", "SKU-002", "1234567890"]).as_str()));
}

// --- parties -------------------------------------------------------------------------------------

#[tokio::test]
async fn parties_search_normalized_covers_the_customer_and_supplier_field_union() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    // Supplier row: exercises `contact_person`, which customers never set.
    let id = Id::new();
    let am = parties::ActiveModel {
        id: Set(id),
        kind: Set("supplier".to_string()),
        r#type: Set("company".to_string()),
        name: Set("مؤسسة النور".to_string()),
        name_en: Set(Some("Al Noor Est.".to_string())),
        code: Set("SUP-001".to_string()),
        group_id: Set(None),
        tags: Set(None),
        active: Set(true),
        phone: Set(Some("0501234567".to_string())),
        email: Set(None),
        contacts: Set(None),
        address: Set(None),
        national_address: Set(None),
        structured_address: Set(None),
        vat_number: Set(Some("300000000000003".to_string())),
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
        contact_person: Set(Some("خالد أحمد".to_string())),
        default_expense_account_id: Set(None),
        search_normalized: sea_orm::ActiveValue::NotSet,
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: sea_orm::ActiveValue::NotSet,
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    am.insert(conn).await.unwrap();

    let loaded = parties::Entity::find_by_id(id).one(conn).await.unwrap().unwrap();
    let expected = haystack_of(&["مؤسسة النور", "Al Noor Est.", "SUP-001", "0501234567", "300000000000003", "خالد أحمد"]);
    assert_eq!(loaded.search_normalized.as_deref(), Some(expected.as_str()));

    // Partial update: only `phone` changes; the rest must still come from the row's own state.
    let update = parties::ActiveModel { id: Set(id), phone: Set(Some("0559999999".to_string())), ..Default::default() };
    update.update(conn).await.unwrap();

    let reloaded = parties::Entity::find_by_id(id).one(conn).await.unwrap().unwrap();
    let expected_after = haystack_of(&["مؤسسة النور", "Al Noor Est.", "SUP-001", "0559999999", "300000000000003", "خالد أحمد"]);
    assert_eq!(reloaded.search_normalized.as_deref(), Some(expected_after.as_str()));
}

// --- journal_entries -----------------------------------------------------------------------------

#[tokio::test]
async fn journal_entries_search_normalized_filled_on_insert_and_refreshed_on_update() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let user_id = seed_minimal_user(conn).await;
    let id = Id::new();
    let am = journal_entries::ActiveModel {
        id: Set(id),
        number: Set("JE-0001".to_string()),
        date_day: Set(chrono::NaiveDate::from_ymd_opt(2026, 9, 27).unwrap()),
        date_instant: Set(None),
        description: Set("قيد افتتاحي للصندوق".to_string()),
        r#type: Set(journal_entries::JournalEntryType::Manual),
        status: Set(journal_entries::JournalEntryStatus::Posted),
        source_kind: Set(None),
        source_id: Set(None),
        source_number: Set(Some("INV-0099".to_string())),
        total_debit: Set(dec!(0)),
        total_credit: Set(dec!(0)),
        reversed: Set(false),
        reversal_of_id: Set(None),
        reversal_reason: Set(None),
        created_by: Set(user_id),
        posted_by: Set(None),
        posted_at_day: Set(None),
        posted_at_instant: Set(None),
        attachment_ids: Set(None),
        template_id: Set(None),
        search_normalized: sea_orm::ActiveValue::NotSet,
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: sea_orm::ActiveValue::NotSet,
        deleted_at: Set(None),
        sync_status: Set(journal_entries::SyncStatus::Local),
    };
    am.insert(conn).await.unwrap();

    let loaded = journal_entries::Entity::find_by_id(id).one(conn).await.unwrap().unwrap();
    let expected = haystack_of(&["JE-0001", "قيد افتتاحي للصندوق", "INV-0099"]);
    assert_eq!(loaded.search_normalized.as_deref(), Some(expected.as_str()));

    // Partial update: only `description` changes.
    let update = journal_entries::ActiveModel { id: Set(id), description: Set("قيد تعديل الصندوق".to_string()), ..Default::default() };
    update.update(conn).await.unwrap();

    let reloaded = journal_entries::Entity::find_by_id(id).one(conn).await.unwrap().unwrap();
    let expected_after = haystack_of(&["JE-0001", "قيد تعديل الصندوق", "INV-0099"]);
    assert_eq!(reloaded.search_normalized.as_deref(), Some(expected_after.as_str()));
}

// --- vouchers ------------------------------------------------------------------------------------

#[tokio::test]
async fn vouchers_search_normalized_filled_on_insert_and_refreshed_on_update() {
    let test_db = TestDb::fresh().await;
    let db_guard = test_db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;

    let user_id = seed_minimal_user(conn).await;
    let id = Id::new();
    let am = vouchers::ActiveModel {
        id: Set(id),
        number: Set("RV-0001".to_string()),
        kind: Set(vouchers::VoucherKind::Receipt),
        date_day: Set(chrono::NaiveDate::from_ymd_opt(2026, 9, 27).unwrap()),
        date_instant: Set(None),
        amount: Set(dec!(500)),
        description: Set("سند قبض من العميل".to_string()),
        note: Set(Some("دفعة أولى".to_string())),
        attachment_ids: Set(None),
        cost_center_id: Set(None),
        created_by: Set(user_id),
        payment_method_id: Set(None),
        credit_account_id: Set(None),
        debit_account_id: Set(None),
        source_account_id: Set(None),
        destination_account_id: Set(None),
        fee_amount: Set(None),
        fee_account_id: Set(None),
        direction: Set(None),
        cash_account_id: Set(None),
        search_normalized: sea_orm::ActiveValue::NotSet,
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: sea_orm::ActiveValue::NotSet,
        deleted_at: Set(None),
        sync_status: Set(vouchers::SyncStatus::Local),
    };
    am.insert(conn).await.unwrap();

    let loaded = vouchers::Entity::find_by_id(id).one(conn).await.unwrap().unwrap();
    let expected = haystack_of(&["RV-0001", "سند قبض من العميل", "دفعة أولى"]);
    assert_eq!(loaded.search_normalized.as_deref(), Some(expected.as_str()));

    // Partial update: only `note` changes.
    let update = vouchers::ActiveModel { id: Set(id), note: Set(Some("دفعة نهائية".to_string())), ..Default::default() };
    update.update(conn).await.unwrap();

    let reloaded = vouchers::Entity::find_by_id(id).one(conn).await.unwrap().unwrap();
    let expected_after = haystack_of(&["RV-0001", "سند قبض من العميل", "دفعة نهائية"]);
    assert_eq!(reloaded.search_normalized.as_deref(), Some(expected_after.as_str()));
}
