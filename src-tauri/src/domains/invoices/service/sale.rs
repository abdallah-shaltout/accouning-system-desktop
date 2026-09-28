//! `domains::invoices::service::sale` — `prepare_sale` (08 §3.2, shared by preview and create),
//! `preview_sale` (§3.3), `create_sale` (§3.4).

use std::collections::BTreeMap;
use std::str::FromStr;

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::catalog::products::{self, Entity as ProductEntity};
use crate::entities::platform::activity::ActivityKind;
use crate::entities::sales::invoice_lines::ActiveModel as LineActiveModel;
use crate::entities::sales::invoice_tenders::ActiveModel as TenderActiveModel;
use crate::entities::sales::invoices::{ActiveModel as InvoiceActiveModel, Entity as InvoiceEntity};
use crate::shared::activity;
use crate::shared::currency::{convert_lines_to_base, is_base_currency, require_rate, to_base};
use crate::shared::defaults::branch_prefix;
use crate::shared::ledger::accounts::{resolve_account, AccountCtx, SystemRole};
use crate::shared::ledger::period::assert_open_period;
use crate::shared::ledger::post::{self, AccountRef, PartyRef, PostJournal, PostingLine, SourceRef};
use crate::shared::numbering::{self, DocumentKind};
use crate::shared::stock::{self, batches, LockedProduct, StockRef};
use crate::shared::totals::{compute_invoice_totals, InvoiceDiscountInput, InvoiceTotalsResult, TotalsLineInput, TotalsTax};
use crate::utils::dates::{DocDate, RawDocDate};
use crate::utils::id::Id;
use crate::utils::money::{js_number_string, round2};
use crate::utils::route::RouteRef;

use super::super::dto::{Invoice, JournalPreviewLine, SaleInput, Tender};
use super::common::{self, base_qty, invoice_dto, legacy_method_by_type, payment_method_by_id, to_totals_category};
use super::credit;

const METHOD_LABEL_CASH: &str = "نقداً";
const METHOD_LABEL_CARD: &str = "بطاقة";
const METHOD_LABEL_BANK: &str = "تحويل بنكي";
const METHOD_LABEL_CREDIT: &str = "آجل";

fn method_label(m: &crate::entities::sales::invoices::SalePaymentMethod) -> &'static str {
    use crate::entities::sales::invoices::SalePaymentMethod as M;
    match m {
        M::Cash => METHOD_LABEL_CASH,
        M::Card => METHOD_LABEL_CARD,
        M::BankTransfer => METHOD_LABEL_BANK,
        M::Credit => METHOD_LABEL_CREDIT,
    }
}

/// A resolved tender: the input plus its live payment-method row.
struct ResolvedTender {
    payment_method_id: Id,
    amount: Decimal,
    reference: Option<String>,
    account_role: String,
}

/// Everything `prepare_sale` computes, shared by `preview_sale` and `create_sale`.
pub struct PreparedSale {
    pub totals: InvoiceTotalsResult,
    pub tenders: Vec<Tender>,
    pub paid_amount: Decimal,
    pub cost_total: Decimal,
    pub posting: Vec<PostingLine>,
    pub rate: Decimal,
    /// First-appearance order, base-unit qty per product — `create_sale`'s stock-application loop
    /// walks this same order.
    pub qty_by_product: Vec<(Id, Decimal)>,
    pub currency: Option<String>,
    pub branch_id: Id,
    pub cost_center_id: Option<Id>,
    pub receivable_base: Decimal,
    pub line_taxes: Vec<common::LineTax>,
}

