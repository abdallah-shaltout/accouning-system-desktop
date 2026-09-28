//! `domains::invoices::service::reads` — 08 §3.6: `get_invoices`/`get_invoices_paged`,
//! `get_invoice`, `get_refund`, `get_invoice_print_data`, quotation reads.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::dto::{PagedQuery, PagedResult};
use crate::core::error::AppError;
use crate::core::tx::{ReadCtx, TxResult};
use crate::entities::sales::invoices::{Column as InvoiceColumn, Entity as InvoiceEntity, Model as InvoiceModel};
use crate::entities::sales::quotations::{Column as QuotationColumn, Entity as QuotationEntity};
use crate::utils::dates::in_date_range;
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::text::matches_search;

use super::super::dto::{
    InvoiceDetail, InvoiceListFilter, InvoiceRow, JournalRef, PrintData, QuotationFilter, QuotationRow, Refund,
};
use super::common::{invoice_dto, invoice_row, invoice_rows};

fn apply_invoice_filter(mut query: sea_orm::Select<InvoiceEntity>, filter: &InvoiceListFilter) -> sea_orm::Select<InvoiceEntity> {
    if let Some(status) = filter.status {
        query = query.filter(InvoiceColumn::Status.eq(super::common::entity_status(status)));
    }
    if let Some(ps) = filter.payment_status {
        query = query.filter(InvoiceColumn::PaymentStatus.eq(super::common::entity_payment_status(ps)));
    }
    if let Some(customer_id) = filter.customer_id {
        query = query.filter(InvoiceColumn::CustomerId.eq(customer_id));
    }
    if let Some(source) = filter.source {
        query = query.filter(InvoiceColumn::Source.eq(super::common::source_dto_to_entity(source)));
    }
    if let Some(invoice_type) = filter.invoice_type {
        query = query.filter(InvoiceColumn::InvoiceType.eq(super::common::invoice_type_dto_to_entity(invoice_type)));
    }
    if let Some(cashier_id) = filter.cashier_id {
        query = query.filter(InvoiceColumn::CashierId.eq(cashier_id));
    }
    if let Some(min) = filter.min_amount {
        query = query.filter(InvoiceColumn::GrandTotal.gte(min));
    }
    if let Some(max) = filter.max_amount {
        query = query.filter(InvoiceColumn::GrandTotal.lte(max));
    }
    query.filter(InvoiceColumn::DeletedAt.is_null())
}

async fn rows_for_filter<C: ConnectionTrait>(conn: &C, filter: &InvoiceListFilter, today_key: &str) -> TxResult<Vec<InvoiceModel>> {
    let mut query = apply_invoice_filter(InvoiceEntity::find(), filter);
    if filter.open_only.unwrap_or(false) {
        // `status='COMPLETED' AND grand_total - refunded_amount - paid_amount > 0` — expressed as a
        // raw comparison via `Expr` since sea-query has no first-class "column arithmetic > 0"
        // builder shortcut here; filtered again in Rust below for clarity/safety.
        query = query.filter(InvoiceColumn::Status.eq(crate::entities::sales::invoices::InvoiceStatus::Completed));
    }
    let rows = query.all(conn).await.map_err(AppError::from)?;
    let from = filter.from.as_deref().filter(|s| !s.is_empty());
    let to = filter.to.as_deref().filter(|s| !s.is_empty());
    let mut out: Vec<InvoiceModel> = rows.into_iter().filter(|i| in_date_range(i.date_day, from, to)).collect();
    if filter.open_only.unwrap_or(false) {
        out.retain(|i| round2(i.grand_total - i.refunded_amount - i.paid_amount) > Decimal::ZERO);
    }
    if filter.overdue_only.unwrap_or(false) {
        out.retain(|i| {
            if matches!(i.status, crate::entities::sales::invoices::InvoiceStatus::Refunded) {
                return false;
            }
            let Some(due) = i.due_date() else { return false };
            let outstanding = crate::shared::totals::document_outstanding(round2(i.grand_total - i.refunded_amount), i.paid_amount);
            outstanding > Decimal::ZERO && due.key().as_str() < today_key
        });
    }
    Ok(out)
}

/// **`get_invoices(filter)`** (`invoiceService.ts:62-116`): default order `date_key DESC,
/// created_at, id`; `search` matches `[number, customer_name]` in Rust after SQL narrows.
pub async fn get_invoices<C: ConnectionTrait>(conn: &C, ctx: &ReadCtx, filter: Option<InvoiceListFilter>) -> TxResult<Vec<InvoiceRow>> {
    let filter = filter.unwrap_or_default();
    let today_key = crate::utils::dates::format_iso_ms(ctx.clock.now);
    let mut rows = rows_for_filter(conn, &filter, &today_key).await?;
    rows.sort_by(|a, b| (b.date().key(), b.created_at, b.id).cmp(&(a.date().key(), a.created_at, a.id)));
    let dtos = invoice_rows(conn, &rows).await?;
    Ok(dtos
        .into_iter()
        .filter(|r| matches_search(&[Some(r.invoice.number.as_str()), r.customer_name.as_deref()], filter.search.as_deref()))
        .collect())
}

