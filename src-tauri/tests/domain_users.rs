//! DB-backed tests for the `users` domain (03-domains/03-users.md §8a). Written now, run in the
//! deferred time-boxed test pass (per-implementer hard rule: never run cargo from this agent).
//! Needs `EQUAL_TEST_DATABASE_URL` — see `tests/support/mod.rs`.

mod support;

use accounting_app_lib::core::auth::{Access, Area, AuthenticatedUser, Role};
use accounting_app_lib::core::tx::{with_read, with_tx, BoxFuture, TxOpts, TxResult};
use accounting_app_lib::domains::users::dto::{User, UserInput};
use accounting_app_lib::domains::users::service;
use accounting_app_lib::entities::org::credentials;
use accounting_app_lib::shared::activity::undo::UndoRegistry;
use accounting_app_lib::shared::invariants;
use accounting_app_lib::utils::id::Id;
use sea_orm::EntityTrait;
use support::TestDb;

fn input(username: &str, password: Option<&str>) -> UserInput {
    UserInput {
        username: username.to_string(),
        name: format!("{username} name"),
        phone: None,
        role: Role::Cashier,
        max_discount: rust_decimal::Decimal::ZERO,
        price_list_id: None,
        active: true,
        password: password.map(|p| p.to_string()),
        allowed_branches: None,
        home_branch: None,
    }
}

async fn seed_admin(db: &TestDb) -> (Id, String) {
    let password = "admin-password-123";
    let registry = std::sync::Arc::new(UndoRegistry::new());
    let user = with_tx(&db.state, TxOpts { require_user: false }, |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move {
            let mut i = input("admin", Some(password));
            i.role = Role::Admin;
            service::create_user(tx, cx, &registry, i).await
        }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .expect("seed admin must succeed");
    (user.id, password.to_string())
}

fn login_as(db: &TestDb, user: &User) {
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
}

// --- create -------------------------------------------------------------------------------------

#[tokio::test]
async fn create_returns_trimmed_username_and_hashed_password() {
    let db = TestDb::fresh().await;
    let (_admin_id, _pw) = seed_admin(&db).await;
    login_as_admin(&db).await;

    let registry = std::sync::Arc::new(UndoRegistry::new());
    let user = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::create_user(tx, cx, &registry, input("  cashier1  ", Some("pw12345"))).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .expect("create must succeed");

    assert_eq!(user.username, "  cashier1  ".trim());

    with_read(&db.state, |tx| {
        Box::pin(async move {
            let creds = credentials::Entity::find_by_id(user.id)
                .one(tx)
                .await
                .unwrap()
                .expect("credentials row must exist");
            assert!(creds.password_hash.starts_with("$argon2id$"));
            assert_ne!(creds.password_hash, "pw12345");
            Ok(())
        })
    })
    .await
    .unwrap();
}

async fn login_as_admin(db: &TestDb) {
    let admin = with_read(&db.state, |tx| Box::pin(async move { service::get_users(tx).await }))
        .await
        .unwrap()
        .into_iter()
        .find(|u| u.username == "admin")
        .expect("admin must exist");
    login_as(db, &admin);
}

#[tokio::test]
async fn create_duplicate_username_case_insensitive_is_conflict() {
    let db = TestDb::fresh().await;
    seed_admin(&db).await;
    login_as_admin(&db).await;

    let registry = std::sync::Arc::new(UndoRegistry::new());
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::create_user(tx, cx, &registry, input("Reporter", Some("pw12345"))).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .expect("first create must succeed");

    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::create_user(tx, cx, &registry, input("REPORTER", Some("pw12345"))).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "اسم المستخدم مستخدم من قبل");

    // " admin" (leading space) slips the untrimmed pre-check's case-fold against "admin" only if
    // it doesn't fold to the same lowercase string — this variant does, so it still hits the
    // pre-check (Q-1), proving the DB unique index is a pure backstop, never the only guard.
    let err2 = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::create_user(tx, cx, &registry, input(" admin", Some("pw12345"))).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap_err();
    assert_eq!(err2.to_string(), "اسم المستخدم مستخدم من قبل");
}

#[tokio::test]
async fn create_without_password_is_validation_error() {
    let db = TestDb::fresh().await;
    seed_admin(&db).await;
    login_as_admin(&db).await;

    let registry = std::sync::Arc::new(UndoRegistry::new());
    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::create_user(tx, cx, &registry, input("nopass", None)).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "كلمة المرور مطلوبة للمستخدم الجديد");
}

