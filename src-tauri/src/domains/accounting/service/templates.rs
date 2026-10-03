//! Journal templates + recurring posting (12-accounting.md §3.5), porting `journal.ts:168-221`
//! and `accountingService.ts:400-451`.

use chrono::Datelike;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::core::error::{map_unique_violation, AppError};
use crate::core::lock;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::journal::journal_templates::{ActiveModel, Column, Entity, RecurrenceEvery as EntityRecurrenceEvery, SyncStatus};
use crate::entities::org::accounts::{Column as AccountColumn, Entity as AccountEntity};
use crate::entities::parties::parties::{Column as PartyColumn, Entity as PartyEntity};
use crate::entities::soft_delete::SoftDelete;
use crate::shared::activity;
use crate::shared::ledger::period::assert_open_period;
use crate::utils::id::Id;
use crate::utils::route::RouteRef;

use super::super::dto::{JournalEntryInput, JournalEntryInputLine, JournalTemplate, JournalTemplateInput};

const NAME_LIVE_CONSTRAINT: &str = "uq_journal_templates_name_live";

/// **`get_journal_templates`** (:400-403): live rows ordered by name (Arabic collation, quirk Q6),
/// then `created_at, id`.
pub async fn get_journal_templates<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<JournalTemplate>> {
    let mut rows = Entity::find_live().all(conn).await.map_err(TxError::from)?;
    rows.sort_by(|a, b| crate::utils::text::compare_ar(&a.name, &b.name).then_with(|| a.created_at.cmp(&b.created_at)).then_with(|| a.id.cmp(&b.id)));
    Ok(rows.iter().map(JournalTemplate::from_model).collect())
}

/// **`get_journal_template(id)`** (:405-410) — also used by `load_template_into_entry`.
pub async fn get_journal_template<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<JournalTemplate> {
    let row = Entity::find_live().filter(Column::Id.eq(id)).one(conn).await.map_err(TxError::from)?;
    let row = row.ok_or_else(|| AppError::not_found("القالب غير موجود"))?;
    Ok(JournalTemplate::from_model(&row))
}

async fn validate_template_lines<C: ConnectionTrait>(conn: &C, lines: &[JournalTemplateLineLike]) -> TxResult<()> {
    if lines.len() < 2 {
        return Err(TxError::App(AppError::validation("يجب أن يحتوي القالب على سطرين على الأقل")));
    }
    for line in lines {
        let account = AccountEntity::find()
            .filter(AccountColumn::Id.eq(line.account_id))
            .filter(AccountColumn::DeletedAt.is_null())
            .one(conn)
            .await
            .map_err(TxError::from)?;
        let Some(account) = account else { return Err(TxError::App(AppError::validation("اختر الحساب لكل سطر"))) };
        let non_zero = line.debit > Decimal::ZERO || line.credit > Decimal::ZERO;
        if account.requires_party.unwrap_or(false) && non_zero && line.party_id.is_none() {
            return Err(TxError::App(AppError::validation(format!("السطر على حساب \"{}\" يتطلب اختيار عميل أو مورد", account.name))));
        }
        // Template lines have no DB-level FK at all (stored as `lines_json`), so a bad party_id
        // would otherwise never be caught — not even by the generic DB error the other domains
        // fall back on. Must be checked explicitly here.
        if let Some(party_id) = line.party_id {
            let exists = PartyEntity::find_live().filter(PartyColumn::Id.eq(party_id)).one(conn).await.map_err(TxError::from)?.is_some();
            if !exists {
                return Err(TxError::App(AppError::validation("العميل أو المورد المختار غير موجود")));
            }
        }
    }
    Ok(())
}

/// A line-shaped view both `JournalTemplateLine` (saved template) and `JournalEntryInputLine`
/// (recurring-post payload, built from the template's own lines) can be validated through.
struct JournalTemplateLineLike {
    account_id: Id,
    debit: Decimal,
    credit: Decimal,
    party_id: Option<Id>,
}

