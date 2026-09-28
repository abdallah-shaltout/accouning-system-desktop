//! `domains::payments::service::read` — reads (09-payments.md §3.5): `get_payments`,
//! `get_payments_paged`, `get_payment`, `get_open_documents`, and the public `payments_for_invoice`
//! read function 08 (`invoices_get_invoice`) depends on (§3.6).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::dto::{PagedQuery, PagedResult, SortDir};
use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::payments::payment_allocations::{Column as AllocColumn, Entity as AllocEntity, PaymentAllocationTargetKind};
use crate::entities::payments::payments::{Column, Entity, Model as PaymentModel, PaymentTargetType, PaymentType};
use crate::utils::dates::in_date_range;
use crate::utils::id::Id;
use crate::utils::text::matches_search;

use super::super::dto::{OpenDocument, Payment, PaymentFilter, PaymentRow, PartyKind};
use super::common::{open_documents, payment_dtos, to_row};

fn apply_filter(mut query: sea_orm::Select<Entity>, filter: &PaymentFilter) -> sea_orm::Select<Entity> {
    if let Some(t) = filter.r#type {
        query = query.filter(Column::Type.eq(PaymentType::from(t)));
    }
    if let Some(m) = filter.method {
        query = query.filter(Column::Method.eq(crate::entities::payments::payments::PaymentMethodKind::from(m)));
    }
    if let Some(target_id) = filter.target_id {
        query = query.filter(Column::TargetId.eq(target_id));
    }
    query.filter(Column::DeletedAt.is_null())
}

async fn rows_for_filter<C: ConnectionTrait>(conn: &C, filter: &PaymentFilter) -> TxResult<Vec<PaymentModel>> {
    let query = apply_filter(Entity::find(), filter);
    let rows = query.all(conn).await.map_err(AppError::from)?;
    let from = filter.from.as_deref().filter(|s| !s.is_empty());
    let to = filter.to.as_deref().filter(|s| !s.is_empty());
    Ok(rows.into_iter().filter(|p| in_date_range(p.date_day, from, to)).collect())
}

/// **`get_payments(filter)`** (`paymentService.ts:19-32`). Default order `date_key DESC,
/// created_at, id` (stable sort keeps ties in insertion order).
pub async fn get_payments<C: ConnectionTrait>(conn: &C, filter: PaymentFilter) -> TxResult<Vec<PaymentRow>> {
    let mut rows = rows_for_filter(conn, &filter).await?;
    rows.sort_by(|a, b| (a.date().key(), a.created_at, a.id).cmp(&(b.date().key(), b.created_at, b.id)));

    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        let dto = to_row(conn, row).await?;
        out.push(dto);
    }
    out.retain(|p| {
        (!filter.unallocated_only.unwrap_or(false) || p.unallocated > Decimal::new(5, 3))
            && matches_search(
                &[
                    Some(p.payment.number.as_str()),
                    Some(p.party_name.as_str()),
                    p.payment.target_ref_number.as_deref(),
                    p.payment.note.as_deref(),
                ],
                filter.search.as_deref(),
            )
    });
    out.sort_by(|a, b| b.payment.date.cmp(&a.payment.date));
    Ok(out)
}

