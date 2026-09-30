//! `domains::products::service::adjustments` — stock adjustments (06b-inventory.md §3 S-3/S-4/S-5,
//! C-A1…C-A4).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::inventory::stock_adjustment_lines::{ActiveModel as LineActiveModel, Column as LineColumn, Entity as LineEntity};
use crate::entities::inventory::stock_adjustments::{ActiveModel as AdjustmentActiveModel, Column as AdjustmentColumn, Entity as AdjustmentEntity, Model as AdjustmentModel};
use crate::entities::journal::journal_entries::{Column as JournalColumn, Entity as JournalEntity, JournalEntryType};
use crate::entities::platform::activity::ActivityKind;
use crate::shared::activity;
use crate::shared::ledger::accounts::{account_by_id, SystemRole};
use crate::shared::ledger::post::{self, AccountRef, PostJournal, PostingLine};
use crate::shared::numbering::{self, DocumentKind};
use crate::shared::stock::{self, batches, LockedProduct, StockRef};
use crate::utils::dates::{DocDate, RawDocDate};
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::route::RouteRef;

use super::super::dto::inventory::{
    StockAdjustment, StockAdjustmentDetail, StockAdjustmentInput, StockAdjustmentLine, StockAdjustmentStatus, StockAdjustmentType, StockInReason,
};
use super::stock_lines::{build_lines, lock_line_products, stock_in_reason_label, to_fixed2, type_label};

fn parse_adjustment_type(s: &str) -> StockAdjustmentType {
    match s {
        "STOCK_IN" => StockAdjustmentType::StockIn,
        "LOSS" => StockAdjustmentType::Loss,
        "STOCKTAKE" => StockAdjustmentType::Stocktake,
        other => panic!("stock_adjustments.type column holds an unrecognized value: {other:?}"),
    }
}

pub(crate) fn adjustment_type_as_str(t: StockAdjustmentType) -> &'static str {
    match t {
        StockAdjustmentType::StockIn => "STOCK_IN",
        StockAdjustmentType::Loss => "LOSS",
        StockAdjustmentType::Stocktake => "STOCKTAKE",
    }
}

fn parse_status(s: &str) -> StockAdjustmentStatus {
    match s {
        "DRAFT" => StockAdjustmentStatus::Draft,
        "COMPLETED" => StockAdjustmentStatus::Completed,
        other => panic!("stock_adjustments.status column holds an unrecognized value: {other:?}"),
    }
}

pub(crate) fn status_as_str(s: StockAdjustmentStatus) -> &'static str {
    match s {
        StockAdjustmentStatus::Draft => "DRAFT",
        StockAdjustmentStatus::Completed => "COMPLETED",
    }
}

fn parse_reason(s: Option<&str>) -> Option<StockInReason> {
    match s {
        None => None,
        Some("opening") => Some(StockInReason::Opening),
        Some("owner_contribution") => Some(StockInReason::OwnerContribution),
        Some("gift") => Some(StockInReason::Gift),
        Some("found") => Some(StockInReason::Found),
        Some("other") => Some(StockInReason::Other),
        Some(other) => panic!("stock_adjustments.reason column holds an unrecognized value: {other:?}"),
    }
}

fn reason_as_str(r: Option<StockInReason>) -> Option<&'static str> {
    r.map(|r| match r {
        StockInReason::Opening => "opening",
        StockInReason::OwnerContribution => "owner_contribution",
        StockInReason::Gift => "gift",
        StockInReason::Found => "found",
        StockInReason::Other => "other",
    })
}

fn line_to_dto(l: &crate::entities::inventory::stock_adjustment_lines::Model) -> StockAdjustmentLine {
    StockAdjustmentLine {
        product_id: l.product_id,
        system_qty: l.system_qty,
        counted_qty: l.counted_qty,
        qty_change: l.qty_change,
        unit_cost: l.unit_cost,
        batch_no: l.batch_no.clone(),
        expiry_date: l.expiry_date,
    }
}

