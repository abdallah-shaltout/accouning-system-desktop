//! m0009_purchases (21.02-B, owner B2): purchase_orders(+lines), purchase_returns(+lines).
//! Source types: `src/modules/purchases/types/index.ts`.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        manager
            .create_table(
                Table::create()
                    .table(PurchaseOrders::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PurchaseOrders::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(PurchaseOrders::Number).string_len(40).not_null())
                    .col(ColumnDef::new(PurchaseOrders::SupplierId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(PurchaseOrders::DateDay).date().not_null())
                    .col(ColumnDef::new(PurchaseOrders::DateInstant).timestamp().null())
                    .col(ColumnDef::new(PurchaseOrders::Status).custom(Alias::new("ENUM('DRAFT','ORDERED','RECEIVED','CANCELED')")).not_null())
                    .col(ColumnDef::new(PurchaseOrders::SubTotal).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(PurchaseOrders::TaxRate).decimal_len(9, 4).not_null())
                    .col(ColumnDef::new(PurchaseOrders::TaxAmount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(PurchaseOrders::GrandTotal).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(PurchaseOrders::PaymentStatus).custom(Alias::new("ENUM('UNPAID','PARTIALLY_PAID','PAID')")).not_null())
                    .col(ColumnDef::new(PurchaseOrders::PaidAmount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(PurchaseOrders::ReturnedAmount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(PurchaseOrders::Note).text().null())
                    .col(ColumnDef::new(PurchaseOrders::InvoiceDiscountPct).decimal_len(9, 4).null())
                    .col(ColumnDef::new(PurchaseOrders::InvoiceDiscountAmount).decimal_len(19, 2).null())
                    .col(ColumnDef::new(PurchaseOrders::LandedCosts).json().null())
                    .col(ColumnDef::new(PurchaseOrders::SupplierInvoiceNo).string_len(64).null())
                    .col(ColumnDef::new(PurchaseOrders::SupplierInvoiceDate).date().null())
                    .col(ColumnDef::new(PurchaseOrders::VatNotRecoverable).boolean().null())
                    .col(ColumnDef::new(PurchaseOrders::SentAt).timestamp().null())
                    .col(ColumnDef::new(PurchaseOrders::BackorderOfId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(PurchaseOrders::ReceivedDate).date().null())
                    .col(ColumnDef::new(PurchaseOrders::AttachmentIds).json().null())
                    .col(ColumnDef::new(PurchaseOrders::CostCenterId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(PurchaseOrders::BranchId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(PurchaseOrders::Currency).char_len(3).null())
                    .col(ColumnDef::new(PurchaseOrders::ExchangeRate).decimal_len(19, 6).null())
                    .col(ColumnDef::new(PurchaseOrders::CreatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(PurchaseOrders::UpdatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(PurchaseOrders::DeletedAt).timestamp().null())
                    .col(ColumnDef::new(PurchaseOrders::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .index(Index::create().name("uq_purchase_orders_number").col(PurchaseOrders::Number).unique())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_purchase_orders_backorder_of_id")
                            .from(PurchaseOrders::Table, PurchaseOrders::BackorderOfId)
                            .to(PurchaseOrders::Table, PurchaseOrders::Id),
                    )
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        add_doc_date_key(db, "purchase_orders", "date").await?;

        manager
            .create_table(
                Table::create()
                    .table(PurchaseOrderLines::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PurchaseOrderLines::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(PurchaseOrderLines::PurchaseOrderId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(PurchaseOrderLines::Position).small_unsigned().not_null())
                    .col(ColumnDef::new(PurchaseOrderLines::ProductId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(PurchaseOrderLines::Qty).decimal_len(19, 4).not_null())
                    .col(ColumnDef::new(PurchaseOrderLines::CostPrice).decimal_len(19, 4).not_null())
                    .col(ColumnDef::new(PurchaseOrderLines::UnitId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(PurchaseOrderLines::UnitFactor).decimal_len(19, 6).null())
                    .col(ColumnDef::new(PurchaseOrderLines::Discount).decimal_len(19, 2).null())
                    .col(ColumnDef::new(PurchaseOrderLines::DiscountIsPct).boolean().null())
                    .col(ColumnDef::new(PurchaseOrderLines::TaxId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(PurchaseOrderLines::ReceivedQty).decimal_len(19, 4).null())
                    .col(ColumnDef::new(PurchaseOrderLines::BatchNo).string_len(64).null())
                    .col(ColumnDef::new(PurchaseOrderLines::ExpiryDate).date().null())
                    .col(ColumnDef::new(PurchaseOrderLines::LandedCostShare).decimal_len(19, 2).null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_purchase_order_lines_purchase_order_id")
                            .from(PurchaseOrderLines::Table, PurchaseOrderLines::PurchaseOrderId)
                            .to(PurchaseOrders::Table, PurchaseOrders::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(Index::create().name("uq_purchase_order_lines_po_id_position").col(PurchaseOrderLines::PurchaseOrderId).col(PurchaseOrderLines::Position).unique())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PurchaseReturns::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PurchaseReturns::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(PurchaseReturns::Number).string_len(40).not_null())
                    .col(ColumnDef::new(PurchaseReturns::PurchaseOrderId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(PurchaseReturns::SupplierId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(PurchaseReturns::DateDay).date().not_null())
                    .col(ColumnDef::new(PurchaseReturns::DateInstant).timestamp().null())
                    .col(ColumnDef::new(PurchaseReturns::Reason).text().null())
                    .col(ColumnDef::new(PurchaseReturns::SubTotal).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(PurchaseReturns::TaxAmount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(PurchaseReturns::GrandTotal).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(PurchaseReturns::SettledToPayable).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(PurchaseReturns::CashBack).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(PurchaseReturns::RefundMethod).custom(Alias::new("ENUM('cash','bank_transfer','credit')")).not_null())
                    .col(ColumnDef::new(PurchaseReturns::FromDraftId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(PurchaseReturns::CreatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(PurchaseReturns::UpdatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(PurchaseReturns::DeletedAt).timestamp().null())
                    .col(ColumnDef::new(PurchaseReturns::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .index(Index::create().name("uq_purchase_returns_number").col(PurchaseReturns::Number).unique())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_purchase_returns_purchase_order_id")
                            .from(PurchaseReturns::Table, PurchaseReturns::PurchaseOrderId)
                            .to(PurchaseOrders::Table, PurchaseOrders::Id),
                    )
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        add_doc_date_key(db, "purchase_returns", "date").await?;

        manager
            .create_table(
                Table::create()
                    .table(PurchaseReturnLines::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PurchaseReturnLines::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(PurchaseReturnLines::PurchaseReturnId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(PurchaseReturnLines::Position).small_unsigned().not_null())
                    .col(ColumnDef::new(PurchaseReturnLines::ProductId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(PurchaseReturnLines::Qty).decimal_len(19, 4).not_null())
                    .col(ColumnDef::new(PurchaseReturnLines::CostPrice).decimal_len(19, 4).not_null())
                    .col(ColumnDef::new(PurchaseReturnLines::BatchId).custom(Alias::new("UUID")).null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_purchase_return_lines_purchase_return_id")
                            .from(PurchaseReturnLines::Table, PurchaseReturnLines::PurchaseReturnId)
                            .to(PurchaseReturns::Table, PurchaseReturns::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(Index::create().name("uq_purchase_return_lines_pr_id_position").col(PurchaseReturnLines::PurchaseReturnId).col(PurchaseReturnLines::Position).unique())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(PurchaseReturnLines::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PurchaseReturns::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PurchaseOrderLines::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PurchaseOrders::Table).to_owned()).await?;
        Ok(())
    }
}

/// See `m0008_sales.rs`'s identical helper doc comment — duplicated per-file since `migration`
/// can't depend on the app crate's `entities::doc_date`.
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
enum PurchaseOrders {
    Table,
    Id,
    Number,
    SupplierId,
    DateDay,
    DateInstant,
    Status,
    SubTotal,
    TaxRate,
    TaxAmount,
    GrandTotal,
    PaymentStatus,
    PaidAmount,
    ReturnedAmount,
    Note,
    InvoiceDiscountPct,
    InvoiceDiscountAmount,
    LandedCosts,
    SupplierInvoiceNo,
    SupplierInvoiceDate,
    VatNotRecoverable,
    SentAt,
    BackorderOfId,
    ReceivedDate,
    AttachmentIds,
    CostCenterId,
    BranchId,
    Currency,
    ExchangeRate,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum PurchaseOrderLines {
    Table,
    Id,
    PurchaseOrderId,
    Position,
    ProductId,
    Qty,
    CostPrice,
    UnitId,
    UnitFactor,
    Discount,
    DiscountIsPct,
    TaxId,
    ReceivedQty,
    BatchNo,
    ExpiryDate,
    LandedCostShare,
}

#[derive(Iden)]
enum PurchaseReturns {
    Table,
    Id,
    Number,
    PurchaseOrderId,
    SupplierId,
    DateDay,
    DateInstant,
    Reason,
    SubTotal,
    TaxAmount,
    GrandTotal,
    SettledToPayable,
    CashBack,
    RefundMethod,
    FromDraftId,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum PurchaseReturnLines {
    Table,
    Id,
    PurchaseReturnId,
    Position,
    ProductId,
    Qty,
    CostPrice,
    BatchId,
}
