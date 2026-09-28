//! `users` DTOs (03-domains/03-users.md §2) — `User`/`UserInput` mirror
//! `src/modules/users/types/index.ts:6-39`; one args struct per command (§3.2 convention).
//! `Role`/`Area`/`Access` are **not** redeclared here — `core::auth` already derives `TS` (G-7) and
//! is exported to the same `users/types/gen/` directory via `domains::export_bindings` (G-8a).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::core::auth::Role;
use crate::entities::org::users;
use crate::utils::id::Id;
use crate::utils::money::serde_number;

/// `User` (`types/index.ts:6-25`). Optional fields are omitted on the wire when absent (never
/// `null`) via `skip_serializing_none` + `#[ts(optional)]`, matching the TS `field?: T` shape.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "users/types/gen/")]
pub struct User {
    #[ts(type = "string")]
    pub id: Id,
    pub username: String,
    pub name: String,
    #[ts(optional)]
    pub phone: Option<String>,
    pub role: Role,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub max_discount: Decimal,
    #[ts(optional, type = "string")]
    pub price_list_id: Option<Id>,
    pub active: bool,
    #[ts(optional)]
    pub avatar: Option<String>,
    #[ts(optional)]
    pub allowed_branches: Option<Vec<String>>,
    #[ts(optional, type = "string")]
    pub home_branch: Option<Id>,
}

/// `UserInput` (`types/index.ts:27-39`) — request-only; `password` never appears in any response
/// DTO. `price_list_id`/`home_branch` are plain `Option<String>` (not `Id`) so an empty string
/// `''` deserialises without failing id-parsing — the service maps `''`/absent -> `None` (D-5).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "users/types/gen/")]
pub struct UserInput {
    pub username: String,
    pub name: String,
    #[ts(optional)]
    pub phone: Option<String>,
    pub role: Role,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub max_discount: Decimal,
    #[ts(optional)]
    pub price_list_id: Option<String>,
    pub active: bool,
    /// Only sent when creating a user or changing the password.
    #[ts(optional)]
    pub password: Option<String>,
    #[ts(optional)]
    pub allowed_branches: Option<Vec<String>>,
    #[ts(optional)]
    pub home_branch: Option<String>,
}

/// Builds the response DTO from the persisted row. `allowed_branches`: `None` stays absent,
/// `Some([])` stays `[]` — the mock keeps whatever was sent (§2).
pub fn to_dto(m: &users::Model) -> User {
    User {
        id: m.id,
        username: m.username.clone(),
        name: m.name.clone(),
        phone: m.phone.clone(),
        role: parse_role(&m.role),
        max_discount: m.max_discount,
        price_list_id: m.price_list_id,
        active: m.active,
        avatar: m.avatar.clone(),
        allowed_branches: m.allowed_branches.clone().map(|list| list.0),
        home_branch: m.home_branch,
    }
}

/// The DB `role` column is a plain string (`'admin' | 'manager' | 'accountant' | 'cashier' |
/// 'storekeeper'`, see `entities/org/users.rs`'s doc comment) — parsed into the shared enum here
/// rather than at every call site. An unrecognized value is a data-integrity bug, not a request the
/// user can retry differently, so it panics rather than silently defaulting to a role with
/// different permissions (fails loudly, per project convention for "this should be impossible").
pub fn parse_role(s: &str) -> Role {
    match s {
        "admin" => Role::Admin,
        "manager" => Role::Manager,
        "accountant" => Role::Accountant,
        "cashier" => Role::Cashier,
        "storekeeper" => Role::Storekeeper,
        other => panic!("users.role column holds an unrecognized value: {other:?}"),
    }
}

pub fn role_as_str(role: Role) -> &'static str {
    match role {
        Role::Admin => "admin",
        Role::Manager => "manager",
        Role::Accountant => "accountant",
        Role::Cashier => "cashier",
        Role::Storekeeper => "storekeeper",
    }
}

// --- Command args (§2, §3.2: one struct per command, camelCase) --------------------------------

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "users/types/gen/")]
pub struct UsersGetUserArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "users/types/gen/")]
pub struct UsersCreateUserArgs {
    pub input: UserInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "users/types/gen/")]
pub struct UsersUpdateUserArgs {
    pub id: String,
    pub input: UserInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "users/types/gen/")]
pub struct UsersLoginArgs {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "users/types/gen/")]
pub struct UsersRestoreSessionArgs {
    pub user_id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "users/types/gen/")]
pub struct UsersVerifyManagerPinArgs {
    pub username: String,
    pub password: String,
}
