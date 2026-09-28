//! `invoices, invoice_lines, invoice_tenders, refunds, refund_lines, quotations, quotation_lines,
//! held_sales, shifts, shift_movements`.

use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait};

use crate::core::error::AppError;
use crate::core::tx::{TxError, TxResult};
use crate::entities::sales::held_sales::ActiveModel as HeldSaleActiveModel;
use crate::entities::sales::invoice_lines::ActiveModel as InvoiceLineActiveModel;
use crate::entities::sales::invoice_tenders::ActiveModel as InvoiceTenderActiveModel;
use crate::entities::sales::invoices::{ActiveModel as InvoiceActiveModel, InvoiceSource, InvoiceStatus, InvoiceType, PaymentStatus, SalePaymentMethod};
use crate::entities::sales::quotation_lines::ActiveModel as QuotationLineActiveModel;
use crate::entities::sales::quotations::{ActiveModel as QuotationActiveModel, QuotationStatus};
use crate::entities::sales::refund_lines::ActiveModel as RefundLineActiveModel;
use crate::entities::sales::refunds::{ActiveModel as RefundActiveModel, RefundMethod};
use crate::entities::sales::shift_movements::{ActiveModel as ShiftMovementActiveModel, ShiftMovementKind};
use crate::entities::sales::shifts::{ActiveModel as ShiftActiveModel, HandoverMode, ShiftStatus};
use crate::entities::values::StringList;
use crate::infrastructure::import::idmap::{is_freetext_product_id, IdMap};
use crate::infrastructure::import::model::{HeldSaleV1, InvoiceLineV1, InvoiceV1, QuotationV1, RefundV1, ShiftV1};
use crate::infrastructure::import::tables::{parse_doc_date, resolve_created_at};
use crate::utils::id::Id;
use crate::utils::money::{round2, round4, round_qty};

fn resolve_line_product(id_map: &IdMap, old: Option<&str>) -> Option<Id> {
    let old = old?;
    if is_freetext_product_id(old) {
        None
    } else {
        id_map.resolve(old)
    }
}

fn round_line(id_map: &IdMap, line: &InvoiceLineV1, rounded: &mut i64) -> (Option<Id>, rust_decimal::Decimal, rust_decimal::Decimal, rust_decimal::Decimal, rust_decimal::Decimal) {
    let product_id = resolve_line_product(id_map, line.product_id.as_deref());
    let qty = round_qty(line.qty);
    let price = round2(line.price);
    let cost_price = round4(line.cost_price);
    let discount = round2(line.discount);
    if qty != line.qty || price != line.price || cost_price != line.cost_price || discount != line.discount {
        *rounded += 1;
    }
    (product_id, qty, price, cost_price, discount)
}

fn parse_invoice_status(s: &str) -> InvoiceStatus {
    match s {
        "COMPLETED" => InvoiceStatus::Completed,
        "REFUNDED" => InvoiceStatus::Refunded,
        _ => InvoiceStatus::Draft,
    }
}

fn parse_payment_status(s: &str) -> PaymentStatus {
    match s {
        "PAID" => PaymentStatus::Paid,
        "PARTIALLY_PAID" => PaymentStatus::PartiallyPaid,
        _ => PaymentStatus::Unpaid,
    }
}

fn parse_payment_method(s: &str) -> SalePaymentMethod {
    match s {
        "card" => SalePaymentMethod::Card,
        "bank_transfer" => SalePaymentMethod::BankTransfer,
        "credit" => SalePaymentMethod::Credit,
        _ => SalePaymentMethod::Cash,
    }
}

