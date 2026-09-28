//! `domains::products::service::batches` — batches & expiry report (06b-inventory.md §3 C-B1…C-B5).

use std::collections::BTreeMap;

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::AppError;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::catalog::product_batches::{Entity as BatchEntity, Model as BatchModel};
use crate::entities::catalog::products::Entity as ProductEntity;
use crate::entities::inventory::debit_note_drafts::{
    ActiveModel as DebitNoteDraftActiveModel, DebitNoteDraftLine as EntityDebitNoteDraftLine, DebitNoteDraftLines, Entity as DebitNoteDraftEntity,
};
use crate::entities::parties::parties::{Column as PartyColumn, Entity as PartyEntity};
use crate::shared::activity;
use crate::shared::numbering::{self, DocumentKind};
use crate::shared::stock::batches as batch_store;
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::route::RouteRef;

use super::super::dto::inventory::{DebitNoteDraft, DebitNoteDraftLine, DebitNoteDraftLineInput, DebitNoteDraftStatus, ExpiryBucket, ExpiryRow, ProductBatch};
use super::adjustments::ApprovalCheck;

fn to_dto(m: &BatchModel) -> ProductBatch {
    ProductBatch {
        id: m.id,
        product_id: m.product_id,
        batch_no: m.batch_no.clone(),
        expiry_date: m.expiry_date,
        qty: m.qty,
        unit_cost: m.unit_cost,
        supplier_id: m.supplier_id,
        received_date: m.received_date().key(),
        source_ref_id: m.source_ref_id,
        source_ref_number: m.source_ref_number.clone(),
    }
}

/// **C-B1 `get_batches(product_id)`** (`:164-167`).
pub async fn get_batches<C: ConnectionTrait>(conn: &C, product_id: Id) -> TxResult<Vec<ProductBatch>> {
    let rows = batch_store::active_batches(conn, product_id).await?;
    Ok(rows.iter().map(to_dto).collect())
}

fn expiry_bucket(expiry: chrono::NaiveDate, today: chrono::NaiveDate) -> (ExpiryBucket, Option<i64>) {
    let days = (expiry - today).num_days();
    if days < 0 {
        (ExpiryBucket::Expired, Some(days))
    } else if days <= 30 {
        (ExpiryBucket::Within30, Some(days))
    } else if days <= 60 {
        (ExpiryBucket::Within60, Some(days))
    } else if days <= 90 {
        (ExpiryBucket::Within90, Some(days))
    } else {
        (ExpiryBucket::Ok, Some(days))
    }
}

/// **C-B2 `get_expiry_report()`** (`:179-203`): `today = UTC_DATE()` (D-I8/Q-I11 — matches the
/// mock's UTC-based "today", not the business day).
pub async fn get_expiry_report<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<ExpiryRow>> {
    let stmt = sea_orm::Statement::from_string(conn.get_database_backend(), "SELECT UTC_DATE() AS today".to_string());
    let row = conn.query_one(stmt).await.map_err(AppError::from)?;
    let today: chrono::NaiveDate = match row {
        Some(r) => r.try_get("", "today").map_err(AppError::from)?,
        None => chrono::Utc::now().date_naive(),
    };

    let mut rows = BatchEntity::find()
        .filter(crate::entities::catalog::product_batches::Column::Qty.gt(rust_decimal::Decimal::new(1, 4)))
        .filter(crate::entities::catalog::product_batches::Column::ExpiryDate.is_not_null())
        .order_by_asc(crate::entities::catalog::product_batches::Column::CreatedAt)
        .order_by_asc(crate::entities::catalog::product_batches::Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    rows.retain(|b| b.expiry_date.is_some());

    let mut out = Vec::with_capacity(rows.len());
    for b in &rows {
        let expiry = b.expiry_date.expect("filtered non-null above");
        let (bucket, days_left) = expiry_bucket(expiry, today);
        if matches!(bucket, ExpiryBucket::Ok) {
            continue;
        }
        let product = ProductEntity::find_by_id(b.product_id).one(conn).await.map_err(AppError::from)?;
        let supplier = match b.supplier_id {
            Some(sid) => PartyEntity::find().filter(PartyColumn::Id.eq(sid)).one(conn).await.map_err(AppError::from)?,
            None => None,
        };
        out.push(ExpiryRow {
            id: b.id,
            product_id: b.product_id,
            batch_no: b.batch_no.clone(),
            expiry_date: b.expiry_date,
            qty: b.qty,
            unit_cost: b.unit_cost,
            supplier_id: b.supplier_id,
            received_date: b.received_date().key(),
            source_ref_id: b.source_ref_id,
            source_ref_number: b.source_ref_number.clone(),
            product_name: product.as_ref().map(|p| p.name.clone()).unwrap_or_else(|| "—".to_string()),
            product_sku: product.map(|p| p.sku).unwrap_or_default(),
            supplier_name: supplier.map(|s| s.name),
            bucket,
            days_left,
        });
    }
    out.sort_by_key(|r| r.days_left.unwrap_or(0));
    Ok(out)
}

/// **C-B3 `write_off_expired_batches(batch_ids, note)`** (`inventory.ts:397-421`, Q-I3: the LOSS
/// consumes FEFO, not necessarily the selected ids).
pub async fn write_off_expired_batches<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::UndoRegistry,
    batch_ids: Vec<Id>,
    note: Option<String>,
) -> TxResult<super::super::dto::inventory::StockAdjustment> {
    if batch_ids.is_empty() {
        return Err(AppError::validation("اختر تشغيلة واحدة على الأقل").into());
    }

    // Read the batches (plain) to find their products, lock those products (serializes against
    // concurrent stock writers), then re-read the batches' qty (now stable under the lock).
    let initial: Vec<BatchModel> = BatchEntity::find()
        .filter(crate::entities::catalog::product_batches::Column::Id.is_in(batch_ids.clone()))
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let product_ids: Vec<Id> = initial.iter().map(|b| b.product_id).collect();
    let _locked = super::stock_lines::lock_line_products(conn, &product_ids).await?;

    let refreshed: Vec<BatchModel> = BatchEntity::find()
        .filter(crate::entities::catalog::product_batches::Column::Id.is_in(batch_ids))
        .all(conn)
        .await
        .map_err(AppError::from)?;

    let mut by_product: BTreeMap<Id, rust_decimal::Decimal> = BTreeMap::new();
    let mut order: Vec<Id> = Vec::new();
    for b in &refreshed {
        if b.qty <= rust_decimal::Decimal::ZERO {
            continue;
        }
        if !by_product.contains_key(&b.product_id) {
            order.push(b.product_id);
        }
        let entry = by_product.entry(b.product_id).or_insert(rust_decimal::Decimal::ZERO);
        *entry = round2(*entry + b.qty);
    }
    if by_product.is_empty() {
        return Err(AppError::validation("لا توجد كميات متبقية في التشغيلات المحددة").into());
    }

    let lines: Vec<super::super::dto::inventory::StockAdjustmentLineInput> = order
        .into_iter()
        .map(|pid| super::super::dto::inventory::StockAdjustmentLineInput {
            product_id: pid,
            qty_change: Some(by_product[&pid]),
            counted_qty: None,
            batch_no: None,
            expiry_date: None,
        })
        .collect();

    let now_date = crate::utils::dates::DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) };
    let input = super::super::dto::inventory::StockAdjustmentInput {
        r#type: super::super::dto::inventory::StockAdjustmentType::Loss,
        date: now_date.key(),
        note: Some(note.unwrap_or_else(|| "إتلاف — بضاعة منتهية الصلاحية".to_string())),
        reason: None,
        offset_account_id: None,
        lines,
        approved_by: None,
    };

    super::adjustments::record_stock_adjustment(conn, cx, registry, input, false, ApprovalCheck::none()).await
}

