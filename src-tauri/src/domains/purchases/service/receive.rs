//! `domains::purchases::service::receive` — U-6 `receive_purchase` (the posting walkthrough, §3
//! steps 1-16) and `allocate_landed_costs`.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::catalog::products::Entity as ProductEntity;
use crate::entities::platform::activity::ActivityKind;
use crate::entities::purchases::purchase_order_lines::{ActiveModel as PoLineActiveModel, Model as PoLineModel};
use crate::entities::purchases::purchase_orders::{ActiveModel as PoActiveModel, Entity as PoEntity};
use crate::shared::activity;
use crate::shared::ledger::accounts::SystemRole;
use crate::shared::ledger::post::{self, AccountRef, PartyRef, PostJournal, PostingLine, SourceRef};
use crate::shared::stock::{self, batches, LockedProduct, StockRef};
use crate::utils::dates::RawDocDate;
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::route::RouteRef;

use super::super::dto::{LandedCostLine, LandedCostSpread, PurchaseLineInput, PurchaseOrder, PurchaseOrderInput, ReceiveLineInput, ReceivePurchaseInput};
use super::orders::insert_or_update_po;
use super::read::{landed_costs_to_json_value, po_lines, to_order_dto};
use super::{base_unit_cost, line_remaining, purchase_line_account};
use crate::domains::products::service::stock_lines::lock_line_products;

/// A stock-tracked line's value at order price, for `allocate_landed_costs`'s weighting pass
/// (`receivedValued.filter((r) => !r.isService)`, `purchases.ts`).
struct ReceivedStockLine {
    product_id: Id,
    qty: Decimal,
    value: Decimal,
}

/// A landed-cost line billed by a supplier other than the PO's own (a shipper, a customs broker).
struct OtherSupplierLine {
    supplier_id: Id,
    label: String,
    amount: Decimal,
}

struct AllocationResult {
    share_by_product: BTreeMap<Id, Decimal>,
    own_supplier_total: Decimal,
    other_supplier_lines: Vec<OtherSupplierLine>,
}

/// **`allocate_landed_costs(supplier_id, landed, received)`** (`purchases.ts:187-213`): spreads each
/// landed-cost line's `amount` over the stock-only receiving lines by `value` or `qty` weight,
/// simultaneously bucketing it into `own_supplier_total` (added to this PO's own AP) or
/// `other_supplier_lines` (a separate AP credit per other supplier). The per-line shares always
/// sum exactly to the landed cost (the `round2` remainder goes to the largest-weight line).
fn allocate_landed_costs(po_supplier_id: Id, landed: &[LandedCostLine], received: &[ReceivedStockLine]) -> AllocationResult {
    let mut share_by_product: BTreeMap<Id, Decimal> = BTreeMap::new();
    let mut own_supplier_total = Decimal::ZERO;
    let mut other_supplier_lines: Vec<OtherSupplierLine> = Vec::new();

    let stock_lines: Vec<&ReceivedStockLine> = received.iter().filter(|r| r.qty > Decimal::ZERO).collect();
    let total_value: Decimal = stock_lines.iter().fold(Decimal::ZERO, |a, r| a + r.value);
    let total_qty: Decimal = round2(stock_lines.iter().fold(Decimal::ZERO, |a, r| a + r.qty));

    for lc in landed {
        if !(lc.amount > Decimal::ZERO) {
            continue;
        }
        match lc.supplier_id {
            Some(sid) if sid != po_supplier_id => other_supplier_lines.push(OtherSupplierLine { supplier_id: sid, label: lc.label.clone(), amount: round2(lc.amount) }),
            _ => own_supplier_total = round2(own_supplier_total + lc.amount),
        }

        if stock_lines.is_empty() {
            continue;
        }
        let base = match lc.spread_by {
            LandedCostSpread::Qty => total_qty,
            LandedCostSpread::Value => total_value,
        };
        if !(base > Decimal::ZERO) {
            continue;
        }
        let weight = |r: &ReceivedStockLine| match lc.spread_by {
            LandedCostSpread::Qty => r.qty,
            LandedCostSpread::Value => r.value,
        };
        // Rule (ACC-0004): each share is round2'd and the rounding remainder goes to the largest-weight line (ties → first), so the shares sum exactly to the landed cost.
        let mut shares: Vec<Decimal> = stock_lines.iter().map(|r| round2((lc.amount * weight(*r)) / base)).collect();
        let mut largest = 0;
        for (i, r) in stock_lines.iter().enumerate() {
            if weight(*r) > weight(stock_lines[largest]) {
                largest = i;
            }
        }
        let allocated = shares.iter().fold(Decimal::ZERO, |a, s| a + *s);
        shares[largest] = round2(shares[largest] + round2(round2(lc.amount) - allocated));
        for (r, share) in stock_lines.iter().zip(shares) {
            let entry = share_by_product.entry(r.product_id).or_insert(Decimal::ZERO);
            *entry = round2(*entry + share);
        }
    }

    AllocationResult { share_by_product, own_supplier_total, other_supplier_lines }
}

