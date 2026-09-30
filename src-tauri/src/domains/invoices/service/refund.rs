//! `domains::invoices::service::refund` — `create_refund` (08 §3.5), a behaviour-exact port of
//! `recordRefund` (`src/mocks/backend/sales.ts`), including its G-25 fix (ACC-0003): a refund
//! reverses exactly its proportional share of each invoice line's snapshotted net and VAT
//! (`refund_line_share`, the port of `refundLineShare` in `src/modules/invoices/helpers/totals.ts`).

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
use crate::shared::currency::{latest_rate, refund_base_split, refund_cash_back_base, to_base};
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
/// without re-deriving the mapping. `invoice_lines` are the refunded invoice's lines: each refund
/// line's `invoice_line_id` row key is shown as that line's DTO id (`line_display_id`), the id the
/// UI sent (`returnedQtyByLine`/`Refund.lines[].invoiceLineId` in the mock hold the same id).
pub(crate) fn refund_to_dto(model: &crate::entities::sales::refunds::Model, lines: &[crate::entities::sales::refund_lines::Model], invoice_lines: &[crate::entities::sales::invoice_lines::Model]) -> Refund {
    Refund {
        id: model.id,
        number: model.number.clone(),
        invoice_id: model.invoice_id,
        date: model.date().key(),
        reason: model.reason.clone(),
        lines: lines.iter().map(|l| RefundLine { invoice_line_id: invoice_line_display_id(invoice_lines, l.invoice_line_id), qty: l.qty, restock: l.restock }).collect(),
        sub_total: model.sub_total,
        tax_amount: model.tax_amount,
        grand_total: model.grand_total,
        settled_to_receivable: model.settled_to_receivable,
        cash_back: model.cash_back,
        refund_method: model.refund_method.as_ref().map(refund_method_to_dto),
        credited_to_account: model.credited_to_account.filter(|v| *v > Decimal::ZERO),
    }
}

