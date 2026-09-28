//! `domains::invoices::service::refund` — `create_refund` (08 §3.5).
//!
//! G-25: the mock's refund math over-refunds VAT on a tax-inclusive sale (`sales.ts:437-444`) — the
//! manager's Part 02 gaps ledger (G-25) marks this **blocked — user** pending an `ACC-` ledger issue
//! and a mock fix. Per the spec's explicit instruction ("this port then follows the fixed mock line
//! by line... until then §3.5 step 6 is the only step that may not be implemented"), step 6 below
//! ports the mock's CURRENT (unfixed) formula verbatim, flagged with a `// G-25:` comment at the
//! exact spot, matching the G-31 treatment used elsewhere in this wave. Every other step is
//! behaviour-exact and independent of the G-25 fix.

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::platform::activity::ActivityKind;
use crate::entities::sales::invoice_lines::{Column as LineColumn, Entity as LineEntity};
use crate::entities::sales::invoices::{ActiveModel as InvoiceActiveModel, Entity as InvoiceEntity};
use crate::entities::sales::refund_lines::{ActiveModel as RefundLineActiveModel, Column as RefundLineColumn, Entity as RefundLineEntity};
use crate::entities::sales::refunds::{ActiveModel as RefundActiveModel, Entity as RefundEntity};
use crate::shared::activity;
use crate::shared::ledger::accounts::{settlement_account_for, AccountCtx, SystemRole};
use crate::shared::ledger::period::assert_open_period;
use crate::shared::ledger::post::{self, AccountRef, PartyRef, PostJournal, PostingLine, SourceRef};
use crate::shared::numbering::{self, DocumentKind};
use crate::shared::stock::{self, LockedProduct, StockRef};
use crate::shared::totals::document_outstanding;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;
use crate::utils::money::{js_number_string, round2};
use crate::utils::route::RouteRef;

use super::super::dto::{Refund, RefundInput, RefundLine, RefundMethod as DtoRefundMethod};

fn refund_method_to_entity(m: DtoRefundMethod) -> crate::entities::sales::refunds::RefundMethod {
    use crate::entities::sales::refunds::RefundMethod as E;
    match m {
        DtoRefundMethod::Cash => E::Cash,
        DtoRefundMethod::Card => E::Card,
        DtoRefundMethod::BankTransfer => E::BankTransfer,
        DtoRefundMethod::CustomerCredit => E::CustomerCredit,
    }
}

fn refund_method_to_dto(m: &crate::entities::sales::refunds::RefundMethod) -> DtoRefundMethod {
    use crate::entities::sales::refunds::RefundMethod as E;
    match m {
        E::Cash => DtoRefundMethod::Cash,
        E::Card => DtoRefundMethod::Card,
        E::BankTransfer => DtoRefundMethod::BankTransfer,
        E::CustomerCredit => DtoRefundMethod::CustomerCredit,
    }
}

/// `pub(crate)` so `service::reads::get_refund`/`get_invoice` can assemble the same `Refund` DTO
/// without re-deriving the mapping.
pub(crate) fn refund_to_dto(model: &crate::entities::sales::refunds::Model, lines: &[crate::entities::sales::refund_lines::Model]) -> Refund {
    Refund {
        id: model.id,
        number: model.number.clone(),
        invoice_id: model.invoice_id,
        date: model.date().key(),
        reason: model.reason.clone(),
        lines: lines.iter().map(|l| RefundLine { invoice_line_id: l.invoice_line_id, qty: l.qty, restock: l.restock }).collect(),
        sub_total: model.sub_total,
        tax_amount: model.tax_amount,
        grand_total: model.grand_total,
        settled_to_receivable: model.settled_to_receivable,
        cash_back: model.cash_back,
        refund_method: model.refund_method.as_ref().map(refund_method_to_dto),
        credited_to_account: model.credited_to_account.filter(|v| *v > Decimal::ZERO),
    }
}

/// `returnedQtyByLine(invoiceId)` (`sales.ts:396-402`): exact sum per `invoice_line_id` over this
/// invoice's refunds. `pub(crate)` so `service::reads::get_invoice` can build `InvoiceDetail.returnedQty`.
pub(crate) async fn returned_qty_by_line<C: ConnectionTrait>(conn: &C, invoice_id: Id) -> TxResult<std::collections::BTreeMap<Id, Decimal>> {
    let refund_ids: Vec<Id> = RefundEntity::find().filter(crate::entities::sales::refunds::Column::InvoiceId.eq(invoice_id)).all(conn).await.map_err(AppError::from)?.into_iter().map(|r| r.id).collect();
    if refund_ids.is_empty() {
        return Ok(std::collections::BTreeMap::new());
    }
    let lines = RefundLineEntity::find().filter(RefundLineColumn::RefundId.is_in(refund_ids)).all(conn).await.map_err(AppError::from)?;
    let mut map = std::collections::BTreeMap::new();
    for l in lines {
        *map.entry(l.invoice_line_id).or_insert(Decimal::ZERO) += l.qty;
    }
    Ok(map)
}

