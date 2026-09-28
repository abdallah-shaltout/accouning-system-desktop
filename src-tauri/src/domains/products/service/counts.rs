//! `domains::products::service::counts` — Stocktake v2 (06b-inventory.md §3 C-K1…C-K6).

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::catalog::products::{Column as ProductColumn, Entity as ProductEntity};
use crate::entities::inventory::stock_adjustment_lines::ActiveModel as AdjustmentLineActiveModel;
use crate::entities::inventory::stock_adjustments::ActiveModel as AdjustmentActiveModel;
use crate::entities::inventory::stock_count_lines::{ActiveModel as CountLineActiveModel, Column as CountLineColumn, Entity as CountLineEntity};
use crate::entities::inventory::stock_counts::{ActiveModel as CountActiveModel, Column as CountColumn, Entity as CountEntity, Model as CountModel};
use crate::shared::activity;
use crate::shared::numbering::{self, DocumentKind};
use crate::utils::dates::DocDate;
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::route::RouteRef;

use super::super::dto::inventory::{StockAdjustment, StockCount, StockCountInput, StockCountLine, StockCountScope, StockCountStatus};
use super::adjustments::{self, adjustment_type_as_str, status_as_str as adjustment_status_as_str};
use super::stock_lines::{build_lines, lock_line_products};

fn parse_scope(s: &str) -> StockCountScope {
    match s {
        "all" => StockCountScope::All,
        "category" => StockCountScope::Category,
        "location" => StockCountScope::Location,
        other => panic!("stock_counts.scope column holds an unrecognized value: {other:?}"),
    }
}

fn scope_as_str(s: StockCountScope) -> &'static str {
    match s {
        StockCountScope::All => "all",
        StockCountScope::Category => "category",
        StockCountScope::Location => "location",
    }
}

fn parse_status(s: &str) -> StockCountStatus {
    match s {
        "OPEN" => StockCountStatus::Open,
        "REVIEW" => StockCountStatus::Review,
        "COMPLETED" => StockCountStatus::Completed,
        other => panic!("stock_counts.status column holds an unrecognized value: {other:?}"),
    }
}

fn status_as_str(s: StockCountStatus) -> &'static str {
    match s {
        StockCountStatus::Open => "OPEN",
        StockCountStatus::Review => "REVIEW",
        StockCountStatus::Completed => "COMPLETED",
    }
}

async fn to_dto<C: ConnectionTrait>(conn: &C, m: &CountModel) -> TxResult<StockCount> {
    let lines = CountLineEntity::find().filter(CountLineColumn::StockCountId.eq(m.id)).order_by_asc(CountLineColumn::Position).all(conn).await.map_err(AppError::from)?;
    Ok(StockCount {
        id: m.id,
        number: m.number.clone(),
        status: parse_status(&m.status),
        scope: parse_scope(&m.scope),
        category_id: m.category_id,
        location: m.location.clone(),
        blind: m.blind,
        started_at: crate::utils::dates::format_iso_ms(m.started_at),
        started_by: m.started_by,
        lines: lines
            .iter()
            .map(|l| StockCountLine { product_id: l.product_id, system_qty: l.system_qty, counted_qty: l.counted_qty, unit_cost: l.unit_cost })
            .collect(),
        note: m.note.clone(),
        adjustment_id: m.adjustment_id,
    })
}

// --- C-K1 reads -----------------------------------------------------------------------------------

pub async fn get_stock_counts<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<StockCount>> {
    let rows = CountEntity::find().order_by_desc(CountColumn::StartedAt).order_by_asc(CountColumn::CreatedAt).order_by_asc(CountColumn::Id).all(conn).await.map_err(AppError::from)?;
    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        out.push(to_dto(conn, row).await?);
    }
    Ok(out)
}

pub async fn get_stock_count<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<StockCount> {
    let row = CountEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("الجرد غير موجود"))?;
    to_dto(conn, &row).await
}