/// **`save_journal_template(input, id)`** (`journal.ts:175-201`).
pub async fn save_journal_template<C: ConnectionTrait>(conn: &C, cx: &TxCtx, input: JournalTemplateInput, id: Option<Id>) -> TxResult<JournalTemplate> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(TxError::App(AppError::validation("اسم القالب مطلوب")));
    }
    let check_lines: Vec<JournalTemplateLineLike> = input
        .lines
        .iter()
        .map(|l| JournalTemplateLineLike { account_id: l.account_id, debit: l.debit, credit: l.credit, party_id: l.party_id })
        .collect();
    validate_template_lines(conn, &check_lines).await?;

    let lines_json = serde_json::to_value(&input.lines).map_err(|e| AppError::internal("تعذر حفظ سطور القالب", Some(e.to_string())))?;
    let (every, day, next_date, auto_post) = match &input.recurrence {
        Some(r) => (
            Some(EntityRecurrenceEvery::from(r.every)),
            Some(r.day as i8),
            Some(chrono::NaiveDate::parse_from_str(&r.next_date, "%Y-%m-%d").map_err(|_| AppError::validation("التاريخ غير صالح"))?),
            Some(r.auto_post),
        ),
        None => (None, None, None, None),
    };

    let saved = if let Some(id) = id {
        lock::for_update_by_id(conn, "journal_templates", &id.to_string()).await.map_err(TxError::from)?;
        let existing = Entity::find_live().filter(Column::Id.eq(id)).one(conn).await.map_err(TxError::from)?;
        let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("القالب غير موجود"))) };

        let mut model: ActiveModel = existing.into();
        model.name = Set(name.clone());
        model.description = Set(input.description.clone());
        model.lines = Set(lines_json);
        model.recurrence_every = Set(every);
        model.recurrence_day = Set(day);
        model.recurrence_next_date = Set(next_date);
        model.recurrence_auto_post = Set(auto_post);
        model
            .update(conn)
            .await
            .map_err(|e| map_unique_violation(e, NAME_LIVE_CONSTRAINT, || "اسم القالب مستخدم من قبل".to_string()))?
    } else {
        let created_by = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;
        let now = cx.clock.now;
        let model = ActiveModel {
            id: Set(Id::new()),
            name: Set(name.clone()),
            description: Set(input.description.clone()),
            lines: Set(lines_json),
            recurrence_every: Set(every),
            recurrence_day: Set(day),
            recurrence_next_date: Set(next_date),
            recurrence_auto_post: Set(auto_post),
            created_by: Set(created_by),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
            sync_status: Set(SyncStatus::Local),
            name_live: sea_orm::ActiveValue::NotSet,
        };
        model
            .insert(conn)
            .await
            .map_err(|e| map_unique_violation(e, NAME_LIVE_CONSTRAINT, || "اسم القالب مستخدم من قبل".to_string()))?
    };

    let verb = if id.is_some() { "تعديل" } else { "إضافة" };
    activity::log(
        conn,
        cx,
        &activity::UndoRegistry::new(),
        crate::entities::platform::activity::ActivityKind::Journal,
        format!("{verb} قالب القيد \"{}\"", saved.name),
        None,
        Some(RouteRef::list("journal-templates")),
    )
    .await?;

    Ok(JournalTemplate::from_model(&saved))
}

/// **`remove_journal_template(id)`** (:203-208).
pub async fn remove_journal_template<C: ConnectionTrait>(conn: &C, cx: &TxCtx, id: Id) -> TxResult<()> {
    lock::for_update_by_id(conn, "journal_templates", &id.to_string()).await.map_err(TxError::from)?;
    let existing = Entity::find_live().filter(Column::Id.eq(id)).one(conn).await.map_err(TxError::from)?;
    let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("القالب غير موجود"))) };

    Entity::soft_delete(conn, id, cx.clock.now).await.map_err(TxError::from)?;

    activity::log(
        conn,
        cx,
        &activity::UndoRegistry::new(),
        crate::entities::platform::activity::ActivityKind::Journal,
        format!("حذف قالب القيد \"{}\"", existing.name),
        None,
        Some(RouteRef::list("journal-templates")),
    )
    .await?;
    Ok(())
}

/// JS `Date` month/quarter/year rollover (quirk Q3, `advanceRecurrence`, `journal.ts:211-221`):
/// `first_of(target_month) + (day - 1) days`, reproducing `new Date(y, m, d)`'s normalization.
fn js_add_months(date: chrono::NaiveDate, n: i32) -> chrono::NaiveDate {
    let day = date.day() as i64;
    let total_months = date.year() * 12 + (date.month() as i32 - 1) + n;
    let year = total_months.div_euclid(12);
    let month0 = total_months.rem_euclid(12);
    let first_of_month = chrono::NaiveDate::from_ymd_opt(year, (month0 + 1) as u32, 1).expect("month0 is 0..=11");
    first_of_month + chrono::Duration::days(day - 1)
}