/// **U-6 `receive_purchase(conn, cx, reg, id, input)`** (`pub(crate)`, `purchases.ts:228-369`) —
/// the 16-step posting walkthrough (07 §3).
pub async fn receive_purchase<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::UndoRegistry,
    id: Id,
    input: ReceivePurchaseInput,
) -> TxResult<PurchaseOrder> {
    // 1. Lock the PO row; reload; status/line guards.
    lock::for_update_by_id(conn, "purchase_orders", &id.to_string()).await.map_err(AppError::from)?;
    let po = PoEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("أمر الشراء غير موجود"))?;
    use crate::entities::purchases::purchase_orders::PurchaseStatus as S;
    if !matches!(po.status, S::Draft | S::Ordered) {
        return Err(AppError::validation("تم استلام أمر الشراء هذا بالفعل أو تم إلغاؤه").into());
    }
    if input.lines.is_empty() {
        return Err(AppError::validation("لا توجد كميات للاستلام").into());
    }

    let po_lines_rows = po_lines(conn, po.id).await?;

    // 2. Lock every product on the PO (S-1 reuse).
    let product_ids: Vec<Id> = po_lines_rows.iter().map(|l| l.product_id).collect();
    let mut locked = lock_line_products(conn, &product_ids).await?;

    // 3. `by_product` (first occurrence wins for the PO-line lookup, last value wins for the input
    // map — JS `Map` semantics, `purchaseService.ts:234`). Validate per input line.
    let mut by_product: BTreeMap<Id, &ReceiveLineInput> = BTreeMap::new();
    for l in &input.lines {
        by_product.insert(l.product_id, l);
    }
    for rl in &input.lines {
        let Some(po_line) = po_lines_rows.iter().find(|l| l.product_id == rl.product_id) else {
            return Err(AppError::validation("صنف غير موجود في أمر الشراء").into());
        };
        if rl.received_qty < Decimal::ZERO {
            return Err(AppError::validation("الكمية المستلمة لا يمكن أن تكون سالبة").into());
        }
        let remaining = line_remaining(po_line.qty, po_line.unit_factor, po_line.received_qty);
        if rl.received_qty > remaining + Decimal::new(1, 4) {
            let name = ProductEntity::find_by_id(rl.product_id).one(conn).await.map_err(AppError::from)?.map(|p| p.name).unwrap_or_default();
            return Err(AppError::validation(format!("الكمية المستلمة لـ \"{name}\" أكبر من المتبقي في الأمر")).into());
        }
    }
    if !by_product.values().any(|l| l.received_qty > Decimal::ZERO) {
        return Err(AppError::validation("أدخل كمية استلام واحدة على الأقل").into());
    }

    // 4. This receipt's own landed costs, or the PO's when the caller passes none.
    let landed: Vec<LandedCostLine> = if let Some(list) = &input.landed_costs {
        if list.is_empty() {
            Vec::new()
        } else {
            list.iter().map(|l| LandedCostLine { id: Id::new().to_string(), label: l.label.clone(), amount: l.amount, supplier_id: l.supplier_id, spread_by: l.spread_by }).collect()
        }
    } else {
        super::read::json_to_landed_costs(&po.landed_costs).unwrap_or_default()
    };

    // 5. Pass 1: value at order price, per receiving line.
    struct ReceivedValued {
        product_id: Id,
        qty: Decimal,
        value: Decimal,
        /// Not stock-tracked (service, or `stockMode 'none'`) — expensed, never inventory.
        is_service: bool,
    }
    let mut received_valued: Vec<ReceivedValued> = Vec::new();
    for l in input.lines.iter().filter(|l| l.received_qty > Decimal::ZERO) {
        let po_line = po_lines_rows.iter().find(|pl| pl.product_id == l.product_id).expect("validated above");
        let product = locked.get(&l.product_id).expect("locked above");
        let unit_cost = base_unit_cost(po_line.cost_price, po_line.unit_factor);
        received_valued.push(ReceivedValued {
            product_id: l.product_id,
            qty: l.received_qty,
            value: round2(l.received_qty * unit_cost),
            // Rule (ACC-0005): a non-stock line (service, or a product with stockMode 'none') is expensed to its purchase account, never debited to inventory.
            is_service: product.is_untracked(),
        });
    }

    // 6. Landed-cost spread over stock-tracked lines.
    let stock_only: Vec<ReceivedStockLine> = received_valued.iter().filter(|r| !r.is_service).map(|r| ReceivedStockLine { product_id: r.product_id, qty: r.qty, value: r.value }).collect();
    let allocation = allocate_landed_costs(po.supplier_id, &landed, &stock_only);

    // 6b. Rule (ACC-0012): each other-supplier landed cost becomes that supplier's own open payable
    // document (step 14b), so the supplier has to exist. Same refusal point as the mock's
    // `planReceipt` (right after the spread, before anything is written).
    let mut other_supplier_names: Vec<String> = Vec::with_capacity(allocation.other_supplier_lines.len());
    for other in &allocation.other_supplier_lines {
        let party = crate::entities::parties::parties::Entity::find_by_id(other.supplier_id)
            .filter(crate::entities::parties::parties::Column::Kind.eq("supplier"))
            .one(conn)
            .await
            .map_err(AppError::from)?;
        let Some(party) = party else {
            return Err(AppError::validation("مورد التكلفة الإضافية غير موجود").into());
        };
        other_supplier_names.push(party.name);
    }

    // 7. Post per line: non-stock -> service_by_account; else stock + landed share -> apply_change (+
    // batches).
    let date = RawDocDate::parse(&input.date).map_err(|_| AppError::validation("تاريخ غير صالح"))?.resolve(&cx.clock);
    let mut inventory_value = Decimal::ZERO;
    let mut service_by_account: Vec<(Id, Decimal)> = Vec::new();
    let default_purchase_account_id = crate::core::settings::load(conn).await?.accounting.and_then(|a| a.default_purchase_account_id);

    // Updated PO-line values, applied to the DB after this loop (`received_qty`/`landed_cost_share`
    // accumulate `round2`'d, matching the mock's exact accumulation points).
    let mut line_updates: BTreeMap<Id, PoLineModel> = po_lines_rows.iter().map(|l| (l.id, l.clone())).collect();
    let default_branch_id = crate::core::settings::load(conn).await?.default_branch_id;
    let branch_id = po.branch_id.unwrap_or(default_branch_id);

    for rv in &received_valued {
        let po_line_id = po_lines_rows.iter().find(|pl| pl.product_id == rv.product_id).expect("validated above").id;
        if rv.is_service {
            let acc_id = {
                let product = locked.get(&rv.product_id).expect("locked above");
                purchase_line_account(conn, product, default_purchase_account_id).await?
            };
            match service_by_account.iter_mut().find(|(a, _)| *a == acc_id) {
                Some((_, amount)) => *amount = round2(*amount + rv.value),
                None => service_by_account.push((acc_id, rv.value)),
            }
            let line = line_updates.get_mut(&po_line_id).expect("built above");
            line.received_qty = Some(round2(line.received_qty.unwrap_or(Decimal::ZERO) + rv.qty));
            continue;
        }

        let landed_share = allocation.share_by_product.get(&rv.product_id).copied().unwrap_or(Decimal::ZERO);
        let posted_value = round2(rv.value + landed_share);
        inventory_value += posted_value;

        {
            let line = line_updates.get_mut(&po_line_id).expect("built above");
            line.received_qty = Some(round2(line.received_qty.unwrap_or(Decimal::ZERO) + rv.qty));
            line.landed_cost_share = Some(round2(line.landed_cost_share.unwrap_or(Decimal::ZERO) + landed_share));
        }

        let track_batches = locked.get(&rv.product_id).expect("locked above").track_batches();
        {
            let p: &mut LockedProduct = locked.get_mut(&rv.product_id).expect("locked above");
            stock::apply_change(conn, cx, p, rv.qty, posted_value, "purchase", StockRef { id: po.id, number: po.number.clone() }, &date, Some(branch_id)).await?;
        }

        if track_batches {
            let requested: Vec<_> = by_product.get(&rv.product_id).and_then(|l| l.batches.as_ref()).map(|b| b.iter().filter(|x| x.qty > Decimal::ZERO).collect::<Vec<_>>()).unwrap_or_default();
            let unit_cost_with_landed = round2(posted_value / rv.qty);
            if !requested.is_empty() {
                for b in requested {
                    let batch_no = if b.batch_no.trim().is_empty() { format!("RCV-{}", cx.clock.now.timestamp_millis()) } else { b.batch_no.trim().to_string() };
                    batches::receive_batch(conn, cx, rv.product_id, b.qty, unit_cost_with_landed, batch_no, b.expiry_date, &date, &StockRef { id: po.id, number: po.number.clone() }).await?;
                }
            } else {
                batches::receive_batch(conn, cx, rv.product_id, rv.qty, unit_cost_with_landed, format!("RCV-{}", po.number), None, &date, &StockRef { id: po.id, number: po.number.clone() }).await?;
            }
        }
    }
    inventory_value = round2(inventory_value);

    // 8. all_received / RECEIVED status / received_date.
    let all_received = line_updates.values().all(|l| line_remaining(l.qty, l.unit_factor, l.received_qty) <= Decimal::new(1, 4));

    // 9. Non-VAT-supplier branch + partial-receipt VAT ratio.
    let sup = crate::entities::parties::parties::Entity::find_by_id(po.supplier_id).one(conn).await.map_err(AppError::from)?;
    let vat_not_recoverable = input.vat_not_recoverable.unwrap_or_else(|| sup.as_ref().and_then(|s| s.vat_number.as_deref()).map(str::trim).unwrap_or("").is_empty());
    let received_sum = round2(received_valued.iter().fold(Decimal::ZERO, |a, r| a + r.value));
    let received_ratio = if po.grand_total > Decimal::ZERO {
        let denom = if po.sub_total.is_zero() { Decimal::ONE } else { po.sub_total };
        round2(received_sum / denom)
    } else {
        Decimal::ONE
    };
    let vat_on_receipt = round2(po.tax_amount * received_ratio.min(Decimal::ONE));

    // 10. Posting lines.
    let dim_branch = Some(branch_id);
    let dim_cost_center = po.cost_center_id;
    let mut lines: Vec<PostingLine> = vec![{
        let mut l = PostingLine::debit(AccountRef::Role(SystemRole::Inventory), inventory_value);
        l.branch_id = dim_branch;
        l.cost_center_id = dim_cost_center;
        l
    }];
    for (acc_id, amount) in &service_by_account {
        let mut l = PostingLine::debit(AccountRef::Id(*acc_id), *amount);
        l.branch_id = dim_branch;
        l.cost_center_id = dim_cost_center;
        lines.push(l);
    }
    if vat_not_recoverable {
        if vat_on_receipt > Decimal::ZERO {
            let mut l = PostingLine::debit(AccountRef::Role(SystemRole::FreightIn), vat_on_receipt);
            l.description = Some("ضريبة مدخلات غير مستردة (مورد بدون رقم ضريبي) — أُضيفت إلى التكلفة".to_string());
            l.branch_id = dim_branch;
            l.cost_center_id = dim_cost_center;
            lines.push(l);
        }
    } else {
        let mut l = PostingLine::debit(AccountRef::Role(SystemRole::VatInput), vat_on_receipt);
        l.branch_id = dim_branch;
        l.cost_center_id = dim_cost_center;
        lines.push(l);
    }
    let ap_amount = round2(received_sum + vat_on_receipt + allocation.own_supplier_total);
    {
        let mut l = PostingLine::credit(AccountRef::Role(SystemRole::Payable), ap_amount);
        l.party = Some(PartyRef { kind: crate::entities::journal::journal_lines::PartyKind::Supplier, id: po.supplier_id });
        l.branch_id = dim_branch;
        l.cost_center_id = dim_cost_center;
        lines.push(l);
    }
    // ACC-0012: each other-supplier line's payable document number is drawn here — before step 12's
    // backorder number and step 13's journal counter (D-U6's fixed domain -> journal order, and the
    // mock's own draw order) — and named on its `Cr payable` line.
    let mut other_supplier_bills: Vec<(Id, String)> = Vec::with_capacity(allocation.other_supplier_lines.len());
    for other in &allocation.other_supplier_lines {
        let prefix = crate::shared::defaults::branch_prefix(conn, Some(branch_id)).await?;
        let number = format!("{prefix}{}", crate::shared::numbering::next_number(conn, crate::shared::numbering::DocumentKind::PurchaseOrder).await?);
        let mut l = PostingLine::credit(AccountRef::Role(SystemRole::Payable), other.amount);
        l.description = Some(format!("تكلفة إضافية \"{}\" — {}", other.label, number));
        l.party = Some(PartyRef { kind: crate::entities::journal::journal_lines::PartyKind::Supplier, id: other.supplier_id });
        l.branch_id = dim_branch;
        l.cost_center_id = dim_cost_center;
        lines.push(l);
        other_supplier_bills.push((Id::new(), number));
    }

    // 11. PO update: totals shrink to the received portion, VAT, flags, invoice no/date only when
    // present, landed costs only when the PO had none.
    let now = cx.clock.now;
    let mut po_am: PoActiveModel = po.clone().into();
    po_am.sub_total = Set(received_sum);
    po_am.tax_amount = Set(vat_on_receipt);
    po_am.grand_total = Set(ap_amount);
    po_am.vat_not_recoverable = Set(Some(vat_not_recoverable));
    if let Some(no) = &input.supplier_invoice_no {
        po_am.supplier_invoice_no = Set(Some(no.clone()));
    }
    if let Some(d) = super::parse_supplier_invoice_date(input.supplier_invoice_date.as_deref(), &cx.clock)? {
        po_am.supplier_invoice_date = Set(Some(d));
    }
    // `!po.landedCosts?.length` (`purchases.ts:425`): absent or empty.
    let had_no_landed_costs = super::read::json_to_landed_costs(&po.landed_costs).map_or(true, |l| l.is_empty());
    if !landed.is_empty() && had_no_landed_costs {
        po_am.landed_costs = Set(landed_costs_to_json_value(&landed));
    }
    po_am.status = Set(S::Received);
    po_am.received_date_day = Set(Some(date.day));
    po_am.received_date_instant = Set(date.instant);
    po_am.updated_at = Set(now);
    let updated_po = po_am.update(conn).await.map_err(AppError::from)?;

    for line in line_updates.into_values() {
        let am = PoLineActiveModel {
            id: Set(line.id),
            purchase_order_id: Set(line.purchase_order_id),
            position: Set(line.position),
            product_id: Set(line.product_id),
            qty: Set(line.qty),
            cost_price: Set(line.cost_price),
            unit_id: Set(line.unit_id),
            unit_factor: Set(line.unit_factor),
            discount: Set(line.discount),
            discount_is_pct: Set(line.discount_is_pct),
            tax_id: Set(line.tax_id),
            received_qty: Set(line.received_qty),
            batch_no: Set(line.batch_no),
            expiry_date: Set(line.expiry_date),
            landed_cost_share: Set(line.landed_cost_share),
        };
        am.update(conn).await.map_err(AppError::from)?;
    }

    // 12. Backorder prep (D-U6): when short, allocate the backorder's own `purchaseOrder` counter
    // value NOW — before step 13's `ledger::post` call consumes the `journal` counter — so every
    // command in this domain takes counters in the same fixed order (domain counter -> journal
    // counter), per the global lock-order rule (`core/lock.rs`) and this spec's D-U6. The backorder
    // row itself is still only inserted after the receipt's journal posts successfully (step 15),
    // matching the mock's own call order (`receivePurchase` posts, then spins off the backorder) —
    // only the *number* is drawn early, not the row.
    let short_lines: Vec<PurchaseLineInput> = po_lines(conn, updated_po.id)
        .await?
        .into_iter()
        .filter(|l| line_remaining(l.qty, l.unit_factor, l.received_qty) > Decimal::new(1, 4))
        .map(|l| PurchaseLineInput {
            product_id: l.product_id,
            qty: round2(line_remaining(l.qty, l.unit_factor, l.received_qty) / l.unit_factor.unwrap_or(Decimal::ONE)),
            cost_price: l.cost_price,
            unit_id: l.unit_id,
            unit_factor: l.unit_factor,
            discount: None,
            discount_is_pct: None,
            tax_id: l.tax_id,
        })
        .collect();
    let wants_backorder = input.create_backorder.unwrap_or(false) && !all_received && !short_lines.is_empty();
    let backorder_number = if wants_backorder {
        let prefix = crate::shared::defaults::branch_prefix(conn, Some(default_branch_id)).await?;
        Some(format!("{prefix}{}", crate::shared::numbering::next_number(conn, crate::shared::numbering::DocumentKind::PurchaseOrder).await?))
    } else {
        None
    };

    // 13. Post (balanced by construction: landed-cost shares sum to the landed cost, ACC-0004).
    post::post(
        conn,
        cx,
        PostJournal {
            date,
            description: format!("استلام أمر شراء {}", updated_po.number),
            entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
            source: Some(SourceRef { kind: "purchaseOrder".to_string(), id: updated_po.id, number: Some(updated_po.number.clone()) }),
            lines,
            allow_closed_period: false,
            attachment_ids: Vec::new(),
            template_id: None,
        },
    )
    .await?;

    // 14. Activity log.
    let supplier_name = sup.as_ref().map(|s| s.name.clone()).unwrap_or_default();
    activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Purchase,
        format!("استلام أمر الشراء {} من {} بقيمة {}", updated_po.number, supplier_name, crate::domains::products::service::stock_lines::to_fixed2(ap_amount)),
        Some(date),
        Some(RouteRef::detail("purchase", updated_po.id.to_string())),
    )
    .await?;

    // 14b. Rule (ACC-0012): the other suppliers' payable documents — a RECEIVED purchase order with no
    // stock lines whose total is the landed amount, so each shipper's `Cr payable` above has an open
    // document behind it (open documents, payment allocation, `supplier-allocation`). Its cost is
    // already inside this receipt's inventory posting; the document carries only the liability.
    for ((other, (bill_id, bill_number)), supplier_name) in allocation.other_supplier_lines.iter().zip(other_supplier_bills).zip(other_supplier_names) {
        let bill = PoActiveModel {
            id: Set(bill_id),
            number: Set(bill_number.clone()),
            supplier_id: Set(other.supplier_id),
            date_day: Set(date.day),
            date_instant: Set(date.instant),
            status: Set(S::Received),
            sub_total: Set(other.amount),
            tax_rate: Set(Decimal::ZERO),
            tax_amount: Set(Decimal::ZERO),
            grand_total: Set(other.amount),
            payment_status: Set(crate::entities::purchases::purchase_orders::PaymentStatus::Unpaid),
            paid_amount: Set(Decimal::ZERO),
            returned_amount: Set(Decimal::ZERO),
            note: Set(Some(format!("تكلفة إضافية \"{}\" على أمر الشراء {}", other.label, updated_po.number))),
            invoice_discount_pct: Set(None),
            invoice_discount_amount: Set(None),
            landed_costs: Set(None),
            supplier_invoice_no: Set(None),
            supplier_invoice_date: Set(None),
            vat_not_recoverable: Set(None),
            sent_at: Set(None),
            backorder_of_id: Set(None),
            received_date_day: Set(Some(date.day)),
            received_date_instant: Set(date.instant),
            attachment_ids: Set(None),
            cost_center_id: Set(updated_po.cost_center_id),
            branch_id: Set(Some(branch_id)),
            currency: Set(None),
            exchange_rate: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
            sync_status: Set(crate::entities::purchases::purchase_orders::SyncStatus::Local),
        };
        bill.insert(conn).await.map_err(AppError::from)?;
        activity::log(
            conn,
            cx,
            registry,
            ActivityKind::Purchase,
            format!(
                "مستحق {} لـ {} بقيمة {} — تكلفة إضافية على أمر الشراء {}",
                bill_number,
                supplier_name,
                crate::domains::products::service::stock_lines::to_fixed2(other.amount),
                updated_po.number
            ),
            Some(date),
            Some(RouteRef::detail("purchase", bill_id.to_string())),
        )
        .await?;
    }

    // 15. Short-delivery backorder — the DRAFT row itself, using the number already allocated above.
    if let Some(number) = backorder_number {
        let backorder_input = PurchaseOrderInput {
            supplier_id: updated_po.supplier_id,
            date: date.key(),
            lines: short_lines,
            note: Some(format!("أمر متبقٍ من {}", updated_po.number)),
            invoice_discount: None,
            landed_costs: None,
            supplier_invoice_no: None,
            supplier_invoice_date: None,
            attachment_ids: None,
            cost_center_id: None,
            branch_id: None,
            currency: None,
            exchange_rate: None,
            confirm: false,
        };
        let backorder = insert_or_update_po(conn, cx, registry, backorder_input, None, Some(number)).await?;
        let mut bo_am: PoActiveModel = PoEntity::find_by_id(backorder.id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::internal("أمر متبقٍ غير موجود بعد إنشائه", None))?.into();
        bo_am.backorder_of_id = Set(Some(updated_po.id));
        bo_am.updated_at = Set(now);
        bo_am.update(conn).await.map_err(AppError::from)?;
        activity::log(
            conn,
            cx,
            registry,
            ActivityKind::Purchase,
            format!("إنشاء أمر متبقٍ {} من {}", backorder.number, updated_po.number),
            Some(date),
            Some(RouteRef::detail("purchase", backorder.id.to_string())),
        )
        .await?;
    }

    to_order_dto(conn, &updated_po).await
}
