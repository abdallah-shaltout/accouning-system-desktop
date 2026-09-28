//! m0007_parties (21.02-B, owner B1): `parties` (customers + suppliers, one table per P2-15),
//! `party_groups`, `party_history`.
//!
//! `parties.kind ENUM('customer','supplier')` plus a composite `(id, kind)` UNIQUE — needed so
//! `m0015_foreign_keys` can add the composite `(party_id, party_kind) → parties(id, kind)` FK on
//! `journal_lines` and `(target_id, target_type) → parties(id, kind)` on `payments` (P2-15's own
//! note, B-2's `m0015` row). Child list: `phones` (from `PartyCommon.phones: PartyPhone[]`) gets a
//! child table (B-1's "child lists get their own row + position + parent_id"); `contacts` is a
//! small (name/role/phone/email), never-independently-queried nested list, so it stays JSON —
//! same "value object vs. child table" split rule m0005 applied to `units`/`unitPrices` vs.
//! `product_prices`.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_parties(manager).await?;
        create_party_phones(manager).await?;
        create_party_groups(manager).await?;
        create_party_history(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(PartyHistory::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PartyGroups::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PartyPhones::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Parties::Table).to_owned()).await?;
        Ok(())
    }
}

// --- parties (parties/types `Customer` | `Supplier`, P2-15 single-table) ---------------------------

