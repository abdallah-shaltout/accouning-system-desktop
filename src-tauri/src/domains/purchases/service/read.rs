//! `domains::purchases::service::read` — U-1 `get_purchase_orders`, U-2 `get_purchase_order`, U-10
//! reads (`get_purchase_return`, `get_active_batches`, `get_debit_note_drafts`).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::{TxError, TxResult};
use crate::domains::products::dto::inventory::{DebitNoteDraft, ProductBatch};
use crate::entities::journal::journal_entries::{Column as JournalColumn, Entity as JournalEntity};
use crate::entities::parties::parties::Entity as PartyEntity;
use crate::entities::payments::payment_allocations::{Column as AllocColumn, Entity as AllocEntity, PaymentAllocationTargetKind};
use crate::entities::payments::payments::{Column as PaymentColumn, Entity as PaymentEntity, PaymentType};
use crate::entities::purchases::purchase_order_lines::{Column as PoLineColumn, Entity as PoLineEntity};
use crate::entities::purchases::purchase_orders::{Column as PoColumn, Entity as PoEntity, Model as PoModel, PurchaseStatus as EntityPurchaseStatus};
use crate::entities::purchases::purchase_return_lines::{Column as RetLineColumn, Entity as RetLineEntity};
use crate::entities::purchases::purchase_returns::{Column as RetColumn, Entity as RetEntity, Model as RetModel};
use crate::utils::id::Id;
use crate::utils::text::matches_search;

use super::super::dto::{
    DebitNoteLine, InvoiceDiscount, LandedCostLine, PaymentStatus as DtoPaymentStatus, PurchaseDetail, PurchaseJournalRef, PurchaseLine, PurchaseListFilter,
    PurchaseOrder, PurchaseProductInfo, PurchaseReturn, PurchaseRow, PurchaseStatus, RefundMethod,
};
use super::{duplicate_supplier_invoice, missing_supplier_invoice, purchase_outstanding, returned_qty_by_product};

// --- shared entity <-> DTO mapping ---------------------------------------------------------------

pub(crate) fn parse_status(s: &EntityPurchaseStatus) -> PurchaseStatus {
    match s {
        EntityPurchaseStatus::Draft => PurchaseStatus::Draft,
        EntityPurchaseStatus::Ordered => PurchaseStatus::Ordered,
        EntityPurchaseStatus::Received => PurchaseStatus::Received,
        EntityPurchaseStatus::Canceled => PurchaseStatus::Canceled,
    }
}

pub(crate) fn status_as_entity(s: PurchaseStatus) -> EntityPurchaseStatus {
    match s {
        PurchaseStatus::Draft => EntityPurchaseStatus::Draft,
        PurchaseStatus::Ordered => EntityPurchaseStatus::Ordered,
        PurchaseStatus::Received => EntityPurchaseStatus::Received,
        PurchaseStatus::Canceled => EntityPurchaseStatus::Canceled,
    }
}

pub(crate) fn parse_payment_status(s: &crate::entities::purchases::purchase_orders::PaymentStatus) -> DtoPaymentStatus {
    use crate::entities::purchases::purchase_orders::PaymentStatus as EntityPaymentStatus;
    match s {
        EntityPaymentStatus::Unpaid => DtoPaymentStatus::Unpaid,
        EntityPaymentStatus::PartiallyPaid => DtoPaymentStatus::PartiallyPaid,
        EntityPaymentStatus::Paid => DtoPaymentStatus::Paid,
    }
}

pub(crate) fn payment_status_as_entity(s: DtoPaymentStatus) -> crate::entities::purchases::purchase_orders::PaymentStatus {
    use crate::entities::purchases::purchase_orders::PaymentStatus as EntityPaymentStatus;
    match s {
        DtoPaymentStatus::Unpaid => EntityPaymentStatus::Unpaid,
        DtoPaymentStatus::PartiallyPaid => EntityPaymentStatus::PartiallyPaid,
        DtoPaymentStatus::Paid => EntityPaymentStatus::Paid,
    }
}

/// `shared::totals::PaymentStatusKind` -> this domain's own `PaymentStatus` DTO (the generic 3-way
/// result `payment_status_for`/G-21 returns, mapped onto the concrete enum, same pattern as
/// `payments::service::common::map_po_status`/`map_invoice_status` map it onto their own entities).
pub(crate) fn payment_status_from_kind(k: crate::shared::totals::PaymentStatusKind) -> DtoPaymentStatus {
    use crate::shared::totals::PaymentStatusKind as K;
    match k {
        K::Paid => DtoPaymentStatus::Paid,
        K::PartiallyPaid => DtoPaymentStatus::PartiallyPaid,
        K::Unpaid => DtoPaymentStatus::Unpaid,
    }
}

