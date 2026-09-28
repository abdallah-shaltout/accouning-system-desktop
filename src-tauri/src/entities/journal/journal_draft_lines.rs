//! `journal_draft_lines` entity (21.02-B, owner B2). Child of `journal_drafts`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "party_kind")]
pub enum PartyKind {
    #[sea_orm(string_value = "customer")]
    Customer,
    #[sea_orm(string_value = "supplier")]
    Supplier,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "journal_draft_lines")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub journal_draft_id: Id,
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
        belongs_to = "super::journal_drafts::Entity",
        from = "Column::JournalDraftId",
        to = "super::journal_drafts::Column::Id"
    )]
    JournalDraft,
}

impl Related<super::journal_drafts::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::JournalDraft.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