pub(crate) async fn to_dto<C: ConnectionTrait>(conn: &C, m: &AdjustmentModel) -> TxResult<StockAdjustment> {
    let lines = LineEntity::find()
        .filter(LineColumn::StockAdjustmentId.eq(m.id))
        .order_by_asc(LineColumn::Position)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(StockAdjustment {
        id: m.id,
        number: m.number.clone(),
        r#type: parse_adjustment_type(&m.r#type),
        date: m.date().key(),
        status: parse_status(&m.status),
        lines: lines.iter().map(line_to_dto).collect(),
        note: m.note.clone(),
        reason: parse_reason(m.reason.as_deref()),
        offset_account_id: m.offset_account_id,
        approved_by: m.approved_by,
        approved_at: m.approved_at.map(crate::utils::dates::format_iso_ms),
    })
}

// --- S-3 validate_stock_in ------------------------------------------------------------------------

/// **S-3 `validate_stock_in(input)`** (`:135-145`).
async fn validate_stock_in<C: ConnectionTrait>(conn: &C, input: &StockAdjustmentInput) -> TxResult<()> {
    if !matches!(input.r#type, StockAdjustmentType::StockIn) {
        return Ok(());
    }
    let Some(reason) = input.reason else {
        return Err(AppError::validation("اختر سبب إدخال المخزون").into());
    };
    if matches!(reason, StockInReason::Other) {
        let Some(offset_account_id) = input.offset_account_id else {
            return Err(AppError::validation("اختر الحساب المقابل لسبب \"أخرى\"").into());
        };
        let account = account_by_id(conn, offset_account_id).await?;
        let role = account.system_role.as_deref();
        if matches!(role, Some("receivable") | Some("payable") | Some("inventory")) || !account.allow_manual {
            return Err(AppError::validation(format!("لا يمكن اختيار \"{}\" كحساب مقابل — اختر حساباً غير رئيسي", account.name)).into());
        }
    }
    Ok(())
}

// --- S-4 assert_approval ----------------------------------------------------------------------

/// Whether the actor's manager-PIN grant covers this adjustment (D-I2) — computed by the command
/// layer from `state.approval_grants` before `with_tx` (G-P3, manager-owned addition to
/// `core::state::AppState` — see this wave's "Needs from manager" note) and passed in here.
pub struct ApprovalCheck {
    pub granted: bool,
}

impl ApprovalCheck {
    pub fn none() -> Self {
        Self { granted: false }
    }
}

/// **S-4 `assert_approval(ty, lines, approved_by, approval)`** (`:218-226` + D-I2): STOCKTAKE -> ok;
/// no/non-positive threshold -> ok; else `value >= threshold` and (`approved_by` absent OR not
/// granted OR the user isn't an active admin/manager) -> `FORBIDDEN`.
async fn assert_approval<C: ConnectionTrait>(
    conn: &C,
    ty: StockAdjustmentType,
    lines: &[StockAdjustmentLine],
    approved_by: Option<Id>,
    approval: &ApprovalCheck,
) -> TxResult<()> {
    if matches!(ty, StockAdjustmentType::Stocktake) {
        return Ok(());
    }
    let settings = crate::core::settings::load(conn).await?;
    let Some(threshold) = settings.inventory_approval_threshold.filter(|t| *t > Decimal::ZERO) else {
        return Ok(());
    };
    let value = round2(lines.iter().fold(Decimal::ZERO, |acc, l| acc + l.qty_change.abs() * l.unit_cost.unwrap_or(Decimal::ZERO)));
    if value < threshold {
        return Ok(());
    }

    let approver_ok = match approved_by {
        Some(approver_id) if approval.granted => {
            let user = crate::entities::org::users::Entity::find_by_id(approver_id).one(conn).await.map_err(AppError::from)?;
            match user {
                Some(u) => u.active && matches!(u.role.as_str(), "admin" | "manager"),
                None => false,
            }
        }
        _ => false,
    };
    if !approver_ok {
        return Err(AppError::forbidden(format!(
            "قيمة هذه الحركة ({}) تتجاوز حد الاعتماد ({}) — يلزم تأكيد المدير",
            to_fixed2(value),
            to_fixed2(threshold)
        ))
        .into());
    }
    Ok(())
}