/// `prepare_sale(conn, cx, input, products)` (08 §3.2). `products` must contain every non-free-text
/// line's product (locked for create, a plain read for preview).
pub async fn prepare_sale<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    input: &SaleInput,
    products: &BTreeMap<Id, products::Model>,
) -> TxResult<PreparedSale> {
    // 1. Cart must not be empty.
    if input.lines.is_empty() {
        return Err(AppError::validation("السلة فارغة").into());
    }

    // 2. Live `users` row of the actor (D-I6, not the session snapshot).
    let actor_id = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;
    let user = crate::entities::org::users::Entity::find_by_id(actor_id).one(conn).await.map_err(AppError::from)?;
    if let Some(user) = &user {
        if input.discount_rate > user.max_discount && input.manager_approved_by.is_none() {
            return Err(AppError::forbidden(format!("الخصم يتجاوز الحد المسموح لك ({}%)", js_number_string(user.max_discount))).into());
        }
    }

    // 3. discount_rate in [0, 100].
    if input.discount_rate < Decimal::ZERO || input.discount_rate > Decimal::ONE_HUNDRED {
        return Err(AppError::validation("نسبة الخصم غير صحيحة").into());
    }

    // 4. Per line validation + qty_by_product (first-appearance order).
    let mut qty_by_product: Vec<(Id, Decimal)> = Vec::new();
    let mut qty_index: BTreeMap<Id, usize> = BTreeMap::new();

    for line in &input.lines {
        if !(line.qty > Decimal::ZERO) {
            return Err(AppError::validation("الكمية يجب أن تكون أكبر من صفر").into());
        }
        if line.price < Decimal::ZERO {
            return Err(AppError::validation("السعر لا يمكن أن يكون سالباً").into());
        }
        if line.is_free_text.unwrap_or(false) {
            if line.revenue_account_id.is_none() {
                return Err(AppError::validation("السطر النصي الحر يحتاج حساب إيراد").into());
            }
            continue;
        }
        let product_id = line.product_id.parse::<Id>().map_err(|_| AppError::not_found("المنتج غير موجود"))?;
        let product = products.get(&product_id).ok_or_else(|| AppError::not_found("المنتج غير موجود"))?;
        if !product.active {
            return Err(AppError::validation(format!("المنتج \"{}\" غير نشط", product.name)).into());
        }
        if product.r#type == "product" {
            let bq = base_qty(line.qty, line.unit_factor);
            match qty_index.get(&product_id) {
                Some(&i) => qty_by_product[i].1 = round2(qty_by_product[i].1 + bq),
                None => {
                    qty_index.insert(product_id, qty_by_product.len());
                    qty_by_product.push((product_id, bq));
                }
            }
        }
    }

    // 5. Per product (first-appearance order): stock CONFLICT + cost accumulation.
    let mut cost_total = Decimal::ZERO;
    for (product_id, qty) in &qty_by_product {
        let product = products.get(product_id).expect("validated above");
        if *qty > product.stock_qty {
            return Err(AppError::conflict(format!(
                "الكمية المطلوبة من \"{}\" غير متوفرة — المتاح {}",
                product.name,
                js_number_string(product.stock_qty)
            ))
            .into());
        }
        cost_total += crate::shared::stock::cost::cost_out_sale(product.stock_qty, product.stock_value, product.cost_price, *qty);
    }

    // 6. Totals.
    let settings = crate::core::settings::load(conn).await?;
    let prices_include_tax = settings.prices_include_tax;
    let mut line_taxes: Vec<common::LineTax> = Vec::with_capacity(input.lines.len());
    for line in &input.lines {
        line_taxes.push(common::tax_for_line(conn, line.tax_id).await?);
    }
    let invoice_discount = if input.discount_amount.map(|a| a > Decimal::ZERO).unwrap_or(false) {
        Some(InvoiceDiscountInput::Amount(input.discount_amount.unwrap()))
    } else if input.discount_rate > Decimal::ZERO {
        Some(InvoiceDiscountInput::Pct(input.discount_rate))
    } else {
        None
    };
    let totals_lines: Vec<TotalsLineInput> = input
        .lines
        .iter()
        .zip(line_taxes.iter())
        .map(|(l, t)| TotalsLineInput {
            qty: l.qty,
            unit_price: l.price,
            discount: l.discount.unwrap_or(Decimal::ZERO),
            discount_is_pct: l.discount_is_pct.unwrap_or(false),
            tax: Some(TotalsTax { rate: t.rate, category: to_totals_category(t.category) }),
        })
        .collect();
    let totals = compute_invoice_totals(&totals_lines, invoice_discount, prices_include_tax);
    let grand_total = totals.gross;

    // 7. Tenders.
    let mut resolved_tenders: Vec<ResolvedTender> = Vec::new();
    if let Some(tenders) = input.tenders.as_ref().filter(|t| !t.is_empty()) {
        for t in tenders {
            let method = payment_method_by_id(conn, t.payment_method_id).await?;
            resolved_tenders.push(ResolvedTender { payment_method_id: t.payment_method_id, amount: t.amount, reference: t.reference.clone(), account_role: method.account_role });
        }
    } else {
        match input.payment_method {
            crate::domains::invoices::dto::SalePaymentMethod::Card => {
                let method = legacy_method_by_type(conn, "card").await?;
                resolved_tenders.push(ResolvedTender { payment_method_id: method.id, amount: grand_total, reference: None, account_role: method.account_role });
            }
            crate::domains::invoices::dto::SalePaymentMethod::BankTransfer => {
                let method = legacy_method_by_type(conn, "bank_transfer").await?;
                resolved_tenders.push(ResolvedTender { payment_method_id: method.id, amount: grand_total, reference: None, account_role: method.account_role });
            }
            crate::domains::invoices::dto::SalePaymentMethod::Credit => {}
            crate::domains::invoices::dto::SalePaymentMethod::Cash => {
                let method = legacy_method_by_type(conn, "cash").await?;
                let amount = round2(input.paid_amount).min(grand_total);
                resolved_tenders.push(ResolvedTender { payment_method_id: method.id, amount, reference: None, account_role: method.account_role });
            }
        }
    }

    let paid_amount = round2(resolved_tenders.iter().fold(Decimal::ZERO, |a, t| a + t.amount));
    if paid_amount < Decimal::ZERO {
        return Err(AppError::validation("المبلغ المدفوع غير صحيح").into());
    }
    if paid_amount > grand_total + Decimal::new(1, 2) {
        return Err(AppError::validation("مجموع طرق الدفع أكبر من إجمالي الفاتورة").into());
    }

    // 9. Partial/credit sale requires an active customer.
    if paid_amount < grand_total {
        let Some(customer_id) = input.customer_id else {
            return Err(AppError::validation("البيع الآجل أو الدفع الجزئي يتطلب اختيار عميل").into());
        };
        let customer = crate::entities::parties::parties::Entity::find()
            .filter(crate::entities::parties::parties::Column::Id.eq(customer_id))
            .filter(crate::entities::parties::parties::Column::Kind.eq("customer"))
            .one(conn)
            .await
            .map_err(AppError::from)?;
        if !customer.map(|c| c.active).unwrap_or(false) {
            return Err(AppError::validation("العميل غير موجود أو غير نشط").into());
        }
    }

    let cost_total = round2(cost_total);
    let receivable = round2(grand_total - paid_amount);

    // 11. Branch / cost center / currency / rate.
    let branch_id = input.branch_id.unwrap_or(settings.default_branch_id);
    let cost_center_id = crate::shared::defaults::default_cost_center_for(conn, Some(branch_id), input.cost_center_id).await?;
    let is_base = is_base_currency(conn, input.currency.as_deref()).await?;
    let currency = if !is_base { input.currency.clone() } else { None };
    let rate = if let Some(cur) = &currency {
        match input.exchange_rate {
            Some(r) => r,
            None => require_rate(conn, cur, cx.clock.today()).await?,
        }
    } else {
        Decimal::ONE
    };
    let dim = (Some(branch_id), cost_center_id);

    // 12. Tender lines (without branch context — same as posting, per §3.2 step 12 "resolved
    // without branch context, like accountFor(role)").
    let mut tender_lines: Vec<PostingLine> = Vec::new();
    for t in resolved_tenders.iter().filter(|t| t.amount > Decimal::ZERO) {
        let role = SystemRole::from_str(&t.account_role).map_err(crate::core::tx::TxError::App)?;
        let description = match &t.reference {
            Some(r) => Some(format!("{} — {r}", payment_method_by_id(conn, t.payment_method_id).await?.name)),
            None => Some(payment_method_by_id(conn, t.payment_method_id).await?.name),
        };
        let mut l = PostingLine::debit(AccountRef::Role(role), t.amount);
        l.description = description;
        l.branch_id = dim.0;
        l.cost_center_id = dim.1;
        tender_lines.push(l);
    }

    // 13. stocked_net / free_text_revenue.
    let mut stocked_net = Decimal::ZERO;
    let mut free_text_revenue: Vec<(Id, Decimal)> = Vec::new();
    for (i, line) in input.lines.iter().enumerate() {
        let lr = &totals.lines[i];
        if line.is_free_text.unwrap_or(false) {
            if let Some(account_id) = line.revenue_account_id {
                match free_text_revenue.iter_mut().find(|(a, _)| *a == account_id) {
                    Some((_, v)) => *v = round2(*v + lr.net),
                    None => free_text_revenue.push((account_id, lr.net)),
                }
            }
        } else {
            stocked_net = round2(stocked_net + lr.net);
        }
    }

    let credit_fc: Vec<Decimal> = std::iter::once(stocked_net).chain(free_text_revenue.iter().map(|(_, v)| *v)).chain(std::iter::once(totals.vat)).collect();
    let credit_base = if currency.is_some() { convert_lines_to_base(&credit_fc, rate) } else { credit_fc.clone() };
    let stocked_net_base = credit_base[0];
    let free_text_base: Vec<(Id, Decimal)> = free_text_revenue.iter().enumerate().map(|(i, (acc, _))| (*acc, credit_base[1 + i])).collect();
    let vat_base = *credit_base.last().unwrap();

    let receivable_base = if currency.is_some() { to_base(receivable, rate) } else { receivable };

    // 14. Posting lines.
    let mut posting: Vec<PostingLine> = Vec::new();
    posting.extend(tender_lines);
    {
        let mut l = PostingLine::debit(AccountRef::Role(SystemRole::Receivable), receivable_base);
        l.party = input.customer_id.map(|id| PartyRef { kind: crate::entities::journal::journal_lines::PartyKind::Customer, id });
        l.branch_id = dim.0;
        l.cost_center_id = dim.1;
        if let Some(cur) = &currency {
            l.currency = Some(cur.clone());
            l.amount_fc = Some(receivable);
            l.rate = Some(rate);
        }
        posting.push(l);
    }
    {
        let mut l = PostingLine::credit(AccountRef::Role(SystemRole::Sales), stocked_net_base);
        l.branch_id = dim.0;
        l.cost_center_id = dim.1;
        posting.push(l);
    }
    for (account_id, amount) in &free_text_base {
        let mut l = PostingLine::credit(AccountRef::Id(*account_id), *amount);
        l.branch_id = dim.0;
        l.cost_center_id = dim.1;
        posting.push(l);
    }
    {
        let mut l = PostingLine::credit(AccountRef::Role(SystemRole::VatOutput), vat_base);
        l.branch_id = dim.0;
        l.cost_center_id = dim.1;
        posting.push(l);
    }
    {
        let mut l = PostingLine::debit(AccountRef::Role(SystemRole::Cogs), cost_total);
        l.branch_id = dim.0;
        l.cost_center_id = dim.1;
        posting.push(l);
    }
    {
        let mut l = PostingLine::credit(AccountRef::Role(SystemRole::Inventory), cost_total);
        l.branch_id = dim.0;
        l.cost_center_id = dim.1;
        posting.push(l);
    }

    let tenders_dto: Vec<Tender> = resolved_tenders.iter().map(|t| Tender { payment_method_id: t.payment_method_id, amount: round2(t.amount), reference: t.reference.clone() }).collect();

    Ok(PreparedSale {
        totals,
        tenders: tenders_dto,
        paid_amount,
        cost_total,
        posting,
        rate,
        qty_by_product,
        currency,
        branch_id,
        cost_center_id,
        receivable_base,
        line_taxes,
    })
}

