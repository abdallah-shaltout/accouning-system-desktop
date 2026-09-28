//! `domains::parties::service::write` — §3 `save_customer`/`save_supplier`: one generic core
//! (`save_party`) parameterised by kind + kind-specific texts/extra-field callbacks, since the two
//! mock functions (`partyService.ts:120-145`, `:178-203`) are identical apart from Arabic strings,
//! the `SequenceLock` variant, and the customer-only `credit_limit` / supplier-only
//! `contact_person`/`default_expense_account_id` fields.

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::parties::parties::{ActiveModel as PartyActiveModel, PartyContacts};
use crate::entities::parties::{party_history, party_phones, parties as parties_entity};
use crate::entities::platform::activity::ActivityKind;
use crate::shared::activity::{self, UndoRegistry};
use crate::shared::numbering::{self, SequenceLock};
use crate::utils::id::Id;
use crate::utils::route::RouteRef;

use super::super::dto::{self, Customer, CustomerInput, PartyContact, PartyKind, Supplier, SupplierInput};
use super::{clean_common, customer_computed, load_phones, next_code, supplier_computed, validate_common};

/// Kind-specific text/behavior the generic core needs (mock line refs in each variant).
struct KindTexts {
    kind: PartyKind,
    sequence_lock: SequenceLock,
    not_found: &'static str,
    deactivate_guard: &'static str,
    create_message_prefix: &'static str,
    update_message_prefix: &'static str,
    history_create_message: &'static str,
    history_update_message: &'static str,
    detail_route_name: &'static str,
}

const CUSTOMER_TEXTS: KindTexts = KindTexts {
    kind: PartyKind::Customer,
    sequence_lock: SequenceLock::CustomerCode,
    not_found: "العميل غير موجود",
    deactivate_guard: "لا يمكن إيقاف عميل عليه رصيد مستحق",
    create_message_prefix: "إضافة العميل",
    update_message_prefix: "تعديل العميل",
    history_create_message: "إنشاء بطاقة العميل",
    history_update_message: "تعديل بيانات العميل",
    detail_route_name: "customer",
};

const SUPPLIER_TEXTS: KindTexts = KindTexts {
    kind: PartyKind::Supplier,
    sequence_lock: SequenceLock::SupplierCode,
    not_found: "المورد غير موجود",
    deactivate_guard: "لا يمكن إيقاف مورد عليه رصيد مستحق",
    create_message_prefix: "إضافة المورد",
    update_message_prefix: "تعديل المورد",
    history_create_message: "إنشاء بطاقة المورد",
    history_update_message: "تعديل بيانات المورد",
    detail_route_name: "supplier",
};

/// Common request-only fields both `CustomerInput`/`SupplierInput` carry — extracted once so
/// `save_party` takes one shape regardless of kind; kind-only fields are applied by the caller
/// after `save_party` returns the persisted row id (customer's `credit_limit`, supplier's
/// `contact_person`/`default_expense_account_id`), inside the *same* transaction/lock, per the
/// mock's single `Object.assign` (§3 step 4) — here split only because Rust needs concrete types.
struct CommonInput {
    r#type: String,
    name: String,
    name_en: Option<String>,
    group_id: Option<String>,
    tags: Option<Vec<String>>,
    active: Option<bool>,
    phone: Option<String>,
    phones: Option<Vec<dto::PartyPhoneInput>>,
    email: Option<String>,
    contacts: Option<Vec<PartyContact>>,
    address: Option<String>,
    national_address: Option<dto::NationalAddressDto>,
    structured_address: Option<crate::entities::values::Address>,
    vat_number: Option<String>,
    cr_number: Option<String>,
    national_id: Option<String>,
    currency: Option<String>,
    price_list_id: Option<String>,
    payment_terms_days: Option<i32>,
    salesperson_id: Option<String>,
    branch_id: Option<String>,
    bank: Option<dto::PartyBankInfo>,
    opening_balance: Option<dto::OpeningBalanceStub>,
    notes: Option<String>,
}