// --- S-5 post_adjustment ----------------------------------------------------------------------

/// **S-5 `post_adjustment(conn, cx, adj, lines, locked)`** (`:153-209`). `pub(crate)` — C-K6
/// (`counts.rs`) builds its own COMPLETED STOCKTAKE row and posts it directly, exactly like the
/// mock's `applyStockCount` calls `postAdjustment` without going through `recordStockAdjustment`.
pub(crate) async fn post_adjustment<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    adj: &AdjustmentModel,
    lines: &[StockAdjustmentLine],
    locked: &mut BTreeMap<Id, LockedProduct>,
) -> TxResult<()> {
    let ty = parse_adjustment_type(&adj.r#type);
    let reason_str: &'static str = match ty {
        StockAdjustmentType::StockIn => "stock_in",
        StockAdjustmentType::Loss => "loss",
        StockAdjustmentType::Stocktake => "stocktake",
    };
    let date = adj.date();
    let today = cx.clock.today();

    let mut gains = Decimal::ZERO;
    let mut losses = Decimal::ZERO;

    for line in lines {
        let unit_cost = line.unit_cost.unwrap_or(Decimal::ZERO);
        let value = round2(line.qty_change.abs() * unit_cost);
        if line.qty_change > Decimal::ZERO {
            gains += value;
        } else {
            losses += value;
        }

        let p = locked.get_mut(&line.product_id).expect("line product must already be locked");
        let track_batches = p.track_batches();
        stock::apply_change(
            conn,
            cx,
            p,
            line.qty_change,
            if line.qty_change > Decimal::ZERO { value } else { -value },
            reason_str,
            StockRef { id: adj.id, number: adj.number.clone() },
            &date,
            None,
        )
        .await?;

        if track_batches {
            if matches!(ty, StockAdjustmentType::StockIn) {
                if let Some(batch_no) = &line.batch_no {
                    batches::receive_batch(
                        conn,
                        cx,
                        line.product_id,
                        line.qty_change,
                        unit_cost,
                        batch_no.clone(),
                        line.expiry_date,
                        &date,
                        &StockRef { id: adj.id, number: adj.number.clone() },
                    )
                    .await?;
                }
            } else if matches!(ty, StockAdjustmentType::Loss) {
                batches::consume_fefo(conn, cx, line.product_id, line.qty_change.abs(), true, today).await?;
            }
        }
    }

    gains = round2(gains);
    losses = round2(losses);
    if gains <= Decimal::ZERO && losses <= Decimal::ZERO {
        return Ok(());
    }

    let posting_lines: Vec<PostingLine> = match ty {
        StockAdjustmentType::StockIn => {
            let credit_line = match parse_reason(adj.reason.as_deref()) {
                Some(StockInReason::Opening) => PostingLine::credit(AccountRef::Role(SystemRole::OpeningBalanceEquity), gains),
                Some(StockInReason::OwnerContribution) => PostingLine::credit(AccountRef::Role(SystemRole::OwnerCurrent), gains),
                Some(StockInReason::Gift) => PostingLine::credit(AccountRef::Role(SystemRole::OtherIncome), gains),
                Some(StockInReason::Found) => PostingLine::credit(AccountRef::Role(SystemRole::InventoryVariance), gains),
                Some(StockInReason::Other) | None => PostingLine::credit(AccountRef::Id(adj.offset_account_id.expect("validate_stock_in guarantees this for reason=other")), gains),
            };
            vec![PostingLine::debit(AccountRef::Role(SystemRole::Inventory), gains), credit_line]
        }
        StockAdjustmentType::Loss => {
            vec![
                PostingLine::debit(AccountRef::Role(SystemRole::InventoryWriteOff), losses),
                PostingLine::credit(AccountRef::Role(SystemRole::Inventory), losses),
            ]
        }
        StockAdjustmentType::Stocktake => {
            let mut v = Vec::new();
            if gains > Decimal::ZERO {
                v.push(PostingLine::debit(AccountRef::Role(SystemRole::Inventory), gains));
                v.push(PostingLine::credit(AccountRef::Role(SystemRole::InventoryVariance), gains));
            }
            if losses > Decimal::ZERO {
                v.push(PostingLine::debit(AccountRef::Role(SystemRole::InventoryVariance), losses));
                v.push(PostingLine::credit(AccountRef::Role(SystemRole::Inventory), losses));
            }
            v
        }
    };

    post::post(
        conn,
        cx,
        PostJournal {
            date,
            description: format!("{} {}{}", type_label(ty), adj.number, adj.note.as_deref().map(|n| format!(" — {n}")).unwrap_or_default()),
            entry_type: JournalEntryType::System,
            source: Some(post::SourceRef { kind: "stockAdjustment".to_string(), id: adj.id, number: Some(adj.number.clone()) }),
            lines: posting_lines,
            allow_closed_period: false,
            attachment_ids: Vec::new(),
            template_id: None,
        },
    )
    .await?;

    Ok(())
}