/// `preview_sale` (08 §3.3): reads named products (plain read, no lock), `prepare_sale`, keeps
/// lines with `debit > 0 || credit > 0`, maps each to an account code/name (previewed **without**
/// branch context, Q-3).
pub async fn preview_sale<C: ConnectionTrait>(conn: &C, cx: &TxCtx, input: SaleInput) -> TxResult<Vec<JournalPreviewLine>> {
    let ids = product_ids_of(&input)?;
    let rows = if ids.is_empty() { Vec::new() } else { ProductEntity::find().filter(products::Column::Id.is_in(ids)).all(conn).await.map_err(AppError::from)? };
    let products_map: BTreeMap<Id, products::Model> = rows.into_iter().map(|p| (p.id, p)).collect();

    let prepared = prepare_sale(conn, cx, &input, &products_map).await?;
    let mut out = Vec::new();
    for line in prepared.posting.into_iter().filter(|l| l.debit > Decimal::ZERO || l.credit > Decimal::ZERO) {
        let account = match &line.account {
            AccountRef::Id(id) => crate::shared::ledger::accounts::account_by_id(conn, *id).await?,
            AccountRef::Role(role) => resolve_account(conn, *role, &AccountCtx::default()).await?,
        };
        out.push(JournalPreviewLine { account_code: account.code, account_name: account.name, debit: round2(line.debit), credit: round2(line.credit) });
    }
    Ok(out)
}

