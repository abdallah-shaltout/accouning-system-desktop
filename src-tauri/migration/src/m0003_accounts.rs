//! m0003_accounts (21.02-B, owner B1): `accounts` (accounting/types `Account`).
//!
//! No FK here for `parent_id` (self-referential — added in `m0015_foreign_keys` per B-1 so the
//! create order never matters). No uniqueness on `system_role` (C-15: several accounts may share a
//! role, e.g. cash/bank, per the TS type's own doc comment) and no uniqueness on `code` beyond what
//! the app enforces at the controller layer — the mock never declares a DB-level unique on account
//! code, and codes intentionally nest by prefix (children start with the parent's code), so a
//! generated-column uniqueness rule isn't a good fit here.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Accounts::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Accounts::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(Accounts::Code).string_len(64).not_null())
                    .col(ColumnDef::new(Accounts::Name).string_len(200).not_null())
                    .col(ColumnDef::new(Accounts::NameEn).string_len(200).null())
                    .col(ColumnDef::new(Accounts::ParentId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Accounts::IsGroup).boolean().not_null().default(false))
                    .col(ColumnDef::new(Accounts::Kind).custom(Alias::new("ENUM('ASSET','LIABILITY','EQUITY','REVENUE','EXPENSE')")).not_null())
                    .col(
                        ColumnDef::new(Accounts::Subtype)
                            .custom(Alias::new(
                                "ENUM('cash','bank','clearing','receivable','payable','inventory','tax','prepaid','otherCurrentAsset',\
                                 'fixedAsset','accumulatedDepreciation','currentLiability','longTermLiability','equity','revenue',\
                                 'otherIncome','costOfSales','operatingExpense','otherExpense','zakatTax')"
                            ))
                            .not_null(),
                    )
                    .col(ColumnDef::new(Accounts::NormalSide).custom(Alias::new("ENUM('DEBIT','CREDIT')")).not_null())
                    .col(
                        ColumnDef::new(Accounts::SystemRole)
                            .custom(Alias::new(
                                "ENUM('cash','bank','cardClearing','walletClearing','receivable','inventory','inventoryInTransit',\
                                 'vatInput','payable','vatOutput','vatPayable','customerAdvances','capital','ownerCurrent','drawings',\
                                 'retainedEarnings','currentEarnings','openingBalanceEquity','sales','serviceRevenue','salesReturns',\
                                 'otherIncome','fxGain','cashOver','purchaseDiscounts','cogs','inventoryVariance','inventoryWriteOff',\
                                 'freightIn','cardFees','bankFees','fxLoss','cashShort','badDebt','depreciation','zakat')"
                            ))
                            .null(),
                    )
                    .col(ColumnDef::new(Accounts::Currency).char_len(3).null())
                    .col(ColumnDef::new(Accounts::BranchId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Accounts::RequiresParty).boolean().null())
                    .col(ColumnDef::new(Accounts::AllowManual).boolean().not_null().default(true))
                    .col(ColumnDef::new(Accounts::RequiresCostCenter).boolean().null())
                    .col(ColumnDef::new(Accounts::Active).boolean().not_null().default(true))
                    .col(ColumnDef::new(Accounts::CanDelete).boolean().not_null().default(true))
                    .col(ColumnDef::new(Accounts::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(
                        ColumnDef::new(Accounts::UpdatedAt)
                            .custom(Alias::new("DATETIME(3)"))
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                    )
                    .col(ColumnDef::new(Accounts::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                    .col(ColumnDef::new(Accounts::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .index(Index::create().name("ix_accounts_parent_id").col(Accounts::ParentId))
                    .index(Index::create().name("ix_accounts_system_role").col(Accounts::SystemRole))
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_bin")
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Accounts::Table).to_owned()).await
    }
}

#[derive(Iden)]
enum Accounts {
    Table,
    Id,
    Code,
    Name,
    NameEn,
    ParentId,
    IsGroup,
    Kind,
    Subtype,
    NormalSide,
    SystemRole,
    Currency,
    BranchId,
    RequiresParty,
    AllowManual,
    RequiresCostCenter,
    Active,
    CanDelete,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}
