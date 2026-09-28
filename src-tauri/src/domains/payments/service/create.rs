//! `domains::payments::service::create` — `create_payment` (09-payments.md §3.2), porting
//! `recordPayment` (`payments.ts:195-307`).

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::parties::parties::{Column as PartyColumn, Entity as PartyEntity};
use crate::entities::payments::payments::{ActiveModel, PaymentTargetType, PaymentType, SyncStatus};
use crate::entities::platform::activity::ActivityKind;
use crate::shared::activity;
use crate::shared::ledger::accounts::{self, AccountCtx, SystemRole};
use crate::shared::ledger::post::{self, AccountRef, PartyRef, PostJournal, PostingLine, SourceRef};
use crate::shared::numbering::{self, DocumentKind};
use crate::utils::dates::RawDocDate;
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::super::dto::{Payment, PaymentAllocation, PaymentInput, PartyKind};
use super::common::{apply_allocation, highlight_link, payment_dto, validate_allocations};

/// **§3.2 `create_payment(conn, cx, reg, input)`**, porting `recordPayment` (`payments.ts:195-307`)
/// in this exact order.
pub async fn create_payment<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::undo::UndoRegistry,
    input: PaymentInput,
) -> TxResult<Payment> {
    // 1. Amount.
    let amount = round2(input.amount);
    if !(amount > Decimal::ZERO) {
        return Err(TxError::App(AppError::validation("المبلغ يجب أن يكون أكبر من صفر")));
    }

    // 2. Branch/currency.
    let settings = crate::core::settings::load(conn).await?;
    let branch_id = input.branch_id.unwrap_or(settings.default_branch_id);
    let currency = input.currency.clone().filter(|c| c != &settings.currency);

    // 3. Party BY `input.type` (the mock ignores `input.targetType`, payments.ts:203-211).
    let target_type = match input.r#type.into() {
        PaymentType::Received => PaymentTargetType::Customer,
        PaymentType::Paid => PaymentTargetType::Supplier,
    };
    let party_kind_str = match target_type {
        PaymentTargetType::Customer => "customer",
        PaymentTargetType::Supplier => "supplier",
    };
    let party = PartyEntity::find()
        .filter(PartyColumn::Id.eq(input.target_id))
        .filter(PartyColumn::Kind.eq(party_kind_str))
        .one(conn)
        .await
        .map_err(TxError::from)?;
    let party = match party {
        Some(p) => p,
        None => {
            let message = if matches!(target_type, PaymentTargetType::Customer) { "اختر العميل" } else { "اختر المورد" };
            return Err(TxError::App(AppError::validation(message)));
        }
    };
    let party_name_str = party.name.clone();

    let dto_target_type: PartyKind = target_type.clone().into();

    // 4. Locks (core/lock.rs order): target documents (invoices, then POs, sorted) -> party.
    let allocation_inputs = input.allocations.clone().unwrap_or_default();
    let positive_inputs: Vec<_> = allocation_inputs.iter().filter(|a| a.amount > Decimal::ZERO).collect();
    let invoice_ids: Vec<String> = positive_inputs
        .iter()
        .filter(|a| matches!(a.target_kind, super::super::dto::AllocationInputTargetKind::Invoice))
        .map(|a| a.target_id.to_string())
        .collect();
    let po_ids: Vec<String> = positive_inputs
        .iter()
        .filter(|a| matches!(a.target_kind, super::super::dto::AllocationInputTargetKind::PurchaseOrder))
        .map(|a| a.target_id.to_string())
        .collect();
    lock::for_update_many_sorted(conn, "invoices", &invoice_ids).await.map_err(TxError::from)?;
    lock::for_update_many_sorted(conn, "purchase_orders", &po_ids).await.map_err(TxError::from)?;
    lock::share_lock_by_id(conn, "parties", &party.id.to_string()).await.map_err(TxError::from)?;

    // 5. Validate allocations (after the locks, so `outstanding` is current).
    let payment_rate = if currency.is_some() { input.rate } else { None };
    let date = RawDocDate::parse(&input.date).map_err(|_| AppError::validation("التاريخ غير صالح"))?.resolve(&cx.clock);
    let allocations = validate_allocations(conn, dto_target_type, input.target_id, amount, payment_rate, &date.key(), Decimal::ZERO, &allocation_inputs).await?;

    // 6. `target_ref`/`target_ref_number` when exactly one allocation.
    let (target_ref, target_ref_number) = if allocations.len() == 1 {
        (Some(allocations[0].target_id), Some(allocations[0].target_number.clone()))
    } else {
        (None, None)
    };

    // 7. Posting math (payments.ts:241-253).
    let allocated_control_amount = round2(allocations.iter().fold(Decimal::ZERO, |a, x| a + x.amount));
    let cash_consumed = round2(allocations.iter().fold(Decimal::ZERO, |a, x| {
        a + match (x.amount_fc, payment_rate) {
            (Some(fc), Some(rate)) => round2(fc * rate),
            _ => x.amount,
        }
    }));
    let unallocated = round2(amount - cash_consumed);
    let control_amount = round2(allocated_control_amount + unallocated);
    let fx_gain_loss_total = round2(allocations.iter().fold(Decimal::ZERO, |a, x| a + x.fx_gain_loss.unwrap_or(Decimal::ZERO)));
    let stored_fx_gain_loss = if fx_gain_loss_total != Decimal::ZERO { Some(fx_gain_loss_total) } else { None };
    let control_amount_fc = {
        let sum = round2(allocations.iter().fold(Decimal::ZERO, |a, x| a + x.amount_fc.unwrap_or(Decimal::ZERO)));
        if sum != Decimal::ZERO {
            Some(sum)
        } else {
            None
        }
    };

    // 8. FX line.
    let dim_branch = Some(branch_id);
    let fx_line: Option<PostingLine> = if fx_gain_loss_total > Decimal::ZERO {
        Some(PostingLine { branch_id: dim_branch, description: Some("فرق عملة محقق".to_string()), ..PostingLine::credit(AccountRef::Role(SystemRole::FxGain), fx_gain_loss_total) })
    } else if fx_gain_loss_total < Decimal::ZERO {
        Some(PostingLine { branch_id: dim_branch, description: Some("فرق عملة محقق".to_string()), ..PostingLine::debit(AccountRef::Role(SystemRole::FxLoss), -fx_gain_loss_total) })
    } else {
        None
    };

    // 9. Settlement account.
    let method_str = crate::domains::payments::dto::PaymentTenderKind::from(input.method).as_str();
    let account_ctx = AccountCtx { branch_id: Some(branch_id), currency: currency.clone() };
    let settlement_account = accounts::settlement_account_for(conn, method_str, &account_ctx).await?;
    let control_fc_tag: (Option<String>, Option<Decimal>, Option<Decimal>) = match (control_amount_fc, &currency) {
        (Some(fc), Some(cur)) => (Some(cur.clone()), Some(fc), Some(round2(allocated_control_amount / fc))),
        _ => (None, None, None),
    };

    // 10. Lines.
    let mut lines: Vec<PostingLine> = Vec::new();
    match target_type {
        PaymentTargetType::Customer => {
            lines.push(PostingLine {
                branch_id: dim_branch,
                currency: currency.clone(),
                amount_fc: input.amount_fc,
                rate: input.rate,
                ..PostingLine::debit(AccountRef::Id(settlement_account.id), amount)
            });
            lines.push(PostingLine {
                party: Some(PartyRef { kind: crate::entities::journal::journal_lines::PartyKind::Customer, id: input.target_id }),
                branch_id: dim_branch,
                currency: control_fc_tag.0.clone(),
                amount_fc: control_fc_tag.1,
                rate: control_fc_tag.2,
                ..PostingLine::credit(AccountRef::Role(SystemRole::Receivable), control_amount)
            });
        }
        PaymentTargetType::Supplier => {
            lines.push(PostingLine {
                party: Some(PartyRef { kind: crate::entities::journal::journal_lines::PartyKind::Supplier, id: input.target_id }),
                branch_id: dim_branch,
                currency: control_fc_tag.0.clone(),
                amount_fc: control_fc_tag.1,
                rate: control_fc_tag.2,
                ..PostingLine::debit(AccountRef::Role(SystemRole::Payable), control_amount)
            });
            lines.push(PostingLine {
                branch_id: dim_branch,
                currency: currency.clone(),
                amount_fc: input.amount_fc,
                rate: input.rate,
                ..PostingLine::credit(AccountRef::Id(settlement_account.id), amount)
            });
        }
    }
    if let Some(fx) = fx_line {
        lines.push(fx);
    }

    // 11. Period + number.
    let number = numbering::next_number(conn, DocumentKind::Payment).await?;

    // 12. Insert `payments` + allocation rows; `apply_allocation(+1)`.
    let id = Id::new();
    let model = ActiveModel {
        id: Set(id),
        number: Set(number.clone()),
        date_day: Set(date.day),
        date_instant: Set(date.instant),
        r#type: Set(input.r#type.into()),
        target_type: Set(target_type.clone()),
        target_id: Set(input.target_id),
        target_ref: Set(target_ref.map(|id| id.to_string())),
        target_ref_number: Set(target_ref_number.clone()),
        amount: Set(amount),
        method: Set(input.method.into()),
        note: Set(input.note.clone()),
        branch_id: Set(Some(branch_id)),
        currency: Set(currency.clone()),
        amount_fc: Set(if currency.is_some() { input.amount_fc } else { None }),
        rate: Set(if currency.is_some() { input.rate } else { None }),
        fx_gain_loss: Set(stored_fx_gain_loss),
        created_at: Set(cx.clock.now),
        updated_at: Set(cx.clock.now),
        deleted_at: Set(None),
        sync_status: Set(SyncStatus::Local),
    };
    let inserted = model.insert(conn).await.map_err(TxError::from)?;

    insert_allocations(conn, id, &allocations, date).await?;
    for alloc in &allocations {
        apply_allocation(conn, alloc, 1).await?;
    }

    // 13. Post.
    let description = match target_type {
        PaymentTargetType::Customer => format!("سند قبض {number} من {party_name_str}{}", target_ref_number.as_ref().map(|n| format!(" — {n}")).unwrap_or_default()),
        PaymentTargetType::Supplier => format!("سند صرف {number} إلى {party_name_str}{}", target_ref_number.as_ref().map(|n| format!(" — {n}")).unwrap_or_default()),
    };
    post::post(
        conn,
        cx,
        PostJournal {
            date,
            description,
            entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
            source: Some(SourceRef { kind: "payment".to_string(), id, number: Some(number.clone()) }),
            lines,
            allow_closed_period: false,
            attachment_ids: Vec::new(),
            template_id: None,
        },
    )
    .await?;

    // 14. Activity.
    let verb = match target_type {
        PaymentTargetType::Customer => "تحصيل",
        PaymentTargetType::Supplier => "سداد",
    };
    let prep = match target_type {
        PaymentTargetType::Customer => "من",
        PaymentTargetType::Supplier => "إلى",
    };
    activity::record::log(conn, cx, registry, ActivityKind::Payment, format!("{verb} {amount:.2} {prep} {party_name_str}"), Some(date), Some(highlight_link(id)))
        .await?;
    cx.touch(crate::core::events::ChangeCategory::Parties);

    payment_dto(conn, &inserted).await
}

