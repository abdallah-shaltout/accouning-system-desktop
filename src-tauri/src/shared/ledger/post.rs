//! `shared::ledger::post` (C-5): the mock's single posting choke point, ported from
//! `postJournal`/`resolvePosting`/`draftJournal`/`updateDraftJournal`/`deleteDraftJournal`/
//! `postDraftJournal` (`src/mocks/backend/core.ts:22-245`). This — together with `reverse.rs` — is
//! the ONLY code allowed to build an `ActiveModel` for or insert into `journal_entries`,
//! `journal_lines`, `journal_drafts`, `journal_draft_lines` (master rule 3, C-8).

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::core::error::AppError;
use crate::core::events::ChangeCategory;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::journal::journal_draft_lines::{
    ActiveModel as DraftLineActiveModel, Column as DraftLineColumn, Entity as DraftLineEntity, PartyKind as DraftPartyKind,
};
use crate::entities::journal::journal_drafts::{
    ActiveModel as DraftActiveModel, Entity as DraftEntity, JournalEntryType as DraftEntryType, Model as JournalDraft, SyncStatus as DraftSyncStatus,
};
use crate::entities::journal::journal_entries::{ActiveModel as EntryActiveModel, JournalEntryStatus, JournalEntryType, Model as JournalEntry, SyncStatus};
use crate::entities::journal::journal_lines::{ActiveModel as LineActiveModel, PartyKind};
use crate::entities::org::settings::Entity as SettingsEntity;
use crate::shared::numbering::{self, DocumentKind};
use crate::utils::dates::DocDate;
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::accounts::{self, AccountCtx, SystemRole};
use super::period::assert_open_period;
use super::trace::{LineTrace, PostingTrace};

/// The mock's own float slack (`core.ts:82`: `Math.abs(totalDebit - totalCredit) > 0.001`). Kept
/// as a named constant so the "why 0.001" isn't a magic number at the call site — `Decimal` never
/// carries float noise post-rounding, so in practice this only ever trips on a genuinely unbalanced
/// entry, exactly like the mock.
const ROUNDING_TOLERANCE: Decimal = Decimal::from_parts(1, 0, 0, false, 3);

/// `AccountRef` — a posting line resolves its account either by system role (preferred,
/// docs/v2/02-accounting-review.md F3) or by a specific id (manual journal, user-picked accounts;
/// never a hard-coded code).
#[derive(Debug, Clone)]
pub enum AccountRef {
    Role(SystemRole),
    Id(Id),
}

/// `PartyRef` (`core.ts`'s `partyKind`/`partyId` pair on `PostingLine`).
#[derive(Debug, Clone, Copy)]
pub struct PartyRef {
    pub kind: PartyKind,
    pub id: Id,
}

/// `SourceRef` (`JournalEntry.sourceRef`).
#[derive(Debug, Clone)]
pub struct SourceRef {
    pub kind: String,
    pub id: Id,
    pub number: Option<String>,
}

/// One posting line before resolution — mirrors `core.ts`'s `PostingLine` field-for-field.
#[derive(Debug, Clone)]
pub struct PostingLine {
    pub account: AccountRef,
    pub debit: Decimal,
    pub credit: Decimal,
    pub description: Option<String>,
    pub party: Option<PartyRef>,
    pub branch_id: Option<Id>,
    pub cost_center_id: Option<Id>,
    pub currency: Option<String>,
    pub amount_fc: Option<Decimal>,
    pub rate: Option<Decimal>,
    pub trace: Option<LineTrace>,
}

impl PostingLine {
    fn bare(account: AccountRef, debit: Decimal, credit: Decimal) -> Self {
        Self {
            account,
            debit,
            credit,
            description: None,
            party: None,
            branch_id: None,
            cost_center_id: None,
            currency: None,
            amount_fc: None,
            rate: None,
            trace: None,
        }
    }

    /// Convenience constructor for a debit line — every other field defaults to `None`.
    pub fn debit(account: AccountRef, amount: Decimal) -> Self {
        Self::bare(account, amount, Decimal::ZERO)
    }

    /// Convenience constructor for a credit line — every other field defaults to `None`.
    pub fn credit(account: AccountRef, amount: Decimal) -> Self {
        Self::bare(account, Decimal::ZERO, amount)
    }
}

