//! Accessor for the singleton `settings` row (21.02-B, B-8). Owned by phase B.
//!
//! Two read paths, matching `with_tx`/`with_read`'s two transaction kinds (`core/tx.rs`):
//! - [`load`] — a plain `SELECT` inside whatever transaction is already open (read-mostly callers:
//!   `settings_get_settings`, the business-clock timezone lookup in `with_tx`).
//! - [`load_shared_locked`] — `SELECT ... FOR UPDATE`, for a caller about to read-modify-write the
//!   row (`settings_update_settings`) so two concurrent branch-DB writers never race each other's
//!   patch (cross-cutting.md §3 / `settings.md` §5/§8's own locking note).
//!
//! Both return the ORM `Model` directly — there being exactly one row (the `ck_settings_singleton`
//! CHECK), a missing row is an `AppError::Internal` (a fresh install always seeds one during setup;
//! Part 03's setup-wizard command is what inserts it), never a `NotFound` the frontend would show.

use sea_orm::{ConnectionTrait, EntityTrait, FromQueryResult, QuerySelect, Statement};

use crate::core::auth::{Access, Area, Role, RoleAccessOverrides as AuthRoleAccessOverrides};
use crate::core::error::{AppError, AppResult};
use crate::entities::org::settings::{Entity, Model};

/// Plain read of the singleton row — no lock. Safe inside a read-only transaction
/// (`with_read`) or as a quick lookup at the top of a read-write one.
pub async fn load<C: ConnectionTrait>(conn: &C) -> AppResult<Model> {
    Entity::find().one(conn).await.map_err(AppError::from)?.ok_or_else(|| {
        AppError::internal("لم يتم العثور على صف الإعدادات — يجب تشغيل معالج الإعداد الأولي أولاً", None)
    })
}

/// `SELECT ... FOR UPDATE` — for a caller about to read-modify-write the row inside a read-write
/// transaction (`settings_update_settings`). `sea_orm`'s query builder has no first-class "lock"
/// modifier on a plain `find()`, so this issues the equivalent raw statement and maps the single
/// row back through the same `Entity`/`Model` the ORM would give `find()`.
pub async fn load_shared_locked<C: ConnectionTrait>(conn: &C) -> AppResult<Model> {
    // `lock_shared`/`lock_exclusive` exist on `Select` for backends that support `FOR UPDATE` via
    // sea-query's `LockType` — MySQL/MariaDB does. Exclusive lock: this path is only ever used by
    // a writer that is about to update the row in the same transaction.
    use sea_orm::QueryTrait;
    let select = Entity::find().lock_exclusive();
    let stmt: Statement = select.build(conn.get_database_backend());
    let row = conn.query_one(stmt).await.map_err(AppError::from)?;
    match row {
        Some(row) => Model::from_query_result(&row, "").map_err(AppError::from),
        None => Err(AppError::internal("لم يتم العثور على صف الإعدادات — يجب تشغيل معالج الإعداد الأولي أولاً", None)),
    }
}

/// Bridges `settings.role_access_overrides` (the JSON `values::RoleAccessOverrides` sparse map of
/// lowercase string keys) into the typed `core::auth::RoleAccessOverrides` map `check_access`
/// expects. An unknown role/area key (e.g. stale data from a renamed area) is skipped rather than
/// failing the whole settings load — a corrupt single override must never block every command in
/// the app from authorizing at all.
pub fn parse_role_access_overrides(raw: Option<&crate::entities::values::RoleAccessOverrides>) -> AuthRoleAccessOverrides {
    let mut out: AuthRoleAccessOverrides = std::collections::HashMap::new();
    let Some(raw) = raw else { return out };
    for (role_key, areas) in &raw.0 {
        let Some(role) = parse_role(role_key) else { continue };
        for (area_key, access_key) in areas {
            let (Some(area), Some(access)) = (parse_area(area_key), parse_access(access_key)) else { continue };
            out.entry(role).or_default().insert(area, access);
        }
    }
    out
}

fn parse_role(s: &str) -> Option<Role> {
    match s {
        "admin" => Some(Role::Admin),
        "manager" => Some(Role::Manager),
        "accountant" => Some(Role::Accountant),
        "cashier" => Some(Role::Cashier),
        "storekeeper" => Some(Role::Storekeeper),
        _ => None,
    }
}

fn parse_area(s: &str) -> Option<Area> {
    match s {
        "dashboard" => Some(Area::Dashboard),
        "pos" => Some(Area::Pos),
        "sales" => Some(Area::Sales),
        "inventory" => Some(Area::Inventory),
        "parties" => Some(Area::Parties),
        "purchases" => Some(Area::Purchases),
        "expenses" => Some(Area::Expenses),
        "accounting" => Some(Area::Accounting),
        "payments" => Some(Area::Payments),
        "reports" => Some(Area::Reports),
        "analytics" => Some(Area::Analytics),
        "approvals" => Some(Area::Approvals),
        "users" => Some(Area::Users),
        "settings" => Some(Area::Settings),
        _ => None,
    }
}

fn parse_access(s: &str) -> Option<Access> {
    match s {
        "none" => Some(Access::None),
        "read" => Some(Access::Read),
        "write" => Some(Access::Write),
        _ => None,
    }
}

/// Resolves `settings.timezone` into a `chrono_tz::Tz`, tolerating "no settings row yet" (fresh
/// install, before the setup wizard has posted its first write) by returning `None` — `with_tx`'s
/// `BusinessClock` already treats `None` as "fall back to the Main PC's OS timezone" (cross-cutting
/// §7), so a missing/unset timezone is never an error here, only a `NotFound`-shaped settings load
/// (a fresher problem than this function's job) would be.
pub async fn load_business_timezone<C: ConnectionTrait>(conn: &C) -> Option<chrono_tz::Tz> {
    let model = load(conn).await.ok()?;
    let tz_name = model.timezone.as_deref()?;
    tz_name.parse().ok()
}

/// `check_access` wired against this row's `role_access_overrides` — `TxCtx::require`'s
/// implementation (`core/tx.rs`) calls this so a controller writes `cx.require(Area::Sales,
/// Access::Write)?` instead of re-deriving the overrides lookup at every call site.
pub async fn require<C: ConnectionTrait>(
    conn: &C,
    actor: Option<&crate::core::auth::AuthenticatedUser>,
    area: Area,
    required: Access,
) -> AppResult<()> {
    let model = load(conn).await?;
    let overrides = parse_role_access_overrides(model.role_access_overrides.as_ref());
    crate::core::auth::check_access(actor, area, required, &overrides)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::values::RoleAccessOverrides as JsonOverrides;
    use std::collections::BTreeMap;

    #[test]
    fn parse_role_access_overrides_skips_unknown_keys() {
        let mut areas = BTreeMap::new();
        areas.insert("reports".to_string(), "read".to_string());
        areas.insert("not_a_real_area".to_string(), "write".to_string());
        let mut roles = BTreeMap::new();
        roles.insert("cashier".to_string(), areas);
        roles.insert("not_a_real_role".to_string(), BTreeMap::new());

        let parsed = parse_role_access_overrides(Some(&JsonOverrides(roles)));
        assert_eq!(parsed.get(&Role::Cashier).and_then(|m| m.get(&Area::Reports)), Some(&Access::Read));
        assert_eq!(parsed.len(), 1, "unknown role key must be skipped, not inserted as a spurious entry");
    }

    #[test]
    fn parse_role_access_overrides_handles_none() {
        let parsed = parse_role_access_overrides(None);
        assert!(parsed.is_empty());
    }
}
