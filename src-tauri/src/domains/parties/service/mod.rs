//! `domains::parties::service` — one file per concern (05-parties.md §3): shared helpers here,
//! `read`/`write`/`link`/`aging` in their own files.

pub mod aging;
pub mod link;
pub mod read;
pub mod write;

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::parties::{party_phones, parties as parties_entity};
use crate::shared::balances;
use crate::shared::validation::tax_id_rule;
use crate::utils::id::Id;

use super::dto::PartyKind;

/// `validateCommon` (`partyService.ts:30-38`): `name.trim()` must be non-empty, else `الاسم مطلوب`.
/// Then, when `vat_number` is a non-empty string (**untrimmed** — Q-5), it must match the settings
/// country's tax-id rule (G-10), else `format!("{label} يجب أن يكون {hint}")`. Name check runs
/// before the VAT check (mock line order).
pub(super) async fn validate_common<C: ConnectionTrait>(conn: &C, name: &str, vat_number: Option<&str>) -> TxResult<()> {
    if name.trim().is_empty() {
        return Err(AppError::validation("الاسم مطلوب").into());
    }
    if let Some(vat) = vat_number {
        if !vat.is_empty() {
            let settings = crate::core::settings::load(conn).await?;
            let rule = tax_id_rule(settings.country.as_deref());
            if !rule.matches(vat) {
                return Err(AppError::validation(rule.error_message()).into());
            }
        }
    }
    Ok(())
}

/// `clean` (`partyService.ts:40-53`): trims `name`; `nameEn`/`phone`/`email`/`address`/`vatNumber`/
/// `crNumber`/`nationalId`/`notes` trimmed with `"" -> None`. Returns the cleaned scalar fields —
/// callers (`write.rs`) assign them onto the `ActiveModel` unconditionally, exactly like the mock's
/// `Object.assign(found, data, ...)` always carrying these keys.
pub(super) struct CleanedCommon {
    pub name: String,
    pub name_en: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub vat_number: Option<String>,
    pub cr_number: Option<String>,
    pub national_id: Option<String>,
    pub notes: Option<String>,
}

fn trim_or_none(s: &Option<String>) -> Option<String> {
    s.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string())
}

pub(super) fn clean_common(name: &str, name_en: &Option<String>, phone: &Option<String>, email: &Option<String>, address: &Option<String>, vat_number: &Option<String>, cr_number: &Option<String>, national_id: &Option<String>, notes: &Option<String>) -> CleanedCommon {
    CleanedCommon {
        name: name.trim().to_string(),
        name_en: trim_or_none(name_en),
        phone: trim_or_none(phone),
        email: trim_or_none(email),
        address: trim_or_none(address),
        vat_number: trim_or_none(vat_number),
        cr_number: trim_or_none(cr_number),
        national_id: trim_or_none(national_id),
        notes: trim_or_none(notes),
    }
}

/// `nextCode` (`partyService.ts:55-63`, §3): after the caller has already taken
/// `numbering::lock(SequenceLock::CustomerCode|SupplierCode)`, scan every `code` of `kind`
/// (**including soft-deleted rows** — the `(kind, code)` unique isn't live-scoped), strip the first
/// occurrence of `prefix`, JS-trim, `"" -> 0`, else parse as a plain integer (unparsable -> skipped,
/// like a non-finite JS `Number`); take the max, format `{prefix}{max+1:0>4}`.
pub(super) async fn next_code<C: ConnectionTrait>(conn: &C, kind: PartyKind) -> TxResult<String> {
    let prefix = match kind {
        PartyKind::Customer => "C-",
        PartyKind::Supplier => "S-",
    };
    let kind_str = kind.as_str();

    // `find_including_deleted` isn't a generated finder on this entity — soft-deleted rows are
    // reached by simply not filtering on `deleted_at` (the mock's `db.customers`/`db.suppliers`
    // arrays never truly delete a row either; a "deactivated" party stays `active: false`, and the
    // few soft-deleted rows in this app never drop out of a `(kind, code)` uniqueness scan).
    let rows: Vec<parties_entity::Model> = parties_entity::Entity::find().filter(parties_entity::Column::Kind.eq(kind_str)).all(conn).await.map_err(AppError::from)?;

    let mut max: i64 = 0;
    for row in &rows {
        let stripped = row.code.replacen(prefix, "", 1);
        let trimmed = stripped.trim();
        let n: i64 = if trimmed.is_empty() { 0 } else { trimmed.parse().unwrap_or(-1) };
        if n > max {
            max = n;
        }
    }
    Ok(format!("{prefix}{:0>4}", max + 1))
}

/// §3 `with_computed`: loads a party's phones (`party_phones WHERE party_id IN (…) ORDER BY
/// party_id, position`, single-party form) and its computed balance/unallocated-credit, then builds
/// the response DTO via `dto::to_customer`/`to_supplier` (called by the caller, not here, since the
/// two kinds return different DTO types).
pub(super) async fn load_phones<C: ConnectionTrait>(conn: &C, party_id: Id) -> TxResult<Vec<party_phones::Model>> {
    let rows = party_phones::Entity::find()
        .filter(party_phones::Column::PartyId.eq(party_id))
        .order_by_asc(party_phones::Column::PartyId)
        .order_by_asc(party_phones::Column::Position)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(rows)
}

/// Batch phones loader for list screens (G-2 pairing): one query, grouped by `party_id`, each
/// group already in `(party_id, position)` order.
pub(super) async fn load_phones_batch<C: ConnectionTrait>(conn: &C, party_ids: &[Id]) -> TxResult<std::collections::HashMap<Id, Vec<party_phones::Model>>> {
    let mut out: std::collections::HashMap<Id, Vec<party_phones::Model>> = std::collections::HashMap::new();
    if party_ids.is_empty() {
        return Ok(out);
    }
    let rows = party_phones::Entity::find()
        .filter(party_phones::Column::PartyId.is_in(party_ids.iter().copied()))
        .order_by_asc(party_phones::Column::PartyId)
        .order_by_asc(party_phones::Column::Position)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    for row in rows {
        out.entry(row.party_id).or_default().push(row);
    }
    Ok(out)
}

/// Customer balance + unallocated credit (single party) — thin wrapper so callers in this domain
/// never import `shared::balances` directly for the common pair.
pub(super) async fn customer_computed<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<(rust_decimal::Decimal, rust_decimal::Decimal)> {
    let balance = balances::customer_balance(conn, id).await?;
    let credit = balances::unallocated_credit_for(conn, crate::entities::journal::journal_lines::PartyKind::Customer, id).await?;
    Ok((balance, credit))
}

pub(super) async fn supplier_computed<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<(rust_decimal::Decimal, rust_decimal::Decimal)> {
    let balance = balances::supplier_balance(conn, id).await?;
    let credit = balances::unallocated_credit_for(conn, crate::entities::journal::journal_lines::PartyKind::Supplier, id).await?;
    Ok((balance, credit))
}