/// The DTO id of the invoice line whose row key is `line_id` (falls back to the row key, which a
/// refund line of this invoice always resolves, FK `refund_lines.invoice_line_id`).
pub(crate) fn invoice_line_display_id(invoice_lines: &[crate::entities::sales::invoice_lines::Model], line_id: Id) -> String {
    invoice_lines.iter().find(|l| l.id == line_id).map(|l| super::common::line_display_id(l.invoice_id, l.position)).unwrap_or_else(|| line_id.to_string())
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

/// `refundLineShare` (`src/modules/invoices/helpers/totals.ts`, G-25): a refund's `(net, vat)` share
/// of one invoice line's snapshotted `net`/`vat` (the VAT engine's per-line result at sale time — no
/// VAT is recomputed). Cumulative rounding: the amount already refunded for `q` units is
/// `round2(total × q / qty)` (the whole snapshot once `q` reaches `qty`), and this refund takes the
/// difference, so any sequence of partial refunds sums to the line's net and VAT exactly.
///
/// ACC-0017: the GROSS share is what gets rounded (`round2(gross × q / qty)`), with the VAT share
/// `round2(vat × q / qty)` and net = gross − VAT — rounding net and VAT separately let a unit drift
/// a halala off its price (2 × 100 inclusive, net 173.91 / VAT 26.09 → 100.01 then 99.99).
fn refund_line_share(qty: Decimal, net: Decimal, vat: Decimal, already_returned: Decimal, returning: Decimal) -> (Decimal, Decimal) {
    let gross = round2(net + vat);
    let refunded_up_to = |q: Decimal| if q >= qty { (gross, vat) } else { (round2((gross * q) / qty), round2((vat * q) / qty)) };
    let (before_gross, before_vat) = refunded_up_to(already_returned);
    let (after_gross, after_vat) = refunded_up_to(already_returned + returning);
    let share_vat = round2(after_vat - before_vat);
    (round2(round2(after_gross - before_gross) - share_vat), share_vat)
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
    let input_lines: Vec<RefundLine> = input.lines.iter().filter(|l| l.qty > Decimal::ZERO).cloned().collect();
    if input_lines.is_empty() {
        return Err(AppError::validation("اختر صنفاً واحداً على الأقل للإرجاع").into());
    }

    // 4. customer_credit requires a customer.
    if matches!(input.refund_method, Some(DtoRefundMethod::CustomerCredit)) && invoice.customer_id.is_none() {
        return Err(AppError::validation("رصيد العميل يتطلب فاتورة مرتبطة بعميل").into());
    }

    // 5. Per-line validation + refunded net/VAT/cost accumulation.
    // G-25 (ACC-0003): a refund reverses exactly its proportional share of each line's net and VAT
    // as the sale's VAT engine snapshotted them (`refund_line_share`) — never re-taxes a
    // tax-inclusive price — so net + VAT = refunded gross and a full refund reverses the sale's
    // revenue/VAT exactly.
    let invoice_lines = LineEntity::find().filter(LineColumn::InvoiceId.eq(invoice.id)).order_by_asc(LineColumn::Position).all(conn).await.map_err(AppError::from)?;
    let returned = returned_qty_by_line(conn, invoice.id).await?;
    let mut sub_total = Decimal::ZERO;
    let mut tax_amount = Decimal::ZERO;
    let mut cost = Decimal::ZERO;
    // Each input line resolved to its `invoice_lines` row (by the DTO id the UI sent).
    let mut lines: Vec<ResolvedLine> = Vec::with_capacity(input_lines.len());
    for input_line in &input_lines {
        let Some(inv_line) = invoice_lines.iter().find(|l| super::common::line_display_id(l.invoice_id, l.position) == input_line.invoice_line_id) else {
            return Err(AppError::validation("سطر الفاتورة غير موجود").into());
        };
        let line = ResolvedLine { invoice_line_id: inv_line.id, qty: input_line.qty, restock: input_line.restock };
        let already_returned = returned.get(&inv_line.id).copied().unwrap_or(Decimal::ZERO);
        let remaining = inv_line.qty - already_returned;
        if line.qty > remaining {
            return Err(AppError::validation(format!("لا يمكن إرجاع أكثر من {} من \"{}\"", js_number_string(remaining), inv_line.name)).into());
        }
        let (Some(line_net), Some(line_vat)) = (inv_line.net, inv_line.vat) else {
            return Err(AppError::validation(format!("سطر الفاتورة \"{}\" بلا تفصيل ضريبي — لا يمكن حساب المرتجع", inv_line.name)).into());
        };
        let (share_net, share_vat) = refund_line_share(inv_line.qty, line_net, line_vat, already_returned, line.qty);
        sub_total = round2(sub_total + share_net);
        tax_amount = round2(tax_amount + share_vat);
        if let Some(product_id) = inv_line.product_id {
            let is_product = crate::entities::catalog::products::Entity::find_by_id(product_id)
                .one(conn)
                .await
                .map_err(AppError::from)?
                .map(|p| p.r#type == "product")
                .unwrap_or(false);
            if is_product {
                cost += super::common::base_qty(line.qty, inv_line.unit_factor) * inv_line.cost_price;
            }
        }
        lines.push(line);
    }

    // 6. is_final (the last return for the invoice — its line shares already end on the exact
    // remainders, so it only drives the REFUNDED status).
    let is_final = invoice_lines.iter().all(|inv_line| {
        let returning = lines.iter().find(|l| l.invoice_line_id == inv_line.id).map(|l| l.qty).unwrap_or(Decimal::ZERO);
        returned.get(&inv_line.id).copied().unwrap_or(Decimal::ZERO) + returning >= inv_line.qty
    });
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

    // 7b. ACC-0009 (docs/v2/10 §2): an FC invoice's refund converts to base at the invoice's own
    // rate (`refund_base_split`, cumulative per invoice). The receivable and customer-credit legs stay
    // at that rate (AR is carried at the document's rate, like a payment allocation). Money paid back
    // in the invoice currency goes out at TODAY's rate (`latest_rate`, falling back to the invoice
    // rate) to the FC settlement account when one exists — same as `create_payment` — and the gap
    // against the obligation at the invoice's rate is realized FX (`FxGain`/`FxLoss`).
    let invoice_rate = match (&invoice.currency, invoice.exchange_rate) {
        (Some(_), Some(rate)) => Some(rate),
        _ => None,
    };
    let mut sub_total_base = sub_total;
    let mut tax_base = tax_amount;
    let mut settled_base = settled_to_receivable;
    let mut cash_back_base = cash_back;
    let mut paid_out_base = paid_out;
    let mut payout_rate: Option<Decimal> = None;
    if let (Some(rate), Some(currency)) = (invoice_rate, invoice.currency.as_deref()) {
        let prior = RefundEntity::find().filter(crate::entities::sales::refunds::Column::InvoiceId.eq(invoice.id)).all(conn).await.map_err(AppError::from)?;
        let before_net = round2(prior.iter().fold(Decimal::ZERO, |a, r| a + r.sub_total));
        let before_vat = round2(prior.iter().fold(Decimal::ZERO, |a, r| a + r.tax_amount));
        (sub_total_base, tax_base) = refund_base_split(before_net, before_vat, sub_total, tax_amount, rate);
        let total_base = round2(sub_total_base + tax_base);
        cash_back_base = refund_cash_back_base(settled_to_receivable, cash_back, total_base, rate);
        settled_base = round2(total_base - cash_back_base);
        if paid_out > Decimal::ZERO {
            let today_rate = latest_rate(conn, currency, cx.clock.today()).await?.unwrap_or(rate);
            payout_rate = Some(today_rate);
            paid_out_base = if today_rate == rate { cash_back_base } else { to_base(paid_out, today_rate) };
        }
    }
    let fx_gain_loss = if paid_out > Decimal::ZERO { round2(cash_back_base - paid_out_base) } else { Decimal::ZERO };
    let ar_fc = |l: &mut PostingLine, amount_fc: Decimal| {
        if let Some(rate) = invoice_rate {
            l.currency = invoice.currency.clone();
            l.amount_fc = Some(amount_fc);
            l.rate = Some(rate);
        }
    };

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

    // 13. Restock / write-off loop. ACC-0032: `line.qty` counts the invoice line's own unit (a box),
    // while stock, `cost_price` and the sale's COGS are all per BASE unit — so the restock/write-off
    // moves `qty × unit_factor` base units (the same `base_qty` the sale took out), never `line.qty`.
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
        let qty = super::common::base_qty(line.qty, inv_line.unit_factor);
        let value = round2(qty * inv_line.cost_price);
        if matches!(line.restock, Some(false)) {
            write_off_value = round2(write_off_value + value);
        } else {
            restock_value = round2(restock_value + value);
            let p: &mut LockedProduct = locked.get_mut(&product_id).expect("locked above");
            stock::apply_change(conn, cx, p, qty, value, "refund", StockRef { id: refund_id, number: number.clone() }, &date, None).await?;
        }
    }
    let restock_value = restock_value;
    let write_off_value = write_off_value;

    // 14. Settlement lines.
    let mut settlement_lines: Vec<PostingLine> = Vec::new();
    if credited > Decimal::ZERO {
        let mut l = PostingLine::credit(AccountRef::Role(SystemRole::Receivable), round2(settled_base + cash_back_base));
        l.party = invoice.customer_id.map(|id| PartyRef { kind: crate::entities::journal::journal_lines::PartyKind::Customer, id });
        ar_fc(&mut l, round2(settled_to_receivable + credited));
        settlement_lines.push(l);
    } else {
        let mut l = PostingLine::credit(AccountRef::Role(SystemRole::Receivable), settled_base);
        l.party = invoice.customer_id.map(|id| PartyRef { kind: crate::entities::journal::journal_lines::PartyKind::Customer, id });
        if settled_to_receivable > Decimal::ZERO {
            ar_fc(&mut l, settled_to_receivable);
        }
        settlement_lines.push(l);
        let settlement_method = match refund_method {
            DtoRefundMethod::Cash => "cash",
            DtoRefundMethod::Card => "card",
            DtoRefundMethod::BankTransfer => "bank_transfer",
            DtoRefundMethod::CustomerCredit => "cash",
        };
        if invoice_rate.is_some() {
            let ctx = AccountCtx { branch_id: None, currency: invoice.currency.clone() };
            let settlement_account = settlement_account_for(conn, settlement_method, &ctx).await?;
            let mut l = PostingLine::credit(AccountRef::Id(settlement_account.id), paid_out_base);
            if paid_out > Decimal::ZERO {
                l.currency = invoice.currency.clone();
                l.amount_fc = Some(paid_out);
                l.rate = payout_rate;
            }
            settlement_lines.push(l);
        } else {
            let settlement_account = settlement_account_for(conn, settlement_method, &AccountCtx::default()).await?;
            settlement_lines.push(PostingLine::credit(AccountRef::Id(settlement_account.id), paid_out));
        }
        if fx_gain_loss > Decimal::ZERO {
            settlement_lines.push(PostingLine { description: Some("فرق عملة محقق".to_string()), ..PostingLine::credit(AccountRef::Role(SystemRole::FxGain), fx_gain_loss) });
        } else if fx_gain_loss < Decimal::ZERO {
            settlement_lines.push(PostingLine { description: Some("فرق عملة محقق".to_string()), ..PostingLine::debit(AccountRef::Role(SystemRole::FxLoss), -fx_gain_loss) });
        }
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
                PostingLine::debit(AccountRef::Role(SystemRole::SalesReturns), sub_total_base),
                PostingLine::debit(AccountRef::Role(SystemRole::VatOutput), tax_base),
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
            paid_out_base,
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
    Ok(refund_to_dto(&refund_model, &inserted_lines, &invoice_lines))
}

/// One refund input line after its DTO line id resolved to the `invoice_lines` row key.
struct ResolvedLine {
    invoice_line_id: Id,
    qty: Decimal,
    restock: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// G-25 / ACC-0003: 1 × 115 tax-inclusive (net 100, VAT 15) refunds 115, not 130.
    #[test]
    fn full_refund_of_inclusive_line_reverses_its_snapshot_exactly() {
        assert_eq!(refund_line_share(dec!(1), dec!(100), dec!(15), Decimal::ZERO, dec!(1)), (dec!(100), dec!(15)));
    }

    /// ACC-0003's case: 3 × 99.99 inclusive, 10% invoice discount → line net 234.76, VAT 35.21.
    /// Three single-unit refunds each give back 89.99 gross and sum to the snapshot exactly.
    #[test]
    fn partial_refunds_sum_to_the_line_snapshot() {
        let (qty, net, vat) = (dec!(3), dec!(234.76), dec!(35.21));
        let r1 = refund_line_share(qty, net, vat, dec!(0), dec!(1));
        let r2 = refund_line_share(qty, net, vat, dec!(1), dec!(1));
        let r3 = refund_line_share(qty, net, vat, dec!(2), dec!(1));
        assert_eq!(r1, (dec!(78.25), dec!(11.74)));
        assert_eq!(r2, (dec!(78.26), dec!(11.73)));
        assert_eq!(r3, (dec!(78.25), dec!(11.74)));
        for (n, v) in [r1, r2, r3] {
            assert_eq!(n + v, dec!(89.99));
        }
        assert_eq!((r1.0 + r2.0 + r3.0, r1.1 + r2.1 + r3.1), (net, vat));
    }

    /// ACC-0017: 2 × 100 inclusive (net 173.91, VAT 26.09) — each unit refunds exactly 100, not
    /// 100.01 then 99.99, and the two refunds still reverse the line's net and VAT exactly.
    #[test]
    fn each_unit_refunds_its_exact_gross() {
        let (qty, net, vat) = (dec!(2), dec!(173.91), dec!(26.09));
        let r1 = refund_line_share(qty, net, vat, dec!(0), dec!(1));
        let r2 = refund_line_share(qty, net, vat, dec!(1), dec!(1));
        assert_eq!(r1, (dec!(86.95), dec!(13.05)));
        assert_eq!(r2, (dec!(86.96), dec!(13.04)));
        assert_eq!((r1.0 + r1.1, r2.0 + r2.1), (dec!(100), dec!(100)));
        assert_eq!((r1.0 + r2.0, r1.1 + r2.1), (net, vat));
    }
}
