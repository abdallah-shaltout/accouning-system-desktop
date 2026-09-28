//! `domains::payments::service::allocate` — `allocate_existing_payment` (§3.3) and
//! `remove_allocation` (§3.4).

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::payments::payment_allocations::{Column as AllocColumn, Entity as AllocEntity};
use crate::entities::payments::payments::{ActiveModel as PaymentActiveModel, Entity as PaymentEntity};
use crate::entities::platform::activity::ActivityKind;
use crate::shared::activity;
use crate::shared::ledger::accounts::SystemRole;
use crate::shared::ledger::post::{self, AccountRef, PartyRef, PostJournal, PostingLine, SourceRef};
use crate::utils::dates::DocDate;
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::super::dto::{Payment, PaymentAllocationInput};
use super::common::{apply_allocation, highlight_link, party_name, payment_dto, validate_allocations};

async fn lock_payment<C: ConnectionTrait>(conn: &C, payment_id: Id) -> TxResult<crate::entities::payments::payments::Model> {
    lock::for_update_by_id(conn, "payments", &payment_id.to_string()).await.map_err(TxError::from)?;
    PaymentEntity::find_by_id(payment_id).one(conn).await.map_err(TxError::from)?.ok_or_else(|| TxError::App(AppError::not_found("السند غير موجود")))
}

async fn lock_target_documents<C: ConnectionTrait>(conn: &C, inputs: &[PaymentAllocationInput]) -> TxResult<()> {
    let positive: Vec<&PaymentAllocationInput> = inputs.iter().filter(|a| a.amount > Decimal::ZERO).collect();
    let invoice_ids: Vec<String> = positive
        .iter()
        .filter(|a| matches!(a.target_kind, super::super::dto::AllocationInputTargetKind::Invoice))
        .map(|a| a.target_id.to_string())
        .collect();
    let po_ids: Vec<String> = positive
        .iter()
        .filter(|a| matches!(a.target_kind, super::super::dto::AllocationInputTargetKind::PurchaseOrder))
        .map(|a| a.target_id.to_string())
        .collect();
    lock::for_update_many_sorted(conn, "invoices", &invoice_ids).await.map_err(TxError::from)?;
    lock::for_update_many_sorted(conn, "purchase_orders", &po_ids).await.map_err(TxError::from)?;
    Ok(())
}