impl From<&CustomerInput> for CommonInput {
    fn from(i: &CustomerInput) -> Self {
        CommonInput {
            r#type: i.r#type.clone(),
            name: i.name.clone(),
            name_en: i.name_en.clone(),
            group_id: i.group_id.clone(),
            tags: i.tags.clone(),
            active: i.active,
            phone: i.phone.clone(),
            phones: i.phones.clone(),
            email: i.email.clone(),
            contacts: i.contacts.clone(),
            address: i.address.clone(),
            national_address: i.national_address.clone(),
            structured_address: i.structured_address.clone(),
            vat_number: i.vat_number.clone(),
            cr_number: i.cr_number.clone(),
            national_id: i.national_id.clone(),
            currency: i.currency.clone(),
            price_list_id: i.price_list_id.clone(),
            payment_terms_days: i.payment_terms_days,
            salesperson_id: i.salesperson_id.clone(),
            branch_id: i.branch_id.clone(),
            bank: i.bank.clone(),
            opening_balance: i.opening_balance.clone(),
            notes: i.notes.clone(),
        }
    }
}

impl From<&SupplierInput> for CommonInput {
    fn from(i: &SupplierInput) -> Self {
        CommonInput {
            r#type: i.r#type.clone(),
            name: i.name.clone(),
            name_en: i.name_en.clone(),
            group_id: i.group_id.clone(),
            tags: i.tags.clone(),
            active: i.active,
            phone: i.phone.clone(),
            phones: i.phones.clone(),
            email: i.email.clone(),
            contacts: i.contacts.clone(),
            address: i.address.clone(),
            national_address: i.national_address.clone(),
            structured_address: i.structured_address.clone(),
            vat_number: i.vat_number.clone(),
            cr_number: i.cr_number.clone(),
            national_id: i.national_id.clone(),
            currency: i.currency.clone(),
            price_list_id: i.price_list_id.clone(),
            payment_terms_days: i.payment_terms_days,
            salesperson_id: i.salesperson_id.clone(),
            branch_id: i.branch_id.clone(),
            bank: i.bank.clone(),
            opening_balance: i.opening_balance.clone(),
            notes: i.notes.clone(),
        }
    }
}

fn parse_optional_id(raw: &Option<String>) -> Option<Id> {
    raw.as_deref().filter(|s| !s.is_empty()).and_then(|s| s.parse::<Id>().ok())
}

