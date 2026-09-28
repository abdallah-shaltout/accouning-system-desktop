//! `users` service logic (03-domains/03-users.md §3) — behaviour-exact port of
//! `userService.ts:8-60` + `authService.ts:67-131`. Every step below cites its mock line.

use std::sync::OnceLock;

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::core::auth::{self, AuthenticatedUser, Role};
use crate::core::error::{map_unique_violation, AppError};
use crate::core::lock;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::org::credentials;
use crate::entities::org::users::{self, ActiveModel as UserActiveModel, Column, Entity as UserEntity};
use crate::entities::values::StringList;
use crate::shared::activity;
use crate::utils::id::Id;
use crate::utils::route::RouteRef;

use super::dto::{role_as_str, to_dto, User, UserInput};

const USERNAME_UNIQUE_CONSTRAINT: &str = "uq_users_username";

/// A process-wide dummy argon2 hash, verified against for an unknown username/missing credential
/// row so a failed login takes the same amount of CPU time whether or not the username exists (D-6,
/// closes the timing side-channel `authService.ts` never had to worry about because it did a plain
/// object-map lookup). Generated once, lazily, with a fixed password — its value is never checked
/// against, only its *shape* (a real argon2id hash) matters for the timing to match a real verify.
fn dummy_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| auth::hash_password("equal-dummy-password-for-timing-parity").expect("dummy hash must succeed"))
}

/// `authService.ts:70` (`db.users.find((u) => u.username.toLowerCase() === username.trim()
/// .toLowerCase())`) — the SQL `WHERE username = ?` (case-insensitive collation, `uq_users_username`
/// table `utf8mb4_unicode_ci`) is a superset (it also folds accents/whitespace differences the JS
/// comparison wouldn't), so every candidate is re-checked in Rust against the exact JS predicate and
/// only the **first** row (by `created_at, id` — the mock's array order) that matches is kept.
async fn find_by_login_name<C: ConnectionTrait>(conn: &C, raw: &str) -> TxResult<Option<users::Model>> {
    let needle = raw.trim().to_lowercase();
    let candidates = UserEntity::find()
        .filter(Column::Username.eq(raw.to_string()))
        .order_by_asc(Column::CreatedAt)
        .order_by_asc(Column::Id)
        .all(conn)
        .await?;
    Ok(candidates.into_iter().find(|u| u.username.to_lowercase() == needle))
}

/// `core::auth::verify_password` inside `spawn_blocking` (D-7, argon2 is CPU-bound) — verifies
/// against `hash`, or the process-wide dummy hash when `hash` is `None` (no such user / no
/// credentials row), so the two "doesn't exist" cases cost the same wall-clock time as a real
/// mismatch (D-6). A hash-parsing error is an infrastructure fault, not a bad password -> `INTERNAL`.
async fn verify(password: &str, hash: Option<&str>) -> TxResult<bool> {
    let password = password.to_string();
    let hash: String = match hash {
        Some(h) => h.to_string(),
        None => dummy_hash().to_string(),
    };
    let result = tokio::task::spawn_blocking(move || auth::verify_password(&password, &hash))
        .await
        .map_err(|e| AppError::internal("تعذر التحقق من كلمة المرور", Some(e.to_string())))?;
    result.map_err(|e| AppError::internal("تعذر التحقق من كلمة المرور", Some(e.to_string())).into())
}

/// `hash_password` inside `spawn_blocking` (D-7).
async fn hash(password: &str) -> TxResult<String> {
    let password = password.to_string();
    let result = tokio::task::spawn_blocking(move || auth::hash_password(&password))
        .await
        .map_err(|e| AppError::internal("تعذر تشفير كلمة المرور", Some(e.to_string())))?;
    result.map_err(|e| AppError::internal("تعذر تشفير كلمة المرور", Some(e.to_string())).into())
}

/// `''`/absent -> `None` (D-5): the request-only `Option<String>` id fields on `UserInput` (not
/// `Id`, so an empty string can deserialize) collapse to `None` before touching an `Option<Id>`
/// column — an empty string is never a valid foreign key.
fn parse_optional_id(raw: &Option<String>) -> Option<Id> {
    raw.as_deref().filter(|s| !s.is_empty()).and_then(|s| s.parse::<Id>().ok())
}

