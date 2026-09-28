//! `domains::products::service::stock_lines` — shared helpers for every posting command in this
//! domain (06b-inventory.md §3 S-1/S-2, labels, `to_fixed2`). `pub(crate)` since 07-purchases reuses
//! `lock_line_products`/`to_fixed2` too (per the plan's cross-references).

use std::collections::{BTreeMap, HashSet};

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::catalog::products::{Column as ProductColumn, Entity as ProductEntity};
use crate::shared::stock::{self, LockedProduct};
use crate::utils::id::Id;
use crate::utils::money::{js_number_string, round2};

use super::super::dto::inventory::{StockAdjustmentLine, StockAdjustmentLineInput, StockAdjustmentType, StockInReason};

/// `TYPE_LABEL` (`inventory.ts:18`).
pub(crate) fn type_label(t: StockAdjustmentType) -> &'static str {
    match t {
        StockAdjustmentType::StockIn => "إدخال مخزون",
        StockAdjustmentType::Loss => "إتلاف/فقد",
        StockAdjustmentType::Stocktake => "جرد",
    }
}

/// `STOCK_IN_REASON_LABEL` (`inventory.ts:20-26`).
pub(crate) fn stock_in_reason_label(r: StockInReason) -> &'static str {
    match r {
        StockInReason::Opening => "رصيد افتتاحي",
        StockInReason::OwnerContribution => "مساهمة من المالك",
        StockInReason::Gift => "هدية / بضاعة مجانية من مورد",
        StockInReason::Found => "فائض تم العثور عليه",
        StockInReason::Other => "أخرى",
    }
}

/// `pub(crate) fn to_fixed2(d) = format!("{:.2}", round2(d))` — JS `toFixed(2)` (07 reuses it).
pub(crate) fn to_fixed2(d: Decimal) -> String {
    format!("{:.2}", round2(d))
}

/// **S-1 `lock_line_products(conn, ids)`**: plain `SELECT id FROM products WHERE id IN (…)`, then
/// `shared::stock::lock_products` (sorted `FOR UPDATE`). A missing id is simply absent from the
/// returned map, so callers raise `NOT_FOUND` at that line's position (keeping the mock's per-line
/// error order) while every check reads locked rows.
pub(crate) async fn lock_line_products<C: ConnectionTrait>(conn: &C, ids: &[Id]) -> TxResult<BTreeMap<Id, LockedProduct>> {
    let mut unique: Vec<Id> = ids.to_vec();
    unique.sort();
    unique.dedup();
    if unique.is_empty() {
        return Ok(BTreeMap::new());
    }
    let existing: Vec<Id> = ProductEntity::find().filter(ProductColumn::Id.is_in(unique)).all(conn).await.map_err(AppError::from)?.into_iter().map(|p| p.id).collect();
    stock::lock_products(conn, &existing).await
}

/// **S-2 `build_lines(ty, lines, locked, snapshot)`** — `buildLines` (`inventory.ts:30-66`), same
/// order: empty -> error; per line: missing product -> `NOT_FOUND` (checked against `locked`, which
/// only contains the ids S-1 actually found — a missing id raises here, at this line's position);
/// untracked -> message; dedupe by `(product, batchNo)` for a STOCK_IN of a `track_batches` product,
/// else by product id; STOCKTAKE / STOCK_IN / LOSS branches per the mock.
pub(crate) fn build_lines(
    ty: StockAdjustmentType,
    lines: &[StockAdjustmentLineInput],
    locked: &BTreeMap<Id, LockedProduct>,
    snapshot: Option<&BTreeMap<Id, Decimal>>,
) -> TxResult<Vec<StockAdjustmentLine>> {
    if lines.is_empty() {
        return Err(AppError::validation("أضف صنفاً واحداً على الأقل").into());
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::with_capacity(lines.len());

    for line in lines {
        let Some(product) = locked.get(&line.product_id) else {
            return Err(AppError::not_found("المنتج غير موجود").into());
        };
        let name = &product.model.name;

        if product.is_untracked() {
            return Err(AppError::validation(format!("\"{name}\" لا يُتتبع مخزونها")).into());
        }

        let dedupe_key = if matches!(ty, StockAdjustmentType::StockIn) && product.track_batches() {
            format!("{}::{}", line.product_id, line.batch_no.as_deref().unwrap_or(""))
        } else {
            line.product_id.to_string()
        };
        if !seen.insert(dedupe_key) {
            let batch_suffix = line.batch_no.as_deref().filter(|b| !b.is_empty()).map(|b| format!(" (تشغيلة {b})")).unwrap_or_default();
            return Err(AppError::validation(format!("\"{name}\"{batch_suffix} مكرر في القائمة")).into());
        }

        let system_qty = snapshot.and_then(|s| s.get(&line.product_id).copied()).unwrap_or(product.stock_qty());

        if matches!(ty, StockAdjustmentType::Stocktake) {
            let counted = line.counted_qty;
            let Some(counted) = counted.filter(|c| *c >= Decimal::ZERO) else {
                return Err(AppError::validation(format!("أدخل الكمية المعدودة لـ \"{name}\"")).into());
            };
            out.push(StockAdjustmentLine {
                product_id: line.product_id,
                system_qty: Some(system_qty),
                counted_qty: Some(counted),
                qty_change: round2(counted - system_qty),
                unit_cost: Some(product.cost_price()),
                batch_no: None,
                expiry_date: None,
            });
            continue;
        }

        let qty = line.qty_change;
        let Some(qty) = qty.filter(|q| *q > Decimal::ZERO) else {
            return Err(AppError::validation(format!("أدخل كمية صحيحة لـ \"{name}\"")).into());
        };
        if matches!(ty, StockAdjustmentType::Loss) && qty > product.stock_qty() {
            return Err(AppError::validation(format!("كمية الإتلاف لـ \"{name}\" أكبر من المتوفر ({})", js_number_string(product.stock_qty()))).into());
        }
        if matches!(ty, StockAdjustmentType::StockIn) && product.track_batches() && line.batch_no.as_deref().map(str::trim).unwrap_or("").is_empty() {
            return Err(AppError::validation(format!("أدخل رقم التشغيلة لـ \"{name}\" — هذا الصنف يتتبع التشغيلات وتاريخ الصلاحية")).into());
        }

        out.push(StockAdjustmentLine {
            product_id: line.product_id,
            system_qty: Some(system_qty),
            counted_qty: None,
            qty_change: if matches!(ty, StockAdjustmentType::Loss) { -qty } else { qty },
            unit_cost: Some(product.cost_price()),
            batch_no: line.batch_no.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string),
            expiry_date: line.expiry_date,
        });
    }

    Ok(out)
}
