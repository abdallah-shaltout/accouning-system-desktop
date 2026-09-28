//! `domains::invoices::service::common` — shared pieces (08 §3.1): `invoice_dto`/row batch loader,
//! `tax_for_line`, `base_qty`, legacy method lookup, `branch_prefix` re-export, `sales_tax_rate`.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::org::payment_methods::{Column as MethodColumn, Entity as MethodEntity, Model as PaymentMethodModel};
use crate::entities::org::taxes::{Column as TaxColumn, Entity as TaxEntity};
use crate::entities::sales::invoice_lines::{Column as LineColumn, Entity as LineEntity, Model as LineModel};
use crate::entities::sales::invoice_tenders::{Column as TenderColumn, Entity as TenderEntity, Model as TenderModel};
use crate::entities::sales::invoices::Model as InvoiceModel;
use crate::entities::soft_delete::SoftDelete;
use crate::utils::id::Id;
use crate::utils::money::round2;

use crate::domains::settings::dto::TaxCategory;

use super::super::dto::{self as d, InvoiceLine, InvoiceRow, PaymentStatus, Tender};

/// `taxForLine`'s minimal return shape (`sales.ts:59-65`).
#[derive(Debug, Clone, Copy)]
pub struct LineTax {
    pub rate: Decimal,
    pub category: TaxCategory,
    pub id: Option<Id>,
}

/// Maps this domain's `shared::totals::TaxCategory` <-> the wire `settings::dto::TaxCategory` via
/// the single-letter code both sides already agree on (`shared::totals::TaxCategory::as_str()` /
/// `from_str_or_other`).
pub fn to_totals_category(c: TaxCategory) -> crate::shared::totals::TaxCategory {
    match c {
        TaxCategory::S => crate::shared::totals::TaxCategory::Standard,
        TaxCategory::Z => crate::shared::totals::TaxCategory::Zero,
        TaxCategory::E => crate::shared::totals::TaxCategory::Exempt,
        TaxCategory::O => crate::shared::totals::TaxCategory::Other,
    }
}

pub fn from_totals_category(c: crate::shared::totals::TaxCategory) -> TaxCategory {
    match c {
        crate::shared::totals::TaxCategory::Standard => TaxCategory::S,
        crate::shared::totals::TaxCategory::Zero => TaxCategory::Z,
        crate::shared::totals::TaxCategory::Exempt => TaxCategory::E,
        crate::shared::totals::TaxCategory::Other => TaxCategory::O,
    }
}

/// `salesTaxRate()` (`src/mocks/backend/core.ts`, referenced by `taxForLine`'s fallback and the
/// `getInvoicePrintData('sample')`/`saveQuotation` fallbacks): the rate of the active `OUTPUT` tax
/// flagged `is_default`, or `0` when none exists. Mirrors `shared::defaults::purchase_tax_rate`'s
/// shape exactly but for the `OUTPUT` direction (invoices are the only caller — kept local rather
/// than in `shared::defaults`, which is purchases/products-owned, G-26).
pub async fn sales_tax_rate<C: ConnectionTrait>(conn: &C) -> TxResult<Decimal> {
    let tax = TaxEntity::find()
        .filter(TaxColumn::Type.eq("OUTPUT"))
        .filter(TaxColumn::IsDefault.eq(true))
        .filter(TaxColumn::Active.eq(true))
        .filter(TaxColumn::DeletedAt.is_null())
        .one(conn)
        .await
        .map_err(AppError::from)?;
    Ok(tax.map(|t| t.rate).unwrap_or(Decimal::ZERO))
}

