//! `reports::service::vat` (13 §3.9, 3.10): VAT report (with box apportionment) and VAT detail.

use indexmap::IndexMap;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::domains::settings::dto::TaxCategory;
use crate::entities::expenses::expenses;
use crate::entities::purchases::{purchase_orders, purchase_returns};
use crate::entities::sales::{invoice_lines, invoices, refunds};
use crate::shared::ledger::accounts::{resolve_account, AccountCtx, SystemRole};
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::super::dto::{DateRangeInput, VatBucket, VatCategoryBox, VatDetailRow, VatDocKind, VatReport};
use super::common::{self, DateRange};

fn parse_tax_category(s: Option<&str>) -> TaxCategory {
    match s {
        Some("S") => TaxCategory::S,
        Some("Z") => TaxCategory::Z,
        Some("E") => TaxCategory::E,
        _ => TaxCategory::O,
    }
}

fn tax_category_key(cat: TaxCategory) -> &'static str {
    match cat {
        TaxCategory::S => "S",
        TaxCategory::Z => "Z",
        TaxCategory::E => "E",
        TaxCategory::O => "O",
    }
}

/// A loaded invoice with its lines, base-converted when the invoice carries `currency`+`exchangeRate`
/// (`toBaseInvoice`, `rs:497-508`).
struct BaseInvoice {
    id: Id,
    number: String,
    date_key: String,
    sub_total: Decimal,
    discount_amount: Decimal,
    tax_amount: Decimal,
    #[allow(dead_code)]
    grand_total: Decimal,
    #[allow(dead_code)]
    tax_rate: Decimal,
    lines: Vec<BaseInvoiceLine>,
}

struct BaseInvoiceLine {
    id: Id,
    name: String,
    tax_category: Option<String>,
    tax_rate: Option<Decimal>,
    net: Option<Decimal>,
    vat: Option<Decimal>,
}

async fn load_base_invoices<C: ConnectionTrait>(conn: &C, range: &DateRange) -> TxResult<Vec<BaseInvoice>> {
    let rows = invoices::Entity::find().order_by_asc(invoices::Column::CreatedAt).order_by_asc(invoices::Column::Id).all(conn).await.map_err(AppError::from)?;
    let mut out = Vec::new();
    for inv in rows {
        if !common::in_range(inv.date_day, range) {
            continue;
        }
        let lines = invoice_lines::Entity::find()
            .filter(invoice_lines::Column::InvoiceId.eq(inv.id))
            .order_by_asc(invoice_lines::Column::Position)
            .all(conn)
            .await
            .map_err(AppError::from)?;

        let rate = inv.currency.as_ref().and(inv.exchange_rate);
        let (sub_total, discount_amount, tax_amount, grand_total) = match rate {
            Some(rate) => (round2(inv.sub_total * rate), round2(inv.discount_amount * rate), round2(inv.tax_amount * rate), round2(inv.grand_total * rate)),
            None => (inv.sub_total, inv.discount_amount, inv.tax_amount, inv.grand_total),
        };
        let base_lines = lines
            .into_iter()
            .map(|l| BaseInvoiceLine {
                id: l.id,
                name: l.name,
                tax_category: l.tax_category,
                tax_rate: l.tax_rate,
                net: match rate {
                    Some(rate) => l.net.map(|n| round2(n * rate)),
                    None => l.net,
                },
                vat: match rate {
                    Some(rate) => l.vat.map(|v| round2(v * rate)),
                    None => l.vat,
                },
            })
            .collect();

        out.push(BaseInvoice { id: inv.id, number: inv.number.clone(), date_key: inv.date().key(), sub_total, discount_amount, tax_amount, grand_total, tax_rate: inv.tax_rate, lines: base_lines });
    }
    Ok(out)
}

/// A refund row (raw, never FX-converted per Q-2).
struct RawRefund {
    id: Id,
    number: String,
    date_key: String,
    invoice_id: Id,
    sub_total: Decimal,
    tax_amount: Decimal,
}

