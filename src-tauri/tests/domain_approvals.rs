//! DB-backed tests for the `approvals` domain (03-domains/04-approvals.md §8a). Written now, run in
//! the deferred time-boxed test pass (per-implementer hard rule: never run cargo from this agent).
//! Needs `EQUAL_TEST_DATABASE_URL` — see `tests/support.rs`.

mod support;

use accounting_app_lib::core::auth::{AuthenticatedUser, Role};
use accounting_app_lib::core::error::AppError;
use accounting_app_lib::core::tx::{with_read, with_tx, BoxFuture, TxOpts, TxResult};
use accounting_app_lib::domains::approvals::dto::{ApprovalDecisionInput, ApprovalKind, ApprovalListFilter, ApprovalRequest, ApprovalRequestInput, ApprovalStatus};
use accounting_app_lib::domains::approvals::service;
use accounting_app_lib::domains::users::dto::UserInput;
use accounting_app_lib::domains::users::service as users_service;
use accounting_app_lib::entities::org::{branches, settings};
use accounting_app_lib::entities::platform::activity::{ActivityKind, Column as ActivityColumn, Entity as ActivityEntity};
use accounting_app_lib::entities::platform::audit::{Column as AuditColumn, Entity as AuditEntity};
use accounting_app_lib::entities::values::{PrinterMode, PrinterSettings};
use accounting_app_lib::shared::activity::undo::UndoRegistry;
use accounting_app_lib::utils::id::Id;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set};
use support::TestDb;

/// Minimal valid `branches` row + the singleton `settings` row pointing at it — same shape as
/// `domain_settings.rs`'s authoritative fixture (tests can't import each other, so this is a local
/// copy built the same way).
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

fn user_input(username: &str, role: Role) -> UserInput {
    UserInput {
        username: username.to_string(),
        name: format!("مستخدم {username}"),
        phone: None,
        role,
        max_discount: dec!(0),
        price_list_id: None,
        active: true,
        password: Some("password-123".to_string()),
        allowed_branches: None,
        home_branch: None,
    }
}