/// **`get_invoices_paged(query)`** (`invoiceService.ts:88-116`): totals over the filtered, unpaged
/// set; server-side sort/paging via a whitelist of `InvoiceRow` fields.
pub async fn get_invoices_paged<C: ConnectionTrait>(conn: &C, ctx: &ReadCtx, query: PagedQuery<InvoiceListFilter>) -> TxResult<PagedResult<InvoiceRow>> {
    let filter = query.filters.clone().unwrap_or_default();
    let today_key = crate::utils::dates::format_iso_ms(ctx.clock.now);
    let rows = rows_for_filter(conn, &filter, &today_key).await?;
    let dtos = invoice_rows(conn, &rows).await?;
    let mut filtered: Vec<InvoiceRow> = dtos.into_iter().filter(|r| matches_search(&[Some(r.invoice.number.as_str()), r.customer_name.as_deref()], filter.search.as_deref())).collect();

    let total = filtered.len() as u32;
    let mut totals: BTreeMap<String, Decimal> = BTreeMap::new();
    totals.insert("grandTotal".to_string(), round2(filtered.iter().fold(Decimal::ZERO, |a, r| a + r.invoice.grand_total - r.invoice.refunded_amount)));
    totals.insert("outstanding".to_string(), round2(filtered.iter().fold(Decimal::ZERO, |a, r| a + r.outstanding)));

    if let Some(sort) = &query.sort {
        let dir = if matches!(sort.dir, crate::core::dto::SortDir::Asc) { 1i32 } else { -1i32 };
        let key = sort.key.clone();
        filtered.sort_by(|a, b| sort_cmp(a, b, &key).map(|o| if dir < 0 { o.reverse() } else { o }).unwrap_or(std::cmp::Ordering::Equal));
    } else {
        filtered.sort_by(|a, b| b.invoice.date.cmp(&a.invoice.date));
    }

    let page = query.page.max(1) as usize;
    let page_size = query.page_size.max(1) as usize;
    let start = (page - 1) * page_size;
    let page_rows: Vec<InvoiceRow> = filtered.into_iter().skip(start).take(page_size).collect();
    Ok(PagedResult { rows: page_rows, total, totals: Some(totals) })
}

fn sort_cmp(a: &InvoiceRow, b: &InvoiceRow, key: &str) -> Option<std::cmp::Ordering> {
    match key {
        "number" => Some(a.invoice.number.cmp(&b.invoice.number)),
        "date" => Some(a.invoice.date.cmp(&b.invoice.date)),
        "grandTotal" => a.invoice.grand_total.partial_cmp(&b.invoice.grand_total),
        "paidAmount" => a.invoice.paid_amount.partial_cmp(&b.invoice.paid_amount),
        "outstanding" => a.outstanding.partial_cmp(&b.outstanding),
        "customerName" => Some(a.customer_name.as_deref().unwrap_or("").cmp(b.customer_name.as_deref().unwrap_or(""))),
        "cashierName" => Some(a.cashier_name.cmp(&b.cashier_name)),
        _ => None,
    }
}