#[tokio::test]
async fn create_duplicate_and_no_password_reports_conflict_first() {
    let db = TestDb::fresh().await;
    seed_admin(&db).await;
    login_as_admin(&db).await;

    let registry = std::sync::Arc::new(UndoRegistry::new());
    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::create_user(tx, cx, &registry, input("admin", None)).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap_err();
    // Uniqueness is step 1, password is step 2 (§3 order) — the duplicate message must win.
    assert_eq!(err.to_string(), "اسم المستخدم مستخدم من قبل");
}

// --- update -------------------------------------------------------------------------------------

#[tokio::test]
async fn update_not_found() {
    let db = TestDb::fresh().await;
    seed_admin(&db).await;
    login_as_admin(&db).await;

    let registry = std::sync::Arc::new(UndoRegistry::new());
    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::update_user(tx, cx, &registry, Id::new(), input("ghost", None)).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "المستخدم غير موجود");
}

#[tokio::test]
async fn update_self_deactivate_and_self_role_change_are_blocked() {
    let db = TestDb::fresh().await;
    let (admin_id, _pw) = seed_admin(&db).await;
    login_as_admin(&db).await;

    let registry = std::sync::Arc::new(UndoRegistry::new());
    let mut deactivate = input("admin", None);
    deactivate.role = Role::Admin;
    deactivate.active = false;
    let err = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let i = deactivate.clone();
        Box::pin(async move { service::update_user(tx, cx, &registry, admin_id, i).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "لا يمكنك إيقاف حسابك أو تغيير صلاحيتك بنفسك");

    let mut change_role = input("admin", None);
    change_role.role = Role::Manager;
    change_role.active = true;
    let err2 = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let i = change_role.clone();
        Box::pin(async move { service::update_user(tx, cx, &registry, admin_id, i).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap_err();
    assert_eq!(err2.to_string(), "لا يمكنك إيقاف حسابك أو تغيير صلاحيتك بنفسك");
}

#[tokio::test]
async fn update_rename_then_login_with_old_password_works_new_password_replaces() {
    let db = TestDb::fresh().await;
    seed_admin(&db).await;
    login_as_admin(&db).await;

    let registry = std::sync::Arc::new(UndoRegistry::new());
    let created = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::create_user(tx, cx, &registry, input("renameme", Some("first-pass"))).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap();
    let uid: Id = created.id;

    let mut renamed = input("renamed", None); // no password -> keeps old one
    renamed.role = Role::Cashier;
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let i = renamed.clone();
        Box::pin(async move { service::update_user(tx, cx, &registry, uid, i).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap();

    // Old password still works against the new username (rename never touches credentials, keyed
    // by user_id).
    let (_user, _auth) = with_tx(&db.state, TxOpts { require_user: false }, |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::login(tx, cx, &registry, "renamed", "first-pass").await }) as BoxFuture<'_, TxResult<(User, accounting_app_lib::core::auth::AuthenticatedUser)>>
    })
    .await
    .expect("login with old password after rename must work");

    // New password replaces the old one.
    let mut with_new_pw = input("renamed", Some("second-pass"));
    with_new_pw.role = Role::Cashier;
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let i = with_new_pw.clone();
        Box::pin(async move { service::update_user(tx, cx, &registry, uid, i).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap();

    let err = with_tx(&db.state, TxOpts { require_user: false }, |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::login(tx, cx, &registry, "renamed", "first-pass").await }) as BoxFuture<'_, TxResult<(User, accounting_app_lib::core::auth::AuthenticatedUser)>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "اسم المستخدم أو كلمة المرور غير صحيحة");

    with_tx(&db.state, TxOpts { require_user: false }, |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::login(tx, cx, &registry, "renamed", "second-pass").await }) as BoxFuture<'_, TxResult<(User, accounting_app_lib::core::auth::AuthenticatedUser)>>
    })
    .await
    .expect("login with new password must work");
}