fn product_ids_of(input: &SaleInput) -> TxResult<Vec<Id>> {
    let mut ids = Vec::new();
    for line in &input.lines {
        if line.is_free_text.unwrap_or(false) {
            continue;
        }
        if let Ok(id) = line.product_id.parse::<Id>() {
            ids.push(id);
        }
    }
    Ok(ids)
}

/// `create_sale(conn, cx, reg, input)` (08 §3.4).
pub async fn create_sale<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::UndoRegistry,
    input: SaleInput,
) -> TxResult<Invoice> {
    // 1. Shift (document row) — locked first.
    let mut locked_shift: Option<crate::entities::sales::shifts::Model> = None;
    if let Some(shift_id) = input.shift_id {
        if let Some(shift) = crate::entities::sales::shifts::Entity::find_by_id(shift_id).one(conn).await.map_err(AppError::from)? {
            locked_shift = super::shifts::lock_open_shift_of_terminal(conn, shift.id).await?;
        }
    }

    // 2. Customer (party row).
    if let Some(customer_id) = input.customer_id {
        lock::for_update_by_id(conn, "parties", &customer_id.to_string()).await.map_err(AppError::from)?;
    }

    // 3. Products.
    let ids = product_ids_of(&input)?;
    let existing_ids: Vec<Id> = if ids.is_empty() {
        Vec::new()
    } else {
        ProductEntity::find().filter(products::Column::Id.is_in(ids)).all(conn).await.map_err(AppError::from)?.into_iter().map(|p| p.id).collect()
    };
    let locked = stock::lock_products(conn, &existing_ids).await?;
    let products_map: BTreeMap<Id, products::Model> = locked.iter().map(|(id, p)| (*id, p.model.clone())).collect();

    // 4. prepare_sale over the locked models.
    let prepared = prepare_sale(conn, cx, &input, &products_map).await?;

    // 5. Credit limit.
    if let Some(customer_id) = input.customer_id {
        let customer = crate::entities::parties::parties::Entity::find()
            .filter(crate::entities::parties::parties::Column::Id.eq(customer_id))
            .filter(crate::entities::parties::parties::Column::Kind.eq("customer"))
            .one(conn)
            .await
            .map_err(AppError::from)?;
        if let Some(customer) = &customer {
            if customer.credit_limit.map(|l| l > Decimal::ZERO).unwrap_or(false) {
                let current = crate::shared::balances::customer_balance(conn, customer_id).await?;
                let actor = cx.actor.as_ref().expect("checked in prepare_sale");
                let can_override = credit::role_can_override_credit_limit(conn, actor.role).await?;
                credit::assert_within_credit_limit(&customer.name, customer.credit_limit, current, prepared.receivable_base, can_override)?;
            }
        }
    }

    // 6. Date + open-period check before numbering.
    let date = DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) };
    assert_open_period(conn, &date.day, false).await?;

    // 7. Number.
    let prefix = branch_prefix(conn, Some(prepared.branch_id)).await?;
    let number = format!("{prefix}{}", numbering::next_number(conn, DocumentKind::Invoice).await?);

    // 8. Due date.
    let mut due_date: Option<DocDate> = None;
    if let Some(customer_id) = input.customer_id {
        let outstanding_now = crate::shared::totals::document_outstanding(prepared.totals.gross, prepared.paid_amount);
        if outstanding_now > Decimal::ZERO {
            due_date = match &input.due_date_override {
                Some(raw) => Some(RawDocDate::parse(raw).map_err(|_| AppError::validation("تاريخ غير صالح"))?.resolve(&cx.clock)),
                None => {
                    let customer = crate::entities::parties::parties::Entity::find_by_id(customer_id).one(conn).await.map_err(AppError::from)?;
                    credit::compute_due_date(cx.clock.now, cx.clock.tz, customer.and_then(|c| c.payment_terms_days))
                }
            };
        }
    }

    // 9. Insert invoice.
    let id = Id::new();
    let now = cx.clock.now;
    let invoice_am = InvoiceActiveModel {
        id: Set(id),
        number: Set(number.clone()),
        date_day: Set(date.day),
        date_instant: Set(date.instant),
        customer_id: Set(input.customer_id),
        cashier_id: Set(cx.actor.as_ref().expect("checked").id),
        status: Set(crate::entities::sales::invoices::InvoiceStatus::Completed),
        payment_status: Set(common::payment_status_kind_to_entity(crate::shared::totals::payment_status_for(prepared.totals.gross, prepared.paid_amount))),
        sub_total: Set(prepared.totals.sub_total_after_line_discounts),
        discount_rate: Set(input.discount_rate),
        discount_amount: Set(prepared.totals.invoice_discount_amount),
        tax_rate: Set(prepared.line_taxes.first().map(|t| t.rate).unwrap_or(Decimal::ZERO)),
        tax_amount: Set(prepared.totals.vat),
        grand_total: Set(prepared.totals.gross),
        payment_method: Set(common::payment_method_dto_to_entity(input.payment_method)),
        paid_amount: Set(prepared.paid_amount),
        refunded_amount: Set(Decimal::ZERO),
        tendered_amount: Set(if matches!(input.payment_method, crate::domains::invoices::dto::SalePaymentMethod::Cash) { input.tendered_amount } else { None }),
        due_date_day: Set(due_date.as_ref().map(|d| d.day)),
        due_date_instant: Set(due_date.as_ref().and_then(|d| d.instant)),
        note: Set(input.note.clone()),
        source: Set(Some(common::source_dto_to_entity(input.source.unwrap_or(crate::domains::invoices::dto::InvoiceSource::Pos)))),
        shift_id: Set(input.shift_id),
        branch_id: Set(Some(prepared.branch_id)),
        invoice_type: Set(input.invoice_type.map(common::invoice_type_dto_to_entity)),
        po_reference: Set(input.po_reference.clone()),
        terms: Set(input.terms.clone()),
        attachment_ids: Set(input.attachment_ids.clone().map(crate::entities::values::StringList)),
        currency: Set(prepared.currency.clone()),
        exchange_rate: Set(prepared.currency.as_ref().map(|_| prepared.rate)),
        cost_center_id: Set(prepared.cost_center_id),
        search_normalized: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(crate::entities::sales::invoices::SyncStatus::Local),
    };
    invoice_am.insert(conn).await.map_err(AppError::from)?;

    for (i, line) in input.lines.iter().enumerate() {
        let lr = &prepared.totals.lines[i];
        let tax = &prepared.line_taxes[i];
        let is_free_text = line.is_free_text.unwrap_or(false);
        let (product_id, name, cost_price, unit_id, unit_factor, list_price, price_override_reason, batch_id, batch_no, revenue_account_id) = if is_free_text {
            (None, line.name.clone().unwrap_or_else(|| "سطر حر".to_string()), Decimal::ZERO, None, None, None, None, None, None, line.revenue_account_id)
        } else {
            let pid = line.product_id.parse::<Id>().expect("validated in prepare_sale");
            let product = products_map.get(&pid).expect("validated in prepare_sale");
            (
                Some(pid),
                product.name.clone(),
                product.cost_price,
                line.unit_id,
                line.unit_factor,
                line.list_price,
                line.price_override_reason.clone(),
                line.batch_id,
                line.batch_no.clone(),
                None,
            )
        };
        let line_am = LineActiveModel {
            id: Set(Id::new()),
            invoice_id: Set(id),
            position: Set(i as i16),
            product_id: Set(product_id),
            name: Set(name),
            qty: Set(line.qty),
            price: Set(line.price),
            cost_price: Set(cost_price),
            discount: Set(line.discount.unwrap_or(Decimal::ZERO)),
            tax_id: Set(tax.id),
            tax_category: Set(Some(from_category_str(tax.category))),
            tax_rate: Set(Some(tax.rate)),
            net: Set(Some(lr.net)),
            vat: Set(Some(lr.vat)),
            unit_id: Set(unit_id),
            unit_factor: Set(unit_factor),
            list_price: Set(list_price),
            price_override_reason: Set(price_override_reason),
            batch_id: Set(batch_id),
            batch_no: Set(batch_no),
            is_free_text: Set(is_free_text),
            revenue_account_id: Set(revenue_account_id),
        };
        line_am.insert(conn).await.map_err(AppError::from)?;
    }

    for (i, t) in prepared.tenders.iter().enumerate() {
        let tender_am = TenderActiveModel {
            id: Set(Id::new()),
            invoice_id: Set(id),
            position: Set(i as i16),
            payment_method_id: Set(t.payment_method_id),
            amount: Set(round2(t.amount)),
            reference: Set(t.reference.clone()),
        };
        tender_am.insert(conn).await.map_err(AppError::from)?;
    }

    // 10. Stock.
    let mut locked = locked;
    for (product_id, qty) in &prepared.qty_by_product {
        let product = locked.get(product_id).expect("locked above");
        let value_out = crate::shared::stock::cost::cost_out_sale(product.stock_qty(), product.stock_value(), product.cost_price(), *qty);
        let track_batches = product.track_batches();
        {
            let p: &mut LockedProduct = locked.get_mut(product_id).expect("locked above");
            stock::apply_change(conn, cx, p, -*qty, -value_out, "sale", StockRef { id, number: number.clone() }, &date, Some(prepared.branch_id)).await?;
        }
        if track_batches {
            let mut remaining = *qty;
            for line in input.lines.iter().filter(|l| !l.is_free_text.unwrap_or(false) && l.product_id.parse::<Id>().ok() == Some(*product_id) && l.batch_id.is_some()) {
                let take = batches::draw_batch(conn, cx, *product_id, line.batch_id.unwrap(), base_qty(line.qty, line.unit_factor)).await?;
                remaining = round2(remaining - take);
            }
            if remaining > Decimal::new(1, 4) {
                batches::consume_fefo(conn, cx, *product_id, remaining, false, cx.clock.today()).await?;
            }
        }
    }

    // 11. Post.
    let currency_suffix = prepared.currency.as_ref().map(|c| format!(" — {c}")).unwrap_or_default();
    post::post(
        conn,
        cx,
        PostJournal {
            date,
            description: format!("فاتورة مبيعات {number} ({}){currency_suffix}", method_label(&common::payment_method_dto_to_entity(input.payment_method))),
            entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
            source: Some(SourceRef { kind: "invoice".to_string(), id, number: Some(number.clone()) }),
            lines: prepared.posting,
            allow_closed_period: false,
            attachment_ids: Vec::new(),
            template_id: None,
        },
    )
    .await?;

    // 12. Drawer movement.
    let cash_method_ids: std::collections::HashSet<Id> = crate::entities::org::payment_methods::Entity::find()
        .filter(crate::entities::org::payment_methods::Column::AccountRole.eq("cash"))
        .all(conn)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .map(|m| m.id)
        .collect();
    let cash_tendered = round2(prepared.tenders.iter().filter(|t| cash_method_ids.contains(&t.payment_method_id)).fold(Decimal::ZERO, |a, t| a + t.amount));
    if input.shift_id.is_some() && cash_tendered > Decimal::ZERO {
        if let Some(shift) = &locked_shift {
            super::shifts::record_shift_movement(conn, cx, shift, crate::domains::invoices::dto::ShiftMovementKind::SaleCash, cash_tendered, None, Some(id), Some(number.clone()), date).await?;
        }
    }

    // 13. Activity.
    let customer_suffix = match input.customer_id {
        Some(customer_id) => crate::entities::parties::parties::Entity::find_by_id(customer_id)
            .one(conn)
            .await
            .map_err(AppError::from)?
            .map(|c| format!(" — {}", c.name))
            .unwrap_or_default(),
        None => String::new(),
    };
    activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Sale,
        format!("فاتورة {number} بقيمة {:.2}{customer_suffix}", round2(prepared.totals.gross)),
        Some(date),
        Some(RouteRef::detail("invoice", id.to_string())),
    )
    .await?;

    // 14.
    if input.customer_id.is_some() {
        cx.touch(crate::core::events::ChangeCategory::Parties);
    }

    let model = InvoiceEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.expect("just inserted");
    invoice_dto(conn, &model).await
}

fn from_category_str(c: crate::domains::settings::dto::TaxCategory) -> String {
    match c {
        crate::domains::settings::dto::TaxCategory::S => "S".to_string(),
        crate::domains::settings::dto::TaxCategory::Z => "Z".to_string(),
        crate::domains::settings::dto::TaxCategory::E => "E".to_string(),
        crate::domains::settings::dto::TaxCategory::O => "O".to_string(),
    }
}
