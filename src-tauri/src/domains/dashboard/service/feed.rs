//! `dashboard_get_low_stock_products`, `dashboard_get_recent_invoices`, `dashboard_get_recent_activity`
//! (14-analytics.md §3.6-3.8) and the 6 notification reads (§3.11).

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::catalog::products::{Column as ProductColumn, Entity as ProductEntity, Model as ProductModel};
use crate::entities::platform::activity::{ActivityKind, Column as ActivityColumn, Entity as ActivityEntity};
use crate::entities::platform::approval_requests::{ApprovalStatus, Column as ApprovalColumn, Entity as ApprovalEntity};
use crate::entities::org::users::Entity as UserEntity;
use crate::entities::sales::invoices::Entity as InvoiceEntity;
use crate::utils::money::round2;

use super::super::dto::{RecentActivityEntry, RecentInvoice};

/// `lowStock()` (`dashboardService.ts:55-57`): live products, active, `type = product`,
/// `stock_mode != none` (NULL passes), `stock_qty <= (min_stock ?? 0)`. Filtered in Rust after one
/// `active && type = product` query — the NULL-passes `stock_mode` and the `min_stock ?? 0`
/// comparison don't translate to a single clean SQL predicate portably, and the products table is
/// small enough (this is the same table every catalog list page already loads in full).
pub async fn low_stock_query<C: ConnectionTrait>(conn: &C) -> Result<Vec<ProductModel>, AppError> {
    let rows = ProductEntity::find()
        .filter(ProductColumn::Active.eq(true))
        .filter(ProductColumn::Type.eq("product"))
        .filter(ProductColumn::DeletedAt.is_null())
        .order_by_asc(ProductColumn::CreatedAt)
        .order_by_asc(ProductColumn::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .filter(|p| p.stock_mode.as_deref() != Some("none") && p.stock_qty <= p.min_stock.unwrap_or(Decimal::ZERO))
        .collect())
}

// --- 3.6 dashboard_get_low_stock_products -----------------------------------------------------------

pub async fn get_low_stock_products<C: ConnectionTrait>(conn: &C, limit: usize) -> TxResult<Vec<crate::domains::products::dto::catalog::Product>> {
    let mut rows = low_stock_query(conn).await?;
    rows.sort_by(|a, b| {
        let ratio_a = a.stock_qty / if a.min_stock.unwrap_or(Decimal::ZERO) == Decimal::ZERO { Decimal::ONE } else { a.min_stock.unwrap() };
        let ratio_b = b.stock_qty / if b.min_stock.unwrap_or(Decimal::ZERO) == Decimal::ZERO { Decimal::ONE } else { b.min_stock.unwrap() };
        ratio_a.cmp(&ratio_b)
    });
    rows.truncate(limit);
    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        out.push(crate::domains::products::service::products::product_dto(conn, row).await?);
    }
    Ok(out)
}

// --- 3.7 dashboard_get_recent_invoices -----------------------------------------------------------------

pub async fn get_recent_invoices<C: ConnectionTrait>(conn: &C, limit: usize) -> TxResult<Vec<RecentInvoice>> {
    use crate::entities::sales::invoices::Column as InvoiceColumn;

    // All invoices, drafts included (quirk Q-5), ordered by the mock's full date-string compare.
    // **Needs from manager**: `invoices::Model` has no `date_key` field (the `m0008_sales.rs`
    // migration adds the generated `date_key` column at the DB level, but the entity in
    // `entities/sales/invoices.rs` never declares it, so there is no `Column::DateKey` to order by
    // — see this domain's `mod.rs` doc comment). Sorted in memory instead via `DocDate::key()`
    // (`utf8mb4_bin`-equivalent byte compare, matching `b.date.localeCompare(a.date)` on plain ASCII
    // date strings) with created_at/id as the tie-break, same order the DB column would have given.
    let mut rows = InvoiceEntity::find().filter(InvoiceColumn::DeletedAt.is_null()).all(conn).await.map_err(AppError::from)?;
    rows.sort_by(|a, b| b.date().key().cmp(&a.date().key()).then(b.created_at.cmp(&a.created_at)).then(b.id.cmp(&a.id)));

    let mut out = Vec::with_capacity(limit.min(rows.len()));
    for row in rows.into_iter().take(limit) {
        let invoice = crate::domains::invoices::service::common::invoice_dto(conn, &row).await?;
        let customer_name = match row.customer_id {
            Some(cid) => crate::entities::parties::parties::Entity::find_by_id(cid).one(conn).await.map_err(AppError::from)?.map(|c| c.name),
            None => None,
        };
        out.push(RecentInvoice { invoice, customer_name });
    }
    Ok(out)
}

// --- 3.8 dashboard_get_recent_activity -----------------------------------------------------------------

