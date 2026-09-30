//! `parties` (parties/types `Customer` | `Supplier`, P2-15 single table). Soft-delete table
//! (B-1 list — not explicitly named in B-1's 12-table list by this exact name, but `active`/
//! `deleted_at` semantics apply the same way here; confirmed against `parties.md`'s own
//! deactivate flow, which is a soft-delete, not a hard delete).
//!
//! `search_normalized` is one of the 4 tables filled by the write path (P2-38/B-6/B-7).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::entities::soft_delete::{self, SoftDelete};
use crate::entities::values::{Address, NationalAddress, StringList};
use crate::utils::id::Id;

/// `PartyCommon.contacts` — small nested list, never independently queried (unlike `phones`, which
/// gets its own child table per the migration's doc comment), so JSON per B-1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyContact {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, sea_orm::FromJsonQueryResult)]
#[serde(transparent)]
pub struct PartyContacts(pub Vec<PartyContact>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sea_orm::FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct PartyBankInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iban: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sea_orm::FromJsonQueryResult)]
#[serde(rename_all = "camelCase")]
pub struct OpeningBalanceStub {
    #[serde(skip_serializing_if = "Option::is_none", with = "crate::utils::money::serde_number::option")]
    pub amount: Option<Decimal>,
    /// `'debit' | 'credit'`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as_of_date: Option<chrono::NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal_entry_id: Option<Id>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "parties")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    /// `'customer' | 'supplier'` (P2-15).
    pub kind: String,
    /// `'individual' | 'company'`.
    pub r#type: String,
    pub name: String,
    pub name_en: Option<String>,
    pub code: String,
    pub group_id: Option<Id>,
    #[sea_orm(column_type = "Json", nullable)]
    pub tags: Option<StringList>,
    pub active: bool,
    pub phone: Option<String>,
    pub email: Option<String>,
    #[sea_orm(column_type = "Json", nullable)]
    pub contacts: Option<PartyContacts>,
    #[sea_orm(column_type = "Text", nullable)]
    pub address: Option<String>,
    /// @deprecated legacy shape — prefer `structured_address`.
    #[sea_orm(column_type = "Json", nullable)]
    pub national_address: Option<NationalAddress>,
    #[sea_orm(column_type = "Json", nullable)]
    pub structured_address: Option<Address>,
    pub vat_number: Option<String>,
    pub cr_number: Option<String>,
    pub national_id: Option<String>,
    #[sea_orm(column_type = "Char(Some(3))", nullable)]
    pub currency: Option<String>,
    pub price_list_id: Option<Id>,
    pub payment_terms_days: Option<i32>,
    pub salesperson_id: Option<Id>,
    pub branch_id: Option<Id>,
    #[sea_orm(column_type = "Json", nullable)]
    pub bank: Option<PartyBankInfo>,
    #[sea_orm(column_type = "Json", nullable)]
    pub opening_balance: Option<OpeningBalanceStub>,
    #[sea_orm(column_type = "Text", nullable)]
    pub notes: Option<String>,
    pub linked_party_id: Option<Id>,
    // `balance`/`unallocated_credit` columns dropped in `m0016` (G-11): nothing ever read or wrote
    // them — every caller derives the figure live from the ledger via
    // `shared::balances::{customer_balance, supplier_balance, unallocated_credit_for}`, exactly like
    // the mock's own computed getters. Keeping two always-stale, never-written columns around was a
    // foot-gun for a future direct read.
    /// Customer-only.
    #[sea_orm(column_type = "Decimal(Some((19, 2)))", nullable)]
    pub credit_limit: Option<Decimal>,
    /// Supplier-only.
    pub contact_person: Option<String>,
    pub default_expense_account_id: Option<Id>,
    #[sea_orm(column_type = "Text", nullable)]
    pub search_normalized: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub sync_status: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::party_phones::Entity")]
    Phones,
    #[sea_orm(has_many = "super::party_history::Entity")]
    History,
    #[sea_orm(belongs_to = "super::party_groups::Entity", from = "Column::GroupId", to = "super::party_groups::Column::Id")]
    Group,
}

impl Related<super::party_phones::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Phones.def()
    }
}

impl Related<super::party_history::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::History.def()
    }
}

impl Related<super::party_groups::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Group.def()
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    /// B-6/B-7 (P2-38): recomputes `search_normalized` from the union of the mock's two search
    /// field sets — `getCustomers`'s `includesText([c.name, c.nameEn, c.code, c.phone,
    /// c.vatNumber], ...)` and `getSuppliers`'s `includesText([s.name, s.nameEn, s.code, s.phone,
    /// s.contactPerson, s.vatNumber], ...)` (`src/modules/parties/services/partyService.ts`) —
    /// since both kinds share this one table (P2-15), `contact_person` is simply `NULL` on a
    /// customer row and contributes nothing to its haystack. Recomputed whenever any of these
    /// fields is `Set`; `Unchanged` fields are read from `self`, `NotSet` ones on a partial update from the
    /// stored row (one `find_by_id`), so a partial update still reflects the row's full state.
    async fn before_save<C>(mut self, db: &C, insert: bool) -> Result<Self, sea_orm::DbErr>
    where
        C: sea_orm::ConnectionTrait,
    {
        let touched = self.name.is_set() || self.name_en.is_set() || self.code.is_set() || self.phone.is_set() || self.vat_number.is_set() || self.contact_person.is_set();
        if touched {
            let stored = match self.id.try_as_ref() {
                Some(id) if !insert && (self.name.is_not_set() || self.name_en.is_not_set() || self.code.is_not_set() || self.phone.is_not_set() || self.vat_number.is_not_set() || self.contact_person.is_not_set()) => Entity::find_by_id(*id).one(db).await?,
                _ => None,
            };
            let name = self.name.try_as_ref().map(|s| s.as_str()).or(stored.as_ref().map(|m| m.name.as_str()));
            let name_en = match self.name_en.try_as_ref() {
                Some(v) => v.as_deref(),
                None => stored.as_ref().and_then(|m| m.name_en.as_deref()),
            };
            let code = self.code.try_as_ref().map(|s| s.as_str()).or(stored.as_ref().map(|m| m.code.as_str()));
            let phone = match self.phone.try_as_ref() {
                Some(v) => v.as_deref(),
                None => stored.as_ref().and_then(|m| m.phone.as_deref()),
            };
            let vat_number = match self.vat_number.try_as_ref() {
                Some(v) => v.as_deref(),
                None => stored.as_ref().and_then(|m| m.vat_number.as_deref()),
            };
            let contact_person = match self.contact_person.try_as_ref() {
                Some(v) => v.as_deref(),
                None => stored.as_ref().and_then(|m| m.contact_person.as_deref()),
            };
            self.search_normalized = sea_orm::ActiveValue::Set(Some(crate::utils::text::search_haystack(&[name, name_en, code, phone, vat_number, contact_person])));
        }
        Ok(self)
    }
}

impl SoftDelete for Entity {
    fn id_column() -> Self::Column {
        Column::Id
    }

    fn deleted_at_column() -> Self::Column {
        Column::DeletedAt
    }

    fn deleted_at_active_model(at: Option<DateTime<Utc>>) -> Self::ActiveModel {
        ActiveModel { deleted_at: soft_delete::deleted_at_value(at), ..Default::default() }
    }
}
