//! m0008_sales (21.02-B, owner B2): invoices(+lines, tenders), refunds(+lines), quotations(+lines),
//! held_sales, shifts(+movements). Source types: `src/modules/invoices/types/index.ts`.
//!
//! FKs are added in m0015 (create order never matters). `down` drops tables in reverse dependency
//! order.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // --- invoices ---------------------------------------------------------------------------
        manager
            .create_table(
                Table::create()
                    .table(Invoices::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Invoices::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(Invoices::Number).string_len(40).not_null())
                    .col(ColumnDef::new(Invoices::DateDay).date().not_null())
                    .col(ColumnDef::new(Invoices::DateInstant).custom(Alias::new("DATETIME(3)")).null())
                    .col(ColumnDef::new(Invoices::CustomerId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Invoices::CashierId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(Invoices::Status).custom(Alias::new("ENUM('DRAFT','COMPLETED','REFUNDED')")).not_null())
                    .col(ColumnDef::new(Invoices::PaymentStatus).custom(Alias::new("ENUM('UNPAID','PARTIALLY_PAID','PAID')")).not_null())
                    .col(ColumnDef::new(Invoices::SubTotal).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Invoices::DiscountRate).decimal_len(9, 4).not_null())
                    .col(ColumnDef::new(Invoices::DiscountAmount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Invoices::TaxRate).decimal_len(9, 4).not_null())
                    .col(ColumnDef::new(Invoices::TaxAmount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Invoices::GrandTotal).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Invoices::PaymentMethod).custom(Alias::new("ENUM('cash','card','bank_transfer','credit')")).not_null())
                    .col(ColumnDef::new(Invoices::PaidAmount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Invoices::RefundedAmount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Invoices::TenderedAmount).decimal_len(19, 2).null())
                    .col(ColumnDef::new(Invoices::DueDate).date().null())
                    .col(ColumnDef::new(Invoices::Note).text().null())
                    .col(ColumnDef::new(Invoices::Source).custom(Alias::new("ENUM('POS','DESK')")).null())
                    .col(ColumnDef::new(Invoices::ShiftId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Invoices::BranchId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Invoices::InvoiceType).custom(Alias::new("ENUM('STANDARD','SIMPLIFIED')")).null())
                    .col(ColumnDef::new(Invoices::PoReference).string_len(200).null())
                    .col(ColumnDef::new(Invoices::Terms).text().null())
                    .col(ColumnDef::new(Invoices::AttachmentIds).json().null())
                    .col(ColumnDef::new(Invoices::Currency).char_len(3).null())
                    .col(ColumnDef::new(Invoices::ExchangeRate).decimal_len(19, 6).null())
                    .col(ColumnDef::new(Invoices::CostCenterId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Invoices::SearchNormalized).text().null())
                    .col(ColumnDef::new(Invoices::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(Invoices::UpdatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(Invoices::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                    .col(ColumnDef::new(Invoices::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .index(Index::create().name("uq_invoices_number").col(Invoices::Number).unique())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        add_doc_date_key(db, "invoices", "date").await?;

        manager
            .create_table(
                Table::create()
                    .table(InvoiceLines::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(InvoiceLines::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(InvoiceLines::InvoiceId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(InvoiceLines::Position).small_unsigned().not_null())
                    .col(ColumnDef::new(InvoiceLines::ProductId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(InvoiceLines::Name).string_len(200).not_null())
                    .col(ColumnDef::new(InvoiceLines::Qty).decimal_len(19, 4).not_null())
                    .col(ColumnDef::new(InvoiceLines::Price).decimal_len(19, 4).not_null())
                    .col(ColumnDef::new(InvoiceLines::CostPrice).decimal_len(19, 4).not_null())
                    .col(ColumnDef::new(InvoiceLines::Discount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(InvoiceLines::TaxId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(InvoiceLines::TaxCategory).string_len(32).null())
                    .col(ColumnDef::new(InvoiceLines::TaxRate).decimal_len(9, 4).null())
                    .col(ColumnDef::new(InvoiceLines::Net).decimal_len(19, 2).null())
                    .col(ColumnDef::new(InvoiceLines::Vat).decimal_len(19, 2).null())
                    .col(ColumnDef::new(InvoiceLines::UnitId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(InvoiceLines::UnitFactor).decimal_len(19, 6).null())
                    .col(ColumnDef::new(InvoiceLines::ListPrice).decimal_len(19, 4).null())
                    .col(ColumnDef::new(InvoiceLines::PriceOverrideReason).text().null())
                    .col(ColumnDef::new(InvoiceLines::BatchId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(InvoiceLines::BatchNo).string_len(64).null())
                    .col(ColumnDef::new(InvoiceLines::IsFreeText).boolean().not_null().default(false))
                    .col(ColumnDef::new(InvoiceLines::RevenueAccountId).custom(Alias::new("UUID")).null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_invoice_lines_invoice_id")
                            .from(InvoiceLines::Table, InvoiceLines::InvoiceId)
                            .to(Invoices::Table, Invoices::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(Index::create().name("uq_invoice_lines_invoice_id_position").col(InvoiceLines::InvoiceId).col(InvoiceLines::Position).unique())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(InvoiceTenders::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(InvoiceTenders::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(InvoiceTenders::InvoiceId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(InvoiceTenders::Position).small_unsigned().not_null())
                    .col(ColumnDef::new(InvoiceTenders::PaymentMethodId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(InvoiceTenders::Amount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(InvoiceTenders::Reference).string_len(200).null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_invoice_tenders_invoice_id")
                            .from(InvoiceTenders::Table, InvoiceTenders::InvoiceId)
                            .to(Invoices::Table, Invoices::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(Index::create().name("uq_invoice_tenders_invoice_id_position").col(InvoiceTenders::InvoiceId).col(InvoiceTenders::Position).unique())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        // --- refunds ----------------------------------------------------------------------------
        manager
            .create_table(
                Table::create()
                    .table(Refunds::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Refunds::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(Refunds::Number).string_len(40).not_null())
                    .col(ColumnDef::new(Refunds::InvoiceId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(Refunds::DateDay).date().not_null())
                    .col(ColumnDef::new(Refunds::DateInstant).custom(Alias::new("DATETIME(3)")).null())
                    .col(ColumnDef::new(Refunds::Reason).text().null())
                    .col(ColumnDef::new(Refunds::SubTotal).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Refunds::TaxAmount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Refunds::GrandTotal).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Refunds::SettledToReceivable).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Refunds::CashBack).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Refunds::RefundMethod).custom(Alias::new("ENUM('cash','card','bank_transfer','customer_credit')")).null())
                    .col(ColumnDef::new(Refunds::CreditedToAccount).decimal_len(19, 2).null())
                    .col(ColumnDef::new(Refunds::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(Refunds::UpdatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(Refunds::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                    .col(ColumnDef::new(Refunds::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .index(Index::create().name("uq_refunds_number").col(Refunds::Number).unique())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_refunds_invoice_id")
                            .from(Refunds::Table, Refunds::InvoiceId)
                            .to(Invoices::Table, Invoices::Id),
                    )
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        add_doc_date_key(db, "refunds", "date").await?;

        manager
            .create_table(
                Table::create()
                    .table(RefundLines::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(RefundLines::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(RefundLines::RefundId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(RefundLines::Position).small_unsigned().not_null())
                    .col(ColumnDef::new(RefundLines::InvoiceLineId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(RefundLines::Qty).decimal_len(19, 4).not_null())
                    .col(ColumnDef::new(RefundLines::Restock).boolean().null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_refund_lines_refund_id")
                            .from(RefundLines::Table, RefundLines::RefundId)
                            .to(Refunds::Table, Refunds::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(Index::create().name("uq_refund_lines_refund_id_position").col(RefundLines::RefundId).col(RefundLines::Position).unique())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        // --- quotations ---------------------------------------------------------------------------
        manager
            .create_table(
                Table::create()
                    .table(Quotations::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Quotations::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(Quotations::Number).string_len(40).not_null())
                    .col(ColumnDef::new(Quotations::DateDay).date().not_null())
                    .col(ColumnDef::new(Quotations::DateInstant).custom(Alias::new("DATETIME(3)")).null())
                    .col(ColumnDef::new(Quotations::ExpiryDate).date().null())
                    .col(ColumnDef::new(Quotations::CustomerId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Quotations::SalespersonId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(Quotations::Status).custom(Alias::new("ENUM('DRAFT','SENT','ACCEPTED','REJECTED','EXPIRED')")).not_null())
                    .col(ColumnDef::new(Quotations::DiscountRate).decimal_len(9, 4).not_null())
                    .col(ColumnDef::new(Quotations::DiscountAmount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Quotations::TaxAmount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Quotations::SubTotal).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Quotations::GrandTotal).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Quotations::Note).text().null())
                    .col(ColumnDef::new(Quotations::Terms).text().null())
                    .col(ColumnDef::new(Quotations::PoReference).string_len(200).null())
                    .col(ColumnDef::new(Quotations::AttachmentIds).json().null())
                    .col(ColumnDef::new(Quotations::ConvertedInvoiceId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Quotations::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(Quotations::UpdatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(Quotations::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                    .col(ColumnDef::new(Quotations::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .index(Index::create().name("uq_quotations_number").col(Quotations::Number).unique())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_quotations_converted_invoice_id")
                            .from(Quotations::Table, Quotations::ConvertedInvoiceId)
                            .to(Invoices::Table, Invoices::Id),
                    )
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        add_doc_date_key(db, "quotations", "date").await?;

        manager
            .create_table(
                Table::create()
                    .table(QuotationLines::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(QuotationLines::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(QuotationLines::QuotationId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(QuotationLines::Position).small_unsigned().not_null())
                    .col(ColumnDef::new(QuotationLines::ProductId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(QuotationLines::Name).string_len(200).not_null())
                    .col(ColumnDef::new(QuotationLines::Qty).decimal_len(19, 4).not_null())
                    .col(ColumnDef::new(QuotationLines::Price).decimal_len(19, 4).not_null())
                    .col(ColumnDef::new(QuotationLines::CostPrice).decimal_len(19, 4).not_null())
                    .col(ColumnDef::new(QuotationLines::Discount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(QuotationLines::TaxId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(QuotationLines::TaxCategory).string_len(32).null())
                    .col(ColumnDef::new(QuotationLines::TaxRate).decimal_len(9, 4).null())
                    .col(ColumnDef::new(QuotationLines::Net).decimal_len(19, 2).null())
                    .col(ColumnDef::new(QuotationLines::Vat).decimal_len(19, 2).null())
                    .col(ColumnDef::new(QuotationLines::UnitId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(QuotationLines::UnitFactor).decimal_len(19, 6).null())
                    .col(ColumnDef::new(QuotationLines::ListPrice).decimal_len(19, 4).null())
                    .col(ColumnDef::new(QuotationLines::PriceOverrideReason).text().null())
                    .col(ColumnDef::new(QuotationLines::BatchId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(QuotationLines::BatchNo).string_len(64).null())
                    .col(ColumnDef::new(QuotationLines::IsFreeText).boolean().not_null().default(false))
                    .col(ColumnDef::new(QuotationLines::RevenueAccountId).custom(Alias::new("UUID")).null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_quotation_lines_quotation_id")
                            .from(QuotationLines::Table, QuotationLines::QuotationId)
                            .to(Quotations::Table, Quotations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(Index::create().name("uq_quotation_lines_quotation_id_position").col(QuotationLines::QuotationId).col(QuotationLines::Position).unique())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        // --- held_sales -----------------------------------------------------------------------
        manager
            .create_table(
                Table::create()
                    .table(HeldSales::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(HeldSales::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(HeldSales::Label).string_len(200).null())
                    .col(ColumnDef::new(HeldSales::TerminalId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(HeldSales::HeldAtDay).date().not_null())
                    .col(ColumnDef::new(HeldSales::HeldAtInstant).custom(Alias::new("DATETIME(3)")).null())
                    .col(ColumnDef::new(HeldSales::HeldBy).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(HeldSales::CustomerId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(HeldSales::DiscountRate).decimal_len(9, 4).not_null())
                    .col(ColumnDef::new(HeldSales::DiscountIsPct).boolean().not_null())
                    .col(ColumnDef::new(HeldSales::Note).text().null())
                    .col(ColumnDef::new(HeldSales::Cart).json().not_null())
                    .col(ColumnDef::new(HeldSales::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(HeldSales::UpdatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(HeldSales::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .index(Index::create().name("ix_held_sales_terminal_id").col(HeldSales::TerminalId))
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        // --- shifts (+ movements) -----------------------------------------------------------------
        manager
            .create_table(
                Table::create()
                    .table(Shifts::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Shifts::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(Shifts::Number).string_len(40).not_null())
                    .col(ColumnDef::new(Shifts::TerminalId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(Shifts::BranchId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Shifts::Status).custom(Alias::new("ENUM('OPEN','CLOSED')")).not_null())
                    .col(ColumnDef::new(Shifts::OpenedBy).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(Shifts::OpenedAtDay).date().not_null())
                    .col(ColumnDef::new(Shifts::OpenedAtInstant).custom(Alias::new("DATETIME(3)")).null())
                    .col(ColumnDef::new(Shifts::OpeningFloat).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Shifts::OpeningDenominations).json().null())
                    .col(ColumnDef::new(Shifts::ClosedBy).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Shifts::ClosedAtDay).date().null())
                    .col(ColumnDef::new(Shifts::ClosedAtInstant).custom(Alias::new("DATETIME(3)")).null())
                    .col(ColumnDef::new(Shifts::CountedCash).decimal_len(19, 2).null())
                    .col(ColumnDef::new(Shifts::ClosingDenominations).json().null())
                    .col(ColumnDef::new(Shifts::ExpectedCash).decimal_len(19, 2).null())
                    .col(ColumnDef::new(Shifts::Variance).decimal_len(19, 2).null())
                    .col(ColumnDef::new(Shifts::HandoverMode).custom(Alias::new("ENUM('HANDOVER','DROP')")).null())
                    .col(ColumnDef::new(Shifts::ForceClosedBy).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Shifts::Note).text().null())
                    .col(ColumnDef::new(Shifts::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(Shifts::UpdatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(Shifts::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .index(Index::create().name("uq_shifts_number").col(Shifts::Number).unique())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        // C-15 partial unique: one OPEN shift per terminal — generated column + unique.
        db.execute_unprepared(
            "ALTER TABLE shifts \
             ADD COLUMN open_key CHAR(36) AS (CASE WHEN status = 'OPEN' THEN terminal_id END) STORED, \
             ADD UNIQUE KEY uq_shifts_open_key (open_key)",
        )
        .await?;

        manager
            .create_table(
                Table::create()
                    .table(ShiftMovements::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ShiftMovements::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(ShiftMovements::ShiftId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(ShiftMovements::Position).small_unsigned().not_null())
                    .col(ColumnDef::new(ShiftMovements::Kind).custom(Alias::new("ENUM('SALE_CASH','REFUND_CASH','PAY_IN','PAY_OUT','BANK_DROP')")).not_null())
                    .col(ColumnDef::new(ShiftMovements::Amount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(ShiftMovements::Note).text().null())
                    .col(ColumnDef::new(ShiftMovements::RefId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(ShiftMovements::RefNumber).string_len(40).null())
                    .col(ColumnDef::new(ShiftMovements::AtDay).date().not_null())
                    .col(ColumnDef::new(ShiftMovements::AtInstant).custom(Alias::new("DATETIME(3)")).null())
                    .col(ColumnDef::new(ShiftMovements::By).custom(Alias::new("UUID")).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_shift_movements_shift_id")
                            .from(ShiftMovements::Table, ShiftMovements::ShiftId)
                            .to(Shifts::Table, Shifts::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(Index::create().name("uq_shift_movements_shift_id_position").col(ShiftMovements::ShiftId).col(ShiftMovements::Position).unique())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(ShiftMovements::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Shifts::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(HeldSales::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(QuotationLines::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Quotations::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(RefundLines::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Refunds::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(InvoiceTenders::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(InvoiceLines::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Invoices::Table).to_owned()).await?;
        Ok(())
    }
}

/// Adds the DocDate `<field>_key` generated column (B1's `entities::doc_date` convention —
/// `COALESCE(<instant as ISO-ms text>, CAST(<day> AS CHAR))`, matching `DocDate::key()`
/// byte-for-byte) on top of the `<field>_day`/`<field>_instant` physical columns this migration
/// already created. The `migration` crate can't depend on the app crate's `entities::doc_date`
/// (that would be a circular dependency — see `lib.rs`'s own note on crate separation), so this is
/// the same raw-SQL shape duplicated here per B2's migration files.
async fn add_doc_date_key<C: sea_orm::ConnectionTrait>(db: &C, table: &str, field: &str) -> Result<(), DbErr> {
    let sql = format!(
        "ALTER TABLE `{table}` ADD COLUMN `{field}_key` VARCHAR(24) AS (COALESCE(\
            CONCAT(REPLACE(LEFT(CAST(`{field}_instant` AS CHAR), 23), ' ', 'T'), 'Z'), \
            CAST(`{field}_day` AS CHAR)\
        )) STORED",
    );
    db.execute_unprepared(&sql).await?;
    Ok(())
}

#[derive(Iden)]
enum Invoices {
    Table,
    Id,
    Number,
    DateDay,
    DateInstant,
    CustomerId,
    CashierId,
    Status,
    PaymentStatus,
    SubTotal,
    DiscountRate,
    DiscountAmount,
    TaxRate,
    TaxAmount,
    GrandTotal,
    PaymentMethod,
    PaidAmount,
    RefundedAmount,
    TenderedAmount,
    DueDate,
    Note,
    Source,
    ShiftId,
    BranchId,
    InvoiceType,
    PoReference,
    Terms,
    AttachmentIds,
    Currency,
    ExchangeRate,
    CostCenterId,
    SearchNormalized,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum InvoiceLines {
    Table,
    Id,
    InvoiceId,
    Position,
    ProductId,
    Name,
    Qty,
    Price,
    CostPrice,
    Discount,
    TaxId,
    TaxCategory,
    TaxRate,
    Net,
    Vat,
    UnitId,
    UnitFactor,
    ListPrice,
    PriceOverrideReason,
    BatchId,
    BatchNo,
    IsFreeText,
    RevenueAccountId,
}

#[derive(Iden)]
enum InvoiceTenders {
    Table,
    Id,
    InvoiceId,
    Position,
    PaymentMethodId,
    Amount,
    Reference,
}

#[derive(Iden)]
enum Refunds {
    Table,
    Id,
    Number,
    InvoiceId,
    DateDay,
    DateInstant,
    Reason,
    SubTotal,
    TaxAmount,
    GrandTotal,
    SettledToReceivable,
    CashBack,
    RefundMethod,
    CreditedToAccount,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum RefundLines {
    Table,
    Id,
    RefundId,
    Position,
    InvoiceLineId,
    Qty,
    Restock,
}

#[derive(Iden)]
enum Quotations {
    Table,
    Id,
    Number,
    DateDay,
    DateInstant,
    ExpiryDate,
    CustomerId,
    SalespersonId,
    Status,
    DiscountRate,
    DiscountAmount,
    TaxAmount,
    SubTotal,
    GrandTotal,
    Note,
    Terms,
    PoReference,
    AttachmentIds,
    ConvertedInvoiceId,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum QuotationLines {
    Table,
    Id,
    QuotationId,
    Position,
    ProductId,
    Name,
    Qty,
    Price,
    CostPrice,
    Discount,
    TaxId,
    TaxCategory,
    TaxRate,
    Net,
    Vat,
    UnitId,
    UnitFactor,
    ListPrice,
    PriceOverrideReason,
    BatchId,
    BatchNo,
    IsFreeText,
    RevenueAccountId,
}

#[derive(Iden)]
enum HeldSales {
    Table,
    Id,
    Label,
    TerminalId,
    HeldAtDay,
    HeldAtInstant,
    HeldBy,
    CustomerId,
    DiscountRate,
    DiscountIsPct,
    Note,
    Cart,
    CreatedAt,
    UpdatedAt,
    SyncStatus,
}

#[derive(Iden)]
enum Shifts {
    Table,
    Id,
    Number,
    TerminalId,
    BranchId,
    Status,
    OpenedBy,
    OpenedAtDay,
    OpenedAtInstant,
    OpeningFloat,
    OpeningDenominations,
    ClosedBy,
    ClosedAtDay,
    ClosedAtInstant,
    CountedCash,
    ClosingDenominations,
    ExpectedCash,
    Variance,
    HandoverMode,
    ForceClosedBy,
    Note,
    CreatedAt,
    UpdatedAt,
    SyncStatus,
}

#[derive(Iden)]
enum ShiftMovements {
    Table,
    Id,
    ShiftId,
    Position,
    Kind,
    Amount,
    Note,
    RefId,
    RefNumber,
    AtDay,
    AtInstant,
    By,
}
