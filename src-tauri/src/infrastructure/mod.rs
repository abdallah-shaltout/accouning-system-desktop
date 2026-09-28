//! Infrastructure: adapters to the outside world that don't hold business logic (master plan §4).
//! `pdf/` (Typst PDF rendering) and `print/` (ESC/POS thermal printing) moved here unchanged in
//! 21.02-A (A-2) — command names are unchanged, only their module path (`pdf::` → `infrastructure::pdf::`).

pub mod backup;
pub mod database;
pub mod import;
pub mod pdf;
pub mod print;
