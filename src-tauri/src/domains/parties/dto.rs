//! `parties` DTOs (03-domains/05-parties.md §2) — mirrors of `src/modules/parties/types/index.ts`
//! and `partyService.ts`'s own inline types. `Customer`/`Supplier` are flat structs (no
//! `serde(flatten)`) since `Equals` in `contract.check.ts` needs one concrete object type per DTO.
//! One `common_fields` builder assembles the shared part for both, so the mapping exists once
//! (§2's own instruction).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::entities::parties::parties::{OpeningBalanceStub as EntityOpeningBalanceStub, PartyBankInfo as EntityPartyBankInfo, PartyContact as EntityPartyContact, PartyContacts};
use crate::entities::parties::{party_groups, party_history, party_phones, parties as parties_entity};
use crate::entities::values::{Address, NationalAddress};
use crate::shared::balances::PartyStatementRow as BalancesStatementRow;
use crate::utils::id::Id;
use crate::utils::money::serde_number;

/// `number | null` — the wire shape of `getLinkedNetBalance`'s return (`partyService.ts:248-252`,
/// §6: `null → undefined` at the switch line). `rust_decimal::Decimal` has no `ts-rs` impl in this
/// crate (`ts-rs`'s `uuid-impl`/`chrono-impl` features are enabled, not a decimal one), so every
/// other DTO field carrying money uses `#[ts(type = "number")]` on a struct field instead — this is
/// the one command whose *entire* return value is a bare optional decimal with no struct to hang
/// that attribute on, hence the manual `TS` impl (ts-rs 12's trait shape, matching
/// `settings::dto::OrderedNumberMap`'s existing manual impl in this codebase).
#[derive(Debug, Clone, Copy, Default)]
pub struct OptionalMoney(pub Option<Decimal>);

impl Serialize for OptionalMoney {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_number::option::serialize(&self.0, serializer)
    }
}

impl TS for OptionalMoney {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name(_cfg: &ts_rs::Config) -> String {
        "number | null".to_string()
    }
    fn inline(_cfg: &ts_rs::Config) -> String {
        "number | null".to_string()
    }
    fn inline_flattened(cfg: &ts_rs::Config) -> String {
        Self::inline(cfg)
    }
    fn decl(_cfg: &ts_rs::Config) -> String {
        String::new()
    }
    fn decl_concrete(_cfg: &ts_rs::Config) -> String {
        String::new()
    }
}

// --- Party kind ----------------------------------------------------------------------------------

/// `'customer' | 'supplier'` (lowercase, §2's `PartyKind`). Distinct from
/// `entities::journal::journal_lines::PartyKind` (a DB enum with no `TS`/serde derives) — this is
/// the wire/DTO copy the parties domain owns; `From`/`Into` bridge the two at the `shared::balances`
/// call sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "parties/types/gen/")]
pub enum PartyKind {
    Customer,
    Supplier,
}

impl From<PartyKind> for crate::entities::journal::journal_lines::PartyKind {
    fn from(k: PartyKind) -> Self {
        match k {
            PartyKind::Customer => crate::entities::journal::journal_lines::PartyKind::Customer,
            PartyKind::Supplier => crate::entities::journal::journal_lines::PartyKind::Supplier,
        }
    }
}

impl PartyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PartyKind::Customer => "customer",
            PartyKind::Supplier => "supplier",
        }
    }
}

// --- Phones / contacts / bank / opening balance ---------------------------------------------------

/// `PartyPhone` (`types/index.ts:2-7`). `id` is the server `Id` as text (D-6): the client's
/// `phone-N` ids are only `v-for` keys, never sent back as a real identity.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartyPhone {
    #[ts(type = "string")]
    pub id: Id,
    /// `'mobile' | 'work' | 'whatsapp'`.
    #[ts(type = "'mobile' | 'work' | 'whatsapp'")]
    pub label: String,
    pub number: String,
}

/// `PartyContact` (`types/index.ts:10-16`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartyContact {
    pub id: String,
    pub name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

impl From<&EntityPartyContact> for PartyContact {
    fn from(c: &EntityPartyContact) -> Self {
        PartyContact { id: c.id.clone(), name: c.name.clone(), role: c.role.clone(), phone: c.phone.clone(), email: c.email.clone() }
    }
}

impl From<&PartyContact> for EntityPartyContact {
    fn from(c: &PartyContact) -> Self {
        EntityPartyContact { id: c.id.clone(), name: c.name.clone(), role: c.role.clone(), phone: c.phone.clone(), email: c.email.clone() }
    }
}

/// `PartyBankInfo` (`types/index.ts:52-56`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartyBankInfo {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_name: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iban: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
}

