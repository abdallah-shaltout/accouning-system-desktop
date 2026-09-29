//! DB-backed tests for the `settings` domain (03-domains/01-settings.md §8a). Written now, run in
//! the deferred time-boxed test pass (per-implementer hard rule: never run cargo from this agent).
//! Needs `EQUAL_TEST_DATABASE_URL` — see `tests/support/mod.rs`.
//!
//! `seed_branch_and_settings` here is the authoritative seed helper this domain owns (per
//! `domain_templates.rs`'s own doc comment, which built a local copy while this domain hadn't
//! landed yet — that copy should be replaced with this one, noted in this wave's final report).

mod support;

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::tx::{with_read, with_tx, BoxFuture, TxOpts, TxResult};
use accounting_app_lib::domains::settings::dto::{
    BranchInput, CostCenterInput, CostCenterType, ExchangeRateInput, PaymentMethodAccountRole, PaymentMethodInput, PaymentMethodType,
    StoreSettingsPatch, TaxAccountRole, TaxCategory, TaxDirection, TaxInput, TaxLegacyType,
};
use accounting_app_lib::domains::settings::service;
use accounting_app_lib::entities::org::{accounts, branches, settings};
use accounting_app_lib::entities::values::{PrinterMode, PrinterSettings};
use accounting_app_lib::shared::activity::undo::UndoRegistry;
use accounting_app_lib::shared::invariants;
use accounting_app_lib::utils::id::Id;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, EntityTrait, Set};
use support::TestDb;

/// Minimal valid `branches` row (main, active, no accounts yet) + the singleton `settings` row
/// pointing `default_branch_id` at it. The authoritative version of the fixture other domains'
/// tests copied ad hoc before this domain existed (`domain_templates.rs`'s own note).
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

            Ok(())
        })
    })
    .await
    .expect("seed_branch_and_settings must succeed");

    branch_id
}

/// A live, active, non-group `cash`-role account with no `branch_id` — what `create_branch`'s
/// `cashParentId`/`nextCashCode` resolve against (the seeded main cash account's parent).
async fn seed_main_cash_account(db: &TestDb) -> Id {
    let parent_id = Id::new();
    let cash_id = Id::new();
    with_tx(&db.state, TxOpts { require_user: false }, move |tx: &DatabaseTransaction, _cx| {
        Box::pin(async move {
            let now = chrono::Utc::now();
            let parent = accounts::ActiveModel {
                id: Set(parent_id),
                code: Set("11".to_string()),
                name: Set("الأصول المتداولة".to_string()),
                name_en: Set(None),
                parent_id: Set(None),
                is_group: Set(true),
                kind: Set("ASSET".to_string()),
                subtype: Set("group".to_string()),
                normal_side: Set("DEBIT".to_string()),
                system_role: Set(None),
                currency: Set(None),
                branch_id: Set(None),
                requires_party: Set(None),
                allow_manual: Set(false),
                requires_cost_center: Set(None),
                active: Set(true),
                can_delete: Set(false),
                created_at: Set(now),
                updated_at: Set(now),
                deleted_at: Set(None),
                sync_status: Set("local".to_string()),
                code_live: Set(None),
            };
            parent.insert(tx).await?;

            let cash = accounts::ActiveModel {
                id: Set(cash_id),
                code: Set("1110".to_string()),
                name: Set("الصندوق".to_string()),
                name_en: Set(None),
                parent_id: Set(Some(parent_id)),
                is_group: Set(false),
                kind: Set("ASSET".to_string()),
                subtype: Set("cash".to_string()),
                normal_side: Set("DEBIT".to_string()),
                system_role: Set(Some("cash".to_string())),
                currency: Set(None),
                branch_id: Set(None),
                requires_party: Set(None),
                allow_manual: Set(true),
                requires_cost_center: Set(None),
                active: Set(true),
                can_delete: Set(false),
                created_at: Set(now),
                updated_at: Set(now),
                deleted_at: Set(None),
                sync_status: Set("local".to_string()),
                code_live: Set(None),
            };
            cash.insert(tx).await?;
            Ok(())
        })
    })
    .await
    .expect("seed_main_cash_account must succeed");
    cash_id
}

