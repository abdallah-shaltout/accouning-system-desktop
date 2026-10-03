//! Manual journal — create/draft/post/reverse (12-accounting.md §3.3), porting `journal.ts:17-162`
//! and `core.ts:207-245`.

use sea_orm::ConnectionTrait;

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::settings::load as load_settings;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::journal::journal_lines::PartyKind as EntityPartyKind;
use crate::entities::org::accounts::{Column as AccountColumn, Entity as AccountEntity};
use crate::entities::org::cost_centers::Entity as CostCenterEntity;
use crate::entities::parties::parties::{Column as PartyColumn, Entity as PartyEntity};
use crate::entities::soft_delete::SoftDelete;
use crate::shared::activity;
use crate::shared::ledger::post::{self, AccountRef, PartyRef, PostJournal, PostingLine};
use crate::shared::ledger::reverse::{self, MirrorDims, ReversalReason, ReverseRequest};
use crate::utils::dates::RawDocDate;
use crate::utils::id::Id;
use crate::utils::route::RouteRef;

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use super::super::dto::{JournalEntry, JournalEntryInput, JournalEntryInputLine};
use crate::domains::parties::dto::PartyKind;
use crate::entities::journal::journal_entries::JournalEntryType as EntityEntryType;

/// **`validate_manual_lines`** (`journal.ts:17-40`), in the mock's exact order.
pub async fn validate_manual_lines<C: ConnectionTrait>(conn: &C, input: &JournalEntryInput) -> TxResult<()> {
    if input.description.trim().is_empty() {
        return Err(TxError::App(AppError::validation("أدخل بيان القيد")));
    }
    if input.lines.len() < 2 {
        return Err(TxError::App(AppError::validation("يجب أن يحتوي القيد على سطرين على الأقل")));
    }

    let settings = load_settings(conn).await.map_err(TxError::App)?;
    let cost_centers_on = settings.features.as_ref().and_then(|f| f.cost_centers).unwrap_or(false);

    for line in &input.lines {
        let account = AccountEntity::find()
            .filter(AccountColumn::Id.eq(line.account_id))
            .filter(AccountColumn::DeletedAt.is_null())
            .one(conn)
            .await
            .map_err(TxError::from)?;
        let Some(account) = account else { return Err(TxError::App(AppError::validation("اختر الحساب لكل سطر"))) };

        if !account.active {
            return Err(TxError::App(AppError::validation(format!("الحساب \"{}\" غير نشط", account.name))));
        }
        if account.is_group {
            return Err(TxError::App(AppError::validation(format!("\"{}\" حساب رئيسي (تجميعي) ولا يقبل الترحيل المباشر", account.name))));
        }
        if line.debit < rust_decimal::Decimal::ZERO || line.credit < rust_decimal::Decimal::ZERO {
            return Err(TxError::App(AppError::validation("المبالغ لا يمكن أن تكون سالبة")));
        }
        if line.debit > rust_decimal::Decimal::ZERO && line.credit > rust_decimal::Decimal::ZERO {
            return Err(TxError::App(AppError::validation("السطر الواحد إما مدين أو دائن")));
        }
        let non_zero = line.debit > rust_decimal::Decimal::ZERO || line.credit > rust_decimal::Decimal::ZERO;
        if non_zero && !account.allow_manual {
            return Err(TxError::App(AppError::forbidden(format!(
                "لا يمكن الترحيل يدوياً على حساب \"{}\" — استخدم تسوية المخزون أو تسوية ضريبة القيمة المضافة",
                account.name
            ))));
        }
        if account.requires_party.unwrap_or(false) && non_zero && line.party_id.is_none() {
            return Err(TxError::App(AppError::validation(format!("السطر على حساب \"{}\" يتطلب اختيار عميل أو مورد", account.name))));
        }
        if account.requires_cost_center.unwrap_or(false) && cost_centers_on && non_zero && line.cost_center_id.is_none() {
            return Err(TxError::App(AppError::validation(format!("السطر على حساب \"{}\" يتطلب اختيار مركز تكلفة", account.name))));
        }
        // A bad/stale party_id or cost_center_id must fail here with a specific message, not fall
        // through to the generic DB FK-violation error on post (same gap class as the products.rs
        // `unit-piece` bug, 2026-10-01/02).
        if let Some(party_id) = line.party_id {
            let exists = PartyEntity::find_live().filter(PartyColumn::Id.eq(party_id)).one(conn).await.map_err(TxError::from)?.is_some();
            if !exists {
                return Err(TxError::App(AppError::validation("العميل أو المورد المختار غير موجود")));
            }
        }
        if let Some(cost_center_id) = line.cost_center_id {
            let exists = CostCenterEntity::find_live().filter(crate::entities::org::cost_centers::Column::Id.eq(cost_center_id)).one(conn).await.map_err(TxError::from)?.is_some();
            if !exists {
                return Err(TxError::App(AppError::validation("مركز التكلفة المختار غير موجود")));
            }
        }
    }
    Ok(())
}