/// **§3.3 `allocate_existing_payment(conn, cx, reg, payment_id, inputs)`**, porting `allocatePayment`
/// (`payments.ts:314-369`) in this exact order.
pub async fn allocate_existing_payment<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::undo::UndoRegistry,
    payment_id: Id,
    inputs: Vec<PaymentAllocationInput>,
) -> TxResult<Payment> {
    // 1. Lock the payment.
    let payment = lock_payment(conn, payment_id).await?;

    // 2. Lock the input target documents, then the party (R-1).
    lock_target_documents(conn, &inputs).await?;
    lock::share_lock_by_id(conn, "parties", &payment.target_id.to_string()).await.map_err(TxError::from)?;

    // 3. Validate.
    let dto_target_type = payment.target_type.clone().into();
    let already = round2(
        AllocEntity::find()
            .filter(AllocColumn::PaymentId.eq(payment_id))
            .all(conn)
            .await
            .map_err(TxError::from)?
            .iter()
            .fold(Decimal::ZERO, |a, x| a + x.amount),
    );
    let existing_count = AllocEntity::find().filter(AllocColumn::PaymentId.eq(payment_id)).count(conn).await.map_err(TxError::from)?;
    let new_rows = validate_allocations(conn, dto_target_type, payment.target_id, payment.amount, payment.rate, &payment.date().key(), already, &inputs).await?;
    if new_rows.is_empty() {
        return Err(TxError::App(AppError::validation("لم يتم إدخال أي تخصيص")));
    }

    // 4. Insert rows at `position = existing_count…`; `apply_allocation(+1)`; single-allocation
    //    `target_ref` update.
    super::create::insert_allocations_at(conn, payment_id, &new_rows, payment.date(), existing_count as i16).await?;
    for alloc in &new_rows {
        apply_allocation(conn, alloc, 1).await?;
    }

    let total_after = existing_count as i64 + new_rows.len() as i64;
    let mut payment_model: PaymentActiveModel = payment.clone().into();
    let mut target_ref_changed = false;
    if total_after == 1 {
        payment_model.target_ref = Set(Some(new_rows[0].target_id.to_string()));
        payment_model.target_ref_number = Set(Some(new_rows[0].target_number.clone()));
        target_ref_changed = true;
    }

    // 5. Realized FX on "allocate later".
    let new_fx = round2(new_rows.iter().fold(Decimal::ZERO, |a, r| a + r.fx_gain_loss.unwrap_or(Decimal::ZERO)));
    let now_date = DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) };
    if new_fx != Decimal::ZERO {
        let updated_fx = round2(payment.fx_gain_loss.unwrap_or(Decimal::ZERO) + new_fx);
        payment_model.fx_gain_loss = Set(Some(updated_fx));

        let new_fc = {
            let sum = round2(new_rows.iter().fold(Decimal::ZERO, |a, r| a + r.amount_fc.unwrap_or(Decimal::ZERO)));
            if sum != Decimal::ZERO {
                Some(sum)
            } else {
                None
            }
        };
        let fc_tag: (Option<String>, Option<Decimal>, Option<Decimal>) = match (new_fc, &payment.currency) {
            (Some(fc), Some(cur)) => (Some(cur.clone()), Some(fc), payment.rate),
            _ => (None, None, None),
        };
        let dim_branch = payment.branch_id;

        let control_line = match payment.r#type {
            crate::entities::payments::payments::PaymentType::Received => PostingLine {
                party: Some(PartyRef { kind: crate::entities::journal::journal_lines::PartyKind::Customer, id: payment.target_id }),
                branch_id: dim_branch,
                currency: fc_tag.0.clone(),
                amount_fc: fc_tag.1,
                rate: fc_tag.2,
                debit: if new_fx > Decimal::ZERO { new_fx } else { Decimal::ZERO },
                credit: if new_fx < Decimal::ZERO { -new_fx } else { Decimal::ZERO },
                ..PostingLine::debit(AccountRef::Role(SystemRole::Receivable), Decimal::ZERO)
            },
            crate::entities::payments::payments::PaymentType::Paid => PostingLine {
                party: Some(PartyRef { kind: crate::entities::journal::journal_lines::PartyKind::Supplier, id: payment.target_id }),
                branch_id: dim_branch,
                currency: fc_tag.0.clone(),
                amount_fc: fc_tag.1,
                rate: fc_tag.2,
                debit: if new_fx < Decimal::ZERO { -new_fx } else { Decimal::ZERO },
                credit: if new_fx > Decimal::ZERO { new_fx } else { Decimal::ZERO },
                ..PostingLine::debit(AccountRef::Role(SystemRole::Payable), Decimal::ZERO)
            },
        };
        let fx_line = if new_fx > Decimal::ZERO {
            PostingLine { branch_id: dim_branch, description: Some("فرق عملة محقق (تخصيص لاحق)".to_string()), ..PostingLine::credit(AccountRef::Role(SystemRole::FxGain), new_fx) }
        } else {
            PostingLine { branch_id: dim_branch, description: Some("فرق عملة محقق (تخصيص لاحق)".to_string()), ..PostingLine::debit(AccountRef::Role(SystemRole::FxLoss), -new_fx) }
        };

        post::post(
            conn,
            cx,
            PostJournal {
                date: now_date,
                description: format!("فرق عملة محقق — تخصيص لاحق على سند {}", payment.number),
                entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
                source: Some(SourceRef { kind: "payment".to_string(), id: payment_id, number: Some(payment.number.clone()) }),
                lines: vec![control_line, fx_line],
                allow_closed_period: false,
                attachment_ids: Vec::new(),
                template_id: None,
            },
        )
        .await?;
    }

    let updated = if target_ref_changed || new_fx != Decimal::ZERO {
        payment_model.update(conn).await.map_err(TxError::from)?
    } else {
        payment
    };

    // 6. Activity.
    let name = party_name(conn, dto_target_type, updated.target_id).await?;
    let numbers: Vec<String> = new_rows.iter().map(|r| r.target_number.clone()).collect();
    let total_new = round2(new_rows.iter().fold(Decimal::ZERO, |a, r| a + r.amount));
    activity::record::log(
        conn,
        cx,
        registry,
        ActivityKind::Payment,
        format!("تخصيص {total_new:.2} من سند {} ({name}) على {}", updated.number, numbers.join("، ")),
        Some(now_date),
        Some(highlight_link(payment_id)),
    )
    .await?;
    cx.touch(crate::core::events::ChangeCategory::Parties);

    payment_dto(conn, &updated).await
}