/// **`get_users`** (`userService.ts:8-11`): all rows, mock array order (`created_at, id`).
pub async fn get_users<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<User>> {
    let rows = UserEntity::find().order_by_asc(Column::CreatedAt).order_by_asc(Column::Id).all(conn).await?;
    Ok(rows.iter().map(to_dto).collect())
}

/// **`get_user`** (`:13-18`).
pub async fn get_user<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<User> {
    let row = UserEntity::find_by_id(id).one(conn).await?.ok_or_else(|| AppError::not_found("المستخدم غير موجود"))?;
    Ok(to_dto(&row))
}

/// `assertUnique` (`:20-24`) — untrimmed username (Q-1), excluding `except_id` on update.
async fn assert_unique<C: ConnectionTrait>(conn: &C, username: &str, except_id: Option<Id>) -> TxResult<()> {
    let needle = username.to_lowercase();
    let rows = UserEntity::find().all(conn).await?;
    let clashes = rows.iter().any(|u| Some(u.id) != except_id && u.username.to_lowercase() == needle);
    if clashes {
        return Err(AppError::conflict("اسم المستخدم مستخدم من قبل").into());
    }
    Ok(())
}

/// **`create_user`** (`userService.ts:26-38`) — `pub` so 02-setup's bootstrap step can reuse it
/// directly (H-1, G-48 asks `to_authenticated` be `pub` for the same reason).
pub async fn create_user<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, input: UserInput) -> TxResult<User> {
    // 1. Uniqueness pre-check on the untrimmed username (`:20-24,28`).
    assert_unique(conn, &input.username, None).await?;

    // 2. Password required for a new user (`:29`, JS falsy: absent or "").
    let password = match input.password.as_deref() {
        Some(p) if !p.is_empty() => p,
        _ => return Err(AppError::validation("كلمة المرور مطلوبة للمستخدم الجديد").into()),
    };

    // 3. Hash before any insert.
    let password_hash = hash(password).await?;

    // 4. Insert `users`.
    let id = Id::new();
    let now = cx.clock.now;
    let model = UserActiveModel {
        id: Set(id),
        username: Set(input.username.trim().to_string()),
        name: Set(input.name.clone()),
        phone: Set(input.phone.clone()),
        role: Set(role_as_str(input.role).to_string()),
        max_discount: Set(crate::utils::money::round4(input.max_discount)),
        price_list_id: Set(parse_optional_id(&input.price_list_id)),
        active: Set(input.active),
        avatar: Set(None),
        allowed_branches: Set(input.allowed_branches.clone().map(StringList)),
        home_branch: Set(parse_optional_id(&input.home_branch)),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    let inserted = model
        .insert(conn)
        .await
        .map_err(|e| map_unique_violation(e, USERNAME_UNIQUE_CONSTRAINT, || "اسم المستخدم مستخدم من قبل".to_string()))?;

    // 5. Insert `credentials`.
    let creds = credentials::ActiveModel {
        user_id: Set(id),
        password_hash: Set(password_hash),
        created_at: Set(now),
        updated_at: Set(now),
    };
    creds.insert(conn).await?;

    // 6. Audit + activity (`:36`).
    activity::log(
        conn,
        cx,
        registry,
        crate::entities::platform::activity::ActivityKind::User,
        format!("إضافة المستخدم {}", inserted.name),
        None,
        Some(RouteRef::detail("user-editor", id.to_string())),
    )
    .await?;

    // 7.
    Ok(to_dto(&inserted))
}

/// **`update_user`** (`userService.ts:40-60`).
pub async fn update_user<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, id: Id, input: UserInput) -> TxResult<User> {
    // 1. Lock + load.
    lock::for_update_by_id(conn, "users", &id.to_string()).await?;
    let existing = UserEntity::find_by_id(id).one(conn).await?.ok_or_else(|| AppError::not_found("المستخدم غير موجود"))?;

    // 2. Uniqueness among others (`:44`).
    assert_unique(conn, &input.username, Some(id)).await?;

    // 3. Self-protection (`:45-47`).
    if let Some(actor) = &cx.actor {
        if actor.id == id {
            let deactivating = !input.active;
            let changing_role = role_as_str(input.role) != existing.role;
            if deactivating || changing_role {
                return Err(AppError::validation("لا يمكنك إيقاف حسابك أو تغيير صلاحيتك بنفسك").into());
            }
        }
    }

    // 4. Assign (`Object.assign` semantics, `:51`).
    let mut active_model: UserActiveModel = existing.clone().into();
    active_model.username = Set(input.username.trim().to_string());
    active_model.name = Set(input.name.clone());
    active_model.role = Set(role_as_str(input.role).to_string());
    active_model.max_discount = Set(crate::utils::money::round4(input.max_discount));
    active_model.active = Set(input.active);
    // `price_list_id` is always reassigned, even when absent/empty (`:51`).
    active_model.price_list_id = Set(parse_optional_id(&input.price_list_id));
    // `phone`/`allowed_branches`/`home_branch` only when present in the input — an absent key means
    // "unchanged" (JS `Object.assign` skips missing keys); since `UserInput` is a plain struct (not
    // a partial map), the DTO always carries every optional field's current value from the caller,
    // so "present" here means "not sent as JSON null/absent" — the frontend switch line forwards the
    // JS object's actual keys, and Rust's `Option::None` for these already models "not sent".
    if let Some(phone) = &input.phone {
        active_model.phone = Set(Some(phone.clone()));
    }
    if let Some(allowed) = &input.allowed_branches {
        active_model.allowed_branches = Set(Some(StringList(allowed.clone())));
    }
    if let Some(home_branch) = &input.home_branch {
        active_model.home_branch = Set(parse_optional_id(&Some(home_branch.clone())));
    }
    active_model.updated_at = Set(cx.clock.now);

    let updated = active_model
        .update(conn)
        .await
        .map_err(|e| map_unique_violation(e, USERNAME_UNIQUE_CONSTRAINT, || "اسم المستخدم مستخدم من قبل".to_string()))?;

    // 5. Password (`:56`): non-empty -> hash + upsert `credentials`.
    if let Some(password) = input.password.as_deref() {
        if !password.is_empty() {
            let password_hash = hash(password).await?;
            match credentials::Entity::find_by_id(id).one(conn).await? {
                Some(existing_creds) => {
                    let mut creds_active: credentials::ActiveModel = existing_creds.into();
                    creds_active.password_hash = Set(password_hash);
                    creds_active.updated_at = Set(cx.clock.now);
                    creds_active.update(conn).await?;
                }
                None => {
                    let creds = credentials::ActiveModel {
                        user_id: Set(id),
                        password_hash: Set(password_hash),
                        created_at: Set(cx.clock.now),
                        updated_at: Set(cx.clock.now),
                    };
                    creds.insert(conn).await?;
                }
            }
        }
    }

    // 6. Audit (Q-2: action is `create`, the mock's `logActivity` default).
    activity::log(
        conn,
        cx,
        registry,
        crate::entities::platform::activity::ActivityKind::User,
        format!("تعديل بيانات المستخدم {}", updated.name),
        None,
        Some(RouteRef::detail("user-editor", id.to_string())),
    )
    .await?;

    // 7. Command layer refreshes `AppState.session` when `id == session.id` (D-4) — not this fn's job.
    Ok(to_dto(&updated))
}

