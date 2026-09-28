//! General vouchers — receipt/payment/transfer/owner (10-vouchers.md §3.1-3.3), porting
//! `src/mocks/backend/vouchers.ts`. Each posts one balanced "fast journal" entry with a source
//! link. `record_transfer_voucher` is first in this file (08b's shift-close cash-drop depends on
//! it) and is also the only function here reachable outside this domain's own commands.

use std::str::FromStr;

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, QueryFilter, Set};

use crate::core::error::AppError;
use crate::entities::org::accounts::Model as AccountModel;
use crate::entities::org::payment_methods::{Column as PaymentMethodColumn, Entity as PaymentMethodEntity};
use crate::entities::payments::vouchers::{ActiveModel, OwnerDirection as EntityOwnerDirection, VoucherKind as EntityVoucherKind};
use crate::entities::platform::activity::ActivityKind;
use crate::entities::soft_delete::SoftDelete;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::shared::activity;
use crate::shared::ledger::accounts::{self, SystemRole};
use crate::shared::ledger::period::assert_open_period;
use crate::shared::ledger::post::{self, AccountRef, PostJournal, PostingLine, SourceRef};
use crate::shared::numbering::{self, DocumentKind};
use crate::utils::dates::{BusinessClock, DocDate, RawDocDate};
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::route::RouteRef;

use super::super::dto::{
    owner_direction_from_dto, voucher_to_dto, OwnerVoucherInput, PaymentVoucherInput, ReceiptVoucherInput, TransferVoucherInput, Voucher,
};

const AMOUNT_MUST_BE_POSITIVE: &str = "المبلغ يجب أن يكون أكبر من صفر";

fn kind_label(kind: EntityVoucherKind) -> &'static str {
    match kind {
        EntityVoucherKind::Receipt => "سند قبض عام",
        EntityVoucherKind::Payment => "سند صرف عام",
        EntityVoucherKind::Transfer => "تحويل بين الحسابات",
        EntityVoucherKind::Owner => "سند مالك",
    }
}

/// Parses a wire date string against the transaction's `BusinessClock`, same convention as
/// `expenses::parse_expense_date` (E-D5): an unparsable string is `VALIDATION`, never stored
/// verbatim like the mock would.
fn parse_voucher_date(raw: &str, clock: &BusinessClock) -> TxResult<DocDate> {
    let parsed = RawDocDate::parse(raw).map_err(|_| AppError::validation("التاريخ غير صالح"))?;
    Ok(parsed.resolve(clock))
}

/// `methodAccount` (`vouchers.ts:21-25`): a live **active** payment method by id, else
/// `VALIDATION` "اختر طريقة الدفع"; returns its `SystemRole` (PG-4) + name.
async fn method_role<C: ConnectionTrait>(conn: &C, payment_method_id: Id) -> TxResult<(SystemRole, String)> {
    let method = PaymentMethodEntity::find_live()
        .filter(PaymentMethodColumn::Id.eq(payment_method_id))
        .filter(PaymentMethodColumn::Active.eq(true))
        .one(conn)
        .await
        .map_err(TxError::from)?
        .ok_or_else(|| AppError::validation("اختر طريقة الدفع"))?;
    let role = SystemRole::from_str(&method.account_role).map_err(TxError::App)?;
    Ok((role, method.name))
}

/// `accountById` + `allowManual` check (`vouchers.ts`'s inline checks in each recorder):
/// `NOT_FOUND` when the account doesn't exist, else `VALIDATION` naming the account when it isn't
/// manual-postable — `verb` is "إلى" (receipt's credit account) or "من" (payment's debit account).
async fn manual_account<C: ConnectionTrait>(conn: &C, id: Id, verb: &str) -> TxResult<AccountModel> {
    let account = accounts::account_by_id(conn, id).await?;
    if !account.allow_manual {
        return Err(TxError::App(AppError::validation(format!("لا يمكن الترحيل يدوياً {verb} حساب \"{}\"", account.name))));
    }
    Ok(account)
}

/// Common insert (`insert_voucher`, §3.1): allocates `number` (or uses a pre-supplied one, D-V5)
/// after `assert_open_period`, stamps `amount` `round2`ed (D-V3), `created_by = actor`, and the
/// kind-specific nullable columns the caller passes in `extra`. The entity's `before_save` hook
/// fills `search_normalized` (P2-38).
#[allow(clippy::too_many_arguments)]
struct InsertVoucher {
    kind: EntityVoucherKind,
    date: DocDate,
    amount: Decimal,
    description: String,
    note: Option<String>,
    attachment_ids: Option<Vec<String>>,
    cost_center_id: Option<Id>,
    number: Option<String>,
    payment_method_id: Option<Id>,
    credit_account_id: Option<Id>,
    debit_account_id: Option<Id>,
    source_account_id: Option<Id>,
    destination_account_id: Option<Id>,
    fee_amount: Option<Decimal>,
    fee_account_id: Option<Id>,
    direction: Option<EntityOwnerDirection>,
    cash_account_id: Option<Id>,
}

