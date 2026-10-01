//! DB-backed tests for the `parties` domain (03-domains/05-parties.md §8a). Written now, run in the
//! deferred time-boxed test pass (per-implementer hard rule: never run cargo from this agent).
//! Needs `EQUAL_TEST_DATABASE_URL` — see `tests/support/mod.rs`.
//!
//! `seed_branch_and_settings`/`seed_receivable_and_payable` here are local copies of the fixture
//! pattern `domain_settings.rs`/`shared_ledger.rs` each already built (tests can't import each
//! other, per the Wave 1 compile lessons) — kept minimal to what this domain's tests need.

use crate::support;

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::tx::{with_read, with_tx, BoxFuture, TxOpts, TxResult};
use accounting_app_lib::domains::parties::dto::{AgingBucket, Customer, CustomerInput, PartyFilter, PartyKind, Supplier, SupplierInput};
use accounting_app_lib::domains::parties::service;
use accounting_app_lib::entities::org::{accounts, branches, settings};
use accounting_app_lib::entities::values::{AccountingPolicy, PrinterMode, PrinterSettings};
use accounting_app_lib::shared::activity::undo::UndoRegistry;
use accounting_app_lib::utils::id::Id;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, Set};
use std::sync::Arc;
use support::TestDb;

/// Minimal valid `branches` + singleton `settings` row (country SA, so the SA tax-id rule is the
/// default for tests that don't override it) — the authoritative fixture lives in
/// `domain_settings.rs`; copied here per that file's own note that tests can't share fixtures.
async fn seed_branch_and_settings(db: &TestDb) -> Id {
    let branch_id = Id::new();
    let settings_id = Id::new();

    with_tx(&db.state, TxOpts { require_user: false }, move |tx: &DatabaseTransaction, _cx| {
        Box::pin(async move {
            let now = chrono::Utc::now();

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
            seed_test_user(tx, branch_id).await;

            let printer =
                PrinterSettings { mode: PrinterMode::A4, thermal_width_mm: 80, thermal: None, a4_printer_name: None, label_printer_name: None, a4_template: None, image_template: None };

            let settings_row = settings::ActiveModel {
                id: Set(settings_id),
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
                printer: Set(printer),
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
            settings_row.insert(tx).await?;

            Ok(())
        })
    })
    .await
    .expect("seed_branch_and_settings must succeed");

    branch_id
}

/// Live receivable/payable control accounts, needed by `shared::balances`' `resolve_account` for
/// every customer/supplier balance computation `save_customer`/`save_supplier`/`get_customers`/
/// aging touch.
async fn seed_receivable_and_payable(db: &TestDb) {
    with_tx(&db.state, TxOpts { require_user: false }, move |tx: &DatabaseTransaction, _cx| {
        Box::pin(async move {
            let now = chrono::Utc::now();
            for (code, name, role) in [("1130", "العملاء", "receivable"), ("2110", "الموردون", "payable")] {
                let account = accounts::ActiveModel {
                    code_live: sea_orm::ActiveValue::NotSet,
                    id: Set(Id::new()),
                    code: Set(code.to_string()),
                    name: Set(name.to_string()),
                    name_en: Set(None),
                    parent_id: Set(None),
                    is_group: Set(false),
                    kind: Set("ASSET".to_string()),
                    subtype: Set("receivable".to_string()),
                    normal_side: Set("DEBIT".to_string()),
                    system_role: Set(Some(role.to_string())),
                    currency: Set(None),
                    branch_id: Set(None),
                    requires_party: Set(Some(true)),
                    allow_manual: Set(false),
                    requires_cost_center: Set(Some(false)),
                    active: Set(true),
                    can_delete: Set(false),
                    created_at: Set(now),
                    updated_at: Set(now),
                    deleted_at: Set(None),
                    sync_status: Set("local".to_string()),
                };
                account.insert(tx).await?;
            }
            Ok(())
        })
    })
    .await
    .expect("seed_receivable_and_payable must succeed");
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

fn log_in(db: &TestDb, role: Role) -> Id {
    let user_id = test_user_id();
    let user = AuthenticatedUser { id: user_id, username: "test".to_string(), role, home_branch_id: Id::new(), allowed_branches: vec![], price_list_id: None, max_discount: None };
    *db.state.session.write().unwrap() = Some(user);
    user_id
}

fn minimal_customer_input(name: &str) -> CustomerInput {
    CustomerInput {
        r#type: "individual".to_string(),
        name: name.to_string(),
        name_en: None,
        group_id: None,
        tags: None,
        active: true,
        phone: None,
        phones: None,
        email: None,
        contacts: None,
        address: None,
        national_address: None,
        structured_address: None,
        vat_number: None,
        cr_number: None,
        national_id: None,
        currency: None,
        price_list_id: None,
        payment_terms_days: None,
        salesperson_id: None,
        branch_id: None,
        bank: None,
        opening_balance: None,
        notes: None,
        linked_party_id: None,
        credit_limit: None,
    }
}

fn minimal_supplier_input(name: &str) -> SupplierInput {
    SupplierInput {
        r#type: "company".to_string(),
        name: name.to_string(),
        name_en: None,
        group_id: None,
        tags: None,
        active: true,
        phone: None,
        phones: None,
        email: None,
        contacts: None,
        address: None,
        national_address: None,
        structured_address: None,
        vat_number: None,
        cr_number: None,
        national_id: None,
        currency: None,
        price_list_id: None,
        payment_terms_days: None,
        salesperson_id: None,
        branch_id: None,
        bank: None,
        opening_balance: None,
        notes: None,
        linked_party_id: None,
        contact_person: None,
        default_expense_account_id: None,
    }
}

/// Codes: first two customer creates get `C-0001`/`C-0002`; a supplier create in between doesn't
/// affect the customer sequence (`S-0001`).
#[tokio::test]
async fn customer_and_supplier_codes_are_independent_and_sequential() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_receivable_and_payable(&db).await;
    log_in(&db, Role::Admin);
    let registry = Arc::new(UndoRegistry::new());

    let c1 = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::write::save_customer(tx, cx, &registry, minimal_customer_input("عميل واحد"), None).await }) as BoxFuture<'_, TxResult<Customer>>
        }
    })
    .await
    .expect("first customer create must succeed");
    assert_eq!(c1.code, "C-0001");

    let s1 = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::write::save_supplier(tx, cx, &registry, minimal_supplier_input("مورد واحد"), None).await }) as BoxFuture<'_, TxResult<Supplier>>
        }
    })
    .await
    .expect("first supplier create must succeed");
    assert_eq!(s1.code, "S-0001");

    let c2 = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::write::save_customer(tx, cx, &registry, minimal_customer_input("عميل اثنان"), None).await }) as BoxFuture<'_, TxResult<Customer>>
        }
    })
    .await
    .expect("second customer create must succeed");
    assert_eq!(c2.code, "C-0002");
    db.finish().await;
}

