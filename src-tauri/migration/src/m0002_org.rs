//! m0002_org (21.02-B, owner B1): `branches`, `cost_centers`, `cost_center_budgets`,
//! `fiscal_years`, `currencies`, `exchange_rates`, `taxes`, `settings`, `payment_methods`.
//!
//! No FKs here (B-1 rule): every `REFERENCES` is added in `m0015_foreign_keys`, owned by B2 —
//! see `fk_manifest_b1.md` for the exact list this file's tables need.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_branches(manager).await?;
        create_cost_centers(manager).await?;
        create_cost_center_budgets(manager).await?;
        create_fiscal_years(manager).await?;
        create_currencies(manager).await?;
        create_exchange_rates(manager).await?;
        create_taxes(manager).await?;
        create_settings(manager).await?;
        create_payment_methods(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(PaymentMethods::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Settings::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Taxes::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(ExchangeRates::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Currencies::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(FiscalYears::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(CostCenterBudgets::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(CostCenters::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Branches::Table).to_owned()).await?;
        Ok(())
    }
}

// --- branches (settings/types/dimensions.ts `Branch`) --------------------------------------------
//
// `active BOOL` instead of the soft-delete convention: `branches.md`/`settings.md`'s
// activate/deactivate pair is a plain flag flip, not a `deleted_at` timestamp (B-2 correction —
// branches is NOT in B-1's 12-table soft-delete list, and its own §2 doc confirms `active`).

async fn create_branches(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Branches::Table)
                .if_not_exists()
                .col(ColumnDef::new(Branches::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(Branches::Name).string_len(200).not_null())
                .col(ColumnDef::new(Branches::Code).string_len(64).not_null())
                .col(ColumnDef::new(Branches::Address).text().null())
                .col(ColumnDef::new(Branches::NationalAddress).json().null())
                .col(ColumnDef::new(Branches::Phone).string_len(32).null())
                .col(ColumnDef::new(Branches::ReceiptHeader).text().null())
                .col(ColumnDef::new(Branches::CashAccountId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Branches::BankAccountId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Branches::DefaultPriceListId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Branches::CostCenterId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Branches::Active).boolean().not_null().default(true))
                .col(ColumnDef::new(Branches::CanDelete).boolean().not_null().default(true))
                .col(ColumnDef::new(Branches::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(Branches::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .col(ColumnDef::new(Branches::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                .col(ColumnDef::new(Branches::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                // Case-insensitive uniqueness on `code` (P2-17: branches.code is ci) — plain UNIQUE
                // (no soft-delete on this table, so no generated `_live` column needed).
                .index(Index::create().name("uq_branches_code").col(Branches::Code).unique())
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- cost_centers (settings/types/dimensions.ts `CostCenter`) -------------------------------------

async fn create_cost_centers(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(CostCenters::Table)
                .if_not_exists()
                .col(ColumnDef::new(CostCenters::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(CostCenters::Code).string_len(64).not_null())
                .col(ColumnDef::new(CostCenters::Name).string_len(200).not_null())
                .col(ColumnDef::new(CostCenters::Kind).custom(Alias::new("ENUM('branch','department','project','other')")).not_null())
                .col(ColumnDef::new(CostCenters::ParentId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(CostCenters::ManagerUserId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(CostCenters::Active).boolean().not_null().default(true))
                .col(ColumnDef::new(CostCenters::CanDelete).boolean().not_null().default(true))
                .col(ColumnDef::new(CostCenters::BranchId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(CostCenters::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(CostCenters::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .col(ColumnDef::new(CostCenters::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                .col(ColumnDef::new(CostCenters::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                // Soft-delete table (B-1 list) — `code` uniqueness only applies while live.
                .col(
                    ColumnDef::new(CostCenters::CodeLive)
                        .string_len(64)
                        .extra("GENERATED ALWAYS AS (CASE WHEN `deleted_at` IS NULL THEN `code` END) STORED"),
                )
                .index(Index::create().name("uq_cost_centers_code_live").col(CostCenters::CodeLive).unique())
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- cost_center_budgets (child of CostCenter.budgets) --------------------------------------------

async fn create_cost_center_budgets(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(CostCenterBudgets::Table)
                .if_not_exists()
                .col(ColumnDef::new(CostCenterBudgets::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(CostCenterBudgets::CostCenterId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(CostCenterBudgets::FiscalYearId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(CostCenterBudgets::Amount).decimal_len(19, 2).not_null())
                .col(ColumnDef::new(CostCenterBudgets::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(CostCenterBudgets::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .index(
                    Index::create()
                        .name("uq_cost_center_budgets_center_year")
                        .col(CostCenterBudgets::CostCenterId)
                        .col(CostCenterBudgets::FiscalYearId)
                        .unique(),
                )
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- fiscal_years (accounting/types `FiscalYear`) -------------------------------------------------

async fn create_fiscal_years(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(FiscalYears::Table)
                .if_not_exists()
                .col(ColumnDef::new(FiscalYears::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(FiscalYears::Name).string_len(200).not_null())
                .col(ColumnDef::new(FiscalYears::StartDate).date().not_null())
                .col(ColumnDef::new(FiscalYears::EndDate).date().not_null())
                .col(ColumnDef::new(FiscalYears::IsClosed).boolean().not_null().default(false))
                // `closing_entry_id`/`closed_by` FKs added in m0015 (they point at tables created
                // later in dependency order — journal_entries/users).
                .col(ColumnDef::new(FiscalYears::ClosingEntryId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(FiscalYears::ClosedAt).custom(Alias::new("DATETIME(3)")).null())
                .col(ColumnDef::new(FiscalYears::ClosedBy).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(FiscalYears::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(FiscalYears::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .index(Index::create().name("ix_fiscal_years_dates").col(FiscalYears::StartDate).col(FiscalYears::EndDate))
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- currencies (settings/types/dimensions.ts `Currency`) -----------------------------------------
//
// PK is the ISO code itself (CHAR(3)) — the mock keys this table by `code`, never a synthetic id
// (`Currency` has no `id` field in the TS type). B-2 correction: not a UUID PK like every other
// table; recorded here since B-1's PK rule only lists `document_counters`/`change_versions` as the
// UUID exceptions — `currencies` is a third, justified by the TS type having no `id` field at all.

async fn create_currencies(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Currencies::Table)
                .if_not_exists()
                .col(ColumnDef::new(Currencies::Code).char_len(3).not_null().primary_key())
                .col(ColumnDef::new(Currencies::NameAr).string_len(200).not_null())
                .col(ColumnDef::new(Currencies::Symbol).string_len(16).not_null())
                .col(ColumnDef::new(Currencies::Decimals).integer().not_null())
                .col(ColumnDef::new(Currencies::Active).boolean().not_null().default(true))
                .col(ColumnDef::new(Currencies::Fixed).boolean().null())
                .col(ColumnDef::new(Currencies::FixedRate).decimal_len(19, 6).null())
                .col(ColumnDef::new(Currencies::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(Currencies::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_bin")
                .to_owned(),
        )
        .await
}

// --- exchange_rates (settings/types/dimensions.ts `ExchangeRate`) ---------------------------------

async fn create_exchange_rates(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(ExchangeRates::Table)
                .if_not_exists()
                .col(ColumnDef::new(ExchangeRates::Id).custom(Alias::new("UUID")).not_null().primary_key())
                // COLLATE matches `currencies.code` (utf8mb4_bin, P2-17) — an FK needs identical collation.
                .col(ColumnDef::new(ExchangeRates::Currency).char_len(3).extra("COLLATE utf8mb4_bin").not_null())
                .col(ColumnDef::new(ExchangeRates::Date).date().not_null())
                .col(ColumnDef::new(ExchangeRates::Rate).decimal_len(19, 6).not_null())
                .col(ColumnDef::new(ExchangeRates::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(ExchangeRates::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .index(Index::create().name("uq_exchange_rates_currency_date").col(ExchangeRates::Currency).col(ExchangeRates::Date).unique())
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- taxes (settings/types `Tax`) ------------------------------------------------------------------

async fn create_taxes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Taxes::Table)
                .if_not_exists()
                .col(ColumnDef::new(Taxes::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(Taxes::Name).string_len(200).not_null())
                .col(ColumnDef::new(Taxes::Rate).decimal_len(9, 4).not_null())
                .col(ColumnDef::new(Taxes::Type).custom(Alias::new("ENUM('OUTPUT','INPUT')")).not_null())
                .col(ColumnDef::new(Taxes::IsDefault).boolean().not_null().default(false))
                .col(ColumnDef::new(Taxes::Active).boolean().not_null().default(true))
                .col(ColumnDef::new(Taxes::Category).custom(Alias::new("ENUM('S','Z','E','O')")).not_null())
                .col(ColumnDef::new(Taxes::Direction).custom(Alias::new("ENUM('sales','purchase')")).not_null())
                .col(ColumnDef::new(Taxes::ExemptionReason).text().null())
                .col(ColumnDef::new(Taxes::AccountRole).custom(Alias::new("ENUM('vatOutput','vatInput')")).null())
                .col(ColumnDef::new(Taxes::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(Taxes::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .col(ColumnDef::new(Taxes::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                .col(ColumnDef::new(Taxes::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- settings (singleton row; StoreSettings, branch-scoped fields only per cross-cutting §3) ------

async fn create_settings(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Settings::Table)
                .if_not_exists()
                .col(ColumnDef::new(Settings::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(Settings::Singleton).tiny_integer().not_null().default(1))
                .col(ColumnDef::new(Settings::StoreName).string_len(200).not_null())
                .col(ColumnDef::new(Settings::Logo).custom(Alias::new("MEDIUMTEXT")).null())
                .col(ColumnDef::new(Settings::Stamp).custom(Alias::new("MEDIUMTEXT")).null())
                .col(ColumnDef::new(Settings::Signature).custom(Alias::new("MEDIUMTEXT")).null())
                // COLLATE matches `currencies.code` (utf8mb4_bin, P2-17) — an FK needs identical collation.
                .col(ColumnDef::new(Settings::Currency).char_len(3).extra("COLLATE utf8mb4_bin").not_null())
                .col(ColumnDef::new(Settings::Country).char_len(2).null())
                .col(ColumnDef::new(Settings::VatNumber).string_len(64).null())
                .col(ColumnDef::new(Settings::DefaultTaxId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Settings::InvoiceNumberPrefix).string_len(40).not_null())
                .col(ColumnDef::new(Settings::Printer).json().not_null())
                .col(ColumnDef::new(Settings::PricesIncludeTax).boolean().not_null().default(true))
                .col(ColumnDef::new(Settings::Address).text().null())
                .col(ColumnDef::new(Settings::NationalAddress).json().null())
                .col(ColumnDef::new(Settings::Phone).string_len(32).null())
                .col(ColumnDef::new(Settings::CommercialRegister).string_len(64).null())
                .col(ColumnDef::new(Settings::ReceiptFooter).text().null())
                .col(ColumnDef::new(Settings::Accounting).json().null())
                .col(ColumnDef::new(Settings::Backup).json().null())
                .col(ColumnDef::new(Settings::InventoryApprovalThreshold).decimal_len(19, 2).null())
                .col(ColumnDef::new(Settings::RoleAccessOverrides).json().null())
                .col(ColumnDef::new(Settings::InsightThresholds).json().null())
                .col(ColumnDef::new(Settings::Pos).json().null())
                .col(ColumnDef::new(Settings::Sales).json().null())
                .col(ColumnDef::new(Settings::Features).json().null())
                .col(ColumnDef::new(Settings::Onboarding).json().null())
                // P2-08: business timezone (IANA name), None = fall back to the Main PC's OS tz.
                .col(ColumnDef::new(Settings::Timezone).string_len(64).null())
                // P2-20: every branch row needs a resolvable default; the settings singleton names
                // which branch is "the" default for a not-yet-branch-aware reader. FK added m0015.
                .col(ColumnDef::new(Settings::DefaultBranchId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(Settings::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(Settings::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .index(Index::create().name("uq_settings_singleton").col(Settings::Singleton).unique())
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await?;

    // `CHECK (singleton = 1)` — sea-query has no first-class CHECK builder on `Table::create`, so
    // this is a raw statement immediately after the table exists.
    manager
        .get_connection()
        .execute(sea_orm::Statement::from_string(
            manager.get_database_backend(),
            "ALTER TABLE `settings` ADD CONSTRAINT `ck_settings_singleton` CHECK (`singleton` = 1)".to_string(),
        ))
        .await?;

    Ok(())
}

// --- payment_methods (settings/types `PaymentMethod`) ---------------------------------------------

async fn create_payment_methods(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(PaymentMethods::Table)
                .if_not_exists()
                .col(ColumnDef::new(PaymentMethods::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(PaymentMethods::Name).string_len(200).not_null())
                .col(
                    ColumnDef::new(PaymentMethods::Type)
                        .custom(Alias::new("ENUM('cash','card','bank_transfer','wallet','credit','store_credit')"))
                        .not_null(),
                )
                .col(ColumnDef::new(PaymentMethods::Icon).string_len(64).null())
                .col(
                    ColumnDef::new(PaymentMethods::AccountRole)
                        .custom(Alias::new("ENUM('cash','bank','cardClearing','walletClearing','receivable')"))
                        .not_null(),
                )
                .col(ColumnDef::new(PaymentMethods::FeePct).decimal_len(9, 4).not_null().default(0))
                .col(ColumnDef::new(PaymentMethods::RequiresReference).boolean().null())
                .col(ColumnDef::new(PaymentMethods::ShowInPos).boolean().not_null().default(true))
                .col(ColumnDef::new(PaymentMethods::ShowInPayments).boolean().not_null().default(true))
                .col(ColumnDef::new(PaymentMethods::SortOrder).small_unsigned().not_null().default(0))
                .col(ColumnDef::new(PaymentMethods::BranchOverrides).json().null())
                .col(ColumnDef::new(PaymentMethods::Active).boolean().not_null().default(true))
                .col(ColumnDef::new(PaymentMethods::CanDelete).boolean().not_null().default(true))
                .col(ColumnDef::new(PaymentMethods::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(PaymentMethods::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .col(ColumnDef::new(PaymentMethods::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                .col(ColumnDef::new(PaymentMethods::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- Idens -----------------------------------------------------------------------------------------

#[derive(Iden)]
enum Branches {
    Table,
    Id,
    Name,
    Code,
    Address,
    NationalAddress,
    Phone,
    ReceiptHeader,
    CashAccountId,
    BankAccountId,
    DefaultPriceListId,
    CostCenterId,
    Active,
    CanDelete,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum CostCenters {
    Table,
    Id,
    Code,
    Name,
    Kind,
    ParentId,
    ManagerUserId,
    Active,
    CanDelete,
    BranchId,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
    CodeLive,
}

#[derive(Iden)]
enum CostCenterBudgets {
    Table,
    Id,
    CostCenterId,
    FiscalYearId,
    Amount,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum FiscalYears {
    Table,
    Id,
    Name,
    StartDate,
    EndDate,
    IsClosed,
    ClosingEntryId,
    ClosedAt,
    ClosedBy,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Currencies {
    Table,
    Code,
    NameAr,
    Symbol,
    Decimals,
    Active,
    Fixed,
    FixedRate,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum ExchangeRates {
    Table,
    Id,
    Currency,
    Date,
    Rate,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Taxes {
    Table,
    Id,
    Name,
    Rate,
    Type,
    IsDefault,
    Active,
    Category,
    Direction,
    ExemptionReason,
    AccountRole,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum Settings {
    Table,
    Id,
    Singleton,
    StoreName,
    Logo,
    Stamp,
    Signature,
    Currency,
    Country,
    VatNumber,
    DefaultTaxId,
    InvoiceNumberPrefix,
    Printer,
    PricesIncludeTax,
    Address,
    NationalAddress,
    Phone,
    CommercialRegister,
    ReceiptFooter,
    Accounting,
    Backup,
    InventoryApprovalThreshold,
    RoleAccessOverrides,
    InsightThresholds,
    Pos,
    Sales,
    Features,
    Onboarding,
    Timezone,
    DefaultBranchId,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum PaymentMethods {
    Table,
    Id,
    Name,
    Type,
    Icon,
    AccountRole,
    FeePct,
    RequiresReference,
    ShowInPos,
    ShowInPayments,
    SortOrder,
    BranchOverrides,
    Active,
    CanDelete,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}