async fn insert_voucher<C: ConnectionTrait>(conn: &C, cx: &TxCtx, req: InsertVoucher) -> TxResult<crate::entities::payments::vouchers::Model> {
    assert_open_period(conn, &req.date.day, false).await?;

    let number = match req.number {
        Some(n) => n,
        None => numbering::next_number(conn, DocumentKind::Voucher).await?,
    };
    let created_by = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;
    let id = Id::new();
    let now = cx.clock.now;

    let model = ActiveModel {
        id: Set(id),
        number: Set(number),
        kind: Set(req.kind),
        date_day: Set(req.date.day),
        date_instant: Set(req.date.instant),
        amount: Set(round2(req.amount)),
        description: Set(req.description),
        note: Set(req.note),
        attachment_ids: Set(req.attachment_ids.map(crate::entities::values::StringList)),
        cost_center_id: Set(req.cost_center_id),
        created_by: Set(created_by),
        payment_method_id: Set(req.payment_method_id),
        credit_account_id: Set(req.credit_account_id),
        debit_account_id: Set(req.debit_account_id),
        source_account_id: Set(req.source_account_id),
        destination_account_id: Set(req.destination_account_id),
        fee_amount: Set(req.fee_amount),
        fee_account_id: Set(req.fee_account_id),
        direction: Set(req.direction),
        cash_account_id: Set(req.cash_account_id),
        search_normalized: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(crate::entities::payments::vouchers::SyncStatus::Local),
    };
    model.insert(conn).await.map_err(TxError::from)
}

/// `log_voucher` (§3.1): "{KIND_LABEL} {number} بقيمة {amount:.2}" -> `voucher-detail`.
async fn log_voucher<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::undo::UndoRegistry,
    kind: EntityVoucherKind,
    number: &str,
    amount: Decimal,
    date: DocDate,
    id: Id,
) -> TxResult<()> {
    activity::record::log(
        conn,
        cx,
        registry,
        ActivityKind::Voucher,
        format!("{} {number} بقيمة {:.2}", kind_label(kind), round2(amount)),
        Some(date),
        Some(RouteRef::detail("voucher-detail", id.to_string())),
    )
    .await?;
    Ok(())
}

/// **3.2 Receipt voucher** (`vouchers.ts:32-53`) — Dr method account / Cr the chosen (manual)
/// account.
pub async fn create_receipt_voucher<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::undo::UndoRegistry,
    input: ReceiptVoucherInput,
) -> TxResult<Voucher> {
    if !(input.amount > Decimal::ZERO) {
        return Err(TxError::App(AppError::validation(AMOUNT_MUST_BE_POSITIVE)));
    }
    let (method_system_role, _method_name) = method_role(conn, input.payment_method_id).await?;
    let credit_account = manual_account(conn, input.credit_account_id, "إلى").await?;

    let date = parse_voucher_date(&input.date, &cx.clock)?;
    let inserted = insert_voucher(
        conn,
        cx,
        InsertVoucher {
            kind: EntityVoucherKind::Receipt,
            date,
            amount: input.amount,
            description: input.description.clone(),
            note: input.note.clone(),
            attachment_ids: input.attachment_ids.clone(),
            cost_center_id: input.cost_center_id,
            number: None,
            payment_method_id: Some(input.payment_method_id),
            credit_account_id: Some(input.credit_account_id),
            debit_account_id: None,
            source_account_id: None,
            destination_account_id: None,
            fee_amount: None,
            fee_account_id: None,
            direction: None,
            cash_account_id: None,
        },
    )
    .await?;

    let lines = vec![
        PostingLine::debit(AccountRef::Role(method_system_role), input.amount),
        PostingLine { cost_center_id: input.cost_center_id, ..PostingLine::credit(AccountRef::Id(credit_account.id), input.amount) },
    ];
    let attachment_ids = parse_attachment_ids(&input.attachment_ids)?;
    post::post(
        conn,
        cx,
        PostJournal {
            date,
            description: format!("{} {} — {}", kind_label(EntityVoucherKind::Receipt), inserted.number, input.description),
            entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
            source: Some(SourceRef { kind: "voucher".to_string(), id: inserted.id, number: Some(inserted.number.clone()) }),
            lines,
            allow_closed_period: false,
            attachment_ids,
            template_id: None,
        },
    )
    .await?;

    log_voucher(conn, cx, registry, EntityVoucherKind::Receipt, &inserted.number, input.amount, date, inserted.id).await?;
    Ok(voucher_to_dto(&inserted))
}

