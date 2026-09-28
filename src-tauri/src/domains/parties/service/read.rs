//! `domains::parties::service::read` — §3: `check_duplicates`, `get_party_groups`,
//! `get_customers`/`get_customer`, `get_suppliers`/`get_supplier`, `get_customer_statement`/
//! `get_supplier_statement`, `get_party_history`.

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::parties::{party_groups, party_history, party_phones, parties as parties_entity};
use crate::shared::balances;
use crate::utils::id::Id;
use crate::utils::text::{like_contains, matches_search, normalize_arabic};

use super::super::dto::{self, Customer, DuplicateCheckInput, DuplicateWarning, PartyFilter, PartyGroup, PartyHistoryEntry, PartyKind, PartyStatementRow, Supplier};
use super::{customer_computed, load_phones, load_phones_batch, supplier_computed};

/// **`check_duplicates`** (`partyService.ts:66-84`, `findDuplicates`): candidates = every live party
/// (both kinds), `WHERE phone = ? OR vat_number = ? OR id IN (SELECT party_id FROM party_phones
/// WHERE number = ?)`, ordered `(kind, created_at, id)` so customers precede suppliers exactly like
/// the mock's `[...customers, ...suppliers]` concatenation. Then exact (byte) comparison in Rust,
/// per party: skip `exclude_id`; phone warning before VAT warning.
pub async fn check_duplicates<C: ConnectionTrait>(conn: &C, input: DuplicateCheckInput, exclude_id: Option<Id>) -> TxResult<Vec<DuplicateWarning>> {
    let phone = input.phone.filter(|p| !p.is_empty());
    let vat = input.vat_number.filter(|v| !v.is_empty());
    if phone.is_none() && vat.is_none() {
        return Ok(Vec::new());
    }

    // Party ids that have a `party_phones` row matching the needle — folded into the main `OR`
    // condition below via a plain `IN (...)` over the (typically tiny) id list, rather than a
    // correlated subquery.
    let phone_child_party_ids: Vec<Id> = match &phone {
        Some(p) => party_phones::Entity::find()
            .filter(party_phones::Column::Number.eq(p.clone()))
            .all(conn)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(|row| row.party_id)
            .collect(),
        None => Vec::new(),
    };

    let mut cond = sea_orm::Condition::any();
    if let Some(p) = &phone {
        cond = cond.add(parties_entity::Column::Phone.eq(p.clone()));
        if !phone_child_party_ids.is_empty() {
            cond = cond.add(parties_entity::Column::Id.is_in(phone_child_party_ids.iter().copied()));
        }
    }
    if let Some(v) = &vat {
        cond = cond.add(parties_entity::Column::VatNumber.eq(v.clone()));
    }

    let candidates = parties_entity::Entity::find()
        .filter(cond)
        .order_by_asc(parties_entity::Column::Kind)
        .order_by_asc(parties_entity::Column::CreatedAt)
        .order_by_asc(parties_entity::Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;

    // Phone numbers stored on `party_phones` for each candidate, batched.
    let ids: Vec<Id> = candidates.iter().map(|p| p.id).collect();
    let phones_by_party = load_phones_batch(conn, &ids).await?;

    let mut warnings = Vec::new();
    for p in &candidates {
        if Some(p.id) == exclude_id {
            continue;
        }
        if let Some(needle) = &phone {
            let own_phone_matches = p.phone.as_deref() == Some(needle.as_str());
            let child_matches = phones_by_party.get(&p.id).map(|list| list.iter().any(|ph| &ph.number == needle)).unwrap_or(false);
            if own_phone_matches || child_matches {
                warnings.push(DuplicateWarning { field: "phone".to_string(), existing_id: p.id, existing_name: p.name.clone() });
            }
        }
        if let Some(needle) = &vat {
            if p.vat_number.as_deref() == Some(needle.as_str()) {
                warnings.push(DuplicateWarning { field: "vatNumber".to_string(), existing_id: p.id, existing_name: p.name.clone() });
            }
        }
    }
    Ok(warnings)
}

/// **`get_party_groups`** (`:88-91`): `WHERE kind = ? ORDER BY created_at, id`.
pub async fn get_party_groups<C: ConnectionTrait>(conn: &C, kind: PartyKind) -> TxResult<Vec<PartyGroup>> {
    let rows = party_groups::Entity::find()
        .filter(party_groups::Column::Kind.eq(kind.as_str()))
        .order_by_asc(party_groups::Column::CreatedAt)
        .order_by_asc(party_groups::Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(rows.iter().map(dto::to_party_group).collect())
}

/// **`get_customers`** (`:99-111`): SQL narrows by `kind`, live, `active = 1` unless
/// `includeInactive`, `group_id` when set, and `search_normalized LIKE %needle%` when `search` is
/// non-empty (a narrowing-only prefilter — the exact `matches_search` haystack check still runs in
/// Rust below, matching the mock's own `includesText` field list exactly). Then batch-computes
/// balance/credit and filters/searches in Rust exactly like the mock.
pub async fn get_customers<C: ConnectionTrait>(conn: &C, filter: PartyFilter) -> TxResult<Vec<Customer>> {
    let mut query = parties_entity::Entity::find().filter(parties_entity::Column::Kind.eq("customer")).filter(parties_entity::Column::DeletedAt.is_null());
    if !filter.include_inactive.unwrap_or(false) {
        query = query.filter(parties_entity::Column::Active.eq(true));
    }
    if let Some(group_id) = &filter.group_id {
        if let Ok(id) = group_id.parse::<Id>() {
            query = query.filter(parties_entity::Column::GroupId.eq(id));
        }
    }
    let search = filter.search.as_deref().filter(|s| !s.trim().is_empty());
    if let Some(s) = search {
        let needle = normalize_arabic(Some(s));
        if !needle.is_empty() {
            query = query.filter(parties_entity::Column::SearchNormalized.like(like_contains(&needle)));
        }
    }

    let rows = query.order_by_asc(parties_entity::Column::CreatedAt).order_by_asc(parties_entity::Column::Id).all(conn).await.map_err(AppError::from)?;

    let ids: Vec<Id> = rows.iter().map(|r| r.id).collect();
    let balances_map = balances::customer_balances(conn, &ids).await?;
    let credits_map = balances::unallocated_credits(conn, crate::entities::journal::journal_lines::PartyKind::Customer, &ids).await?;
    let phones_map = load_phones_batch(conn, &ids).await?;

    let with_balance_only = filter.with_balance_only.unwrap_or(false);
    let over_limit_only = filter.over_limit_only.unwrap_or(false);

    let mut out = Vec::new();
    for row in rows {
        let balance = balances_map.get(&row.id).copied().unwrap_or(rust_decimal::Decimal::ZERO);
        let credit = credits_map.get(&row.id).copied().unwrap_or(rust_decimal::Decimal::ZERO);

        if with_balance_only && !(balance > rust_decimal::Decimal::ZERO) {
            continue;
        }
        let credit_limit = row.credit_limit.unwrap_or(rust_decimal::Decimal::ZERO);
        if over_limit_only && !(credit_limit > rust_decimal::Decimal::ZERO && balance > credit_limit) {
            continue;
        }
        if let Some(s) = search {
            let haystack = [Some(row.name.as_str()), row.name_en.as_deref(), Some(row.code.as_str()), row.phone.as_deref(), row.vat_number.as_deref()];
            if !matches_search(&haystack, Some(s)) {
                continue;
            }
        }

        let phones = phones_map.get(&row.id).cloned().unwrap_or_default();
        out.push(dto::to_customer(&row, phones, balance, credit));
    }
    Ok(out)
}

/// **`get_suppliers`** (`:158-169`): same pattern; search haystack adds `contactPerson`.
pub async fn get_suppliers<C: ConnectionTrait>(conn: &C, filter: PartyFilter) -> TxResult<Vec<Supplier>> {
    let mut query = parties_entity::Entity::find().filter(parties_entity::Column::Kind.eq("supplier")).filter(parties_entity::Column::DeletedAt.is_null());
    if !filter.include_inactive.unwrap_or(false) {
        query = query.filter(parties_entity::Column::Active.eq(true));
    }
    if let Some(group_id) = &filter.group_id {
        if let Ok(id) = group_id.parse::<Id>() {
            query = query.filter(parties_entity::Column::GroupId.eq(id));
        }
    }
    let search = filter.search.as_deref().filter(|s| !s.trim().is_empty());
    if let Some(s) = search {
        let needle = normalize_arabic(Some(s));
        if !needle.is_empty() {
            query = query.filter(parties_entity::Column::SearchNormalized.like(like_contains(&needle)));
        }
    }

    let rows = query.order_by_asc(parties_entity::Column::CreatedAt).order_by_asc(parties_entity::Column::Id).all(conn).await.map_err(AppError::from)?;

    let ids: Vec<Id> = rows.iter().map(|r| r.id).collect();
    let balances_map = balances::supplier_balances(conn, &ids).await?;
    let credits_map = balances::unallocated_credits(conn, crate::entities::journal::journal_lines::PartyKind::Supplier, &ids).await?;
    let phones_map = load_phones_batch(conn, &ids).await?;

    let with_balance_only = filter.with_balance_only.unwrap_or(false);

    let mut out = Vec::new();
    for row in rows {
        let balance = balances_map.get(&row.id).copied().unwrap_or(rust_decimal::Decimal::ZERO);
        let credit = credits_map.get(&row.id).copied().unwrap_or(rust_decimal::Decimal::ZERO);

        if with_balance_only && !(balance > rust_decimal::Decimal::ZERO) {
            continue;
        }
        if let Some(s) = search {
            let haystack = [Some(row.name.as_str()), row.name_en.as_deref(), Some(row.code.as_str()), row.phone.as_deref(), row.contact_person.as_deref(), row.vat_number.as_deref()];
            if !matches_search(&haystack, Some(s)) {
                continue;
            }
        }

        let phones = phones_map.get(&row.id).cloned().unwrap_or_default();
        out.push(dto::to_supplier(&row, phones, balance, credit));
    }
    Ok(out)
}

async fn find_party<C: ConnectionTrait>(conn: &C, id: Id, kind: &str) -> TxResult<Option<parties_entity::Model>> {
    Ok(parties_entity::Entity::find_by_id(id).filter(parties_entity::Column::DeletedAt.is_null()).one(conn).await.map_err(AppError::from)?.filter(|p| p.kind == kind))
}

/// **`get_customer`** (`:113-118`): `id`+`kind` must match, else `NOT_FOUND` `العميل غير موجود`.
pub async fn get_customer<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<Customer> {
    let row = find_party(conn, id, "customer").await?.ok_or_else(|| AppError::not_found("العميل غير موجود"))?;
    let phones = load_phones(conn, id).await?;
    let (balance, credit) = customer_computed(conn, id).await?;
    Ok(dto::to_customer(&row, phones, balance, credit))
}

/// **`get_supplier`** (`:171-176`): else `NOT_FOUND` `المورد غير موجود`.
pub async fn get_supplier<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<Supplier> {
    let row = find_party(conn, id, "supplier").await?.ok_or_else(|| AppError::not_found("المورد غير موجود"))?;
    let phones = load_phones(conn, id).await?;
    let (balance, credit) = supplier_computed(conn, id).await?;
    Ok(dto::to_supplier(&row, phones, balance, credit))
}

/// **`get_customer_statement`** (`:147-150`): no existence check — an unknown id returns `[]`,
/// matching the mock (Q-6).
pub async fn get_customer_statement<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<Vec<PartyStatementRow>> {
    let rows = balances::customer_statement(conn, id).await?;
    Ok(rows.into_iter().map(PartyStatementRow::from).collect())
}

/// **`get_supplier_statement`** (`:205-208`).
pub async fn get_supplier_statement<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<Vec<PartyStatementRow>> {
    let rows = balances::supplier_statement(conn, id).await?;
    Ok(rows.into_iter().map(PartyStatementRow::from).collect())
}

/// **`get_party_history`** (`:256-259`): `WHERE party_id = ? ORDER BY created_at DESC, id ASC`.
pub async fn get_party_history<C: ConnectionTrait>(conn: &C, party_id: Id) -> TxResult<Vec<PartyHistoryEntry>> {
    let rows = party_history::Entity::find()
        .filter(party_history::Column::PartyId.eq(party_id))
        .order_by_desc(party_history::Column::CreatedAt)
        .order_by_asc(party_history::Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(rows.iter().map(dto::to_party_history_entry).collect())
}