/// **`login`** (`authService.ts:68-78`). Returns both the response `User` DTO and the
/// `AuthenticatedUser` the command layer installs into `AppState.session` only after the
/// transaction commits successfully.
pub async fn login<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::UndoRegistry,
    username: &str,
    password: &str,
) -> TxResult<(User, AuthenticatedUser)> {
    let found = find_by_login_name(conn, username).await?;
    let creds = match &found {
        Some(u) => credentials::Entity::find_by_id(u.id).one(conn).await?,
        None => None,
    };

    let ok = verify(password, creds.as_ref().map(|c| c.password_hash.as_str())).await?;
    let Some(user) = found.filter(|_| ok && creds.is_some()) else {
        return Err(AppError::unauthorized("اسم المستخدم أو كلمة المرور غير صحيحة").into());
    };

    // `!active` after the password check (`:74`).
    if !user.active {
        return Err(AppError::forbidden("هذا الحساب موقوف — تواصل مع مدير النظام").into());
    }

    // `authService.ts:76` — `record` directly (not `log`): the actor is `None` before login, so
    // `entity_from_link(Auth, None)` must be forced to `("auth", new id)` (`log()` takes the user
    // from `cx.actor`, which doesn't exist yet).
    let (entity, entity_id) = activity::entity_from_link(crate::entities::platform::activity::ActivityKind::Auth, None);
    activity::record(
        conn,
        cx,
        registry,
        activity::AuditInput {
            entity,
            entity_id,
            entity_label: None,
            action: crate::entities::platform::audit::AuditAction::Login,
            before: None,
            after: None,
            user_id: Some(user.id),
            branch_id: None,
            at: None,
            reason: None,
            message: format!("تسجيل دخول {}", user.name),
            link: None,
            activity_kind: Some(crate::entities::platform::activity::ActivityKind::Auth),
            undo: None,
        },
    )
    .await?;

    let settings = crate::core::settings::load(conn).await?;
    let authenticated = to_authenticated(&user, settings.default_branch_id);

    Ok((to_dto(&user), authenticated))
}