/// **3.2 Payment voucher** (`vouchers.ts:56-77`) — Dr the chosen (manual) account / Cr method
/// account.
pub async fn create_payment_voucher<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::undo::UndoRegistry,
    input: PaymentVoucherInput,
) -> TxResult<Voucher> {
    if !(input.amount > Decimal::ZERO) {
        return Err(TxError::App(AppError::validation(AMOUNT_MUST_BE_POSITIVE)));
    }
    let (method_system_role, _method_name) = method_role(conn, input.payment_method_id).await?;
    let debit_account = manual_account(conn, input.debit_account_id, "من").await?;

    let date = parse_voucher_date(&input.date, &cx.clock)?;
    let inserted = insert_voucher(
        conn,
        cx,
        InsertVoucher {
            kind: EntityVoucherKind::Payment,
            date,
            amount: input.amount,
            description: input.description.clone(),
            note: input.note.clone(),
            attachment_ids: input.attachment_ids.clone(),
            cost_center_id: input.cost_center_id,
            number: None,
            payment_method_id: Some(input.payment_method_id),
            credit_account_id: None,
            debit_account_id: Some(input.debit_account_id),
            source_account_id: None,
            destination_account_id: None,
            fee_amount: None,
            fee_account_id: None,
            direction: None,
            cash_account_id: None,
        },
    )
    .await?;

    let lines = vec![
        PostingLine { cost_center_id: input.cost_center_id, ..PostingLine::debit(AccountRef::Id(debit_account.id), input.amount) },
        PostingLine::credit(AccountRef::Role(method_system_role), input.amount),
    ];
    let attachment_ids = parse_attachment_ids(&input.attachment_ids)?;
    post::post(
        conn,
        cx,
        PostJournal {
            date,
            description: format!("{} {} — {}", kind_label(EntityVoucherKind::Payment), inserted.number, input.description),
            entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
            source: Some(SourceRef { kind: "voucher".to_string(), id: inserted.id, number: Some(inserted.number.clone()) }),
            lines,
            allow_closed_period: false,
            attachment_ids,
            template_id: None,
        },
    )
    .await?;

    log_voucher(conn, cx, registry, EntityVoucherKind::Payment, &inserted.number, input.amount, date, inserted.id).await?;
    Ok(voucher_to_dto(&inserted))
}

/// **3.3 `record_transfer_voucher`** (`vouchers.ts:88-117`) — public so the invoices domain's
/// shift-close cash-drop (08b) can call it directly with a pre-allocated `number` (D-V5); everything
/// else about it is the mock's function. `vouchers_create_transfer_voucher` calls this with
/// `number = None`.
pub async fn record_transfer_voucher<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::undo::UndoRegistry,
    input: TransferVoucherInput,
    number: Option<String>,
) -> TxResult<Voucher> {
    if !(input.amount > Decimal::ZERO) {
        return Err(TxError::App(AppError::validation(AMOUNT_MUST_BE_POSITIVE)));
    }
    if input.source_account_id == input.destination_account_id {
        return Err(TxError::App(AppError::validation("اختر حسابين مختلفين للتحويل")));
    }
    let source = accounts::account_by_id(conn, input.source_account_id).await?;
    let destination = accounts::account_by_id(conn, input.destination_account_id).await?;

    let fee = round2(input.fee_amount.unwrap_or(Decimal::ZERO));
    if fee > Decimal::ZERO && input.fee_account_id.is_none() {
        return Err(TxError::App(AppError::validation("اختر حساب العمولة")));
    }

    let date = parse_voucher_date(&input.date, &cx.clock)?;
    let stored_fee_amount = if fee > Decimal::ZERO { Some(fee) } else { None };
    let inserted = insert_voucher(
        conn,
        cx,
        InsertVoucher {
            kind: EntityVoucherKind::Transfer,
            date,
            amount: input.amount,
            description: input.description.clone(),
            note: input.note.clone(),
            attachment_ids: input.attachment_ids.clone(),
            cost_center_id: input.cost_center_id,
            number,
            payment_method_id: None,
            credit_account_id: None,
            debit_account_id: None,
            source_account_id: Some(input.source_account_id),
            destination_account_id: Some(input.destination_account_id),
            // Q-V1 kept: the voucher row stores `feeAccountId` exactly as sent, even when the fee
            // rounds to 0 — only `fee_amount` collapses to `None` when it's not positive.
            fee_amount: stored_fee_amount,
            fee_account_id: input.fee_account_id,
            direction: None,
            cash_account_id: None,
        },
    )
    .await?;

    let mut lines = vec![
        PostingLine::debit(AccountRef::Id(destination.id), input.amount),
        PostingLine::credit(AccountRef::Id(source.id), round2(input.amount + fee)),
    ];
    if fee > Decimal::ZERO {
        // `account_by_id` inside `post::resolve_posting` -> NOT_FOUND if the fee account is
        // missing, matching the mock's `resolvePosting` behaviour (§3.3 step 6).
        lines.push(PostingLine::debit(AccountRef::Id(input.fee_account_id.expect("checked above")), fee));
    }

    let attachment_ids = parse_attachment_ids(&input.attachment_ids)?;
    post::post(
        conn,
        cx,
        PostJournal {
            date,
            description: format!("{} {} — {}", kind_label(EntityVoucherKind::Transfer), inserted.number, input.description),
            entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
            source: Some(SourceRef { kind: "voucher".to_string(), id: inserted.id, number: Some(inserted.number.clone()) }),
            lines,
            allow_closed_period: false,
            attachment_ids,
            template_id: None,
        },
    )
    .await?;

    log_voucher(conn, cx, registry, EntityVoucherKind::Transfer, &inserted.number, input.amount, date, inserted.id).await?;
    Ok(voucher_to_dto(&inserted))
}

