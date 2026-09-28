//! `journal_entries, journal_lines, journal_drafts, journal_draft_lines, journal_templates`.
//!
//! Per-row `insert` (never `insert_many`, §3.2.6) for `journal_entries` so
//! `ActiveModelBehavior::before_save` computes `search_normalized` (P2-38).

use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait};

use crate::core::error::AppError;
use crate::core::tx::{TxError, TxResult};
use crate::entities::journal::journal_draft_lines::{ActiveModel as JournalDraftLineActiveModel, PartyKind as DraftPartyKind};
use crate::entities::journal::journal_drafts::{ActiveModel as JournalDraftActiveModel, JournalEntryType as DraftEntryType};
use crate::entities::journal::journal_entries::{ActiveModel as JournalEntryActiveModel, JournalEntryStatus, JournalEntryType};
use crate::entities::journal::journal_lines::{ActiveModel as JournalLineActiveModel, PartyKind};
use crate::entities::journal::journal_templates::{ActiveModel as JournalTemplateActiveModel, RecurrenceEvery};
use crate::entities::values::StringList;
use crate::infrastructure::import::idmap::IdMap;
use crate::infrastructure::import::model::{JournalEntryV1, JournalLineV1, JournalTemplateV1};
use crate::infrastructure::import::tables::{parse_doc_date, resolve_created_at};
use crate::utils::id::Id;
use crate::utils::money::{round2, round4};

fn parse_entry_type(kind: &str) -> JournalEntryType {
    match kind {
        "MANUAL" => JournalEntryType::Manual,
        "OPENING" => JournalEntryType::Opening,
        "CLOSING" => JournalEntryType::Closing,
        "VAT_SETTLEMENT" => JournalEntryType::VatSettlement,
        _ => JournalEntryType::System,
    }
}

fn parse_party_kind(kind: &str) -> PartyKind {
    if kind == "supplier" {
        PartyKind::Supplier
    } else {
        PartyKind::Customer
    }
}

/// Resolves a journal line's `party_id`, per §3.2.4's fixed mappings — `'onboarding'` and
/// `'onboarding-close'` source strings never appear as a *line's* party id in practice (they're
/// source-ref ids), but a line's `account_id`/`party_id` still goes through the ordinary `resolve`
/// (a dangling reference here is a real data problem the DB's FK check should catch, per §3.2.6's
/// "the database rejects it").
fn resolve_line<'a>(id_map: &IdMap, line: &'a JournalLineV1, rounded: &mut i64) -> Result<(Id, Option<Id>, Option<PartyKind>, rust_decimal::Decimal, rust_decimal::Decimal), AppError> {
    let account_id = id_map
        .resolve(&line.account_id)
        .ok_or_else(|| AppError::validation("تعذر الاستيراد: مرجع غير موجود في journal_lines"))?;
    let party_id = line.party_id.as_deref().and_then(|old| id_map.resolve(old));
    let party_kind = line.party_kind.as_deref().map(parse_party_kind);
    let debit = round2(line.debit);
    let credit = round2(line.credit);
    if debit != line.debit || credit != line.credit {
        *rounded += 1;
    }
    Ok((account_id, party_id, party_kind, debit, credit))
}

