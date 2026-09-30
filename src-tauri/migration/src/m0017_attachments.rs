//! m0017_attachments (21.02-B / C-16): attachment blobs, moved from the browser's per-device
//! IndexedDB (`src/mocks/attachments.ts`) into MariaDB so every terminal sharing the Main-PC DB
//! (D8) sees a file attached on another terminal, and so backups can include it. Source:
//! `AttachmentRecord`/`AttachmentMeta` in `src/mocks/attachments.ts`.
//!
//! Not branch-scoped (the mock's IndexedDB store wasn't either — `ownerRef` alone identifies what
//! a file is attached to, e.g. `customer:cus-3`, `journal:je-12`, and is optional for demo/unowned
//! files). `blob`/`thumbnail` are `LONGBLOB` — deliberately **excluded** from the generic
//! `dataset.rs` table dump (a `BLOB` column anywhere makes that dumper `INTERNAL`, 17-backup.md
//! §3.2); the backup archive builder packs/restores this table's rows as
//! `attachments/<id>.{blob,json,thumb}` zip entries instead (see `infrastructure/backup`), the same
//! layout `backupArchive.ts` already uses for its IndexedDB store.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Attachments::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Attachments::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(Attachments::OwnerRef).string_len(191).null())
                    .col(ColumnDef::new(Attachments::Name).string_len(255).not_null())
                    .col(ColumnDef::new(Attachments::Mime).string_len(127).not_null())
                    .col(ColumnDef::new(Attachments::Kind).custom(Alias::new("ENUM('image','pdf','office','other')")).not_null())
                    .col(ColumnDef::new(Attachments::Size).big_integer().not_null())
                    .col(ColumnDef::new(Attachments::Width).integer().null())
                    .col(ColumnDef::new(Attachments::Height).integer().null())
                    .col(ColumnDef::new(Attachments::Blob).custom(Alias::new("LONGBLOB")).not_null())
                    .col(ColumnDef::new(Attachments::Thumbnail).custom(Alias::new("LONGBLOB")).null())
                    .col(ColumnDef::new(Attachments::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(Attachments::CreatedBy).custom(Alias::new("UUID")).null())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_attachments_owner_ref")
                    .table(Attachments::Table)
                    .col(Attachments::OwnerRef)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Attachments::Table).to_owned()).await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Attachments {
    Table,
    Id,
    OwnerRef,
    Name,
    Mime,
    Kind,
    Size,
    Width,
    Height,
    Blob,
    Thumbnail,
    CreatedAt,
    CreatedBy,
}
