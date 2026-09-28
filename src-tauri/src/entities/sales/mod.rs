//! `sales` table entities (21.02-B, owner B2). Declare each table as `pub mod <table>;`.

pub mod held_sales;
pub mod invoice_lines;
pub mod invoice_tenders;
pub mod invoices;
pub mod quotation_lines;
pub mod quotations;
pub mod refund_lines;
pub mod refunds;
pub mod shift_movements;
pub mod shifts;

pub use held_sales::Entity as HeldSales;
pub use invoice_lines::Entity as InvoiceLines;
pub use invoice_tenders::Entity as InvoiceTenders;
pub use invoices::Entity as Invoices;
pub use quotation_lines::Entity as QuotationLines;
pub use quotations::Entity as Quotations;
pub use refund_lines::Entity as RefundLines;
pub use refunds::Entity as Refunds;
pub use shift_movements::Entity as ShiftMovements;
pub use shifts::Entity as Shifts;