pub(crate) fn parse_refund_method(s: &crate::entities::purchases::purchase_returns::RefundMethod) -> RefundMethod {
    use crate::entities::purchases::purchase_returns::RefundMethod as EntityRefundMethod;
    match s {
        EntityRefundMethod::Cash => RefundMethod::Cash,
        EntityRefundMethod::BankTransfer => RefundMethod::BankTransfer,
        EntityRefundMethod::Credit => RefundMethod::Credit,
    }
}

pub(crate) fn refund_method_as_entity(m: RefundMethod) -> crate::entities::purchases::purchase_returns::RefundMethod {
    use crate::entities::purchases::purchase_returns::RefundMethod as EntityRefundMethod;
    match m {
        RefundMethod::Cash => EntityRefundMethod::Cash,
        RefundMethod::BankTransfer => EntityRefundMethod::BankTransfer,
        RefundMethod::Credit => EntityRefundMethod::Credit,
    }
}

// `LandedCostLine` already derives `Serialize`/`Deserialize` (camelCase, matching the mock's own
// `LandedCostLine` JSON shape byte-for-byte) — round-trip through it directly rather than poking at
// a `serde_json::Value` by hand.
fn landed_costs_from_json(v: &Option<serde_json::Value>) -> Option<Vec<LandedCostLine>> {
    let v = v.as_ref()?;
    let lines: Vec<LandedCostLine> = serde_json::from_value(v.clone()).ok()?;
    if lines.is_empty() {
        None
    } else {
        Some(lines)
    }
}

fn landed_costs_to_json(lines: &[LandedCostLine]) -> Option<serde_json::Value> {
    if lines.is_empty() {
        return None;
    }
    serde_json::to_value(lines).ok()
}

pub(crate) fn json_to_landed_costs(v: &Option<serde_json::Value>) -> Option<Vec<LandedCostLine>> {
    landed_costs_from_json(v)
}

pub(crate) fn landed_costs_to_json_value(lines: &[LandedCostLine]) -> Option<serde_json::Value> {
    landed_costs_to_json(lines)
}

fn line_to_dto(l: &crate::entities::purchases::purchase_order_lines::Model) -> PurchaseLine {
    PurchaseLine {
        product_id: l.product_id,
        qty: l.qty,
        cost_price: l.cost_price,
        unit_id: l.unit_id,
        unit_factor: l.unit_factor,
        discount: l.discount,
        discount_is_pct: l.discount_is_pct,
        tax_id: l.tax_id,
        received_qty: l.received_qty,
        batch_no: l.batch_no.clone(),
        expiry_date: l.expiry_date,
        landed_cost_share: l.landed_cost_share,
    }
}

pub(crate) async fn po_lines<C: ConnectionTrait>(conn: &C, po_id: Id) -> TxResult<Vec<crate::entities::purchases::purchase_order_lines::Model>> {
    PoLineEntity::find().filter(PoLineColumn::PurchaseOrderId.eq(po_id)).order_by_asc(PoLineColumn::Position).all(conn).await.map_err(|e| AppError::from(e).into())
}

pub(crate) async fn to_order_dto<C: ConnectionTrait>(conn: &C, m: &PoModel) -> TxResult<PurchaseOrder> {
    let lines = po_lines(conn, m.id).await?;
    Ok(PurchaseOrder {
        id: m.id,
        number: m.number.clone(),
        supplier_id: m.supplier_id,
        date: m.date().key(),
        status: parse_status(&m.status),
        lines: lines.iter().map(line_to_dto).collect(),
        sub_total: m.sub_total,
        tax_rate: m.tax_rate,
        tax_amount: m.tax_amount,
        grand_total: m.grand_total,
        payment_status: parse_payment_status(&m.payment_status),
        paid_amount: m.paid_amount,
        returned_amount: m.returned_amount,
        note: m.note.clone(),
        invoice_discount: if m.invoice_discount_pct.is_some() || m.invoice_discount_amount.is_some() {
            Some(InvoiceDiscount { pct: m.invoice_discount_pct, amount: m.invoice_discount_amount })
        } else {
            None
        },
        landed_costs: landed_costs_from_json(&m.landed_costs),
        supplier_invoice_no: m.supplier_invoice_no.clone(),
        supplier_invoice_date: m.supplier_invoice_date,
        vat_not_recoverable: m.vat_not_recoverable,
        sent_at: m.sent_at.map(crate::utils::dates::format_iso_ms),
        backorder_of_id: m.backorder_of_id,
        received_date: m.received_date().map(|d| d.key()),
        attachment_ids: m.attachment_ids.as_ref().map(|l| l.0.clone()),
        cost_center_id: m.cost_center_id,
        branch_id: m.branch_id,
        currency: m.currency.clone(),
        exchange_rate: m.exchange_rate,
    })
}