impl From<&EntityPartyBankInfo> for PartyBankInfo {
    fn from(b: &EntityPartyBankInfo) -> Self {
        PartyBankInfo { bank_name: b.bank_name.clone(), iban: b.iban.clone(), account_name: b.account_name.clone() }
    }
}

impl From<&PartyBankInfo> for EntityPartyBankInfo {
    fn from(b: &PartyBankInfo) -> Self {
        EntityPartyBankInfo { bank_name: b.bank_name.clone(), iban: b.iban.clone(), account_name: b.account_name.clone() }
    }
}

/// `OpeningBalanceStub` (`types/index.ts:44-50`). `amount` via `serde_number::option`;
/// `asOfDate` kept as the stored string shape (`NaiveDate`, matching the entity column).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct OpeningBalanceStub {
    #[serde(default, with = "serde_number::option")]
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Decimal>,
    /// `'debit' | 'credit'`.
    #[ts(optional, type = "'debit' | 'credit'")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as_of_date: Option<chrono::NaiveDate>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal_entry_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
}

impl From<&EntityOpeningBalanceStub> for OpeningBalanceStub {
    fn from(s: &EntityOpeningBalanceStub) -> Self {
        OpeningBalanceStub { amount: s.amount, side: s.side.clone(), as_of_date: s.as_of_date, journal_entry_id: s.journal_entry_id, locked: s.locked }
    }
}

impl From<&OpeningBalanceStub> for EntityOpeningBalanceStub {
    fn from(s: &OpeningBalanceStub) -> Self {
        EntityOpeningBalanceStub { amount: s.amount, side: s.side.clone(), as_of_date: s.as_of_date, journal_entry_id: s.journal_entry_id, locked: s.locked }
    }
}

// --- Customer / Supplier ---------------------------------------------------------------------------

/// The fields every `Customer`/`Supplier` shares (`PartyCommon`, `types/index.ts:58-108`), built
/// once by [`common_fields`] and spread into both flat response structs. `national_address`/
/// `structured_address` are read straight off `m` by the callers instead of round-tripping through
/// here (no transformation needed beyond a clone/`From`).
struct CommonParts {
    id: Id,
    r#type: String,
    name: String,
    name_en: Option<String>,
    code: String,
    group_id: Option<Id>,
    tags: Option<Vec<String>>,
    active: bool,
    phone: Option<String>,
    phones: Option<Vec<PartyPhone>>,
    email: Option<String>,
    contacts: Option<Vec<PartyContact>>,
    address: Option<String>,
    vat_number: Option<String>,
    cr_number: Option<String>,
    national_id: Option<String>,
    currency: Option<String>,
    price_list_id: Option<Id>,
    payment_terms_days: Option<i32>,
    salesperson_id: Option<Id>,
    branch_id: Option<Id>,
    bank: Option<PartyBankInfo>,
    opening_balance: Option<OpeningBalanceStub>,
    notes: Option<String>,
    linked_party_id: Option<Id>,
    balance: Decimal,
    unallocated_credit: Option<Decimal>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

/// §3 `with_computed`: builds the shared part of a `Customer`/`Supplier` DTO from the row plus its
/// already-loaded phones and already-computed balance/credit — one mapping, reused by both kinds
/// (§2's `common_fields` instruction).
fn common_fields(m: &parties_entity::Model, phones: Vec<party_phones::Model>, balance: Decimal, unallocated_credit: Decimal) -> CommonParts {
    // Ordering is the caller's responsibility (query `ORDER BY party_id, position`, §3) — never
    // re-sorted here.
    let phones: Vec<PartyPhone> = phones.into_iter().map(|p| PartyPhone { id: p.id, label: p.label, number: p.number }).collect();
    CommonParts {
        id: m.id,
        r#type: m.r#type.clone(),
        name: m.name.clone(),
        name_en: m.name_en.clone(),
        code: m.code.clone(),
        group_id: m.group_id,
        tags: m.tags.clone().map(|t| t.0),
        active: m.active,
        phone: m.phone.clone(),
        phones: Some(phones),
        email: m.email.clone(),
        contacts: m.contacts.clone().map(|c: PartyContacts| c.0.iter().map(PartyContact::from).collect()),
        address: m.address.clone(),
        vat_number: m.vat_number.clone(),
        cr_number: m.cr_number.clone(),
        national_id: m.national_id.clone(),
        currency: m.currency.clone(),
        price_list_id: m.price_list_id,
        payment_terms_days: m.payment_terms_days,
        salesperson_id: m.salesperson_id,
        branch_id: m.branch_id,
        bank: m.bank.as_ref().map(PartyBankInfo::from),
        opening_balance: m.opening_balance.as_ref().map(OpeningBalanceStub::from),
        notes: m.notes.clone(),
        linked_party_id: m.linked_party_id,
        balance,
        unallocated_credit: Some(unallocated_credit),
        created_at: Some(crate::utils::dates::format_iso_ms(m.created_at)),
        updated_at: if m.updated_at != m.created_at { Some(crate::utils::dates::format_iso_ms(m.updated_at)) } else { None },
    }
}

/// `Customer` (`types/index.ts:110-112`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct Customer {
    #[ts(type = "string")]
    pub id: Id,
    #[ts(type = "'individual' | 'company'")]
    pub r#type: String,
    pub name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_en: Option<String>,
    pub code: String,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub active: bool,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phones: Option<Vec<PartyPhone>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contacts: Option<Vec<PartyContact>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub national_address: Option<NationalAddressDto>,
    #[ts(optional, type = "import('@/modules/core/types/address').Address")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_address: Option<Address>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat_number: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cr_number: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub national_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_list_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_terms_days: Option<i32>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub salesperson_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank: Option<PartyBankInfo>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening_balance: Option<OpeningBalanceStub>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_party_id: Option<Id>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub balance: Decimal,
    #[serde(default, with = "serde_number::option")]
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unallocated_credit: Option<Decimal>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, with = "serde_number::option")]
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credit_limit: Option<Decimal>,
}