pub async fn insert_invoices<C: ConnectionTrait>(
    conn: &C,
    rows: &[InvoiceV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(cashier_id) = id_map.resolve(&row.cashier_id) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في invoices")));
        };
        let (day, instant) = parse_doc_date(&row.date, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في invoices"))?;
        let due_date = row.due_date.as_deref().and_then(|s| parse_doc_date(s, tz));

        let sub_total = round2(row.sub_total);
        let discount_amount = round2(row.discount_amount);
        let tax_amount = round2(row.tax_amount);
        let grand_total = round2(row.grand_total);
        let paid_amount = round2(row.paid_amount);
        let refunded_amount = round2(row.refunded_amount);
        if sub_total != row.sub_total
            || discount_amount != row.discount_amount
            || tax_amount != row.tax_amount
            || grand_total != row.grand_total
            || paid_amount != row.paid_amount
            || refunded_amount != row.refunded_amount
        {
            *rounded += 1;
        }

        let model = InvoiceActiveModel {
            id: Set(id),
            number: Set(row.number.clone()),
            date_day: Set(day),
            date_instant: Set(instant),
            customer_id: Set(row.customer_id.as_deref().and_then(|old| id_map.resolve(old))),
            cashier_id: Set(cashier_id),
            status: Set(parse_invoice_status(&row.status)),
            payment_status: Set(parse_payment_status(&row.payment_status)),
            sub_total: Set(sub_total),
            discount_rate: Set(round4(row.discount_rate)),
            discount_amount: Set(discount_amount),
            tax_rate: Set(row.tax_rate),
            tax_amount: Set(tax_amount),
            grand_total: Set(grand_total),
            payment_method: Set(parse_payment_method(&row.payment_method)),
            paid_amount: Set(paid_amount),
            refunded_amount: Set(refunded_amount),
            tendered_amount: Set(row.tendered_amount.map(round2)),
            due_date_day: Set(due_date.map(|(d, _)| d)),
            due_date_instant: Set(due_date.and_then(|(_, i)| i)),
            note: Set(row.note.clone()),
            source: Set(row.source.as_deref().map(|s| match s {
                "DESK" => InvoiceSource::Desk,
                _ => InvoiceSource::Pos,
            })),
            shift_id: Set(row.shift_id.as_deref().and_then(|old| id_map.resolve(old))),
            branch_id: Set(row.branch_id.as_deref().and_then(|old| id_map.resolve(old))),
            invoice_type: Set(row.invoice_type.as_deref().map(|s| match s {
                "SIMPLIFIED" => InvoiceType::Simplified,
                _ => InvoiceType::Standard,
            })),
            po_reference: Set(row.po_reference.clone()),
            terms: Set(row.terms.clone()),
            attachment_ids: Set(row.attachment_ids.clone().map(StringList)),
            currency: Set(row.currency.clone().map(|c| c.to_uppercase())),
            exchange_rate: Set(row.exchange_rate.map(round4)),
            cost_center_id: Set(row.cost_center_id.as_deref().and_then(|old| id_map.resolve(old))),
            search_normalized: sea_orm::ActiveValue::NotSet,
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::sales::invoices::SyncStatus::Local),
        };
        model.insert(conn).await.map_err(TxError::from)?;

        for (li, line) in row.lines.iter().enumerate() {
            let (product_id, qty, price, cost_price, discount) = round_line(id_map, line, rounded);
            let line_id = line.id.as_deref().map(|old| id_map.assign(old)).unwrap_or_else(Id::new);
            let line_model = InvoiceLineActiveModel {
                id: Set(line_id),
                invoice_id: Set(id),
                position: Set(li as i16),
                product_id: Set(product_id),
                name: Set(line.name.clone()),
                qty: Set(qty),
                price: Set(price),
                cost_price: Set(cost_price),
                discount: Set(discount),
                tax_id: Set(line.tax_id.as_deref().and_then(|old| id_map.resolve(old))),
                tax_category: Set(line.tax_category.clone()),
                tax_rate: Set(line.tax_rate),
                net: Set(line.net.map(round2)),
                vat: Set(line.vat.map(round2)),
                unit_id: Set(line.unit_id.as_deref().and_then(|old| id_map.resolve(old))),
                unit_factor: Set(line.unit_factor.map(round4)),
                list_price: Set(line.list_price.map(round2)),
                price_override_reason: Set(line.price_override_reason.clone()),
                batch_id: Set(line.batch_id.as_deref().and_then(|old| id_map.resolve(old))),
                batch_no: Set(line.batch_no.clone()),
                is_free_text: Set(line.is_free_text || line.product_id.as_deref().is_some_and(is_freetext_product_id)),
                revenue_account_id: Set(line.revenue_account_id.as_deref().and_then(|old| id_map.resolve(old))),
            };
            line_model.insert(conn).await.map_err(TxError::from)?;
        }

        for (ti, tender) in row.tenders.iter().enumerate() {
            let Some(payment_method_id) = id_map.resolve(&tender.payment_method_id) else { continue };
            let amount = round2(tender.amount);
            if amount != tender.amount {
                *rounded += 1;
            }
            let tender_model = InvoiceTenderActiveModel {
                id: Set(Id::new()),
                invoice_id: Set(id),
                position: Set(ti as i16),
                payment_method_id: Set(payment_method_id),
                amount: Set(amount),
                reference: Set(tender.reference.clone()),
            };
            tender_model.insert(conn).await.map_err(TxError::from)?;
        }
    }
    Ok(())
}