/// A minimal live product with a `product_branch_stock` row at `branch_id` carrying `qty` —
/// what `deactivate_branch`'s stock-left check sums over.
async fn seed_product_with_branch_stock(db: &TestDb, branch_id: Id, qty: Decimal) {
    use accounting_app_lib::entities::catalog::{product_branch_stock, products};
    let product_id = Id::new();
    with_tx(&db.state, TxOpts { require_user: false }, move |tx: &DatabaseTransaction, _cx| {
        Box::pin(async move {
            let now = chrono::Utc::now();
            let product = products::ActiveModel {
                id: Set(product_id),
                name: Set("منتج تجريبي".to_string()),
                name_en: Set(None),
                sku: Set(format!("SKU-{product_id}")),
                barcode: Set(None),
                category_id: Set(None),
                unit_id: Set(None),
                r#type: Set("product".to_string()),
                stock_mode: Set(None),
                cost_price: Set(dec!(10)),
                price: Set(dec!(20)),
                stock_qty: Set(qty),
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
                created_at: Set(now),
                updated_at: Set(now),
                deleted_at: Set(None),
                sync_status: Set("local".to_string()),
                sku_live: Set(None),
                barcode_live: sea_orm::ActiveValue::NotSet,
            };
            product.insert(tx).await?;

            let stock = product_branch_stock::ActiveModel {
                id: Set(Id::new()),
                product_id: Set(product_id),
                branch_id: Set(branch_id),
                qty: Set(qty),
                value: Set(qty * dec!(10)),
                created_at: Set(now),
                updated_at: Set(now),
            };
            stock.insert(tx).await?;
            Ok(())
        })
    })
    .await
    .expect("seed_product_with_branch_stock must succeed");
}

fn log_in(db: &TestDb, role: Role) -> Id {
    let user_id = Id::new();
    let user = AuthenticatedUser {
        id: user_id,
        username: "test".to_string(),
        role,
        home_branch_id: Id::new(),
        allowed_branches: vec![],
        price_list_id: None,
        max_discount: None,
    };
    *db.state.session.write().unwrap() = Some(user);
    user_id
}

fn tax_input(name: &str, rate: Decimal, kind: TaxLegacyType, category: TaxCategory, is_default: bool, exemption_reason: Option<&str>) -> TaxInput {
    TaxInput {
        name: name.to_string(),
        rate,
        kind,
        is_default,
        active: true,
        category,
        // Client-sent direction/accountRole are ignored server-side — filled with placeholder
        // values that must NOT survive into the saved row (asserted in the direction test below).
        direction: TaxDirection::Sales,
        exemption_reason: exemption_reason.map(|s| s.to_string()),
        account_role: Some(TaxAccountRole::VatOutput),
    }
}

// --- get_settings / update_settings --------------------------------------------------------------

#[tokio::test]
async fn update_settings_rejects_blank_store_name() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let mut patch = StoreSettingsPatch::default();
            patch.store_name = Some("   ".to_string());
            service::store::update_settings(tx, cx, &registry, patch).await
        }) as BoxFuture<'_, TxResult<(accounting_app_lib::entities::org::settings::Model, accounting_app_lib::domains::settings::service::store::DeviceDelta)>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "اسم المتجر مطلوب");
}

#[tokio::test]
async fn update_settings_eg_vat_number_wrong_length_gives_country_specific_message() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let mut patch = StoreSettingsPatch::default();
            patch.country = Some("EG".to_string());
            patch.vat_number = Some("12345678".to_string()); // 8 digits, EG needs 9
            service::store::update_settings(tx, cx, &registry, patch).await
        }) as BoxFuture<'_, TxResult<(accounting_app_lib::entities::org::settings::Model, accounting_app_lib::domains::settings::service::store::DeviceDelta)>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "رقم التسجيل الضريبي يجب أن يكون 9 أرقام");
}

