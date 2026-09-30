//! `domains::purchases::service::orders` — U-3 `validate_lines`, `insert_or_update_po`, U-4
//! `save_purchase_order`, U-5 `send_purchase_order_to_supplier`, U-8 `cancel_purchase_order`.

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::catalog::products::Entity as ProductEntity;
use crate::entities::platform::activity::ActivityKind;
use crate::entities::purchases::purchase_order_lines::{ActiveModel as PoLineActiveModel, Column as PoLineColumn, Entity as PoLineEntity};
use crate::entities::purchases::purchase_orders::{ActiveModel as PoActiveModel, Entity as PoEntity, Model as PoModel};
use crate::shared::activity;
use crate::shared::defaults::{branch_prefix, default_cost_center_for};
use crate::shared::numbering::{self, DocumentKind};
use crate::utils::dates::RawDocDate;
use crate::utils::id::Id;
use crate::utils::route::RouteRef;

use super::super::dto::{PurchaseLineInput, PurchaseOrder, PurchaseOrderInput, ReceiveLineInput, ReceivePurchaseInput};
use super::read::{landed_costs_to_json_value, po_lines, to_order_dto};
use super::{base_qty, compute_purchase_totals, PurchaseTotalsLine};

// --- U-3 validate_lines ---------------------------------------------------------------------------

/// **U-3 `validate_lines(input)`** (`purchases.ts:92-102`), exact order and codes (analysis §3
/// note): supplier missing -> `VALIDATION` `اختر المورد`; inactive -> `المورد غير نشط`; no lines ->
/// `أضف صنفاً واحداً على الأقل`; per line: product missing -> `NOT_FOUND` `المنتج غير موجود`;
/// `qty <= 0` -> `الكمية يجب أن تكون أكبر من صفر`; `cost_price < 0` -> `سعر التكلفة لا يمكن أن يكون
/// سالباً`.
pub(crate) async fn validate_lines<C: ConnectionTrait>(conn: &C, supplier_id: Id, lines: &[PurchaseLineInput]) -> TxResult<()> {
    let sup = crate::entities::parties::parties::Entity::find_by_id(supplier_id)
        .filter(crate::entities::parties::parties::Column::Kind.eq("supplier"))
        .one(conn)
        .await
        .map_err(AppError::from)?;
    let Some(sup) = sup else {
        return Err(AppError::validation("اختر المورد").into());
    };
    if !sup.active {
        return Err(AppError::validation("المورد غير نشط").into());
    }
    if lines.is_empty() {
        return Err(AppError::validation("أضف صنفاً واحداً على الأقل").into());
    }
    for line in lines {
        let exists = ProductEntity::find_by_id(line.product_id).one(conn).await.map_err(AppError::from)?.is_some();
        if !exists {
            return Err(AppError::not_found("المنتج غير موجود").into());
        }
        if !(line.qty > Decimal::ZERO) {
            return Err(AppError::validation("الكمية يجب أن تكون أكبر من صفر").into());
        }
        if line.cost_price < Decimal::ZERO {
            return Err(AppError::validation("سعر التكلفة لا يمكن أن يكون سالباً").into());
        }
    }
    Ok(())
}

// --- insert_or_update_po ---------------------------------------------------------------------------

