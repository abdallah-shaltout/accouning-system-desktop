//! `stock_adjustments, stock_adjustment_lines, stock_counts, stock_count_lines, stock_transfers,
//! stock_transfer_lines, stock_movements, debit_note_drafts`.

use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait};

use crate::core::error::AppError;
use crate::core::tx::{TxError, TxResult};
use crate::entities::inventory::debit_note_drafts::{ActiveModel as DebitNoteDraftActiveModel, DebitNoteDraftLine, DebitNoteDraftLines};
use crate::entities::inventory::stock_adjustment_lines::ActiveModel as StockAdjustmentLineActiveModel;
use crate::entities::inventory::stock_adjustments::ActiveModel as StockAdjustmentActiveModel;
use crate::entities::inventory::stock_count_lines::ActiveModel as StockCountLineActiveModel;
use crate::entities::inventory::stock_counts::ActiveModel as StockCountActiveModel;
use crate::entities::inventory::stock_movements::ActiveModel as StockMovementActiveModel;
use crate::entities::inventory::stock_transfer_lines::ActiveModel as StockTransferLineActiveModel;
use crate::entities::inventory::stock_transfers::ActiveModel as StockTransferActiveModel;
use crate::infrastructure::import::idmap::{is_freetext_product_id, IdMap};
use crate::infrastructure::import::model::{DebitNoteDraftV1, StockAdjustmentV1, StockCountV1, StockMovementV1, StockTransferV1};
use crate::infrastructure::import::tables::{parse_doc_date, resolve_created_at, strict_ref};
use crate::utils::id::Id;
use crate::utils::money::{round2, round4, round_qty};

fn resolve_product(id_map: &IdMap, old: &str) -> Option<Id> {
    if is_freetext_product_id(old) {
        None
    } else {
        id_map.resolve(old)
    }
}

pub async fn insert_stock_adjustments<C: ConnectionTrait>(
    conn: &C,
    rows: &[StockAdjustmentV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let (day, instant) =
            parse_doc_date(&row.date, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في stock_adjustments"))?;
        let model = StockAdjustmentActiveModel {
            id: Set(id),
            number: Set(row.number.clone()),
            r#type: Set(row.kind.clone()),
            date_day: Set(day),
            date_instant: Set(instant),
            date_key: sea_orm::ActiveValue::NotSet,
            status: Set(row.status.clone()),
            note: Set(row.note.clone()),
            reason: Set(row.reason.clone()),
            offset_account_id: Set(strict_ref(id_map, row.offset_account_id.as_deref(), "stock_adjustments")?),
            approved_by: Set(strict_ref(id_map, row.approved_by.as_deref(), "stock_adjustments")?),
            approved_at: Set(row.approved_at.as_deref().and_then(crate::infrastructure::import::model::parse_instant)),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            deleted_at: Set(None),
            sync_status: Set("local".to_string()),
        };
        model.insert(conn).await.map_err(TxError::from)?;

        for (li, line) in row.lines.iter().enumerate() {
            let Some(product_id) = resolve_product(id_map, &line.product_id) else {
                return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في stock_adjustment_lines")));
            };
            let qty_change = round_qty(line.qty_change);
            let unit_cost = line.unit_cost.map(round4);
            if qty_change != line.qty_change {
                *rounded += 1;
            }
            let line_model = StockAdjustmentLineActiveModel {
                id: Set(Id::new()),
                stock_adjustment_id: Set(id),
                position: Set(li as i16),
                product_id: Set(product_id),
                system_qty: Set(line.system_qty.map(round_qty)),
                counted_qty: Set(line.counted_qty.map(round_qty)),
                qty_change: Set(qty_change),
                unit_cost: Set(unit_cost),
                batch_no: Set(line.batch_no.clone()),
                expiry_date: Set(line.expiry_date.as_deref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())),
            };
            line_model.insert(conn).await.map_err(TxError::from)?;
        }
    }
    Ok(())
}