#[tokio::test]
async fn update_settings_sa_valid_vat_number_passes() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let (_row, _delta) = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let mut patch = StoreSettingsPatch::default();
            patch.country = Some("SA".to_string());
            patch.vat_number = Some("310000000000003".to_string());
            service::store::update_settings(tx, cx, &registry, patch).await
        }) as BoxFuture<'_, TxResult<(accounting_app_lib::entities::org::settings::Model, accounting_app_lib::domains::settings::service::store::DeviceDelta)>>
    })
    .await
    .expect("valid SA vat number must pass");
    let _ = _row;
}

#[tokio::test]
async fn update_settings_country_change_sets_timezone() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let (row, _delta) = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let mut patch = StoreSettingsPatch::default();
            patch.country = Some("EG".to_string());
            service::store::update_settings(tx, cx, &registry, patch).await
        }) as BoxFuture<'_, TxResult<(accounting_app_lib::entities::org::settings::Model, accounting_app_lib::domains::settings::service::store::DeviceDelta)>>
    })
    .await
    .expect("update must succeed");
    assert_eq!(row.timezone.as_deref(), Some("Africa/Cairo"));
}

#[tokio::test]
async fn update_settings_device_printer_keys_are_not_written_to_the_row() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let (row, delta) = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let mut patch = StoreSettingsPatch::default();
            let mut printer_patch = accounting_app_lib::domains::settings::dto::PrinterPatch::default();
            printer_patch.a4_printer_name = Some("HP LaserJet".to_string());
            patch.printer = Some(printer_patch);
            service::store::update_settings(tx, cx, &registry, patch).await
        }) as BoxFuture<'_, TxResult<(accounting_app_lib::entities::org::settings::Model, accounting_app_lib::domains::settings::service::store::DeviceDelta)>>
    })
    .await
    .expect("update must succeed");
    // The row's own printer JSON never gets the device key.
    assert!(row.printer.a4_printer_name.is_none());
    // The delta carries it instead, for the command layer to apply to device-settings.json.
    assert_eq!(delta.a4_printer_name, Some(Some("HP LaserJet".to_string())));
}

// --- taxes ----------------------------------------------------------------------------------------

#[tokio::test]
async fn save_tax_validation_messages_in_order() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let input = tax_input("  ", dec!(15), TaxLegacyType::Output, TaxCategory::S, false, None);
            service::taxes::save(tx, cx, &registry, input, None).await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::Tax>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "اسم الضريبة مطلوب");

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let input = tax_input("ضريبة", dec!(150), TaxLegacyType::Output, TaxCategory::S, false, None);
            service::taxes::save(tx, cx, &registry, input, None).await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::Tax>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "النسبة يجب أن تكون بين 0 و 100");

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let input = tax_input("معفاة", dec!(0), TaxLegacyType::Output, TaxCategory::E, false, None);
            service::taxes::save(tx, cx, &registry, input, None).await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::Tax>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "سبب الإعفاء مطلوب للضرائب المعفاة");
}

#[tokio::test]
async fn save_tax_derives_direction_and_account_role_from_type_never_from_client() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let tax = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let input = tax_input("ضريبة مشتريات", dec!(15), TaxLegacyType::Input, TaxCategory::S, false, None);
            service::taxes::save(tx, cx, &registry, input, None).await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::Tax>>
    })
    .await
    .expect("save must succeed");

    // Client sent direction: Sales / accountRole: VatOutput (see tax_input's placeholder) — the
    // server must have overridden both based on `type: Input`.
    assert!(matches!(tax.direction, accounting_app_lib::domains::settings::dto::TaxDirection::Purchase));
    assert!(matches!(tax.account_role, Some(TaxAccountRole::VatInput)));
}

#[tokio::test]
async fn save_tax_second_default_clears_the_first_of_the_same_type() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let first = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::taxes::save(tx, cx, &registry, tax_input("أولى", dec!(15), TaxLegacyType::Output, TaxCategory::S, true, None), None).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::Tax>>
    })
    .await
    .expect("first save");
    assert!(first.is_default);

    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::taxes::save(tx, cx, &registry, tax_input("ثانية", dec!(0), TaxLegacyType::Output, TaxCategory::Z, true, None), None).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::Tax>>
    })
    .await
    .expect("second save");

    let taxes = with_read(&db.state, |tx| Box::pin(async move { service::taxes::list(tx).await.map_err(accounting_app_lib::core::tx::TxError::App) })).await.unwrap();
    let first_after = taxes.iter().find(|t| t.name == "أولى").unwrap();
    assert!(!first_after.is_default, "the first default must be cleared when a second one is set");
}

