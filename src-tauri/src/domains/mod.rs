//! Business domains (plan 21 Part 03, master plan §4): one module per Vue module, each with
//! `commands.rs` (thin IPC layer) · `service.rs` (logic) · `dto.rs` (ts-rs DTOs = the TS types).
//! Manager-owned: implementers add their domain's `pub mod` line and hook lines through the
//! manager, never directly (entry file 03-DOMAINS-IMPLEMENTATION.md §3.7).

use crate::core::ipc::IpcSig;
use crate::shared::activity::undo::UndoRegistry;

/// Every domain command's `ipc_sig!` line, concatenated — `core::ipc::all_signatures()` appends
/// this so `ipc_manifest_matches_handler` sees each new command next to `generate_handler!`.
pub fn all_ipc_signatures() -> Vec<IpcSig> {
    Vec::new()
}

/// Registers every domain's undo compensators (phase-e E-5: accounting ×4, setup ×1) into the
/// registry `AppState` holds — "scale by adding": one line per domain that has compensators.
pub fn register_undo(_registry: &mut UndoRegistry) {}
