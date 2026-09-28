//! `domains::purchases::service::returns` — U-9 `record_purchase_return`, U-11
//! `post_debit_note_draft`.

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::platform::activity::ActivityKind;
use crate::entities::purchases::purchase_orders::{ActiveModel as PoActiveModel, Entity as PoEntity};
use crate::entities::purchases::purchase_return_lines::ActiveModel as RetLineActiveModel;
use crate::entities::purchases::purchase_returns::ActiveModel as RetActiveModel;
use crate::shared::activity;
use crate::shared::ledger::accounts::{settlement_account_for, AccountCtx, SystemRole};
use crate::shared::ledger::post::{self, AccountRef, PartyRef, PostJournal, PostingLine, SourceRef};
use crate::shared::numbering::{self, DocumentKind};
use crate::shared::stock::{self, batches, cost, LockedProduct, StockRef};
use crate::shared::totals::payment_status_for;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;
use crate::utils::money::js_number_string;
use crate::utils::route::RouteRef;

use super::super::dto::{DebitNoteLine, PurchaseReturn, PurchaseReturnInput, PurchaseReturnInputLine, RefundMethod};
use super::read::{payment_status_from_kind, refund_method_as_entity, to_return_dto};
use super::{base_qty, base_unit_cost, purchase_line_account, purchase_outstanding};
use crate::domains::products::service::stock_lines::lock_line_products;