pub async fn insert_stock_counts<C: ConnectionTrait>(
    conn: &C,
    rows: &[StockCountV1],
    id_map: &IdMap,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(started_by) = id_map.resolve(&row.started_by) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في stock_counts")));
        };
        let started_at =
            crate::infrastructure::import::model::parse_instant(&row.started_at).ok_or_else(|| AppError::validation("تاريخ غير صالح في stock_counts"))?;
        let model = StockCountActiveModel {
            id: Set(id),
            number: Set(row.number.clone()),
            status: Set(row.status.clone()),
            scope: Set(row.scope.clone()),
            category_id: Set(strict_ref(id_map, row.category_id.as_deref(), "stock_counts")?),
            location: Set(row.location.clone()),
            blind: Set(row.blind),
            started_at: Set(started_at),
            started_by: Set(started_by),
            note: Set(row.note.clone()),
            adjustment_id: Set(strict_ref(id_map, row.adjustment_id.as_deref(), "stock_counts")?),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
        };
        model.insert(conn).await.map_err(TxError::from)?;

        for (li, line) in row.lines.iter().enumerate() {
            let Some(product_id) = resolve_product(id_map, &line.product_id) else { continue };
            let system_qty = round_qty(line.system_qty);
            let unit_cost = round4(line.unit_cost);
            if system_qty != line.system_qty || unit_cost != line.unit_cost {
                *rounded += 1;
            }
            let line_model = StockCountLineActiveModel {
                id: Set(Id::new()),
                stock_count_id: Set(id),
                position: Set(li as i16),
                product_id: Set(product_id),
                system_qty: Set(system_qty),
                counted_qty: Set(line.counted_qty.map(round_qty)),
                unit_cost: Set(unit_cost),
            };
            line_model.insert(conn).await.map_err(TxError::from)?;
        }
    }
    Ok(())
}

pub async fn insert_stock_transfers<C: ConnectionTrait>(
    conn: &C,
    rows: &[StockTransferV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let (Some(from_branch_id), Some(to_branch_id)) = (id_map.resolve(&row.from_branch_id), id_map.resolve(&row.to_branch_id)) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في stock_transfers")));
        };
        let (day, instant) = parse_doc_date(&row.date, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في stock_transfers"))?;
        let sent_at = row.sent_at.as_deref().and_then(|s| parse_doc_date(s, tz));
        let received_at = row.received_at.as_deref().and_then(|s| parse_doc_date(s, tz));
        let rejected_at = row.rejected_at.as_deref().and_then(|s| parse_doc_date(s, tz));

        let model = StockTransferActiveModel {
            id: Set(id),
            number: Set(row.number.clone()),
            from_branch_id: Set(from_branch_id),
            to_branch_id: Set(to_branch_id),
            status: Set(row.status.clone()),
            date_day: Set(day),
            date_instant: Set(instant),
            date_key: sea_orm::ActiveValue::NotSet,
            sent_at_day: Set(sent_at.map(|(d, _)| d)),
            sent_at_instant: Set(sent_at.and_then(|(_, i)| i)),
            sent_at_key: sea_orm::ActiveValue::NotSet,
            received_at_day: Set(received_at.map(|(d, _)| d)),
            received_at_instant: Set(received_at.and_then(|(_, i)| i)),
            received_at_key: sea_orm::ActiveValue::NotSet,
            rejected_at_day: Set(rejected_at.map(|(d, _)| d)),
            rejected_at_instant: Set(rejected_at.and_then(|(_, i)| i)),
            rejected_at_key: sea_orm::ActiveValue::NotSet,
            note: Set(row.note.clone()),
            sent_by: Set(strict_ref(id_map, row.sent_by.as_deref(), "stock_transfers")?),
            received_by: Set(strict_ref(id_map, row.received_by.as_deref(), "stock_transfers")?),
            rejected_by: Set(strict_ref(id_map, row.rejected_by.as_deref(), "stock_transfers")?),
            reject_reason: Set(row.reject_reason.clone()),
            shortage_value: Set(row.shortage_value.map(round2)),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            sync_status: Set("local".to_string()),
        };
        model.insert(conn).await.map_err(TxError::from)?;

        for (li, line) in row.lines.iter().enumerate() {
            let Some(product_id) = resolve_product(id_map, &line.product_id) else { continue };
            let qty = round_qty(line.qty);
            if qty != line.qty {
                *rounded += 1;
            }
            let line_model = StockTransferLineActiveModel {
                id: Set(Id::new()),
                stock_transfer_id: Set(id),
                position: Set(li as i16),
                product_id: Set(product_id),
                qty: Set(qty),
                // The product's own `ProductUnit.id` (a free string kept verbatim in `products.units`),
                // never an id-map key — copied as is (m0020).
                unit_id: Set(line.unit_id.clone()),
                unit_factor: Set(line.unit_factor.map(round4)),
                batch_id: Set(strict_ref(id_map, line.batch_id.as_deref(), "stock_transfer_lines")?),
                batch_no: Set(line.batch_no.clone()),
                received_qty: Set(line.received_qty.map(round_qty)),
                unit_cost: Set(line.unit_cost.map(round4)),
            };
            line_model.insert(conn).await.map_err(TxError::from)?;
        }
    }
    Ok(())
}