/// **§3.4 `remove_allocation(conn, cx, reg, payment_id, allocation_id)`**, porting
/// `unallocatePayment` (`payments.ts:372-388`).
pub async fn remove_allocation<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::undo::UndoRegistry,
    payment_id: Id,
    allocation_id: Id,
) -> TxResult<Payment> {
    // 1. Lock the payment.
    let payment = lock_payment(conn, payment_id).await?;

    // 2. The allocation.
    let alloc = AllocEntity::find()
        .filter(AllocColumn::Id.eq(allocation_id))
        .filter(AllocColumn::PaymentId.eq(payment_id))
        .one(conn)
        .await
        .map_err(TxError::from)?
        .ok_or_else(|| TxError::App(AppError::not_found("التخصيص غير موجود")))?;

    // 3. Q-P1 (spec §7, CLAUDE.md architectural-autonomy: pick the strictest/recommended option and
    //    apply it, don't leave it half-built): an allocation that realized a non-zero `fxGainLoss`
    //    (from `allocate_existing_payment`'s conditional FX-adjustment posting) can never be removed
    //    through this path — its removal would leave that FX-adjustment journal entry orphaned with
    //    no inverse, silently misstating the realized-FX total. Conservative refusal, same stance as
    //    the shift close / card settlements: the accountant posts a manual correcting entry instead.
    if alloc.fx_gain_loss.map(|fx| fx != Decimal::ZERO).unwrap_or(false) {
        return Err(TxError::App(AppError::validation(
            "لا يمكن إلغاء تخصيص حقق فرق عملة — قم بعكسه عبر قيد تسوية يدوي بدلاً من ذلك",
        )));
    }

    // 4. Lock its target document (invoice/PO; `opening` has none).
    match alloc.target_kind {
        crate::entities::payments::payment_allocations::PaymentAllocationTargetKind::Invoice => {
            lock::for_update_by_id(conn, "invoices", &alloc.target_id.to_string()).await.map_err(TxError::from)?;
        }
        crate::entities::payments::payment_allocations::PaymentAllocationTargetKind::PurchaseOrder => {
            lock::for_update_by_id(conn, "purchase_orders", &alloc.target_id.to_string()).await.map_err(TxError::from)?;
        }
        crate::entities::payments::payment_allocations::PaymentAllocationTargetKind::Opening => {}
    }

    // 5. `apply_allocation(-1)`; hard-delete the allocation row.
    let alloc_dto = super::common::alloc_to_dto(&alloc);
    apply_allocation(conn, &alloc_dto, -1).await?;
    let removed_target_id = alloc.target_id;
    AllocEntity::delete_by_id(alloc.id).exec(conn).await.map_err(TxError::from)?;

    // 6. `target_ref`/`target_ref_number` follow the first remaining allocation, when the removed
    //    one was the referenced one.
    let mut payment_model: PaymentActiveModel = payment.clone().into();
    let target_ref_matches = payment.target_ref.as_deref() == Some(removed_target_id.to_string().as_str());
    let updated = if target_ref_matches {
        let remaining = AllocEntity::find()
            .filter(AllocColumn::PaymentId.eq(payment_id))
            .order_by_asc(AllocColumn::Position)
            .all(conn)
            .await
            .map_err(TxError::from)?;
        match remaining.first() {
            Some(first) => {
                payment_model.target_ref = Set(Some(first.target_id.to_string()));
                payment_model.target_ref_number = Set(Some(first.target_number.clone()));
            }
            None => {
                payment_model.target_ref = Set(None);
                payment_model.target_ref_number = Set(None);
            }
        }
        payment_model.update(conn).await.map_err(TxError::from)?
    } else {
        payment
    };

    // 7. No GL entry — removing a non-FX allocation never touches the ledger. Activity.
    let now_date = DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) };
    activity::record::log(
        conn,
        cx,
        registry,
        ActivityKind::Payment,
        format!("إلغاء تخصيص {:.2} من سند {} عن {}", alloc.amount, updated.number, alloc.target_number),
        Some(now_date),
        Some(highlight_link(payment_id)),
    )
    .await?;
    cx.touch(crate::core::events::ChangeCategory::Parties);

    payment_dto(conn, &updated).await
}