pub async fn insert_journal_entries<C: ConnectionTrait>(
    conn: &C,
    rows: &[JournalEntryV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(created_by) = id_map.resolve(&row.created_by) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في journal_entries")));
        };
        let (day, instant) = parse_doc_date(&row.date, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في journal_entries"))?;
        let posted_at = row.posted_at.as_deref().and_then(|s| parse_doc_date(s, tz));

        let total_debit = round2(row.total_debit);
        let total_credit = round2(row.total_credit);
        if total_debit != row.total_debit || total_credit != row.total_credit {
            *rounded += 1;
        }

        // §3.2.4 fixed mappings for the polymorphic source_id (B-1): 'onboarding'/'onboarding-close'
        // map to the shared constants; anything else resolves through the id map (a real imported
        // document) or mints a fresh id (a source document this snapshot didn't itself carry a row
        // for — rare, but the mock never guaranteed every source_ref target exists as a live row).
        let source_ref = row.source_ref.clone().unwrap_or_default();
        let source_id = source_ref.id.as_deref().map(|old| match old {
            "onboarding" => crate::infrastructure::import::idmap::ONBOARDING_SOURCE_ID,
            "onboarding-close" => crate::infrastructure::import::idmap::ONBOARDING_CLOSE_SOURCE_ID,
            other => id_map.resolve(other).unwrap_or_else(|| id_map.resolve_or_mint(other)),
        });

        let status = if row.status.as_deref() == Some("DRAFT") { JournalEntryStatus::Draft } else { JournalEntryStatus::Posted };

        let model = JournalEntryActiveModel {
            id: Set(id),
            number: Set(row.number.clone().unwrap_or_default()),
            date_day: Set(day),
            date_instant: Set(instant),
            description: Set(row.description.clone()),
            r#type: Set(parse_entry_type(&row.kind)),
            status: Set(status),
            source_kind: Set(source_ref.kind.clone()),
            source_id: Set(source_id),
            source_number: Set(source_ref.number.clone()),
            total_debit: Set(total_debit),
            total_credit: Set(total_credit),
            reversed: Set(row.reversed),
            // Deferred (self-reference: a reversal entry may point at an entry inserted later in
            // this same array — resolved via order::DEFERRED's ('journal_entries','reversal_of_id')).
            reversal_of_id: Set(None),
            reversal_reason: Set(row.reversal_reason.clone()),
            created_by: Set(created_by),
            posted_by: Set(row.posted_by.as_deref().and_then(|old| id_map.resolve(old))),
            posted_at_day: Set(posted_at.map(|(d, _)| d)),
            posted_at_instant: Set(posted_at.and_then(|(_, i)| i)),
            attachment_ids: Set(row.attachment_ids.clone().map(StringList)),
            template_id: Set(row.template_id.as_deref().and_then(|old| id_map.resolve(old))),
            search_normalized: sea_orm::ActiveValue::NotSet,
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::journal::journal_entries::SyncStatus::Local),
        };
        model.insert(conn).await.map_err(TxError::from)?;

        for (li, line) in row.lines.iter().enumerate() {
            let (account_id, party_id, party_kind, debit, credit) = resolve_line(id_map, line, rounded).map_err(TxError::App)?;
            let line_model = JournalLineActiveModel {
                id: Set(line.id.as_deref().map(|old| id_map.assign(old)).unwrap_or_else(Id::new)),
                journal_entry_id: Set(id),
                position: Set(li as i16),
                account_id: Set(account_id),
                description: Set(line.description.clone()),
                debit: Set(debit),
                credit: Set(credit),
                party_kind: Set(party_kind),
                party_id: Set(party_id),
                branch_id: Set(line.branch_id.as_deref().and_then(|old| id_map.resolve(old))),
                cost_center_id: Set(line.cost_center_id.as_deref().and_then(|old| id_map.resolve(old))),
                currency: Set(line.currency.clone().map(|c| c.to_uppercase())),
                amount_fc: Set(line.amount_fc.map(round2)),
                rate: Set(line.rate.map(round4)),
            };
            line_model.insert(conn).await.map_err(TxError::from)?;
        }
    }
    Ok(())
}

