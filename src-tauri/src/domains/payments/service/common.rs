//! `domains::payments::service` common pieces (09-payments.md §3.1): `payment_dto`/`payment_dtos`,
//! `party_name`, `to_row`, `open_documents`, `apply_allocation`, `validate_allocations`,
//! `highlight_link`.

use std::collections::HashMap;

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::parties::parties::{Column as PartyColumn, Entity as PartyEntity};
use crate::entities::payments::payment_allocations;
use crate::entities::payments::payments;
use crate::entities::purchases::purchase_orders::{ActiveModel as PoActiveModel, Column as PoColumn, Entity as PoEntity};
use crate::entities::sales::invoices::{ActiveModel as InvoiceActiveModel, Column as InvoiceColumn, Entity as InvoiceEntity};
use crate::shared::balances::{self, OpenDocumentKind};
use crate::shared::totals::payment_status_for;
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::route::RouteRef;
use sea_orm::{ActiveModelTrait, Set};

use super::super::dto::{
    allocation_status_for, AllocationInputTargetKind, AllocationTargetKind, OpenDocument, Payment, PaymentAllocation, PaymentAllocationInput, PaymentRow, PartyKind,
};

/// `payment_dto` — loads a payment's allocations (`ORDER BY position`) and builds the full `Payment`
/// DTO (09-payments.md §3.1).
pub async fn payment_dto<C: ConnectionTrait>(conn: &C, model: &payments::Model) -> TxResult<Payment> {
    let allocations = payment_allocations::Entity::find()
        .filter(payment_allocations::Column::PaymentId.eq(model.id))
        .order_by_asc(payment_allocations::Column::Position)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(to_payment_dto(model, &allocations))
}

fn to_payment_dto(model: &payments::Model, allocations: &[payment_allocations::Model]) -> Payment {
    Payment {
        id: model.id,
        number: model.number.clone(),
        date: model.date().key(),
        r#type: model.r#type.clone().into(),
        target_type: model.target_type.clone().into(),
        target_id: model.target_id,
        target_ref: model.target_ref.as_deref().and_then(|s| s.parse::<Id>().ok()),
        target_ref_number: model.target_ref_number.clone(),
        amount: model.amount,
        method: model.method.clone().into(),
        note: model.note.clone(),
        allocations: allocations.iter().map(alloc_to_dto).collect(),
        branch_id: model.branch_id,
        currency: model.currency.clone(),
        amount_fc: model.amount_fc,
        rate: model.rate,
        fx_gain_loss: model.fx_gain_loss,
    }
}

/// Converts a stored allocation row back into its `PaymentAllocation` DTO — used both when
/// assembling a `Payment`'s `allocations[]` (`payment_dto`) and by `remove_allocation` to build the
/// `apply_allocation(-1)` argument from the row it's about to delete.
pub fn alloc_to_dto(a: &payment_allocations::Model) -> PaymentAllocation {
    PaymentAllocation {
        id: a.id,
        target_kind: a.target_kind.clone().into(),
        target_id: a.target_id,
        target_number: a.target_number.clone(),
        amount: a.amount,
        date: a.date().key(),
        amount_fc: a.amount_fc,
        fx_gain_loss: a.fx_gain_loss,
    }
}

/// Batch variant of `payment_dto` for list screens (G-2 pairing).
pub async fn payment_dtos<C: ConnectionTrait>(conn: &C, models: &[payments::Model]) -> TxResult<Vec<Payment>> {
    if models.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<Id> = models.iter().map(|m| m.id).collect();
    let rows = payment_allocations::Entity::find()
        .filter(payment_allocations::Column::PaymentId.is_in(ids))
        .order_by_asc(payment_allocations::Column::Position)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let mut by_payment: HashMap<Id, Vec<payment_allocations::Model>> = HashMap::new();
    for row in rows {
        by_payment.entry(row.payment_id).or_default().push(row);
    }
    Ok(models
        .iter()
        .map(|m| to_payment_dto(m, by_payment.get(&m.id).map(|v| v.as_slice()).unwrap_or(&[])))
        .collect())
}

/// `partyName` (`payments.ts:135-137`): the live party name of the payment's own kind, else `—`.
/// Existence lookup only — every row (active or not, soft-deleted or not) can still have its name
/// resolved for display, matching the mock's plain `.find()` over the in-memory array.
pub async fn party_name<C: ConnectionTrait>(conn: &C, target_type: PartyKind, target_id: Id) -> TxResult<String> {
    let kind_str = match target_type {
        PartyKind::Customer => "customer",
        PartyKind::Supplier => "supplier",
    };
    let row = PartyEntity::find()
        .filter(PartyColumn::Id.eq(target_id))
        .filter(PartyColumn::Kind.eq(kind_str))
        .one(conn)
        .await
        .map_err(AppError::from)?;
    Ok(row.map(|p| p.name).unwrap_or_else(|| "—".to_string()))
}

