//! `domains::invoices::service::quotations` — 08 §3.7: `save_quotation`, `set_quotation_status`,
//! `convert_quotation_to_invoice`.

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::sales::quotation_lines::{ActiveModel as QLineActiveModel, Column as QLineColumn, Entity as QLineEntity, Model as QLineModel};
use crate::entities::sales::quotations::{ActiveModel as QuotationActiveModel, Entity as QuotationEntity, Model as QuotationModel, QuotationStatus as EntityQuotationStatus};
use crate::shared::activity;
use crate::shared::numbering::{self, DocumentKind};
use crate::shared::totals::{compute_invoice_totals, InvoiceDiscountInput, TotalsLineInput, TotalsTax};
use crate::utils::dates::{DocDate, RawDocDate};
use crate::utils::id::Id;

use super::super::dto::{Invoice, Quotation, QuotationInput, QuotationStatus, SaleInputLine};
use super::common::to_totals_category;

pub fn status_dto_to_entity(s: QuotationStatus) -> EntityQuotationStatus {
    match s {
        QuotationStatus::Draft => EntityQuotationStatus::Draft,
        QuotationStatus::Sent => EntityQuotationStatus::Sent,
        QuotationStatus::Accepted => EntityQuotationStatus::Accepted,
        QuotationStatus::Rejected => EntityQuotationStatus::Rejected,
        QuotationStatus::Expired => EntityQuotationStatus::Expired,
    }
}

pub fn status_to_dto(s: &EntityQuotationStatus) -> QuotationStatus {
    match s {
        EntityQuotationStatus::Draft => QuotationStatus::Draft,
        EntityQuotationStatus::Sent => QuotationStatus::Sent,
        EntityQuotationStatus::Accepted => QuotationStatus::Accepted,
        EntityQuotationStatus::Rejected => QuotationStatus::Rejected,
        EntityQuotationStatus::Expired => QuotationStatus::Expired,
    }
}

fn q_line_to_dto(l: &QLineModel) -> super::super::dto::InvoiceLine {
    let product_id = match l.product_id {
        Some(id) => id.to_string(),
        None => "freetext".to_string(),
    };
    super::super::dto::InvoiceLine {
        id: format!("{}-l{}", l.quotation_id, l.position + 1),
        product_id,
        name: l.name.clone(),
        qty: l.qty,
        price: l.price,
        cost_price: l.cost_price,
        discount: l.discount,
        tax_id: l.tax_id,
        tax_category: l.tax_category.as_deref().map(parse_category),
        tax_rate: l.tax_rate,
        net: l.net,
        vat: l.vat,
        unit_id: l.unit_id,
        unit_factor: l.unit_factor,
        list_price: l.list_price,
        price_override_reason: l.price_override_reason.clone(),
        batch_id: l.batch_id,
        batch_no: l.batch_no.clone(),
        is_free_text: if l.is_free_text { Some(true) } else { None },
        revenue_account_id: l.revenue_account_id,
    }
}

fn parse_category(raw: &str) -> crate::domains::settings::dto::TaxCategory {
    use crate::domains::settings::dto::TaxCategory as C;
    match raw {
        "S" => C::S,
        "Z" => C::Z,
        "E" => C::E,
        _ => C::O,
    }
}

fn from_category_str(c: crate::domains::settings::dto::TaxCategory) -> String {
    use crate::domains::settings::dto::TaxCategory as C;
    match c {
        C::S => "S".to_string(),
        C::Z => "Z".to_string(),
        C::E => "E".to_string(),
        C::O => "O".to_string(),
    }
}

pub fn quotation_to_dto(model: &QuotationModel, lines: &[QLineModel]) -> Quotation {
    Quotation {
        id: model.id,
        number: model.number.clone(),
        date: model.date().key(),
        expiry_date: model.expiry_date().map(|d| d.key()),
        customer_id: model.customer_id,
        salesperson_id: model.salesperson_id,
        status: status_to_dto(&model.status),
        lines: lines.iter().map(q_line_to_dto).collect(),
        discount_rate: model.discount_rate,
        discount_amount: model.discount_amount,
        tax_amount: model.tax_amount,
        sub_total: model.sub_total,
        grand_total: model.grand_total,
        note: model.note.clone(),
        terms: model.terms.clone(),
        po_reference: model.po_reference.clone(),
        attachment_ids: model.attachment_ids.as_ref().map(|l| l.0.clone()),
        converted_invoice_id: model.converted_invoice_id,
    }
}