/// In-scope products for a count (`scopedProductIds`, `inventory.ts:293-302`): `type='product' &&
/// COALESCE(stock_mode,'tracked') <> 'none' && active`, narrowed by SQL for `type`/`stock_mode`/
/// `active` (an index-friendly prefilter), then the scope's exact predicate applied in Rust — same
/// "narrow in SQL, exact predicate in Rust" pattern as `products::get_products`'s search filter.
async fn scoped_product_ids<C: ConnectionTrait>(conn: &C, input: &StockCountInput) -> TxResult<Vec<crate::entities::catalog::products::Model>> {
    let candidates = ProductEntity::find()
        .filter(ProductColumn::Type.eq("product"))
        .filter(sea_orm::Condition::any().add(ProductColumn::StockMode.is_null()).add(ProductColumn::StockMode.ne("none")))
        .filter(ProductColumn::Active.eq(true))
        .order_by_asc(ProductColumn::CreatedAt)
        .order_by_asc(ProductColumn::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;

    let filtered = match input.scope {
        StockCountScope::All => candidates,
        // NULL-safe compare (Q-I12): no id sent matches uncategorised products too, mirroring the
        // mock's `p.categoryId === input.categoryId` where both sides may be `undefined`.
        StockCountScope::Category => candidates.into_iter().filter(|p| p.category_id == input.category_id).collect(),
        StockCountScope::Location => {
            let wanted = input.location.clone().unwrap_or_default();
            candidates.into_iter().filter(|p| p.shelf_location.clone().unwrap_or_default() == wanted).collect()
        }
    };
    Ok(filtered)
}

/// **C-K2 `create_stock_count(input)`** (`:293-327`).
pub async fn create_stock_count<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, input: StockCountInput) -> TxResult<StockCount> {
    let products = scoped_product_ids(conn, &input).await?;
    if products.is_empty() {
        return Err(AppError::validation("لا توجد أصناف ضمن نطاق الجرد المحدد").into());
    }

    let number = numbering::next_number(conn, DocumentKind::StockCount).await?;
    let now = cx.clock.now;
    let id = Id::new();
    let actor_id = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;

    let am = CountActiveModel {
        id: Set(id),
        number: Set(number.clone()),
        status: Set(status_as_str(StockCountStatus::Open).to_string()),
        scope: Set(scope_as_str(input.scope).to_string()),
        category_id: Set(input.category_id),
        location: Set(input.location.clone()),
        blind: Set(input.blind),
        started_at: Set(now),
        started_by: Set(actor_id),
        note: Set(input.note.clone()),
        adjustment_id: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let inserted = am.insert(conn).await.map_err(AppError::from)?;

    for (i, p) in products.iter().enumerate() {
        let line = CountLineActiveModel {
            id: Set(Id::new()),
            stock_count_id: Set(id),
            position: Set(i as i16),
            product_id: Set(p.id),
            system_qty: Set(p.stock_qty),
            counted_qty: Set(None),
            unit_cost: Set(p.cost_price),
        };
        line.insert(conn).await.map_err(AppError::from)?;
    }

    let now_date = DocDate { day: cx.clock.today(), instant: Some(now) };
    let blind_suffix = if input.blind { " (أعمى)" } else { "" };
    activity::log(
        conn,
        cx,
        registry,
        crate::entities::platform::activity::ActivityKind::Stock,
        format!("بدء جرد {number}{blind_suffix} — {} صنف", products.len()),
        Some(now_date),
        Some(RouteRef::detail("count", id.to_string())),
    )
    .await?;

    to_dto(conn, &inserted).await
}

/// **C-K3 `update_stock_count_line(count_id, product_id, qty, delta)`** (`:330-340`).
pub async fn update_stock_count_line<C: ConnectionTrait>(conn: &C, count_id: Id, product_id: Id, qty: Decimal, delta: bool) -> TxResult<StockCount> {
    lock::for_update_by_id(conn, "stock_counts", &count_id.to_string()).await.map_err(AppError::from)?;
    let count = CountEntity::find_by_id(count_id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("الجرد غير موجود"))?;
    if parse_status(&count.status) != StockCountStatus::Open {
        return Err(AppError::validation("لا يمكن تعديل جرد غير مفتوح").into());
    }
    let line = CountLineEntity::find()
        .filter(CountLineColumn::StockCountId.eq(count_id))
        .filter(CountLineColumn::ProductId.eq(product_id))
        .one(conn)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::validation("الصنف خارج نطاق هذا الجرد"))?;

    let new_counted = if delta { round2(line.counted_qty.unwrap_or(Decimal::ZERO) + qty) } else { round2(qty) };
    let mut am: CountLineActiveModel = line.into();
    am.counted_qty = Set(Some(new_counted));
    am.update(conn).await.map_err(AppError::from)?;

    to_dto(conn, &count).await
}

/// **C-K4 `submit_count_for_review`** (`:343-350`).
pub async fn submit_count_for_review<C: ConnectionTrait>(conn: &C, count_id: Id) -> TxResult<StockCount> {
    lock::for_update_by_id(conn, "stock_counts", &count_id.to_string()).await.map_err(AppError::from)?;
    let count = CountEntity::find_by_id(count_id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("الجرد غير موجود"))?;
    if parse_status(&count.status) != StockCountStatus::Open {
        return Err(AppError::validation("الجرد ليس في حالة مفتوحة").into());
    }
    let lines = CountLineEntity::find().filter(CountLineColumn::StockCountId.eq(count_id)).all(conn).await.map_err(AppError::from)?;
    if lines.iter().any(|l| l.counted_qty.is_none()) {
        return Err(AppError::validation("أكمل عدّ جميع الأصناف قبل المتابعة للمراجعة").into());
    }
    let mut am: CountActiveModel = count.into();
    am.status = Set(status_as_str(StockCountStatus::Review).to_string());
    let updated = am.update(conn).await.map_err(AppError::from)?;
    to_dto(conn, &updated).await
}