/// **`get_invoice(id)`** (`invoiceService.ts:118-135`).
pub async fn get_invoice<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<InvoiceDetail> {
    let model = InvoiceEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("الفاتورة غير موجودة"))?;
    let row = invoice_row(conn, &model).await?;

    let refunds_models = crate::entities::sales::refunds::Entity::find()
        .filter(crate::entities::sales::refunds::Column::InvoiceId.eq(id))
        .order_by_asc(crate::entities::sales::refunds::Column::CreatedAt)
        .order_by_asc(crate::entities::sales::refunds::Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let mut refunds: Vec<Refund> = Vec::with_capacity(refunds_models.len());
    for r in &refunds_models {
        let lines = crate::entities::sales::refund_lines::Entity::find()
            .filter(crate::entities::sales::refund_lines::Column::RefundId.eq(r.id))
            .order_by_asc(crate::entities::sales::refund_lines::Column::Position)
            .all(conn)
            .await
            .map_err(AppError::from)?;
        refunds.push(super::refund::refund_to_dto(r, &lines));
    }

    let payments = crate::domains::payments::service::read::payments_for_invoice(conn, id).await?;

    let mut source_ids: Vec<Id> = vec![id];
    source_ids.extend(refunds_models.iter().map(|r| r.id));
    source_ids.extend(payments.iter().map(|p| p.id));
    let journal_entries_models = crate::entities::journal::journal_entries::Entity::find()
        .filter(crate::entities::journal::journal_entries::Column::SourceId.is_in(source_ids))
        .order_by_asc(crate::entities::journal::journal_entries::Column::CreatedAt)
        .order_by_asc(crate::entities::journal::journal_entries::Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let journal_entries: Vec<JournalRef> = journal_entries_models.iter().map(|e| JournalRef { id: e.id, number: e.number.clone(), description: e.description.clone() }).collect();

    let returned_qty_map = super::refund::returned_qty_by_line(conn, id).await?;
    let returned_qty: BTreeMap<String, Decimal> = returned_qty_map.into_iter().map(|(k, v)| (k.to_string(), v)).collect();

    let customer = match model.customer_id {
        Some(cid) => crate::domains::parties::service::read::get_customer(conn, cid).await.ok(),
        None => None,
    };

    Ok(InvoiceDetail { row, customer, refunds, payments, journal_entries, returned_qty })
}

/// **`get_refund(id)`** (`invoiceService.ts:186-191`).
pub async fn get_refund<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<Refund> {
    let model = crate::entities::sales::refunds::Entity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("إشعار الدائن غير موجود"))?;
    let lines = crate::entities::sales::refund_lines::Entity::find()
        .filter(crate::entities::sales::refund_lines::Column::RefundId.eq(id))
        .order_by_asc(crate::entities::sales::refund_lines::Column::Position)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(super::refund::refund_to_dto(&model, &lines))
}

/// **`get_invoice_print_data(id)`** (`invoiceService.ts:203-247`). `id == "sample"` builds the
/// synthetic demo invoice; otherwise a real lookup.
pub async fn get_invoice_print_data<C: ConnectionTrait>(conn: &C, ctx: &ReadCtx, device: &crate::core::device::DeviceSettings, id: &str) -> TxResult<PrintData> {
    let settings_dto = crate::domains::settings::service::store::get_settings(conn, device).await.map_err(crate::core::tx::TxError::App)?;
    if id == "sample" {
        let actor = ctx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?;
        let products = crate::entities::catalog::products::Entity::find()
            .filter(crate::entities::catalog::products::Column::Type.eq("product"))
            .order_by_asc(crate::entities::catalog::products::Column::CreatedAt)
            .order_by_asc(crate::entities::catalog::products::Column::Id)
            .all(conn)
            .await
            .map_err(AppError::from)?;
        let products: Vec<_> = products.into_iter().take(3).collect();
        let lines: Vec<super::super::dto::InvoiceLine> = products
            .iter()
            .enumerate()
            .map(|(i, p)| super::super::dto::InvoiceLine {
                id: format!("s-{i}"),
                product_id: p.id.to_string(),
                name: p.name.clone(),
                qty: Decimal::from(i as i64 + 1),
                price: p.price,
                cost_price: p.cost_price,
                discount: Decimal::ZERO,
                tax_id: None,
                tax_category: None,
                tax_rate: None,
                net: None,
                vat: None,
                unit_id: None,
                unit_factor: None,
                list_price: None,
                price_override_reason: None,
                batch_id: None,
                batch_no: None,
                is_free_text: None,
                revenue_account_id: None,
            })
            .collect();
        let sub_total = round2(lines.iter().fold(Decimal::ZERO, |a, l| a + l.qty * l.price));
        let tax_rate = super::common::sales_tax_rate(conn).await?;
        let tax_amount = round2((sub_total * tax_rate) / Decimal::ONE_HUNDRED);
        let grand_total = sub_total + tax_amount;
        let now_date = crate::utils::dates::DocDate { day: ctx.clock.today(), instant: Some(ctx.clock.now) };
        let tendered = (grand_total / Decimal::ONE_HUNDRED).ceil() * Decimal::ONE_HUNDRED;
        let invoice = super::super::dto::Invoice {
            id: Id::new(),
            number: format!("{}000000", settings_dto.invoice_number_prefix),
            date: now_date.key(),
            customer_id: None,
            cashier_id: actor.id,
            status: super::super::dto::InvoiceStatus::Completed,
            payment_status: super::super::dto::PaymentStatus::Paid,
            lines,
            sub_total,
            discount_rate: Decimal::ZERO,
            discount_amount: Decimal::ZERO,
            tax_rate,
            tax_amount,
            grand_total,
            payment_method: super::super::dto::SalePaymentMethod::Cash,
            paid_amount: grand_total,
            tenders: None,
            refunded_amount: Decimal::ZERO,
            tendered_amount: Some(tendered),
            due_date: None,
            note: None,
            source: None,
            shift_id: None,
            branch_id: None,
            invoice_type: None,
            po_reference: None,
            terms: None,
            attachment_ids: None,
            currency: None,
            exchange_rate: None,
            cost_center_id: None,
        };
        let cashier_name = crate::entities::org::users::Entity::find_by_id(actor.id).one(conn).await.map_err(AppError::from)?.map(|u| u.name).unwrap_or_else(|| "—".to_string());
        return Ok(PrintData { invoice, customer: None, cashier_name, settings: settings_dto, sample: Some(true) });
    }

    let parsed_id = id.parse::<Id>().map_err(|_| AppError::not_found("الفاتورة غير موجودة"))?;
    let model = InvoiceEntity::find_by_id(parsed_id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("الفاتورة غير موجودة"))?;
    let invoice = invoice_dto(conn, &model).await?;
    let customer = match model.customer_id {
        Some(cid) => crate::domains::parties::service::read::get_customer(conn, cid).await.ok(),
        None => None,
    };
    let cashier_name = crate::entities::org::users::Entity::find_by_id(model.cashier_id).one(conn).await.map_err(AppError::from)?.map(|u| u.name).unwrap_or_else(|| "—".to_string());
    Ok(PrintData { invoice, customer, cashier_name, settings: settings_dto, sample: None })
}