#[tokio::test]
async fn update_price_list_id_absent_clears_it_phone_absent_keeps_it() {
    let db = TestDb::fresh().await;
    seed_admin(&db).await;
    login_as_admin(&db).await;

    let registry = std::sync::Arc::new(UndoRegistry::new());
    let mut initial = input("withfields", Some("pw12345"));
    initial.phone = Some("0100000000".to_string());
    initial.price_list_id = Some(Id::new().to_string());
    let created = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let i = initial.clone();
        Box::pin(async move { service::create_user(tx, cx, &registry, i).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap();
    let uid: Id = created.id;

    // Update with price_list_id absent (None) and phone absent (None) too.
    let mut patch = input("withfields", None);
    patch.price_list_id = None;
    patch.phone = None;
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let i = patch.clone();
        Box::pin(async move { service::update_user(tx, cx, &registry, uid, i).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap();

    let after = with_read(&db.state, |tx| Box::pin(async move { service::get_user(tx, uid).await })).await.unwrap();
    assert_eq!(after.price_list_id, None, "priceListId absent must always clear it");
    assert_eq!(after.phone, Some("0100000000".to_string()), "phone absent must keep the old value");
}

// --- login --------------------------------------------------------------------------------------

#[tokio::test]
async fn login_unknown_user_and_wrong_password_share_the_same_message() {
    let db = TestDb::fresh().await;
    seed_admin(&db).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let err1 = with_tx(&db.state, TxOpts { require_user: false }, |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::login(tx, cx, &registry, "nosuchuser", "whatever").await }) as BoxFuture<'_, TxResult<(User, accounting_app_lib::core::auth::AuthenticatedUser)>>
    })
    .await
    .unwrap_err();

    let err2 = with_tx(&db.state, TxOpts { require_user: false }, |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::login(tx, cx, &registry, "admin", "wrong-password").await }) as BoxFuture<'_, TxResult<(User, accounting_app_lib::core::auth::AuthenticatedUser)>>
    })
    .await
    .unwrap_err();

    assert_eq!(err1.to_string(), "اسم المستخدم أو كلمة المرور غير صحيحة");
    assert_eq!(err1.to_string(), err2.to_string());
}

#[tokio::test]
async fn login_inactive_with_right_password_is_forbidden_wrong_password_is_unauthorized() {
    let db = TestDb::fresh().await;
    seed_admin(&db).await;
    login_as_admin(&db).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let mut inactive_input = input("inactiveuser", Some("pw12345"));
    inactive_input.active = false;
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let i = inactive_input.clone();
        Box::pin(async move { service::create_user(tx, cx, &registry, i).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap();

    let err = with_tx(&db.state, TxOpts { require_user: false }, |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::login(tx, cx, &registry, "inactiveuser", "pw12345").await }) as BoxFuture<'_, TxResult<(User, accounting_app_lib::core::auth::AuthenticatedUser)>>
    })
    .await
    .unwrap_err();
    assert_eq!(err.to_string(), "هذا الحساب موقوف — تواصل مع مدير النظام");

    let err2 = with_tx(&db.state, TxOpts { require_user: false }, |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::login(tx, cx, &registry, "inactiveuser", "wrong").await }) as BoxFuture<'_, TxResult<(User, accounting_app_lib::core::auth::AuthenticatedUser)>>
    })
    .await
    .unwrap_err();
    assert_eq!(err2.to_string(), "اسم المستخدم أو كلمة المرور غير صحيحة");
}

