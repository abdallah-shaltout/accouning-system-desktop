//! `purchases` domain service logic — split per 07-purchases.md §3's file plan: `orders` (U-3…U-5,
//! U-7, U-8), `receive` (U-6), `returns` (U-9, U-11), `read` (U-1, U-2, U-10). This file holds the
//! shared helpers §3 names before the per-file sections: `supplier`, `purchase_outstanding`,
//! `missing_supplier_invoice`, `duplicate_supplier_invoice`, `base_qty`, `base_unit_cost`,
//! `line_remaining`, `returned_qty_by_product`, `purchase_line_account`, `compute_purchase_totals`.

pub mod orders;
pub mod read;
pub mod receive;
pub mod returns;

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::parties::parties::{Column as PartyColumn, Entity as PartyEntity, Model as PartyModel};
use crate::entities::purchases::purchase_return_lines::{Column as ReturnLineColumn, Entity as ReturnLineEntity};
use crate::entities::purchases::purchase_returns::{Column as ReturnColumn, Entity as ReturnEntity};
use crate::shared::ledger::accounts::{purchase_account_for, ProductAccountOverrides};
use crate::shared::stock::LockedProduct;
use crate::shared::totals::{compute_invoice_totals, InvoiceDiscountInput, TaxCategory, TotalsLineInput, TotalsTax};
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::dto::{InvoiceDiscount, PurchaseOrder};

/// `supplier(conn, id)` (07 §3): the `parties` row with `kind = 'supplier'` — `NOT_FOUND` when
/// missing or not a supplier row (matches every other domain's `productById`/party-lookup shape).
/// Not called by this wave's own service functions — every call site in `purchases.ts` that reads
/// a supplier uses a plain `db.suppliers.find(...)` (never throws, `undefined` handled inline:
/// `validateLines`'s own `اختر المورد` message, `receivePurchase`'s `supplier?.vatNumber`,
/// `recordPurchaseReturn`'s nothing-at-all), so a throwing lookup never matches the mock's actual
/// per-call-site behavior anywhere in this domain. Kept `pub(crate)` for API completeness/parity
/// with `products::service::stock_lines`'s equivalent helper shape, and because a later domain
/// (payments, when it needs a supplier by id with a hard failure) may want it.
#[allow(dead_code)]
pub(crate) async fn supplier<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<PartyModel> {
    let party = PartyEntity::find_by_id(id).filter(PartyColumn::Kind.eq("supplier")).one(conn).await.map_err(AppError::from)?;
    party.ok_or_else(|| AppError::not_found("المورد غير موجود").into())
}

/// `purchaseOutstanding` (`purchases.ts:74-76`): `max(0, round2(grand - returned - paid))`.
pub(crate) fn purchase_outstanding(po: &PurchaseOrder) -> Decimal {
    round2(po.grand_total - po.returned_amount - po.paid_amount).max(Decimal::ZERO)
}

/// `missingSupplierInvoice` (`purchases.ts:79-81`): RECEIVED and blank number or no date.
pub(crate) fn missing_supplier_invoice(po: &PurchaseOrder) -> bool {
    matches!(po.status, super::dto::PurchaseStatus::Received)
        && (po.supplier_invoice_no.as_deref().map(str::trim).unwrap_or("").is_empty() || po.supplier_invoice_date.is_none())
}

