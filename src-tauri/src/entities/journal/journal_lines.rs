//! `journal_lines` entity (21.02-B, owner B2). Child of `journal_entries`. Source: `JournalLine` in
//! `src/modules/accounting/types/index.ts`. `ck_journal_lines_one_sided`/`ck_journal_lines_non_negative`
//! (P2-39) enforced at the DB level (migration m0012). The composite `(party_id, party_kind) ->
//! parties(id, kind)` FK (P2-15) is added in `m0015_foreign_keys`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "party_kind")]
pub enum PartyKind {
    #[sea_orm(string_value = "customer")]
    Customer,
    #[sea_orm(string_value = "supplier")]
    Supplier,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "journal_lines")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub journal_entry_id: Id,
    pub position: i16,
    pub account_id: Id,
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    pub debit: Decimal,
    pub credit: Decimal,
    pub party_kind: Option<PartyKind>,
    pub party_id: Option<Id>,
    pub branch_id: Option<Id>,
    pub cost_center_id: Option<Id>,
    pub currency: Option<String>,
    pub amount_fc: Option<Decimal>,
    pub rate: Option<Decimal>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::journal_entries::Entity",
        from = "Column::JournalEntryId",
        to = "super::journal_entries::Column::Id"
    )]
    JournalEntry,
}

impl Related<super::journal_entries::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalEntry.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