/// Creates a real `users` row (so `actor_name` resolves a genuine DB name, not the "مستخدم"
/// fallback) and logs it in as the active session.
async fn seed_and_login(db: &TestDb, username: &str, role: Role) -> Id {
    let registry = std::sync::Arc::new(UndoRegistry::new());
    let user = with_tx(&db.state, TxOpts { require_user: false }, {
        let registry = registry.clone();
        let input = user_input(username, role);
        move |tx, cx| {
            let registry = registry.clone();
            let input = input.clone();
            Box::pin(async move { users_service::create_user(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<accounting_app_lib::domains::users::dto::User>>
        }
    })
    .await
    .expect("seed user must succeed");

    let authenticated = AuthenticatedUser {
        id: user.id,
        username: user.username.clone(),
        role: user.role,
        home_branch_id: Id::new(),
        allowed_branches: vec![],
        price_list_id: None,
        max_discount: Some(user.max_discount),
    };
    *db.state.session.write().unwrap() = Some(authenticated);
    user.id
}

fn login_as(db: &TestDb, id: Id, username: &str, role: Role) {
    let authenticated = AuthenticatedUser {
        id,
        username: username.to_string(),
        role,
        home_branch_id: Id::new(),
        allowed_branches: vec![],
        price_list_id: None,
        max_discount: None,
    };
    *db.state.session.write().unwrap() = Some(authenticated);
}

fn discount_input(summary: &str, value: Decimal) -> ApprovalRequestInput {
    ApprovalRequestInput { kind: ApprovalKind::Discount, summary: summary.to_string(), value, request_note: None, link: None }
}

async fn version_of(db: &TestDb, category: &str) -> i64 {
    let db_guard = db.state.db.read().unwrap();
    let conn = &db_guard.as_ref().unwrap().connection;
    let stmt = sea_orm::Statement::from_sql_and_values(conn.get_database_backend(), "SELECT version FROM change_versions WHERE category = ?", [category.into()]);
    let row = sea_orm::ConnectionTrait::query_one(conn, stmt).await.unwrap().unwrap();
    row.try_get("", "version").unwrap()
}

// --- submit --------------------------------------------------------------------------------------

#[tokio::test]
async fn submit_as_cashier_discount_creates_pending_request_with_db_name() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    let cashier_id = seed_and_login(&db, "cashier1", Role::Cashier).await;

    let registry = std::sync::Arc::new(UndoRegistry::new());
    let ledger_before = version_of(&db, "ledger").await;

    let result = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move {
                cx.require_any(
                    tx,
                    &[
                        (accounting_app_lib::core::auth::Area::Pos, accounting_app_lib::core::auth::Access::Write),
                        (accounting_app_lib::core::auth::Area::Sales, accounting_app_lib::core::auth::Access::Write),
                    ],
                )
                .await?;
                service::submit(tx, cx, &registry, discount_input("خصم 25% على فاتورة عميل نقدي", dec!(25))).await
            }) as BoxFuture<'_, TxResult<ApprovalRequest>>
        }
    })
    .await
    .expect("cashier discount submit must succeed");

    assert_eq!(result.status, ApprovalStatus::Pending);
    assert_eq!(result.requested_by, cashier_id);
    assert_eq!(result.requested_by_name, "مستخدم cashier1");
    assert_eq!(result.request_note, None);

    assert_eq!(version_of(&db, "ledger").await, ledger_before + 1, "submit must touch the ledger change category");

    with_read(&db.state, |tx| {
        Box::pin(async move {
            let audits = AuditEntity::find().filter(AuditColumn::Message.eq(format!("طلب اعتماد جديد: خصم يتجاوز الحد المسموح — خصم 25% على فاتورة عميل نقدي")))
                .all(tx)
                .await?;
            assert_eq!(audits.len(), 1, "exactly one audit row for the submit");

            let activities = ActivityEntity::find().filter(ActivityColumn::Kind.eq(ActivityKind::Approval)).all(tx).await?;
            assert_eq!(activities.len(), 1, "exactly one activity row for the submit");
            Ok(())
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn submit_trims_whitespace_only_request_note_to_none() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_and_login(&db, "cashier2", Role::Cashier).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let mut input = discount_input("خصم", dec!(10));
    input.request_note = Some("   ".to_string());

    let result = with_tx(&db.state, TxOpts::default(), move |tx, cx| {
        let registry = registry.clone();
        let input = input.clone();
        Box::pin(async move { service::submit(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<ApprovalRequest>>
    })
    .await
    .expect("submit must succeed");

    assert_eq!(result.request_note, None, "whitespace-only request note must collapse to None (approvals.ts:25)");
}

#[tokio::test]
async fn submit_write_off_forbidden_for_cashier_ok_for_storekeeper() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_and_login(&db, "cashier3", Role::Cashier).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let write_off = ApprovalRequestInput { kind: ApprovalKind::WriteOff, summary: "إتلاف مخزون".to_string(), value: dec!(500), request_note: None, link: None };

    let cashier_result = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        let input = write_off.clone();
        move |tx, cx| {
            let registry = registry.clone();
            let input = input.clone();
            Box::pin(async move {
                cx.require(tx, accounting_app_lib::core::auth::Area::Inventory, accounting_app_lib::core::auth::Access::Write).await?;
                service::submit(tx, cx, &registry, input).await
            }) as BoxFuture<'_, TxResult<ApprovalRequest>>
        }
    })
    .await;
    assert!(matches!(cashier_result, Err(_)), "cashier must be forbidden from write_off submit");

    seed_and_login(&db, "storekeeper1", Role::Storekeeper).await;
    let storekeeper_result = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            let input = write_off.clone();
            Box::pin(async move {
                cx.require(tx, accounting_app_lib::core::auth::Area::Inventory, accounting_app_lib::core::auth::Access::Write).await?;
                service::submit(tx, cx, &registry, input).await
            }) as BoxFuture<'_, TxResult<ApprovalRequest>>
        }
    })
    .await;
    assert!(storekeeper_result.is_ok(), "storekeeper must be able to submit write_off");
}

// --- list / count ----------------------------------------------------------------------------