pub async fn get_recent_activity<C: ConnectionTrait>(conn: &C, limit: usize) -> TxResult<Vec<RecentActivityEntry>> {
    let rows = ActivityEntity::find()
        .filter(ActivityColumn::Kind.ne(ActivityKind::Auth))
        .all(conn)
        .await
        .map_err(AppError::from)?;

    let mut rows: Vec<_> = rows.into_iter().collect();
    // Stable sort by the `DocDate` key of `date` desc (mock: `b.date.localeCompare(a.date)`).
    rows.sort_by(|a, b| {
        let ka = crate::entities::doc_date::key(a.date_day, a.date_instant);
        let kb = crate::entities::doc_date::key(b.date_day, b.date_instant);
        kb.cmp(&ka)
    });

    let mut out = Vec::with_capacity(limit.min(rows.len()));
    for row in rows.into_iter().take(limit) {
        let key = crate::entities::doc_date::key(row.date_day, row.date_instant);
        let user_name = UserEntity::find_by_id(row.user_id).one(conn).await.map_err(AppError::from)?.map(|u| u.name);
        let entry = crate::core::dto::ActivityEntry {
            id: row.id,
            date: key,
            user_id: row.user_id,
            kind: row.kind.into(),
            message: row.message,
            link: row.link.map(Into::into),
        };
        out.push(RecentActivityEntry { entry, user_name });
    }
    Ok(out)
}

// --- 3.11 the six notification reads --------------------------------------------------------------

/// `getInTransitTransfers` (`ds:304`).
///
/// **Needs from manager**: `products::service::transfers::to_dto` is a private `fn` in
/// `domains/products/service/transfers.rs` (G-39 — still open in `_part02-gaps.md`). This calls a
/// `pub async fn transfer_dto` the manager exposes there (a `pub` rename of the existing `to_dto`,
/// or a thin `pub` wrapper) so this domain never re-implements the `StockTransfer` assembly.
pub async fn in_transit_transfers<C: ConnectionTrait>(conn: &C, home_branch: Option<crate::utils::id::Id>) -> TxResult<Vec<crate::domains::products::dto::inventory::StockTransfer>> {
    use crate::entities::inventory::stock_transfers::{Column as TransferColumn, Entity as TransferEntity};

    let mut query = TransferEntity::find().filter(TransferColumn::Status.eq("SENT"));
    if let Some(branch) = home_branch {
        query = query.filter(TransferColumn::ToBranchId.eq(branch));
    }
    let rows = query.order_by_asc(TransferColumn::CreatedAt).order_by_asc(TransferColumn::Id).all(conn).await.map_err(AppError::from)?;

    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        out.push(crate::domains::products::service::transfers::transfer_dto(conn, row).await?);
    }
    Ok(out)
}

/// `getPendingApprovalRequests` (`ds:309`).
pub async fn pending_approval_requests<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<crate::domains::approvals::dto::ApprovalRequest>> {
    let rows = ApprovalEntity::find()
        .filter(ApprovalColumn::Status.eq(ApprovalStatus::Pending))
        .order_by_asc(ApprovalColumn::CreatedAt)
        .order_by_asc(ApprovalColumn::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(rows.iter().map(crate::domains::approvals::dto::to_dto).collect())
}

/// `getLastBackupFailedAt` (`ds:314`).
pub async fn last_backup_failed_at<C: ConnectionTrait>(conn: &C) -> TxResult<Option<String>> {
    let settings = crate::core::settings::load(conn).await?;
    Ok(settings.backup.and_then(|b| b.last_backup_failed_at).map(crate::utils::dates::format_iso_ms))
}

/// `getJournalDraftCount` (`ds:319`).
pub async fn journal_draft_count<C: ConnectionTrait>(conn: &C) -> TxResult<i32> {
    let count = crate::entities::journal::journal_drafts::Entity::find()
        .filter(crate::entities::journal::journal_drafts::Column::DeletedAt.is_null())
        .count(conn)
        .await
        .map_err(AppError::from)?;
    Ok(count as i32)
}

/// `getStockValueSnapshot` (`ds:324`).
pub async fn stock_value_snapshot<C: ConnectionTrait>(conn: &C) -> TxResult<Decimal> {
    let rows = ProductEntity::find()
        .filter(ProductColumn::Active.eq(true))
        .filter(ProductColumn::Type.eq("product"))
        .filter(ProductColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(round2(rows.iter().map(|p| p.stock_value).sum()))
}

/// `hasAnyProducts` (`ds:329`) — active or not (the mock's own `db.products.length > 0`).
pub async fn has_any_products<C: ConnectionTrait>(conn: &C) -> TxResult<bool> {
    let count = ProductEntity::find().filter(ProductColumn::DeletedAt.is_null()).count(conn).await.map_err(AppError::from)?;
    Ok(count > 0)
}
