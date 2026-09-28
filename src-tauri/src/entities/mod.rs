//! SeaORM entities, one file per table (21.02-B). Grouped by owning module so the two parallel
//! phase-B agents never edit the same registration file; every table module is re-exported flat,
//! so callers write `crate::entities::<table>` regardless of its group. Shared helpers:
//! `values` (JSON value objects), `doc_date` (the P2-09 DocDate bridge), `soft_delete` (P2-16) —
//! owned by B1.

pub mod doc_date;
pub mod soft_delete;
pub mod values;

pub mod org;
pub mod catalog;
pub mod inventory;
pub mod parties;
pub mod sales;
pub mod purchases;
pub mod payments;
pub mod expenses;
pub mod journal;
pub mod platform;

pub use org::*;
pub use catalog::*;
pub use inventory::*;
pub use parties::*;
pub use sales::*;
pub use purchases::*;
pub use payments::*;
pub use expenses::*;
pub use journal::*;
pub use platform::*;