/// `Supplier` (`types/index.ts:116-120`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct Supplier {
    #[ts(type = "string")]
    pub id: Id,
    #[ts(type = "'individual' | 'company'")]
    pub r#type: String,
    pub name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_en: Option<String>,
    pub code: String,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub active: bool,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phones: Option<Vec<PartyPhone>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contacts: Option<Vec<PartyContact>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub national_address: Option<NationalAddressDto>,
    #[ts(optional, type = "import('@/modules/core/types/address').Address")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_address: Option<Address>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat_number: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cr_number: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub national_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_list_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_terms_days: Option<i32>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub salesperson_id: Option<Id>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank: Option<PartyBankInfo>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening_balance: Option<OpeningBalanceStub>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_party_id: Option<Id>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub balance: Decimal,
    #[serde(default, with = "serde_number::option")]
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unallocated_credit: Option<Decimal>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_person: Option<String>,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_expense_account_id: Option<Id>,
}

/// `NationalAddress` (`types/index.ts:25-36`, deprecated legacy shape) as a DTO mirror of
/// `entities::values::NationalAddress`.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct NationalAddressDto {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub district: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub building_no: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_no: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_no: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_address: Option<String>,
}

impl From<&NationalAddress> for NationalAddressDto {
    fn from(n: &NationalAddress) -> Self {
        NationalAddressDto {
            country: n.country.clone(),
            city: n.city.clone(),
            district: n.district.clone(),
            street: n.street.clone(),
            building_no: n.building_no.clone(),
            additional_no: n.additional_no.clone(),
            postal_code: n.postal_code.clone(),
            unit_no: n.unit_no.clone(),
            short_address: n.short_address.clone(),
        }
    }
}

/// Builds the `Customer` response DTO (§3 `with_computed`).
pub fn to_customer(m: &parties_entity::Model, phones: Vec<party_phones::Model>, balance: Decimal, unallocated_credit: Decimal) -> Customer {
    let c = common_fields(m, phones, balance, unallocated_credit);
    Customer {
        id: c.id,
        r#type: c.r#type,
        name: c.name,
        name_en: c.name_en,
        code: c.code,
        group_id: c.group_id,
        tags: c.tags,
        active: c.active,
        phone: c.phone,
        phones: c.phones,
        email: c.email,
        contacts: c.contacts,
        address: c.address,
        national_address: m.national_address.as_ref().map(NationalAddressDto::from),
        structured_address: m.structured_address.clone(),
        vat_number: c.vat_number,
        cr_number: c.cr_number,
        national_id: c.national_id,
        currency: c.currency,
        price_list_id: c.price_list_id,
        payment_terms_days: c.payment_terms_days,
        salesperson_id: c.salesperson_id,
        branch_id: c.branch_id,
        bank: c.bank,
        opening_balance: c.opening_balance,
        notes: c.notes,
        linked_party_id: c.linked_party_id,
        balance: c.balance,
        unallocated_credit: c.unallocated_credit,
        created_at: c.created_at,
        updated_at: c.updated_at,
        credit_limit: m.credit_limit,
    }
}