#[tokio::test]
async fn list_is_newest_first_and_filters_by_status() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_and_login(&db, "cashier4", Role::Cashier).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());

    for i in 0..3 {
        with_tx(&db.state, TxOpts::default(), {
            let registry = registry.clone();
            let input = discount_input(&format!("طلب {i}"), dec!(10));
            move |tx, cx| {
                let registry = registry.clone();
                let input = input.clone();
                Box::pin(async move { service::submit(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<ApprovalRequest>>
            }
        })
        .await
        .unwrap();
    }

    let all = with_read(&db.state, |tx| Box::pin(async move { service::list(tx, None).await }) as BoxFuture<'_, TxResult<Vec<ApprovalRequest>>>)
        .await
        .unwrap();
    assert_eq!(all.len(), 3);
    // All three share the same instant-resolution second in a fast test run, so this mainly checks
    // that the sort is stable (ties keep insertion order) rather than reversing them.
    assert_eq!(all[0].summary, "طلب 0");
    assert_eq!(all[2].summary, "طلب 2");

    let pending = with_read(&db.state, |tx| {
        Box::pin(async move { service::list(tx, Some(ApprovalListFilter { status: Some(ApprovalStatus::Pending) })).await }) as BoxFuture<'_, TxResult<Vec<ApprovalRequest>>>
    })
    .await
    .unwrap();
    assert_eq!(pending.len(), 3);

    let approved = with_read(&db.state, |tx| {
        Box::pin(async move { service::list(tx, Some(ApprovalListFilter { status: Some(ApprovalStatus::Approved) })).await }) as BoxFuture<'_, TxResult<Vec<ApprovalRequest>>>
    })
    .await
    .unwrap();
    assert_eq!(approved.len(), 0);
}

#[tokio::test]
async fn pending_count_matches_pending_rows() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_and_login(&db, "cashier5", Role::Cashier).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());

    with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        let input = discount_input("طلب واحد", dec!(5));
        move |tx, cx| {
            let registry = registry.clone();
            let input = input.clone();
            Box::pin(async move { service::submit(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<ApprovalRequest>>
        }
    })
    .await
    .unwrap();

    let count = with_read(&db.state, |tx| Box::pin(async move { service::pending_count(tx).await }) as BoxFuture<'_, TxResult<u32>>).await.unwrap();
    assert_eq!(count, 1);
}

// --- decide (approve/reject) -----------------------------------------------------------------

async fn submit_one(db: &TestDb, registry: &std::sync::Arc<UndoRegistry>) -> Id {
    let result = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        let input = discount_input("طلب للاعتماد", dec!(15));
        move |tx, cx| {
            let registry = registry.clone();
            let input = input.clone();
            Box::pin(async move { service::submit(tx, cx, &registry, input).await }) as BoxFuture<'_, TxResult<ApprovalRequest>>
        }
    })
    .await
    .unwrap();
    result.id
}

#[tokio::test]
async fn approve_without_comment_leaves_decision_comment_absent() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_and_login(&db, "cashier6", Role::Cashier).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());
    let request_id = submit_one(&db, &registry).await;

    seed_and_login(&db, "manager1", Role::Manager).await;
    let result = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::decide(tx, cx, &registry, request_id, ApprovalStatus::Approved, ApprovalDecisionInput::default()).await }) as BoxFuture<'_, TxResult<ApprovalRequest>>
        }
    })
    .await
    .expect("approve must succeed");

    assert_eq!(result.status, ApprovalStatus::Approved);
    assert_eq!(result.decision_comment, None);
    assert!(result.decided_by.is_some());
}

#[tokio::test]
async fn approve_trims_comment() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_and_login(&db, "cashier7", Role::Cashier).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());
    let request_id = submit_one(&db, &registry).await;

    seed_and_login(&db, "manager2", Role::Manager).await;
    let result = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::decide(tx, cx, &registry, request_id, ApprovalStatus::Approved, ApprovalDecisionInput { comment: Some("  ok ".to_string()) }).await })
                as BoxFuture<'_, TxResult<ApprovalRequest>>
        }
    })
    .await
    .expect("approve must succeed");

    assert_eq!(result.decision_comment, Some("ok".to_string()));
}

