//! `party_groups, parties, party_phones, party_history` — `customers` + `suppliers` become one
//! `parties` table (P2-15), `kind` = source array.

use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait};

use crate::core::tx::{TxError, TxResult};
use crate::entities::parties::parties::{ActiveModel as PartyActiveModel, OpeningBalanceStub};
use crate::entities::parties::party_groups::ActiveModel as PartyGroupActiveModel;
use crate::entities::parties::party_history::ActiveModel as PartyHistoryActiveModel;
use crate::entities::parties::party_phones::ActiveModel as PartyPhoneActiveModel;
use crate::entities::values::StringList;
use crate::infrastructure::import::idmap::IdMap;
use crate::infrastructure::import::model::{PartyGroupV1, PartyHistoryV1, PartyV1};
use crate::infrastructure::import::tables::{resolve_created_at, strict_ref};
use crate::utils::id::Id;
use crate::utils::money::round2;

pub async fn insert_party_groups<C: ConnectionTrait>(
    conn: &C,
    rows: &[PartyGroupV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let model = PartyGroupActiveModel {
            id: Set(id),
            kind: Set(row.kind.clone()),
            name: Set(row.name.clone()),
            price_list_id: Set(strict_ref(id_map, row.price_list_id.as_deref(), "party_groups")?),
            payment_terms_days: Set(row.payment_terms_days),
            discount_percent: Set(row.discount_percent),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

/// `kind` names which mock array this row came from (`"customer"` or `"supplier"`) — `customers`
/// and `suppliers` are inserted through this one function, called twice by `run.rs`.
pub async fn insert_parties<C: ConnectionTrait>(
    conn: &C,
    rows: &[PartyV1],
    kind: &str,
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);

        let national_address = row.national_address.as_ref().and_then(|v| serde_json::from_value(v.clone()).ok());
        let structured_address = row.structured_address.as_ref().and_then(|v| serde_json::from_value(v.clone()).ok());
        let bank = row.bank.as_ref().and_then(|v| serde_json::from_value(v.clone()).ok());
        let contacts: Option<crate::entities::parties::parties::PartyContacts> =
            row.contacts.as_ref().and_then(|v| serde_json::from_value(v.clone()).ok());

        let opening_balance = row.opening_balance.as_ref().map(|ob| {
            let amount = ob.amount.map(round2);
            if let (Some(rounded_amount), Some(original)) = (amount, ob.amount) {
                if rounded_amount != original {
                    *rounded += 1;
                }
            }
            OpeningBalanceStub {
                amount,
                side: ob.side.clone(),
                as_of_date: ob.as_of_date.as_deref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
                // A JSON id (no FK) pointing at a journal entry inserted later in IMPORT_ORDER:
                // `resolve_or_mint` hands out the id `insert_journal_entries` will then `assign`
                // (plain `resolve` would always miss here and drop the link).
                journal_entry_id: ob.journal_entry_id.as_deref().filter(|s| !s.is_empty()).map(|old| id_map.resolve_or_mint(old)),
                locked: ob.locked,
            }
        });

        let credit_limit = row.credit_limit.map(round2);
        if let (Some(a), Some(b)) = (credit_limit, row.credit_limit) {
            if a != b {
                *rounded += 1;
            }
        }

        let model = PartyActiveModel {
            id: Set(id),
            kind: Set(kind.to_string()),
            r#type: Set(row.kind.clone()),
            name: Set(row.name.clone()),
            name_en: Set(row.name_en.clone()),
            code: Set(row.code.clone()),
            group_id: Set(strict_ref(id_map, row.group_id.as_deref(), "parties")?),
            tags: Set(row.tags.clone().map(StringList)),
            active: Set(row.active),
            phone: Set(row.phone.clone()),
            email: Set(row.email.clone()),
            contacts: Set(contacts),
            address: Set(row.address.clone()),
            national_address: Set(national_address),
            structured_address: Set(structured_address),
            vat_number: Set(row.vat_number.clone()),
            cr_number: Set(row.cr_number.clone()),
            national_id: Set(row.national_id.clone()),
            currency: Set(row.currency.clone().map(|c| c.to_uppercase())),
            price_list_id: Set(strict_ref(id_map, row.price_list_id.as_deref(), "parties")?),
            payment_terms_days: Set(row.payment_terms_days),
            salesperson_id: Set(strict_ref(id_map, row.salesperson_id.as_deref(), "parties")?),
            branch_id: Set(strict_ref(id_map, row.branch_id.as_deref(), "parties")?),
            bank: Set(bank),
            opening_balance: Set(opening_balance),
            notes: Set(row.notes.clone()),
            linked_party_id: Set(None), // DEFERRED (self-ref; a customer may link a later supplier) — phase B.
            credit_limit: Set(credit_limit),
            contact_person: Set(row.contact_person.clone()),
            default_expense_account_id: Set(strict_ref(id_map, row.default_expense_account_id.as_deref(), "parties")?),
            search_normalized: sea_orm::ActiveValue::NotSet, // computed by before_save.
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
        };
        model.insert(conn).await.map_err(TxError::from)?;

        for (pi, phone) in row.phones.iter().enumerate() {
            let phone_model = PartyPhoneActiveModel {
                id: Set(Id::new()),
                party_id: Set(id),
                position: Set(pi as i16),
                label: Set(phone.label.clone()),
                number: Set(phone.number.clone()),
            };
            phone_model.insert(conn).await.map_err(TxError::from)?;
        }
    }
    Ok(())
}

pub async fn insert_party_history<C: ConnectionTrait>(
    conn: &C,
    rows: &[PartyHistoryV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(party_id) = id_map.resolve(&row.party_id) else { continue };
        let Some(user_id) = id_map.resolve(&row.user_id) else { continue };
        let date = chrono::NaiveDate::parse_from_str(&row.date, "%Y-%m-%d")
            .map_err(|_| crate::core::error::AppError::validation("تاريخ غير صالح في party_history"))?;
        let model = PartyHistoryActiveModel {
            id: Set(id),
            party_id: Set(party_id),
            party_kind: Set(row.party_kind.clone()),
            date: Set(date),
            message: Set(row.message.clone()),
            user_id: Set(user_id),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}