/// `taxForLine(taxId)` (`sales.ts:59-65`): live active tax by id -> live active tax
/// `settings.default_tax_id` -> first live active `OUTPUT` `is_default` tax by `(created_at, id)`
/// -> else `{ rate: 0, category: 'S' }` (the `salesTaxRate()` fallback is 0 once both lookups
/// failed, category defaults to Standard as the mock's own inline object literal does).
pub async fn tax_for_line<C: ConnectionTrait>(conn: &C, tax_id: Option<Id>) -> TxResult<LineTax> {
    if let Some(id) = tax_id {
        if let Some(tax) = TaxEntity::find_live().filter(TaxColumn::Id.eq(id)).filter(TaxColumn::Active.eq(true)).one(conn).await.map_err(AppError::from)? {
            return Ok(LineTax { rate: tax.rate, category: parse_category(&tax.category), id: Some(tax.id) });
        }
    }

    let settings = crate::core::settings::load(conn).await?;
    if let Some(default_id) = settings.default_tax_id {
        if let Some(tax) = TaxEntity::find_live().filter(TaxColumn::Id.eq(default_id)).filter(TaxColumn::Active.eq(true)).one(conn).await.map_err(AppError::from)? {
            return Ok(LineTax { rate: tax.rate, category: parse_category(&tax.category), id: Some(tax.id) });
        }
    }

    let default_output = TaxEntity::find_live()
        .filter(TaxColumn::Type.eq("OUTPUT"))
        .filter(TaxColumn::IsDefault.eq(true))
        .filter(TaxColumn::Active.eq(true))
        .order_by_asc(TaxColumn::CreatedAt)
        .order_by_asc(TaxColumn::Id)
        .one(conn)
        .await
        .map_err(AppError::from)?;
    if let Some(tax) = default_output {
        return Ok(LineTax { rate: tax.rate, category: parse_category(&tax.category), id: Some(tax.id) });
    }

    Ok(LineTax { rate: Decimal::ZERO, category: TaxCategory::S, id: None })
}

fn parse_category(raw: &str) -> TaxCategory {
    match raw {
        "S" => TaxCategory::S,
        "Z" => TaxCategory::Z,
        "E" => TaxCategory::E,
        _ => TaxCategory::O,
    }
}

/// `baseQty(line)` (`sales.ts:81-83`, P2-19): `round2(qty × (unitFactor ?? 1))`.
pub fn base_qty(qty: Decimal, unit_factor: Option<Decimal>) -> Decimal {
    round2(qty * unit_factor.unwrap_or(Decimal::ONE))
}

/// Legacy method ids (`sales.ts:51-56`, D-I5): resolved by payment-method `type` — first live
/// method of that type by `(sort_order, created_at, id)`; none -> `NOT_FOUND` `طريقة الدفع غير موجودة`.
pub async fn legacy_method_by_type<C: ConnectionTrait>(conn: &C, method_type: &str) -> TxResult<PaymentMethodModel> {
    MethodEntity::find_live()
        .filter(MethodColumn::Type.eq(method_type))
        .order_by_asc(MethodColumn::SortOrder)
        .order_by_asc(MethodColumn::CreatedAt)
        .order_by_asc(MethodColumn::Id)
        .one(conn)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("طريقة الدفع غير موجودة").into())
}

/// A payment method by id, live or not (mirrors the mock's plain `db.paymentMethods.find`, which
/// never filters `active`/soft-delete — a tender snapshot on an already-posted invoice must still
/// resolve its method for display even if the method was later deactivated).
pub async fn payment_method_by_id<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<PaymentMethodModel> {
    MethodEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("طريقة الدفع غير موجودة").into())
}

/// Live, active payment method by id — used at resolution time inside `prepare_sale` (a tender
/// must name an active method to be accepted on a NEW sale, matching `paymentMethodById`'s use
/// there, which reads the same unfiltered `db.paymentMethods.find` the mock always used — kept
/// identical to `payment_method_by_id` above; this alias exists only for call-site clarity).
pub use payment_method_by_id as resolve_tender_method;

// --- DTO assembly ------------------------------------------------------------------------------

fn tender_to_dto(t: &TenderModel) -> Tender {
    Tender { payment_method_id: t.payment_method_id, amount: t.amount, reference: t.reference.clone() }
}

