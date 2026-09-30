//! `purchase_orders, purchase_order_lines, purchase_returns, purchase_return_lines`.

use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait};

use crate::core::error::AppError;
use crate::core::tx::{TxError, TxResult};
use crate::entities::purchases::purchase_order_lines::ActiveModel as PurchaseOrderLineActiveModel;
use crate::entities::purchases::purchase_orders::{ActiveModel as PurchaseOrderActiveModel, PaymentStatus, PurchaseStatus};
use crate::entities::purchases::purchase_return_lines::ActiveModel as PurchaseReturnLineActiveModel;
use crate::entities::purchases::purchase_returns::{ActiveModel as PurchaseReturnActiveModel, RefundMethod};
use crate::entities::values::StringList;
use crate::infrastructure::import::idmap::IdMap;
use crate::infrastructure::import::model::{PurchaseOrderV1, PurchaseReturnV1};
use crate::infrastructure::import::tables::{lenient_ref, parse_doc_date, resolve_created_at, strict_ref};
use crate::utils::id::Id;
use crate::utils::money::{round2, round4, round_qty};

pub async fn insert_purchase_orders<C: ConnectionTrait>(
    conn: &C,
    rows: &[PurchaseOrderV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(supplier_id) = id_map.resolve(&row.supplier_id) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في purchase_orders")));
        };
        let (day, instant) = parse_doc_date(&row.date, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في purchase_orders"))?;
        let received_date = row.received_date.as_deref().and_then(|s| parse_doc_date(s, tz));

        let sub_total = round2(row.sub_total);
        let tax_amount = round2(row.tax_amount);
        let grand_total = round2(row.grand_total);
        let paid_amount = round2(row.paid_amount);
        let returned_amount = round2(row.returned_amount);
        if sub_total != row.sub_total
            || tax_amount != row.tax_amount
            || grand_total != row.grand_total
            || paid_amount != row.paid_amount
            || returned_amount != row.returned_amount
        {
            *rounded += 1;
        }

        let status = match row.status.as_str() {
            "ORDERED" => PurchaseStatus::Ordered,
            "RECEIVED" => PurchaseStatus::Received,
            "CANCELED" => PurchaseStatus::Canceled,
            _ => PurchaseStatus::Draft,
        };
        let payment_status = match row.payment_status.as_str() {
            "PAID" => PaymentStatus::Paid,
            "PARTIALLY_PAID" => PaymentStatus::PartiallyPaid,
            _ => PaymentStatus::Unpaid,
        };

        let model = PurchaseOrderActiveModel {
            id: Set(id),
            number: Set(row.number.clone()),
            supplier_id: Set(supplier_id),
            date_day: Set(day),
            date_instant: Set(instant),
            status: Set(status),
            sub_total: Set(sub_total),
            tax_rate: Set(row.tax_rate),
            tax_amount: Set(tax_amount),
            grand_total: Set(grand_total),
            payment_status: Set(payment_status),
            paid_amount: Set(paid_amount),
            returned_amount: Set(returned_amount),
            note: Set(row.note.clone()),
            invoice_discount_pct: Set(row.invoice_discount_pct),
            invoice_discount_amount: Set(row.invoice_discount_amount.map(round2)),
            landed_costs: Set(row.landed_costs.clone()),
            supplier_invoice_no: Set(row.supplier_invoice_no.clone()),
            // A date key or an ISO instant (the seed/UI write `dateKeyToIso(...)`) → its business day; a
            // `%Y-%m-%d`-only parse dropped every instant-shaped value (Part 04 parity, L2).
            supplier_invoice_date: Set(row.supplier_invoice_date.as_deref().and_then(|s| parse_doc_date(s, tz)).map(|(day, _)| day)),
            vat_not_recoverable: Set(row.vat_not_recoverable),
            sent_at: Set(row.sent_at.as_deref().and_then(crate::infrastructure::import::model::parse_instant)),
            backorder_of_id: Set(None), // DEFERRED (self-ref) — set in run.rs phase B.
            received_date_day: Set(received_date.map(|(d, _)| d)),
            received_date_instant: Set(received_date.and_then(|(_, i)| i)),
            attachment_ids: Set(row.attachment_ids.clone().map(StringList)),
            cost_center_id: Set(lenient_ref(id_map, row.cost_center_id.as_deref())),
            branch_id: Set(strict_ref(id_map, row.branch_id.as_deref(), "purchase_orders")?),
            currency: Set(row.currency.clone().map(|c| c.to_uppercase())),
            exchange_rate: Set(row.exchange_rate.map(round4)),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::purchases::purchase_orders::SyncStatus::Local),
        };
        model.insert(conn).await.map_err(TxError::from)?;

        for (li, line) in row.lines.iter().enumerate() {
            let Some(product_id) = id_map.resolve(&line.product_id) else { continue };
            let qty = round_qty(line.qty);
            let cost_price = round4(line.cost_price);
            if qty != line.qty || cost_price != line.cost_price {
                *rounded += 1;
            }
            let line_model = PurchaseOrderLineActiveModel {
                id: Set(Id::new()),
                purchase_order_id: Set(id),
                position: Set(li as i16),
                product_id: Set(product_id),
                qty: Set(qty),
                cost_price: Set(cost_price),
                // The product's own `ProductUnit.id` (a free string kept verbatim in `products.units`),
                // never an id-map key — copied as is (m0020).
                unit_id: Set(line.unit_id.clone()),
                unit_factor: Set(line.unit_factor.map(round4)),
                discount: Set(line.discount.map(round2)),
                discount_is_pct: Set(line.discount_is_pct),
                tax_id: Set(strict_ref(id_map, line.tax_id.as_deref(), "purchase_order_lines")?),
                received_qty: Set(line.received_qty.map(round_qty)),
                batch_no: Set(line.batch_no.clone()),
                expiry_date: Set(line.expiry_date.as_deref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())),
                landed_cost_share: Set(line.landed_cost_share.map(round2)),
            };
            line_model.insert(conn).await.map_err(TxError::from)?;
        }
    }
    Ok(())
}