/// Builds the `Supplier` response DTO (§3 `with_computed`).
pub fn to_supplier(m: &parties_entity::Model, phones: Vec<party_phones::Model>, balance: Decimal, unallocated_credit: Decimal) -> Supplier {
    let c = common_fields(m, phones, balance, unallocated_credit);
    Supplier {
        id: c.id,
        r#type: c.r#type,
        name: c.name,
        name_en: c.name_en,
        code: c.code,
        group_id: c.group_id,
        tags: c.tags,
        active: c.active,
        phone: c.phone,
        phones: c.phones,
        email: c.email,
        contacts: c.contacts,
        address: c.address,
        national_address: m.national_address.as_ref().map(NationalAddressDto::from),
        structured_address: m.structured_address.clone(),
        vat_number: c.vat_number,
        cr_number: c.cr_number,
        national_id: c.national_id,
        currency: c.currency,
        price_list_id: c.price_list_id,
        payment_terms_days: c.payment_terms_days,
        salesperson_id: c.salesperson_id,
        branch_id: c.branch_id,
        bank: c.bank,
        opening_balance: c.opening_balance,
        notes: c.notes,
        linked_party_id: c.linked_party_id,
        balance: c.balance,
        unallocated_credit: c.unallocated_credit,
        created_at: c.created_at,
        updated_at: c.updated_at,
        contact_person: m.contact_person.clone(),
        default_expense_account_id: m.default_expense_account_id,
    }
}

// --- Request-only inputs ---------------------------------------------------------------------------

/// `CustomerInput` (`types/index.ts:114`) — request-only mirror of the TS `Omit<Customer, 'id' |
/// 'balance' | 'code' | 'unallocatedCredit' | 'createdAt' | 'updatedAt'>`. `linkedPartyId` is
/// accepted but always ignored server-side (D-3 — links change only through link/unlink).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct CustomerInput {
    #[ts(type = "'individual' | 'company'")]
    pub r#type: String,
    pub name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_en: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub active: bool,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phones: Option<Vec<PartyPhone>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contacts: Option<Vec<PartyContact>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub national_address: Option<NationalAddressDto>,
    #[ts(optional, type = "import('@/modules/core/types/address').Address")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_address: Option<Address>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat_number: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cr_number: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub national_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_list_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_terms_days: Option<i32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub salesperson_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank: Option<PartyBankInfo>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening_balance: Option<OpeningBalanceStub>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Accepted but always ignored (D-3).
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_party_id: Option<String>,
    #[serde(default, with = "serde_number::option")]
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credit_limit: Option<Decimal>,
}

/// `SupplierInput` (`types/index.ts:122`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct SupplierInput {
    #[ts(type = "'individual' | 'company'")]
    pub r#type: String,
    pub name: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_en: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub active: bool,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phones: Option<Vec<PartyPhone>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contacts: Option<Vec<PartyContact>>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub national_address: Option<NationalAddressDto>,
    #[ts(optional, type = "import('@/modules/core/types/address').Address")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_address: Option<Address>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat_number: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cr_number: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub national_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_list_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_terms_days: Option<i32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub salesperson_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank: Option<PartyBankInfo>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening_balance: Option<OpeningBalanceStub>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_party_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_person: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_expense_account_id: Option<String>,
}

/// A phone as sent by the client on save — no `id` required (a new phone doesn't have a server id
/// yet); the service assigns a fresh `Id::new()` to every row on write (§3 step 4).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartyPhoneInput {
    /// `'mobile' | 'work' | 'whatsapp'`.
    #[ts(type = "'mobile' | 'work' | 'whatsapp'")]
    pub label: String,
    pub number: String,
}

// --- PartyGroup ------------------------------------------------------------------------------------

/// `PartyGroup` (`types/index.ts:125-132`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartyGroup {
    #[ts(type = "string")]
    pub id: Id,
    pub kind: PartyKind,
    pub name: String,
    #[ts(optional, type = "string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_list_id: Option<Id>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_terms_days: Option<i32>,
    #[serde(default, with = "serde_number::option")]
    #[ts(optional, type = "number")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_percent: Option<Decimal>,
}

pub fn to_party_group(m: &party_groups::Model) -> PartyGroup {
    let kind = if m.kind == "customer" { PartyKind::Customer } else { PartyKind::Supplier };
    PartyGroup { id: m.id, kind, name: m.name.clone(), price_list_id: m.price_list_id, payment_terms_days: m.payment_terms_days, discount_percent: m.discount_percent }
}

// --- PartyStatementRow -----------------------------------------------------------------------------