pub(crate) async fn to_row<C: ConnectionTrait>(conn: &C, m: &PoModel) -> TxResult<PurchaseRow> {
    let order = to_order_dto(conn, m).await?;
    let supplier_name = PartyEntity::find_by_id(m.supplier_id).one(conn).await.map_err(AppError::from)?.map(|p| p.name).unwrap_or_else(|| "—".to_string());
    let outstanding = if matches!(order.status, PurchaseStatus::Received) { purchase_outstanding(&order) } else { Decimal::ZERO };
    let missing = missing_supplier_invoice(&order);
    Ok(PurchaseRow::from_order(order, supplier_name, outstanding, missing))
}

fn return_line_to_dto(l: &crate::entities::purchases::purchase_return_lines::Model) -> DebitNoteLine {
    DebitNoteLine { product_id: l.product_id, qty: l.qty, cost_price: l.cost_price, batch_id: l.batch_id }
}

pub(crate) async fn to_return_dto<C: ConnectionTrait>(conn: &C, m: &RetModel) -> TxResult<PurchaseReturn> {
    let lines = RetLineEntity::find().filter(RetLineColumn::PurchaseReturnId.eq(m.id)).order_by_asc(RetLineColumn::Position).all(conn).await.map_err(AppError::from)?;
    Ok(PurchaseReturn {
        id: m.id,
        number: m.number.clone(),
        purchase_order_id: m.purchase_order_id,
        supplier_id: m.supplier_id,
        date: m.date().key(),
        reason: m.reason.clone(),
        lines: lines.iter().map(return_line_to_dto).collect(),
        sub_total: m.sub_total,
        tax_amount: m.tax_amount,
        grand_total: m.grand_total,
        settled_to_payable: m.settled_to_payable,
        cash_back: m.cash_back,
        refund_method: parse_refund_method(&m.refund_method),
        from_draft_id: m.from_draft_id,
    })
}

// --- U-1 get_purchase_orders --------------------------------------------------------------------

pub async fn get_purchase_orders<C: ConnectionTrait>(conn: &C, filter: Option<PurchaseListFilter>) -> TxResult<Vec<PurchaseRow>> {
    let mut query = PoEntity::find();
    if let Some(f) = &filter {
        if let Some(status) = f.status {
            query = query.filter(PoColumn::Status.eq(status_as_entity(status)));
        }
        if let Some(ps) = f.payment_status {
            query = query.filter(PoColumn::PaymentStatus.eq(payment_status_as_entity(ps)));
        }
        if let Some(supplier_id) = f.supplier_id {
            query = query.filter(PoColumn::SupplierId.eq(supplier_id));
        }
    }
    let rows = query.all(conn).await.map_err(AppError::from)?;

    let from = filter.as_ref().and_then(|f| f.from.as_deref());
    let to = filter.as_ref().and_then(|f| f.to.as_deref());
    let mut rows: Vec<PoModel> = rows.into_iter().filter(|r| crate::utils::dates::in_date_range(r.date_day, from, to)).collect();
    // Stable sort by date key descending (ties keep created_at, id — DB PK order is arbitrary, so
    // sort by (created_at, id) first to make the "ties" stable, then by date key descending).
    rows.sort_by(|a, b| (a.created_at, a.id).cmp(&(b.created_at, b.id)));

    let search = filter.as_ref().and_then(|f| f.search.as_deref());
    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        let dto = to_row(conn, row).await?;
        if matches_search(&[Some(dto.number.as_str()), Some(dto.supplier_name.as_str()), dto.note.as_deref()], search) {
            out.push(dto);
        }
    }
    out.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(out)
}

// --- U-2 get_purchase_order ----------------------------------------------------------------------

