//! `products` domain service logic — split per 06/06b's file plan: `products`/`catalog` (catalog
//! half) and `stock_lines`/`adjustments`/`movements`/`batches`/`counts`/`transfers` (inventory half).

pub mod adjustments;
pub mod batches;
pub mod catalog;
pub mod counts;
pub mod movements;
pub mod products;
pub mod stock_lines;
pub mod transfers;