/// `duplicateSupplierInvoice` (`purchases.ts:84-90`): another RECEIVED order from the same supplier
/// (excluding `exclude_po_id`) already used this trimmed number, first by `(created_at, id)`.
pub(crate) async fn duplicate_supplier_invoice<C: ConnectionTrait>(
    conn: &C,
    supplier_id: Id,
    supplier_invoice_no: &str,
    exclude_po_id: Option<Id>,
) -> TxResult<Option<crate::entities::purchases::purchase_orders::Model>> {
    let no = supplier_invoice_no.trim();
    if no.is_empty() {
        return Ok(None);
    }
    use crate::entities::purchases::purchase_orders::{Column as PoColumn, Entity as PoEntity};
    let mut query = PoEntity::find()
        .filter(PoColumn::SupplierId.eq(supplier_id))
        .filter(PoColumn::Status.eq(crate::entities::purchases::purchase_orders::PurchaseStatus::Received))
        .filter(PoColumn::SupplierInvoiceNo.eq(no))
        .order_by_asc(PoColumn::CreatedAt)
        .order_by_asc(PoColumn::Id);
    if let Some(exclude) = exclude_po_id {
        query = query.filter(PoColumn::Id.ne(exclude));
    }
    let rows = query.all(conn).await.map_err(AppError::from)?;
    // The trim happens app-side (SQL equality above compares the stored value verbatim, which is
    // already trimmed on write) — re-check with `trim()` defensively for any legacy/imported row.
    Ok(rows.into_iter().find(|p| p.supplier_invoice_no.as_deref().map(str::trim) == Some(no)))
}

/// `baseQty` (`purchases.ts:105-107`): `round2(qty * (unitFactor ?? 1))`.
pub(crate) fn base_qty(qty: Decimal, unit_factor: Option<Decimal>) -> Decimal {
    round2(qty * unit_factor.unwrap_or(Decimal::ONE))
}

/// `baseUnitCost` (`purchases.ts:110-113`): unrounded `cost / factor` (factor > 0), else `cost`.
pub(crate) fn base_unit_cost(cost_price: Decimal, unit_factor: Option<Decimal>) -> Decimal {
    let factor = unit_factor.unwrap_or(Decimal::ONE);
    if factor > Decimal::ZERO {
        cost_price / factor
    } else {
        cost_price
    }
}

/// `lineRemaining` (`purchases.ts:215-217`): `round2(baseQty(l) - (receivedQty ?? 0))`.
pub(crate) fn line_remaining(qty: Decimal, unit_factor: Option<Decimal>, received_qty: Option<Decimal>) -> Decimal {
    round2(base_qty(qty, unit_factor) - received_qty.unwrap_or(Decimal::ZERO))
}

/// `returnedQtyByProduct(poId)` (`purchases.ts:387-393`): `SUM(qty)` of the PO's
/// `purchase_return_lines` per product, across every return on this PO.
pub(crate) async fn returned_qty_by_product<C: ConnectionTrait>(conn: &C, po_id: Id) -> TxResult<BTreeMap<Id, Decimal>> {
    let returns = ReturnEntity::find().filter(ReturnColumn::PurchaseOrderId.eq(po_id)).all(conn).await.map_err(AppError::from)?;
    if returns.is_empty() {
        return Ok(BTreeMap::new());
    }
    let return_ids: Vec<Id> = returns.iter().map(|r| r.id).collect();
    let lines = ReturnLineEntity::find().filter(ReturnLineColumn::PurchaseReturnId.is_in(return_ids)).all(conn).await.map_err(AppError::from)?;
    let mut map: BTreeMap<Id, Decimal> = BTreeMap::new();
    for line in lines {
        *map.entry(line.product_id).or_insert(Decimal::ZERO) += line.qty;
    }
    Ok(map)
}

/// `purchaseLineAccountId` (`purchases.ts:27-34`): product override -> live category's
/// `purchase_account_id` -> settings default -> `freightIn` fallback (via the shared
/// `purchase_account_for` helper, which already implements this exact chain — G-21/G-26 style
/// reuse, no second copy).
pub(crate) async fn purchase_line_account<C: ConnectionTrait>(
    conn: &C,
    product: &LockedProduct,
    default_purchase_account_id: Option<Id>,
) -> TxResult<Id> {
    let category_account_id = match product.model.category_id {
        Some(cat_id) => {
            use crate::entities::catalog::categories::Entity as CategoryEntity;
            CategoryEntity::find_by_id(cat_id).one(conn).await.map_err(AppError::from)?.and_then(|c| c.purchase_account_id)
        }
        None => None,
    };
    let overrides = ProductAccountOverrides { product_account_id: product.model.purchase_account_id, category_account_id };
    let account = purchase_account_for(conn, &overrides, default_purchase_account_id).await?;
    Ok(account.id)
}

