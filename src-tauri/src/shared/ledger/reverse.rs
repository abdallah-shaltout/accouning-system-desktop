//! `shared::ledger::reverse` (C-6): reversal = a new mirrored entry, ported from `reverseJournal`
//! (`src/mocks/backend/journal.ts:129-152`) and reopen's mirror (`core.ts:640-677`). The
//! `MANUAL`-only rule and the required-reason validation stay in the accounting domain (Part 03),
//! because the mock checks them there too (`journal.ts:133-135`) — this module only does the
//! mechanical part both callers share: lock the original, refuse a double reversal, post the
//! mirror, and stamp both entries.

use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::journal::journal_entries::{ActiveModel as EntryActiveModel, Entity as EntryEntity, JournalEntryType, Model as JournalEntry};
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

use super::post::{post, AccountRef, PartyRef, PostJournal, PostingLine};

/// Which dimensions the mirror copies from the original line. `Keep` copies branch and cost
/// center too (`reverseJournal`'s behaviour); `Default` omits them so `resolve_posting` applies the
/// normal branch/cost-center defaults instead (reopen's mirror at `core.ts:651-658`). Currency and
/// FC amounts are never copied by either path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorDims {
    Keep,
    Default,
}

/// The optional reversal reason — required by manual reversal (enforced by the accounting domain,
/// not here), optional for reopen's automatic mirror. `stamp_original` controls whether the
/// original entry's own `reversal_reason` is set too (manual reversal always does; reopen's mirror
/// at `core.ts:651-677` does not touch the original's reason field the same way).
#[derive(Debug, Clone)]
pub struct ReversalReason {
    pub text: String,
    pub stamp_original: bool,
}

/// `ReverseRequest`.
#[derive(Debug, Clone)]
pub struct ReverseRequest {
    pub original_id: Id,
    pub date: DocDate,
    pub description: String,
    pub entry_type: JournalEntryType,
    pub allow_closed_period: bool,
    pub reason: Option<ReversalReason>,
    pub dims: MirrorDims,
}

/// `reverseJournal` (`journal.ts:129-152`) mechanics: lock the original (`FOR UPDATE`), refuse a
/// row that's already reversed or is itself a reversal-of, post the mirror (debit ⇄ credit, same
/// account/description/party; branch+cost-center kept only for `MirrorDims::Keep`; currency/FC
/// never copied), link both rows (`reversal_of_id` on the mirror; `original.reversed = true`, and
/// `original.reversal_reason` only when `stamp_original`), and return the mirror.
pub async fn reverse<C: ConnectionTrait>(conn: &C, cx: &TxCtx, req: ReverseRequest) -> TxResult<JournalEntry> {
    lock::for_update_by_id(conn, "journal_entries", &req.original_id.to_string()).await?;

    let original = EntryEntity::find_by_id(req.original_id)
        .one(conn)
        .await
        .map_err(TxError::from)?
        .ok_or_else(|| AppError::not_found("القيد غير موجود"))?;

    if original.reversed || original.reversal_of_id.is_some() {
        return Err(TxError::App(AppError::validation("هذا القيد معكوس بالفعل")));
    }

    let mirror_lines = mirror_lines_for(conn, req.original_id, req.dims).await?;

    let description = req.description.clone();
    let post_req = PostJournal {
        date: req.date,
        description,
        entry_type: req.entry_type,
        source: None,
        lines: mirror_lines,
        allow_closed_period: req.allow_closed_period,
        attachment_ids: Vec::new(),
        template_id: None,
    };

    let mirror = post(conn, cx, post_req).await?;

    let mut mirror_model: EntryActiveModel = mirror.clone().into();
    mirror_model.reversal_of_id = Set(Some(original.id));
    if let Some(reason) = &req.reason {
        mirror_model.reversal_reason = Set(Some(reason.text.clone()));
    }
    let mirror = mirror_model.update(conn).await.map_err(TxError::from)?;

    let mut original_model: EntryActiveModel = original.clone().into();
    original_model.reversed = Set(true);
    if let Some(reason) = &req.reason {
        if reason.stamp_original {
            original_model.reversal_reason = Set(Some(reason.text.clone()));
        }
    }
    original_model.update(conn).await.map_err(TxError::from)?;

    Ok(mirror)
}

/// Builds the mirror's posting lines from the original entry's lines (debit ⇄ credit, keeping
/// account/description/party; branch+cost-center only under `MirrorDims::Keep`; currency/FC never
/// copied, matching both mock reversal paths).
async fn mirror_lines_for<C: ConnectionTrait>(conn: &C, original_id: Id, dims: MirrorDims) -> TxResult<Vec<PostingLine>> {
    use sea_orm::{ColumnTrait, QueryFilter, QueryOrder};

    use crate::entities::journal::journal_lines::{Column as LineColumn, Entity as LineEntity};

    let lines = LineEntity::find()
        .filter(LineColumn::JournalEntryId.eq(original_id))
        .order_by_asc(LineColumn::Position)
        .all(conn)
        .await
        .map_err(TxError::from)?;

    Ok(lines
        .into_iter()
        .map(|l| PostingLine {
            account: AccountRef::Id(l.account_id),
            debit: l.credit,
            credit: l.debit,
            description: l.description,
            party: match (l.party_kind, l.party_id) {
                (Some(kind), Some(id)) => Some(PartyRef { kind, id }),
                _ => None,
            },
            branch_id: if dims == MirrorDims::Keep { l.branch_id } else { None },
            cost_center_id: if dims == MirrorDims::Keep { l.cost_center_id } else { None },
            currency: None,
            amount_fc: None,
            rate: None,
            trace: None,
        })
        .collect())
}
