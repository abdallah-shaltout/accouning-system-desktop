//! m0013_platform (21.02-B, owner B2): activity, audit (incl. 02.E undo columns), approval_requests.
//! Source: `src/modules/core/types/index.ts` (ActivityEntry), `src/modules/diagnostics/types/index.ts`
//! (AuditEntry) + phase-e-activity.md E-2/E-4 undo columns, `src/modules/approvals/types/index.ts`.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // --- activity (append-only feed; no FK on entity_id — polymorphic) ------------------------
        manager
            .create_table(
                Table::create()
                    .table(Activity::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Activity::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(Activity::DateDay).date().not_null())
                    .col(ColumnDef::new(Activity::DateInstant).timestamp().null())
                    .col(ColumnDef::new(Activity::UserId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(Activity::Kind).custom(Alias::new(
                        "ENUM('sale','refund','purchase','purchase_return','payment','stock','journal','product','party','user','settings','auth','shift','expense','voucher','approval')",
                    )).not_null())
                    .col(ColumnDef::new(Activity::Message).text().not_null())
                    .col(ColumnDef::new(Activity::Link).json().null())
                    .col(ColumnDef::new(Activity::AuditId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Activity::CreatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(Index::create().name("ix_activity_date_day").table(Activity::Table).col(Activity::DateDay).to_owned())
            .await?;
        add_doc_date_key(db, "activity", "date").await?;

        // --- audit (append-only, structured — the "real" audit table; undo registry columns) ------
        manager
            .create_table(
                Table::create()
                    .table(Audit::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Audit::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(Audit::Entity).string_len(64).not_null())
                    // Polymorphic ref (B-1): no FK on entity_id.
                    .col(ColumnDef::new(Audit::EntityId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(Audit::EntityLabel).string_len(200).null())
                    .col(ColumnDef::new(Audit::Action).custom(Alias::new("ENUM('create','update','post','void','reverse','delete','login','settings')")).not_null())
                    .col(ColumnDef::new(Audit::Before).json().null())
                    .col(ColumnDef::new(Audit::After).json().null())
                    .col(ColumnDef::new(Audit::UserId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(Audit::BranchId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Audit::AtDay).date().not_null())
                    .col(ColumnDef::new(Audit::AtInstant).timestamp().null())
                    .col(ColumnDef::new(Audit::Reason).text().null())
                    .col(ColumnDef::new(Audit::Message).text().not_null())
                    .col(ColumnDef::new(Audit::Link).json().null())
                    // 02.E undo registry columns:
                    .col(ColumnDef::new(Audit::ActionType).string_len(64).null())
                    .col(ColumnDef::new(Audit::Payload).json().null())
                    .col(ColumnDef::new(Audit::IsUndoable).boolean().not_null().default(false))
                    .col(ColumnDef::new(Audit::UndoOf).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Audit::UndoneBy).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Audit::TerminalId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(Audit::CreatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_audit_undo_of")
                            .from(Audit::Table, Audit::UndoOf)
                            .to(Audit::Table, Audit::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_audit_undone_by")
                            .from(Audit::Table, Audit::UndoneBy)
                            .to(Audit::Table, Audit::Id),
                    )
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(Index::create().name("ix_audit_entity_entity_id").table(Audit::Table).col(Audit::Entity).col(Audit::EntityId).to_owned())
            .await?;
        add_doc_date_key(db, "audit", "at").await?;

        // --- approval_requests ---------------------------------------------------------------------
        manager
            .create_table(
                Table::create()
                    .table(ApprovalRequests::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ApprovalRequests::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(ApprovalRequests::Kind).custom(Alias::new("ENUM('discount','write_off','below_cost')")).not_null())
                    .col(ColumnDef::new(ApprovalRequests::Summary).text().not_null())
                    .col(ColumnDef::new(ApprovalRequests::Value).decimal_len(19, 4).not_null())
                    .col(ColumnDef::new(ApprovalRequests::RequestNote).text().null())
                    .col(ColumnDef::new(ApprovalRequests::RequestedBy).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(ApprovalRequests::RequestedByName).string_len(200).not_null())
                    .col(ColumnDef::new(ApprovalRequests::RequestedAtDay).date().not_null())
                    .col(ColumnDef::new(ApprovalRequests::RequestedAtInstant).timestamp().null())
                    .col(ColumnDef::new(ApprovalRequests::Status).custom(Alias::new("ENUM('pending','approved','rejected')")).not_null())
                    .col(ColumnDef::new(ApprovalRequests::DecidedBy).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(ApprovalRequests::DecidedByName).string_len(200).null())
                    .col(ColumnDef::new(ApprovalRequests::DecidedAtDay).date().null())
                    .col(ColumnDef::new(ApprovalRequests::DecidedAtInstant).timestamp().null())
                    .col(ColumnDef::new(ApprovalRequests::DecisionComment).text().null())
                    .col(ColumnDef::new(ApprovalRequests::Link).json().null())
                    .col(ColumnDef::new(ApprovalRequests::CreatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(ApprovalRequests::UpdatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(ApprovalRequests::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(ApprovalRequests::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Audit::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Activity::Table).to_owned()).await?;
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
enum Activity {
    Table,
    Id,
    DateDay,
    DateInstant,
    UserId,
    Kind,
    Message,
    Link,
    AuditId,
    CreatedAt,
}

#[derive(Iden)]
enum Audit {
    Table,
    Id,
    Entity,
    EntityId,
    EntityLabel,
    Action,
    Before,
    After,
    UserId,
    BranchId,
    AtDay,
    AtInstant,
    Reason,
    Message,
    Link,
    ActionType,
    Payload,
    IsUndoable,
    UndoOf,
    UndoneBy,
    TerminalId,
    CreatedAt,
}

#[derive(Iden)]
enum ApprovalRequests {
    Table,
    Id,
    Kind,
    Summary,
    Value,
    RequestNote,
    RequestedBy,
    RequestedByName,
    RequestedAtDay,
    RequestedAtInstant,
    Status,
    DecidedBy,
    DecidedByName,
    DecidedAtDay,
    DecidedAtInstant,
    DecisionComment,
    Link,
    CreatedAt,
    UpdatedAt,
    SyncStatus,
}