pub async fn insert_refunds<C: ConnectionTrait>(
    conn: &C,
    rows: &[RefundV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(invoice_id) = id_map.resolve(&row.invoice_id) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في refunds")));
        };
        let (day, instant) = parse_doc_date(&row.date, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في refunds"))?;

        let sub_total = round2(row.sub_total);
        let tax_amount = round2(row.tax_amount);
        let grand_total = round2(row.grand_total);
        let settled_to_receivable = round2(row.settled_to_receivable);
        let cash_back = round2(row.cash_back);
        if sub_total != row.sub_total || tax_amount != row.tax_amount || grand_total != row.grand_total {
            *rounded += 1;
        }

        let model = RefundActiveModel {
            id: Set(id),
            number: Set(row.number.clone()),
            invoice_id: Set(invoice_id),
            date_day: Set(day),
            date_instant: Set(instant),
            reason: Set(row.reason.clone()),
            sub_total: Set(sub_total),
            tax_amount: Set(tax_amount),
            grand_total: Set(grand_total),
            settled_to_receivable: Set(settled_to_receivable),
            cash_back: Set(cash_back),
            refund_method: Set(row.refund_method.as_deref().map(|s| match s {
                "card" => RefundMethod::Card,
                "bank_transfer" => RefundMethod::BankTransfer,
                "customer_credit" => RefundMethod::CustomerCredit,
                _ => RefundMethod::Cash,
            })),
            credited_to_account: Set(row.credited_to_account.map(round2)),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::sales::refunds::SyncStatus::Local),
        };
        model.insert(conn).await.map_err(TxError::from)?;

        for (li, line) in row.lines.iter().enumerate() {
            let Some(invoice_line_id) = id_map.resolve(&line.invoice_line_id) else { continue };
            let qty = round_qty(line.qty);
            if qty != line.qty {
                *rounded += 1;
            }
            let line_model = RefundLineActiveModel {
                id: Set(Id::new()),
                refund_id: Set(id),
                position: Set(li as i16),
                invoice_line_id: Set(invoice_line_id),
                qty: Set(qty),
                restock: Set(line.restock),
            };
            line_model.insert(conn).await.map_err(TxError::from)?;
        }
    }
    Ok(())
}