// --- C-A1 record_stock_adjustment ---------------------------------------------------------------

/// **C-A1 `record_stock_adjustment(conn, cx, reg, input, as_draft, approval)`** (`:228-250`; also
/// called by 06 C-7 and 06b C-B3). The plan marks this `pub(crate)` for cross-domain (07-purchases)
/// reuse; widened to `pub` here so `tests/domain_products.rs` (an external integration-test crate)
/// can drive the STOCK_IN/LOSS/STOCKTAKE/draft scenarios §8a's checklist calls for directly, the
/// same way every other command in this file is `pub` — `pub(crate)` would make those cases
/// untestable from `tests/*.rs` (this codebase's tests exercise the service layer, never the Tauri
/// `State` layer). No behavior change; 07-purchases' planned call site still compiles unchanged.
#[allow(clippy::too_many_arguments)]
pub async fn record_stock_adjustment<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::UndoRegistry,
    input: StockAdjustmentInput,
    as_draft: bool,
    approval: ApprovalCheck,
) -> TxResult<StockAdjustment> {
    validate_stock_in(conn, &input).await?;

    let product_ids: Vec<Id> = input.lines.iter().map(|l| l.product_id).collect();
    let mut locked = lock_line_products(conn, &product_ids).await?;

    let date = RawDocDate::parse(&input.date).map_err(|_| AppError::validation("تاريخ غير صالح"))?.resolve(&cx.clock);
    let built_lines = build_lines(input.r#type, &input.lines, &locked, None)?;

    if !as_draft {
        assert_approval(conn, input.r#type, &built_lines, input.approved_by, &approval).await?;
    }

    let number = numbering::next_number(conn, DocumentKind::Adjustment).await?;
    let now = cx.clock.now;
    let id = Id::new();
    let approved_at = if input.approved_by.is_some() { Some(now) } else { None };

    let am = AdjustmentActiveModel {
        id: Set(id),
        number: Set(number.clone()),
        r#type: Set(adjustment_type_as_str(input.r#type).to_string()),
        date_day: Set(date.day),
        date_instant: Set(date.instant),
        date_key: Set(date.key()),
        status: Set(status_as_str(if as_draft { StockAdjustmentStatus::Draft } else { StockAdjustmentStatus::Completed }).to_string()),
        note: Set(input.note.clone()),
        reason: Set(reason_as_str(input.reason).map(str::to_string)),
        offset_account_id: Set(input.offset_account_id),
        approved_by: Set(input.approved_by),
        approved_at: Set(approved_at),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set("local".to_string()),
    };
    let inserted = am.insert(conn).await.map_err(AppError::from)?;

    for (i, line) in built_lines.iter().enumerate() {
        let line_am = LineActiveModel {
            id: Set(Id::new()),
            stock_adjustment_id: Set(id),
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

    if !as_draft {
        post_adjustment(conn, cx, &inserted, &built_lines, &mut locked).await?;
    }

    let reason_suffix = input.reason.map(|r| format!(" — {}", stock_in_reason_label(r))).unwrap_or_default();
    let draft_suffix = if as_draft { " (مسودة)" } else { "" };
    activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Stock,
        format!("{}{reason_suffix} {number}{draft_suffix} — {} صنف", type_label(input.r#type), built_lines.len()),
        Some(date),
        Some(RouteRef::detail("adjustment", id.to_string())),
    )
    .await?;

    if !as_draft {
        cx.touch(crate::core::events::ChangeCategory::Catalog);
    }

    to_dto(conn, &inserted).await
}

// --- C-A2 complete_adjustment -------------------------------------------------------------------

/// **C-A2 `complete_adjustment(id, approved_by, approval)`** (`:259-282`).
pub async fn complete_adjustment<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::UndoRegistry,
    id: Id,
    approved_by: Option<Id>,
    approval: ApprovalCheck,
) -> TxResult<StockAdjustment> {
    lock::for_update_by_id(conn, "stock_adjustments", &id.to_string()).await.map_err(AppError::from)?;
    let adj = AdjustmentEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("التسوية غير موجودة"))?;
    if parse_status(&adj.status) != StockAdjustmentStatus::Draft {
        return Err(AppError::validation("التسوية مكتملة بالفعل").into());
    }

    let existing_lines = LineEntity::find().filter(LineColumn::StockAdjustmentId.eq(id)).order_by_asc(LineColumn::Position).all(conn).await.map_err(AppError::from)?;
    let snapshot: BTreeMap<Id, Decimal> = existing_lines.iter().map(|l| (l.product_id, l.system_qty.unwrap_or(Decimal::ZERO))).collect();

    let product_ids: Vec<Id> = existing_lines.iter().map(|l| l.product_id).collect();
    let mut locked = lock_line_products(conn, &product_ids).await?;

    let ty = parse_adjustment_type(&adj.r#type);
    let line_inputs: Vec<super::super::dto::inventory::StockAdjustmentLineInput> = existing_lines
        .iter()
        .map(|l| super::super::dto::inventory::StockAdjustmentLineInput {
            product_id: l.product_id,
            qty_change: Some(l.qty_change.abs()),
            counted_qty: l.counted_qty,
            batch_no: l.batch_no.clone(),
            expiry_date: l.expiry_date,
        })
        .collect();
    let built_lines = build_lines(ty, &line_inputs, &locked, Some(&snapshot))?;
    // Rule (ACC-0006): completing a draft posts it, so it passes the same S-4 approval threshold / manager-PIN check as a direct adjustment.
    assert_approval(conn, ty, &built_lines, approved_by, &approval).await?;

    LineEntity::delete_many().filter(LineColumn::StockAdjustmentId.eq(id)).exec(conn).await.map_err(AppError::from)?;
    for (i, line) in built_lines.iter().enumerate() {
        let line_am = LineActiveModel {
            id: Set(Id::new()),
            stock_adjustment_id: Set(id),
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

    let now = cx.clock.now;
    let today = cx.clock.today();
    let date = DocDate { day: today, instant: Some(now) };
    let mut am: AdjustmentActiveModel = adj.clone().into();
    am.date_day = Set(date.day);
    am.date_instant = Set(date.instant);
    am.date_key = Set(date.key());
    am.status = Set(status_as_str(StockAdjustmentStatus::Completed).to_string());
    if approved_by.is_some() {
        am.approved_by = Set(approved_by);
        am.approved_at = Set(Some(now));
    }
    am.updated_at = Set(now);
    let updated = am.update(conn).await.map_err(AppError::from)?;

    post_adjustment(conn, cx, &updated, &built_lines, &mut locked).await?;

    activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Stock,
        format!("اعتماد {} {}", type_label(ty), updated.number),
        Some(date),
        Some(RouteRef::detail("adjustment", id.to_string())),
    )
    .await?;
    cx.touch(crate::core::events::ChangeCategory::Catalog);

    to_dto(conn, &updated).await
}

// --- C-A3 delete_draft_adjustment ---------------------------------------------------------------

/// **C-A3 `delete_draft_adjustment(id)`** (`inventoryService.ts:79-86`).
pub async fn delete_draft_adjustment<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, id: Id) -> TxResult<()> {
    lock::for_update_by_id(conn, "stock_adjustments", &id.to_string()).await.map_err(AppError::from)?;
    let adj = AdjustmentEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("التسوية غير موجودة"))?;
    if parse_status(&adj.status) != StockAdjustmentStatus::Draft {
        return Err(AppError::validation("لا يمكن حذف تسوية معتمدة — أنشئ تسوية عكسية بدلاً من ذلك").into());
    }

    LineEntity::delete_many().filter(LineColumn::StockAdjustmentId.eq(id)).exec(conn).await.map_err(AppError::from)?;
    AdjustmentEntity::delete_by_id(id).exec(conn).await.map_err(AppError::from)?;

    let now_date = DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) };
    activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Stock,
        format!("حذف مسودة تسوية {}", adj.number),
        Some(now_date),
        Some(RouteRef::detail("adjustment", id.to_string())),
    )
    .await?;
    Ok(())
}