/// **`to_posting_lines`** (:76-87). A line with `party_id` but no `party_kind` reads it from the
/// `parties` row (decision A-D6).
async fn to_posting_lines<C: ConnectionTrait>(conn: &C, lines: &[JournalEntryInputLine]) -> TxResult<Vec<PostingLine>> {
    let mut out = Vec::with_capacity(lines.len());
    for l in lines {
        let party = match (l.party_id, l.party_kind) {
            (Some(id), Some(kind)) => Some(PartyRef { kind: party_kind_to_entity(kind), id }),
            (Some(id), None) => {
                let row = PartyEntity::find().filter(PartyColumn::Id.eq(id)).one(conn).await.map_err(TxError::from)?;
                row.map(|p| PartyRef { kind: party_kind_to_entity(entity_party_kind_from_str(&p.kind)), id })
            }
            (None, _) => None,
        };
        out.push(PostingLine {
            account: AccountRef::Id(l.account_id),
            debit: l.debit,
            credit: l.credit,
            description: l.description.clone(),
            party,
            branch_id: l.branch_id,
            cost_center_id: l.cost_center_id,
            currency: None,
            amount_fc: None,
            rate: None,
            trace: None,
        });
    }
    Ok(out)
}

fn party_kind_to_entity(kind: PartyKind) -> EntityPartyKind {
    match kind {
        PartyKind::Customer => EntityPartyKind::Customer,
        PartyKind::Supplier => EntityPartyKind::Supplier,
    }
}

/// `parties.kind` is stored as a plain string (`'customer' | 'supplier'`) — read it back into the
/// DTO enum for the fallback path in `to_posting_lines`.
fn entity_party_kind_from_str(s: &str) -> PartyKind {
    match s {
        "supplier" => PartyKind::Supplier,
        _ => PartyKind::Customer,
    }
}

fn parse_date(raw: &str, cx: &TxCtx) -> TxResult<crate::utils::dates::DocDate> {
    let parsed = RawDocDate::parse(raw).map_err(|_| AppError::validation("التاريخ غير صالح"))?;
    Ok(parsed.resolve(&cx.clock))
}

/// **`record_manual_journal`** (`journal.ts:89-113`).
pub async fn record_manual_journal<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    undo: &activity::UndoRegistry,
    input: JournalEntryInput,
    action_type: &'static str,
) -> TxResult<JournalEntry> {
    validate_manual_lines(conn, &input).await?;
    let date = parse_date(&input.date, cx)?;
    let description = input.description.trim().to_string();
    let lines = to_posting_lines(conn, &input.lines).await?;

    if input.as_draft.unwrap_or(false) {
        let draft = post::save_draft(conn, cx, date, description, lines, ids_from_strings(&input.attachment_ids), input.template_id).await?;
        let settings = load_settings(conn).await.map_err(TxError::App)?;
        return super::rows::draft_dto(conn, &draft, &settings.currency).await;
    }

    let is_admin = super::is_admin(cx);
    let entry = post::post(
        conn,
        cx,
        PostJournal {
            date,
            description: description.clone(),
            entry_type: EntityEntryType::Manual,
            source: None,
            lines,
            allow_closed_period: is_admin,
            attachment_ids: ids_from_strings(&input.attachment_ids),
            template_id: input.template_id,
        },
    )
    .await?;

    activity::log_undoable(
        conn,
        cx,
        undo,
        crate::entities::platform::activity::ActivityKind::Journal,
        format!("قيد يدوي {} — {}", entry.number, description),
        Some(entry.date()),
        Some(RouteRef::detail("journal-entry", entry.id.to_string())),
        activity::UndoSpec { action_type, payload: serde_json::json!({ "journalEntryId": entry.id.to_string() }) },
    )
    .await?;

    super::rows::entry_dto(conn, &entry).await
}

fn ids_from_strings(ids: &Option<Vec<String>>) -> Vec<Id> {
    ids.as_ref()
        .map(|v| v.iter().filter_map(|s| s.parse::<Id>().ok()).collect())
        .unwrap_or_default()
}

/// `createJournalEntry` (:277-280).
pub async fn create_journal_entry<C: ConnectionTrait>(conn: &C, cx: &TxCtx, undo: &activity::UndoRegistry, input: JournalEntryInput) -> TxResult<JournalEntry> {
    record_manual_journal(conn, cx, undo, input, "accounting.createJournalEntry").await
}

