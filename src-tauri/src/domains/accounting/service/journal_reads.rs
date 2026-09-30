//! Journal reads (12-accounting.md §3.4): filtered list, paged list w/ totals, detail, "for
//! source" — porting `accountingService.ts:169-275`.

use std::collections::{BTreeMap, HashSet};

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::dto::PagedQuery;
use crate::core::tx::{TxError, TxResult};
use crate::entities::journal::journal_entries::{Column as EntryColumn, Entity as EntryEntity};
use crate::entities::org::accounts::Entity as AccountEntity;
use crate::utils::id::Id;
use crate::utils::text::matches_search;

use super::super::dto::{JournalEntry, JournalEntryDetail, JournalFilter, JournalRow, LinkedJournalEntry};
use super::rows::{self, RowContext};

/// `accountIsDescendantOf` (:187-194) — builds the full parent map once, then walks it for the
/// filter's `accountId`.
async fn descendant_set<C: ConnectionTrait>(conn: &C, ancestor_id: Id) -> TxResult<HashSet<Id>> {
    let all = AccountEntity::find().all(conn).await.map_err(TxError::from)?;
    let parent_of: BTreeMap<Id, Option<Id>> = all.iter().map(|a| (a.id, a.parent_id)).collect();
    let mut out = HashSet::new();
    for a in &all {
        let mut cursor = a.parent_id;
        while let Some(p) = cursor {
            if p == ancestor_id {
                out.insert(a.id);
                break;
            }
            cursor = parent_of.get(&p).copied().flatten();
        }
    }
    Ok(out)
}

fn entry_matches(e: &JournalEntry, filter: &JournalFilter, descendants: Option<&HashSet<Id>>) -> bool {
    if let Some(t) = filter.r#type {
        if e.kind != t {
            return false;
        }
    }
    if let Some(status) = filter.status {
        if e.status != status {
            return false;
        }
    }
    if let Some(account_id) = filter.account_id {
        let matches_account = e.lines.iter().any(|l| l.account_id == account_id || descendants.is_some_and(|d| d.contains(&l.account_id)));
        if !matches_account {
            return false;
        }
    }
    if let Some(party_id) = filter.party_id {
        if !e.lines.iter().any(|l| l.party_id == Some(party_id)) {
            return false;
        }
    }
    if let Some(user_id) = filter.user_id {
        if e.created_by != user_id {
            return false;
        }
    }
    if let Some(source_kind) = filter.source_kind {
        if e.source_ref.as_ref().map(|r| r.kind.as_str()) != Some(source_kind.as_str()) {
            return false;
        }
    }
    if let Some(reversed) = filter.reversed {
        if e.reversed.unwrap_or(false) != reversed {
            return false;
        }
    }
    if let Some(has_attachments) = filter.has_attachments {
        let n = e.attachment_ids.as_ref().map(|v| !v.is_empty()).unwrap_or(false);
        if n != has_attachments {
            return false;
        }
    }
    if let Some(min) = filter.min_amount {
        if e.total_debit < min {
            return false;
        }
    }
    if let Some(max) = filter.max_amount {
        if e.total_debit > max {
            return false;
        }
    }
    // `inDateRange` on the business-day key (the mock's `localDateKey`, cross-cutting §7): compares
    // the `YYYY-MM-DD` prefix of `e.date` (already `DocDate::key()`) against the bounds as strings —
    // safe because the format is fixed-width and zero-padded.
    let day_key = &e.date[..10.min(e.date.len())];
    if let Some(from) = filter.from.as_deref() {
        if day_key < from {
            return false;
        }
    }
    if let Some(to) = filter.to.as_deref() {
        if day_key > to {
            return false;
        }
    }
    let source_number = e.source_ref.as_ref().and_then(|r| r.number.as_deref());
    if !matches_search(&[Some(e.number.as_str()), Some(e.description.as_str()), source_number], filter.search.as_deref()) {
        return false;
    }
    true
}