/// **C-K5 `resume_counting`** (`:352-358`).
pub async fn resume_counting<C: ConnectionTrait>(conn: &C, count_id: Id) -> TxResult<StockCount> {
    lock::for_update_by_id(conn, "stock_counts", &count_id.to_string()).await.map_err(AppError::from)?;
    let count = CountEntity::find_by_id(count_id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("الجرد غير موجود"))?;
    if parse_status(&count.status) != StockCountStatus::Review {
        return Err(AppError::validation("الجرد ليس قيد المراجعة").into());
    }
    let mut am: CountActiveModel = count.into();
    am.status = Set(status_as_str(StockCountStatus::Open).to_string());
    let updated = am.update(conn).await.map_err(AppError::from)?;
    to_dto(conn, &updated).await
}

/// **C-K6 `complete_stock_count(count_id)`** (`:361-388`).
pub async fn complete_stock_count<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, count_id: Id) -> TxResult<StockAdjustment> {
    lock::for_update_by_id(conn, "stock_counts", &count_id.to_string()).await.map_err(AppError::from)?;
    let count = CountEntity::find_by_id(count_id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::validation("يجب إرسال الجرد للمراجعة أولاً"))?;
    if parse_status(&count.status) != StockCountStatus::Review {
        return Err(AppError::validation("يجب إرسال الجرد للمراجعة أولاً").into());
    }

    let lines = CountLineEntity::find().filter(CountLineColumn::StockCountId.eq(count_id)).order_by_asc(CountLineColumn::Position).all(conn).await.map_err(AppError::from)?;
    let snapshot: std::collections::BTreeMap<Id, Decimal> = lines.iter().map(|l| (l.product_id, l.system_qty)).collect();

    let product_ids: Vec<Id> = lines.iter().map(|l| l.product_id).collect();
    let mut locked = lock_line_products(conn, &product_ids).await?;

    let line_inputs: Vec<super::super::dto::inventory::StockAdjustmentLineInput> = lines
        .iter()
        .map(|l| super::super::dto::inventory::StockAdjustmentLineInput {
            product_id: l.product_id,
            qty_change: None,
            counted_qty: l.counted_qty,
            batch_no: None,
            expiry_date: None,
        })
        .collect();
    let ty = super::super::dto::inventory::StockAdjustmentType::Stocktake;
    let built_lines = build_lines(ty, &line_inputs, &locked, Some(&snapshot))?;

    // `next_number(Adjustment)` is burned AFTER S-2 (the mock's own ordering, `:369` — a rollback
    // makes the burned number invisible, since numbering is itself transactional/gapless).
    let number = numbering::next_number(conn, DocumentKind::Adjustment).await?;
    let now_date = DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) };
    let note = format!("جرد {}{}", count.number, count.note.as_deref().map(|n| format!(" — {n}")).unwrap_or_default());
    let adj_id = Id::new();

    let adj_am = AdjustmentActiveModel {
        id: Set(adj_id),
        number: Set(number.clone()),
        r#type: Set(adjustment_type_as_str(ty).to_string()),
        date_day: Set(now_date.day),
        date_instant: Set(now_date.instant),
        date_key: Set(now_date.key()),
        status: Set(adjustment_status_as_str(super::super::dto::inventory::StockAdjustmentStatus::Completed).to_string()),
        note: Set(Some(note)),
        reason: Set(None),
        offset_account_id: Set(None),
        approved_by: Set(None),
        approved_at: Set(None),
        created_at: Set(cx.clock.now),
        updated_at: Set(cx.clock.now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    let inserted_adj = adj_am.insert(conn).await.map_err(AppError::from)?;

    for (i, line) in built_lines.iter().enumerate() {
        let line_am = AdjustmentLineActiveModel {
            id: Set(Id::new()),
            stock_adjustment_id: Set(adj_id),
            position: Set(i as i16),
            product_id: Set(line.product_id),
            system_qty: Set(line.system_qty),
            counted_qty: Set(line.counted_qty),
            qty_change: Set(line.qty_change),
            unit_cost: Set(line.unit_cost),
            batch_no: Set(line.batch_no.clone()),
            expiry_date: Set(line.expiry_date),
        };
        line_am.insert(conn).await.map_err(AppError::from)?;
    }

    adjustments::post_adjustment(conn, cx, &inserted_adj, &built_lines, &mut locked).await?;

    let mut count_am: CountActiveModel = count.into();
    count_am.status = Set(status_as_str(StockCountStatus::Completed).to_string());
    count_am.adjustment_id = Set(Some(adj_id));
    count_am.update(conn).await.map_err(AppError::from)?;

    activity::log(
        conn,
        cx,
        registry,
        crate::entities::platform::activity::ActivityKind::Stock,
        format!("اعتماد نتيجة الجرد {number}"),
        Some(now_date),
        Some(RouteRef::detail("adjustment", adj_id.to_string())),
    )
    .await?;
    cx.touch(crate::core::events::ChangeCategory::Catalog);

    adjustments::to_dto(conn, &inserted_adj).await
}
