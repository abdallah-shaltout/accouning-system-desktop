//! `domains::invoices::service::shifts` — 08b §3.1-3.5: shared helpers (also called by 08's
//! sale/refund), reads, open, close, force-close, cash in/out.

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder, QuerySelect, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::platform::activity::ActivityKind;
use crate::entities::sales::shift_movements::{ActiveModel as MovementActiveModel, Entity as MovementEntity, Model as MovementModel, ShiftMovementKind as EntityMovementKind};
use crate::entities::sales::shifts::{ActiveModel as ShiftActiveModel, Entity as ShiftEntity, HandoverMode as EntityHandoverMode, Model as ShiftModel, ShiftStatus};
use crate::shared::activity;
use crate::shared::ledger::accounts::{resolve_account, AccountCtx, SystemRole};
use crate::shared::ledger::period::assert_open_period;
use crate::shared::ledger::post::{self, AccountRef, PostJournal, PostingLine, SourceRef};
use crate::shared::numbering::{self, DocumentKind};
use crate::utils::dates::DocDate;
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::route::RouteRef;

use super::super::dto::{self as d, CloseShiftInput, OpenShiftInput, Shift, ShiftMovementKind, ShiftRow, ShiftSummary};

// --- 3.1 shared helpers --------------------------------------------------------------------------

/// `open_shift_for_terminal(conn, terminal_id, lock) -> Option<shifts::Model>` (`currentOpenShift`).
pub async fn open_shift_for_terminal<C: ConnectionTrait>(conn: &C, terminal_id: Id, lock: bool) -> TxResult<Option<ShiftModel>> {
    if lock {
        // `SELECT ... FOR UPDATE` scoped to this terminal's OPEN shift row (at most one, by
        // `uq_shifts_open_key`) — raw statement since sea-query's `lock_exclusive` combined with a
        // filter-by-enum works fine through the ORM query builder here.
        use sea_orm::QueryTrait;
        let select = ShiftEntity::find()
            .filter(crate::entities::sales::shifts::Column::TerminalId.eq(terminal_id))
            .filter(crate::entities::sales::shifts::Column::Status.eq(ShiftStatus::Open))
            .lock_exclusive();
        let stmt = select.build(conn.get_database_backend());
        let rows = conn.query_all(stmt).await.map_err(TxError::from)?;
        if let Some(row) = rows.into_iter().next() {
            return Ok(Some(ShiftModel::from_query_result(&row, "").map_err(AppError::from)?));
        }
        return Ok(None);
    }
    ShiftEntity::find()
        .filter(crate::entities::sales::shifts::Column::TerminalId.eq(terminal_id))
        .filter(crate::entities::sales::shifts::Column::Status.eq(ShiftStatus::Open))
        .one(conn)
        .await
        .map_err(TxError::from)
}

/// `lock_open_shift_of_terminal(conn, shift_id) -> Option<shifts::Model>` — reads that shift's
/// `terminal_id` (no lock), then `open_shift_for_terminal(..., lock = true)`.
pub async fn lock_open_shift_of_terminal<C: ConnectionTrait>(conn: &C, shift_id: Id) -> TxResult<Option<ShiftModel>> {
    let terminal_id = match ShiftEntity::find_by_id(shift_id).one(conn).await.map_err(AppError::from)? {
        Some(s) => s.terminal_id,
        None => return Ok(None),
    };
    open_shift_for_terminal(conn, terminal_id, true).await
}