async fn load_refunds<C: ConnectionTrait>(conn: &C, range: &DateRange) -> TxResult<Vec<RawRefund>> {
    let rows = refunds::Entity::find().order_by_asc(refunds::Column::CreatedAt).order_by_asc(refunds::Column::Id).all(conn).await.map_err(AppError::from)?;
    Ok(rows
        .into_iter()
        .filter(|r| common::in_range(r.date_day, range))
        .map(|r| RawRefund { id: r.id, number: r.number.clone(), date_key: r.date().key(), invoice_id: r.invoice_id, sub_total: r.sub_total, tax_amount: r.tax_amount })
        .collect())
}

/// `salesVatBoxes` (`rs:450-487`): `(category, rate)` boxes, first-appearance order (`IndexMap`).
fn sales_vat_boxes(invoices: &[BaseInvoice], refunds: &[RawRefund], raw_invoices_by_id: &std::collections::HashMap<Id, RawInvoiceForRefund>) -> Vec<VatCategoryBox> {
    let mut boxes: IndexMap<(String, String), VatCategoryBox> = IndexMap::new();
    let mut add = |category: TaxCategory, rate: Decimal, net: Decimal, vat: Decimal| {
        let key = (tax_category_key(category).to_string(), rate.to_string());
        let entry = boxes.entry(key).or_insert(VatCategoryBox { category, rate, net: Decimal::ZERO, vat: Decimal::ZERO, count: 0 });
        entry.net = round2(entry.net + net);
        entry.vat = round2(entry.vat + vat);
    };

    for inv in invoices {
        for line in &inv.lines {
            let cat = parse_tax_category(line.tax_category.as_deref());
            let rate = line.tax_rate.unwrap_or(Decimal::ZERO);
            add(cat, rate, line.net.unwrap_or(Decimal::ZERO), line.vat.unwrap_or(Decimal::ZERO));
        }
    }

    for refund in refunds {
        let Some(inv) = raw_invoices_by_id.get(&refund.invoice_id) else { continue };
        if inv.tax_amount == Decimal::ZERO {
            continue;
        }
        let inv_vat = inv.tax_amount;
        let inv_net = inv.sub_total - inv.discount_amount;
        for line in &inv.lines {
            let line_vat = line.vat.unwrap_or(Decimal::ZERO);
            let line_net = line.net.unwrap_or(Decimal::ZERO);
            let share_of_vat = if inv_vat > Decimal::ZERO { line_vat / inv_vat } else { Decimal::ZERO };
            let share_of_net = if inv_net > Decimal::ZERO { line_net / inv_net } else { Decimal::ZERO };
            let cat = parse_tax_category(line.tax_category.as_deref());
            let rate = line.tax_rate.unwrap_or(Decimal::ZERO);
            add(cat, rate, -round2(refund.sub_total * share_of_net), -round2(refund.tax_amount * share_of_vat));
        }
    }

    boxes.into_values().filter(|b| b.net != Decimal::ZERO || b.vat != Decimal::ZERO).collect()
}

/// The refund apportionment reads the invoice **un-converted** (Q-2) — a distinct, raw-figure load
/// from `load_base_invoices`'s base-converted one, matching `rs:470` exactly.
struct RawInvoiceForRefund {
    sub_total: Decimal,
    discount_amount: Decimal,
    tax_amount: Decimal,
    lines: Vec<RawInvoiceLineForRefund>,
}
struct RawInvoiceLineForRefund {
    tax_category: Option<String>,
    tax_rate: Option<Decimal>,
    net: Option<Decimal>,
    vat: Option<Decimal>,
}