/// `PartyStatementRow` (`types/index.ts:135-146`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartyStatementRow {
    #[ts(type = "string")]
    pub id: Id,
    pub date: String,
    /// `'invoice' | 'refund' | 'payment' | 'purchaseOrder' | 'purchaseReturn' | 'opening'`.
    #[ts(type = "'invoice' | 'refund' | 'payment' | 'purchaseOrder' | 'purchaseReturn' | 'opening'")]
    pub kind: String,
    #[ts(type = "string")]
    pub ref_id: Id,
    pub number: String,
    pub description: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub debit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub credit: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub balance: Decimal,
}

impl From<BalancesStatementRow> for PartyStatementRow {
    fn from(r: BalancesStatementRow) -> Self {
        PartyStatementRow { id: r.id, date: r.date_key, kind: r.kind.to_string(), ref_id: r.ref_id, number: r.number, description: r.description, debit: r.debit, credit: r.credit, balance: r.balance }
    }
}

// --- Aging -------------------------------------------------------------------------------------

/// `AgingBucketKey` (`types/index.ts:158`) — variant renames per §2 (`current`, `"30"`, `"60"`,
/// `"90plus"`; not valid bare Rust identifiers, hence explicit `#[serde(rename = ...)]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "parties/types/gen/")]
pub enum AgingBucketKey {
    #[serde(rename = "current")]
    Current,
    #[serde(rename = "30")]
    D30,
    #[serde(rename = "60")]
    D60,
    #[serde(rename = "90plus")]
    D90Plus,
}

/// `AgingBucket.documents[]` element (`types/index.ts:164`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct AgingDocument {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    pub date: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub outstanding: Decimal,
}

/// `AgingBucket` (`types/index.ts:160-165`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct AgingBucket {
    pub key: AgingBucketKey,
    pub label: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
    pub documents: Vec<AgingDocument>,
}

// --- PartyHistoryEntry -----------------------------------------------------------------------------

/// `PartyHistoryEntry` (`types/index.ts:149-156`). `date` is the ISO of `created_at` (D-7 — every
/// mock writer stores an ISO instant here).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartyHistoryEntry {
    #[ts(type = "string")]
    pub id: Id,
    #[ts(type = "string")]
    pub party_id: Id,
    pub party_kind: PartyKind,
    pub date: String,
    pub message: String,
    #[ts(type = "string")]
    pub user_id: Id,
}

pub fn to_party_history_entry(m: &party_history::Model) -> PartyHistoryEntry {
    let party_kind = if m.party_kind == "customer" { PartyKind::Customer } else { PartyKind::Supplier };
    PartyHistoryEntry { id: m.id, party_id: m.party_id, party_kind, date: crate::utils::dates::format_iso_ms(m.created_at), message: m.message.clone(), user_id: m.user_id }
}

// --- Filters / duplicate check ----------------------------------------------------------------------

/// `PartyFilter` (`partyService.ts:16-22`).
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartyFilter {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_inactive: Option<bool>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_balance_only: Option<bool>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub over_limit_only: Option<bool>,
}

/// `DuplicateWarning` (`partyService.ts:24-28`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct DuplicateWarning {
    /// `'phone' | 'vatNumber'`.
    #[ts(type = "'phone' | 'vatNumber'")]
    pub field: String,
    #[ts(type = "string")]
    pub existing_id: Id,
    pub existing_name: String,
}

/// `checkDuplicates`'s input shape (`partyService.ts:66`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct DuplicateCheckInput {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat_number: Option<String>,
}

// --- Command args (§1, one struct per command, camelCase) -------------------------------------------

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartiesCheckDuplicatesArgs {
    pub input: DuplicateCheckInput,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartiesGetPartyGroupsArgs {
    pub kind: PartyKind,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartiesGetCustomersArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<PartyFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartiesGetCustomerArgs {
    pub id: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartiesSaveCustomerArgs {
    pub input: CustomerInput,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartiesGetCustomerStatementArgs {
    pub id: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartiesGetSuppliersArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<PartyFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartiesGetSupplierArgs {
    pub id: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartiesSaveSupplierArgs {
    pub input: SupplierInput,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartiesGetSupplierStatementArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartiesLinkPartyRecordsArgs {
    pub customer_id: String,
    pub supplier_id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartiesUnlinkPartyRecordArgs {
    pub party_id: String,
    pub kind: PartyKind,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartiesGetLinkedNetBalanceArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartiesGetPartyHistoryArgs {
    pub party_id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "parties/types/gen/")]
pub struct PartiesGetPartyAgingArgs {
    pub kind: PartyKind,
    pub party_id: String,
}