/// `vouchers_create_transfer_voucher` — always allocates its own number.
pub async fn create_transfer_voucher<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::undo::UndoRegistry,
    input: TransferVoucherInput,
) -> TxResult<Voucher> {
    record_transfer_voucher(conn, cx, registry, input, None).await
}

/// **3.2 Owner voucher** (`vouchers.ts:120-145`) — drawings: Dr drawings / Cr cash; contribution:
/// the reverse.
pub async fn create_owner_voucher<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::undo::UndoRegistry,
    input: OwnerVoucherInput,
) -> TxResult<Voucher> {
    if !(input.amount > Decimal::ZERO) {
        return Err(TxError::App(AppError::validation(AMOUNT_MUST_BE_POSITIVE)));
    }
    let cash_account = accounts::account_by_id(conn, input.cash_account_id).await?;
    let drawings_account = accounts::resolve_account(conn, SystemRole::Drawings, &accounts::AccountCtx::default()).await?;

    let date = parse_voucher_date(&input.date, &cx.clock)?;
    let entity_direction = owner_direction_from_dto(input.direction);
    let inserted = insert_voucher(
        conn,
        cx,
        InsertVoucher {
            kind: EntityVoucherKind::Owner,
            date,
            amount: input.amount,
            description: input.description.clone(),
            note: input.note.clone(),
            attachment_ids: input.attachment_ids.clone(),
            cost_center_id: input.cost_center_id,
            number: None,
            payment_method_id: None,
            credit_account_id: None,
            debit_account_id: None,
            source_account_id: None,
            destination_account_id: None,
            fee_amount: None,
            fee_account_id: None,
            direction: Some(entity_direction.clone()),
            cash_account_id: Some(input.cash_account_id),
        },
    )
    .await?;

    let lines = match &entity_direction {
        EntityOwnerDirection::Drawings => vec![
            PostingLine::debit(AccountRef::Id(drawings_account.id), input.amount),
            PostingLine::credit(AccountRef::Id(cash_account.id), input.amount),
        ],
        EntityOwnerDirection::Contribution => vec![
            PostingLine::debit(AccountRef::Id(cash_account.id), input.amount),
            PostingLine::credit(AccountRef::Id(drawings_account.id), input.amount),
        ],
    };

    let suffix = match &entity_direction {
        EntityOwnerDirection::Drawings => "مسحوبات شخصية",
        EntityOwnerDirection::Contribution => "إضافة رأس مال",
    };
    let attachment_ids = parse_attachment_ids(&input.attachment_ids)?;
    post::post(
        conn,
        cx,
        PostJournal {
            date,
            description: format!("{} {} — {suffix}", kind_label(EntityVoucherKind::Owner), inserted.number),
            entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
            source: Some(SourceRef { kind: "voucher".to_string(), id: inserted.id, number: Some(inserted.number.clone()) }),
            lines,
            allow_closed_period: false,
            attachment_ids,
            template_id: None,
        },
    )
    .await?;

    log_voucher(conn, cx, registry, EntityVoucherKind::Owner, &inserted.number, input.amount, date, inserted.id).await?;
    Ok(voucher_to_dto(&inserted))
}

/// Attachment ids are UUID strings (`Id`s) on the wire — a value that doesn't parse as one is a
/// client bug, not a business error the user can retry differently (P2-23's "should be impossible"
/// convention, same as `expenses::record_expense`).
fn parse_attachment_ids(ids: &Option<Vec<String>>) -> TxResult<Vec<Id>> {
    match ids {
        Some(ids) => ids
            .iter()
            .map(|s| s.parse::<Id>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TxError::App(AppError::internal("معرّف مرفق غير صالح", Some(e.to_string())))),
        None => Ok(Vec::new()),
    }
}
