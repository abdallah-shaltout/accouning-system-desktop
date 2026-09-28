//! m0012_journal (21.02-B, owner B2): journal_entries(+lines), journal_drafts(+draft_lines,
//! kept as SEPARATE tables per P2-14), journal_templates. Source: `src/modules/accounting/types/index.ts`.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // --- journal_entries (+lines) -------------------------------------------------------------
        manager
            .create_table(
                Table::create()
                    .table(JournalEntries::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(JournalEntries::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(JournalEntries::Number).string_len(40).not_null())
                    .col(ColumnDef::new(JournalEntries::DateDay).date().not_null())
                    .col(ColumnDef::new(JournalEntries::DateInstant).timestamp().null())
                    .col(ColumnDef::new(JournalEntries::Description).text().not_null())
                    .col(ColumnDef::new(JournalEntries::Type).custom(Alias::new("ENUM('SYSTEM','MANUAL','OPENING','CLOSING','VAT_SETTLEMENT')")).not_null())
                    .col(ColumnDef::new(JournalEntries::Status).custom(Alias::new("ENUM('DRAFT','POSTED')")).not_null())
                    .col(ColumnDef::new(JournalEntries::SourceKind).string_len(32).null())
                    .col(ColumnDef::new(JournalEntries::SourceId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(JournalEntries::SourceNumber).string_len(40).null())
                    .col(ColumnDef::new(JournalEntries::TotalDebit).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(JournalEntries::TotalCredit).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(JournalEntries::Reversed).boolean().not_null().default(false))
                    .col(ColumnDef::new(JournalEntries::ReversalOfId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(JournalEntries::ReversalReason).text().null())
                    .col(ColumnDef::new(JournalEntries::CreatedBy).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(JournalEntries::PostedBy).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(JournalEntries::PostedAtDay).date().null())
                    .col(ColumnDef::new(JournalEntries::PostedAtInstant).timestamp().null())
                    .col(ColumnDef::new(JournalEntries::AttachmentIds).json().null())
                    .col(ColumnDef::new(JournalEntries::TemplateId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(JournalEntries::SearchNormalized).text().null())
                    .col(ColumnDef::new(JournalEntries::CreatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(JournalEntries::UpdatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(JournalEntries::DeletedAt).timestamp().null())
                    .col(ColumnDef::new(JournalEntries::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .index(Index::create().name("uq_journal_entries_number").col(JournalEntries::Number).unique())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_journal_entries_reversal_of_id")
                            .from(JournalEntries::Table, JournalEntries::ReversalOfId)
                            .to(JournalEntries::Table, JournalEntries::Id),
                    )
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        // P2-39: total debit = total credit (defence in depth alongside the Rust posting rule).
        db.execute_unprepared("ALTER TABLE journal_entries ADD CONSTRAINT ck_journal_entries_balanced CHECK (total_debit = total_credit)").await?;

        manager
            .create_table(
                Table::create()
                    .table(JournalLines::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(JournalLines::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(JournalLines::JournalEntryId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(JournalLines::Position).small_unsigned().not_null())
                    .col(ColumnDef::new(JournalLines::AccountId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(JournalLines::Description).text().null())
                    .col(ColumnDef::new(JournalLines::Debit).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(JournalLines::Credit).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(JournalLines::PartyKind).custom(Alias::new("ENUM('customer','supplier')")).null())
                    .col(ColumnDef::new(JournalLines::PartyId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(JournalLines::BranchId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(JournalLines::CostCenterId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(JournalLines::Currency).char_len(3).null())
                    .col(ColumnDef::new(JournalLines::AmountFc).decimal_len(19, 2).null())
                    .col(ColumnDef::new(JournalLines::Rate).decimal_len(19, 6).null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_journal_lines_journal_entry_id")
                            .from(JournalLines::Table, JournalLines::JournalEntryId)
                            .to(JournalEntries::Table, JournalEntries::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(Index::create().name("uq_journal_lines_entry_id_position").col(JournalLines::JournalEntryId).col(JournalLines::Position).unique())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        // P2-39: one-sided (never both debit and credit) and non-negative.
        db.execute_unprepared(
            "ALTER TABLE journal_lines \
             ADD CONSTRAINT ck_journal_lines_one_sided CHECK (NOT (debit > 0 AND credit > 0)), \
             ADD CONSTRAINT ck_journal_lines_non_negative CHECK (debit >= 0 AND credit >= 0)",
        )
        .await?;

        // --- journal_drafts (+draft_lines) — kept SEPARATE from journal_entries per P2-14 ---------
        manager
            .create_table(
                Table::create()
                    .table(JournalDrafts::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(JournalDrafts::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(JournalDrafts::Number).string_len(40).null())
                    .col(ColumnDef::new(JournalDrafts::DateDay).date().not_null())
                    .col(ColumnDef::new(JournalDrafts::DateInstant).timestamp().null())
                    .col(ColumnDef::new(JournalDrafts::Description).text().not_null())
                    .col(ColumnDef::new(JournalDrafts::Type).custom(Alias::new("ENUM('SYSTEM','MANUAL','OPENING','CLOSING','VAT_SETTLEMENT')")).not_null())
                    .col(ColumnDef::new(JournalDrafts::SourceKind).string_len(32).null())
                    .col(ColumnDef::new(JournalDrafts::SourceId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(JournalDrafts::SourceNumber).string_len(40).null())
                    .col(ColumnDef::new(JournalDrafts::TotalDebit).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(JournalDrafts::TotalCredit).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(JournalDrafts::CreatedBy).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(JournalDrafts::AttachmentIds).json().null())
                    .col(ColumnDef::new(JournalDrafts::TemplateId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(JournalDrafts::CreatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(JournalDrafts::UpdatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(JournalDrafts::DeletedAt).timestamp().null())
                    .col(ColumnDef::new(JournalDrafts::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(JournalDraftLines::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(JournalDraftLines::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(JournalDraftLines::JournalDraftId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(JournalDraftLines::Position).small_unsigned().not_null())
                    .col(ColumnDef::new(JournalDraftLines::AccountId).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(JournalDraftLines::Description).text().null())
                    .col(ColumnDef::new(JournalDraftLines::Debit).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(JournalDraftLines::Credit).decimal_len(19, 2).not_null())
                    .col(ColumnDef::new(JournalDraftLines::PartyKind).custom(Alias::new("ENUM('customer','supplier')")).null())
                    .col(ColumnDef::new(JournalDraftLines::PartyId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(JournalDraftLines::BranchId).custom(Alias::new("UUID")).null())
                    .col(ColumnDef::new(JournalDraftLines::CostCenterId).custom(Alias::new("UUID")).null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_journal_draft_lines_journal_draft_id")
                            .from(JournalDraftLines::Table, JournalDraftLines::JournalDraftId)
                            .to(JournalDrafts::Table, JournalDrafts::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(Index::create().name("uq_journal_draft_lines_draft_id_position").col(JournalDraftLines::JournalDraftId).col(JournalDraftLines::Position).unique())
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;

        // --- journal_templates (+lines/recurrence as JSON — small, fixed-shape sub-documents) -----
        manager
            .create_table(
                Table::create()
                    .table(JournalTemplates::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(JournalTemplates::Id).custom(Alias::new("UUID")).not_null().primary_key())
                    .col(ColumnDef::new(JournalTemplates::Name).string_len(200).not_null())
                    .col(ColumnDef::new(JournalTemplates::Description).text().not_null())
                    .col(ColumnDef::new(JournalTemplates::Lines).json().not_null())
                    .col(ColumnDef::new(JournalTemplates::RecurrenceEvery).custom(Alias::new("ENUM('month','quarter','year')")).null())
                    .col(ColumnDef::new(JournalTemplates::RecurrenceDay).tiny_unsigned().null())
                    .col(ColumnDef::new(JournalTemplates::RecurrenceNextDate).date().null())
                    .col(ColumnDef::new(JournalTemplates::RecurrenceAutoPost).boolean().null())
                    .col(ColumnDef::new(JournalTemplates::CreatedBy).custom(Alias::new("UUID")).not_null())
                    .col(ColumnDef::new(JournalTemplates::CreatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(JournalTemplates::UpdatedAt).timestamp().not_null().extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"))
                    .col(ColumnDef::new(JournalTemplates::DeletedAt).timestamp().null())
                    .col(ColumnDef::new(JournalTemplates::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                    .engine("InnoDB")
                    .character_set("utf8mb4")
                    .collate("utf8mb4_unicode_ci")
                    .to_owned(),
            )
            .await?;
        db.execute_unprepared(
            "ALTER TABLE journal_templates \
             ADD COLUMN name_live VARCHAR(200) AS (CASE WHEN deleted_at IS NULL THEN name END) STORED, \
             ADD UNIQUE KEY uq_journal_templates_name_live (name_live)",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(JournalTemplates::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(JournalDraftLines::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(JournalDrafts::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(JournalLines::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(JournalEntries::Table).to_owned()).await?;
        Ok(())
    }
}

#[derive(Iden)]
enum JournalEntries {
    Table,
    Id,
    Number,
    DateDay,
    DateInstant,
    Description,
    Type,
    Status,
    SourceKind,
    SourceId,
    SourceNumber,
    TotalDebit,
    TotalCredit,
    Reversed,
    ReversalOfId,
    ReversalReason,
    CreatedBy,
    PostedBy,
    PostedAtDay,
    PostedAtInstant,
    AttachmentIds,
    TemplateId,
    SearchNormalized,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum JournalLines {
    Table,
    Id,
    JournalEntryId,
    Position,
    AccountId,
    Description,
    Debit,
    Credit,
    PartyKind,
    PartyId,
    BranchId,
    CostCenterId,
    Currency,
    AmountFc,
    Rate,
}

#[derive(Iden)]
enum JournalDrafts {
    Table,
    Id,
    Number,
    DateDay,
    DateInstant,
    Description,
    Type,
    SourceKind,
    SourceId,
    SourceNumber,
    TotalDebit,
    TotalCredit,
    CreatedBy,
    AttachmentIds,
    TemplateId,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum JournalDraftLines {
    Table,
    Id,
    JournalDraftId,
    Position,
    AccountId,
    Description,
    Debit,
    Credit,
    PartyKind,
    PartyId,
    BranchId,
    CostCenterId,
}

#[derive(Iden)]
enum JournalTemplates {
    Table,
    Id,
    Name,
    Description,
    Lines,
    RecurrenceEvery,
    RecurrenceDay,
    RecurrenceNextDate,
    RecurrenceAutoPost,
    CreatedBy,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}
