//! `domains::accounting::commands` — thin IPC layer, split by area (12-accounting.md §1,
//! 12b-period-close.md §1). 30 commands total (19 here + 11 in `period.rs`, 12b-owned).

pub mod accounts;
pub mod journal;
pub mod period;
pub mod templates;