pub async fn insert_journal_drafts<C: ConnectionTrait>(
    conn: &C,
    rows: &[JournalEntryV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(created_by) = id_map.resolve(&row.created_by) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في journal_drafts")));
        };
        let (day, instant) = parse_doc_date(&row.date, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في journal_drafts"))?;

        let total_debit = round2(row.total_debit);
        let total_credit = round2(row.total_credit);
        if total_debit != row.total_debit || total_credit != row.total_credit {
            *rounded += 1;
        }

        let source_ref = row.source_ref.clone().unwrap_or_default();
        let source_id = source_ref.id.as_deref().map(|old| id_map.resolve(old).unwrap_or_else(|| id_map.resolve_or_mint(old)));

        let entry_type = match row.kind.as_str() {
            "MANUAL" => DraftEntryType::Manual,
            "OPENING" => DraftEntryType::Opening,
            "CLOSING" => DraftEntryType::Closing,
            "VAT_SETTLEMENT" => DraftEntryType::VatSettlement,
            _ => DraftEntryType::System,
        };

        let model = JournalDraftActiveModel {
            id: Set(id),
            number: Set(row.number.clone()),
            date_day: Set(day),
            date_instant: Set(instant),
            description: Set(row.description.clone()),
            r#type: Set(entry_type),
            source_kind: Set(source_ref.kind.clone()),
            source_id: Set(source_id),
            source_number: Set(source_ref.number.clone()),
            total_debit: Set(total_debit),
            total_credit: Set(total_credit),
            created_by: Set(created_by),
            attachment_ids: Set(row.attachment_ids.clone().map(StringList)),
            template_id: Set(row.template_id.as_deref().and_then(|old| id_map.resolve(old))),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::journal::journal_drafts::SyncStatus::Local),
        };
        model.insert(conn).await.map_err(TxError::from)?;

        for (li, line) in row.lines.iter().enumerate() {
            let Some(account_id) = id_map.resolve(&line.account_id) else {
                return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في journal_draft_lines")));
            };
            let debit = round2(line.debit);
            let credit = round2(line.credit);
            if debit != line.debit || credit != line.credit {
                *rounded += 1;
            }
            let line_model = JournalDraftLineActiveModel {
                id: Set(line.id.as_deref().map(|old| id_map.assign(old)).unwrap_or_else(Id::new)),
                journal_draft_id: Set(id),
                position: Set(li as i16),
                account_id: Set(account_id),
                description: Set(line.description.clone()),
                debit: Set(debit),
                credit: Set(credit),
                party_kind: Set(line.party_kind.as_deref().map(|k| if k == "supplier" { DraftPartyKind::Supplier } else { DraftPartyKind::Customer })),
                party_id: Set(line.party_id.as_deref().and_then(|old| id_map.resolve(old))),
                branch_id: Set(line.branch_id.as_deref().and_then(|old| id_map.resolve(old))),
                cost_center_id: Set(line.cost_center_id.as_deref().and_then(|old| id_map.resolve(old))),
                currency: Set(line.currency.clone().map(|c| c.to_uppercase())),
                amount_fc: Set(line.amount_fc.map(round2)),
                rate: Set(line.rate.map(round4)),
            };
            line_model.insert(conn).await.map_err(TxError::from)?;
        }
    }
    Ok(())
}

/// G-20: `journal_templates.name_live` is unique (`m0012`'s `uq_journal_templates_name_live`) — see
/// `tables::expenses::dedupe_names`'s doc comment for why/how; the same rule applies here.
fn dedupe_names<'a>(names: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut seen: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut out = Vec::new();
    for name in names {
        let key = name.trim().to_lowercase();
        let count = seen.entry(key).or_insert(0);
        *count += 1;
        if *count == 1 {
            out.push(name.to_string());
        } else {
            out.push(format!("{name} ({})", *count));
        }
    }
    out
}

pub async fn insert_journal_templates<C: ConnectionTrait>(
    conn: &C,
    rows: &[JournalTemplateV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
) -> TxResult<()> {
    let deduped_names = dedupe_names(rows.iter().map(|r| r.name.as_str()));
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(created_by) = id_map.resolve(&row.created_by) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في journal_templates")));
        };

        let mut lines = row.lines.clone();
        id_map.remap_json(&mut lines);

        let recurrence = row.recurrence.clone();
        let (recurrence_every, recurrence_day, recurrence_next_date, recurrence_auto_post) = match &recurrence {
            Some(r) => (
                r.get("every").and_then(|v| v.as_str()).map(|s| match s {
                    "quarter" => RecurrenceEvery::Quarter,
                    "year" => RecurrenceEvery::Year,
                    _ => RecurrenceEvery::Month,
                }),
                r.get("day").and_then(|v| v.as_i64()).map(|n| n as i8),
                r.get("nextDate").and_then(|v| v.as_str()).and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
                r.get("autoPost").and_then(|v| v.as_bool()),
            ),
            None => (None, None, None, None),
        };

        let model = JournalTemplateActiveModel {
            id: Set(id),
            name: Set(deduped_names[i].clone()),
            description: Set(row.description.clone()),
            lines: Set(lines),
            recurrence_every: Set(recurrence_every),
            recurrence_day: Set(recurrence_day),
            recurrence_next_date: Set(recurrence_next_date),
            recurrence_auto_post: Set(recurrence_auto_post),
            created_by: Set(created_by),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::journal::journal_templates::SyncStatus::Local),
            name_live: sea_orm::ActiveValue::NotSet,
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}