/// `toRow` (`paymentService.ts:14-17`): `allocated = round2(Σ allocation.amount)`,
/// `unallocated = shared::balances::unallocated_amount`, `allocationStatus = allocation_status_for`.
pub async fn to_row<C: ConnectionTrait>(conn: &C, model: &payments::Model) -> TxResult<PaymentRow> {
    let payment = payment_dto(conn, model).await?;
    let name = party_name(conn, payment.target_type, payment.target_id).await?;
    let allocated = round2(payment.allocations.iter().fold(Decimal::ZERO, |a, x| a + x.amount));
    let unallocated = balances::unallocated_amount(conn, model).await?;
    let status = allocation_status_for(payment.amount, allocated);
    Ok(PaymentRow { payment, party_name: name, allocated, unallocated, allocation_status: status })
}

/// `open_documents` (`payments.ts:29-74`, via `shared::balances`, PG-6/D-P5 — the one copy).
pub async fn open_documents<C: ConnectionTrait>(conn: &C, target_type: PartyKind, target_id: Id) -> TxResult<Vec<OpenDocument>> {
    let docs = match target_type {
        PartyKind::Customer => balances::open_invoices_for(conn, target_id).await?,
        PartyKind::Supplier => balances::open_purchase_orders_for(conn, target_id).await?,
    };
    Ok(docs.into_iter().map(OpenDocument::from).collect())
}

/// `applyAllocationToDocument` (`payments.ts:108-128`): `settled = amount_fc ?? amount`. The target
/// row is already locked by the caller (09-payments.md §3.1).
pub async fn apply_allocation<C: ConnectionTrait>(conn: &C, alloc: &PaymentAllocation, sign: i32) -> TxResult<()> {
    let settled = alloc.amount_fc.unwrap_or(alloc.amount);
    let signed = if sign >= 0 { settled } else { -settled };
    match alloc.target_kind {
        AllocationTargetKind::Invoice => {
            let invoice = InvoiceEntity::find()
                .filter(InvoiceColumn::Id.eq(alloc.target_id))
                .one(conn)
                .await
                .map_err(AppError::from)?
                .ok_or_else(|| AppError::not_found("الفاتورة غير موجودة"))?;
            let paid = round2(invoice.paid_amount + signed);
            let status = payment_status_for(invoice.grand_total - invoice.refunded_amount, paid);
            let mut model: InvoiceActiveModel = invoice.into();
            model.paid_amount = Set(paid);
            model.payment_status = Set(map_invoice_status(status));
            model.update(conn).await.map_err(AppError::from)?;
        }
        AllocationTargetKind::PurchaseOrder => {
            let po = PoEntity::find()
                .filter(PoColumn::Id.eq(alloc.target_id))
                .one(conn)
                .await
                .map_err(AppError::from)?
                .ok_or_else(|| AppError::not_found("أمر الشراء غير موجود"))?;
            let paid = round2(po.paid_amount + signed);
            let status = payment_status_for(po.grand_total - po.returned_amount, paid);
            let mut model: PoActiveModel = po.into();
            model.paid_amount = Set(paid);
            model.payment_status = Set(map_po_status(status));
            model.update(conn).await.map_err(AppError::from)?;
        }
        AllocationTargetKind::Opening => {
            // `opening` allocations are never written by this domain's own paths (Q-P5) — nothing
            // to update on the (nonexistent, from this domain's view) target document.
        }
    }
    Ok(())
}

fn map_invoice_status(k: crate::shared::totals::PaymentStatusKind) -> crate::entities::sales::invoices::PaymentStatus {
    use crate::entities::sales::invoices::PaymentStatus as InvoicePs;
    use crate::shared::totals::PaymentStatusKind as K;
    match k {
        K::Paid => InvoicePs::Paid,
        K::PartiallyPaid => InvoicePs::PartiallyPaid,
        K::Unpaid => InvoicePs::Unpaid,
    }
}

fn map_po_status(k: crate::shared::totals::PaymentStatusKind) -> crate::entities::purchases::purchase_orders::PaymentStatus {
    use crate::entities::purchases::purchase_orders::PaymentStatus as PoPs;
    use crate::shared::totals::PaymentStatusKind as K;
    match k {
        K::Paid => PoPs::Paid,
        K::PartiallyPaid => PoPs::PartiallyPaid,
        K::Unpaid => PoPs::Unpaid,
    }
}