/// **`load_journal(filter)`** (§3.4) — posted entries (`ORDER BY created_at, id`) followed by
/// drafts (`ORDER BY created_at, id`), the mock's `allEntriesAndDrafts()` order, then filtered by
/// every predicate `matchesJournalFilter` checks.
pub async fn load_journal<C: ConnectionTrait>(conn: &C, filter: &JournalFilter) -> TxResult<Vec<JournalEntry>> {
    let posted = rows::all_posted_ordered(conn).await?;
    let drafts = rows::all_drafts_ordered(conn).await?;

    let settings = crate::core::settings::load(conn).await.map_err(TxError::App)?;
    let base_currency = settings.currency.clone();

    let mut entries = rows::entry_dtos(conn, &posted).await?;
    entries.extend(rows::draft_dtos(conn, &drafts, &base_currency).await?);

    let descendants = match filter.account_id {
        Some(id) => Some(descendant_set(conn, id).await?),
        None => None,
    };

    Ok(entries.into_iter().filter(|e| entry_matches(e, filter, descendants.as_ref())).collect())
}

/// **`get_journal_entries(filter)`** (:211-217): `load_journal`, sort by `date` key desc then
/// `number` desc, map `to_row`.
pub async fn get_journal_entries<C: ConnectionTrait>(conn: &C, filter: Option<JournalFilter>) -> TxResult<Vec<JournalRow>> {
    let filter = filter.unwrap_or_default();
    let mut entries = load_journal(conn, &filter).await?;
    entries.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| b.number.cmp(&a.number)));
    let ctx = RowContext::build(conn, &entries).await?;
    Ok(entries.into_iter().map(|e| rows::to_row(e, &ctx)).collect())
}

/// **`get_journal_entries_for_source(kind, id)`** (:226-231).
pub async fn get_journal_entries_for_source<C: ConnectionTrait>(conn: &C, source_kind: &str, source_id: &str) -> TxResult<Vec<LinkedJournalEntry>> {
    let Ok(id) = source_id.parse::<Id>() else { return Ok(Vec::new()) };
    let posted = EntryEntity::find()
        .filter(EntryColumn::SourceKind.eq(source_kind))
        .filter(EntryColumn::SourceId.eq(id))
        .all(conn)
        .await
        .map_err(TxError::from)?;
    let mut posted = posted;
    posted.sort_by(|a, b| a.created_at.cmp(&b.created_at).then_with(|| a.id.cmp(&b.id)));
    Ok(posted.into_iter().map(|e| LinkedJournalEntry { id: e.id, number: e.number, description: e.description }).collect())
}

fn sort_key_string(row: &JournalRow, key: &str) -> Option<String> {
    Some(match key {
        "id" => row.id.to_string(),
        "number" => row.number.clone(),
        "date" => row.date.clone(),
        "description" => row.description.clone(),
        "type" => format!("{:?}", row.kind),
        "status" => format!("{:?}", row.status),
        "createdBy" => row.created_by.to_string(),
        "createdByName" => row.created_by_name.clone(),
        "createdAt" => row.created_at.clone(),
        "postedBy" => row.posted_by.map(|i| i.to_string()).unwrap_or_default(),
        "postedAt" => row.posted_at.clone().unwrap_or_default(),
        "sourceLabel" => row.source_label.clone().unwrap_or_default(),
        "reversalOfId" => row.reversal_of_id.map(|i| i.to_string()).unwrap_or_default(),
        "reversalReason" => row.reversal_reason.clone().unwrap_or_default(),
        "templateId" => row.template_id.map(|i| i.to_string()).unwrap_or_default(),
        "reversed" => if row.reversed.unwrap_or(false) { "true".to_string() } else { String::new() },
        _ => return None,
    })
}

fn sort_key_number(row: &JournalRow, key: &str) -> Option<Decimal> {
    match key {
        "totalDebit" => Some(row.total_debit),
        "totalCredit" => Some(row.total_credit),
        "attachmentCount" => Some(Decimal::from(row.attachment_count)),
        _ => None,
    }
}