/// **`restore_session`** (`authService.ts:81-87`, D-1). Only re-attaches *this process's existing*
/// session — the `user_id` argument alone never authenticates anything.
pub async fn restore_session<C: ConnectionTrait>(conn: &C, current_session: Option<&AuthenticatedUser>, user_id: Id) -> TxResult<Option<(User, AuthenticatedUser)>> {
    let Some(session) = current_session else { return Ok(None) };
    if session.id != user_id {
        return Ok(None);
    }
    let row = UserEntity::find_by_id(user_id).one(conn).await?;
    match row {
        Some(user) if user.active => {
            let settings = crate::core::settings::load(conn).await?;
            let authenticated = to_authenticated(&user, settings.default_branch_id);
            Ok(Some((to_dto(&user), authenticated)))
        }
        _ => Ok(None),
    }
}

/// **`verify_manager_pin`** (`authService.ts:101-110`). No write, no session change.
pub async fn verify_manager_pin<C: ConnectionTrait>(conn: &C, username: &str, password: &str) -> TxResult<User> {
    let found = find_by_login_name(conn, username).await?;
    let creds = match &found {
        Some(u) => credentials::Entity::find_by_id(u.id).one(conn).await?,
        None => None,
    };
    let ok = verify(password, creds.as_ref().map(|c| c.password_hash.as_str())).await?;
    let Some(user) = found.filter(|_| ok && creds.is_some()) else {
        return Err(AppError::unauthorized("اسم المستخدم أو كلمة المرور غير صحيحة").into());
    };
    if !user.active {
        return Err(AppError::forbidden("هذا الحساب موقوف").into());
    }
    let role = super::dto::parse_role(&user.role);
    if !matches!(role, Role::Admin | Role::Manager) {
        return Err(AppError::forbidden("هذا المستخدم ليس مديراً — الاعتماد يتطلب صلاحية مدير").into());
    }
    Ok(to_dto(&user))
}

/// Builds the `AppState.session` shape from a persisted row (§3 "login" step 4, and reused by
/// `restore_session`/the command layer's D-4 self-edit refresh). **`pub`** per gap G-48 — 02-setup's
/// bootstrap-admin flow needs to build a session the same way right after creating the first user,
/// without a real `login` round trip.
pub fn to_authenticated(user: &users::Model, default_branch_id: Id) -> AuthenticatedUser {
    let allowed_branches = user
        .allowed_branches
        .as_ref()
        .map(|list| list.0.iter().filter_map(|s| s.parse::<Id>().ok()).collect())
        .unwrap_or_default();
    AuthenticatedUser {
        id: user.id,
        username: user.username.clone(),
        role: super::dto::parse_role(&user.role),
        home_branch_id: user.home_branch.unwrap_or(default_branch_id),
        allowed_branches,
        price_list_id: user.price_list_id,
        max_discount: Some(user.max_discount),
    }
}