pub async fn insert_quotations<C: ConnectionTrait>(
    conn: &C,
    rows: &[QuotationV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(salesperson_id) = id_map.resolve(&row.salesperson_id) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في quotations")));
        };
        let (day, instant) = parse_doc_date(&row.date, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في quotations"))?;
        let expiry = row.expiry_date.as_deref().and_then(|s| parse_doc_date(s, tz));

        let sub_total = round2(row.sub_total);
        let discount_amount = round2(row.discount_amount);
        let tax_amount = round2(row.tax_amount);
        let grand_total = round2(row.grand_total);
        if sub_total != row.sub_total || discount_amount != row.discount_amount || tax_amount != row.tax_amount || grand_total != row.grand_total {
            *rounded += 1;
        }

        let status = match row.status.as_str() {
            "SENT" => QuotationStatus::Sent,
            "ACCEPTED" => QuotationStatus::Accepted,
            "REJECTED" => QuotationStatus::Rejected,
            "EXPIRED" => QuotationStatus::Expired,
            _ => QuotationStatus::Draft,
        };

        let model = QuotationActiveModel {
            id: Set(id),
            number: Set(row.number.clone()),
            date_day: Set(day),
            date_instant: Set(instant),
            expiry_date_day: Set(expiry.map(|(d, _)| d)),
            expiry_date_instant: Set(expiry.and_then(|(_, i)| i)),
            customer_id: Set(row.customer_id.as_deref().and_then(|old| id_map.resolve(old))),
            salesperson_id: Set(salesperson_id),
            status: Set(status),
            discount_rate: Set(round4(row.discount_rate)),
            discount_amount: Set(discount_amount),
            tax_amount: Set(tax_amount),
            sub_total: Set(sub_total),
            grand_total: Set(grand_total),
            note: Set(row.note.clone()),
            terms: Set(row.terms.clone()),
            po_reference: Set(row.po_reference.clone()),
            attachment_ids: Set(row.attachment_ids.clone().map(StringList)),
            converted_invoice_id: Set(row.converted_invoice_id.as_deref().and_then(|old| id_map.resolve(old))),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::sales::quotations::SyncStatus::Local),
        };
        model.insert(conn).await.map_err(TxError::from)?;

        for (li, line) in row.lines.iter().enumerate() {
            let (product_id, qty, price, cost_price, discount) = round_line(id_map, line, rounded);
            let line_model = QuotationLineActiveModel {
                id: Set(Id::new()),
                quotation_id: Set(id),
                position: Set(li as i16),
                product_id: Set(product_id),
                name: Set(line.name.clone()),
                qty: Set(qty),
                price: Set(price),
                cost_price: Set(cost_price),
                discount: Set(discount),
                tax_id: Set(line.tax_id.as_deref().and_then(|old| id_map.resolve(old))),
                tax_category: Set(line.tax_category.clone()),
                tax_rate: Set(line.tax_rate),
                net: Set(line.net.map(round2)),
                vat: Set(line.vat.map(round2)),
                unit_id: Set(line.unit_id.as_deref().and_then(|old| id_map.resolve(old))),
                unit_factor: Set(line.unit_factor.map(round4)),
                list_price: Set(line.list_price.map(round2)),
                price_override_reason: Set(line.price_override_reason.clone()),
                batch_id: Set(line.batch_id.as_deref().and_then(|old| id_map.resolve(old))),
                batch_no: Set(line.batch_no.clone()),
                is_free_text: Set(line.is_free_text || line.product_id.as_deref().is_some_and(is_freetext_product_id)),
                revenue_account_id: Set(line.revenue_account_id.as_deref().and_then(|old| id_map.resolve(old))),
            };
            line_model.insert(conn).await.map_err(TxError::from)?;
        }
    }
    Ok(())
}

/// D-5: the single legacy terminal string (usually `'pos-1'`) adopts `opts_adopt_terminal` when
/// given and exactly one distinct terminal string exists across `held_sales`+`shifts`; otherwise
/// every distinct terminal string gets one fresh `Id` (handoff §9's default rule), consistent
/// between `held_sales` and `shifts` since both resolve through the same `IdMap` (a terminal string
/// is not itself a row id anywhere, so `resolve_or_mint` is exactly the right mechanism — first use
/// mints, every later use of the same string reuses it).
fn resolve_terminal(id_map: &IdMap, old: &str, adopt_terminal: Option<Id>, single_terminal: Option<&str>) -> Id {
    if let (Some(adopt), Some(single)) = (adopt_terminal, single_terminal) {
        if single == old {
            return adopt;
        }
    }
    id_map.resolve_or_mint(old)
}

/// The distinct terminal id strings across the snapshot's `held_sales`+`shifts` — used by the
/// caller to decide whether `adopt_terminal` applies (exactly one distinct string, D-5).
pub fn distinct_terminal_ids<'a>(held_sales: &'a [HeldSaleV1], shifts: &'a [ShiftV1]) -> Vec<&'a str> {
    let mut set = std::collections::BTreeSet::new();
    for h in held_sales {
        set.insert(h.terminal_id.as_str());
    }
    for s in shifts {
        set.insert(s.terminal_id.as_str());
    }
    set.into_iter().collect()
}