/// **`insert_or_update_po`** (`pub(crate)` — U-6's backorder path reuses it; `savePurchase`'s
/// create/update body, `purchaseService.ts:115-171` minus the `confirm` delegation, which the
/// caller drives itself). Runs U-3 first (an inactive supplier fails the whole call, including a
/// backorder spun off from a receipt — Q-U5). `existing_id` selects update vs. create;
/// `preallocated_number` lets U-6's backorder path (D-U6) reuse a document number allocated earlier
/// in the same transaction, before the receipt's own `next_number(Journal)` call.
pub(crate) async fn insert_or_update_po<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::UndoRegistry,
    input: PurchaseOrderInput,
    existing_id: Option<Id>,
    preallocated_number: Option<String>,
) -> TxResult<PurchaseOrder> {
    validate_lines(conn, input.supplier_id, &input.lines).await?;

    let settings = crate::core::settings::load(conn).await?;
    let tax_rate = crate::shared::defaults::purchase_tax_rate(conn).await?;
    let totals_lines: Vec<PurchaseTotalsLine> = input
        .lines
        .iter()
        .map(|l| PurchaseTotalsLine { qty: l.qty, cost_price: l.cost_price, discount: l.discount, discount_is_pct: l.discount_is_pct, tax_id: l.tax_id })
        .collect();
    let totals = compute_purchase_totals(conn, &totals_lines, input.invoice_discount.as_ref(), tax_rate).await?;

    let date = RawDocDate::parse(&input.date).map_err(|_| AppError::validation("تاريخ غير صالح"))?.resolve(&cx.clock);
    let supplier_invoice_date = super::parse_supplier_invoice_date(input.supplier_invoice_date.as_deref(), &cx.clock)?;
    let now = cx.clock.now;

    // `(input.landedCosts ?? []).map(...)` (`purchases.ts:136`): always an array on a saved order.
    let landed_costs_json = Some(
        input
            .landed_costs
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|l| super::super::dto::LandedCostLine { id: Id::new().to_string(), label: l.label.clone(), amount: l.amount, supplier_id: l.supplier_id, spread_by: l.spread_by })
            .collect::<Vec<_>>(),
    );

    let po: PoModel = if let Some(id) = existing_id {
        lock::for_update_by_id(conn, "purchase_orders", &id.to_string()).await.map_err(AppError::from)?;
        let found = PoEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("أمر الشراء غير موجود"))?;
        if found.status != crate::entities::purchases::purchase_orders::PurchaseStatus::Draft {
            return Err(AppError::validation("لا يمكن تعديل أمر شراء تم إرساله أو استلامه أو إلغاؤه").into());
        }

        PoLineEntity::delete_many().filter(PoLineColumn::PurchaseOrderId.eq(id)).exec(conn).await.map_err(AppError::from)?;
        insert_lines(conn, id, &input.lines).await?;

        let cost_center_id = default_cost_center_for(conn, found.branch_id, input.cost_center_id).await?;
        let mut am: PoActiveModel = found.clone().into();
        am.supplier_id = Set(input.supplier_id);
        am.date_day = Set(date.day);
        am.date_instant = Set(date.instant);
        am.note = Set(input.note.clone());
        am.invoice_discount_pct = Set(input.invoice_discount.as_ref().and_then(|d| d.pct));
        am.invoice_discount_amount = Set(input.invoice_discount.as_ref().and_then(|d| d.amount));
        am.landed_costs = Set(landed_costs_json.as_ref().map(|l| landed_costs_to_json_value(l)).flatten());
        am.supplier_invoice_no = Set(input.supplier_invoice_no.clone());
        am.supplier_invoice_date = Set(supplier_invoice_date);
        am.attachment_ids = Set(input.attachment_ids.clone().map(crate::entities::values::StringList));
        am.cost_center_id = Set(cost_center_id);
        am.sub_total = Set(totals.sub_total);
        am.tax_amount = Set(totals.tax_amount);
        am.grand_total = Set(totals.grand_total);
        am.tax_rate = Set(totals.tax_rate);
        am.updated_at = Set(now);
        am.update(conn).await.map_err(AppError::from)?
    } else {
        let branch_id = input.branch_id.unwrap_or(settings.default_branch_id);
        let number = match preallocated_number {
            Some(n) => n,
            None => {
                let prefix = branch_prefix(conn, Some(branch_id)).await?;
                format!("{prefix}{}", numbering::next_number(conn, DocumentKind::PurchaseOrder).await?)
            }
        };
        let cost_center_id = default_cost_center_for(conn, Some(branch_id), input.cost_center_id).await?;
        let id = Id::new();
        let am = PoActiveModel {
            id: Set(id),
            number: Set(number),
            supplier_id: Set(input.supplier_id),
            date_day: Set(date.day),
            date_instant: Set(date.instant),
            status: Set(crate::entities::purchases::purchase_orders::PurchaseStatus::Draft),
            sub_total: Set(totals.sub_total),
            tax_rate: Set(totals.tax_rate),
            tax_amount: Set(totals.tax_amount),
            grand_total: Set(totals.grand_total),
            payment_status: Set(crate::entities::purchases::purchase_orders::PaymentStatus::Unpaid),
            paid_amount: Set(Decimal::ZERO),
            returned_amount: Set(Decimal::ZERO),
            note: Set(input.note.clone()),
            invoice_discount_pct: Set(input.invoice_discount.as_ref().and_then(|d| d.pct)),
            invoice_discount_amount: Set(input.invoice_discount.as_ref().and_then(|d| d.amount)),
            landed_costs: Set(landed_costs_json.as_ref().map(|l| landed_costs_to_json_value(l)).flatten()),
            supplier_invoice_no: Set(input.supplier_invoice_no.clone()),
            supplier_invoice_date: Set(supplier_invoice_date),
            vat_not_recoverable: Set(None),
            sent_at: Set(None),
            backorder_of_id: Set(None),
            received_date_day: Set(None),
            received_date_instant: Set(None),
            attachment_ids: Set(input.attachment_ids.clone().map(crate::entities::values::StringList)),
            cost_center_id: Set(cost_center_id),
            branch_id: Set(Some(branch_id)),
            currency: Set(input.currency.clone()),
            exchange_rate: Set(input.exchange_rate),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::purchases::purchase_orders::SyncStatus::Local),
        };
        let inserted = am.insert(conn).await.map_err(AppError::from)?;
        insert_lines(conn, id, &input.lines).await?;
        inserted
    };

    if !input.confirm {
        // `savePurchase`'s `if (input.confirm) receivePurchase(...) else logActivity(...)`
        // (`purchaseService.ts:168-169`) — the draft-save log fires only on the non-confirm path;
        // the confirm path's own log line comes from U-6/`receive_purchase` instead.
        activity::log(
            conn,
            cx,
            registry,
            ActivityKind::Purchase,
            format!("حفظ أمر الشراء {} كمسودة", po.number),
            Some(date),
            Some(RouteRef::detail("purchase", po.id.to_string())),
        )
        .await?;
    }

    to_order_dto(conn, &po).await
}