/// The generic core behind `save_customer`/`save_supplier` (§3 steps 1-7 / 2′-7). Returns the
/// persisted row so the caller can apply kind-only fields (`credit_limit` /
/// `contact_person`+`default_expense_account_id`) before building the response DTO.
#[allow(clippy::too_many_arguments)]
async fn save_party<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    texts: &KindTexts,
    input: &CommonInput,
    id: Option<Id>,
) -> TxResult<parties_entity::Model> {
    // 1. Validation (before anything else — mock line order).
    validate_common(conn, &input.name, input.vat_number.as_deref()).await?;
    let cleaned = clean_common(&input.name, &input.name_en, &input.phone, &input.email, &input.address, &input.vat_number, &input.cr_number, &input.national_id, &input.notes);

    let kind_str = texts.kind.as_str();

    let (mut active_model, is_fresh_row, party_id, is_create) = if let Some(id) = id {
        // 2. Update path: lock + load, kind must match.
        lock::for_update_by_id(conn, "parties", &id.to_string()).await?;
        let existing = parties_entity::Entity::find_by_id(id)
            .one(conn)
            .await
            .map_err(AppError::from)?
            .filter(|p| p.kind == kind_str)
            .ok_or_else(|| AppError::not_found(texts.not_found))?;

        // 3. Deactivate-with-balance guard.
        if input.active == Some(false) {
            let balance = match texts.kind {
                PartyKind::Customer => super::customer_computed(conn, id).await?.0,
                PartyKind::Supplier => super::supplier_computed(conn, id).await?.0,
            };
            if balance > rust_decimal::Decimal::ZERO {
                return Err(AppError::validation(texts.deactivate_guard).into());
            }
        }

        let am: PartyActiveModel = existing.into();
        (am, false, id, false)
    } else {
        // 2′. Create path: lock the sequence, compute the next code.
        numbering::lock(conn, texts.sequence_lock).await?;
        let code = next_code(conn, texts.kind).await?;
        let new_id = Id::new();
        let now = cx.clock.now;
        let am = PartyActiveModel {
            id: Set(new_id),
            kind: Set(kind_str.to_string()),
            r#type: Set(input.r#type.clone()),
            name: Set(String::new()),
            name_en: Set(None),
            code: Set(code),
            group_id: Set(None),
            tags: Set(None),
            active: Set(true),
            phone: Set(None),
            email: Set(None),
            contacts: Set(None),
            address: Set(None),
            national_address: Set(None),
            structured_address: Set(None),
            vat_number: Set(None),
            cr_number: Set(None),
            national_id: Set(None),
            currency: Set(None),
            price_list_id: Set(None),
            payment_terms_days: Set(None),
            salesperson_id: Set(None),
            branch_id: Set(None),
            bank: Set(None),
            opening_balance: Set(None),
            notes: Set(None),
            linked_party_id: Set(None),
            credit_limit: Set(None),
            contact_person: Set(None),
            default_expense_account_id: Set(None),
            search_normalized: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
        };
        (am, true, new_id, true)
    };

    // 4. Assign (Object.assign semantics, §3 step 4): cleaned keys always; every other input key
    // only when present.
    active_model.name = Set(cleaned.name);
    active_model.name_en = Set(cleaned.name_en);
    active_model.phone = Set(cleaned.phone);
    active_model.email = Set(cleaned.email);
    active_model.address = Set(cleaned.address);
    active_model.vat_number = Set(cleaned.vat_number);
    active_model.cr_number = Set(cleaned.cr_number);
    active_model.national_id = Set(cleaned.national_id);
    active_model.notes = Set(cleaned.notes);

    // `type` is a required field on both `CustomerInput`/`SupplierInput` (always a key on the JS
    // object), so it's always reassigned — matches `Object.assign(found, data, ...)` (§3 step 4).
    active_model.r#type = Set(input.r#type.clone());
    if is_create {
        // `active` defaults to `true` on create when the form omits it (the mock's `...data` spread
        // simply keeps whatever the object carries; every create form sends `active`, but the
        // request DTO models it as optional for symmetry with the update path).
        active_model.active = Set(input.active.unwrap_or(true));
    } else if let Some(active) = input.active {
        active_model.active = Set(active);
    }

    if let Some(group_id) = &input.group_id {
        active_model.group_id = Set(parse_optional_id(&Some(group_id.clone())));
    }
    if let Some(tags) = &input.tags {
        active_model.tags = Set(Some(crate::entities::values::StringList(tags.clone())));
    }
    if let Some(contacts) = &input.contacts {
        active_model.contacts = Set(Some(PartyContacts(contacts.iter().map(crate::entities::parties::parties::PartyContact::from).collect())));
    }
    if let Some(na) = &input.national_address {
        active_model.national_address = Set(Some(crate::entities::values::NationalAddress {
            country: na.country.clone(),
            city: na.city.clone(),
            district: na.district.clone(),
            street: na.street.clone(),
            building_no: na.building_no.clone(),
            additional_no: na.additional_no.clone(),
            postal_code: na.postal_code.clone(),
            unit_no: na.unit_no.clone(),
            short_address: na.short_address.clone(),
        }));
    }
    if input.structured_address.is_some() {
        active_model.structured_address = Set(input.structured_address.clone());
    }
    if let Some(currency) = &input.currency {
        active_model.currency = Set(Some(currency.clone()));
    }
    if let Some(price_list_id) = &input.price_list_id {
        active_model.price_list_id = Set(parse_optional_id(&Some(price_list_id.clone())));
    }
    if let Some(days) = input.payment_terms_days {
        active_model.payment_terms_days = Set(Some(days));
    }
    if let Some(salesperson_id) = &input.salesperson_id {
        active_model.salesperson_id = Set(parse_optional_id(&Some(salesperson_id.clone())));
    }
    if let Some(branch_id) = &input.branch_id {
        active_model.branch_id = Set(parse_optional_id(&Some(branch_id.clone())));
    }
    if let Some(bank) = &input.bank {
        active_model.bank = Set(Some(crate::entities::parties::parties::PartyBankInfo::from(bank)));
    }
    // `opening_balance` (D-3): server-owned `journalEntryId`/`locked` are preserved when already
    // set; only when the stored stub has no `journal_entry_id` do the client's amount/side/asOfDate
    // apply (the setup wizard's posting step is the only writer of `journal_entry_id`/`locked`).
    if let Some(ob) = &input.opening_balance {
        let existing_stub = active_model.opening_balance.try_as_ref().and_then(|o| o.clone());
        let merged = match existing_stub {
            Some(stub) if stub.journal_entry_id.is_some() => stub,
            _ => crate::entities::parties::parties::OpeningBalanceStub {
                amount: ob.amount,
                side: ob.side.clone(),
                as_of_date: ob.as_of_date,
                journal_entry_id: None,
                locked: None,
            },
        };
        active_model.opening_balance = Set(Some(merged));
    }
    // `linkedPartyId` from the input is never assigned (D-3) — links change only through
    // link/unlink.

    active_model.updated_at = Set(cx.clock.now);

    let saved = if is_create {
        active_model.insert(conn).await.map_err(|e| crate::core::error::map_unique_violation(e, "uq_parties_kind_code", || "هذا السجل موجود بالفعل".to_string()))?
    } else {
        active_model.update(conn).await.map_err(AppError::from)?
    };

    // Phones: only when `phones` is present in the request — delete + reinsert in order.
    if let Some(phones) = &input.phones {
        if !is_fresh_row {
            party_phones::Entity::delete_many().filter(party_phones::Column::PartyId.eq(party_id)).exec(conn).await.map_err(AppError::from)?;
        }
        for (position, phone) in phones.iter().enumerate() {
            let row = party_phones::ActiveModel {
                id: Set(Id::new()),
                party_id: Set(party_id),
                position: Set(position as i16),
                label: Set(phone.label.clone()),
                number: Set(phone.number.clone()),
            };
            row.insert(conn).await.map_err(AppError::from)?;
        }
    }

    // 5/5′. Activity log.
    let message = if is_create { format!("{} {}", texts.create_message_prefix, saved.name) } else { format!("{} {}", texts.update_message_prefix, saved.name) };
    activity::log(conn, cx, registry, ActivityKind::Party, message, None, Some(RouteRef::detail(texts.detail_route_name, party_id.to_string()))).await?;

    // 6/6′. `party_history` row.
    let history_message = if is_create { texts.history_create_message } else { texts.history_update_message };
    let actor_id = cx.actor.as_ref().map(|a| a.id).ok_or_else(|| AppError::internal("لا يمكن تسجيل العملية بدون مستخدم", None))?;
    let history = party_history::ActiveModel {
        id: Set(Id::new()),
        party_id: Set(party_id),
        party_kind: Set(kind_str.to_string()),
        date: Set(cx.clock.today()),
        message: Set(history_message.to_string()),
        user_id: Set(actor_id),
        created_at: Set(cx.clock.now),
    };
    history.insert(conn).await.map_err(AppError::from)?;

    // 7. Touch `Parties` for the cache-invalidation event.
    cx.touch(crate::core::events::ChangeCategory::Parties);

    // Reload so `phones`/`search_normalized`/any DB-side defaults are current.
    let reloaded = parties_entity::Entity::find_by_id(party_id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found(texts.not_found))?;
    Ok(reloaded)
}