/// Quotation's own tax fallback (Q-6, `sales.ts` differs from invoices): live active tax by id ->
/// live active `settings.default_tax_id` -> `{ rate: 0, category: 'O' }`.
async fn quotation_tax_for_line<C: ConnectionTrait>(conn: &C, tax_id: Option<Id>) -> TxResult<(Decimal, crate::domains::settings::dto::TaxCategory, Option<Id>)> {
    use crate::entities::org::taxes::{Column as TaxColumn, Entity as TaxEntity};
    use crate::entities::soft_delete::SoftDelete;
    if let Some(id) = tax_id {
        if let Some(tax) = TaxEntity::find_live().filter(TaxColumn::Id.eq(id)).filter(TaxColumn::Active.eq(true)).one(conn).await.map_err(AppError::from)? {
            return Ok((tax.rate, parse_category(&tax.category), Some(tax.id)));
        }
    }
    let settings = crate::core::settings::load(conn).await?;
    if let Some(default_id) = settings.default_tax_id {
        if let Some(tax) = TaxEntity::find_live().filter(TaxColumn::Id.eq(default_id)).filter(TaxColumn::Active.eq(true)).one(conn).await.map_err(AppError::from)? {
            return Ok((tax.rate, parse_category(&tax.category), Some(tax.id)));
        }
    }
    Ok((Decimal::ZERO, crate::domains::settings::dto::TaxCategory::O, None))
}

/// **`save_quotation`** (`invoiceService.ts:384-444`).
pub async fn save_quotation<C: ConnectionTrait>(conn: &C, cx: &TxCtx, input: QuotationInput) -> TxResult<Quotation> {
    if input.lines.is_empty() {
        return Err(AppError::validation("أضف صنفاً واحداً على الأقل").into());
    }
    let settings = crate::core::settings::load(conn).await?;
    let prices_include_tax = settings.prices_include_tax;

    let mut line_taxes = Vec::with_capacity(input.lines.len());
    for l in &input.lines {
        line_taxes.push(quotation_tax_for_line(conn, l.tax_id).await?);
    }
    let invoice_discount = if input.discount_rate > Decimal::ZERO { Some(InvoiceDiscountInput::Pct(input.discount_rate)) } else { None };
    let totals_lines: Vec<TotalsLineInput> = input
        .lines
        .iter()
        .zip(line_taxes.iter())
        .map(|(l, (rate, category, _))| TotalsLineInput {
            qty: l.qty,
            unit_price: l.price,
            discount: l.discount.unwrap_or(Decimal::ZERO),
            discount_is_pct: l.discount_is_pct.unwrap_or(false),
            tax: Some(TotalsTax { rate: *rate, category: to_totals_category(*category) }),
        })
        .collect();
    let totals = compute_invoice_totals(&totals_lines, invoice_discount, prices_include_tax);

    let number = numbering::next_number(conn, DocumentKind::Quotation).await?;
    let now = cx.clock.now;
    let salesperson_id = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;
    let expiry: Option<DocDate> = match &input.expiry_date {
        Some(raw) => Some(RawDocDate::parse(raw).map_err(|_| AppError::validation("تاريخ غير صالح"))?.resolve(&cx.clock)),
        None => None,
    };
    let id = Id::new();

    // Product lookups for `name`/`costPrice` fallbacks (products only; free-text lines skip this).
    let product_ids: Vec<Id> = input.lines.iter().filter_map(|l| l.product_id.parse::<Id>().ok()).collect();
    let products: std::collections::BTreeMap<Id, crate::entities::catalog::products::Model> = if product_ids.is_empty() {
        std::collections::BTreeMap::new()
    } else {
        crate::entities::catalog::products::Entity::find()
            .filter(crate::entities::catalog::products::Column::Id.is_in(product_ids))
            .all(conn)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(|p| (p.id, p))
            .collect()
    };

    let am = QuotationActiveModel {
        id: Set(id),
        number: Set(number.clone()),
        date_day: Set(cx.clock.today()),
        date_instant: Set(Some(now)),
        expiry_date_day: Set(expiry.as_ref().map(|d| d.day)),
        expiry_date_instant: Set(expiry.as_ref().and_then(|d| d.instant)),
        customer_id: Set(input.customer_id),
        salesperson_id: Set(salesperson_id),
        status: Set(EntityQuotationStatus::Draft),
        discount_rate: Set(input.discount_rate),
        discount_amount: Set(totals.invoice_discount_amount),
        tax_amount: Set(totals.vat),
        sub_total: Set(totals.sub_total_after_line_discounts),
        grand_total: Set(totals.gross),
        note: Set(input.note.clone()),
        terms: Set(input.terms.clone()),
        po_reference: Set(input.po_reference.clone()),
        attachment_ids: Set(None),
        converted_invoice_id: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(crate::entities::sales::quotations::SyncStatus::Local),
    };
    am.insert(conn).await.map_err(AppError::from)?;

    for (i, l) in input.lines.iter().enumerate() {
        let lr = &totals.lines[i];
        let (rate, category, tax_id) = line_taxes[i];
        let is_free_text = l.product_id == "freetext" || l.is_free_text.unwrap_or(false);
        let (product_id, name, cost_price) = if is_free_text {
            (None, l.name.clone().unwrap_or_else(|| "—".to_string()), Decimal::ZERO)
        } else {
            let pid = l.product_id.parse::<Id>().ok();
            let product = pid.and_then(|id| products.get(&id));
            (pid, l.name.clone().or_else(|| product.map(|p| p.name.clone())).unwrap_or_else(|| "—".to_string()), product.map(|p| p.cost_price).unwrap_or(Decimal::ZERO))
        };
        let am = QLineActiveModel {
            id: Set(Id::new()),
            quotation_id: Set(id),
            position: Set(i as i16),
            product_id: Set(product_id),
            name: Set(name),
            qty: Set(l.qty),
            price: Set(l.price),
            cost_price: Set(cost_price),
            discount: Set(l.discount.unwrap_or(Decimal::ZERO)),
            tax_id: Set(tax_id),
            tax_category: Set(Some(from_category_str(category))),
            tax_rate: Set(Some(rate)),
            net: Set(Some(lr.net)),
            vat: Set(Some(lr.vat)),
            unit_id: Set(None),
            unit_factor: Set(None),
            list_price: Set(None),
            price_override_reason: Set(None),
            batch_id: Set(None),
            batch_no: Set(None),
            is_free_text: Set(false),
            revenue_account_id: Set(None),
        };
        am.insert(conn).await.map_err(AppError::from)?;
    }

    let lines = QLineEntity::find().filter(QLineColumn::QuotationId.eq(id)).order_by_asc(QLineColumn::Position).all(conn).await.map_err(AppError::from)?;
    let model = QuotationEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.expect("just inserted");
    Ok(quotation_to_dto(&model, &lines))
}

