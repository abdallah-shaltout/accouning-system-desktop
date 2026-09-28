//! Voucher/settlement reads (10-vouchers.md §3.4), porting `voucherService.ts:155-194` and
//! `settlements.ts:113-117`.

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::{TxError, TxResult};
use crate::entities::payments::card_settlements::{Column as SettlementColumn, Entity as SettlementEntity};
use crate::entities::payments::vouchers::{Column, Entity, VoucherKind as EntityVoucherKind};
use crate::utils::dates::in_date_range;
use crate::utils::id::Id;
use crate::utils::text::{like_contains, normalize_arabic};

use super::super::dto::{voucher_to_dto, CardSettlement, Voucher, VoucherFilter};
use super::settlements::get_card_settlement_dto;

fn entity_kind_from_dto(kind: super::super::dto::VoucherKind) -> EntityVoucherKind {
    match kind {
        super::super::dto::VoucherKind::Receipt => EntityVoucherKind::Receipt,
        super::super::dto::VoucherKind::Payment => EntityVoucherKind::Payment,
        super::super::dto::VoucherKind::Transfer => EntityVoucherKind::Transfer,
        super::super::dto::VoucherKind::Owner => EntityVoucherKind::Owner,
    }
}

/// **3.4 `get_vouchers(filter)`** (`voucherService.ts:155-162`): `kind`, date range on the
/// business day, and `search` against `search_normalized` via a real SQL `LIKE` (P2-38) — an empty
/// normalized query applies no search filter at all. Order `date_key DESC, created_at, id` (stable
/// tie-break matching the mock's insertion-order-stable sort on `date.localeCompare`).
pub async fn get_vouchers<C: ConnectionTrait>(conn: &C, filter: Option<VoucherFilter>) -> TxResult<Vec<Voucher>> {
    let filter = filter.unwrap_or_default();

    let mut q = Entity::find();
    if let Some(kind) = filter.kind {
        q = q.filter(Column::Kind.eq(entity_kind_from_dto(kind)));
    }
    let normalized_search = Some(normalize_arabic(filter.search.as_deref())).filter(|s| !s.is_empty());
    if let Some(ref q_norm) = normalized_search {
        q = q.filter(Column::SearchNormalized.like(like_contains(q_norm)));
    }

    let mut rows = q.order_by_asc(Column::CreatedAt).order_by_asc(Column::Id).all(conn).await.map_err(TxError::from)?;

    let from = filter.from.as_deref().filter(|s| !s.is_empty());
    let to = filter.to.as_deref().filter(|s| !s.is_empty());
    rows.retain(|r| in_date_range(r.date_day, from, to));

    // `date_key DESC` on top of the stable `(created_at, id)` order above — a stable sort keeps
    // same-day ties in that insertion order, matching `b.date.localeCompare(a.date)`'s stability
    // (same convention as `expenses::get_expenses`).
    rows.sort_by(|a, b| b.date().key().cmp(&a.date().key()));

    Ok(rows.iter().map(voucher_to_dto).collect())
}

/// **3.4 `get_voucher(id)`** (`vouchers.ts:147-151`).
pub async fn get_voucher<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<Voucher> {
    let row = Entity::find_by_id(id).one(conn).await.map_err(TxError::from)?.ok_or_else(|| AppError::not_found("السند غير موجود"))?;
    Ok(voucher_to_dto(&row))
}

/// **3.4 `get_card_settlements`**: all, `date_key DESC, created_at, id`.
pub async fn get_card_settlements<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<CardSettlement>> {
    let mut rows = SettlementEntity::find()
        .order_by_asc(SettlementColumn::CreatedAt)
        .order_by_asc(SettlementColumn::Id)
        .all(conn)
        .await
        .map_err(TxError::from)?;
    rows.sort_by(|a, b| b.date().key().cmp(&a.date().key()));

    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        out.push(get_card_settlement_dto(conn, row).await?);
    }
    Ok(out)
}

/// **3.4 `get_card_settlement(id)`** (`settlements.ts:113-117`).
pub async fn get_card_settlement<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<CardSettlement> {
    let row = SettlementEntity::find_by_id(id).one(conn).await.map_err(TxError::from)?.ok_or_else(|| AppError::not_found("سند التسوية غير موجود"))?;
    get_card_settlement_dto(conn, &row).await
}