async fn load_raw_invoices_by_id<C: ConnectionTrait>(conn: &C, ids: &[Id]) -> TxResult<std::collections::HashMap<Id, RawInvoiceForRefund>> {
    let mut out = std::collections::HashMap::new();
    if ids.is_empty() {
        return Ok(out);
    }
    let invs = invoices::Entity::find().filter(invoices::Column::Id.is_in(ids.iter().copied())).all(conn).await.map_err(AppError::from)?;
    for inv in invs {
        let lines = invoice_lines::Entity::find()
            .filter(invoice_lines::Column::InvoiceId.eq(inv.id))
            .order_by_asc(invoice_lines::Column::Position)
            .all(conn)
            .await
            .map_err(AppError::from)?;
        out.insert(
            inv.id,
            RawInvoiceForRefund {
                sub_total: inv.sub_total,
                discount_amount: inv.discount_amount,
                tax_amount: inv.tax_amount,
                lines: lines.into_iter().map(|l| RawInvoiceLineForRefund { tax_category: l.tax_category, tax_rate: l.tax_rate, net: l.net, vat: l.vat }).collect(),
            },
        );
    }
    Ok(out)
}

/// **`reports_get_vat_report`** (13 §3.9).
pub async fn vat_report<C: ConnectionTrait>(conn: &C, range: &DateRangeInput) -> TxResult<VatReport> {
    let date_range = DateRange::parse(range)?;
    let base_invoices = load_base_invoices(conn, &date_range).await?;
    let refund_rows = load_refunds(conn, &date_range).await?;

    let refund_invoice_ids: Vec<Id> = refund_rows.iter().map(|r| r.invoice_id).collect();
    let raw_invoices_by_id = load_raw_invoices_by_id(conn, &refund_invoice_ids).await?;

    let boxes = sales_vat_boxes(&base_invoices, &refund_rows, &raw_invoices_by_id);

    // POs: RECEIVED + VAT-recoverable, in range; recoverable_po_ids = ALL such POs (any date), for
    // purchase-return matching.
    let all_pos = purchase_orders::Entity::find().all(conn).await.map_err(AppError::from)?;
    let recoverable_po_ids: std::collections::HashSet<Id> = all_pos
        .iter()
        .filter(|p| p.status == purchase_orders::PurchaseStatus::Received && !p.vat_not_recoverable.unwrap_or(false))
        .map(|p| p.id)
        .collect();
    let pos_in_range: Vec<&purchase_orders::Model> =
        all_pos.iter().filter(|p| recoverable_po_ids.contains(&p.id) && common::in_range(p.date_day, &date_range)).collect();

    let all_returns = purchase_returns::Entity::find().all(conn).await.map_err(AppError::from)?;
    let p_returns: Vec<&purchase_returns::Model> =
        all_returns.iter().filter(|r| recoverable_po_ids.contains(&r.purchase_order_id) && common::in_range(r.date_day, &date_range)).collect();

    let expense_rows = expenses::Entity::find().all(conn).await.map_err(AppError::from)?;
    let tax_expenses: Vec<&expenses::Model> = expense_rows.iter().filter(|e| e.is_tax_invoice && common::in_range(e.date_day, &date_range)).collect();

    let sales = VatBucket {
        taxable: common::sum2(base_invoices.iter().map(|i| i.sub_total - i.discount_amount)),
        vat: common::sum2(base_invoices.iter().map(|i| i.tax_amount)),
        count: base_invoices.len() as i32,
    };
    let sales_returns =
        VatBucket { taxable: common::sum2(refund_rows.iter().map(|r| r.sub_total)), vat: common::sum2(refund_rows.iter().map(|r| r.tax_amount)), count: refund_rows.len() as i32 };
    let purchases = VatBucket {
        taxable: round2(common::sum2(pos_in_range.iter().map(|p| p.sub_total)) + common::sum2(tax_expenses.iter().map(|e| e.net_amount))),
        vat: round2(common::sum2(pos_in_range.iter().map(|p| p.tax_amount)) + common::sum2(tax_expenses.iter().map(|e| e.tax_amount))),
        count: pos_in_range.len() as i32 + tax_expenses.len() as i32,
    };
    let purchase_returns = VatBucket { taxable: common::sum2(p_returns.iter().map(|r| r.sub_total)), vat: common::sum2(p_returns.iter().map(|r| r.tax_amount)), count: p_returns.len() as i32 };

    let output_vat = round2(sales.vat - sales_returns.vat);
    let input_vat = round2(purchases.vat - purchase_returns.vat);
    let net_payable = round2(output_vat - input_vat);

    let mv = common::movements(conn, &date_range, None).await?;
    let vat_output_account = resolve_account(conn, SystemRole::VatOutput, &AccountCtx::default()).await.map_err(TxErrFromApp)?;
    let vat_input_account = resolve_account(conn, SystemRole::VatInput, &AccountCtx::default()).await.map_err(TxErrFromApp)?;
    let out = mv.get(&vat_output_account.id).copied().unwrap_or_default();
    let inp = mv.get(&vat_input_account.id).copied().unwrap_or_default();
    let ledger_output = round2(out.c - out.d);
    let ledger_input = round2(inp.d - inp.c);

    Ok(VatReport { sales, sales_returns, purchases, purchase_returns, output_vat, input_vat, net_payable, ledger_output, ledger_input, sales_boxes: boxes })
}