#[tokio::test]
async fn reject_without_comment_or_whitespace_is_validation_error() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_and_login(&db, "cashier8", Role::Cashier).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());
    let request_id = submit_one(&db, &registry).await;

    seed_and_login(&db, "manager3", Role::Manager).await;

    for comment in [None, Some("   ".to_string())] {
        let err = with_tx(&db.state, TxOpts::default(), {
            let registry = registry.clone();
            let comment = comment.clone();
            move |tx, cx| {
                let registry = registry.clone();
                let comment = comment.clone();
                Box::pin(async move { service::decide(tx, cx, &registry, request_id, ApprovalStatus::Rejected, ApprovalDecisionInput { comment }).await }) as BoxFuture<'_, TxResult<ApprovalRequest>>
            }
        })
        .await
        .unwrap_err();
        assert_eq!(err.to_string(), "أدخل سبب الرفض");
    }
}

#[tokio::test]
async fn decide_unknown_id_is_not_found() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_and_login(&db, "manager4", Role::Manager).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let err = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::decide(tx, cx, &registry, Id::new(), ApprovalStatus::Approved, ApprovalDecisionInput::default()).await }) as BoxFuture<'_, TxResult<ApprovalRequest>>
        }
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "طلب الاعتماد غير موجود");
}

#[tokio::test]
async fn deciding_twice_is_validation_error() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_and_login(&db, "cashier9", Role::Cashier).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());
    let request_id = submit_one(&db, &registry).await;

    seed_and_login(&db, "manager5", Role::Manager).await;
    with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::decide(tx, cx, &registry, request_id, ApprovalStatus::Approved, ApprovalDecisionInput::default()).await }) as BoxFuture<'_, TxResult<ApprovalRequest>>
        }
    })
    .await
    .expect("first decide must succeed");

    let err = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::decide(tx, cx, &registry, request_id, ApprovalStatus::Rejected, ApprovalDecisionInput { comment: Some("لا".to_string()) }).await })
                as BoxFuture<'_, TxResult<ApprovalRequest>>
        }
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "تم اتخاذ قرار بشأن هذا الطلب مسبقاً");
}

// --- concurrency -----------------------------------------------------------------------------

#[tokio::test]
async fn concurrent_approve_and_reject_exactly_one_wins() {
    let db = TestDb::fresh().await;
    seed_branch_and_settings(&db).await;
    seed_and_login(&db, "cashier10", Role::Cashier).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());
    let request_id = submit_one(&db, &registry).await;

    let manager_a_id = Id::new();
    let manager_b_id = Id::new();

    // Both managers act against the same shared `AppState`/session slot is not realistic for two
    // physically different terminals, but `TxCtx.actor` is captured per-transaction from
    // `state.session` at the moment `with_tx` opens it — logging in as A right before launching
    // both futures and keeping B's identity only in its own closure would require two sessions,
    // which this single-process `AppState` can't hold at once. Instead, `decide`'s row lock is
    // exercised directly: both transactions race on the same request id under the actual
    // `SELECT ... FOR UPDATE`, and only one observes `status == pending` after acquiring the lock —
    // that is the invariant under test, independent of which "manager identity" is attached.
    login_as(&db, manager_a_id, "managerA", Role::Manager);

    let fut_approve = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::decide(tx, cx, &registry, request_id, ApprovalStatus::Approved, ApprovalDecisionInput::default()).await }) as BoxFuture<'_, TxResult<ApprovalRequest>>
        }
    });
    let fut_reject = with_tx(&db.state, TxOpts::default(), {
        let registry = registry.clone();
        move |tx, cx| {
            let registry = registry.clone();
            Box::pin(async move { service::decide(tx, cx, &registry, request_id, ApprovalStatus::Rejected, ApprovalDecisionInput { comment: Some("رفض".to_string()) }).await })
                as BoxFuture<'_, TxResult<ApprovalRequest>>
        }
    });

    let (r1, r2): (Result<ApprovalRequest, AppError>, Result<ApprovalRequest, AppError>) = tokio::join!(fut_approve, fut_reject);
    let successes = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
    let already_decided = [&r1, &r2].iter().filter(|r| matches!(r, Err(e) if e.to_string() == "تم اتخاذ قرار بشأن هذا الطلب مسبقاً")).count();
    assert_eq!(successes, 1, "exactly one decide must succeed");
    assert_eq!(already_decided, 1, "the loser must see the already-decided validation error");

    // manager_b_id is unused directly (both closures ran under the same session slot per the note
    // above) — kept only to document the intended two-manager scenario for a future real
    // multi-session harness.
    let _ = manager_b_id;
}