/// **`save_customer`** (`partyService.ts:120-145`).
pub async fn save_customer<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &UndoRegistry, input: CustomerInput, id: Option<Id>) -> TxResult<Customer> {
    let common: CommonInput = (&input).into();
    let saved = save_party(conn, cx, registry, &CUSTOMER_TEXTS, &common, id).await?;

    // Customer-only field, applied inside the same row (already committed as part of the same
    // transaction the caller opened) — the mock's single `Object.assign` includes `creditLimit`
    // whenever present; done as a follow-up update here only because Rust splits common/kind-only
    // fields into two structs.
    let saved = if input.credit_limit.is_some() {
        let mut am: PartyActiveModel = saved.into();
        am.credit_limit = Set(input.credit_limit.map(crate::utils::money::round2));
        am.update(conn).await.map_err(AppError::from)?
    } else {
        saved
    };

    let phones = load_phones(conn, saved.id).await?;
    let (balance, credit) = customer_computed(conn, saved.id).await?;
    Ok(dto::to_customer(&saved, phones, balance, credit))
}

/// **`save_supplier`** (`partyService.ts:178-203`).
pub async fn save_supplier<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &UndoRegistry, input: SupplierInput, id: Option<Id>) -> TxResult<Supplier> {
    let common: CommonInput = (&input).into();
    let saved = save_party(conn, cx, registry, &SUPPLIER_TEXTS, &common, id).await?;

    let contact_person = input.contact_person.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string());
    let needs_update = input.contact_person.is_some() || input.default_expense_account_id.is_some();
    let saved = if needs_update {
        let mut am: PartyActiveModel = saved.into();
        if input.contact_person.is_some() {
            am.contact_person = Set(contact_person);
        }
        if let Some(acc) = &input.default_expense_account_id {
            am.default_expense_account_id = Set(parse_optional_id(&Some(acc.clone())));
        }
        am.update(conn).await.map_err(AppError::from)?
    } else {
        saved
    };

    let phones = load_phones(conn, saved.id).await?;
    let (balance, credit) = supplier_computed(conn, saved.id).await?;
    Ok(dto::to_supplier(&saved, phones, balance, credit))
}
