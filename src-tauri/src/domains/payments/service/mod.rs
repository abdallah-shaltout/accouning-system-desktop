//! `domains::payments::service` (09-payments.md §3): shared helpers in `common`, then one file per
//! concern — `read` (§3.5/§3.6), `create` (§3.2), `allocate` (§3.3/§3.4).

pub mod allocate;
pub mod common;
pub mod create;
pub mod read;