pub async fn get_purchase_order<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<PurchaseDetail> {
    let po = PoEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("أمر الشراء غير موجود"))?;
    let row = to_row(conn, &po).await?;

    let mut returns_rows = RetEntity::find().filter(RetColumn::PurchaseOrderId.eq(id)).all(conn).await.map_err(AppError::from)?;
    returns_rows.sort_by(|a, b| (a.created_at, a.id).cmp(&(b.created_at, b.id)));
    let mut returns = Vec::with_capacity(returns_rows.len());
    for r in &returns_rows {
        returns.push(to_return_dto(conn, r).await?);
    }

    let mut payment_rows = PaymentEntity::find().filter(PaymentColumn::Type.eq(PaymentType::Paid)).all(conn).await.map_err(AppError::from)?;
    let mut matching_payment_ids: Vec<Id> = Vec::new();
    for p in &payment_rows {
        let has_alloc = AllocEntity::find()
            .filter(AllocColumn::PaymentId.eq(p.id))
            .filter(AllocColumn::TargetKind.eq(PaymentAllocationTargetKind::PurchaseOrder))
            .filter(AllocColumn::TargetId.eq(id))
            .one(conn)
            .await
            .map_err(AppError::from)?
            .is_some();
        if has_alloc {
            matching_payment_ids.push(p.id);
        }
    }
    payment_rows.retain(|p| matching_payment_ids.contains(&p.id));
    payment_rows.sort_by(|a, b| (a.created_at, a.id).cmp(&(b.created_at, b.id)));
    // G-30: the payments domain's own DTO-builder, reused cross-domain (the same pattern
    // `payments::service::read::payments_for_invoice` establishes for 08-invoices — a sibling
    // domain's `pub` service function is a public surface, not the mock-backend seam).
    let payments = crate::domains::payments::service::common::payment_dtos(conn, &payment_rows).await?;

    let mut source_ids: Vec<Id> = vec![id];
    source_ids.extend(returns_rows.iter().map(|r| r.id));
    source_ids.extend(payment_rows.iter().map(|p| p.id));

    let mut journal_rows = JournalEntity::find().filter(JournalColumn::SourceId.is_in(source_ids)).all(conn).await.map_err(AppError::from)?;
    journal_rows.sort_by(|a, b| (a.created_at, a.id).cmp(&(b.created_at, b.id)));
    let journal_entries: Vec<PurchaseJournalRef> = journal_rows.iter().map(|e| PurchaseJournalRef { id: e.id, number: e.number.clone(), description: e.description.clone() }).collect();

    let mut products: BTreeMap<String, PurchaseProductInfo> = BTreeMap::new();
    for line in &row.lines {
        if let Some(p) = crate::entities::catalog::products::Entity::find_by_id(line.product_id).one(conn).await.map_err(AppError::from)? {
            products.insert(
                p.id.to_string(),
                PurchaseProductInfo {
                    name: p.name.clone(),
                    sku: p.sku.clone(),
                    stock_qty: p.stock_qty,
                    r#type: if p.r#type == "service" {
                        crate::domains::products::dto::catalog::ProductType::Service
                    } else {
                        crate::domains::products::dto::catalog::ProductType::Product
                    },
                    track_batches: p.track_batches,
                },
            );
        }
    }

    // `parties::service::read::get_supplier` is the domain's own public reader (returns the full
    // `Supplier` DTO with phones/balance/credit already assembled) — its `NOT_FOUND` becomes
    // `None` here (the PO's `supplier` field is optional; a supplier can be soft-deleted after the
    // PO was created, same as `purchaseService.ts:86`'s plain `db.suppliers.find(...)`).
    let supplier_exists = PartyEntity::find_by_id(po.supplier_id).one(conn).await.map_err(AppError::from)?.is_some();
    let supplier = if supplier_exists {
        match crate::domains::parties::service::read::get_supplier(conn, po.supplier_id).await {
            Ok(s) => Some(s),
            Err(TxError::App(AppError::NotFound { .. })) => None,
            Err(e) => return Err(e),
        }
    } else {
        None
    };

    let dup = match (&po.supplier_invoice_no, supplier_exists) {
        (Some(no), true) if !no.trim().is_empty() => duplicate_supplier_invoice(conn, po.supplier_id, no, Some(po.id)).await?,
        _ => None,
    };
    let duplicate_invoice_warning = dup.map(|d| format!("رقم الفاتورة مستخدم من قبل في أمر الشراء {} من نفس المورد", d.number));

    let returned_qty = returned_qty_by_product(conn, id).await?.into_iter().map(|(k, v)| (k.to_string(), v)).collect();

    Ok(PurchaseDetail::from_row(row, supplier, products, returns, payments, journal_entries, returned_qty, duplicate_invoice_warning))
}

// --- U-10 reads -----------------------------------------------------------------------------------

pub async fn get_purchase_return<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<PurchaseReturn> {
    let ret = RetEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("إشعار المدين غير موجود"))?;
    to_return_dto(conn, &ret).await
}

pub async fn get_active_batches<C: ConnectionTrait>(conn: &C, product_id: Id) -> TxResult<Vec<ProductBatch>> {
    crate::domains::products::service::batches::get_batches(conn, product_id).await
}

pub async fn get_debit_note_drafts<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<DebitNoteDraft>> {
    crate::domains::products::service::batches::get_debit_note_drafts(conn).await
}