/// **`get_payments_paged(query)`** — server-mode `DataTable` variant (§3.5): filters + paging +
/// `totals.amount` over the filtered, UNPAGED set. Sort key whitelisted over `PaymentRow` fields.
pub async fn get_payments_paged<C: ConnectionTrait>(conn: &C, query: PagedQuery<PaymentFilter>) -> TxResult<PagedResult<PaymentRow>> {
    let filter = query.filters.clone().unwrap_or_default();
    let rows = rows_for_filter(conn, &filter).await?;

    let mut dtos = Vec::with_capacity(rows.len());
    for row in &rows {
        dtos.push(to_row(conn, row).await?);
    }
    dtos.retain(|p| {
        (!filter.unallocated_only.unwrap_or(false) || p.unallocated > Decimal::new(5, 3))
            && matches_search(
                &[
                    Some(p.payment.number.as_str()),
                    Some(p.party_name.as_str()),
                    p.payment.target_ref_number.as_deref(),
                    p.payment.note.as_deref(),
                ],
                filter.search.as_deref(),
            )
    });

    let total = dtos.len() as u32;
    let total_amount: Decimal = dtos.iter().fold(Decimal::ZERO, |a, r| a + r.payment.amount);
    let mut totals = BTreeMap::new();
    totals.insert("amount".to_string(), total_amount);

    match &query.sort {
        Some(sort) => {
            let dir = sort.dir.clone();
            dtos.sort_by(|a, b| {
                let ord = match sort.key.as_str() {
                    "amount" => a.payment.amount.cmp(&b.payment.amount),
                    "allocated" => a.allocated.cmp(&b.allocated),
                    "unallocated" => a.unallocated.cmp(&b.unallocated),
                    "number" => a.payment.number.cmp(&b.payment.number),
                    "date" => a.payment.date.cmp(&b.payment.date),
                    "partyName" => a.party_name.cmp(&b.party_name),
                    "note" => a.payment.note.as_deref().unwrap_or("").cmp(b.payment.note.as_deref().unwrap_or("")),
                    "targetRefNumber" => a.payment.target_ref_number.as_deref().unwrap_or("").cmp(b.payment.target_ref_number.as_deref().unwrap_or("")),
                    _ => std::cmp::Ordering::Equal,
                };
                match &dir {
                    SortDir::Asc => ord,
                    SortDir::Desc => ord.reverse(),
                }
            });
        }
        None => dtos.sort_by(|a, b| b.payment.date.cmp(&a.payment.date)),
    }

    let page = query.page.max(1) as usize;
    let page_size = query.page_size.max(1) as usize;
    let start = (page - 1) * page_size;
    let page_rows = dtos.into_iter().skip(start).take(page_size).collect();

    Ok(PagedResult { rows: page_rows, total, totals: Some(totals) })
}

/// **`get_payment(id)`** — `NOT_FOUND` `السند غير موجود` (01.C fix).
pub async fn get_payment<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<PaymentRow> {
    let row = Entity::find()
        .filter(Column::Id.eq(id))
        .filter(Column::DeletedAt.is_null())
        .one(conn)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("السند غير موجود"))?;
    to_row(conn, &row).await
}

/// **`get_open_documents(targetType, targetId)`**.
pub async fn get_open_documents<C: ConnectionTrait>(conn: &C, target_type: PartyKind, target_id: Id) -> TxResult<Vec<OpenDocument>> {
    open_documents(conn, target_type, target_id).await
}

/// **§3.6 `payments_for_invoice(conn, invoice_id)`** (for 08 `invoices_get_invoice`): `type =
/// RECEIVED` payments having an allocation `target_kind = 'invoice' AND target_id = invoice_id`,
/// ordered `created_at, id`, as full `Payment` DTOs.
pub async fn payments_for_invoice<C: ConnectionTrait>(conn: &C, invoice_id: Id) -> TxResult<Vec<Payment>> {
    let alloc_payment_ids: Vec<Id> = AllocEntity::find()
        .filter(AllocColumn::TargetKind.eq(PaymentAllocationTargetKind::Invoice))
        .filter(AllocColumn::TargetId.eq(invoice_id))
        .all(conn)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .map(|a| a.payment_id)
        .collect();

    if alloc_payment_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut rows = Entity::find()
        .filter(Column::Id.is_in(alloc_payment_ids))
        .filter(Column::Type.eq(PaymentType::Received))
        .filter(Column::TargetType.eq(PaymentTargetType::Customer))
        .filter(Column::DeletedAt.is_null())
        .order_by_asc(Column::CreatedAt)
        .order_by_asc(Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    rows.sort_by(|a, b| (a.created_at, a.id).cmp(&(b.created_at, b.id)));

    payment_dtos(conn, &rows).await
}
