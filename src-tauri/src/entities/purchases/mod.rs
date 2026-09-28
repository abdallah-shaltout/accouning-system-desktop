//! `purchases` table entities (21.02-B, owner B2). Declare each table as `pub mod <table>;`.

pub mod purchase_order_lines;
pub mod purchase_orders;
pub mod purchase_return_lines;
pub mod purchase_returns;

pub use purchase_order_lines::Entity as PurchaseOrderLines;
pub use purchase_orders::Entity as PurchaseOrders;
pub use purchase_return_lines::Entity as PurchaseReturnLines;
pub use purchase_returns::Entity as PurchaseReturns;