/// A purchase totals line — mirrors `computePurchaseTotals`'s per-line input shape
/// (`purchaseService.ts:47-51`) before it's mapped into `shared::totals::TotalsLineInput`.
pub(crate) struct PurchaseTotalsLine {
    pub qty: Decimal,
    pub cost_price: Decimal,
    pub discount: Option<Decimal>,
    pub discount_is_pct: Option<bool>,
    pub tax_id: Option<Id>,
}

/// The resolved totals `computePurchaseTotals` returns (`purchaseService.ts:66-71`).
pub(crate) struct PurchaseTotals {
    pub sub_total: Decimal,
    pub tax_amount: Decimal,
    pub grand_total: Decimal,
    pub tax_rate: Decimal,
}

/// **`compute_purchase_totals(lines, invoice_discount, default_tax_rate)`** (`purchaseService.ts:
/// 47-72`) = G-21 `shared::totals::compute_invoice_totals(lines, discount, prices_include_tax =
/// false)` where each line's tax is the **active** live tax with `tax_id` (rate, category), else
/// `(default_rate, 'S')` — purchases have no "no tax" default category the way an untaxed sales
/// line does (`taxRateFor`, `purchaseService.ts:36-40`); the invoice discount is `pct` when
/// non-zero, else `amount` when non-zero, else none; `tax_rate` is `lines.is_empty() ?
/// default_tax_rate : round2(vat / (net == 0 ? 1 : net) * 100)`.
pub(crate) async fn compute_purchase_totals<C: ConnectionTrait>(
    conn: &C,
    lines: &[PurchaseTotalsLine],
    invoice_discount: Option<&InvoiceDiscount>,
    default_tax_rate: Decimal,
) -> TxResult<PurchaseTotals> {
    use crate::entities::org::taxes::{Column as TaxColumn, Entity as TaxEntity};

    let mut totals_lines = Vec::with_capacity(lines.len());
    for line in lines {
        let tax = match line.tax_id {
            Some(tax_id) => {
                let tax = TaxEntity::find_by_id(tax_id).filter(TaxColumn::Active.eq(true)).one(conn).await.map_err(AppError::from)?;
                match tax {
                    Some(t) => Some(TotalsTax { rate: t.rate, category: TaxCategory::from_str_or_other(&t.category) }),
                    None => Some(TotalsTax { rate: default_tax_rate, category: TaxCategory::Standard }),
                }
            }
            None => Some(TotalsTax { rate: default_tax_rate, category: TaxCategory::Standard }),
        };
        totals_lines.push(TotalsLineInput {
            qty: line.qty,
            unit_price: line.cost_price,
            discount: line.discount.unwrap_or(Decimal::ZERO),
            discount_is_pct: line.discount_is_pct.unwrap_or(false),
            tax,
        });
    }

    let discount_input = invoice_discount.and_then(|d| {
        if d.pct.map(|p| p != Decimal::ZERO).unwrap_or(false) {
            Some(InvoiceDiscountInput::Pct(d.pct.unwrap()))
        } else if d.amount.map(|a| a != Decimal::ZERO).unwrap_or(false) {
            Some(InvoiceDiscountInput::Amount(d.amount.unwrap()))
        } else {
            None
        }
    });

    let result = compute_invoice_totals(&totals_lines, discount_input, false);
    let tax_rate = if lines.is_empty() {
        default_tax_rate
    } else {
        let net = if result.net.is_zero() { Decimal::ONE } else { result.net };
        round2(result.vat / net * Decimal::ONE_HUNDRED)
    };

    Ok(PurchaseTotals { sub_total: result.net, tax_amount: result.vat, grand_total: result.gross, tax_rate })
}
