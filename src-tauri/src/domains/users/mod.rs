//! `users` domain (03-domains/03-users.md) — user records, login/session with argon2 credentials,
//! manager PIN. 8 IPC commands (§1); no undo compensators (§5 — not undoable via the registry).

pub mod commands;
pub mod dto;
pub mod service;

use crate::core::ipc::IpcSig;
use crate::ipc_sig;

/// §1's 8 commands, in table order.
pub fn ipc_signatures() -> Vec<IpcSig> {
    vec![
        ipc_sig!(users_get_users, (), Vec<dto::User>),
        ipc_sig!(users_get_user, dto::UsersGetUserArgs, dto::User),
        ipc_sig!(users_create_user, dto::UsersCreateUserArgs, dto::User),
        ipc_sig!(users_update_user, dto::UsersUpdateUserArgs, dto::User),
        ipc_sig!(users_login, dto::UsersLoginArgs, dto::User),
        ipc_sig!(users_logout, (), ()),
        ipc_sig!(users_restore_session, dto::UsersRestoreSessionArgs, Option<dto::User>),
        ipc_sig!(users_verify_manager_pin, dto::UsersVerifyManagerPinArgs, dto::User),
    ]
}

/// G-8a: this domain's DTO exports (§2, plus `core::auth`'s G-7 enums, exported alongside since
/// nothing else in this wave exports them yet). The manager calls this one line from the top-level
/// `domains::export_bindings` hook (`domains/mod.rs`, manager-owned) in the same commit that adds
/// `pub mod users;` there — see this checklist item in `03-domains/03-users.md` §9.
pub fn export_bindings(cfg: &ts_rs::Config) {
    use ts_rs::TS;
    dto::User::export_all(cfg).expect("export User");
    dto::UserInput::export_all(cfg).expect("export UserInput");
    dto::UsersGetUserArgs::export_all(cfg).expect("export UsersGetUserArgs");
    dto::UsersCreateUserArgs::export_all(cfg).expect("export UsersCreateUserArgs");
    dto::UsersUpdateUserArgs::export_all(cfg).expect("export UsersUpdateUserArgs");
    dto::UsersLoginArgs::export_all(cfg).expect("export UsersLoginArgs");
    dto::UsersRestoreSessionArgs::export_all(cfg).expect("export UsersRestoreSessionArgs");
    dto::UsersVerifyManagerPinArgs::export_all(cfg).expect("export UsersVerifyManagerPinArgs");
    crate::core::auth::Role::export_all(cfg).expect("export Role");
    crate::core::auth::Area::export_all(cfg).expect("export Area");
    crate::core::auth::Access::export_all(cfg).expect("export Access");
}