async fn insert_lines<C: ConnectionTrait>(conn: &C, po_id: Id, lines: &[PurchaseLineInput]) -> TxResult<()> {
    for (i, l) in lines.iter().enumerate() {
        let am = PoLineActiveModel {
            id: Set(Id::new()),
            purchase_order_id: Set(po_id),
            position: Set(i as i16),
            product_id: Set(l.product_id),
            qty: Set(l.qty),
            cost_price: Set(l.cost_price),
            unit_id: Set(l.unit_id.clone()),
            unit_factor: Set(l.unit_factor),
            discount: Set(l.discount),
            discount_is_pct: Set(l.discount_is_pct),
            tax_id: Set(l.tax_id),
            received_qty: Set(None),
            batch_no: Set(None),
            expiry_date: Set(None),
            landed_cost_share: Set(None),
        };
        am.insert(conn).await.map_err(AppError::from)?;
    }
    Ok(())
}

// --- U-4 save_purchase_order -------------------------------------------------------------------

/// **U-4 `save_purchase_order(input, id)`** (`purchaseService.ts:115-171`): `insert_or_update_po`,
/// then `confirm` -> U-6 with every line's full base qty, else the draft-save log (already emitted
/// inside `insert_or_update_po`).
pub async fn save_purchase_order<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::UndoRegistry,
    input: PurchaseOrderInput,
    id: Option<Id>,
) -> TxResult<PurchaseOrder> {
    let confirm = input.confirm;
    let date = input.date.clone();
    let po = insert_or_update_po(conn, cx, registry, input, id, None).await?;

    if confirm {
        let lines = po_lines(conn, po.id).await?;
        let receive_input = ReceivePurchaseInput {
            date,
            lines: lines.iter().map(|l| ReceiveLineInput { product_id: l.product_id, received_qty: base_qty(l.qty, l.unit_factor), batches: None }).collect(),
            supplier_invoice_no: None,
            supplier_invoice_date: None,
            vat_not_recoverable: None,
            landed_costs: None,
            create_backorder: None,
        };
        return super::receive::receive_purchase(conn, cx, registry, po.id, receive_input).await;
    }

    Ok(po)
}

