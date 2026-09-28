//! `inventory` table entities (21.02-B, owner B1): stock adjustments, stock movements, stock
//! counts, debit note drafts, stock transfers.

pub mod debit_note_drafts;
pub mod stock_adjustment_lines;
pub mod stock_adjustments;
pub mod stock_count_lines;
pub mod stock_counts;
pub mod stock_movements;
pub mod stock_transfer_lines;
pub mod stock_transfers;

pub use debit_note_drafts::Entity as DebitNoteDrafts;
pub use stock_adjustment_lines::Entity as StockAdjustmentLines;
pub use stock_adjustments::Entity as StockAdjustments;
pub use stock_count_lines::Entity as StockCountLines;
pub use stock_counts::Entity as StockCounts;
pub use stock_movements::Entity as StockMovements;
pub use stock_transfer_lines::Entity as StockTransferLines;
pub use stock_transfers::Entity as StockTransfers;