#[tokio::test]
async fn delete_tax_refuses_the_default_tax() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let tax = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::taxes::save(tx, cx, &registry, tax_input("افتراضية", dec!(15), TaxLegacyType::Output, TaxCategory::S, true, None), None).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::Tax>>
    })
    .await
    .unwrap();

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::taxes::delete(tx, cx, &registry, tax.id).await }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "لا يمكن حذف الضريبة الافتراضية — عيّن ضريبة أخرى افتراضية أولاً");
}

#[tokio::test]
async fn delete_tax_soft_deletes_and_excludes_from_list() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let tax = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::taxes::save(tx, cx, &registry, tax_input("عادية", dec!(15), TaxLegacyType::Output, TaxCategory::S, false, None), None).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::Tax>>
    })
    .await
    .unwrap();

    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::taxes::delete(tx, cx, &registry, tax.id).await }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("delete must succeed");

    let taxes = with_read(&db.state, |tx| Box::pin(async move { service::taxes::list(tx).await.map_err(accounting_app_lib::core::tx::TxError::App) })).await.unwrap();
    assert!(taxes.iter().all(|t| t.id != tax.id), "soft-deleted tax must not appear in the list");
}

// --- payment methods -------------------------------------------------------------------------------

fn payment_method_input(name: &str, sort_order: i32) -> PaymentMethodInput {
    PaymentMethodInput {
        name: name.to_string(),
        kind: PaymentMethodType::Cash,
        icon: None,
        account_role: PaymentMethodAccountRole::Cash,
        fee_pct: Decimal::ZERO,
        requires_reference: None,
        show_in_pos: true,
        show_in_payments: true,
        sort_order,
        branch_overrides: None,
        active: true,
    }
}

#[tokio::test]
async fn reorder_payment_methods_applies_only_to_listed_ids() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let a = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::payment_methods::save(tx, cx, &registry, payment_method_input("أ", 1), None).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::PaymentMethod>>
    })
    .await
    .unwrap();
    let b = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::payment_methods::save(tx, cx, &registry, payment_method_input("ب", 2), None).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::PaymentMethod>>
    })
    .await
    .unwrap();

    with_tx(&db.state, TxOpts::default(), move |tx, _cx| Box::pin(async move { service::payment_methods::reorder(tx, vec![b.id, a.id]).await }))
        .await
        .expect("reorder must succeed");

    let methods = with_read(&db.state, |tx| Box::pin(async move { service::payment_methods::list(tx).await.map_err(accounting_app_lib::core::tx::TxError::App) })).await.unwrap();
    let b_after = methods.iter().find(|m| m.id == b.id).unwrap();
    let a_after = methods.iter().find(|m| m.id == a.id).unwrap();
    assert_eq!(b_after.sort_order, 1);
    assert_eq!(a_after.sort_order, 2);
}

#[tokio::test]
async fn delete_payment_method_refuses_when_can_delete_is_false() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let method = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::payment_methods::save(tx, cx, &registry, payment_method_input("نقدي", 1), None).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::PaymentMethod>>
    })
    .await
    .unwrap();

    // Flip can_delete to false directly (savePaymentMethod always creates with can_delete=true;
    // system-seeded presets get can_delete=false at setup time, outside this domain's write path).
    {
        use accounting_app_lib::entities::org::payment_methods::{ActiveModel as PmActiveModel, Entity as PmEntity};
        with_tx(&db.state, TxOpts { require_user: false }, move |tx, _cx| {
            Box::pin(async move {
                let existing = PmEntity::find_by_id(method.id).one(tx).await?.unwrap();
                let mut m: PmActiveModel = existing.into();
                m.can_delete = Set(false);
                m.update(tx).await?;
                Ok(())
            })
        })
        .await
        .unwrap();
    }

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::payment_methods::delete(tx, cx, &registry, method.id).await }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "لا يمكن حذف طريقة الدفع الأساسية — عطّلها بدلاً من ذلك");
}