/// `record_shift_movement` (`shifts.ts:45-66`): `amount == 0` -> no-op; else insert a movement row
/// at `position = COALESCE(MAX(position) + 1, 0)`. The caller must already hold the shift row lock.
#[allow(clippy::too_many_arguments)]
pub async fn record_shift_movement<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    shift: &ShiftModel,
    kind: ShiftMovementKind,
    amount: Decimal,
    note: Option<String>,
    ref_id: Option<Id>,
    ref_number: Option<String>,
    at: DocDate,
) -> TxResult<()> {
    if amount.is_zero() {
        return Ok(());
    }
    let max_position: Option<i16> = MovementEntity::find()
        .filter(crate::entities::sales::shift_movements::Column::ShiftId.eq(shift.id))
        .all(conn)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .map(|m| m.position)
        .max();
    let position = max_position.map(|p| p + 1).unwrap_or(0);
    let by = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;
    let am = MovementActiveModel {
        id: Set(Id::new()),
        shift_id: Set(shift.id),
        position: Set(position),
        kind: Set(movement_kind_to_entity(kind)),
        amount: Set(round2(amount)),
        note: Set(note),
        ref_id: Set(ref_id),
        ref_number: Set(ref_number),
        at_day: Set(at.day),
        at_instant: Set(at.instant),
        by: Set(by),
    };
    am.insert(conn).await.map_err(AppError::from)?;
    Ok(())
}

fn movement_kind_to_entity(k: ShiftMovementKind) -> EntityMovementKind {
    match k {
        ShiftMovementKind::SaleCash => EntityMovementKind::SaleCash,
        ShiftMovementKind::RefundCash => EntityMovementKind::RefundCash,
        ShiftMovementKind::PayIn => EntityMovementKind::PayIn,
        ShiftMovementKind::PayOut => EntityMovementKind::PayOut,
        ShiftMovementKind::BankDrop => EntityMovementKind::BankDrop,
    }
}

fn movement_kind_to_dto(k: &EntityMovementKind) -> ShiftMovementKind {
    match k {
        EntityMovementKind::SaleCash => ShiftMovementKind::SaleCash,
        EntityMovementKind::RefundCash => ShiftMovementKind::RefundCash,
        EntityMovementKind::PayIn => ShiftMovementKind::PayIn,
        EntityMovementKind::PayOut => ShiftMovementKind::PayOut,
        EntityMovementKind::BankDrop => ShiftMovementKind::BankDrop,
    }
}

fn handover_mode_to_entity(m: d::HandoverMode) -> EntityHandoverMode {
    match m {
        d::HandoverMode::Handover => EntityHandoverMode::Handover,
        d::HandoverMode::Drop => EntityHandoverMode::Drop,
    }
}

fn handover_mode_to_dto(m: &EntityHandoverMode) -> d::HandoverMode {
    match m {
        EntityHandoverMode::Handover => d::HandoverMode::Handover,
        EntityHandoverMode::Drop => d::HandoverMode::Drop,
    }
}

fn shift_status_to_dto(s: &ShiftStatus) -> d::ShiftStatus {
    match s {
        ShiftStatus::Open => d::ShiftStatus::Open,
        ShiftStatus::Closed => d::ShiftStatus::Closed,
    }
}

fn movement_to_dto(m: &MovementModel) -> d::ShiftMovement {
    d::ShiftMovement { id: m.id, kind: movement_kind_to_dto(&m.kind), amount: m.amount, note: m.note.clone(), ref_id: m.ref_id, ref_number: m.ref_number.clone(), at: m.at().key(), by: m.by }
}

fn shift_base_to_dto(model: &ShiftModel, movements: &[MovementModel]) -> d::ShiftBase {
    d::ShiftBase {
        id: model.id,
        number: model.number.clone(),
        terminal_id: model.terminal_id.to_string(),
        branch_id: model.branch_id,
        status: shift_status_to_dto(&model.status),
        opened_by: model.opened_by,
        opened_at: crate::utils::dates::DocDate { day: model.opened_at_day, instant: model.opened_at_instant }.key(),
        opening_float: model.opening_float,
        opening_denominations: parse_denominations(&model.opening_denominations),
        movements: movements.iter().map(movement_to_dto).collect(),
        closed_by: model.closed_by,
        closed_at: model.closed_at_day.map(|day| crate::utils::dates::DocDate { day, instant: model.closed_at_instant }.key()),
        counted_cash: model.counted_cash,
        closing_denominations: parse_denominations(&model.closing_denominations),
        variance: model.variance,
        handover_mode: model.handover_mode.as_ref().map(handover_mode_to_dto),
        force_closed_by: model.force_closed_by,
        note: model.note.clone(),
    }
}