/// `validateAllocations` (`payments.ts:139-192`, §3.1): exact order — `open_docs =
/// open_documents(payment.target)`; `remaining = round2(payment.amount − already)`; for each input,
/// in order: skip `!(amount > 0)`; doc lookup; over-allocation check; FC-document branch (partial
/// base-vs-FC refusal, FC-outstanding check, AR-outstanding check, realized FX) or base-document
/// branch. All messages are `VALIDATION`. `payment_date`/`payment_rate`/`payment_amount`/`already`
/// are passed explicitly (not a full `payments::Model`) since `create_payment` calls this before the
/// row exists.
pub async fn validate_allocations<C: ConnectionTrait>(
    conn: &C,
    target_type: PartyKind,
    target_id: Id,
    payment_amount: Decimal,
    payment_rate: Option<Decimal>,
    payment_date_key: &str,
    already: Decimal,
    inputs: &[PaymentAllocationInput],
) -> TxResult<Vec<PaymentAllocation>> {
    let open_docs = open_documents(conn, target_type, target_id).await?;
    let tol_5 = Decimal::new(5, 3);
    let tol_1 = Decimal::new(1, 2);
    let mut remaining = round2(payment_amount - already);
    let mut rows: Vec<PaymentAllocation> = Vec::new();

    for input in inputs {
        if !(input.amount > Decimal::ZERO) {
            continue;
        }
        let input_kind: AllocationTargetKind = input.target_kind.into();
        let doc = open_docs
            .iter()
            .find(|d| d.id == input.target_id && d.kind == input_kind_to_open_kind(input.target_kind))
            .ok_or_else(|| AppError::validation("المستند غير موجود ضمن المستندات المفتوحة لهذا الطرف"))?;

        let cash_amount = round2(input.amount);
        if cash_amount > remaining + tol_5 {
            return Err(AppError::validation("إجمالي التخصيص أكبر من مبلغ السند").into());
        }

        if let (Some(currency), Some(rate), Some(fc_outstanding)) = (doc.currency.as_ref(), doc.rate, doc.fc_outstanding) {
            let fc_settled = if let Some(payment_rate) = payment_rate {
                round2(cash_amount / payment_rate)
            } else if round2(cash_amount - doc.outstanding) != Decimal::ZERO {
                return Err(AppError::validation(format!(
                    "التخصيص الجزئي بالعملة الأساسية على مستند بعملة {currency} غير مدعوم — خصص المبلغ كاملاً أو استخدم دفعة بنفس العملة"
                ))
                .into());
            } else {
                fc_outstanding
            };

            if fc_settled > fc_outstanding + tol_5 {
                return Err(AppError::validation(format!(
                    "الكمية المخصصة لـ {} أكبر من المتبقي عليه ({:.2} {currency})",
                    doc.number, fc_outstanding
                ))
                .into());
            }
            let ar_amount = round2(fc_settled * rate);
            if ar_amount > doc.outstanding + tol_1 {
                return Err(AppError::validation(format!("المبلغ المخصص لـ {} أكبر من المتبقي عليه ({:.2})", doc.number, doc.outstanding)).into());
            }
            // + = gain. A receipt gains when more base cash came in than the receivable carried; a
            // supplier payment gains when less base cash went out than the payable carried — the
            // sign flips for it (ACC-0015; the old `cash − ar` booked a PAID loss as a gain and left
            // the entry unbalanced).
            let fx_gain_loss = if matches!(target_type, PartyKind::Customer) { round2(cash_amount - ar_amount) } else { round2(ar_amount - cash_amount) };
            remaining = round2(remaining - cash_amount);
            rows.push(PaymentAllocation {
                id: Id::new(),
                target_kind: input_kind,
                target_id: input.target_id,
                target_number: doc.number.clone(),
                amount: ar_amount,
                date: payment_date_key.to_string(),
                amount_fc: Some(fc_settled),
                fx_gain_loss: if fx_gain_loss != Decimal::ZERO { Some(fx_gain_loss) } else { None },
            });
            continue;
        }

        if cash_amount > doc.outstanding + tol_5 {
            return Err(AppError::validation(format!("المبلغ المخصص لـ {} أكبر من المتبقي عليه ({:.2})", doc.number, doc.outstanding)).into());
        }
        remaining = round2(remaining - cash_amount);
        rows.push(PaymentAllocation {
            id: Id::new(),
            target_kind: input_kind,
            target_id: input.target_id,
            target_number: doc.number.clone(),
            amount: cash_amount,
            date: payment_date_key.to_string(),
            amount_fc: None,
            fx_gain_loss: None,
        });
    }
    Ok(rows)
}

fn input_kind_to_open_kind(k: AllocationInputTargetKind) -> super::super::dto::OpenDocumentKindDto {
    let kind = match k {
        AllocationInputTargetKind::Invoice => OpenDocumentKind::Invoice,
        AllocationInputTargetKind::PurchaseOrder => OpenDocumentKind::PurchaseOrder,
    };
    kind.into()
}

/// `highlight_link` (PG-5): `{ name: 'payments', query: { highlight: id } }`.
pub fn highlight_link(payment_id: Id) -> RouteRef {
    RouteRef::list("payments").with_query("highlight", payment_id.to_string())
}