/// **`set_quotation_status`** (`invoiceService.ts:446-452`).
pub async fn set_quotation_status<C: ConnectionTrait>(conn: &C, id: Id, status: QuotationStatus) -> TxResult<Quotation> {
    lock::for_update_by_id(conn, "quotations", &id.to_string()).await.map_err(AppError::from)?;
    let model = QuotationEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("عرض السعر غير موجود"))?;
    let mut am: QuotationActiveModel = model.clone().into();
    am.status = Set(status_dto_to_entity(status));
    let updated = am.update(conn).await.map_err(AppError::from)?;
    let lines = QLineEntity::find().filter(QLineColumn::QuotationId.eq(id)).order_by_asc(QLineColumn::Position).all(conn).await.map_err(AppError::from)?;
    Ok(quotation_to_dto(&updated, &lines))
}

/// **`convert_quotation_to_invoice`** (`invoiceService.ts:455-475`).
pub async fn convert_quotation_to_invoice<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::UndoRegistry,
    id: Id,
    payment: super::super::dto::ConvertPayment,
) -> TxResult<Invoice> {
    lock::for_update_by_id(conn, "quotations", &id.to_string()).await.map_err(AppError::from)?;
    let model = QuotationEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("عرض السعر غير موجود"))?;
    if model.converted_invoice_id.is_some() {
        return Err(AppError::conflict("تم تحويل عرض السعر إلى فاتورة بالفعل").into());
    }

    let lines = QLineEntity::find().filter(QLineColumn::QuotationId.eq(id)).order_by_asc(QLineColumn::Position).all(conn).await.map_err(AppError::from)?;
    let sale_lines: Vec<SaleInputLine> = lines
        .iter()
        .map(|l| SaleInputLine {
            product_id: match l.product_id {
                Some(pid) => pid.to_string(),
                None => "freetext".to_string(),
            },
            qty: l.qty,
            price: l.price,
            discount: Some(l.discount),
            discount_is_pct: None,
            tax_id: l.tax_id,
            unit_id: None,
            unit_factor: None,
            list_price: None,
            price_override_reason: None,
            batch_id: None,
            batch_no: None,
            is_free_text: None,
            revenue_account_id: None,
            name: None,
        })
        .collect();

    let sale_input = super::super::dto::SaleInput {
        customer_id: model.customer_id,
        lines: sale_lines,
        discount_rate: model.discount_rate,
        discount_amount: None,
        payment_method: payment.payment_method,
        paid_amount: payment.paid_amount,
        tendered_amount: payment.tendered_amount,
        tenders: None,
        note: model.note.clone(),
        source: Some(super::super::dto::InvoiceSource::Desk),
        shift_id: None,
        invoice_type: None,
        due_date_override: None,
        po_reference: model.po_reference.clone(),
        terms: model.terms.clone(),
        attachment_ids: None,
        manager_approved_by: None,
        branch_id: None,
        cost_center_id: None,
        currency: None,
        exchange_rate: None,
    };

    let invoice = super::sale::create_sale(conn, cx, registry, sale_input).await?;

    let mut am: QuotationActiveModel = model.into();
    am.status = Set(EntityQuotationStatus::Accepted);
    am.converted_invoice_id = Set(Some(invoice.id));
    am.update(conn).await.map_err(AppError::from)?;

    Ok(invoice)
}