fn shift_to_dto(model: &ShiftModel, movements: &[MovementModel]) -> Shift {
    Shift { base: shift_base_to_dto(model, movements), expected_cash: model.expected_cash }
}

fn parse_denominations(v: &Option<serde_json::Value>) -> Option<Vec<d::DenominationCount>> {
    v.as_ref().and_then(|v| serde_json::from_value(v.clone()).ok())
}

fn denominations_to_json(v: &Option<Vec<d::DenominationCount>>) -> Option<serde_json::Value> {
    v.as_ref().map(|v| serde_json::to_value(v).expect("DenominationCount always serializes"))
}

/// `shift_summary(conn, shift)` (`shifts.ts:69-91`).
pub async fn shift_summary<C: ConnectionTrait>(conn: &C, shift: &ShiftModel) -> TxResult<ShiftSummary> {
    let movements = MovementEntity::find().filter(crate::entities::sales::shift_movements::Column::ShiftId.eq(shift.id)).all(conn).await.map_err(AppError::from)?;
    let sum_kind = |k: EntityMovementKind| round2(movements.iter().filter(|m| m.kind == k).fold(Decimal::ZERO, |a, m| a + m.amount));
    let cash_sales = sum_kind(EntityMovementKind::SaleCash);
    let cash_refunds = sum_kind(EntityMovementKind::RefundCash);
    let pay_ins = sum_kind(EntityMovementKind::PayIn);
    let pay_outs = sum_kind(EntityMovementKind::PayOut);
    let bank_drops = sum_kind(EntityMovementKind::BankDrop);
    let expected_cash = round2(shift.opening_float + cash_sales - cash_refunds + pay_ins - pay_outs - bank_drops);

    let invoices = crate::entities::sales::invoices::Entity::find()
        .filter(crate::entities::sales::invoices::Column::ShiftId.eq(shift.id))
        .order_by_asc(crate::entities::sales::invoices::Column::CreatedAt)
        .order_by_asc(crate::entities::sales::invoices::Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;

    let mut labels: Vec<(String, Decimal)> = Vec::new();
    if !invoices.is_empty() {
        let invoice_ids: Vec<Id> = invoices.iter().map(|i| i.id).collect();
        let tenders = crate::entities::sales::invoice_tenders::Entity::find()
            .filter(crate::entities::sales::invoice_tenders::Column::InvoiceId.is_in(invoice_ids))
            .order_by_asc(crate::entities::sales::invoice_tenders::Column::Position)
            .all(conn)
            .await
            .map_err(AppError::from)?;
        let mut by_invoice: std::collections::BTreeMap<Id, Vec<crate::entities::sales::invoice_tenders::Model>> = std::collections::BTreeMap::new();
        for t in tenders {
            by_invoice.entry(t.invoice_id).or_default().push(t);
        }
        let method_ids: std::collections::BTreeSet<Id> = by_invoice.values().flatten().map(|t| t.payment_method_id).collect();
        let methods: std::collections::BTreeMap<Id, String> = if method_ids.is_empty() {
            std::collections::BTreeMap::new()
        } else {
            <crate::entities::org::payment_methods::Entity as crate::entities::soft_delete::SoftDelete>::find_including_deleted()
                .filter(crate::entities::org::payment_methods::Column::Id.is_in(method_ids))
                .all(conn)
                .await
                .map_err(AppError::from)?
                .into_iter()
                .map(|m| (m.id, m.name))
                .collect()
        };
        for inv in &invoices {
            for t in by_invoice.get(&inv.id).into_iter().flatten() {
                let label = methods.get(&t.payment_method_id).cloned().unwrap_or_else(|| t.payment_method_id.to_string());
                match labels.iter_mut().find(|(l, _)| *l == label) {
                    Some((_, amount)) => *amount = round2(*amount + t.amount),
                    None => labels.push((label, t.amount)),
                }
            }
        }
    }

    let sales_total = round2(invoices.iter().fold(Decimal::ZERO, |a, i| a + i.grand_total));

    Ok(ShiftSummary {
        cash_sales,
        cash_refunds,
        pay_ins,
        pay_outs,
        bank_drops,
        expected_cash,
        sales_by_method: labels.into_iter().map(|(label, amount)| d::SalesByMethod { label, amount }).collect(),
        sales_total,
        invoice_count: invoices.len() as i32,
    })
}

/// `to_shift_row` (`invoiceService.ts:255-262`).
pub async fn to_shift_row<C: ConnectionTrait>(conn: &C, model: &ShiftModel) -> TxResult<ShiftRow> {
    let movements = MovementEntity::find()
        .filter(crate::entities::sales::shift_movements::Column::ShiftId.eq(model.id))
        .order_by_asc(crate::entities::sales::shift_movements::Column::Position)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let shift = shift_base_to_dto(model, &movements);
    let summary = shift_summary(conn, model).await?;
    let opened_by_name = crate::entities::org::users::Entity::find_by_id(model.opened_by)
        .one(conn)
        .await
        .map_err(AppError::from)?
        .map(|u| u.name)
        .unwrap_or_else(|| "—".to_string());
    let closed_by_name = match model.closed_by {
        Some(id) => crate::entities::org::users::Entity::find_by_id(id).one(conn).await.map_err(AppError::from)?.map(|u| u.name),
        None => None,
    };
    Ok(ShiftRow { shift, summary, opened_by_name, closed_by_name })
}

// --- 3.2 reads -------------------------------------------------------------------------------

pub async fn get_current_shift<C: ConnectionTrait>(conn: &C, terminal_id: Id) -> TxResult<Option<ShiftRow>> {
    match open_shift_for_terminal(conn, terminal_id, false).await? {
        Some(shift) => Ok(Some(to_shift_row(conn, &shift).await?)),
        None => Ok(None),
    }
}

pub async fn get_shifts<C: ConnectionTrait>(conn: &C, filter: super::super::dto::ShiftFilter) -> TxResult<Vec<ShiftRow>> {
    let mut query = ShiftEntity::find();
    if let Some(status) = filter.status {
        query = query.filter(crate::entities::sales::shifts::Column::Status.eq(match status {
            d::ShiftStatus::Open => ShiftStatus::Open,
            d::ShiftStatus::Closed => ShiftStatus::Closed,
        }));
    }
    let mut rows = query.all(conn).await.map_err(AppError::from)?;
    rows.sort_by(|a, b| {
        (b.opened_at_instant, b.opened_at_day, b.created_at, b.id).cmp(&(a.opened_at_instant, a.opened_at_day, a.created_at, a.id))
    });
    let mut out = Vec::with_capacity(rows.len());
    for r in &rows {
        out.push(to_shift_row(conn, r).await?);
    }
    Ok(out)
}

pub async fn get_shift<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<ShiftRow> {
    let shift = ShiftEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("الوردية غير موجودة"))?;
    to_shift_row(conn, &shift).await
}

