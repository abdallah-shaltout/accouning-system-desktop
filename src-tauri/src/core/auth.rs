//! Role/access model — `ROLE_ACCESS` copied verbatim from
//! `src/modules/users/helpers/permissions.ts:15-36`, plus argon2 password hashing (cross-cutting.md
//! §1) and the `AuthenticatedUser` shape held in `AppState`.

use std::collections::HashMap;

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::core::error::AppError;
use crate::utils::id::Id;

/// G-7: `TS` derives so DTOs that reference these (e.g. `User.role`, permission-matrix screens) get
/// a real generated type instead of hand-written duplicates. Exported like the other domain DTOs —
/// no `#[ts(export)]` attribute; `core/ipc.rs`'s single `export_bindings` test (via the G-8 hook)
/// drives every export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "users/types/gen/")]
pub enum Role {
    Admin,
    Manager,
    Accountant,
    Cashier,
    Storekeeper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "users/types/gen/")]
pub enum Area {
    Dashboard,
    Pos,
    Sales,
    Inventory,
    Parties,
    Purchases,
    Expenses,
    Accounting,
    Payments,
    Reports,
    Analytics,
    Approvals,
    Users,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "users/types/gen/")]
pub enum Access {
    None,
    Read,
    Write,
}

/// Ported verbatim from `permissions.ts:15-36` — every role × area cell.
pub fn role_access(role: Role, area: Area) -> Access {
    use Access::*;
    use Area::*;
    match (role, area) {
        (Role::Admin, _) => Write,

        (Role::Manager, Users) => None,
        (Role::Manager, _) => Write,

        (Role::Accountant, Dashboard) => Read,
        (Role::Accountant, Pos) => None,
        (Role::Accountant, Sales) => Write,
        (Role::Accountant, Inventory) => Read,
        (Role::Accountant, Parties) => Read,
        (Role::Accountant, Purchases) => Read,
        (Role::Accountant, Expenses) => Write,
        (Role::Accountant, Accounting) => Write,
        (Role::Accountant, Payments) => Write,
        (Role::Accountant, Reports) => Write,
        (Role::Accountant, Analytics) => Read,
        (Role::Accountant, Approvals) => None,
        (Role::Accountant, Users) => None,
        (Role::Accountant, Settings) => None,

        (Role::Cashier, Dashboard) => Read,
        (Role::Cashier, Pos) => Write,
        (Role::Cashier, Sales) => Write,
        (Role::Cashier, Inventory) => Read,
        (Role::Cashier, Parties) => Read,
        (Role::Cashier, Purchases) => None,
        (Role::Cashier, Expenses) => Write,
        (Role::Cashier, Accounting) => None,
        (Role::Cashier, Payments) => None,
        (Role::Cashier, Reports) => None,
        (Role::Cashier, Analytics) => None,
        (Role::Cashier, Approvals) => None,
        (Role::Cashier, Users) => None,
        (Role::Cashier, Settings) => None,

        (Role::Storekeeper, Dashboard) => Read,
        (Role::Storekeeper, Pos) => None,
        (Role::Storekeeper, Sales) => None,
        (Role::Storekeeper, Inventory) => Write,
        (Role::Storekeeper, Parties) => Read,
        (Role::Storekeeper, Purchases) => Write,
        (Role::Storekeeper, Expenses) => None,
        (Role::Storekeeper, Accounting) => None,
        (Role::Storekeeper, Payments) => None,
        (Role::Storekeeper, Reports) => Read,
        (Role::Storekeeper, Analytics) => None,
        (Role::Storekeeper, Approvals) => None,
        (Role::Storekeeper, Users) => None,
        (Role::Storekeeper, Settings) => None,
    }
}

fn rank(access: Access) -> u8 {
    match access {
        Access::None => 0,
        Access::Read => 1,
        Access::Write => 2,
    }
}

/// Sparse per-(role, area) overrides an admin has set in `db.settings.roleAccessOverrides`
/// (`permissions.ts:48-59`).
pub type RoleAccessOverrides = HashMap<Role, HashMap<Area, Access>>;

pub fn effective_access(role: Role, area: Area, overrides: &RoleAccessOverrides) -> Access {
    overrides.get(&role).and_then(|m| m.get(&area)).copied().unwrap_or_else(|| role_access(role, area))
}

pub fn role_can(role: Role, area: Area, required: Access, overrides: &RoleAccessOverrides) -> bool {
    rank(effective_access(role, area, overrides)) >= rank(required)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub id: Id,
    pub username: String,
    pub role: Role,
    pub home_branch_id: Id,
    pub allowed_branches: Vec<Id>,
    pub price_list_id: Option<Id>,
    pub max_discount: Option<rust_decimal::Decimal>,
}

