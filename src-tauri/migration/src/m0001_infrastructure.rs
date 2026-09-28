//! m0001 — infrastructure tables (21.02-A / A-7): `change_versions` (cross-terminal change
//! polling, cross-cutting.md §5) and `document_counters` (per-kind numbering sequences, rule 8).
//!
//! These are the two tables in the whole schema that do NOT get a UUID primary key (P2-40 / C-11):
//! they are per-branch mechanics that are never synced, so D2's UUIDv7 reasoning (sync ordering,
//! index fragmentation across branches) doesn't apply, and a short natural key (`category`/`kind`)
//! is both simpler and enough.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ChangeVersions::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ChangeVersions::Category).string_len(16).not_null().primary_key())
                    // Signed BIGINT: every reader decodes `i64` (sqlx refuses UNSIGNED → i64); never negative.
                    .col(ColumnDef::new(ChangeVersions::Version).big_integer().not_null().default(0))
                    .col(ColumnDef::new(ChangeVersions::UpdatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)".to_string()))
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(DocumentCounters::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(DocumentCounters::Kind).string_len(32).not_null().primary_key())
                    .col(ColumnDef::new(DocumentCounters::Value).big_integer().not_null().default(0))
                    .col(ColumnDef::new(DocumentCounters::UpdatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)".to_string()))
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        // Seed change_versions (cross-cutting §5: ledger/catalog/parties).
        for category in ["ledger", "catalog", "parties"] {
            let insert = Query::insert()
                .into_table(ChangeVersions::Table)
                .columns([ChangeVersions::Category, ChangeVersions::Version])
                .values_panic([category.into(), 0i64.into()])
                .to_owned();
            manager.exec_stmt(insert).await?;
        }

        // Seed document_counters: the 15 `DocumentKind` values (db.ts:126-141) plus the two
        // MAX+1-code locks (parties.md §5 `nextCode`, P2-21).
        const KINDS: [&str; 17] = [
            "invoice",
            "refund",
            "purchaseOrder",
            "purchaseReturn",
            "payment",
            "journal",
            "adjustment",
            "stockCount",
            "debitNoteDraft",
            "quotation",
            "shift",
            "expense",
            "voucher",
            "cardSettlement",
            "stockTransfer",
            "customerCodeLock",
            "supplierCodeLock",
        ];
        for kind in KINDS {
            let insert = Query::insert()
                .into_table(DocumentCounters::Table)
                .columns([DocumentCounters::Kind, DocumentCounters::Value])
                .values_panic([kind.into(), 0i64.into()])
                .to_owned();
            manager.exec_stmt(insert).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(DocumentCounters::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(ChangeVersions::Table).to_owned()).await?;
        Ok(())
    }
}

#[derive(Iden)]
enum ChangeVersions {
    Table,
    Category,
    Version,
    UpdatedAt,
}

#[derive(Iden)]
enum DocumentCounters {
    Table,
    Kind,
    Value,
    UpdatedAt,
}