pub async fn get_x_report<C: ConnectionTrait>(conn: &C, shift_id: Id) -> TxResult<ShiftRow> {
    get_shift(conn, shift_id).await
}

// --- 3.3 open_pos_shift ------------------------------------------------------------------------

pub async fn open_pos_shift<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, input: OpenShiftInput) -> TxResult<Shift> {
    if open_shift_for_terminal(conn, cx.terminal_id, true).await?.is_some() {
        return Err(AppError::conflict("توجد وردية مفتوحة بالفعل على هذا الجهاز").into());
    }
    if input.opening_float < Decimal::ZERO {
        return Err(AppError::validation("رصيد الافتتاح لا يمكن أن يكون سالباً").into());
    }
    let number = numbering::next_number(conn, DocumentKind::Shift).await?;
    let now = cx.clock.now;
    let opened_by = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;
    let id = Id::new();
    let am = ShiftActiveModel {
        id: Set(id),
        number: Set(number.clone()),
        terminal_id: Set(cx.terminal_id),
        branch_id: Set(input.branch_id),
        status: Set(ShiftStatus::Open),
        opened_by: Set(opened_by),
        opened_at_day: Set(cx.clock.today()),
        opened_at_instant: Set(Some(now)),
        opening_float: Set(round2(input.opening_float)),
        opening_denominations: Set(denominations_to_json(&input.opening_denominations)),
        closed_by: Set(None),
        closed_at_day: Set(None),
        closed_at_instant: Set(None),
        counted_cash: Set(None),
        closing_denominations: Set(None),
        expected_cash: Set(None),
        variance: Set(None),
        handover_mode: Set(None),
        force_closed_by: Set(None),
        note: Set(None),
        open_key: sea_orm::ActiveValue::NotSet,
        created_at: Set(now),
        updated_at: Set(now),
        sync_status: Set(crate::entities::sales::shifts::SyncStatus::Local),
    };
    let inserted = am.insert(conn).await.map_err(|e| match crate::core::error::duplicate_key_name(&e) {
        Some(k) if k == "uq_shifts_open_key" => TxError::App(AppError::conflict("توجد وردية مفتوحة بالفعل على هذا الجهاز")),
        _ => TxError::from(e),
    })?;

    activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Shift,
        format!("فتح وردية {number} — رصيد افتتاحي {:.2}", round2(input.opening_float)),
        Some(DocDate { day: cx.clock.today(), instant: Some(now) }),
        Some(RouteRef::list("pos-shifts")),
    )
    .await?;

    Ok(shift_to_dto(&inserted, &[]))
}