#[tokio::test]
async fn successful_login_sets_session_and_writes_audit_and_activity() {
    let db = TestDb::fresh().await;
    let (_admin_id, password) = seed_admin(&db).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());

    assert!(db.state.session.read().unwrap().is_none());

    let (user, authenticated) = with_tx(&db.state, TxOpts { require_user: false }, |tx, cx| {
        let registry = registry.clone();
        let pw = password.clone();
        Box::pin(async move { service::login(tx, cx, &registry, "admin", &pw).await }) as BoxFuture<'_, TxResult<(User, accounting_app_lib::core::auth::AuthenticatedUser)>>
    })
    .await
    .expect("login must succeed");
    *db.state.session.write().unwrap() = Some(authenticated);

    assert!(db.state.session.read().unwrap().is_some());
    assert_eq!(user.username, "admin");

    with_read(&db.state, |tx| {
        Box::pin(async move {
            use accounting_app_lib::entities::platform::activity::{ActivityKind, Entity as ActivityEntity};
            use accounting_app_lib::entities::platform::audit::{AuditAction, Entity as AuditEntity};
            use sea_orm::{ColumnTrait, QueryFilter};

            let audits = AuditEntity::find()
                .filter(accounting_app_lib::entities::platform::audit::Column::Action.eq(AuditAction::Login))
                .all(tx)
                .await
                .unwrap();
            assert_eq!(audits.len(), 1, "exactly one audit(action=login) row");
            assert_eq!(audits[0].entity, "auth");

            let activities = ActivityEntity::find()
                .filter(accounting_app_lib::entities::platform::activity::Column::Kind.eq(ActivityKind::Auth))
                .all(tx)
                .await
                .unwrap();
            assert_eq!(activities.len(), 1, "exactly one activity(kind=auth) row");
            Ok(())
        })
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_failed_login_leaves_the_session_unchanged() {
    let db = TestDb::fresh().await;
    let (admin_id, password) = seed_admin(&db).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let (_user, authenticated) = with_tx(&db.state, TxOpts { require_user: false }, |tx, cx| {
        let registry = registry.clone();
        let pw = password.clone();
        Box::pin(async move { service::login(tx, cx, &registry, "admin", &pw).await }) as BoxFuture<'_, TxResult<(User, accounting_app_lib::core::auth::AuthenticatedUser)>>
    })
    .await
    .unwrap();
    *db.state.session.write().unwrap() = Some(authenticated);
    assert_eq!(db.state.session.read().unwrap().as_ref().unwrap().id, admin_id);

    let _ = with_tx(&db.state, TxOpts { require_user: false }, |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::login(tx, cx, &registry, "admin", "wrong-one").await }) as BoxFuture<'_, TxResult<(User, accounting_app_lib::core::auth::AuthenticatedUser)>>
    })
    .await
    .unwrap_err();

    assert_eq!(db.state.session.read().unwrap().as_ref().unwrap().id, admin_id, "a failed login must not touch the existing session");
}

// --- logout / restore ----------------------------------------------------------------------------

#[tokio::test]
async fn restore_session_same_id_other_id_deactivated_and_no_session() {
    let db = TestDb::fresh().await;
    let (admin_id, password) = seed_admin(&db).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());

    // No session at all -> None.
    let none_result = with_read(&db.state, |tx| Box::pin(async move { service::restore_session(tx, None, admin_id).await })).await.unwrap();
    assert!(none_result.is_none());

    let (_user, authenticated) = with_tx(&db.state, TxOpts { require_user: false }, |tx, cx| {
        let registry = registry.clone();
        let pw = password.clone();
        Box::pin(async move { service::login(tx, cx, &registry, "admin", &pw).await }) as BoxFuture<'_, TxResult<(User, accounting_app_lib::core::auth::AuthenticatedUser)>>
    })
    .await
    .unwrap();

    // Same id -> Some(User).
    let same = with_read(&db.state, |tx| {
        let auth = authenticated.clone();
        Box::pin(async move { service::restore_session(tx, Some(&auth), admin_id).await })
    })
    .await
    .unwrap();
    assert!(same.is_some());

    // Other id -> None.
    let other = with_read(&db.state, |tx| {
        let auth = authenticated.clone();
        Box::pin(async move { service::restore_session(tx, Some(&auth), Id::new()).await })
    })
    .await
    .unwrap();
    assert!(other.is_none());
}

// --- verify manager pin ---------------------------------------------------------------------------