/// Append-only ledger row — inserted after every other document table so its polymorphic `ref_id`
/// (B-1: a purchase/sale/adjustment/transfer id) always resolves through `resolve_or_mint` to a
/// value already assigned by the time this runs (every document table precedes `stock_movements`
/// in `IMPORT_ORDER`).
pub async fn insert_stock_movements<C: ConnectionTrait>(
    conn: &C,
    rows: &[StockMovementV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(product_id) = resolve_product(id_map, &row.product_id) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في stock_movements")));
        };
        let (day, instant) = parse_doc_date(&row.date, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في stock_movements"))?;
        let qty_change = round_qty(row.qty_change);
        let value_change = round2(row.value_change);
        if qty_change != row.qty_change || value_change != row.value_change {
            *rounded += 1;
        }
        let model = StockMovementActiveModel {
            id: Set(id),
            date_day: Set(day),
            date_instant: Set(instant),
            date_key: sea_orm::ActiveValue::NotSet,
            product_id: Set(product_id),
            qty_change: Set(qty_change),
            value_change: Set(value_change),
            reason: Set(row.reason.clone()),
            ref_id: Set(id_map.resolve_or_mint(&row.ref_id)),
            ref_number: Set(row.ref_number.clone()),
            balance_after: Set(row.balance_after.map(round_qty)),
            batch_id: Set(strict_ref(id_map, row.batch_id.as_deref(), "stock_movements")?),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

pub async fn insert_debit_note_drafts<C: ConnectionTrait>(
    conn: &C,
    rows: &[DebitNoteDraftV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(supplier_id) = id_map.resolve(&row.supplier_id) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في debit_note_drafts")));
        };
        let (day, instant) = parse_doc_date(&row.date, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في debit_note_drafts"))?;

        let mut lines = Vec::with_capacity(row.lines.len());
        for line in &row.lines {
            let (Some(product_id), Some(batch_id)) = (id_map.resolve(&line.product_id), id_map.resolve(&line.batch_id)) else {
                continue;
            };
            let qty = round_qty(line.qty);
            let unit_cost = round4(line.unit_cost);
            if qty != line.qty || unit_cost != line.unit_cost {
                *rounded += 1;
            }
            lines.push(DebitNoteDraftLine { product_id, batch_id, qty, unit_cost });
        }

        let model = DebitNoteDraftActiveModel {
            id: Set(id),
            number: Set(row.number.clone()),
            supplier_id: Set(supplier_id),
            date_day: Set(day),
            date_instant: Set(instant),
            date_key: sea_orm::ActiveValue::NotSet,
            status: Set("DRAFT".to_string()),
            lines: Set(DebitNoteDraftLines(lines)),
            note: Set(row.note.clone()),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}