/// `PostJournal` (`core.ts`'s `postJournal` options object).
#[derive(Debug, Clone)]
pub struct PostJournal {
    /// The full `DocDate` (day + optional instant, P2-09): the mock sometimes posts an ISO instant
    /// (`core.ts:648`), which must survive into `date_instant`/`date_key`.
    pub date: DocDate,
    pub description: String,
    pub entry_type: JournalEntryType,
    pub source: Option<SourceRef>,
    pub lines: Vec<PostingLine>,
    pub allow_closed_period: bool,
    pub attachment_ids: Vec<Id>,
    pub template_id: Option<Id>,
}

impl PostJournal {
    /// A minimal constructor for the common case (no source/attachments/template, period-checked).
    pub fn new(date: impl Into<DocDate>, description: impl Into<String>, entry_type: JournalEntryType, lines: Vec<PostingLine>) -> Self {
        Self {
            date: date.into(),
            description: description.into(),
            entry_type,
            source: None,
            lines,
            allow_closed_period: false,
            attachment_ids: Vec::new(),
            template_id: None,
        }
    }
}

/// One resolved-and-kept line, plus the input that produced it — kept together so `trace.rs` can
/// build an `accountResolution` step per line from the same pass `post`/`save_draft` already did.
pub struct ResolvedLine {
    pub input: PostingLine,
    pub account_id: Id,
    pub account_code: String,
    pub account_name: String,
    pub description: Option<String>,
    pub debit: Decimal,
    pub credit: Decimal,
    pub party: Option<PartyRef>,
    pub branch_id: Id,
    pub cost_center_id: Option<Id>,
    pub currency: String,
    pub amount_fc: Option<Decimal>,
    pub rate: Option<Decimal>,
}

pub struct ResolvedPosting {
    pub lines: Vec<ResolvedLine>,
    pub total_debit: Decimal,
    pub total_credit: Decimal,
}

/// `resolvePosting` (`core.ts:56-82`): resolves every line's account (`accounts::resolve_account`/
/// `account_by_id`), fills `branch_id`/`cost_center_id`/`currency` defaults, rounds `debit`/`credit`
/// with `round2`, drops zero lines, and asserts `Σdebit == Σcredit` (within `0.001` — the mock's own
/// float slack; `Decimal` never has float noise, so this reduces to an exact compare after
/// rounding, but the tolerance constant is kept to document the mock's rule verbatim). Refuses
/// group accounts (P2-36) — a strict addition the mock's `resolvePosting` doesn't make (that check
/// lives in the manual-journal domain, `journal.ts:24`); this is the shared choke point so every
/// caller gets it, not just the manual-journal form. Used by both `post` and the draft path (no
/// period check here — that's `assert_open_period`, called by `post`/`post_draft` only).
pub async fn resolve_posting<C: ConnectionTrait>(conn: &C, lines: Vec<PostingLine>) -> TxResult<ResolvedPosting> {
    let settings = SettingsEntity::find()
        .one(conn)
        .await
        .map_err(TxError::from)?
        .ok_or_else(|| AppError::internal("لم يتم العثور على صف الإعدادات — يجب تشغيل معالج الإعداد الأولي أولاً", None))?;
    let base_currency = settings.currency.clone();
    let default_branch_id = settings.default_branch_id;

    let mut resolved: Vec<ResolvedLine> = Vec::with_capacity(lines.len());

    for input in lines {
        let ctx = AccountCtx { branch_id: input.branch_id, currency: input.currency.clone() };
        let account = match &input.account {
            AccountRef::Role(role) => accounts::resolve_account(conn, *role, &ctx).await?,
            AccountRef::Id(id) => accounts::account_by_id(conn, *id).await?,
        };
        if account.is_group {
            return Err(TxError::App(AppError::validation(format!(
                "\"{}\" حساب رئيسي (تجميعي) ولا يقبل الترحيل المباشر",
                account.name
            ))));
        }

        let branch_id = input.branch_id.unwrap_or(default_branch_id);
        let cost_center_id = match input.cost_center_id {
            Some(id) => Some(id),
            None => branch_cost_center(conn, branch_id).await?,
        };
        let currency = input.currency.clone().unwrap_or_else(|| base_currency.clone());
        let debit = round2(input.debit);
        let credit = round2(input.credit);

        resolved.push(ResolvedLine {
            account_id: account.id,
            account_code: account.code.clone(),
            account_name: account.name.clone(),
            description: input.description.clone(),
            debit,
            credit,
            party: input.party,
            branch_id,
            cost_center_id,
            currency,
            amount_fc: input.amount_fc,
            rate: input.rate,
            input,
        });
    }

    let kept: Vec<ResolvedLine> = resolved.into_iter().filter(|l| l.debit > Decimal::ZERO || l.credit > Decimal::ZERO).collect();

    let total_debit = round2(kept.iter().fold(Decimal::ZERO, |a, l| a + l.debit));
    let total_credit = round2(kept.iter().fold(Decimal::ZERO, |a, l| a + l.credit));

    if (total_debit - total_credit).abs() > ROUNDING_TOLERANCE {
        return Err(TxError::App(AppError::unbalanced(total_debit, total_credit)));
    }

    Ok(ResolvedPosting { lines: kept, total_debit, total_credit })
}