async fn create_parties(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Parties::Table)
                .if_not_exists()
                .col(ColumnDef::new(Parties::Id).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(Parties::Kind).custom(Alias::new("ENUM('customer','supplier')")).not_null())
                .col(ColumnDef::new(Parties::Type).custom(Alias::new("ENUM('individual','company')")).not_null())
                .col(ColumnDef::new(Parties::Name).string_len(200).not_null())
                .col(ColumnDef::new(Parties::NameEn).string_len(200).null())
                .col(ColumnDef::new(Parties::Code).string_len(64).not_null())
                .col(ColumnDef::new(Parties::GroupId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Parties::Tags).json().null())
                .col(ColumnDef::new(Parties::Active).boolean().not_null().default(true))
                .col(ColumnDef::new(Parties::Phone).string_len(32).null())
                .col(ColumnDef::new(Parties::Email).string_len(200).null())
                .col(ColumnDef::new(Parties::Contacts).json().null())
                .col(ColumnDef::new(Parties::Address).text().null())
                .col(ColumnDef::new(Parties::NationalAddress).json().null())
                .col(ColumnDef::new(Parties::StructuredAddress).json().null())
                .col(ColumnDef::new(Parties::VatNumber).string_len(64).null())
                .col(ColumnDef::new(Parties::CrNumber).string_len(64).null())
                .col(ColumnDef::new(Parties::NationalId).string_len(64).null())
                .col(ColumnDef::new(Parties::Currency).char_len(3).null())
                .col(ColumnDef::new(Parties::PriceListId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Parties::PaymentTermsDays).integer().null())
                .col(ColumnDef::new(Parties::SalespersonId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Parties::BranchId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Parties::Bank).json().null())
                .col(ColumnDef::new(Parties::OpeningBalance).json().null())
                .col(ColumnDef::new(Parties::Notes).text().null())
                .col(ColumnDef::new(Parties::LinkedPartyId).custom(Alias::new("UUID")).null())
                // Computed-on-read in the mock (`balance`/`unallocatedCredit` are derived from the
                // ledger, never stored input) — kept as persisted, kept-in-sync columns here since a
                // real backend needs a materialized value for statement/aging queries to stay fast;
                // every posting path that affects a party's balance updates these two columns in the
                // same transaction (Part 03's job, not this migration's).
                .col(ColumnDef::new(Parties::Balance).decimal_len(19, 2).not_null().default(0))
                .col(ColumnDef::new(Parties::UnallocatedCredit).decimal_len(19, 2).null())
                // Customer-only.
                .col(ColumnDef::new(Parties::CreditLimit).decimal_len(19, 2).null())
                // Supplier-only.
                .col(ColumnDef::new(Parties::ContactPerson).string_len(200).null())
                .col(ColumnDef::new(Parties::DefaultExpenseAccountId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Parties::SearchNormalized).custom(Alias::new("TEXT")).null())
                .col(ColumnDef::new(Parties::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(Parties::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .col(ColumnDef::new(Parties::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                .col(ColumnDef::new(Parties::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                .primary_key(Index::create().col(Parties::Id))
                // Composite (id, kind) unique — needed by m0015's composite FKs (P2-15).
                .index(Index::create().name("uq_parties_id_kind").col(Parties::Id).col(Parties::Kind).unique())
                // `code` is auto "C-0001"/"S-0001" per kind — unique within (kind, code), not
                // globally (a customer and a supplier code sequence are independent per
                // `parties.md` §5 `nextCode`, P2-21).
                .index(Index::create().name("uq_parties_kind_code").col(Parties::Kind).col(Parties::Code).unique())
                .index(Index::create().name("ix_parties_group_id").col(Parties::GroupId))
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- party phones (child of PartyCommon.phones) -----------------------------------------------------

async fn create_party_phones(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(PartyPhones::Table)
                .if_not_exists()
                .col(ColumnDef::new(PartyPhones::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(PartyPhones::PartyId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(PartyPhones::Position).small_unsigned().not_null())
                .col(ColumnDef::new(PartyPhones::Label).custom(Alias::new("ENUM('mobile','work','whatsapp')")).not_null())
                .col(ColumnDef::new(PartyPhones::Number).string_len(32).not_null())
                .index(Index::create().name("uq_party_phones_parent_position").col(PartyPhones::PartyId).col(PartyPhones::Position).unique())
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- party_groups (parties/types `PartyGroup`) -------------------------------------------------------

async fn create_party_groups(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(PartyGroups::Table)
                .if_not_exists()
                .col(ColumnDef::new(PartyGroups::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(PartyGroups::Kind).custom(Alias::new("ENUM('customer','supplier')")).not_null())
                .col(ColumnDef::new(PartyGroups::Name).string_len(200).not_null())
                .col(ColumnDef::new(PartyGroups::PriceListId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(PartyGroups::PaymentTermsDays).integer().null())
                .col(ColumnDef::new(PartyGroups::DiscountPercent).decimal_len(9, 4).null())
                .col(ColumnDef::new(PartyGroups::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(PartyGroups::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- party_history (parties/types `PartyHistoryEntry`, append-only) ---------------------------------

async fn create_party_history(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(PartyHistory::Table)
                .if_not_exists()
                .col(ColumnDef::new(PartyHistory::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(PartyHistory::PartyId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(PartyHistory::PartyKind).custom(Alias::new("ENUM('customer','supplier')")).not_null())
                .col(ColumnDef::new(PartyHistory::Date).date().not_null())
                .col(ColumnDef::new(PartyHistory::Message).text().not_null())
                .col(ColumnDef::new(PartyHistory::UserId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(PartyHistory::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .index(Index::create().name("ix_party_history_party_id").col(PartyHistory::PartyId))
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- Idens -----------------------------------------------------------------------------------------

#[derive(Iden)]
enum Parties {
    Table,
    Id,
    Kind,
    Type,
    Name,
    NameEn,
    Code,
    GroupId,
    Tags,
    Active,
    Phone,
    Email,
    Contacts,
    Address,
    NationalAddress,
    StructuredAddress,
    VatNumber,
    CrNumber,
    NationalId,
    Currency,
    PriceListId,
    PaymentTermsDays,
    SalespersonId,
    BranchId,
    Bank,
    OpeningBalance,
    Notes,
    LinkedPartyId,
    Balance,
    UnallocatedCredit,
    CreditLimit,
    ContactPerson,
    DefaultExpenseAccountId,
    SearchNormalized,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum PartyPhones {
    Table,
    Id,
    PartyId,
    Position,
    Label,
    Number,
}

#[derive(Iden)]
enum PartyGroups {
    Table,
    Id,
    Kind,
    Name,
    PriceListId,
    PaymentTermsDays,
    DiscountPercent,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum PartyHistory {
    Table,
    Id,
    PartyId,
    PartyKind,
    Date,
    Message,
    UserId,
    CreatedAt,
}
