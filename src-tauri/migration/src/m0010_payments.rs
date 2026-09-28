//! m0010_payments (21.02-B, owner B2): payments(+allocations), vouchers (one flat table),
//! card_settlements(+groups). Source types: `src/modules/payments/types/index.ts`,
//! `src/modules/vouchers/types/index.ts`.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // --- payments ---------------------------------------------------------------------------
        manager
            .create_table(
                Table::create()
                    .table(Payments::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Payments::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(Payments::Number).string_len(40).not_null())
                    .col(ColumnDef::new(Payments::DateDay).date().not_null())
                    .col(ColumnDef::new(Payments::DateInstant).timestamp().null())
                    .col(ColumnDef::new(Payments::Type).custom(Alias::new("ENUM('RECEIVED','PAID')")).not_null())
                    .col(ColumnDef::new(Payments::TargetType).custom(Alias::new("ENUM('customer','supplier')")).not_null())
                    .col(ColumnDef::new(Payments::TargetId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(Payments::TargetRef).string_len(64).null())
                    .col(ColumnDef::new(Payments::TargetRefNumber).string_len(40).null())
                    .col(ColumnDef::new(Payments::Amount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Payments::Method).custom(Alias::new("ENUM('cash','card','bank_transfer')")).not_null())
                    .col(ColumnDef::new(Payments::Note).text().null())
                    .col(ColumnDef::new(Payments::BranchId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Payments::Currency).char_len(3).null())
                    .col(ColumnDef::new(Payments::AmountFc).decimal_len(19, 2).null())
                    .col(ColumnDef::new(Payments::Rate).decimal_len(19, 6).null())
                    .col(ColumnDef::new(Payments::FxGainLoss).decimal_len(19, 2).null())
                    .col(ColumnDef::new(Payments::CreatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(Payments::UpdatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(Payments::DeletedAt).timestamp().null())
                    .col(ColumnDef::new(Payments::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .index(Index::create().name("uq_payments_number").col(Payments::Number).unique())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        add_doc_date_key(db, "payments", "date").await?;

        manager
            .create_table(
                Table::create()
                    .table(PaymentAllocations::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PaymentAllocations::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(PaymentAllocations::PaymentId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(PaymentAllocations::Position).small_unsigned().not_null())
                    .col(ColumnDef::new(PaymentAllocations::TargetKind).custom(Alias::new("ENUM('invoice','purchaseOrder','opening')")).not_null())
                    // Polymorphic ref (B-1): no FK on target_id.
                    .col(ColumnDef::new(PaymentAllocations::TargetId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(PaymentAllocations::TargetNumber).string_len(40).not_null())
                    .col(ColumnDef::new(PaymentAllocations::Amount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(PaymentAllocations::DateDay).date().not_null())
                    .col(ColumnDef::new(PaymentAllocations::DateInstant).timestamp().null())
                    .col(ColumnDef::new(PaymentAllocations::AmountFc).decimal_len(19, 2).null())
                    .col(ColumnDef::new(PaymentAllocations::FxGainLoss).decimal_len(19, 2).null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_payment_allocations_payment_id")
                            .from(PaymentAllocations::Table, PaymentAllocations::PaymentId)
                            .to(Payments::Table, Payments::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(Index::create().name("uq_payment_allocations_payment_id_position").col(PaymentAllocations::PaymentId).col(PaymentAllocations::Position).unique())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        add_doc_date_key(db, "payment_allocations", "date").await?;

        // --- vouchers (one flat table, nullable kind-specific columns) ---------------------------
        manager
            .create_table(
                Table::create()
                    .table(Vouchers::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Vouchers::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(Vouchers::Number).string_len(40).not_null())
                    .col(ColumnDef::new(Vouchers::Kind).custom(Alias::new("ENUM('RECEIPT','PAYMENT','TRANSFER','OWNER')")).not_null())
                    .col(ColumnDef::new(Vouchers::DateDay).date().not_null())
                    .col(ColumnDef::new(Vouchers::DateInstant).timestamp().null())
                    .col(ColumnDef::new(Vouchers::Amount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Vouchers::Description).text().not_null())
                    .col(ColumnDef::new(Vouchers::Note).text().null())
                    .col(ColumnDef::new(Vouchers::AttachmentIds).json().null())
                    .col(ColumnDef::new(Vouchers::CostCenterId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Vouchers::CreatedBy).custom(Alias::new("UUID")).not_null())
                    // RECEIPT
                    .col(ColumnDef::new(Vouchers::PaymentMethodId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Vouchers::CreditAccountId).custom(Alias::new("UUID")).null())
                    // PAYMENT
                    .col(ColumnDef::new(Vouchers::DebitAccountId).custom(Alias::new("UUID")).null())
                    // TRANSFER
                    .col(ColumnDef::new(Vouchers::SourceAccountId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Vouchers::DestinationAccountId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Vouchers::FeeAmount).decimal_len(19, 2).null())
                    .col(ColumnDef::new(Vouchers::FeeAccountId).custom(Alias::new("UUID")).null())
                    // OWNER
                    .col(ColumnDef::new(Vouchers::Direction).custom(Alias::new("ENUM('drawings','contribution')")).null())
                    .col(ColumnDef::new(Vouchers::CashAccountId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Vouchers::SearchNormalized).text().null())
                    .col(ColumnDef::new(Vouchers::CreatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(Vouchers::UpdatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(Vouchers::DeletedAt).timestamp().null())
                    .col(ColumnDef::new(Vouchers::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .index(Index::create().name("uq_vouchers_number").col(Vouchers::Number).unique())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        add_doc_date_key(db, "vouchers", "date").await?;

        // --- card_settlements (+groups) -----------------------------------------------------------
        manager
            .create_table(
                Table::create()
                    .table(CardSettlements::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(CardSettlements::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(CardSettlements::Number).string_len(40).not_null())
                    .col(ColumnDef::new(CardSettlements::DateDay).date().not_null())
                    .col(ColumnDef::new(CardSettlements::DateInstant).timestamp().null())
                    .col(ColumnDef::new(CardSettlements::GrossAmount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(CardSettlements::DepositAmount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(CardSettlements::FeeAmount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(CardSettlements::Note).text().null())
                    .col(ColumnDef::new(CardSettlements::CreatedBy).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(CardSettlements::CreatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(CardSettlements::UpdatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(CardSettlements::DeletedAt).timestamp().null())
                    .col(ColumnDef::new(CardSettlements::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .index(Index::create().name("uq_card_settlements_number").col(CardSettlements::Number).unique())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        add_doc_date_key(db, "card_settlements", "date").await?;

        manager
            .create_table(
                Table::create()
                    .table(CardSettlementGroups::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(CardSettlementGroups::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(CardSettlementGroups::CardSettlementId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(CardSettlementGroups::Position).small_unsigned().not_null())
                    .col(ColumnDef::new(CardSettlementGroups::DateDay).date().not_null())
                    .col(ColumnDef::new(CardSettlementGroups::PaymentMethodId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(CardSettlementGroups::Amount).decimal_len(19, 2).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_card_settlement_groups_card_settlement_id")
                            .from(CardSettlementGroups::Table, CardSettlementGroups::CardSettlementId)
                            .to(CardSettlements::Table, CardSettlements::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(Index::create().name("uq_card_settlement_groups_cs_id_position").col(CardSettlementGroups::CardSettlementId).col(CardSettlementGroups::Position).unique())
                    // Concurrency guard (vouchers.md): a given (date, payment_method) can only be
                    // claimed by one settlement group — prevents two terminals settling the same day×method twice.
                    .index(
                        Index::create()
                            .name("uq_card_settlement_groups_date_method")
                            .col(CardSettlementGroups::DateDay)
                            .col(CardSettlementGroups::PaymentMethodId)
                            .unique(),
                    )
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(CardSettlementGroups::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(CardSettlements::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Vouchers::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PaymentAllocations::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Payments::Table).to_owned()).await?;
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
enum Payments {
    Table,
    Id,
    Number,
    DateDay,
    DateInstant,
    Type,
    TargetType,
    TargetId,
    TargetRef,
    TargetRefNumber,
    Amount,
    Method,
    Note,
    BranchId,
    Currency,
    AmountFc,
    Rate,
    FxGainLoss,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum PaymentAllocations {
    Table,
    Id,
    PaymentId,
    Position,
    TargetKind,
    TargetId,
    TargetNumber,
    Amount,
    DateDay,
    DateInstant,
    AmountFc,
    FxGainLoss,
}

#[derive(Iden)]
enum Vouchers {
    Table,
    Id,
    Number,
    Kind,
    DateDay,
    DateInstant,
    Amount,
    Description,
    Note,
    AttachmentIds,
    CostCenterId,
    CreatedBy,
    PaymentMethodId,
    CreditAccountId,
    DebitAccountId,
    SourceAccountId,
    DestinationAccountId,
    FeeAmount,
    FeeAccountId,
    Direction,
    CashAccountId,
    SearchNormalized,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum CardSettlements {
    Table,
    Id,
    Number,
    DateDay,
    DateInstant,
    GrossAmount,
    DepositAmount,
    FeeAmount,
    Note,
    CreatedBy,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum CardSettlementGroups {
    Table,
    Id,
    CardSettlementId,
    Position,
    DateDay,
    PaymentMethodId,
    Amount,
}