/// `updateJournalDraft` (`journal.ts:116-124`, `core.ts:208-224`).
pub async fn update_journal_draft<C: ConnectionTrait>(conn: &C, cx: &TxCtx, id: Id, input: JournalEntryInput) -> TxResult<JournalEntry> {
    validate_manual_lines(conn, &input).await?;
    lock::for_update_by_id(conn, "journal_drafts", &id.to_string()).await.map_err(TxError::from)?;

    let date = parse_date(&input.date, cx)?;
    let description = input.description.trim().to_string();
    let lines = to_posting_lines(conn, &input.lines).await?;
    let attachment_ids = input.attachment_ids.as_ref().map(|v| ids_from_strings(&Some(v.clone())));

    let draft = post::update_draft(conn, id, date, description, lines, attachment_ids).await?;
    let settings = load_settings(conn).await.map_err(TxError::App)?;
    super::rows::draft_dto(conn, &draft, &settings.currency).await
}

/// `postJournalDraft` (`core.ts:233-245`) — also writes the undoable `قيد يدوي …` activity row
/// (decision A-D1, required by E-5 since this action is undoable).
pub async fn post_journal_draft<C: ConnectionTrait>(conn: &C, cx: &TxCtx, undo: &activity::UndoRegistry, id: Id) -> TxResult<JournalEntry> {
    lock::for_update_by_id(conn, "journal_drafts", &id.to_string()).await.map_err(TxError::from)?;
    let is_admin = super::is_admin(cx);
    let entry = post::post_draft(conn, cx, id, is_admin).await?;

    activity::log_undoable(
        conn,
        cx,
        undo,
        crate::entities::platform::activity::ActivityKind::Journal,
        format!("قيد يدوي {} — {}", entry.number, entry.description),
        Some(entry.date()),
        Some(RouteRef::detail("journal-entry", entry.id.to_string())),
        activity::UndoSpec { action_type: "accounting.postJournalDraft", payload: serde_json::json!({ "journalEntryId": entry.id.to_string() }) },
    )
    .await?;

    super::rows::entry_dto(conn, &entry).await
}

/// `deleteJournalDraft` (`core.ts:226-230`) — no audit row (quirk Q2).
pub async fn delete_journal_draft<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<()> {
    lock::for_update_by_id(conn, "journal_drafts", &id.to_string()).await.map_err(TxError::from)?;
    post::delete_draft(conn, id).await
}

/// `reverseJournalEntry` (`journal.ts:130-162`) — the MANUAL-only rule and required reason live
/// here (phase-c C-6).
pub async fn reverse_journal_entry<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    undo: &activity::UndoRegistry,
    id: Id,
    date: crate::utils::dates::DocDate,
    reason: &str,
) -> TxResult<(JournalEntry, Id)> {
    lock::for_update_by_id(conn, "journal_entries", &id.to_string()).await.map_err(TxError::from)?;

    let original = crate::entities::journal::journal_entries::Entity::find_by_id(id)
        .one(conn)
        .await
        .map_err(TxError::from)?
        .ok_or_else(|| AppError::not_found("القيد غير موجود"))?;

    if original.r#type != EntityEntryType::Manual {
        return Err(TxError::App(AppError::validation("القيود الآلية تُعكس من المستند المصدر (مرتجع/إلغاء)")));
    }
    if original.reversed || original.reversal_of_id.is_some() {
        return Err(TxError::App(AppError::validation("هذا القيد معكوس بالفعل")));
    }
    let reason_trimmed = reason.trim();
    if reason_trimmed.is_empty() {
        return Err(TxError::App(AppError::validation("سبب العكس مطلوب")));
    }

    let is_admin = super::is_admin(cx);
    let number = original.number.clone();
    let description = original.description.clone();

    let reversal = reverse::reverse(
        conn,
        cx,
        ReverseRequest {
            original_id: id,
            date,
            description: format!("عكس القيد {number} — {description}"),
            entry_type: EntityEntryType::Manual,
            allow_closed_period: is_admin,
            reason: Some(ReversalReason { text: reason_trimmed.to_string(), stamp_original: true }),
            dims: MirrorDims::Keep,
        },
    )
    .await?;

    let audit_id = activity::log(
        conn,
        cx,
        undo,
        crate::entities::platform::activity::ActivityKind::Journal,
        format!("عكس القيد {number}"),
        Some(reversal.date()),
        Some(RouteRef::detail("journal-entry", reversal.id.to_string())),
    )
    .await?;

    let dto = super::rows::entry_dto(conn, &reversal).await?;
    Ok((dto, audit_id))
}