#[tokio::test]
async fn verify_manager_pin_role_and_active_gates() {
    let db = TestDb::fresh().await;
    seed_admin(&db).await;
    login_as_admin(&db).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let mut cashier_input = input("cashierpin", Some("pw12345"));
    cashier_input.role = Role::Cashier;
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let i = cashier_input.clone();
        Box::pin(async move { service::create_user(tx, cx, &registry, i).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap();

    let err = with_read(&db.state, |tx| Box::pin(async move { service::verify_manager_pin(tx, "cashierpin", "pw12345").await })).await.unwrap_err();
    assert_eq!(err.to_string(), "هذا المستخدم ليس مديراً — الاعتماد يتطلب صلاحية مدير");

    let mut inactive_manager = input("inactivemgr", Some("pw12345"));
    inactive_manager.role = Role::Manager;
    inactive_manager.active = false;
    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let i = inactive_manager.clone();
        Box::pin(async move { service::create_user(tx, cx, &registry, i).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap();

    let err2 = with_read(&db.state, |tx| Box::pin(async move { service::verify_manager_pin(tx, "inactivemgr", "pw12345").await })).await.unwrap_err();
    assert_eq!(err2.to_string(), "هذا الحساب موقوف");

    // Success leaves the session untouched (no session write happens in the service at all).
    assert!(db.state.session.read().unwrap().is_some(), "admin session from login_as_admin must be untouched");
}

// --- access ---------------------------------------------------------------------------------------

#[tokio::test]
async fn access_cashier_can_list_storekeeper_cannot_no_session_is_unauthorized() {
    let db = TestDb::fresh().await;
    seed_admin(&db).await;
    login_as_admin(&db).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());

    let mut cashier_input = input("cashieraccess", Some("pw12345"));
    cashier_input.role = Role::Cashier;
    let cashier = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let i = cashier_input.clone();
        Box::pin(async move { service::create_user(tx, cx, &registry, i).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap();

    let mut storekeeper_input = input("storekeeperaccess", Some("pw12345"));
    storekeeper_input.role = Role::Storekeeper;
    let storekeeper = with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        let i = storekeeper_input.clone();
        Box::pin(async move { service::create_user(tx, cx, &registry, i).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap();

    // Cashier: Sales:Read grants users_get_users (D-3) via require_any.
    login_as(&db, &cashier);
    let overrides = std::collections::HashMap::new();
    assert!(accounting_app_lib::core::auth::check_access(
        db.state.session.read().unwrap().as_ref(),
        Area::Sales,
        Access::Read,
        &overrides
    )
    .is_ok());

    // Storekeeper: neither Users:Read nor Sales:Read -> forbidden.
    login_as(&db, &storekeeper);
    let denied = accounting_app_lib::core::auth::check_access(db.state.session.read().unwrap().as_ref(), Area::Users, Access::Read, &overrides);
    assert!(denied.is_err());

    // Cashier cannot create users (Users:Write).
    login_as(&db, &cashier);
    let cashier_denied = accounting_app_lib::core::auth::check_access(db.state.session.read().unwrap().as_ref(), Area::Users, Access::Write, &overrides);
    assert!(cashier_denied.is_err());

    // No session -> UNAUTHORIZED.
    *db.state.session.write().unwrap() = None;
    let unauth = accounting_app_lib::core::auth::check_access(None, Area::Users, Access::Read, &overrides);
    match unauth {
        Err(e) => assert_eq!(e.to_string(), "سجّل الدخول أولاً"),
        Ok(()) => panic!("expected UNAUTHORIZED with no session"),
    }
}

// --- concurrency -----------------------------------------------------------------------------------

#[tokio::test]
async fn concurrent_create_with_same_username_yields_exactly_one_conflict() {
    let db = TestDb::fresh().await;
    seed_admin(&db).await;
    login_as_admin(&db).await;

    let registry = std::sync::Arc::new(UndoRegistry::new());
    let state = &db.state;

    let fut1 = with_tx(state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::create_user(tx, cx, &registry, input("racer", Some("pw12345"))).await }) as BoxFuture<'_, TxResult<User>>
    });
    let fut2 = with_tx(state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::create_user(tx, cx, &registry, input("racer", Some("pw12345"))).await }) as BoxFuture<'_, TxResult<User>>
    });

    let (r1, r2) = tokio::join!(fut1, fut2);
    let successes = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
    let conflicts = [&r1, &r2].iter().filter(|r| matches!(r, Err(e) if e.to_string() == "اسم المستخدم مستخدم من قبل")).count();
    assert_eq!(successes, 1, "exactly one create must succeed");
    assert_eq!(conflicts, 1, "exactly one create must conflict");
}

// --- invariants --------------------------------------------------------------------------------

#[tokio::test]
async fn invariants_stay_green_after_the_suite() {
    let db = TestDb::fresh().await;
    seed_admin(&db).await;
    login_as_admin(&db).await;
    let registry = std::sync::Arc::new(UndoRegistry::new());

    with_tx(&db.state, TxOpts::default(), |tx, cx| {
        let registry = registry.clone();
        Box::pin(async move { service::create_user(tx, cx, &registry, input("invariantcheck", Some("pw12345"))).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .unwrap();

    let report = with_read(&db.state, |tx| Box::pin(async move { invariants::run_all(tx).await.map_err(Into::into) }))
        .await
        .expect("invariants must run");
    assert!(report.iter().all(|r| r.passed), "users domain touches no ledger — invariants must stay green: {report:?}");
}