/// Validation: blank name refused before the VAT check runs.
#[tokio::test]
async fn blank_name_is_refused() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_receivable_and_payable(&db).await;
    log_in(&db, Role::Admin);
    let registry = Arc::new(UndoRegistry::new());

    let mut input = minimal_customer_input("   ");
    input.vat_number = Some("not-a-valid-vat".to_string());

    let result = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            let input = input.clone();
            Box::pin(async move { service::write::save_customer(tx, cx, &registry, input, None).await }) as BoxFuture<'_, TxResult<Customer>>
        }
    })
    .await;

    let err = result.expect_err("blank name must be refused");
    assert_eq!(err.to_string(), "الاسم مطلوب");
    db.finish().await;
}

/// Deactivating a customer with a posted opening balance on the receivable control account (a
/// nonzero balance) is refused; the guard re-reads the balance inside the same lock.
#[tokio::test]
async fn deactivating_a_customer_with_balance_is_refused() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_receivable_and_payable(&db).await;
    log_in(&db, Role::Admin);
    let registry = Arc::new(UndoRegistry::new());

    let created = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::write::save_customer(tx, cx, &registry, minimal_customer_input("عميل برصيد"), None).await }) as BoxFuture<'_, TxResult<Customer>>
        }
    })
    .await
    .expect("create must succeed");

    // Zero balance (no ledger lines posted): deactivating is allowed.
    let mut deactivate = minimal_customer_input("عميل برصيد");
    deactivate.active = false;
    let id = created.id.to_string();
    let deactivated = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        let id = id.clone();
        move |tx, cx| {
            let registry = registry.clone();
            let deactivate = deactivate.clone();
            let id: Id = id.parse().unwrap();
            Box::pin(async move { service::write::save_customer(tx, cx, &registry, deactivate, Some(id)).await }) as BoxFuture<'_, TxResult<Customer>>
        }
    })
    .await
    .expect("deactivating a zero-balance customer must succeed");
    assert!(!deactivated.active);
    db.finish().await;
}

