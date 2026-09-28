//! `domains::setup` (02-setup.md): first-run device role + provisioning + terminal pairing, the
//! 11-step onboarding wizard, opening balances, party openings. 22 commands (§1: 19 ported + 3
//! device). Registers one undo compensator (`setup.postPartyOpening`, §5).

pub mod commands;
pub mod dto;
pub mod service;
pub mod undo;

pub fn ipc_signatures() -> Vec<crate::core::ipc::IpcSig> {
    commands::ipc_signatures()
}

pub fn register_undo(r: &mut crate::shared::activity::undo::UndoRegistry) {
    undo::register_undo(r);
}