pub async fn insert_purchase_returns<C: ConnectionTrait>(
    conn: &C,
    rows: &[PurchaseReturnV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let (Some(purchase_order_id), Some(supplier_id)) = (id_map.resolve(&row.purchase_order_id), id_map.resolve(&row.supplier_id)) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في purchase_returns")));
        };
        let (day, instant) = parse_doc_date(&row.date, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في purchase_returns"))?;

        let sub_total = round2(row.sub_total);
        let tax_amount = round2(row.tax_amount);
        let grand_total = round2(row.grand_total);
        let settled_to_payable = round2(row.settled_to_payable);
        let cash_back = round2(row.cash_back);
        if sub_total != row.sub_total || tax_amount != row.tax_amount || grand_total != row.grand_total {
            *rounded += 1;
        }

        let refund_method = match row.refund_method.as_str() {
            "bank_transfer" => RefundMethod::BankTransfer,
            "credit" => RefundMethod::Credit,
            _ => RefundMethod::Cash,
        };

        let model = PurchaseReturnActiveModel {
            id: Set(id),
            number: Set(row.number.clone()),
            purchase_order_id: Set(purchase_order_id),
            supplier_id: Set(supplier_id),
            date_day: Set(day),
            date_instant: Set(instant),
            reason: Set(row.reason.clone()),
            sub_total: Set(sub_total),
            tax_amount: Set(tax_amount),
            grand_total: Set(grand_total),
            settled_to_payable: Set(settled_to_payable),
            cash_back: Set(cash_back),
            refund_method: Set(refund_method),
            from_draft_id: Set(lenient_ref(id_map, row.from_draft_id.as_deref())),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::purchases::purchase_returns::SyncStatus::Local),
        };
        model.insert(conn).await.map_err(TxError::from)?;

        for (li, line) in row.lines.iter().enumerate() {
            let Some(product_id) = id_map.resolve(&line.product_id) else { continue };
            let qty = round_qty(line.qty);
            let cost_price = round4(line.cost_price);
            if qty != line.qty || cost_price != line.cost_price {
                *rounded += 1;
            }
            let line_model = PurchaseReturnLineActiveModel {
                id: Set(Id::new()),
                purchase_return_id: Set(id),
                position: Set(li as i16),
                product_id: Set(product_id),
                qty: Set(qty),
                cost_price: Set(cost_price),
                batch_id: Set(strict_ref(id_map, line.batch_id.as_deref(), "purchase_return_lines")?),
            };
            line_model.insert(conn).await.map_err(TxError::from)?;
        }
    }
    Ok(())
}
