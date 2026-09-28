//! `products` domain IPC commands — split into `catalog` (22 commands, 06-products.md) and
//! `inventory` + `transfers` (25 commands, 06b-inventory.md).

pub mod catalog;
pub mod inventory;
pub mod transfers;