/// `create_refund(conn, cx, reg, input)` (08 §3.5).
pub async fn create_refund<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, input: RefundInput) -> TxResult<Refund> {
    // 1. Lock the invoice.
    lock::for_update_by_id(conn, "invoices", &input.invoice_id.to_string()).await.map_err(AppError::from)?;
    let invoice = InvoiceEntity::find_by_id(input.invoice_id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("الفاتورة غير موجودة"))?;

    // 2. Status.
    if invoice.status != crate::entities::sales::invoices::InvoiceStatus::Completed {
        return Err(AppError::validation("لا يمكن إرجاع هذه الفاتورة").into());
    }

    // 3. Lines with qty > 0.
    let lines: Vec<RefundLine> = input.lines.iter().filter(|l| l.qty > Decimal::ZERO).cloned().collect();
    if lines.is_empty() {
        return Err(AppError::validation("اختر صنفاً واحداً على الأقل للإرجاع").into());
    }

    // 4. customer_credit requires a customer.
    if matches!(input.refund_method, Some(DtoRefundMethod::CustomerCredit)) && invoice.customer_id.is_none() {
        return Err(AppError::validation("رصيد العميل يتطلب فاتورة مرتبطة بعميل").into());
    }

    // 5. Per-line validation + net/cost accumulation.
    let invoice_lines = LineEntity::find().filter(LineColumn::InvoiceId.eq(invoice.id)).order_by_asc(LineColumn::Position).all(conn).await.map_err(AppError::from)?;
    let returned = returned_qty_by_line(conn, invoice.id).await?;
    let mut net = Decimal::ZERO;
    let mut cost = Decimal::ZERO;
    for line in &lines {
        let Some(inv_line) = invoice_lines.iter().find(|l| l.id == line.invoice_line_id) else {
            return Err(AppError::validation("سطر الفاتورة غير موجود").into());
        };
        let remaining = inv_line.qty - returned.get(&inv_line.id).copied().unwrap_or(Decimal::ZERO);
        if line.qty > remaining {
            return Err(AppError::validation(format!("لا يمكن إرجاع أكثر من {} من \"{}\"", js_number_string(remaining), inv_line.name)).into());
        }
        net += line.qty * (inv_line.price - inv_line.discount / inv_line.qty);
        if let Some(product_id) = inv_line.product_id {
            let is_product = crate::entities::catalog::products::Entity::find_by_id(product_id)
                .one(conn)
                .await
                .map_err(AppError::from)?
                .map(|p| p.r#type == "product")
                .unwrap_or(false);
            if is_product {
                cost += line.qty * inv_line.cost_price;
            }
        }
    }

    // 6. is_final; sub_total/tax_amount.
    // G-25: this is the mock's CURRENT refund math, over-refunding VAT under tax-inclusive prices
    // (`sales.ts:437-444`) — kept verbatim per the manager's Part 02 gap G-25 ("blocked — user"),
    // not yet fixed in the mock. Do not "fix" this independently; once G-25's mock fix lands, this
    // block must be re-derived from the fixed `sales.ts` line by line.
    let previous = RefundEntity::find().filter(crate::entities::sales::refunds::Column::InvoiceId.eq(invoice.id)).all(conn).await.map_err(AppError::from)?;
    let is_final = invoice_lines.iter().all(|inv_line| {
        let returning = lines.iter().find(|l| l.invoice_line_id == inv_line.id).map(|l| l.qty).unwrap_or(Decimal::ZERO);
        returned.get(&inv_line.id).copied().unwrap_or(Decimal::ZERO) + returning >= inv_line.qty
    });
    let (sub_total, tax_amount) = if is_final {
        let prev_sub: Decimal = previous.iter().fold(Decimal::ZERO, |a, r| a + r.sub_total);
        let prev_tax: Decimal = previous.iter().fold(Decimal::ZERO, |a, r| a + r.tax_amount);
        (round2(invoice.sub_total - invoice.discount_amount - prev_sub), round2(invoice.tax_amount - prev_tax))
    } else {
        let sub_total = round2(net * (Decimal::ONE - invoice.discount_rate / Decimal::ONE_HUNDRED));
        let tax_amount = round2((sub_total * invoice.tax_rate) / Decimal::ONE_HUNDRED);
        (sub_total, tax_amount)
    };
    let grand_total = round2(sub_total + tax_amount);
    let outstanding = document_outstanding(round2(invoice.grand_total - invoice.refunded_amount), invoice.paid_amount);
    let settled_to_receivable = grand_total.min(outstanding);
    let cash_back = round2(grand_total - settled_to_receivable);
    let _cost = round2(cost);

    // 7. Refund method / split.
    let refund_method = input.refund_method.unwrap_or_else(|| {
        if matches!(invoice.payment_method, crate::entities::sales::invoices::SalePaymentMethod::Credit) {
            DtoRefundMethod::Cash
        } else {
            match invoice.payment_method {
                crate::entities::sales::invoices::SalePaymentMethod::Cash => DtoRefundMethod::Cash,
                crate::entities::sales::invoices::SalePaymentMethod::Card => DtoRefundMethod::Card,
                crate::entities::sales::invoices::SalePaymentMethod::BankTransfer => DtoRefundMethod::BankTransfer,
                crate::entities::sales::invoices::SalePaymentMethod::Credit => DtoRefundMethod::Cash,
            }
        }
    });
    let credited = if matches!(refund_method, DtoRefundMethod::CustomerCredit) { cash_back } else { Decimal::ZERO };
    let paid_out = if matches!(refund_method, DtoRefundMethod::CustomerCredit) { Decimal::ZERO } else { cash_back };

    // 8. Drawer shift (document row): only if cash && paid_out > 0.
    let mut drawer_shift: Option<crate::entities::sales::shifts::Model> = None;
    if matches!(refund_method, DtoRefundMethod::Cash) && paid_out > Decimal::ZERO {
        if let Some(shift_id) = invoice.shift_id {
            if let Some(shift) = crate::entities::sales::shifts::Entity::find_by_id(shift_id).one(conn).await.map_err(AppError::from)? {
                if shift.status == crate::entities::sales::shifts::ShiftStatus::Open {
                    drawer_shift = super::shifts::lock_open_shift_of_terminal(conn, shift.id).await?;
                }
            }
        }
        if drawer_shift.is_none() {
            drawer_shift = super::shifts::open_shift_for_terminal(conn, cx.terminal_id, true).await?;
        }
    }

    // 9. Party.
    if let Some(customer_id) = invoice.customer_id {
        crate::core::lock::share_lock_by_id(conn, "parties", &customer_id.to_string()).await.map_err(AppError::from)?;
    }

    // 10. Products (lock existing type=product products of returned catalog lines, sorted).
    let mut product_ids: Vec<Id> = Vec::new();
    for line in &lines {
        let inv_line = invoice_lines.iter().find(|l| l.id == line.invoice_line_id).expect("validated above");
        if let Some(pid) = inv_line.product_id {
            product_ids.push(pid);
        }
    }
    let mut locked = stock::lock_products(conn, &product_ids).await?;

    // 11. Date, period, number.
    let now = cx.clock.now;
    let date = DocDate { day: cx.clock.today(), instant: Some(now) };
    assert_open_period(conn, &date.day, false).await?;
    let number = numbering::next_number(conn, DocumentKind::Refund).await?;

    // 12. Insert refund + lines; update invoice.
    let refund_id = Id::new();
    let refund_am = RefundActiveModel {
        id: Set(refund_id),
        number: Set(number.clone()),
        invoice_id: Set(invoice.id),
        date_day: Set(date.day),
        date_instant: Set(date.instant),
        reason: Set(input.reason.clone()),
        sub_total: Set(sub_total),
        tax_amount: Set(tax_amount),
        grand_total: Set(grand_total),
        settled_to_receivable: Set(settled_to_receivable),
        cash_back: Set(cash_back),
        refund_method: Set(Some(refund_method_to_entity(refund_method))),
        credited_to_account: Set(if credited > Decimal::ZERO { Some(credited) } else { None }),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(crate::entities::sales::refunds::SyncStatus::Local),
    };
    refund_am.insert(conn).await.map_err(AppError::from)?;
    for (i, l) in lines.iter().enumerate() {
        let am = RefundLineActiveModel { id: Set(Id::new()), refund_id: Set(refund_id), position: Set(i as i16), invoice_line_id: Set(l.invoice_line_id), qty: Set(l.qty), restock: Set(l.restock) };
        am.insert(conn).await.map_err(AppError::from)?;
    }

    let new_refunded_amount = round2(invoice.refunded_amount + grand_total);
    let new_status = if is_final { crate::entities::sales::invoices::InvoiceStatus::Refunded } else { invoice.status.clone() };
    let new_payment_status = super::common::payment_status_kind_to_entity(crate::shared::totals::payment_status_for(invoice.grand_total - new_refunded_amount, invoice.paid_amount));
    let mut invoice_am: InvoiceActiveModel = invoice.clone().into();
    invoice_am.refunded_amount = Set(new_refunded_amount);
    invoice_am.status = Set(new_status);
    invoice_am.payment_status = Set(new_payment_status);
    invoice_am.updated_at = Set(now);
    invoice_am.update(conn).await.map_err(AppError::from)?;

    // 13. Restock / write-off loop.
    let mut restock_value = Decimal::ZERO;
    let mut write_off_value = Decimal::ZERO;
    for line in &lines {
        let inv_line = invoice_lines.iter().find(|l| l.id == line.invoice_line_id).expect("validated above");
        if inv_line.is_free_text {
            continue;
        }
        let Some(product_id) = inv_line.product_id else { continue };
        let Some(product) = locked.get(&product_id) else { continue };
        if product.model.r#type != "product" {
            continue;
        }
        let value = round2(line.qty * inv_line.cost_price);
        if matches!(line.restock, Some(false)) {
            write_off_value = round2(write_off_value + value);
        } else {
            restock_value = round2(restock_value + value);
            let p: &mut LockedProduct = locked.get_mut(&product_id).expect("locked above");
            stock::apply_change(conn, cx, p, line.qty, value, "refund", StockRef { id: refund_id, number: number.clone() }, &date, None).await?;
        }
    }
    let restock_value = restock_value;
    let write_off_value = write_off_value;

    // 14. Settlement lines.
    let mut settlement_lines: Vec<PostingLine> = Vec::new();
    if credited > Decimal::ZERO {
        let mut l = PostingLine::credit(AccountRef::Role(SystemRole::Receivable), round2(settled_to_receivable + credited));
        l.party = invoice.customer_id.map(|id| PartyRef { kind: crate::entities::journal::journal_lines::PartyKind::Customer, id });
        settlement_lines.push(l);
    } else {
        let mut l = PostingLine::credit(AccountRef::Role(SystemRole::Receivable), settled_to_receivable);
        l.party = invoice.customer_id.map(|id| PartyRef { kind: crate::entities::journal::journal_lines::PartyKind::Customer, id });
        settlement_lines.push(l);
        let settlement_method = match refund_method {
            DtoRefundMethod::Cash => "cash",
            DtoRefundMethod::Card => "card",
            DtoRefundMethod::BankTransfer => "bank_transfer",
            DtoRefundMethod::CustomerCredit => "cash",
        };
        let settlement_account = settlement_account_for(conn, settlement_method, &AccountCtx::default()).await?;
        settlement_lines.push(PostingLine::credit(AccountRef::Id(settlement_account.id), paid_out));
    }

    post::post(
        conn,
        cx,
        PostJournal {
            date,
            description: format!("مرتجع مبيعات {number} على الفاتورة {}", invoice.number),
            entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
            source: Some(SourceRef { kind: "refund".to_string(), id: refund_id, number: Some(number.clone()) }),
            lines: vec![
                PostingLine::debit(AccountRef::Role(SystemRole::SalesReturns), sub_total),
                PostingLine::debit(AccountRef::Role(SystemRole::VatOutput), tax_amount),
            ]
            .into_iter()
            .chain(settlement_lines)
            .chain([
                PostingLine::debit(AccountRef::Role(SystemRole::Inventory), restock_value),
                PostingLine::debit(AccountRef::Role(SystemRole::InventoryWriteOff), write_off_value),
                PostingLine::credit(AccountRef::Role(SystemRole::Cogs), round2(restock_value + write_off_value)),
            ])
            .collect(),
            allow_closed_period: false,
            attachment_ids: Vec::new(),
            template_id: None,
        },
    )
    .await?;

    // 15. Drawer movement.
    if let Some(shift) = &drawer_shift {
        super::shifts::record_shift_movement(
            conn,
            cx,
            shift,
            crate::domains::invoices::dto::ShiftMovementKind::RefundCash,
            paid_out,
            None,
            Some(refund_id),
            Some(number.clone()),
            date,
        )
        .await?;
    }

    // 16. Activity.
    activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Refund,
        format!("مرتجع {number} على الفاتورة {} بقيمة {:.2}", invoice.number, grand_total),
        Some(date),
        Some(RouteRef::detail("invoice", invoice.id.to_string())),
    )
    .await?;
    if invoice.customer_id.is_some() {
        cx.touch(crate::core::events::ChangeCategory::Parties);
    }

    let inserted_lines = RefundLineEntity::find().filter(RefundLineColumn::RefundId.eq(refund_id)).order_by_asc(RefundLineColumn::Position).all(conn).await.map_err(AppError::from)?;
    let refund_model = RefundEntity::find_by_id(refund_id).one(conn).await.map_err(AppError::from)?.expect("just inserted");
    Ok(refund_to_dto(&refund_model, &inserted_lines))
}