// --- Quotations ----------------------------------------------------------------------------------

fn quotation_row(model: &crate::entities::sales::quotations::Model, lines: &[crate::entities::sales::quotation_lines::Model], customer_name: Option<String>) -> QuotationRow {
    QuotationRow { quotation: super::quotations::quotation_to_dto(model, lines), customer_name }
}

pub async fn get_quotations<C: ConnectionTrait>(conn: &C, filter: Option<QuotationFilter>) -> TxResult<Vec<QuotationRow>> {
    let filter = filter.unwrap_or_default();
    let mut query = QuotationEntity::find();
    if let Some(status) = filter.status {
        query = query.filter(QuotationColumn::Status.eq(super::quotations::status_dto_to_entity(status)));
    }
    let mut rows = query.filter(QuotationColumn::DeletedAt.is_null()).all(conn).await.map_err(AppError::from)?;
    rows.sort_by(|a, b| (b.date().key(), b.created_at, b.id).cmp(&(a.date().key(), a.created_at, a.id)));

    let ids: Vec<Id> = rows.iter().map(|r| r.id).collect();
    let all_lines = if ids.is_empty() {
        Vec::new()
    } else {
        crate::entities::sales::quotation_lines::Entity::find()
            .filter(crate::entities::sales::quotation_lines::Column::QuotationId.is_in(ids))
            .order_by_asc(crate::entities::sales::quotation_lines::Column::Position)
            .all(conn)
            .await
            .map_err(AppError::from)?
    };
    let mut lines_by_q: BTreeMap<Id, Vec<crate::entities::sales::quotation_lines::Model>> = BTreeMap::new();
    for l in all_lines {
        lines_by_q.entry(l.quotation_id).or_default().push(l);
    }

    let customer_ids: Vec<Id> = rows.iter().filter_map(|r| r.customer_id).collect();
    let customer_names: BTreeMap<Id, String> = if customer_ids.is_empty() {
        BTreeMap::new()
    } else {
        crate::entities::parties::parties::Entity::find()
            .filter(crate::entities::parties::parties::Column::Id.is_in(customer_ids))
            .filter(crate::entities::parties::parties::Column::DeletedAt.is_null())
            .all(conn)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(|p| (p.id, p.name))
            .collect()
    };

    let empty: Vec<crate::entities::sales::quotation_lines::Model> = Vec::new();
    let out: Vec<QuotationRow> = rows
        .iter()
        .map(|r| {
            let lines = lines_by_q.get(&r.id).unwrap_or(&empty);
            let name = r.customer_id.and_then(|id| customer_names.get(&id).cloned());
            quotation_row(r, lines, name)
        })
        .filter(|r| matches_search(&[Some(r.quotation.number.as_str()), r.customer_name.as_deref()], filter.search.as_deref()))
        .collect();
    Ok(out)
}

pub async fn get_quotation<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<QuotationRow> {
    let model = QuotationEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("عرض السعر غير موجود"))?;
    let lines = crate::entities::sales::quotation_lines::Entity::find()
        .filter(crate::entities::sales::quotation_lines::Column::QuotationId.eq(id))
        .order_by_asc(crate::entities::sales::quotation_lines::Column::Position)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let customer_name = match model.customer_id {
        Some(cid) => crate::entities::parties::parties::Entity::find()
            .filter(crate::entities::parties::parties::Column::Id.eq(cid))
            .filter(crate::entities::parties::parties::Column::DeletedAt.is_null())
            .one(conn)
            .await
            .map_err(AppError::from)?
            .map(|p| p.name),
        None => None,
    };
    Ok(quotation_row(&model, &lines, customer_name))
}
