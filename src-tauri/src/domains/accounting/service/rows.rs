//! Shared mapping helpers (12-accounting.md §3.1) — used by both accounts/journal (12) and
//! period-close (12b).

use std::collections::BTreeMap;

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::tx::{TxError, TxResult};
use crate::entities::journal::journal_drafts::{Column as DraftColumn, Entity as DraftEntity, Model as DraftModel};
use crate::entities::journal::journal_draft_lines::{Column as DraftLineColumn, Entity as DraftLineEntity};
use crate::entities::journal::journal_entries::{Column as EntryColumn, Entity as EntryEntity, Model as EntryModel};
use crate::entities::journal::journal_lines::{Column as LineColumn, Entity as LineEntity};
use crate::entities::org::users::Entity as UserEntity;
use crate::entities::purchases::purchase_returns::Entity as PurchaseReturnEntity;
use crate::entities::sales::refunds::Entity as RefundEntity;
use crate::utils::id::Id;
use crate::utils::route::RouteRef;

use super::super::dto::{JournalEntry, JournalRow, JournalSourceKind};

/// `entry_dto` — a posted entry, its lines loaded and ordered by `position`.
pub async fn entry_dto<C: ConnectionTrait>(conn: &C, m: &EntryModel) -> TxResult<JournalEntry> {
    let lines = LineEntity::find().filter(LineColumn::JournalEntryId.eq(m.id)).order_by_asc(LineColumn::Position).all(conn).await.map_err(TxError::from)?;
    Ok(JournalEntry::from_model(m, &lines))
}

/// `draft_dto` — a saved draft, its lines loaded and ordered by `position`. Draft lines are always
/// resolved to the base currency by `resolvePosting`/`resolve_posting` (manual journal lines never
/// carry a foreign currency), so a line with no stored `currency` falls back to it.
pub async fn draft_dto<C: ConnectionTrait>(conn: &C, m: &DraftModel, base_currency: &str) -> TxResult<JournalEntry> {
    let lines = DraftLineEntity::find().filter(DraftLineColumn::JournalDraftId.eq(m.id)).order_by_asc(DraftLineColumn::Position).all(conn).await.map_err(TxError::from)?;
    Ok(JournalEntry::from_draft_model(m, &lines, base_currency))
}

/// `entry_dto` for many entries with one lines query (not one per entry) — `load_journal` maps the
/// whole journal, and ~900 per-entry round trips made the journal reads time out.
pub async fn entry_dtos<C: ConnectionTrait>(conn: &C, models: &[EntryModel]) -> TxResult<Vec<JournalEntry>> {
    let mut by_entry: BTreeMap<Id, Vec<crate::entities::journal::journal_lines::Model>> = BTreeMap::new();
    for chunk in models.chunks(500) {
        let ids: Vec<Id> = chunk.iter().map(|m| m.id).collect();
        let lines = LineEntity::find().filter(LineColumn::JournalEntryId.is_in(ids)).order_by_asc(LineColumn::Position).all(conn).await.map_err(TxError::from)?;
        for l in lines {
            by_entry.entry(l.journal_entry_id).or_default().push(l);
        }
    }
    Ok(models.iter().map(|m| JournalEntry::from_model(m, by_entry.get(&m.id).map(Vec::as_slice).unwrap_or(&[]))).collect())
}

/// `draft_dto` for many drafts with one lines query (see `entry_dtos`).
pub async fn draft_dtos<C: ConnectionTrait>(conn: &C, models: &[DraftModel], base_currency: &str) -> TxResult<Vec<JournalEntry>> {
    let mut by_draft: BTreeMap<Id, Vec<crate::entities::journal::journal_draft_lines::Model>> = BTreeMap::new();
    for chunk in models.chunks(500) {
        let ids: Vec<Id> = chunk.iter().map(|m| m.id).collect();
        let lines = DraftLineEntity::find().filter(DraftLineColumn::JournalDraftId.is_in(ids)).order_by_asc(DraftLineColumn::Position).all(conn).await.map_err(TxError::from)?;
        for l in lines {
            by_draft.entry(l.journal_draft_id).or_default().push(l);
        }
    }
    Ok(models.iter().map(|m| JournalEntry::from_draft_model(m, by_draft.get(&m.id).map(Vec::as_slice).unwrap_or(&[]), base_currency)).collect())
}

/// Loads several posted entries (with lines) by id, in the given order is NOT guaranteed — callers
/// that need a specific order sort the returned `Vec` themselves. Used by journal-list readers that
/// already have a filtered id set.
pub async fn load_entries<C: ConnectionTrait>(conn: &C, ids: &[Id]) -> TxResult<Vec<EntryModel>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    EntryEntity::find().filter(EntryColumn::Id.is_in(ids.to_vec())).all(conn).await.map_err(TxError::from)
}