/// **U-9 `record_purchase_return(conn, cx, reg, input, date)`** (`pub(crate)`,
/// `purchases.ts:396-521`).
#[allow(clippy::too_many_arguments)]
pub async fn record_purchase_return<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::UndoRegistry,
    input: PurchaseReturnInput,
    date_str: String,
) -> TxResult<PurchaseReturn> {
    // 1. Lock the PO row; validate status/reason/lines.
    lock::for_update_by_id(conn, "purchase_orders", &input.purchase_order_id.to_string()).await.map_err(AppError::from)?;
    let po = PoEntity::find_by_id(input.purchase_order_id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("أمر الشراء غير موجود"))?;
    use crate::entities::purchases::purchase_orders::PurchaseStatus as S;
    if po.status != S::Received {
        return Err(AppError::validation("يمكن الإرجاع من أوامر الشراء المستلمة فقط").into());
    }
    let reason = input.reason.clone().unwrap_or_default();
    if reason.trim().is_empty() {
        return Err(AppError::validation("سبب الإرجاع مطلوب").into());
    }
    let requested: Vec<PurchaseReturnInputLine> = input.lines.iter().filter(|l| l.qty > Decimal::ZERO).cloned().collect();
    if requested.is_empty() {
        return Err(AppError::validation("اختر صنفاً واحداً على الأقل للإرجاع").into());
    }

    // 2. Already-returned qty per product (under the PO lock) + lock requested products.
    let returned = super::returned_qty_by_product(conn, po.id).await?;
    let product_ids: Vec<Id> = requested.iter().map(|l| l.product_id).collect();
    let mut locked = lock_line_products(conn, &product_ids).await?;

    // 3. Per requested line: PO line must exist, product must exist, remaining-qty and stock-CONFLICT
    // guards, cost-price snapshot.
    let po_lines_rows = super::read::po_lines(conn, po.id).await?;
    let mut lines: Vec<DebitNoteLine> = Vec::with_capacity(requested.len());
    for line in &requested {
        let Some(po_line) = po_lines_rows.iter().find(|l| l.product_id == line.product_id) else {
            return Err(AppError::validation("الصنف غير موجود في أمر الشراء").into());
        };
        let Some(product) = locked.get(&line.product_id) else {
            return Err(AppError::not_found("المنتج غير موجود").into());
        };
        let name = product.model.name.clone();
        let base = base_qty(po_line.qty, po_line.unit_factor);
        let already = returned.get(&line.product_id).copied().unwrap_or(Decimal::ZERO);
        let remaining = crate::utils::money::round2(base - already);
        if line.qty > remaining {
            return Err(AppError::validation(format!("لا يمكن إرجاع أكثر من {} من \"{name}\"", js_number_string(remaining))).into());
        }
        if product.model.r#type == "product" && line.qty > product.stock_qty() {
            return Err(AppError::conflict(format!("المخزون الحالي من \"{name}\" ({}) أقل من كمية الإرجاع", js_number_string(product.stock_qty()))).into());
        }
        let base_cost = base_unit_cost(po_line.cost_price, po_line.unit_factor);
        let landed_per_unit = if po_line.landed_cost_share.unwrap_or(Decimal::ZERO) > Decimal::ZERO && base > Decimal::ZERO {
            po_line.landed_cost_share.unwrap_or(Decimal::ZERO) / base
        } else {
            Decimal::ZERO
        };
        let cost_price = crate::utils::money::round2(base_cost + landed_per_unit);
        lines.push(DebitNoteLine { product_id: line.product_id, qty: line.qty, cost_price, batch_id: line.batch_id });
    }

    // 4. Totals (no invoice discount on a return), settlement split, refund method.
    let totals_lines: Vec<super::PurchaseTotalsLine> =
        lines.iter().map(|l| super::PurchaseTotalsLine { qty: l.qty, cost_price: l.cost_price, discount: None, discount_is_pct: None, tax_id: None }).collect();
    let totals = super::compute_purchase_totals(conn, &totals_lines, None, po.tax_rate).await?;
    let po_dto = super::read::to_order_dto(conn, &po).await?;
    let outstanding = purchase_outstanding(&po_dto);
    let settled_to_payable = totals.grand_total.min(outstanding);
    let cash_back = crate::utils::money::round2(totals.grand_total - settled_to_payable);
    let refund_method = input.refund_method.unwrap_or(RefundMethod::Credit);

    // 5. Insert the return + lines; update PO returned_amount/payment_status.
    let date = crate::utils::dates::RawDocDate::parse(&date_str).map_err(|_| AppError::validation("تاريخ غير صالح"))?.resolve(&cx.clock);
    let number = numbering::next_number(conn, DocumentKind::PurchaseReturn).await?;
    let ret_id = Id::new();
    let now = cx.clock.now;
    let ret_am = RetActiveModel {
        id: Set(ret_id),
        number: Set(number.clone()),
        purchase_order_id: Set(po.id),
        supplier_id: Set(po.supplier_id),
        date_day: Set(date.day),
        date_instant: Set(date.instant),
        reason: Set(Some(reason.clone())),
        sub_total: Set(totals.sub_total),
        tax_amount: Set(totals.tax_amount),
        grand_total: Set(totals.grand_total),
        settled_to_payable: Set(settled_to_payable),
        cash_back: Set(cash_back),
        refund_method: Set(refund_method_as_entity(refund_method)),
        from_draft_id: Set(input.from_draft_id),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(crate::entities::purchases::purchase_returns::SyncStatus::Local),
    };
    let inserted_ret = ret_am.insert(conn).await.map_err(AppError::from)?;
    for (i, l) in lines.iter().enumerate() {
        let am = RetLineActiveModel { id: Set(Id::new()), purchase_return_id: Set(ret_id), position: Set(i as i16), product_id: Set(l.product_id), qty: Set(l.qty), cost_price: Set(l.cost_price), batch_id: Set(l.batch_id) };
        am.insert(conn).await.map_err(AppError::from)?;
    }

    let new_returned_amount = crate::utils::money::round2(po.returned_amount + totals.grand_total);
    let new_payment_status = payment_status_from_kind(payment_status_for(po.grand_total - new_returned_amount, po.paid_amount));

    // 6. Per line: service -> service_by_account; else cost_out_at_price + variance + apply_change
    // (+ batch draw).
    let branch_id = po.branch_id.unwrap_or(crate::core::settings::load(conn).await?.default_branch_id);
    let default_purchase_account_id = crate::core::settings::load(conn).await?.accounting.and_then(|a| a.default_purchase_account_id);
    let mut inventory_value = Decimal::ZERO;
    let mut variance = Decimal::ZERO;
    let mut service_by_account: Vec<(Id, Decimal)> = Vec::new();
    let today = cx.clock.today();

    for line in &lines {
        let product_type = locked.get(&line.product_id).expect("locked above").model.r#type.clone();
        if product_type == "service" {
            let acc_id = {
                let product = locked.get(&line.product_id).expect("locked above");
                purchase_line_account(conn, product, default_purchase_account_id).await?
            };
            let amount = crate::utils::money::round2(line.qty * line.cost_price);
            match service_by_account.iter_mut().find(|(a, _)| *a == acc_id) {
                Some((_, amt)) => *amt = crate::utils::money::round2(*amt + amount),
                None => service_by_account.push((acc_id, amount)),
            }
            continue;
        }

        let (stock_qty, stock_value) = {
            let p = locked.get(&line.product_id).expect("locked above");
            (p.stock_qty(), p.stock_value())
        };
        let (value_out, v) = cost::cost_out_at_price(stock_qty, stock_value, line.qty, line.cost_price);
        variance = crate::utils::money::round2(variance + v);
        inventory_value += value_out;

        let track_batches = locked.get(&line.product_id).expect("locked above").track_batches();
        {
            let p: &mut LockedProduct = locked.get_mut(&line.product_id).expect("locked above");
            stock::apply_change(conn, cx, p, -line.qty, -value_out, "purchase_return", StockRef { id: ret_id, number: number.clone() }, &date, Some(branch_id)).await?;
        }

        if track_batches {
            match line.batch_id {
                Some(batch_id) => {
                    batches::draw_batch(conn, cx, line.product_id, batch_id, line.qty).await?;
                }
                None => {
                    batches::consume_fefo(conn, cx, line.product_id, line.qty, true, today).await?;
                }
            }
        }
    }
    inventory_value = crate::utils::money::round2(inventory_value);

    // 7. Return posting lines (no branch/cost-center dims on VAT/variance, Q-U9).
    let dim = (Some(branch_id), po.cost_center_id);
    let mut posting_lines: Vec<PostingLine> = Vec::new();
    {
        let mut l = PostingLine::debit(AccountRef::Role(SystemRole::Payable), settled_to_payable);
        l.party = Some(PartyRef { kind: crate::entities::journal::journal_lines::PartyKind::Supplier, id: po.supplier_id });
        l.branch_id = dim.0;
        l.cost_center_id = dim.1;
        posting_lines.push(l);
    }
    {
        let refund_str = match refund_method {
            RefundMethod::Cash => "cash",
            RefundMethod::BankTransfer => "bank_transfer",
            RefundMethod::Credit => "credit",
        };
        let settlement_account = settlement_account_for(conn, refund_str, &AccountCtx::default()).await?;
        let mut l = PostingLine::debit(AccountRef::Id(settlement_account.id), if matches!(refund_method, RefundMethod::Credit) { Decimal::ZERO } else { cash_back });
        l.branch_id = dim.0;
        l.cost_center_id = dim.1;
        posting_lines.push(l);
    }
    {
        let mut l = PostingLine::debit(AccountRef::Role(SystemRole::Payable), if matches!(refund_method, RefundMethod::Credit) { cash_back } else { Decimal::ZERO });
        l.party = Some(PartyRef { kind: crate::entities::journal::journal_lines::PartyKind::Supplier, id: po.supplier_id });
        l.branch_id = dim.0;
        l.cost_center_id = dim.1;
        posting_lines.push(l);
    }
    {
        let mut l = PostingLine::credit(AccountRef::Role(SystemRole::Inventory), inventory_value);
        l.branch_id = dim.0;
        l.cost_center_id = dim.1;
        posting_lines.push(l);
    }
    for (acc_id, amount) in &service_by_account {
        let mut l = PostingLine::credit(AccountRef::Id(*acc_id), *amount);
        l.branch_id = dim.0;
        l.cost_center_id = dim.1;
        posting_lines.push(l);
    }
    if po.vat_not_recoverable.unwrap_or(false) {
        if totals.tax_amount > Decimal::ZERO {
            posting_lines.push(PostingLine::credit(AccountRef::Role(SystemRole::FreightIn), totals.tax_amount));
        }
    } else {
        posting_lines.push(PostingLine::credit(AccountRef::Role(SystemRole::VatInput), totals.tax_amount));
    }
    if variance > Decimal::ZERO {
        posting_lines.push(PostingLine::debit(AccountRef::Role(SystemRole::InventoryVariance), variance));
    } else if variance < Decimal::ZERO {
        posting_lines.push(PostingLine::credit(AccountRef::Role(SystemRole::InventoryVariance), -variance));
    }

    // 8. Post.
    post::post(
        conn,
        cx,
        PostJournal {
            date,
            description: format!("مرتجع مشتريات {number} على أمر الشراء {} — {reason}", po.number),
            entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
            source: Some(SourceRef { kind: "purchaseReturn".to_string(), id: ret_id, number: Some(number.clone()) }),
            lines: posting_lines,
            allow_closed_period: false,
            attachment_ids: Vec::new(),
            template_id: None,
        },
    )
    .await?;

    // PO's returned_amount/payment_status persist only after the posting succeeds (mirrors the
    // mock's own `mutate` call ordering — the PO row was already written above via `mutate`, but
    // the actual DB write happens here so the return row + PO update + journal post either all
    // commit together or none do, inside this same transaction).
    let mut po_am: PoActiveModel = po.clone().into();
    po_am.returned_amount = Set(new_returned_amount);
    po_am.payment_status = Set(super::read::payment_status_as_entity(new_payment_status));
    po_am.updated_at = Set(now);
    po_am.update(conn).await.map_err(AppError::from)?;

    // 9. Activity log.
    activity::log(
        conn,
        cx,
        registry,
        ActivityKind::PurchaseReturn,
        format!("مرتجع مشتريات {number} بقيمة {}", crate::domains::products::service::stock_lines::to_fixed2(totals.grand_total)),
        Some(date),
        Some(RouteRef::detail("purchase", po.id.to_string())),
    )
    .await?;

    to_return_dto(conn, &inserted_ret).await
}