// --- 3.4 close_shift ---------------------------------------------------------------------------

/// `close_shift(conn, cx, reg, shift_id, input, forced_by)` (§3.4).
pub async fn close_shift<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::UndoRegistry,
    shift_id: Id,
    input: CloseShiftInput,
    forced_by: Option<Id>,
) -> TxResult<Shift> {
    lock::for_update_by_id(conn, "shifts", &shift_id.to_string()).await.map_err(AppError::from)?;
    let shift = ShiftEntity::find_by_id(shift_id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("الوردية غير موجودة"))?;
    if shift.status != ShiftStatus::Open {
        return Err(AppError::validation("الوردية مغلقة بالفعل").into());
    }

    let summary = shift_summary(conn, &shift).await?;
    let expected = summary.expected_cash;
    let counted = round2(input.counted_cash);
    let variance = round2(counted - expected);
    let posts_variance = variance.abs() > Decimal::new(5, 3);
    let drop = matches!(input.handover_mode, Some(d::HandoverMode::Drop)) && counted > Decimal::ZERO;

    let now = cx.clock.now;
    let today = cx.clock.today();
    let date = DocDate { day: today, instant: Some(now) };

    let mut voucher_number: Option<String> = None;
    if posts_variance || drop {
        assert_open_period(conn, &today, false).await?;
        if drop {
            voucher_number = Some(numbering::next_number(conn, DocumentKind::Voucher).await?);
        }
    }

    if posts_variance {
        let lines = if variance > Decimal::ZERO {
            vec![PostingLine::debit(AccountRef::Role(SystemRole::Cash), variance), PostingLine::credit(AccountRef::Role(SystemRole::CashOver), variance)]
        } else {
            vec![PostingLine::debit(AccountRef::Role(SystemRole::CashShort), -variance), PostingLine::credit(AccountRef::Role(SystemRole::Cash), -variance)]
        };
        post::post(
            conn,
            cx,
            PostJournal {
                date,
                description: format!("تسوية عجز/زيادة الصندوق — وردية {}", shift.number),
                entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
                source: Some(SourceRef { kind: "shift".to_string(), id: shift.id, number: Some(shift.number.clone()) }),
                lines,
                allow_closed_period: false,
                attachment_ids: Vec::new(),
                template_id: None,
            },
        )
        .await?;
    }

    if drop {
        let cash_account = resolve_account(conn, SystemRole::Cash, &AccountCtx::default()).await?;
        let bank_account = resolve_account(conn, SystemRole::Bank, &AccountCtx::default()).await?;
        crate::domains::vouchers::service::general::record_transfer_voucher(
            conn,
            cx,
            registry,
            crate::domains::vouchers::dto::TransferVoucherInput {
                date: date.key(),
                amount: counted,
                description: format!("إيداع نقدية الوردية {} إلى الخزينة/البنك", shift.number),
                note: None,
                attachment_ids: None,
                cost_center_id: None,
                source_account_id: cash_account.id,
                destination_account_id: bank_account.id,
                fee_amount: None,
                fee_account_id: None,
            },
            voucher_number,
        )
        .await?;
    }

    let closer_id = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;
    let handover_mode_entity = input.handover_mode.map(handover_mode_to_entity).unwrap_or(EntityHandoverMode::Handover);
    let mut am: ShiftActiveModel = shift.clone().into();
    am.status = Set(ShiftStatus::Closed);
    am.closed_by = Set(Some(closer_id));
    am.closed_at_day = Set(Some(today));
    am.closed_at_instant = Set(Some(now));
    am.counted_cash = Set(Some(counted));
    am.closing_denominations = Set(denominations_to_json(&input.closing_denominations));
    am.expected_cash = Set(Some(expected));
    am.variance = Set(Some(variance));
    am.handover_mode = Set(Some(handover_mode_entity));
    am.note = Set(input.note.clone());
    am.force_closed_by = Set(forced_by);
    am.updated_at = Set(now);
    let updated = am.update(conn).await.map_err(AppError::from)?;

    activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Shift,
        format!("إغلاق وردية {} — المتوقع {:.2}، المعدود {:.2}، الفرق {:.2}", shift.number, expected, counted, variance),
        Some(date),
        Some(RouteRef::list("pos-shifts")),
    )
    .await?;

    cx.touch(crate::core::events::ChangeCategory::Ledger);

    let movements = MovementEntity::find()
        .filter(crate::entities::sales::shift_movements::Column::ShiftId.eq(updated.id))
        .order_by_asc(crate::entities::sales::shift_movements::Column::Position)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(shift_to_dto(&updated, &movements))
}