pub async fn insert_held_sales<C: ConnectionTrait>(
    conn: &C,
    rows: &[HeldSaleV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    adopt_terminal: Option<Id>,
    single_terminal: Option<&str>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(held_by) = id_map.resolve(&row.held_by) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في held_sales")));
        };
        let (day, instant) = parse_doc_date(&row.held_at, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في held_sales"))?;
        let terminal_id = resolve_terminal(id_map, &row.terminal_id, adopt_terminal, single_terminal);

        let discount_rate = round4(row.discount_rate);
        if discount_rate != row.discount_rate {
            *rounded += 1;
        }

        let mut cart = row.cart.clone();
        id_map.remap_json(&mut cart);

        let model = HeldSaleActiveModel {
            id: Set(id),
            label: Set(row.label.clone()),
            terminal_id: Set(terminal_id),
            held_at_day: Set(day),
            held_at_instant: Set(instant),
            held_by: Set(held_by),
            customer_id: Set(row.customer_id.as_deref().and_then(|old| id_map.resolve(old))),
            discount_rate: Set(discount_rate),
            discount_is_pct: Set(row.discount_is_pct),
            note: Set(row.note.clone()),
            cart: Set(cart),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            sync_status: Set(crate::entities::sales::held_sales::SyncStatus::Local),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

pub async fn insert_shifts<C: ConnectionTrait>(
    conn: &C,
    rows: &[ShiftV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    adopt_terminal: Option<Id>,
    single_terminal: Option<&str>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(opened_by) = id_map.resolve(&row.opened_by) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في shifts")));
        };
        let (opened_day, opened_instant) =
            parse_doc_date(&row.opened_at, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في shifts"))?;
        let closed = row.closed_at.as_deref().and_then(|s| parse_doc_date(s, tz));
        let terminal_id = resolve_terminal(id_map, &row.terminal_id, adopt_terminal, single_terminal);

        let opening_float = round2(row.opening_float);
        if opening_float != row.opening_float {
            *rounded += 1;
        }

        let status = if row.status == "CLOSED" { ShiftStatus::Closed } else { ShiftStatus::Open };
        let handover_mode = row.handover_mode.as_deref().map(|s| if s == "DROP" { HandoverMode::Drop } else { HandoverMode::Handover });

        let model = ShiftActiveModel {
            id: Set(id),
            number: Set(row.number.clone()),
            terminal_id: Set(terminal_id),
            branch_id: Set(row.branch_id.as_deref().and_then(|old| id_map.resolve(old))),
            status: Set(status),
            opened_by: Set(opened_by),
            opened_at_day: Set(opened_day),
            opened_at_instant: Set(opened_instant),
            opening_float: Set(opening_float),
            opening_denominations: Set(row.opening_denominations.clone()),
            closed_by: Set(row.closed_by.as_deref().and_then(|old| id_map.resolve(old))),
            closed_at_day: Set(closed.map(|(d, _)| d)),
            closed_at_instant: Set(closed.and_then(|(_, i)| i)),
            counted_cash: Set(row.counted_cash.map(round2)),
            closing_denominations: Set(row.closing_denominations.clone()),
            expected_cash: Set(row.expected_cash.map(round2)),
            variance: Set(row.variance.map(round2)),
            handover_mode: Set(handover_mode),
            force_closed_by: Set(row.force_closed_by.as_deref().and_then(|old| id_map.resolve(old))),
            note: Set(row.note.clone()),
            open_key: sea_orm::ActiveValue::NotSet,
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            sync_status: Set(crate::entities::sales::shifts::SyncStatus::Local),
        };
        model.insert(conn).await.map_err(TxError::from)?;

        for (mi, movement) in row.movements.iter().enumerate() {
            let Some(by_user) = id_map.resolve(&movement.by_user) else { continue };
            let (at_day, at_instant) = match parse_doc_date(&movement.at, tz) {
                Some(v) => v,
                None => continue,
            };
            let amount = round2(movement.amount);
            if amount != movement.amount {
                *rounded += 1;
            }
            let kind = match movement.kind.as_str() {
                "REFUND_CASH" => ShiftMovementKind::RefundCash,
                "PAY_IN" => ShiftMovementKind::PayIn,
                "PAY_OUT" => ShiftMovementKind::PayOut,
                "BANK_DROP" => ShiftMovementKind::BankDrop,
                _ => ShiftMovementKind::SaleCash,
            };
            let movement_model = ShiftMovementActiveModel {
                id: Set(movement.id.as_deref().map(|old| id_map.assign(old)).unwrap_or_else(Id::new)),
                shift_id: Set(id),
                position: Set(mi as i16),
                kind: Set(kind),
                amount: Set(amount),
                note: Set(movement.note.clone()),
                ref_id: Set(movement.ref_id.as_deref().map(|old| id_map.resolve_or_mint(old))),
                ref_number: Set(movement.ref_number.clone()),
                at_day: Set(at_day),
                at_instant: Set(at_instant),
                by: Set(by_user),
            };
            movement_model.insert(conn).await.map_err(TxError::from)?;
        }
    }
    Ok(())
}
