//! `diagnostics` service logic, split by area (entry-file §3.1 convention: split past ~400 lines).
//! Slice A (W1): audit reads + the support-bundle's settings/server parts (`audit.rs`/`support.rs`).
//! Slice A2 (after 17-backup): the support-bundle DB snapshot, folded into `support.rs`. Slice B
//! (after 12-accounting): the 7 debug-build accounting-debugger reads, `debugger.rs` — see
//! `plans/pending/21-rust-backend/03-domains/16-diagnostics.md`.

pub mod audit;
pub mod debugger;
pub mod support;