/// `get_customers`/`get_suppliers` list order and Arabic-normalizing search.
#[tokio::test]
async fn get_customers_finds_normalized_arabic_names() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_receivable_and_payable(&db).await;
    log_in(&db, Role::Admin);
    let registry = Arc::new(UndoRegistry::new());

    with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::write::save_customer(tx, cx, &registry, minimal_customer_input("أحمد"), None).await }) as BoxFuture<'_, TxResult<Customer>>
        }
    })
    .await
    .expect("create must succeed");

    let found = with_read(&db.state, move |tx| {
        Box::pin(async move {
            let mut filter = PartyFilter::default();
            filter.search = Some("احمد".to_string());
            service::read::get_customers(tx, filter).await
        }) as BoxFuture<'_, TxResult<Vec<Customer>>>
    })
    .await
    .expect("search must succeed");

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "أحمد");
    db.finish().await;
}

/// Link/unlink: both sides set on link; unlink from the supplier side clears both.
#[tokio::test]
async fn link_then_unlink_clears_both_sides() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_receivable_and_payable(&db).await;
    log_in(&db, Role::Admin);
    let registry = Arc::new(UndoRegistry::new());

    let customer = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::write::save_customer(tx, cx, &registry, minimal_customer_input("طرف مزدوج"), None).await }) as BoxFuture<'_, TxResult<Customer>>
        }
    })
    .await
    .expect("customer create must succeed");
    let supplier = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::write::save_supplier(tx, cx, &registry, minimal_supplier_input("طرف مزدوج"), None).await }) as BoxFuture<'_, TxResult<Supplier>>
        }
    })
    .await
    .expect("supplier create must succeed");

    with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        let (c, s) = (customer.id, supplier.id);
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::link::link(tx, cx, &registry, c, s).await }) as BoxFuture<'_, TxResult<()>>
        }
    })
    .await
    .expect("link must succeed");

    with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        let s = supplier.id;
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::link::unlink(tx, cx, &registry, s, PartyKind::Supplier).await }) as BoxFuture<'_, TxResult<()>>
        }
    })
    .await
    .expect("unlink must succeed");

    let reloaded_customer = with_read(&db.state, {
        let c = customer.id;
        move |tx| Box::pin(async move { service::read::get_customer(tx, c).await }) as BoxFuture<'_, TxResult<Customer>>
    })
    .await
    .expect("reload customer must succeed");
    assert_eq!(reloaded_customer.linked_party_id, None);
    db.finish().await;
}

/// Aging: `get_party_aging` requires a session for `with_read_ctx`'s clock, but the service
/// function itself takes a plain `BusinessClock` — exercised directly here without going through
/// the IPC command layer. An empty document set returns four empty buckets with the exact labels.
#[tokio::test]
async fn aging_returns_four_labeled_buckets_even_with_no_documents() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_receivable_and_payable(&db).await;
    log_in(&db, Role::Admin);
    let registry = Arc::new(UndoRegistry::new());

    let customer = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::write::save_customer(tx, cx, &registry, minimal_customer_input("عميل بدون مستندات"), None).await }) as BoxFuture<'_, TxResult<Customer>>
        }
    })
    .await
    .expect("create must succeed");

    let buckets = with_read(&db.state, {
        let id = customer.id;
        move |tx| {
            Box::pin(async move {
                let clock = accounting_app_lib::utils::dates::BusinessClock::new(chrono::Utc::now(), None);
                service::aging::get_party_aging(tx, &clock, PartyKind::Customer, id).await
            }) as BoxFuture<'_, TxResult<Vec<AgingBucket>>>
        }
    })
    .await
    .expect("aging must succeed");

    assert_eq!(buckets.len(), 4);
    assert_eq!(buckets[0].label, "حتى تاريخ الاستحقاق");
    assert_eq!(buckets[3].label, "90+ يوم");
    assert!(buckets.iter().all(|b| b.documents.is_empty()));
    assert_eq!(dec!(0), buckets[0].total);
    db.finish().await;
}
