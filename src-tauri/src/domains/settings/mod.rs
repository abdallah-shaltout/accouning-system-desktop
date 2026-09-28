//! `domains::settings` (01-settings.md): store settings with the branch/device split, taxes,
//! payment methods, branches, cost centers, currencies, revaluation, LAN sharing + server-failure
//! screen. 33 commands (§1).

pub mod commands;
pub mod dto;
pub mod service;

pub fn ipc_signatures() -> Vec<crate::core::ipc::IpcSig> {
    commands::ipc_signatures()
}