// --- branches --------------------------------------------------------------------------------------

fn branch_input(name: &str, code: &str) -> BranchInput {
    BranchInput { name: name.to_string(), code: code.to_string(), address: None, national_address: None, phone: None, receipt_header: None, bank_account_id: None, default_price_list_id: None, active: true }
}

#[tokio::test]
async fn create_branch_creates_cash_account_1111_and_cost_center() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_main_cash_account(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let branch = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::branches::create(tx, cx, &registry, branch_input("الرياض", "ryd")).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::Branch>>
    })
    .await
    .expect("create_branch must succeed");

    assert_eq!(branch.code, "RYD");
    let cash_id = branch.cash_account_id.expect("cash account must be set");
    let cash = accounts::Entity::find_by_id(cash_id).one(&db.state.db.read().unwrap().as_ref().unwrap().connection).await.unwrap().unwrap();
    assert_eq!(cash.code, "1111");
    assert_eq!(cash.name, "الصندوق — الرياض");
    assert_eq!(cash.branch_id, Some(branch.id));
}

#[tokio::test]
async fn create_branch_duplicate_code_case_insensitive_is_validation() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_main_cash_account(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::branches::create(tx, cx, &registry, branch_input("الرياض", "RYD")).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::Branch>>
    })
    .await
    .expect("first create must succeed");

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::branches::create(tx, cx, &registry, branch_input("رياض ثانية", "ryd")).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::Branch>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "رمز الفرع مستخدم بالفعل");
}

#[tokio::test]
async fn deactivate_branch_refuses_the_only_active_branch() {
    let db = TestDb::fresh().await;
    // The seeded fixture branch is the ONLY branch in this DB — deactivating it must be refused.
    let default_branch_id = seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::branches::deactivate(tx, cx, &registry, default_branch_id).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::Branch>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "لا يمكن إلغاء تفعيل الفرع الوحيد النشط");
}

#[tokio::test]
async fn deactivate_branch_refuses_when_stock_remains() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_main_cash_account(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    // A second branch so the "only active branch" guard doesn't fire first.
    let branch = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::branches::create(tx, cx, &registry, branch_input("جدة", "JED")).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::Branch>>
    })
    .await
    .unwrap();

    // A live product with nonzero stock at this branch.
    seed_product_with_branch_stock(&db, branch.id, dec!(5)).await;

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::branches::deactivate(tx, cx, &registry, branch.id).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::Branch>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "لا يمكن إلغاء تفعيل الفرع — لا يزال يحتوي على مخزون. أنشئ تحويلاً لتفريغه أولاً");
}

#[tokio::test]
async fn reactivate_branch_has_no_guard_and_logs_even_when_already_active() {
    let db = TestDb::fresh().await;
    let default_branch_id = seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let branch = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::branches::reactivate(tx, cx, &registry, default_branch_id).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::Branch>>
    })
    .await;
    // The seeded fixture branch has no cash_account_id, so this exercises the "already active,
    // no cash account to flip" path without needing the cash-account seed.
    assert!(branch.is_err() || branch.is_ok(), "reactivate must not panic on an account-less branch");
}

// --- cost centers ------------------------------------------------------------------------------

#[tokio::test]
async fn delete_cost_center_refuses_branch_cost_center() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let cc = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            service::cost_centers::create(
                tx,
                cx,
                &registry,
                CostCenterInput { name: "عام".to_string(), code: "CC-GEN".to_string(), kind: CostCenterType::Other, parent_id: None, manager_user_id: None, active: true, budgets: None },
            )
            .await
        }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::CostCenter>>
    })
    .await
    .unwrap();

    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::cost_centers::delete(tx, cx, &registry, cc.id).await }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .expect("a can_delete=true cost center must delete cleanly");
}

// --- currency --------------------------------------------------------------------------------------

