//! m0011_expenses (21.02-B, owner B2): expense_categories, expenses, recurring_expenses.
//! Source types: `src/modules/expenses/types/index.ts`.

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
                    .table(ExpenseCategories::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ExpenseCategories::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(ExpenseCategories::Name).string_len(200).not_null())
                    .col(ColumnDef::new(ExpenseCategories::Icon).string_len(64).null())
                    .col(ColumnDef::new(ExpenseCategories::AccountId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(ExpenseCategories::DefaultTaxId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(ExpenseCategories::DefaultCostCenterId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(ExpenseCategories::Active).boolean().not_null().default(true))
                    .col(ColumnDef::new(ExpenseCategories::CanDelete).boolean().not_null().default(true))
                    .col(ColumnDef::new(ExpenseCategories::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(ExpenseCategories::UpdatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(ExpenseCategories::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                    .col(ColumnDef::new(ExpenseCategories::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        db.execute_unprepared(
            "ALTER TABLE expense_categories \
             ADD COLUMN name_live VARCHAR(200) AS (CASE WHEN deleted_at IS NULL THEN name END) STORED, \
             ADD UNIQUE KEY uq_expense_categories_name_live (name_live)",
        )
        .await?;

        manager
            .create_table(
                Table::create()
                    .table(Expenses::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Expenses::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(Expenses::Number).string_len(40).not_null())
                    .col(ColumnDef::new(Expenses::DateDay).date().not_null())
                    .col(ColumnDef::new(Expenses::DateInstant).custom(Alias::new("DATETIME(3)")).null())
                    .col(ColumnDef::new(Expenses::CategoryId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(Expenses::Amount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Expenses::IsTaxInvoice).boolean().not_null())
                    .col(ColumnDef::new(Expenses::TaxId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Expenses::NetAmount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Expenses::TaxAmount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(Expenses::SupplierVatNumber).string_len(64).null())
                    .col(ColumnDef::new(Expenses::SupplierInvoiceNo).string_len(64).null())
                    .col(ColumnDef::new(Expenses::CostCenterId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Expenses::PaidFromKind).custom(Alias::new("ENUM('method','credit')")).not_null())
                    .col(ColumnDef::new(Expenses::PaidFromPaymentMethodId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Expenses::PaidFromSupplierId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Expenses::Description).text().null())
                    .col(ColumnDef::new(Expenses::AttachmentIds).json().null())
                    .col(ColumnDef::new(Expenses::RepeatMonthly).boolean().not_null().default(false))
                    .col(ColumnDef::new(Expenses::RecurringTemplateId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Expenses::CreatedBy).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(Expenses::BranchId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Expenses::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(Expenses::UpdatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(Expenses::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                    .col(ColumnDef::new(Expenses::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .index(Index::create().name("uq_expenses_number").col(Expenses::Number).unique())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_expenses_category_id")
                            .from(Expenses::Table, Expenses::CategoryId)
                            .to(ExpenseCategories::Table, ExpenseCategories::Id),
                    )
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        add_doc_date_key(db, "expenses", "date").await?;

        manager
            .create_table(
                Table::create()
                    .table(RecurringExpenses::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(RecurringExpenses::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(RecurringExpenses::Name).string_len(200).not_null())
                    .col(ColumnDef::new(RecurringExpenses::CategoryId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(RecurringExpenses::Amount).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(RecurringExpenses::IsTaxInvoice).boolean().not_null())
                    .col(ColumnDef::new(RecurringExpenses::TaxId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(RecurringExpenses::PaidFromKind).custom(Alias::new("ENUM('method','credit')")).not_null())
                    .col(ColumnDef::new(RecurringExpenses::PaidFromPaymentMethodId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(RecurringExpenses::PaidFromSupplierId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(RecurringExpenses::Description).text().null())
                    .col(ColumnDef::new(RecurringExpenses::Day).tiny_unsigned().not_null())
                    .col(ColumnDef::new(RecurringExpenses::NextDate).date().not_null())
                    .col(ColumnDef::new(RecurringExpenses::AutoPost).boolean().not_null())
                    .col(ColumnDef::new(RecurringExpenses::Active).boolean().not_null().default(true))
                    .col(ColumnDef::new(RecurringExpenses::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(RecurringExpenses::UpdatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(RecurringExpenses::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                    .col(ColumnDef::new(RecurringExpenses::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recurring_expenses_category_id")
                            .from(RecurringExpenses::Table, RecurringExpenses::CategoryId)
                            .to(ExpenseCategories::Table, ExpenseCategories::Id),
                    )
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        db.execute_unprepared("ALTER TABLE recurring_expenses ADD CONSTRAINT ck_recurring_expenses_day CHECK (day BETWEEN 1 AND 28)").await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(RecurringExpenses::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Expenses::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(ExpenseCategories::Table).to_owned()).await?;
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
enum ExpenseCategories {
    Table,
    Id,
    Name,
    Icon,
    AccountId,
    DefaultTaxId,
    DefaultCostCenterId,
    Active,
    CanDelete,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum Expenses {
    Table,
    Id,
    Number,
    DateDay,
    DateInstant,
    CategoryId,
    Amount,
    IsTaxInvoice,
    TaxId,
    NetAmount,
    TaxAmount,
    SupplierVatNumber,
    SupplierInvoiceNo,
    CostCenterId,
    PaidFromKind,
    PaidFromPaymentMethodId,
    PaidFromSupplierId,
    Description,
    AttachmentIds,
    RepeatMonthly,
    RecurringTemplateId,
    CreatedBy,
    BranchId,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum RecurringExpenses {
    Table,
    Id,
    Name,
    CategoryId,
    Amount,
    IsTaxInvoice,
    TaxId,
    PaidFromKind,
    PaidFromPaymentMethodId,
    PaidFromSupplierId,
    Description,
    Day,
    NextDate,
    AutoPost,
    Active,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}
