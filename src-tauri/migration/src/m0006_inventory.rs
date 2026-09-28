//! m0006_inventory (21.02-B, owner B1): `stock_adjustments` (+lines), `stock_movements`,
//! `stock_counts` (+lines), `debit_note_drafts`, `stock_transfers` (+lines).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// The DocDate triple bridge (P2-09 / C-09), local to this migration file: the `migration` crate
/// has no dependency on the app crate (`lib.rs`'s own doc comment: the app depends on `migration`,
/// never the reverse), so this can't call `crate::entities::doc_date` from the app crate — the
/// three-column shape is duplicated here byte-for-byte instead of shared. Keep any change to the
/// generated-column SQL in sync with `src/entities/doc_date.rs`'s `generated_key_extra`.
mod doc_date_migration {
    use sea_orm_migration::prelude::*;

    pub fn day_column(field: &str) -> String {
        format!("{field}_day")
    }

    pub fn instant_column(field: &str) -> String {
        format!("{field}_instant")
    }

    pub fn key_column(field: &str) -> String {
        format!("{field}_key")
    }

    pub fn generated_key_extra(field: &str) -> String {
        let day = day_column(field);
        let instant = instant_column(field);
        format!(
            "GENERATED ALWAYS AS (COALESCE(\
                CONCAT(REPLACE(LEFT(CAST(`{instant}` AS CHAR), 23), ' ', 'T'), 'Z'), \
                CAST(`{day}` AS CHAR)\
            )) STORED"
        )
    }

    pub fn add_doc_date_columns(table: &mut TableCreateStatement, field: &str) {
        table
            .col(ColumnDef::new(Alias::new(day_column(field))).date().not_null())
            .col(ColumnDef::new(Alias::new(instant_column(field))).custom(Alias::new("DATETIME(3)")).null())
            .col(
                // No NULL/NOT NULL spec: MariaDB rejects either on a generated column (error 1064);
                // the value is non-NULL whenever `_day` is, which is itself NOT NULL.
                ColumnDef::new(Alias::new(key_column(field)))
                    .string_len(24)
                    .extra(generated_key_extra(field)),
            );
    }