/// **C-B4 `return_batches_to_supplier(supplier_id, lines, note)`** (`:432-446`).
pub async fn return_batches_to_supplier<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::UndoRegistry,
    supplier_id: Id,
    lines: Vec<DebitNoteDraftLineInput>,
    note: Option<String>,
) -> TxResult<DebitNoteDraft> {
    if lines.is_empty() {
        return Err(AppError::validation("اختر تشغيلة واحدة على الأقل للإرجاع").into());
    }

    let number = numbering::next_number(conn, DocumentKind::DebitNoteDraft).await?;
    let now_date = crate::utils::dates::DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) };
    let now = cx.clock.now;
    let id = Id::new();

    let entity_lines: Vec<EntityDebitNoteDraftLine> = lines
        .iter()
        .map(|l| EntityDebitNoteDraftLine { product_id: l.product_id, batch_id: l.batch_id, qty: l.qty, unit_cost: l.unit_cost })
        .collect();

    let am = DebitNoteDraftActiveModel {
        id: Set(id),
        number: Set(number.clone()),
        supplier_id: Set(supplier_id),
        date_day: Set(now_date.day),
        date_instant: Set(now_date.instant),
        date_key: Set(now_date.key()),
        status: Set("DRAFT".to_string()),
        lines: Set(DebitNoteDraftLines(entity_lines)),
        note: Set(note.clone()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let inserted = am.insert(conn).await.map_err(AppError::from)?;
    let inserted_date_key = inserted.date().key();

    activity::log(
        conn,
        cx,
        registry,
        crate::entities::platform::activity::ActivityKind::Stock,
        format!("مسودة إرجاع للمورد {number} — بانتظار مرحلة المشتريات (Phase 8)"),
        Some(now_date),
        Some(RouteRef::list("products")),
    )
    .await?;

    Ok(DebitNoteDraft {
        id: inserted.id,
        number: inserted.number,
        supplier_id: inserted.supplier_id,
        date: inserted_date_key,
        status: DebitNoteDraftStatus::Draft,
        lines: lines.iter().map(|l| DebitNoteDraftLine { product_id: l.product_id, batch_id: l.batch_id, qty: l.qty, unit_cost: l.unit_cost }).collect(),
        note,
    })
}

/// **C-B5 `get_debit_note_drafts()`** (`pub(crate)`, `:226-229`; 07 reuses it).
pub(crate) async fn get_debit_note_drafts<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<DebitNoteDraft>> {
    let rows = DebitNoteDraftEntity::find()
        .order_by_asc(crate::entities::inventory::debit_note_drafts::Column::CreatedAt)
        .order_by_asc(crate::entities::inventory::debit_note_drafts::Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(rows
        .iter()
        .map(|m| DebitNoteDraft {
            id: m.id,
            number: m.number.clone(),
            supplier_id: m.supplier_id,
            date: m.date().key(),
            status: DebitNoteDraftStatus::Draft,
            lines: m.lines.0.iter().map(|l| DebitNoteDraftLine { product_id: l.product_id, batch_id: l.batch_id, qty: l.qty, unit_cost: l.unit_cost }).collect(),
            note: m.note.clone(),
        })
        .collect())
}
