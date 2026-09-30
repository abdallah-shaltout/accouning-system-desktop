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

/// The rate an FC allocation's target document was booked at (its AR/AP rate).
async fn document_rate<C: ConnectionTrait>(conn: &C, alloc: &super::super::dto::PaymentAllocation) -> TxResult<Option<Decimal>> {
    use super::super::dto::AllocationTargetKind;
    Ok(match alloc.target_kind {
        AllocationTargetKind::Invoice => crate::entities::sales::invoices::Entity::find_by_id(alloc.target_id).one(conn).await.map_err(TxError::from)?.and_then(|i| i.exchange_rate),
        AllocationTargetKind::PurchaseOrder => crate::entities::purchases::purchase_orders::Entity::find_by_id(alloc.target_id).one(conn).await.map_err(TxError::from)?.and_then(|p| p.exchange_rate),
        AllocationTargetKind::Opening => None,
    })
}

/// **ACC-0015** — `allocationFxLines` (`payments.ts`): the entry an "allocate later" realizes FX
/// with. The payment's own entry posted this money as unallocated — base cash against the control
/// account with no FC tag. Allocating it to an FC document now (a) releases that untagged cash from
/// the control account (Dr AR / Cr AP), (b) settles the document at ITS OWN rate, FC-tagged (Cr AR /
/// Dr AP `amount`, `amount_fc` at the document's rate) — so the party's FC balance goes down by the
/// FC settled and every tagged line converts at its own rate (`fx-conversion`) — and (c) books the
/// gap to `FxGain`/`FxLoss` (`fx_gain_loss`, + = gain for either direction). Same net GL effect as
/// the old two-line entry (control ± FX); only the tagging is right.
async fn allocation_fx_lines<C: ConnectionTrait>(
    conn: &C,
    payment: &crate::entities::payments::payments::Model,
    rows: &[&super::super::dto::PaymentAllocation],
    fx: Decimal,
) -> TxResult<Vec<PostingLine>> {
    use crate::entities::journal::journal_lines::PartyKind as LinePartyKind;
    let received = matches!(payment.r#type, crate::entities::payments::payments::PaymentType::Received);
    let (role, party_kind) = if received { (SystemRole::Receivable, LinePartyKind::Customer) } else { (SystemRole::Payable, LinePartyKind::Supplier) };
    let party = Some(PartyRef { kind: party_kind, id: payment.target_id });
    let branch_id = payment.branch_id;

    // Base cash each row consumed: RECEIVED cash = AR + gain, PAID cash = AP - gain.
    let cash = round2(rows.iter().fold(Decimal::ZERO, |a, r| {
        let f = r.fx_gain_loss.unwrap_or(Decimal::ZERO);
        a + if received { r.amount + f } else { r.amount - f }
    }));
    let release = if received { PostingLine::debit(AccountRef::Role(role), cash) } else { PostingLine::credit(AccountRef::Role(role), cash) };
    let mut lines = vec![PostingLine { party, branch_id, ..release }];

    for r in rows {
        let base = if received { PostingLine::credit(AccountRef::Role(role), r.amount) } else { PostingLine::debit(AccountRef::Role(role), r.amount) };
        let mut l = PostingLine { party, branch_id, ..base };
        if let (Some(cur), Some(rate), Some(fc)) = (&payment.currency, document_rate(conn, r).await?, r.amount_fc) {
            l.currency = Some(cur.clone());
            l.amount_fc = Some(fc);
            l.rate = Some(rate);
        }
        lines.push(l);
    }

    let description = Some("فرق عملة محقق (تخصيص لاحق)".to_string());
    lines.push(if fx > Decimal::ZERO {
        PostingLine { branch_id, description, ..PostingLine::credit(AccountRef::Role(SystemRole::FxGain), fx) }
    } else {
        PostingLine { branch_id, description, ..PostingLine::debit(AccountRef::Role(SystemRole::FxLoss), -fx) }
    });
    Ok(lines)
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

    // 5. Realized FX on "allocate later" — ACC-0015 (`allocationFxLines`, payments.ts). Only rows that
    //    realized FX post; a zero-FX allocation still posts nothing (§3.3).
    let fx_rows: Vec<&super::super::dto::PaymentAllocation> = new_rows.iter().filter(|r| r.fx_gain_loss.is_some_and(|f| f != Decimal::ZERO)).collect();
    let new_fx = round2(fx_rows.iter().fold(Decimal::ZERO, |a, r| a + r.fx_gain_loss.unwrap_or(Decimal::ZERO)));
    let now_date = DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) };
    if new_fx != Decimal::ZERO {
        let updated_fx = round2(payment.fx_gain_loss.unwrap_or(Decimal::ZERO) + new_fx);
        payment_model.fx_gain_loss = Set(Some(updated_fx));
        let lines = allocation_fx_lines(conn, &payment, &fx_rows, new_fx).await?;

        post::post(
            conn,
            cx,
            PostJournal {
                date: now_date,
                description: format!("فرق عملة محقق — تخصيص لاحق على سند {}", payment.number),
                entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
                source: Some(SourceRef { kind: "payment".to_string(), id: payment_id, number: Some(payment.number.clone()) }),
                lines,
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