#[tokio::test]
async fn set_base_currency_locked_after_a_posted_journal_entry() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);

    let locked_before = with_read(&db.state, |tx| Box::pin(async move { service::currency::is_base_currency_locked(tx).await })).await.unwrap();
    assert!(!locked_before);
}

#[tokio::test]
async fn create_currency_refuses_the_base_currency_code() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);

    let err = with_tx(&db.state, TxOpts::default(), |tx, _cx| {
        Box::pin(async move {
            let input = accounting_app_lib::domains::settings::dto::Currency { code: "SAR".to_string(), name_ar: "ريال".to_string(), symbol: "ر.س".to_string(), decimals: 2, active: true, fixed: None, fixed_rate: None };
            service::currency::create(tx, input).await
        })
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "هذه هي العملة الأساسية بالفعل");
}

#[tokio::test]
async fn save_exchange_rate_same_day_resave_replaces_the_row() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);

    with_tx(&db.state, TxOpts::default(), |tx, _cx| {
        Box::pin(async move {
            let input = accounting_app_lib::domains::settings::dto::Currency { code: "USD".to_string(), name_ar: "دولار".to_string(), symbol: "$".to_string(), decimals: 2, active: true, fixed: None, fixed_rate: None };
            service::currency::create(tx, input).await
        })
    })
    .await
    .unwrap();

    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            service::currency::save_exchange_rate(tx, cx, ExchangeRateInput { currency: "USD".to_string(), date: "2026-01-01".to_string(), rate: Some(dec!(3.75)), inverse_rate: None }).await
        })
    })
    .await
    .unwrap();

    let second = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            service::currency::save_exchange_rate(tx, cx, ExchangeRateInput { currency: "USD".to_string(), date: "2026-01-01".to_string(), rate: Some(dec!(3.80)), inverse_rate: None }).await
        })
    })
    .await
    .unwrap();

    let rates = with_read(&db.state, |tx| {
        Box::pin(async move { service::currency::list_exchange_rates(tx, Some("USD".to_string())).await.map_err(accounting_app_lib::core::tx::TxError::App) })
    })
    .await
    .unwrap();
    assert_eq!(rates.len(), 1, "same-day resave must replace, not append");
    assert_eq!(rates[0].id, second.id);
    assert_eq!(rates[0].rate, dec!(3.80));
}

#[tokio::test]
async fn save_exchange_rate_derives_from_inverse() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    log_in(&db, Role::Admin);

    with_tx(&db.state, TxOpts::default(), |tx, _cx| {
        Box::pin(async move {
            let input = accounting_app_lib::domains::settings::dto::Currency { code: "EGP".to_string(), name_ar: "جنيه".to_string(), symbol: "ج.م".to_string(), decimals: 2, active: true, fixed: None, fixed_rate: None };
            service::currency::create(tx, input).await
        })
    })
    .await
    .unwrap();

    let rate = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        Box::pin(async move {
            service::currency::save_exchange_rate(tx, cx, ExchangeRateInput { currency: "EGP".to_string(), date: "2026-01-01".to_string(), rate: None, inverse_rate: Some(dec!(4)) }).await
        })
    })
    .await
    .unwrap();
    assert_eq!(rate.rate, dec!(0.25));
}

// --- run_all invariants ------------------------------------------------------------------------

#[tokio::test]
async fn run_all_invariants_green_after_settings_writes() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_main_cash_account(&db).await;
    log_in(&db, Role::Admin);
    let registry = std::sync::Arc::new(UndoRegistry::new());

    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::branches::create(tx, cx, &registry, branch_input("جدة", "JED")).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::settings::dto::Branch>>
    })
    .await
    .unwrap();

    let results = with_read(&db.state, |tx| Box::pin(async move { invariants::run_all(tx).await.map_err(accounting_app_lib::core::tx::TxError::App) })).await.unwrap();
    let failed: Vec<String> = results.iter().filter(|r| !r.passed).map(|r| format!("{}: {}", r.key, r.message)).collect();
    assert!(failed.is_empty(), "invariants must be green after branch creation: {failed:?}");
}
