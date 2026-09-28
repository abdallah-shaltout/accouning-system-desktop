//! `domains::reports` (03-domains/13-reports.md, 13b-reports-operational.md): 32 read-only
//! commands — financial statements/ledgers (13) and operational reports (13b). No writes, no
//! events, no undo (both files' §5): every command runs inside one `with_read_ctx` REPEATABLE READ
//! snapshot and returns exactly the mock's DTO shape.

pub mod commands;
pub mod dto;
pub mod service;

pub fn ipc_signatures() -> Vec<crate::core::ipc::IpcSig> {
    commands::ipc_signatures()
}

/// G-8a: this domain's DTO exports — the manager calls this one line from
/// `domains::export_bindings` (`domains/mod.rs`) in the same commit that adds `pub mod reports;`
/// there, per this checklist item in `03-domains/13-reports.md` §9 / `13b-reports-operational.md` §9.
pub fn export_bindings(cfg: &ts_rs::Config) {
    dto::export_bindings(cfg);
}