    /// Nullable variant: for a DocDate field that may not have happened yet (e.g. `stock_transfers`
    /// .`sent_at`/`received_at`/`rejected_at` — a DRAFT transfer has none of the three). `day` is
    /// nullable here (unlike the base convention's `NOT NULL`), so the generated `_key` column's
    /// `COALESCE` also falls through to plain `NULL` when both source columns are NULL.
    pub fn add_nullable_doc_date_columns(table: &mut TableCreateStatement, field: &str) {
        let day = day_column(field);
        let instant = instant_column(field);
        let extra = format!(
            "GENERATED ALWAYS AS (COALESCE(\
                CONCAT(REPLACE(LEFT(CAST(`{instant}` AS CHAR), 23), ' ', 'T'), 'Z'), \
                CAST(`{day}` AS CHAR)\
            )) STORED"
        );
        table
            .col(ColumnDef::new(Alias::new(day)).date().null())
            .col(ColumnDef::new(Alias::new(instant)).custom(Alias::new("DATETIME(3)")).null())
            .col(ColumnDef::new(Alias::new(key_column(field))).string_len(24).extra(extra));
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_stock_adjustments(manager).await?;
        create_stock_adjustment_lines(manager).await?;
        create_stock_movements(manager).await?;
        create_stock_counts(manager).await?;
        create_stock_count_lines(manager).await?;
        create_debit_note_drafts(manager).await?;
        create_stock_transfers(manager).await?;
        create_stock_transfer_lines(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(StockTransferLines::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(StockTransfers::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(DebitNoteDrafts::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(StockCountLines::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(StockCounts::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(StockMovements::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(StockAdjustmentLines::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(StockAdjustments::Table).to_owned()).await?;
        Ok(())
    }
}

// --- stock_adjustments (products/types `StockAdjustment`) -------------------------------------------

async fn create_stock_adjustments(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let mut table = Table::create();
    table
        .table(StockAdjustments::Table)
        .if_not_exists()
        .col(ColumnDef::new(StockAdjustments::Id).custom(Alias::new("UUID")).not_null().primary_key())
        .col(ColumnDef::new(StockAdjustments::Number).string_len(40).not_null())
        .col(ColumnDef::new(StockAdjustments::Type).custom(Alias::new("ENUM('STOCK_IN','LOSS','STOCKTAKE')")).not_null());
    doc_date_migration::add_doc_date_columns(&mut table, "date");
    table
        .col(ColumnDef::new(StockAdjustments::Status).custom(Alias::new("ENUM('DRAFT','COMPLETED')")).not_null())
        .col(ColumnDef::new(StockAdjustments::Note).text().null())
        .col(ColumnDef::new(StockAdjustments::Reason).custom(Alias::new("ENUM('opening','owner_contribution','gift','found','other')")).null())
        .col(ColumnDef::new(StockAdjustments::OffsetAccountId).custom(Alias::new("UUID")).null())
        .col(ColumnDef::new(StockAdjustments::ApprovedBy).custom(Alias::new("UUID")).null())
        .col(ColumnDef::new(StockAdjustments::ApprovedAt).custom(Alias::new("DATETIME(3)")).null())
        .col(ColumnDef::new(StockAdjustments::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
        .col(
            ColumnDef::new(StockAdjustments::UpdatedAt)
                .custom(Alias::new("DATETIME(3)"))
                .not_null()
                .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
        )
        .col(ColumnDef::new(StockAdjustments::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
        .col(ColumnDef::new(StockAdjustments::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
        .index(Index::create().name("uq_stock_adjustments_number").col(StockAdjustments::Number).unique())
        .engine("InnoDB")
        .character_set("utf8mb4")
        .collate("utf8mb4_unicode_ci");
    manager.create_table(table.to_owned()).await
}

async fn create_stock_adjustment_lines(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(StockAdjustmentLines::Table)
                .if_not_exists()
                .col(ColumnDef::new(StockAdjustmentLines::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(StockAdjustmentLines::StockAdjustmentId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(StockAdjustmentLines::Position).small_unsigned().not_null())
                .col(ColumnDef::new(StockAdjustmentLines::ProductId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(StockAdjustmentLines::SystemQty).decimal_len(19, 4).null())
                .col(ColumnDef::new(StockAdjustmentLines::CountedQty).decimal_len(19, 4).null())
                .col(ColumnDef::new(StockAdjustmentLines::QtyChange).decimal_len(19, 4).not_null())
                .col(ColumnDef::new(StockAdjustmentLines::UnitCost).decimal_len(19, 4).null())
                .col(ColumnDef::new(StockAdjustmentLines::BatchNo).string_len(64).null())
                .col(ColumnDef::new(StockAdjustmentLines::ExpiryDate).date().null())
                .index(
                    Index::create()
                        .name("uq_stock_adjustment_lines_parent_position")
                        .col(StockAdjustmentLines::StockAdjustmentId)
                        .col(StockAdjustmentLines::Position)
                        .unique(),
                )
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- stock_movements (products/types `StockMovement`, append-only) ---------------------------------

async fn create_stock_movements(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let mut table = Table::create();
    table
        .table(StockMovements::Table)
        .if_not_exists()
        .col(ColumnDef::new(StockMovements::Id).custom(Alias::new("UUID")).not_null().primary_key());
    doc_date_migration::add_doc_date_columns(&mut table, "date");
    table
        .col(ColumnDef::new(StockMovements::ProductId).custom(Alias::new("UUID")).not_null())
        .col(ColumnDef::new(StockMovements::QtyChange).decimal_len(19, 4).not_null())
        .col(ColumnDef::new(StockMovements::ValueChange).decimal_len(19, 2).not_null())
        .col(
            ColumnDef::new(StockMovements::Reason)
                .custom(Alias::new(
                    "ENUM('sale','purchase','stock_in','loss','stocktake','refund','purchase_return','transfer_out','transfer_in')"
                ))
                .not_null(),
        )
        // Polymorphic ref — no FK (B-1).
        .col(ColumnDef::new(StockMovements::RefId).custom(Alias::new("UUID")).not_null())
        .col(ColumnDef::new(StockMovements::RefNumber).string_len(40).null())
        .col(ColumnDef::new(StockMovements::BalanceAfter).decimal_len(19, 4).null())
        .col(ColumnDef::new(StockMovements::BatchId).custom(Alias::new("UUID")).null())
        .col(ColumnDef::new(StockMovements::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
        .index(Index::create().name("ix_stock_movements_product_id").col(StockMovements::ProductId))
        .index(Index::create().name("ix_stock_movements_ref_id").col(StockMovements::RefId))
        .engine("InnoDB")
        .character_set("utf8mb4")
        .collate("utf8mb4_unicode_ci");
    manager.create_table(table.to_owned()).await
}

// --- stock_counts (products/types `StockCount`) -----------------------------------------------------

async fn create_stock_counts(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(StockCounts::Table)
                .if_not_exists()
                .col(ColumnDef::new(StockCounts::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(StockCounts::Number).string_len(40).not_null())
                .col(ColumnDef::new(StockCounts::Status).custom(Alias::new("ENUM('OPEN','REVIEW','COMPLETED')")).not_null())
                .col(ColumnDef::new(StockCounts::Scope).custom(Alias::new("ENUM('all','category','location')")).not_null())
                .col(ColumnDef::new(StockCounts::CategoryId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(StockCounts::Location).string_len(200).null())
                .col(ColumnDef::new(StockCounts::Blind).boolean().not_null().default(false))
                .col(ColumnDef::new(StockCounts::StartedAt).custom(Alias::new("DATETIME(3)")).not_null())
                .col(ColumnDef::new(StockCounts::StartedBy).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(StockCounts::Note).text().null())
                .col(ColumnDef::new(StockCounts::AdjustmentId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(StockCounts::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(StockCounts::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .index(Index::create().name("uq_stock_counts_number").col(StockCounts::Number).unique())
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

async fn create_stock_count_lines(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(StockCountLines::Table)
                .if_not_exists()
                .col(ColumnDef::new(StockCountLines::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(StockCountLines::StockCountId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(StockCountLines::Position).small_unsigned().not_null())
                .col(ColumnDef::new(StockCountLines::ProductId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(StockCountLines::SystemQty).decimal_len(19, 4).not_null())
                .col(ColumnDef::new(StockCountLines::CountedQty).decimal_len(19, 4).null())
                .col(ColumnDef::new(StockCountLines::UnitCost).decimal_len(19, 4).not_null())
                .index(
                    Index::create()
                        .name("uq_stock_count_lines_parent_position")
                        .col(StockCountLines::StockCountId)
                        .col(StockCountLines::Position)
                        .unique(),
                )
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- debit_note_drafts (products/types `DebitNoteDraft`, hard-deletable worklist row) ---------------

async fn create_debit_note_drafts(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let mut table = Table::create();
    table
        .table(DebitNoteDrafts::Table)
        .if_not_exists()
        .col(ColumnDef::new(DebitNoteDrafts::Id).custom(Alias::new("UUID")).not_null().primary_key())
        .col(ColumnDef::new(DebitNoteDrafts::Number).string_len(40).not_null())
        .col(ColumnDef::new(DebitNoteDrafts::SupplierId).custom(Alias::new("UUID")).not_null());
    doc_date_migration::add_doc_date_columns(&mut table, "date");
    table
        .col(ColumnDef::new(DebitNoteDrafts::Status).custom(Alias::new("ENUM('DRAFT')")).not_null().default("DRAFT"))
        .col(ColumnDef::new(DebitNoteDrafts::Lines).json().not_null())
        .col(ColumnDef::new(DebitNoteDrafts::Note).text().null())
        .col(ColumnDef::new(DebitNoteDrafts::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
        .col(
            ColumnDef::new(DebitNoteDrafts::UpdatedAt)
                .custom(Alias::new("DATETIME(3)"))
                .not_null()
                .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
        )
        .index(Index::create().name("uq_debit_note_drafts_number").col(DebitNoteDrafts::Number).unique())
        .engine("InnoDB")
        .character_set("utf8mb4")
        .collate("utf8mb4_unicode_ci");
    manager.create_table(table.to_owned()).await
}

// --- stock_transfers (products/types `StockTransfer`) -----------------------------------------------

async fn create_stock_transfers(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let mut table = Table::create();
    table
        .table(StockTransfers::Table)
        .if_not_exists()
        .col(ColumnDef::new(StockTransfers::Id).custom(Alias::new("UUID")).not_null().primary_key())
        .col(ColumnDef::new(StockTransfers::Number).string_len(40).not_null())
        .col(ColumnDef::new(StockTransfers::FromBranchId).custom(Alias::new("UUID")).not_null())
        .col(ColumnDef::new(StockTransfers::ToBranchId).custom(Alias::new("UUID")).not_null())
        .col(ColumnDef::new(StockTransfers::Status).custom(Alias::new("ENUM('DRAFT','SENT','RECEIVED','REJECTED')")).not_null());
    doc_date_migration::add_doc_date_columns(&mut table, "date");
    doc_date_migration::add_nullable_doc_date_columns(&mut table, "sent_at");
    doc_date_migration::add_nullable_doc_date_columns(&mut table, "received_at");
    doc_date_migration::add_nullable_doc_date_columns(&mut table, "rejected_at");
    table
        .col(ColumnDef::new(StockTransfers::Note).text().null())
        .col(ColumnDef::new(StockTransfers::SentBy).custom(Alias::new("UUID")).null())
        .col(ColumnDef::new(StockTransfers::ReceivedBy).custom(Alias::new("UUID")).null())
        .col(ColumnDef::new(StockTransfers::RejectedBy).custom(Alias::new("UUID")).null())
        .col(ColumnDef::new(StockTransfers::RejectReason).text().null())
        .col(ColumnDef::new(StockTransfers::ShortageValue).decimal_len(19, 2).null())
        .col(ColumnDef::new(StockTransfers::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
        .col(
            ColumnDef::new(StockTransfers::UpdatedAt)
                .custom(Alias::new("DATETIME(3)"))
                .not_null()
                .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
        )
        .col(ColumnDef::new(StockTransfers::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
        .index(Index::create().name("uq_stock_transfers_number").col(StockTransfers::Number).unique())
        .engine("InnoDB")
        .character_set("utf8mb4")
        .collate("utf8mb4_unicode_ci");
    manager.create_table(table.to_owned()).await
}

async fn create_stock_transfer_lines(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(StockTransferLines::Table)
                .if_not_exists()
                .col(ColumnDef::new(StockTransferLines::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(StockTransferLines::StockTransferId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(StockTransferLines::Position).small_unsigned().not_null())
                .col(ColumnDef::new(StockTransferLines::ProductId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(StockTransferLines::Qty).decimal_len(19, 4).not_null())
                .col(ColumnDef::new(StockTransferLines::UnitId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(StockTransferLines::UnitFactor).decimal_len(19, 6).null())
                .col(ColumnDef::new(StockTransferLines::BatchId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(StockTransferLines::BatchNo).string_len(64).null())
                .col(ColumnDef::new(StockTransferLines::ReceivedQty).decimal_len(19, 4).null())
                .col(ColumnDef::new(StockTransferLines::UnitCost).decimal_len(19, 4).null())
                .index(
                    Index::create()
                        .name("uq_stock_transfer_lines_parent_position")
                        .col(StockTransferLines::StockTransferId)
                        .col(StockTransferLines::Position)
                        .unique(),
                )
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- Idens -----------------------------------------------------------------------------------------

#[derive(Iden)]
enum StockAdjustments {
    Table,
    Id,
    Number,
    Type,
    Status,
    Note,
    Reason,
    OffsetAccountId,
    ApprovedBy,
    ApprovedAt,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum StockAdjustmentLines {
    Table,
    Id,
    StockAdjustmentId,
    Position,
    ProductId,
    SystemQty,
    CountedQty,
    QtyChange,
    UnitCost,
    BatchNo,
    ExpiryDate,
}

#[derive(Iden)]
enum StockMovements {
    Table,
    Id,
    ProductId,
    QtyChange,
    ValueChange,
    Reason,
    RefId,
    RefNumber,
    BalanceAfter,
    BatchId,
    CreatedAt,
}

#[derive(Iden)]
enum StockCounts {
    Table,
    Id,
    Number,
    Status,
    Scope,
    CategoryId,
    Location,
    Blind,
    StartedAt,
    StartedBy,
    Note,
    AdjustmentId,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum StockCountLines {
    Table,
    Id,
    StockCountId,
    Position,
    ProductId,
    SystemQty,
    CountedQty,
    UnitCost,
}

#[derive(Iden)]
enum DebitNoteDrafts {
    Table,
    Id,
    Number,
    SupplierId,
    Status,
    Lines,
    Note,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum StockTransfers {
    Table,
    Id,
    Number,
    FromBranchId,
    ToBranchId,
    Status,
    Note,
    SentBy,
    ReceivedBy,
    RejectedBy,
    RejectReason,
    ShortageValue,
    CreatedAt,
    UpdatedAt,
    SyncStatus,
}

#[derive(Iden)]
enum StockTransferLines {
    Table,
    Id,
    StockTransferId,
    Position,
    ProductId,
    Qty,
    UnitId,
    UnitFactor,
    BatchId,
    BatchNo,
    ReceivedQty,
    UnitCost,
}