/// **`post_recurring_template(id)`** (`accountingService.ts:434-451`, `journal.ts:210-221`).
pub async fn post_recurring_template<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    undo: &activity::UndoRegistry,
    id: Id,
) -> TxResult<crate::entities::journal::journal_entries::Model> {
    // Steps 2-3 may loop once if a concurrent poster advances `next_date` between the read and the
    // lock (§3.5 step 3).
    loop {
        let template = Entity::find_live().filter(Column::Id.eq(id)).one(conn).await.map_err(TxError::from)?;
        let Some(template) = template else { return Err(TxError::App(AppError::not_found("القالب غير موجود"))) };
        let Some(next_date) = template.recurrence_next_date else { return Err(TxError::App(AppError::validation("القالب ليس متكرراً"))) };

        let lines: Vec<super::super::dto::JournalTemplateLine> = serde_json::from_value(template.lines.clone()).unwrap_or_default();
        let input_lines: Vec<JournalEntryInputLine> = lines
            .iter()
            .map(|l| JournalEntryInputLine {
                account_id: l.account_id,
                description: l.description.clone(),
                debit: l.debit,
                credit: l.credit,
                party_kind: l.party_kind,
                party_id: l.party_id,
                branch_id: None,
                cost_center_id: None,
            })
            .collect();
        super::journal::validate_manual_lines(
            conn,
            &JournalEntryInput {
                date: next_date.format("%Y-%m-%d").to_string(),
                description: if template.description.trim().is_empty() { template.name.clone() } else { template.description.clone() },
                reference: None,
                lines: input_lines.clone(),
                attachment_ids: None,
                as_draft: None,
                template_id: Some(id),
            },
        )
        .await?;

        let is_admin = super::is_admin(cx);
        assert_open_period(conn, &next_date, is_admin).await?;

        lock::for_update_by_id(conn, "journal_templates", &id.to_string()).await.map_err(TxError::from)?;
        let fresh = Entity::find_live().filter(Column::Id.eq(id)).one(conn).await.map_err(TxError::from)?;
        let Some(fresh) = fresh else { return Err(TxError::App(AppError::not_found("القالب غير موجود"))) };
        if fresh.recurrence_next_date != Some(next_date) {
            // Another terminal already posted this template and advanced it — retry with the fresh date.
            continue;
        }

        let description = if fresh.description.trim().is_empty() { fresh.name.clone() } else { fresh.description.clone() };
        let entry_dto = super::journal::record_manual_journal(
            conn,
            cx,
            undo,
            JournalEntryInput {
                date: next_date.format("%Y-%m-%d").to_string(),
                description,
                reference: None,
                lines: input_lines,
                attachment_ids: None,
                as_draft: None,
                template_id: Some(id),
            },
            "accounting.postRecurringTemplate",
        )
        .await?;

        // Advance the recurrence (quirk Q3: JS month rollover, no day clamp).
        let every = fresh.recurrence_every.clone().expect("recurrence_next_date implies recurrence_every");
        let months = match every {
            EntityRecurrenceEvery::Month => 1,
            EntityRecurrenceEvery::Quarter => 3,
            EntityRecurrenceEvery::Year => 12,
        };
        let new_next = js_add_months(next_date, months);
        let mut model: ActiveModel = fresh.into();
        model.recurrence_next_date = Set(Some(new_next));
        model.update(conn).await.map_err(TxError::from)?;

        let entry_model = crate::entities::journal::journal_entries::Entity::find_by_id(entry_dto.id)
            .one(conn)
            .await
            .map_err(TxError::from)?
            .ok_or_else(|| AppError::internal("تعذر إيجاد القيد بعد ترحيله", None))?;
        return Ok(entry_model);
    }
}

/// **`due_recurring_templates(today)`** — `pub`, for 14-analytics' `recurring-journal-due` insight.
pub async fn due_recurring_templates<C: ConnectionTrait>(conn: &C, today: chrono::NaiveDate) -> TxResult<Vec<JournalTemplate>> {
    use crate::entities::journal::journal_templates::Column as TplColumn;
    let mut rows = Entity::find_live()
        .filter(TplColumn::RecurrenceNextDate.is_not_null())
        .filter(TplColumn::RecurrenceNextDate.lte(today))
        .all(conn)
        .await
        .map_err(TxError::from)?;
    rows.sort_by(|a, b| a.created_at.cmp(&b.created_at).then_with(|| a.id.cmp(&b.id)));
    Ok(rows.iter().map(JournalTemplate::from_model).collect())
}