// --- U-5 send_purchase_order_to_supplier ---------------------------------------------------------

/// **U-5 `send_purchase_order_to_supplier(id)`** (`purchases.ts:174-184`).
pub async fn send_purchase_order_to_supplier<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, id: Id) -> TxResult<PurchaseOrder> {
    lock::for_update_by_id(conn, "purchase_orders", &id.to_string()).await.map_err(AppError::from)?;
    let po = PoEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("أمر الشراء غير موجود"))?;
    if po.status != crate::entities::purchases::purchase_orders::PurchaseStatus::Draft {
        return Err(AppError::validation("لا يمكن إرسال إلا مسودة").into());
    }
    let now = cx.clock.now;
    let mut am: PoActiveModel = po.clone().into();
    am.status = Set(crate::entities::purchases::purchase_orders::PurchaseStatus::Ordered);
    am.sent_at = Set(Some(now));
    am.updated_at = Set(now);
    let updated = am.update(conn).await.map_err(AppError::from)?;

    let now_date = crate::utils::dates::DocDate { day: cx.clock.today(), instant: Some(now) };
    activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Purchase,
        format!("إرسال أمر الشراء {} للمورد", updated.number),
        Some(now_date),
        Some(RouteRef::detail("purchase", id.to_string())),
    )
    .await?;

    to_order_dto(conn, &updated).await
}

// --- U-7 confirm_purchase_order -----------------------------------------------------------------

/// **U-7 `confirm_purchase_order(id)`** (`purchaseService.ts:119-124`, the service version, not
/// `purchases.ts:372`): lock + load (missing -> `أمر الشراء غير موجود`); U-6 with `{ date: now_date,
/// lines: po.lines -> { productId, receivedQty: qty * (unitFactor ?? 1) - (receivedQty ?? 0) } }` —
/// **unrounded**, as the service does (Q-U13, not `base_qty`/`line_remaining`'s `round2`).
pub async fn confirm_purchase_order<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, id: Id) -> TxResult<PurchaseOrder> {
    let po = PoEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("أمر الشراء غير موجود"))?;
    let lines = po_lines(conn, po.id).await?;
    let now_date_str = {
        let now_date = crate::utils::dates::DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) };
        now_date.key()
    };
    let receive_input = ReceivePurchaseInput {
        date: now_date_str,
        lines: lines
            .iter()
            .map(|l| ReceiveLineInput { product_id: l.product_id, received_qty: l.qty * l.unit_factor.unwrap_or(Decimal::ONE) - l.received_qty.unwrap_or(Decimal::ZERO), batches: None })
            .collect(),
        supplier_invoice_no: None,
        supplier_invoice_date: None,
        vat_not_recoverable: None,
        landed_costs: None,
        create_backorder: None,
    };
    super::receive::receive_purchase(conn, cx, registry, id, receive_input).await
}

// --- U-8 cancel_purchase_order -----------------------------------------------------------------

/// **U-8 `cancel_purchase_order(id)`** (`purchases.ts:378-385`).
pub async fn cancel_purchase_order<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, id: Id) -> TxResult<PurchaseOrder> {
    lock::for_update_by_id(conn, "purchase_orders", &id.to_string()).await.map_err(AppError::from)?;
    let po = PoEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("أمر الشراء غير موجود"))?;
    use crate::entities::purchases::purchase_orders::PurchaseStatus as S;
    if !matches!(po.status, S::Draft | S::Ordered) {
        return Err(AppError::validation("يمكن إلغاء المسودات والأوامر المرسلة فقط — استخدم مرتجع المشتريات للأوامر المستلمة").into());
    }
    let now = cx.clock.now;
    let mut am: PoActiveModel = po.clone().into();
    am.status = Set(S::Canceled);
    am.updated_at = Set(now);
    let updated = am.update(conn).await.map_err(AppError::from)?;

    let now_date = crate::utils::dates::DocDate { day: cx.clock.today(), instant: Some(now) };
    activity::log(conn, cx, registry, ActivityKind::Purchase, format!("إلغاء أمر الشراء {}", updated.number), Some(now_date), Some(RouteRef::detail("purchase", id.to_string()))).await?;

    to_order_dto(conn, &updated).await
}
