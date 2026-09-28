//! `payments` table entities (21.02-B, owner B2). Declare each table as `pub mod <table>;`.

pub mod card_settlement_groups;
pub mod card_settlements;
pub mod payment_allocations;
pub mod payments;
pub mod vouchers;

pub use card_settlement_groups::Entity as CardSettlementGroups;
pub use card_settlements::Entity as CardSettlements;
pub use payment_allocations::Entity as PaymentAllocations;
pub use payments::Entity as Payments;
pub use vouchers::Entity as Vouchers;