/// `check_access` (phase-a-foundation.md A-5): `UNAUTHORIZED` when there's no session at all,
/// `FORBIDDEN` when there is one but it lacks the required access.
pub fn check_access(
    user: Option<&AuthenticatedUser>,
    area: Area,
    required: Access,
    overrides: &RoleAccessOverrides,
) -> Result<(), AppError> {
    let Some(user) = user else {
        return Err(AppError::unauthorized("سجّل الدخول أولاً"));
    };
    if role_can(user.role, area, required, overrides) {
        Ok(())
    } else {
        Err(AppError::forbidden("ليست لديك صلاحية لهذه العملية"))
    }
}

// --- Passwords (argon2id, cross-cutting.md §1) --------------------------------------------------

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut rand_core_from_argon2());
    let argon2 = Argon2::default();
    Ok(argon2.hash_password(password.as_bytes(), &salt)?.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, argon2::password_hash::Error> {
    let parsed = PasswordHash::new(hash)?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
}

/// `argon2`/`password-hash` re-export `rand_core` transitively; using its own `OsRng` keeps this
/// module dependency-free of a direct `rand` pin.
fn rand_core_from_argon2() -> impl argon2::password_hash::rand_core::CryptoRngCore {
    argon2::password_hash::rand_core::OsRng
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drift guard: parses `ROLE_ACCESS` straight out of `permissions.ts`'s source text and
    /// compares it cell-by-cell against `role_access` above, so the two tables can never silently
    /// diverge.
    #[test]
    fn role_access_matches_permissions_ts_source() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let ts_path = std::path::Path::new(manifest_dir)
            .join("..")
            .join("src")
            .join("modules")
            .join("users")
            .join("helpers")
            .join("permissions.ts");
        let source = std::fs::read_to_string(&ts_path).expect("permissions.ts must be readable from src-tauri/");

        let roles = [
            (Role::Admin, "admin"),
            (Role::Manager, "manager"),
            (Role::Accountant, "accountant"),
            (Role::Cashier, "cashier"),
            (Role::Storekeeper, "storekeeper"),
        ];
        let areas = [
            (Area::Dashboard, "dashboard"),
            (Area::Pos, "pos"),
            (Area::Sales, "sales"),
            (Area::Inventory, "inventory"),
            (Area::Parties, "parties"),
            (Area::Purchases, "purchases"),
            (Area::Expenses, "expenses"),
            (Area::Accounting, "accounting"),
            (Area::Payments, "payments"),
            (Area::Reports, "reports"),
            (Area::Analytics, "analytics"),
            (Area::Approvals, "approvals"),
            (Area::Users, "users"),
            (Area::Settings, "settings"),
        ];

        for (role, role_key) in roles {
            // Find this role's block: from `<role_key>: {` to the next role key's start.
            let start = source.find(&format!("{role_key}: {{")).unwrap_or_else(|| panic!("role '{role_key}' not found in permissions.ts"));
            let rest = &source[start..];
            let end = rest[1..].find("},").map(|i| i + 1).unwrap_or(rest.len());
            let block = &rest[..end];

            for (area, area_key) in areas {
                let needle = format!("{area_key}: '");
                let pos = block.find(&needle).unwrap_or_else(|| panic!("area '{area_key}' not found in {role_key}'s block"));
                let after = &block[pos + needle.len()..];
                let value_end = after.find('\'').unwrap();
                let ts_value = &after[..value_end];
                let expected = match ts_value {
                    "write" => Access::Write,
                    "read" => Access::Read,
                    "none" => Access::None,
                    other => panic!("unexpected access value '{other}' for {role_key}.{area_key}"),
                };
                let actual = role_access(role, area);
                assert_eq!(actual, expected, "{role_key}.{area_key}: expected {expected:?}, got {actual:?}");
            }
        }
    }

    #[test]
    fn override_precedence() {
        let mut overrides: RoleAccessOverrides = HashMap::new();
        overrides.entry(Role::Cashier).or_default().insert(Area::Reports, Access::Read);
        assert_eq!(effective_access(Role::Cashier, Area::Reports, &overrides), Access::Read);
        assert_eq!(role_access(Role::Cashier, Area::Reports), Access::None);
        // Unaffected cell falls back to the base table.
        assert_eq!(effective_access(Role::Cashier, Area::Pos, &overrides), Access::Write);
    }

    #[test]
    fn argon2_round_trip() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(verify_password("correct horse battery staple", &hash).unwrap());
        assert!(!verify_password("wrong password", &hash).unwrap());
    }
}