// --- C-A4 reads -----------------------------------------------------------------------------------

/// **`get_stock_adjustments(filter)`** (`:47-67`).
pub async fn get_stock_adjustments<C: ConnectionTrait>(conn: &C, filter: Option<super::super::dto::inventory::AdjustmentFilter>) -> TxResult<Vec<StockAdjustment>> {
    let mut query = AdjustmentEntity::find();
    if let Some(f) = &filter {
        if let Some(t) = f.r#type {
            query = query.filter(AdjustmentColumn::Type.eq(adjustment_type_as_str(t)));
        }
        if let Some(s) = f.status {
            query = query.filter(AdjustmentColumn::Status.eq(status_as_str(s)));
        }
    }
    let rows = query.order_by_asc(AdjustmentColumn::CreatedAt).order_by_asc(AdjustmentColumn::Id).all(conn).await.map_err(AppError::from)?;

    let from = filter.as_ref().and_then(|f| f.from.as_deref());
    let to = filter.as_ref().and_then(|f| f.to.as_deref());
    let mut rows: Vec<AdjustmentModel> = rows.into_iter().filter(|r| crate::utils::dates::in_date_range(r.date_day, from, to)).collect();
    // Stable sort by date key descending (ties keep created_at, id — already the query order).
    rows.sort_by(|a, b| b.date().key().cmp(&a.date().key()));

    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        out.push(to_dto(conn, row).await?);
    }
    Ok(out)
}

/// **`get_stock_adjustment(id)`** (`:47-67`).
pub async fn get_stock_adjustment<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<StockAdjustmentDetail> {
    let adj = AdjustmentEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("التسوية غير موجودة"))?;
    let dto = to_dto(conn, &adj).await?;
    let je = JournalEntity::find()
        .filter(JournalColumn::SourceKind.eq("stockAdjustment"))
        .filter(JournalColumn::SourceId.eq(id))
        .order_by_asc(JournalColumn::CreatedAt)
        .order_by_asc(JournalColumn::Id)
        .one(conn)
        .await
        .map_err(AppError::from)?;
    Ok(StockAdjustmentDetail::new(dto, je.map(|j| j.id)))
}