struct TxErrFromApp(AppError);
impl From<AppError> for TxErrFromApp {
    fn from(e: AppError) -> Self {
        TxErrFromApp(e)
    }
}
impl From<TxErrFromApp> for crate::core::tx::TxError {
    fn from(e: TxErrFromApp) -> Self {
        crate::core::tx::TxError::App(e.0)
    }
}

/// **`reports_get_vat_detail`** (13 §3.10).
pub async fn vat_detail<C: ConnectionTrait>(conn: &C, range: &DateRangeInput) -> TxResult<Vec<VatDetailRow>> {
    let date_range = DateRange::parse(range)?;
    let base_invoices = load_base_invoices(conn, &date_range).await?;
    let refund_rows = load_refunds(conn, &date_range).await?;

    let refund_invoice_ids: Vec<Id> = refund_rows.iter().map(|r| r.invoice_id).collect();
    // For the productName fallback ("مرتجع — <inv.number>") we need each refund's invoice number
    // and taxRate — a lighter lookup than the full raw-invoice load `vat_report` needs.
    let invoice_lookup: std::collections::HashMap<Id, (String, Decimal)> = if refund_invoice_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        invoices::Entity::find()
            .filter(invoices::Column::Id.is_in(refund_invoice_ids.iter().copied()))
            .all(conn)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(|i| (i.id, (i.number, i.tax_rate)))
            .collect()
    };

    let mut rows = Vec::new();
    for inv in &base_invoices {
        for line in &inv.lines {
            // `!line.net && !line.vat` (`rs:564`) is true for both `undefined` and `0` — skip when
            // both are falsy (None or exactly zero).
            let net_falsy = line.net.is_none() || line.net == Some(Decimal::ZERO);
            let vat_falsy = line.vat.is_none() || line.vat == Some(Decimal::ZERO);
            if net_falsy && vat_falsy {
                continue;
            }
            rows.push(VatDetailRow {
                id: format!("{}:{}", inv.id, line.id),
                date: inv.date_key.clone(),
                document_number: inv.number.clone(),
                document_kind: VatDocKind::Sale,
                product_name: line.name.clone(),
                category: parse_tax_category(line.tax_category.as_deref()),
                rate: line.tax_rate.unwrap_or(Decimal::ZERO),
                net: line.net.unwrap_or(Decimal::ZERO),
                vat: line.vat.unwrap_or(Decimal::ZERO),
            });
        }
    }

    for refund in &refund_rows {
        let inv = invoice_lookup.get(&refund.invoice_id);
        let product_name = match inv {
            Some((number, _)) => format!("مرتجع — {number}"),
            None => "مرتجع".to_string(),
        };
        let rate = inv.map(|(_, r)| *r).unwrap_or(Decimal::ZERO);
        rows.push(VatDetailRow {
            id: refund.id.to_string(),
            date: refund.date_key.clone(),
            document_number: refund.number.clone(),
            document_kind: VatDocKind::SalesReturn,
            product_name,
            category: TaxCategory::S,
            rate,
            net: -refund.sub_total,
            vat: -refund.tax_amount,
        });
    }

    rows.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(rows)
}