/// `create_purchase_return(input)` (`purchaseService.ts:131-134`) = U-9 with `date = now_date`.
pub async fn create_purchase_return<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, input: PurchaseReturnInput) -> TxResult<PurchaseReturn> {
    let now_date = DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) };
    record_purchase_return(conn, cx, registry, input, now_date.key()).await
}

/// **U-11 `post_debit_note_draft(draft_id, refund)`** (`purchases.ts:547-575`).
pub async fn post_debit_note_draft<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::UndoRegistry,
    draft_id: Id,
    refund_method: Option<RefundMethod>,
) -> TxResult<PurchaseReturn> {
    lock::for_update_by_id(conn, "debit_note_drafts", &draft_id.to_string()).await.map_err(AppError::from)?;
    let draft = crate::entities::inventory::debit_note_drafts::Entity::find_by_id(draft_id)
        .one(conn)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("مسودة الإرجاع غير موجودة"))?;
    if draft.lines.0.is_empty() {
        return Err(AppError::validation("لا توجد أصناف في المسودة").into());
    }

    let mut po_ids: std::collections::BTreeSet<Id> = std::collections::BTreeSet::new();
    for line in &draft.lines.0 {
        let batch = crate::entities::catalog::product_batches::Entity::find_by_id(line.batch_id).one(conn).await.map_err(AppError::from)?;
        let po_id = match batch.as_ref().and_then(|b| b.source_ref_id) {
            Some(candidate) => {
                let po = PoEntity::find_by_id(candidate).one(conn).await.map_err(AppError::from)?;
                match po {
                    Some(p) if p.status == crate::entities::purchases::purchase_orders::PurchaseStatus::Received => Some(candidate),
                    _ => None,
                }
            }
            None => None,
        };
        let Some(po_id) = po_id else {
            return Err(AppError::conflict("إحدى التشغيلات ليست من أمر شراء مستلم — استخدم الإتلاف بدلاً من الإرجاع لهذه التشغيلة").into());
        };
        po_ids.insert(po_id);
    }
    if po_ids.len() > 1 {
        return Err(AppError::conflict("تشغيلات المسودة من أوامر شراء مختلفة — أنشئ مرتجعاً منفصلاً لكل أمر شراء").into());
    }
    let purchase_order_id = *po_ids.iter().next().expect("checked non-empty above");

    let note = draft.note.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "بضاعة منتهية/قاربت على انتهاء الصلاحية".to_string());
    let input = PurchaseReturnInput {
        purchase_order_id,
        reason: Some(note),
        refund_method,
        lines: draft.lines.0.iter().map(|l| PurchaseReturnInputLine { product_id: l.product_id, qty: l.qty, batch_id: Some(l.batch_id) }).collect(),
        from_draft_id: Some(draft.id),
    };
    let now_date = DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) };
    let ret = record_purchase_return(conn, cx, registry, input, now_date.key()).await?;

    // Only after the return posted successfully — a failed post leaves the draft in place for retry.
    crate::entities::inventory::debit_note_drafts::Entity::delete_by_id(draft.id).exec(conn).await.map_err(AppError::from)?;

    Ok(ret)
}
