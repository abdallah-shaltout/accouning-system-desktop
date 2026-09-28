//! m0004_users (21.02-B, owner B1): `users`, `credentials`.
//!
//! `credentials` is keyed by the immutable `user_id`, not `username` (cross-cutting.md §1's fix
//! for the mock's `Record<username,password>` rekey-on-rename fragility) — `user_id` is both the PK
//! and (via `m0015_foreign_keys`) an FK to `users.id`, `ON DELETE CASCADE` since a credentials row
//! has no meaning without its user (users are never hard-deleted in practice, but the child-cascade
//! convention still applies per B-1).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Users::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(Users::Username).string_len(64).not_null())
                    .col(ColumnDef::new(Users::Name).string_len(200).not_null())
                    .col(ColumnDef::new(Users::Phone).string_len(32).null())
                    .col(ColumnDef::new(Users::Role).custom(Alias::new("ENUM('admin','manager','accountant','cashier','storekeeper')")).not_null())
                    .col(ColumnDef::new(Users::MaxDiscount).decimal_len(9, 4).not_null().default(0))
                    .col(ColumnDef::new(Users::PriceListId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Users::Active).boolean().not_null().default(true))
                    .col(ColumnDef::new(Users::Avatar).custom(Alias::new("MEDIUMTEXT")).null())
                    .col(ColumnDef::new(Users::AllowedBranches).json().null())
                    .col(ColumnDef::new(Users::HomeBranch).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Users::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(
                        ColumnDef::new(Users::UpdatedAt)
                            .custom(Alias::new("DATETIME(3)"))
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                    )
                    .col(ColumnDef::new(Users::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                    .col(ColumnDef::new(Users::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    // `username` is ci-unique while live (P2-17) — users aren't hard-deleted by any
                    // controller today but the mock's rename-collision handling implies the same
                    // "unique among active identities" semantics as the other soft-delete tables;
                    // modeled as a generated `_live` unique for consistency even though `users`
                    // itself isn't in B-1's 12-table soft-delete list (no `deleted_at` reader
                    // deactivates by soft-delete — `active` is the flag). Plain unique on `username`.
                    .index(Index::create().name("uq_users_username").col(Users::Username).unique())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Credentials::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Credentials::UserId).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(Credentials::PasswordHash).string_len(255).not_null())
                    .col(ColumnDef::new(Credentials::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(
                        ColumnDef::new(Credentials::UpdatedAt)
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

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Credentials::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Users::Table).to_owned()).await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
    Username,
    Name,
    Phone,
    Role,
    MaxDiscount,
    PriceListId,
    Active,
    Avatar,
    AllowedBranches,
    HomeBranch,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum Credentials {
    Table,
    UserId,
    PasswordHash,
    CreatedAt,
    UpdatedAt,
}