pub async fn close_pos_shift<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, shift_id: Id, input: CloseShiftInput) -> TxResult<Shift> {
    close_shift(conn, cx, registry, shift_id, input, None).await
}

/// `force_close_pos_shift` (§3.4): status check happens inside `close_shift`, as in the mock.
pub async fn force_close_pos_shift<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, shift_id: Id, counted_cash: Option<Decimal>) -> TxResult<Shift> {
    let shift = ShiftEntity::find_by_id(shift_id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("الوردية غير موجودة"))?;
    let summary = shift_summary(conn, &shift).await?;
    let actor_id = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;
    close_shift(
        conn,
        cx,
        registry,
        shift_id,
        CloseShiftInput { counted_cash: counted_cash.unwrap_or(summary.expected_cash), closing_denominations: None, handover_mode: Some(d::HandoverMode::Handover), note: Some("إغلاق إجباري من المدير".to_string()) },
        Some(actor_id),
    )
    .await
}

// --- 3.5 record_cash_in_out ----------------------------------------------------------------------

pub async fn record_cash_in_out<C: ConnectionTrait>(conn: &C, cx: &TxCtx, kind: super::super::dto::CashMovementKind, amount: Decimal, note: Option<String>) -> TxResult<()> {
    let shift = open_shift_for_terminal(conn, cx.terminal_id, true).await?.ok_or_else(|| AppError::conflict("لا توجد وردية مفتوحة"))?;
    if !(amount > Decimal::ZERO) {
        return Err(AppError::validation("المبلغ يجب أن يكون أكبر من صفر").into());
    }
    let now_date = DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) };
    record_shift_movement(conn, cx, &shift, kind.into(), amount, note, None, None, now_date).await?;
    cx.touch(crate::core::events::ChangeCategory::Ledger);
    Ok(())
}