/// **`get_journal_entries_paged(query)`** (:234-258).
pub async fn get_journal_entries_paged<C: ConnectionTrait>(conn: &C, query: PagedQuery<JournalFilter>) -> TxResult<crate::core::dto::PagedResult<JournalRow>> {
    let filter = query.filters.clone().unwrap_or_default();
    let mut entries = load_journal(conn, &filter).await?;
    // Default order (no `sort`): `get_journal_entries`'s date/number desc. With a `sort`, the mock
    // sorts the unsorted list (posted in insertion order, then drafts), so ties — and an unknown
    // key, where every row ties — keep `load_journal`'s order, not date/number desc.
    if query.sort.is_none() {
        entries.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| b.number.cmp(&a.number)));
    }

    let ctx = RowContext::build(conn, &entries).await?;
    let mut rows: Vec<JournalRow> = entries.into_iter().map(|e| rows::to_row(e, &ctx)).collect();

    let total = rows.len() as u32;
    let total_debit: Decimal = rows.iter().fold(Decimal::ZERO, |a, r| a + r.total_debit);
    let total_credit: Decimal = rows.iter().fold(Decimal::ZERO, |a, r| a + r.total_credit);
    let mut totals = BTreeMap::new();
    totals.insert("totalDebit".to_string(), total_debit);
    totals.insert("totalCredit".to_string(), total_credit);

    if let Some(sort) = &query.sort {
        let dir = if matches!(sort.dir, crate::core::dto::SortDir::Asc) { 1i32 } else { -1i32 };
        rows.sort_by(|a, b| {
            if let (Some(na), Some(nb)) = (sort_key_number(a, &sort.key), sort_key_number(b, &sort.key)) {
                let ord = na.cmp(&nb);
                return if dir < 0 { ord.reverse() } else { ord };
            }
            let sa = sort_key_string(a, &sort.key).unwrap_or_default();
            let sb = sort_key_string(b, &sort.key).unwrap_or_default();
            let ord = sa.cmp(&sb);
            if dir < 0 { ord.reverse() } else { ord }
        });
    }

    let page = query.page.max(1);
    let page_size = query.page_size.max(1);
    let start = ((page - 1) * page_size) as i64;
    let start = if start < 0 { (rows.len() as i64 + start).max(0) as usize } else { start as usize };
    let end = (start + page_size as usize).min(rows.len());
    let page_rows = if start < rows.len() { rows[start..end].to_vec() } else { Vec::new() };

    Ok(crate::core::dto::PagedResult { rows: page_rows, total, totals: Some(totals) })
}

/// **`get_journal_entry(id)`** (:260-275).
pub async fn get_journal_entry<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<JournalEntryDetail> {
    // `allEntriesAndDrafts().find(id)`: the posted entry, else the draft — loaded by id, not by
    // mapping the whole journal (the unfiltered `load_journal` matches every row anyway).
    let entry = if let Some(m) = EntryEntity::find_by_id(id).one(conn).await.map_err(TxError::from)? {
        rows::entry_dto(conn, &m).await?
    } else if let Some(d) = crate::entities::journal::journal_drafts::Entity::find_by_id(id).one(conn).await.map_err(TxError::from)? {
        let settings = crate::core::settings::load(conn).await.map_err(TxError::App)?;
        rows::draft_dto(conn, &d, &settings.currency).await?
    } else {
        return Err(crate::core::error::AppError::not_found("القيد غير موجود").into());
    };

    let reversal = EntryEntity::find().filter(EntryColumn::ReversalOfId.eq(id)).one(conn).await.map_err(TxError::from)?;

    let related_models = if let Some(source_ref) = &entry.source_ref {
        let mut r = EntryEntity::find()
            .filter(EntryColumn::SourceKind.eq(source_ref.kind.as_str()))
            .filter(EntryColumn::SourceId.eq(source_ref.id))
            .filter(EntryColumn::Id.ne(id))
            .all(conn)
            .await
            .map_err(TxError::from)?;
        r.sort_by(|a, b| a.created_at.cmp(&b.created_at).then_with(|| a.id.cmp(&b.id)));
        r
    } else {
        Vec::new()
    };
    let mut related = Vec::with_capacity(related_models.len());
    for m in &related_models {
        related.push(rows::entry_dto(conn, m).await?);
    }

    let mut all_for_ctx = vec![entry.clone()];
    all_for_ctx.extend(related.iter().cloned());
    let ctx = RowContext::build(conn, &all_for_ctx).await?;

    let row = rows::to_row(entry, &ctx);
    let related_rows = related.into_iter().map(|e| rows::to_row(e, &ctx)).collect();

    Ok(JournalEntryDetail {
        row,
        reversed_by_id: reversal.as_ref().map(|r| r.id),
        reversed_by_number: reversal.map(|r| r.number),
        related: related_rows,
    })
}