const SOURCE_LABEL: &[(&str, &str)] = &[
    ("invoice", "فاتورة مبيعات"),
    ("refund", "مرتجع مبيعات"),
    ("purchaseOrder", "أمر شراء"),
    ("purchaseReturn", "مرتجع مشتريات"),
    ("payment", "سند"),
    ("stockAdjustment", "تسوية مخزون"),
];

fn source_label(kind: &str) -> Option<String> {
    SOURCE_LABEL.iter().find(|(k, _)| *k == kind).map(|(_, label)| label.to_string())
}

/// Batches the user-name, refund->invoice, and purchase-return->purchase-order lookups this
/// function's callers need (`to_row`'s per-entry work, done once per list call).
pub struct RowContext {
    pub user_names: BTreeMap<Id, String>,
    pub refund_invoice: BTreeMap<Id, Id>,
    pub purchase_return_order: BTreeMap<Id, Id>,
}

impl RowContext {
    pub async fn build<C: ConnectionTrait>(conn: &C, entries: &[JournalEntry]) -> TxResult<Self> {
        let user_ids: Vec<Id> = entries.iter().map(|e| e.created_by).collect();
        let users = if user_ids.is_empty() {
            Vec::new()
        } else {
            UserEntity::find().filter(crate::entities::org::users::Column::Id.is_in(user_ids)).all(conn).await.map_err(TxError::from)?
        };
        let user_names = users.into_iter().map(|u| (u.id, u.name)).collect();

        let refund_ids: Vec<Id> = entries
            .iter()
            .filter_map(|e| e.source_ref.as_ref())
            .filter(|r| r.kind == JournalSourceKind::Refund)
            .map(|r| r.id)
            .collect();
        let refund_invoice = if refund_ids.is_empty() {
            BTreeMap::new()
        } else {
            RefundEntity::find()
                .filter(crate::entities::sales::refunds::Column::Id.is_in(refund_ids))
                .all(conn)
                .await
                .map_err(TxError::from)?
                .into_iter()
                .map(|r| (r.id, r.invoice_id))
                .collect()
        };

        let pr_ids: Vec<Id> = entries
            .iter()
            .filter_map(|e| e.source_ref.as_ref())
            .filter(|r| r.kind == JournalSourceKind::PurchaseReturn)
            .map(|r| r.id)
            .collect();
        let purchase_return_order = if pr_ids.is_empty() {
            BTreeMap::new()
        } else {
            PurchaseReturnEntity::find()
                .filter(crate::entities::purchases::purchase_returns::Column::Id.is_in(pr_ids))
                .all(conn)
                .await
                .map_err(TxError::from)?
                .into_iter()
                .map(|r| (r.id, r.purchase_order_id))
                .collect()
        };

        Ok(RowContext { user_names, refund_invoice, purchase_return_order })
    }
}

/// `to_row` (`accountingService.ts:196-204`).
pub fn to_row(entry: JournalEntry, ctx: &RowContext) -> JournalRow {
    let created_by_name = ctx.user_names.get(&entry.created_by).cloned().unwrap_or_else(|| "—".to_string());
    let source_label = entry.source_ref.as_ref().and_then(|r| source_label(r.kind.as_str()));
    let source_link = source_link_for(&entry, ctx);
    JournalRow::from_entry(entry, created_by_name, source_link, source_label)
}

fn source_link_for(entry: &JournalEntry, ctx: &RowContext) -> Option<RouteRef> {
    let r = entry.source_ref.as_ref()?;
    match r.kind.as_str() {
        "invoice" => Some(RouteRef::detail("invoice", r.id.to_string())),
        "refund" => ctx.refund_invoice.get(&r.id).map(|invoice_id| RouteRef::detail("invoice", invoice_id.to_string())),
        "purchaseOrder" => Some(RouteRef::detail("purchase", r.id.to_string())),
        "purchaseReturn" => ctx.purchase_return_order.get(&r.id).map(|order_id| RouteRef::detail("purchase", order_id.to_string())),
        "payment" => Some(RouteRef::list("payments").with_query("highlight", r.id.to_string())),
        "stockAdjustment" => Some(RouteRef::detail("adjustment", r.id.to_string())),
        _ => None,
    }
}

/// Every posted + draft "entry" for the journal list, in insertion order (`allEntriesAndDrafts`,
/// `accountingService.ts:207-209`) — posted entries `ORDER BY created_at, id` first, then drafts
/// `ORDER BY created_at, id`.
pub async fn all_posted_ordered<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<EntryModel>> {
    EntryEntity::find().order_by_asc(EntryColumn::CreatedAt).order_by_asc(EntryColumn::Id).all(conn).await.map_err(TxError::from)
}

pub async fn all_drafts_ordered<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<DraftModel>> {
    DraftEntity::find().order_by_asc(DraftColumn::CreatedAt).order_by_asc(DraftColumn::Id).all(conn).await.map_err(TxError::from)
}