fn line_to_dto(l: &LineModel) -> InvoiceLine {
    let product_id = match l.product_id {
        Some(id) => id.to_string(),
        None => format!("freetext-{}", l.position),
    };
    InvoiceLine {
        id: format!("{}-l{}", l.invoice_id, l.position + 1),
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

pub fn invoice_status_to_dto(s: &crate::entities::sales::invoices::InvoiceStatus) -> d::InvoiceStatus {
    use crate::entities::sales::invoices::InvoiceStatus as E;
    match s {
        E::Draft => d::InvoiceStatus::Draft,
        E::Completed => d::InvoiceStatus::Completed,
        E::Refunded => d::InvoiceStatus::Refunded,
    }
}

pub fn payment_status_to_dto(s: &crate::entities::sales::invoices::PaymentStatus) -> PaymentStatus {
    use crate::entities::sales::invoices::PaymentStatus as E;
    match s {
        E::Unpaid => PaymentStatus::Unpaid,
        E::PartiallyPaid => PaymentStatus::PartiallyPaid,
        E::Paid => PaymentStatus::Paid,
    }
}

/// `InvoiceListFilter.status` (DTO) -> the entity's `InvoiceStatus` column value.
pub fn entity_status(s: d::InvoiceStatus) -> crate::entities::sales::invoices::InvoiceStatus {
    use crate::entities::sales::invoices::InvoiceStatus as E;
    match s {
        d::InvoiceStatus::Draft => E::Draft,
        d::InvoiceStatus::Completed => E::Completed,
        d::InvoiceStatus::Refunded => E::Refunded,
    }
}

/// `InvoiceListFilter.paymentStatus` (DTO) -> the entity's `PaymentStatus` column value.
pub fn entity_payment_status(s: PaymentStatus) -> crate::entities::sales::invoices::PaymentStatus {
    use crate::entities::sales::invoices::PaymentStatus as E;
    match s {
        PaymentStatus::Unpaid => E::Unpaid,
        PaymentStatus::PartiallyPaid => E::PartiallyPaid,
        PaymentStatus::Paid => E::Paid,
    }
}

pub fn payment_status_kind_to_entity(k: crate::shared::totals::PaymentStatusKind) -> crate::entities::sales::invoices::PaymentStatus {
    use crate::entities::sales::invoices::PaymentStatus as E;
    use crate::shared::totals::PaymentStatusKind as K;
    match k {
        K::Paid => E::Paid,
        K::PartiallyPaid => E::PartiallyPaid,
        K::Unpaid => E::Unpaid,
    }
}

pub fn payment_method_to_dto(m: &crate::entities::sales::invoices::SalePaymentMethod) -> d::SalePaymentMethod {
    use crate::entities::sales::invoices::SalePaymentMethod as E;
    match m {
        E::Cash => d::SalePaymentMethod::Cash,
        E::Card => d::SalePaymentMethod::Card,
        E::BankTransfer => d::SalePaymentMethod::BankTransfer,
        E::Credit => d::SalePaymentMethod::Credit,
    }
}

pub fn payment_method_dto_to_entity(m: d::SalePaymentMethod) -> crate::entities::sales::invoices::SalePaymentMethod {
    use crate::entities::sales::invoices::SalePaymentMethod as E;
    match m {
        d::SalePaymentMethod::Cash => E::Cash,
        d::SalePaymentMethod::Card => E::Card,
        d::SalePaymentMethod::BankTransfer => E::BankTransfer,
        d::SalePaymentMethod::Credit => E::Credit,
    }
}

pub fn source_to_dto(s: &Option<crate::entities::sales::invoices::InvoiceSource>) -> Option<d::InvoiceSource> {
    use crate::entities::sales::invoices::InvoiceSource as E;
    s.as_ref().map(|s| match s {
        E::Pos => d::InvoiceSource::Pos,
        E::Desk => d::InvoiceSource::Desk,
    })
}

pub fn source_dto_to_entity(s: d::InvoiceSource) -> crate::entities::sales::invoices::InvoiceSource {
    use crate::entities::sales::invoices::InvoiceSource as E;
    match s {
        d::InvoiceSource::Pos => E::Pos,
        d::InvoiceSource::Desk => E::Desk,
    }
}

pub fn invoice_type_to_dto(t: &Option<crate::entities::sales::invoices::InvoiceType>) -> Option<d::InvoiceType> {
    use crate::entities::sales::invoices::InvoiceType as E;
    t.as_ref().map(|t| match t {
        E::Standard => d::InvoiceType::Standard,
        E::Simplified => d::InvoiceType::Simplified,
    })
}

pub fn invoice_type_dto_to_entity(t: d::InvoiceType) -> crate::entities::sales::invoices::InvoiceType {
    use crate::entities::sales::invoices::InvoiceType as E;
    match t {
        d::InvoiceType::Standard => E::Standard,
        d::InvoiceType::Simplified => E::Simplified,
    }
}

/// `invoice_dto(conn, model)` (08 §3.1): lines by `position`, tenders by `position`.
pub async fn invoice_dto<C: ConnectionTrait>(conn: &C, model: &InvoiceModel) -> TxResult<d::Invoice> {
    let lines = LineEntity::find().filter(LineColumn::InvoiceId.eq(model.id)).order_by_asc(LineColumn::Position).all(conn).await.map_err(AppError::from)?;
    let tenders = TenderEntity::find().filter(TenderColumn::InvoiceId.eq(model.id)).order_by_asc(TenderColumn::Position).all(conn).await.map_err(AppError::from)?;
    Ok(to_invoice_dto(model, &lines, &tenders))
}

fn to_invoice_dto(model: &InvoiceModel, lines: &[LineModel], tenders: &[TenderModel]) -> d::Invoice {
    d::Invoice {
        id: model.id,
        number: model.number.clone(),
        date: model.date().key(),
        customer_id: model.customer_id,
        cashier_id: model.cashier_id,
        status: invoice_status_to_dto(&model.status),
        payment_status: payment_status_to_dto(&model.payment_status),
        lines: lines.iter().map(line_to_dto).collect(),
        sub_total: model.sub_total,
        discount_rate: model.discount_rate,
        discount_amount: model.discount_amount,
        tax_rate: model.tax_rate,
        tax_amount: model.tax_amount,
        grand_total: model.grand_total,
        payment_method: payment_method_to_dto(&model.payment_method),
        paid_amount: model.paid_amount,
        tenders: Some(tenders.iter().map(tender_to_dto).collect()),
        refunded_amount: model.refunded_amount,
        tendered_amount: model.tendered_amount,
        due_date: model.due_date().map(|d| d.key()),
        note: model.note.clone(),
        source: source_to_dto(&model.source),
        shift_id: model.shift_id,
        branch_id: model.branch_id,
        invoice_type: invoice_type_to_dto(&model.invoice_type),
        po_reference: model.po_reference.clone(),
        terms: model.terms.clone(),
        attachment_ids: model.attachment_ids.as_ref().map(|l| l.0.clone()),
        currency: model.currency.clone(),
        exchange_rate: model.exchange_rate,
        cost_center_id: model.cost_center_id,
    }
}

/// Live party name (deleted ≡ absent, P2-16) — used for `customer_name` on rows.
async fn live_party_name<C: ConnectionTrait>(conn: &C, id: Option<Id>) -> TxResult<Option<String>> {
    let Some(id) = id else { return Ok(None) };
    let row = crate::entities::parties::parties::Entity::find_live()
        .filter(crate::entities::parties::parties::Column::Id.eq(id))
        .one(conn)
        .await
        .map_err(AppError::from)?;
    Ok(row.map(|p| p.name))
}

/// Live user name, else `—`.
async fn user_name<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<String> {
    let row = crate::entities::org::users::Entity::find_by_id(id).one(conn).await.map_err(AppError::from)?;
    Ok(row.map(|u| u.name).unwrap_or_else(|| "—".to_string()))
}

/// `invoiceService.ts:46-53`: builds one `InvoiceRow` for a single invoice (used by `get_invoice`).
pub async fn invoice_row<C: ConnectionTrait>(conn: &C, model: &InvoiceModel) -> TxResult<InvoiceRow> {
    let invoice = invoice_dto(conn, model).await?;
    let customer_name = live_party_name(conn, model.customer_id).await?;
    let cashier_name = user_name(conn, model.cashier_id).await?;
    let outstanding = if matches!(model.status, crate::entities::sales::invoices::InvoiceStatus::Refunded) {
        Decimal::ZERO
    } else {
        crate::shared::totals::document_outstanding(round2(model.grand_total - model.refunded_amount), model.paid_amount)
    };
    Ok(InvoiceRow { invoice, customer_name, cashier_name, outstanding })
}

/// Batch row builder for lists (08 §3.1 "batch-load lines, tenders, party names and user names in
/// 5 queries, never N+1"): loads every line/tender for the given invoices in 2 queries, every
/// distinct customer/cashier name in 2 more, then assembles rows in memory.
pub async fn invoice_rows<C: ConnectionTrait>(conn: &C, models: &[InvoiceModel]) -> TxResult<Vec<InvoiceRow>> {
    if models.is_empty() {
        return Ok(Vec::new());
    }
    let ids: Vec<Id> = models.iter().map(|m| m.id).collect();

    let all_lines = LineEntity::find().filter(LineColumn::InvoiceId.is_in(ids.clone())).order_by_asc(LineColumn::Position).all(conn).await.map_err(AppError::from)?;
    let mut lines_by_invoice: BTreeMap<Id, Vec<LineModel>> = BTreeMap::new();
    for l in all_lines {
        lines_by_invoice.entry(l.invoice_id).or_default().push(l);
    }

    let all_tenders = TenderEntity::find().filter(TenderColumn::InvoiceId.is_in(ids.clone())).order_by_asc(TenderColumn::Position).all(conn).await.map_err(AppError::from)?;
    let mut tenders_by_invoice: BTreeMap<Id, Vec<TenderModel>> = BTreeMap::new();
    for t in all_tenders {
        tenders_by_invoice.entry(t.invoice_id).or_default().push(t);
    }

    let customer_ids: Vec<Id> = models.iter().filter_map(|m| m.customer_id).collect();
    let customer_names: BTreeMap<Id, String> = if customer_ids.is_empty() {
        BTreeMap::new()
    } else {
        crate::entities::parties::parties::Entity::find_live()
            .filter(crate::entities::parties::parties::Column::Id.is_in(customer_ids))
            .all(conn)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(|p| (p.id, p.name))
            .collect()
    };

    let cashier_ids: Vec<Id> = models.iter().map(|m| m.cashier_id).collect();
    let user_names: BTreeMap<Id, String> = crate::entities::org::users::Entity::find()
        .filter(crate::entities::org::users::Column::Id.is_in(cashier_ids))
        .all(conn)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .map(|u| (u.id, u.name))
        .collect();

    let mut out = Vec::with_capacity(models.len());
    for m in models {
        let empty_lines: Vec<LineModel> = Vec::new();
        let empty_tenders: Vec<TenderModel> = Vec::new();
        let lines = lines_by_invoice.get(&m.id).unwrap_or(&empty_lines);
        let tenders = tenders_by_invoice.get(&m.id).unwrap_or(&empty_tenders);
        let invoice = to_invoice_dto(m, lines, tenders);
        let customer_name = m.customer_id.and_then(|id| customer_names.get(&id).cloned());
        let cashier_name = user_names.get(&m.cashier_id).cloned().unwrap_or_else(|| "—".to_string());
        let outstanding = if matches!(m.status, crate::entities::sales::invoices::InvoiceStatus::Refunded) {
            Decimal::ZERO
        } else {
            crate::shared::totals::document_outstanding(round2(m.grand_total - m.refunded_amount), m.paid_amount)
        };
        out.push(InvoiceRow { invoice, customer_name, cashier_name, outstanding });
    }
    Ok(out)
}
