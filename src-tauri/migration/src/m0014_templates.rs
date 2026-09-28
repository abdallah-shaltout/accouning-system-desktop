//! m0014_templates (21.02-B, owner B2): print_templates (D9 — moved from `localStorage` to the
//! branch DB, shared per branch). Source: `src/modules/templates/types/index.ts` `PdfTemplate`.

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
                    .table(PrintTemplates::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PrintTemplates::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(PrintTemplates::Name).string_len(200).not_null())
                    .col(ColumnDef::new(PrintTemplates::Kind).custom(Alias::new(
                        "ENUM('invoice','quotation','creditNote','debitNote','purchaseOrder','voucher','statement','zReport','transferNote','report')",
                    )).not_null())
                    .col(ColumnDef::new(PrintTemplates::BaseTemplateId).string_len(32).not_null())
                    .col(ColumnDef::new(PrintTemplates::Options).json().not_null())
                    .col(ColumnDef::new(PrintTemplates::CustomSource).text().null())
                    .col(ColumnDef::new(PrintTemplates::IsDefault).boolean().not_null().default(false))
                    .col(ColumnDef::new(PrintTemplates::BranchId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(PrintTemplates::CreatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(PrintTemplates::UpdatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(PrintTemplates::DeletedAt).timestamp().null())
                    .col(ColumnDef::new(PrintTemplates::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        // C-15: only one default template per (branch, kind), among live (non-soft-deleted) rows.
        db.execute_unprepared(
            "ALTER TABLE print_templates \
             ADD COLUMN default_key VARCHAR(96) AS (CASE WHEN is_default AND deleted_at IS NULL THEN CONCAT(branch_id, ':', kind) END) STORED, \
             ADD UNIQUE KEY uq_print_templates_default_key (default_key)",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(PrintTemplates::Table).to_owned()).await?;
        Ok(())
    }
}

#[derive(Iden)]
enum PrintTemplates {
    Table,
    Id,
    Name,
    Kind,
    BaseTemplateId,
    Options,
    CustomSource,
    IsDefault,
    BranchId,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}