/// Inserts allocation rows at `position = 0, 1, …` — every row shares the SAME `date` (the
/// payment's own `DocDate`, passed through directly rather than re-parsed from `alloc.date`'s
/// already-formatted `DocDate::key()` string, so the exact day/instant survive unchanged).
async fn insert_allocations<C: ConnectionTrait>(conn: &C, payment_id: Id, allocations: &[PaymentAllocation], date: crate::utils::dates::DocDate) -> TxResult<()> {
    insert_allocations_from(conn, payment_id, allocations, date, 0).await
}

/// Same as `insert_allocations`, starting at an arbitrary `position` offset (`allocate_existing_payment`
/// appends after the payment's existing rows).
pub(super) async fn insert_allocations_from<C: ConnectionTrait>(
    conn: &C,
    payment_id: Id,
    allocations: &[PaymentAllocation],
    date: crate::utils::dates::DocDate,
    start_position: i16,
) -> TxResult<()> {
    for (i, alloc) in allocations.iter().enumerate() {
        let model = crate::entities::payments::payment_allocations::ActiveModel {
            id: Set(alloc.id),
            payment_id: Set(payment_id),
            position: Set(start_position + i as i16),
            target_kind: Set(alloc.target_kind.into()),
            target_id: Set(alloc.target_id),
            target_number: Set(alloc.target_number.clone()),
            amount: Set(alloc.amount),
            date_day: Set(date.day),
            date_instant: Set(date.instant),
            amount_fc: Set(alloc.amount_fc),
            fx_gain_loss: Set(alloc.fx_gain_loss),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

pub(super) use insert_allocations_from as insert_allocations_at;