async fn branch_cost_center<C: ConnectionTrait>(conn: &C, branch_id: Id) -> TxResult<Option<Id>> {
    use crate::entities::org::branches::{Column as BranchColumn, Entity as BranchEntity};
    let branch = BranchEntity::find().filter(BranchColumn::Id.eq(branch_id)).one(conn).await.map_err(TxError::from)?;
    Ok(branch.and_then(|b| b.cost_center_id))
}

pub(super) fn party_kind_str(kind: PartyKind) -> &'static str {
    match kind {
        PartyKind::Customer => "customer",
        PartyKind::Supplier => "supplier",
    }
}

fn draft_party_kind(kind: PartyKind) -> DraftPartyKind {
    match kind {
        PartyKind::Customer => DraftPartyKind::Customer,
        PartyKind::Supplier => DraftPartyKind::Supplier,
    }
}

/// `postJournal` (`core.ts:117-166`), in this exact order: `assert_open_period` → `resolve_posting`
/// → fewer than 2 kept lines is a `VALIDATION` → `next_number(Journal)` → insert the entry (status
/// POSTED, `posted_by = created_by`, `posted_at = cx.clock.now`) and its lines in input order →
/// `cx.touch(Ledger)` (+ `Parties` if any line carries a party, P2-12) → queue a posting trace
/// (C-7, pushed into `AppState.traces` only after this transaction commits).
pub async fn post<C: ConnectionTrait>(conn: &C, cx: &TxCtx, req: PostJournal) -> TxResult<JournalEntry> {
    let created_by = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;

    assert_open_period(conn, &req.date.day, req.allow_closed_period).await?;
    let resolved = resolve_posting(conn, req.lines).await?;

    if resolved.lines.len() < 2 {
        return Err(TxError::App(AppError::validation("يجب أن يحتوي القيد على سطرين على الأقل")));
    }

    let number = numbering::next_number(conn, DocumentKind::Journal).await?;

    let now = cx.clock.now;
    let today = cx.clock.today();
    let id = Id::new();
    let has_party = resolved.lines.iter().any(|l| l.party.is_some());

    let model = EntryActiveModel {
        id: Set(id),
        number: Set(number),
        date_day: Set(req.date.day),
        date_instant: Set(req.date.instant),
        description: Set(req.description),
        r#type: Set(req.entry_type),
        status: Set(JournalEntryStatus::Posted),
        source_kind: Set(req.source.as_ref().map(|s| s.kind.clone())),
        source_id: Set(req.source.as_ref().map(|s| s.id)),
        source_number: Set(req.source.as_ref().and_then(|s| s.number.clone())),
        total_debit: Set(resolved.total_debit),
        total_credit: Set(resolved.total_credit),
        reversed: Set(false),
        reversal_of_id: Set(None),
        reversal_reason: Set(None),
        created_by: Set(created_by),
        posted_by: Set(Some(created_by)),
        posted_at_day: Set(Some(today)),
        posted_at_instant: Set(Some(now)),
        attachment_ids: Set(non_empty_string_list(&req.attachment_ids)),
        template_id: Set(req.template_id),
        search_normalized: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(SyncStatus::Local),
    };
    let entry = model.insert(conn).await.map_err(TxError::from)?;

    insert_lines(conn, id, &resolved.lines).await?;

    cx.touch(ChangeCategory::Ledger);
    if has_party {
        cx.touch(ChangeCategory::Parties);
    }

    let trace = PostingTrace::build(req.source.as_ref().map(|s| s.kind.clone()).unwrap_or_else(|| format!("{:?}", entry.r#type)), entry.id, &resolved.lines, resolved.total_debit, resolved.total_credit, now);
    cx.push_trace(trace);

    Ok(entry)
}

async fn insert_lines<C: ConnectionTrait>(conn: &C, journal_entry_id: Id, lines: &[ResolvedLine]) -> TxResult<()> {
    for (position, line) in lines.iter().enumerate() {
        let model = LineActiveModel {
            id: Set(Id::new()),
            journal_entry_id: Set(journal_entry_id),
            position: Set(position as i16),
            account_id: Set(line.account_id),
            description: Set(line.description.clone()),
            debit: Set(line.debit),
            credit: Set(line.credit),
            party_kind: Set(line.party.map(|p| p.kind)),
            party_id: Set(line.party.map(|p| p.id)),
            branch_id: Set(Some(line.branch_id)),
            cost_center_id: Set(line.cost_center_id),
            currency: Set(Some(line.currency.clone())),
            amount_fc: Set(line.amount_fc),
            rate: Set(line.rate),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

fn non_empty_string_list(ids: &[Id]) -> Option<crate::entities::values::StringList> {
    if ids.is_empty() {
        None
    } else {
        Some(crate::entities::values::StringList(ids.iter().map(|i| i.to_string()).collect()))
    }
}

// --- Drafts (A2 in core.ts's own numbering: draftJournal/updateDraftJournal/deleteDraftJournal/
// postDraftJournal, `core.ts:168-245`) -----------------------------------------------------------

/// `draftJournal` (`core.ts:178-205`): saves a manual entry without posting it (no period check,
/// no GL effect yet) into `journal_drafts`/`journal_draft_lines` — never `journal_entries`, so
/// every ledger/balance/report reader stays unaware of it (P2-14).
pub async fn save_draft<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    date: NaiveDate,
    description: String,
    lines: Vec<PostingLine>,
    attachment_ids: Vec<Id>,
    template_id: Option<Id>,
) -> TxResult<JournalDraft> {
    let created_by = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;
    let resolved = resolve_posting(conn, lines).await?;

    let number = numbering::next_number(conn, DocumentKind::Journal).await?;
    let now = cx.clock.now;
    let id = Id::new();

    let model = DraftActiveModel {
        id: Set(id),
        number: Set(Some(number)),
        date_day: Set(date),
        date_instant: Set(None),
        description: Set(description),
        r#type: Set(DraftEntryType::Manual),
        source_kind: Set(None),
        source_id: Set(None),
        source_number: Set(None),
        total_debit: Set(resolved.total_debit),
        total_credit: Set(resolved.total_credit),
        created_by: Set(created_by),
        attachment_ids: Set(non_empty_string_list(&attachment_ids)),
        template_id: Set(template_id),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(DraftSyncStatus::Local),
    };
    let draft = model.insert(conn).await.map_err(TxError::from)?;

    insert_draft_lines(conn, id, &resolved.lines).await?;

    Ok(draft)
}

async fn insert_draft_lines<C: ConnectionTrait>(conn: &C, journal_draft_id: Id, lines: &[ResolvedLine]) -> TxResult<()> {
    for (position, line) in lines.iter().enumerate() {
        let model = DraftLineActiveModel {
            id: Set(Id::new()),
            journal_draft_id: Set(journal_draft_id),
            position: Set(position as i16),
            account_id: Set(line.account_id),
            description: Set(line.description.clone()),
            debit: Set(line.debit),
            credit: Set(line.credit),
            party_kind: Set(line.party.map(|p| draft_party_kind(p.kind))),
            party_id: Set(line.party.map(|p| p.id)),
            branch_id: Set(Some(line.branch_id)),
            cost_center_id: Set(line.cost_center_id),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

/// `updateDraftJournal` (`core.ts:207-222`): re-resolves the lines and updates the draft's
/// date/description/totals in place — replaces its child lines wholesale (delete + re-insert,
/// simplest correct port of "replace the array").
pub async fn update_draft<C: ConnectionTrait>(
    conn: &C,
    id: Id,
    date: NaiveDate,
    description: String,
    lines: Vec<PostingLine>,
    attachment_ids: Option<Vec<Id>>,
) -> TxResult<JournalDraft> {
    let existing = DraftEntity::find_by_id(id).one(conn).await.map_err(TxError::from)?.ok_or_else(|| AppError::not_found("المسودة غير موجودة"))?;
    let resolved = resolve_posting(conn, lines).await?;

    DraftLineEntity::delete_many().filter(DraftLineColumn::JournalDraftId.eq(id)).exec(conn).await.map_err(TxError::from)?;
    insert_draft_lines(conn, id, &resolved.lines).await?;

    let mut model: DraftActiveModel = existing.into();
    model.date_day = Set(date);
    model.date_instant = Set(None);
    model.description = Set(description);
    model.total_debit = Set(resolved.total_debit);
    model.total_credit = Set(resolved.total_credit);
    if let Some(ids) = attachment_ids {
        model.attachment_ids = Set(non_empty_string_list(&ids));
    }
    let updated = model.update(conn).await.map_err(TxError::from)?;
    Ok(updated)
}

/// `deleteDraftJournal` (`core.ts:224-227`): hard-deletes the draft (drafts are explicitly exempt
/// from the posted-document no-delete rule — P2-16, "drafts, held sales and worklists are still
/// hard-deleted"). Its lines cascade via the FK's `ON DELETE CASCADE`.
pub async fn delete_draft<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<()> {
    let existing = DraftEntity::find_by_id(id).one(conn).await.map_err(TxError::from)?.ok_or_else(|| AppError::not_found("المسودة غير موجودة"))?;
    DraftEntity::delete_by_id(existing.id).exec(conn).await.map_err(TxError::from)?;
    Ok(())
}

/// `postDraftJournal` (`core.ts:229-245`): moves a saved draft into the real ledger with the
/// **same id and number**, deletes the draft row (its lines cascade), and touches `Ledger`.
pub async fn post_draft<C: ConnectionTrait>(conn: &C, cx: &TxCtx, id: Id, allow_closed_period: bool) -> TxResult<JournalEntry> {
    let draft = DraftEntity::find_by_id(id).one(conn).await.map_err(TxError::from)?.ok_or_else(|| AppError::not_found("المسودة غير موجودة"))?;
    assert_open_period(conn, &draft.date_day, allow_closed_period).await?;

    let draft_lines = DraftLineEntity::find().filter(DraftLineColumn::JournalDraftId.eq(id)).all(conn).await.map_err(TxError::from)?;

    let now = cx.clock.now;
    let today = cx.clock.today();
    let number = draft.number.clone().unwrap_or_default();

    let entry_model = EntryActiveModel {
        id: Set(draft.id),
        number: Set(number),
        date_day: Set(draft.date_day),
        date_instant: Set(draft.date_instant),
        description: Set(draft.description.clone()),
        r#type: Set(map_draft_entry_type(draft.r#type.clone())),
        status: Set(JournalEntryStatus::Posted),
        source_kind: Set(draft.source_kind.clone()),
        source_id: Set(draft.source_id),
        source_number: Set(draft.source_number.clone()),
        total_debit: Set(draft.total_debit),
        total_credit: Set(draft.total_credit),
        reversed: Set(false),
        reversal_of_id: Set(None),
        reversal_reason: Set(None),
        created_by: Set(draft.created_by),
        posted_by: Set(Some(draft.created_by)),
        posted_at_day: Set(Some(today)),
        posted_at_instant: Set(Some(now)),
        attachment_ids: Set(draft.attachment_ids.clone()),
        template_id: Set(draft.template_id),
        search_normalized: Set(None),
        created_at: Set(draft.created_at),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(SyncStatus::Local),
    };
    let entry = entry_model.insert(conn).await.map_err(TxError::from)?;

    for line in draft_lines {
        let model = LineActiveModel {
            id: Set(Id::new()),
            journal_entry_id: Set(entry.id),
            position: Set(line.position),
            account_id: Set(line.account_id),
            description: Set(line.description),
            debit: Set(line.debit),
            credit: Set(line.credit),
            party_kind: Set(line.party_kind.map(map_draft_party_kind)),
            party_id: Set(line.party_id),
            branch_id: Set(line.branch_id),
            cost_center_id: Set(line.cost_center_id),
            currency: Set(None),
            amount_fc: Set(None),
            rate: Set(None),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }

    DraftEntity::delete_by_id(id).exec(conn).await.map_err(TxError::from)?;

    cx.touch(ChangeCategory::Ledger);

    Ok(entry)
}

fn map_draft_entry_type(t: DraftEntryType) -> JournalEntryType {
    match t {
        DraftEntryType::System => JournalEntryType::System,
        DraftEntryType::Manual => JournalEntryType::Manual,
        DraftEntryType::Opening => JournalEntryType::Opening,
        DraftEntryType::Closing => JournalEntryType::Closing,
        DraftEntryType::VatSettlement => JournalEntryType::VatSettlement,
    }
}

fn map_draft_party_kind(k: DraftPartyKind) -> PartyKind {
    match k {
        DraftPartyKind::Customer => PartyKind::Customer,
        DraftPartyKind::Supplier => PartyKind::Supplier,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn posting_line_debit_credit_constructors() {
        let d = PostingLine::debit(AccountRef::Role(SystemRole::Cash), dec!(100));
        assert_eq!(d.debit, dec!(100));
        assert_eq!(d.credit, Decimal::ZERO);
        assert!(d.party.is_none());

        let c = PostingLine::credit(AccountRef::Id(Id::new()), dec!(50));
        assert_eq!(c.credit, dec!(50));
        assert_eq!(c.debit, Decimal::ZERO);
    }
}
