//! `domains::products::service::transfers` — branch transfers (06b-inventory.md §3 C-T1…C-T5).

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::inventory::stock_transfer_lines::{ActiveModel as TransferLineActiveModel, Column as TransferLineColumn, Entity as TransferLineEntity, Model as TransferLineModel};
use crate::entities::inventory::stock_transfers::{ActiveModel as TransferActiveModel, Column as TransferColumn, Entity as TransferEntity, Model as TransferModel};
use crate::entities::org::branches::Entity as BranchEntity;
use crate::shared::activity;
use crate::shared::defaults::branch_prefix;
use crate::shared::ledger::accounts::SystemRole;
use crate::shared::ledger::post::{self, AccountRef, PostJournal, PostingLine, SourceRef};
use crate::shared::numbering::{self, DocumentKind};
use crate::shared::stock::{self, StockRef};
use crate::utils::dates::DocDate;
use crate::utils::id::Id;
use crate::utils::money::{round2, round4};
use crate::utils::route::RouteRef;

use super::super::dto::inventory::{ReceiveTransferInput, StockTransfer, StockTransferInput, StockTransferLine, StockTransferStatus};
use super::stock_lines::{lock_line_products, to_fixed2};

fn parse_status(s: &str) -> StockTransferStatus {
    match s {
        "DRAFT" => StockTransferStatus::Draft,
        "SENT" => StockTransferStatus::Sent,
        "RECEIVED" => StockTransferStatus::Received,
        "REJECTED" => StockTransferStatus::Rejected,
        other => panic!("stock_transfers.status column holds an unrecognized value: {other:?}"),
    }
}

fn status_as_str(s: StockTransferStatus) -> &'static str {
    match s {
        StockTransferStatus::Draft => "DRAFT",
        StockTransferStatus::Sent => "SENT",
        StockTransferStatus::Received => "RECEIVED",
        StockTransferStatus::Rejected => "REJECTED",
    }
}

fn line_to_dto(l: &TransferLineModel) -> StockTransferLine {
    StockTransferLine {
        product_id: l.product_id,
        qty: l.qty,
        unit_id: l.unit_id.clone(),
        unit_factor: l.unit_factor,
        batch_id: l.batch_id,
        batch_no: l.batch_no.clone(),
        received_qty: l.received_qty,
        unit_cost: l.unit_cost,
    }
}

/// `pub` entry point for `dashboard::service::feed::in_transit_transfers` (G-39) — delegates to
/// `to_dto` so the `StockTransfer` assembly stays in one place.
pub async fn transfer_dto<C: ConnectionTrait>(conn: &C, m: &TransferModel) -> TxResult<StockTransfer> {
    to_dto(conn, m).await
}

async fn to_dto<C: ConnectionTrait>(conn: &C, m: &TransferModel) -> TxResult<StockTransfer> {
    let lines = TransferLineEntity::find().filter(TransferLineColumn::StockTransferId.eq(m.id)).order_by_asc(TransferLineColumn::Position).all(conn).await.map_err(AppError::from)?;
    Ok(StockTransfer {
        id: m.id,
        number: m.number.clone(),
        from_branch_id: m.from_branch_id,
        to_branch_id: m.to_branch_id,
        status: parse_status(&m.status),
        date: m.date().key(),
        lines: lines.iter().map(line_to_dto).collect(),
        note: m.note.clone(),
        sent_at: m.sent_at().map(|d| d.key()),
        sent_by: m.sent_by,
        received_at: m.received_at().map(|d| d.key()),
        received_by: m.received_by,
        rejected_at: m.rejected_at().map(|d| d.key()),
        rejected_by: m.rejected_by,
        reject_reason: m.reject_reason.clone(),
        shortage_value: m.shortage_value,
    })
}

/// `baseQty` (`transfers.ts:17-19`): `round2(qty * (unitFactor ?? 1))`.
fn base_qty(qty: Decimal, unit_factor: Option<Decimal>) -> Decimal {
    round2(qty * unit_factor.unwrap_or(Decimal::ONE))
}

/// D-I4: the receive/reject journal source id — the transfer id's 16 bytes with the version nibble
/// set to 8 and the variant bits set to `110x`, deterministic and never equal to any v7 id (mirrors
/// the mock's `${transfer.id}-recv` suffix, which can't be a UUID here). `pub` for 00-import to map
/// `<mockId>-recv` the same way.
///
/// Why the variant too: MariaDB 11.4's `UUID` type refuses a version-8+ value whose byte 8 is in
/// `0x01..=0x80` (`1292 Incorrect uuid value`) — with the RFC variant (`10xx`) kept, that is the
/// ~1/64 of transfer ids whose byte 8 is exactly `0x80` (found by the Part 04 parity run: reject
/// failed with INTERNAL). `0xC0..=0xDF` is always accepted.
/// `Id` has no `into_bytes`/`from_bytes` of its own (it's a thin `Uuid` newtype, `utils/id.rs`), so
/// this goes through `Uuid::into_bytes`/`Uuid::from_bytes` via `Id`'s `From<Uuid>`/`Into<Uuid>`.
pub fn receipt_source_id(transfer_id: Id) -> Id {
    let uuid: uuid::Uuid = transfer_id.into();
    let mut bytes = uuid.into_bytes();
    bytes[6] = (bytes[6] & 0x0F) | 0x80;
    bytes[8] = (bytes[8] & 0x1F) | 0xC0;
    Id::from(uuid::Uuid::from_bytes(bytes))
}

// --- C-T1 reads -----------------------------------------------------------------------------------

pub async fn get_transfers<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<StockTransfer>> {
    let rows = TransferEntity::find().order_by_asc(TransferColumn::CreatedAt).order_by_asc(TransferColumn::Id).all(conn).await.map_err(AppError::from)?;
    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        out.push(to_dto(conn, row).await?);
    }
    Ok(out)
}

pub async fn get_transfer<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<StockTransfer> {
    let row = TransferEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("التحويل غير موجود"))?;
    to_dto(conn, &row).await
}

// --- C-T2 create_transfer ---------------------------------------------------------------------

pub async fn create_transfer<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, input: StockTransferInput) -> TxResult<StockTransfer> {
    if input.from_branch_id == input.to_branch_id {
        return Err(AppError::validation("لا يمكن التحويل لنفس الفرع").into());
    }
    let from_branch = BranchEntity::find_by_id(input.from_branch_id).one(conn).await.map_err(AppError::from)?;
    let to_branch = BranchEntity::find_by_id(input.to_branch_id).one(conn).await.map_err(AppError::from)?;
    let (Some(from_branch), Some(to_branch)) = (from_branch, to_branch) else {
        return Err(AppError::not_found("فرع غير موجود").into());
    };
    if input.lines.is_empty() {
        return Err(AppError::validation("أضف صنفاً واحداً على الأقل").into());
    }

    let product_ids: Vec<Id> = input.lines.iter().map(|l| l.product_id).collect();
    let locked = lock_line_products(conn, &product_ids).await?;
    for line in &input.lines {
        let Some(product) = locked.get(&line.product_id) else {
            return Err(AppError::not_found("المنتج غير موجود").into());
        };
        if product.is_untracked() {
            return Err(AppError::validation(format!("\"{}\" لا يُتتبع مخزونها", product.model.name)).into());
        }
        if !(line.qty > Decimal::ZERO) {
            return Err(AppError::validation("الكمية يجب أن تكون أكبر من صفر").into());
        }
    }

    let prefix = branch_prefix(conn, Some(input.from_branch_id)).await?;
    let number = format!("{prefix}{}", numbering::next_number(conn, DocumentKind::StockTransfer).await?);
    let now = cx.clock.now;
    let id = Id::new();
    let date = crate::utils::dates::RawDocDate::parse(&input.date).map_err(|_| AppError::validation("تاريخ غير صالح"))?.resolve(&cx.clock);

    let am = TransferActiveModel {
        id: Set(id),
        number: Set(number.clone()),
        from_branch_id: Set(input.from_branch_id),
        to_branch_id: Set(input.to_branch_id),
        status: Set(status_as_str(StockTransferStatus::Draft).to_string()),
        date_day: Set(date.day),
        date_instant: Set(date.instant),
        date_key: Set(date.key()),
        sent_at_day: Set(None),
        sent_at_instant: Set(None),
        sent_at_key: Set(None),
        received_at_day: Set(None),
        received_at_instant: Set(None),
        received_at_key: Set(None),
        rejected_at_day: Set(None),
        rejected_at_instant: Set(None),
        rejected_at_key: Set(None),
        note: Set(input.note.clone()),
        sent_by: Set(None),
        received_by: Set(None),
        rejected_by: Set(None),
        reject_reason: Set(None),
        shortage_value: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        sync_status: Set("local".to_string()),
    };
    let inserted = am.insert(conn).await.map_err(AppError::from)?;

    for (i, line) in input.lines.iter().enumerate() {
        let line_am = TransferLineActiveModel {
            id: Set(Id::new()),
            stock_transfer_id: Set(id),
            position: Set(i as i16),
            product_id: Set(line.product_id),
            qty: Set(line.qty),
            unit_id: Set(line.unit_id.clone()),
            unit_factor: Set(line.unit_factor),
            batch_id: Set(line.batch_id),
            batch_no: Set(line.batch_no.clone()),
            received_qty: Set(None),
            unit_cost: Set(None),
        };
        line_am.insert(conn).await.map_err(AppError::from)?;
    }

    activity::log(
        conn,
        cx,
        registry,
        crate::entities::platform::activity::ActivityKind::Stock,
        format!("إنشاء تحويل مخزون {number} من {} إلى {}", from_branch.name, to_branch.name),
        Some(date),
        Some(RouteRef::list("transfers")),
    )
    .await?;

    to_dto(conn, &inserted).await
}

// --- C-T3 send_transfer ------------------------------------------------------------------------

pub async fn send_transfer<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, id: Id) -> TxResult<StockTransfer> {
    lock::for_update_by_id(conn, "stock_transfers", &id.to_string()).await.map_err(AppError::from)?;
    let transfer = TransferEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("التحويل غير موجود"))?;
    if parse_status(&transfer.status) != StockTransferStatus::Draft {
        return Err(AppError::validation("لا يمكن إرسال تحويل تم إرساله بالفعل").into());
    }

    let lines = TransferLineEntity::find().filter(TransferLineColumn::StockTransferId.eq(id)).order_by_asc(TransferLineColumn::Position).all(conn).await.map_err(AppError::from)?;
    let product_ids: Vec<Id> = lines.iter().map(|l| l.product_id).collect();
    let mut locked = lock_line_products(conn, &product_ids).await?;

    struct SourceLine {
        product_id: Id,
        qty: Decimal,
        value: Decimal,
    }
    let mut transit_value = Decimal::ZERO;
    let mut source_lines = Vec::with_capacity(lines.len());
    for line in &lines {
        let qty = base_qty(line.qty, line.unit_factor);
        let p = locked.get(&line.product_id).expect("locked above");
        let available = stock::branch_stock_qty(conn, line.product_id, transfer.from_branch_id).await?;
        if qty > available + Decimal::new(1, 4) {
            return Err(AppError::conflict(format!(
                "الكمية المطلوب تحويلها من \"{}\" أكبر من المتوفر بالفرع ({})",
                p.model.name,
                crate::utils::money::js_number_string(available)
            ))
            .into());
        }
        let unit_cost = p.cost_price();
        let value = round2(qty * unit_cost);
        transit_value = round2(transit_value + value);
        source_lines.push(SourceLine { product_id: line.product_id, qty, value });
    }

    let now_date = DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) };
    let actor_id = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;

    // Per-line unit_cost = the first posting entry for its product (Q-I5: two lines of the same
    // product both read the same entry, "first" is a no-op distinction with one entry per product).
    for line in &lines {
        let src = source_lines.iter().find(|s| s.product_id == line.product_id).expect("built above");
        let unit_cost = if src.qty > Decimal::ZERO { round4(src.value / src.qty) } else { Decimal::ZERO };
        let mut line_am: TransferLineActiveModel = line.clone().into();
        line_am.unit_cost = Set(Some(unit_cost));
        line_am.update(conn).await.map_err(AppError::from)?;
    }

    for s in &source_lines {
        let p = locked.get_mut(&s.product_id).expect("locked above");
        stock::apply_change(
            conn,
            cx,
            p,
            -s.qty,
            -s.value,
            "transfer_out",
            StockRef { id: transfer.id, number: transfer.number.clone() },
            &now_date,
            Some(transfer.from_branch_id),
        )
        .await?;
    }

    let mut am: TransferActiveModel = transfer.clone().into();
    am.status = Set(status_as_str(StockTransferStatus::Sent).to_string());
    am.sent_at_day = Set(Some(now_date.day));
    am.sent_at_instant = Set(now_date.instant);
    am.sent_at_key = Set(Some(now_date.key()));
    am.sent_by = Set(Some(actor_id));
    am.updated_at = Set(cx.clock.now);
    let updated = am.update(conn).await.map_err(AppError::from)?;

    let from_branch = BranchEntity::find_by_id(transfer.from_branch_id).one(conn).await.map_err(AppError::from)?;
    let to_branch = BranchEntity::find_by_id(transfer.to_branch_id).one(conn).await.map_err(AppError::from)?;

    post::post(
        conn,
        cx,
        PostJournal {
            date: now_date,
            description: format!(
                "إرسال تحويل مخزون {} — {} → {}",
                transfer.number,
                from_branch.map(|b| b.name).unwrap_or_default(),
                to_branch.map(|b| b.name).unwrap_or_default()
            ),
            entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
            source: Some(SourceRef { kind: "stockAdjustment".to_string(), id: transfer.id, number: Some(transfer.number.clone()) }),
            lines: vec![
                {
                    let mut l = PostingLine::debit(AccountRef::Role(SystemRole::InventoryInTransit), transit_value);
                    l.branch_id = Some(transfer.from_branch_id);
                    l
                },
                {
                    let mut l = PostingLine::credit(AccountRef::Role(SystemRole::Inventory), transit_value);
                    l.branch_id = Some(transfer.from_branch_id);
                    l
                },
            ],
            allow_closed_period: false,
            attachment_ids: Vec::new(),
            template_id: None,
        },
    )
    .await?;

    activity::log(
        conn,
        cx,
        registry,
        crate::entities::platform::activity::ActivityKind::Stock,
        format!("إرسال تحويل {} بقيمة {}", transfer.number, to_fixed2(transit_value)),
        Some(now_date),
        Some(RouteRef::list("transfers")),
    )
    .await?;
    cx.touch(crate::core::events::ChangeCategory::Ledger);
    cx.touch(crate::core::events::ChangeCategory::Catalog);

    to_dto(conn, &updated).await
}

// --- C-T4 receive_transfer ---------------------------------------------------------------------

pub async fn receive_transfer<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, id: Id, input: ReceiveTransferInput) -> TxResult<StockTransfer> {
    lock::for_update_by_id(conn, "stock_transfers", &id.to_string()).await.map_err(AppError::from)?;
    let transfer = TransferEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("التحويل غير موجود"))?;
    if parse_status(&transfer.status) != StockTransferStatus::Sent {
        return Err(AppError::validation("لا يمكن استلام تحويل لم يُرسل بعد").into());
    }

    let lines = TransferLineEntity::find().filter(TransferLineColumn::StockTransferId.eq(id)).order_by_asc(TransferLineColumn::Position).all(conn).await.map_err(AppError::from)?;
    let product_ids: Vec<Id> = lines.iter().map(|l| l.product_id).collect();
    let mut locked = lock_line_products(conn, &product_ids).await?;

    // Input lines keyed by product, last wins (`Map` overwrite semantics, `transfers.ts:133`).
    let mut by_product: std::collections::HashMap<Id, Decimal> = std::collections::HashMap::new();
    for l in &input.lines {
        by_product.insert(l.product_id, l.received_qty);
    }

    struct DestLine {
        product_id: Id,
        qty: Decimal,
        value: Decimal,
    }
    let mut transit_value = Decimal::ZERO;
    let mut received_value = Decimal::ZERO;
    let mut shortage_value = Decimal::ZERO;
    let mut dest_lines = Vec::new();
    let mut received_qtys: Vec<(Id, Decimal)> = Vec::new();

    for line in &lines {
        let sent_qty = base_qty(line.qty, line.unit_factor);
        let unit_cost = line.unit_cost.unwrap_or(Decimal::ZERO);
        let sent_value = round2(sent_qty * unit_cost);
        transit_value = round2(transit_value + sent_value);

        let received_qty = by_product.get(&line.product_id).copied().unwrap_or(sent_qty).min(sent_qty);
        if received_qty < Decimal::ZERO {
            return Err(AppError::validation("الكمية المستلمة لا يمكن أن تكون سالبة").into());
        }
        received_qtys.push((line.id, received_qty));

        let received_line_value = round2(received_qty * unit_cost);
        received_value = round2(received_value + received_line_value);
        if received_qty > Decimal::ZERO {
            dest_lines.push(DestLine { product_id: line.product_id, qty: received_qty, value: received_line_value });
        }
        if received_qty < sent_qty - Decimal::new(1, 4) {
            shortage_value = round2(shortage_value + round2((sent_qty - received_qty) * unit_cost));
        }
    }

    for (line_id, received_qty) in received_qtys {
        let line_model = lines.iter().find(|l| l.id == line_id).expect("built above").clone();
        let mut line_am: TransferLineActiveModel = line_model.into();
        line_am.received_qty = Set(Some(received_qty));
        line_am.update(conn).await.map_err(AppError::from)?;
    }

    let now_date = DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) };
    let actor_id = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;

    for d in &dest_lines {
        let p = locked.get_mut(&d.product_id).expect("locked above");
        stock::apply_change(
            conn,
            cx,
            p,
            d.qty,
            d.value,
            "transfer_in",
            StockRef { id: receipt_source_id(transfer.id), number: transfer.number.clone() },
            &now_date,
            Some(transfer.to_branch_id),
        )
        .await?;
    }

    let mut am: TransferActiveModel = transfer.clone().into();
    am.status = Set(status_as_str(StockTransferStatus::Received).to_string());
    am.received_at_day = Set(Some(now_date.day));
    am.received_at_instant = Set(now_date.instant);
    am.received_at_key = Set(Some(now_date.key()));
    am.received_by = Set(Some(actor_id));
    am.shortage_value = Set(if shortage_value > Decimal::ZERO { Some(shortage_value) } else { None });
    am.updated_at = Set(cx.clock.now);
    let updated = am.update(conn).await.map_err(AppError::from)?;

    let to_branch = BranchEntity::find_by_id(transfer.to_branch_id).one(conn).await.map_err(AppError::from)?;

    let mut posting_lines = vec![
        {
            let mut l = PostingLine::debit(AccountRef::Role(SystemRole::Inventory), received_value);
            l.branch_id = Some(transfer.to_branch_id);
            l
        },
        {
            let mut l = PostingLine::credit(AccountRef::Role(SystemRole::InventoryInTransit), transit_value);
            l.branch_id = Some(transfer.to_branch_id);
            l
        },
    ];
    if shortage_value > Decimal::ZERO {
        let mut l = PostingLine::debit(AccountRef::Role(SystemRole::InventoryVariance), shortage_value);
        l.description = Some(format!("عجز تحويل {}", transfer.number));
        l.branch_id = Some(transfer.to_branch_id);
        posting_lines.push(l);
    }

    post::post(
        conn,
        cx,
        PostJournal {
            date: now_date,
            description: format!("استلام تحويل مخزون {} في {}", transfer.number, to_branch.map(|b| b.name).unwrap_or_default()),
            entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
            source: Some(SourceRef { kind: "stockAdjustment".to_string(), id: receipt_source_id(transfer.id), number: Some(transfer.number.clone()) }),
            lines: posting_lines,
            allow_closed_period: false,
            attachment_ids: Vec::new(),
            template_id: None,
        },
    )
    .await?;

    let shortage_suffix = if shortage_value > Decimal::ZERO { format!(" — عجز {}", to_fixed2(shortage_value)) } else { String::new() };
    activity::log(
        conn,
        cx,
        registry,
        crate::entities::platform::activity::ActivityKind::Stock,
        format!("استلام تحويل {}{shortage_suffix}", transfer.number),
        Some(now_date),
        Some(RouteRef::list("transfers")),
    )
    .await?;
    cx.touch(crate::core::events::ChangeCategory::Ledger);
    cx.touch(crate::core::events::ChangeCategory::Catalog);

    to_dto(conn, &updated).await
}

// --- C-T5 reject_transfer ----------------------------------------------------------------------

pub async fn reject_transfer<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, id: Id, reason: String) -> TxResult<StockTransfer> {
    lock::for_update_by_id(conn, "stock_transfers", &id.to_string()).await.map_err(AppError::from)?;
    let transfer = TransferEntity::find_by_id(id).one(conn).await.map_err(AppError::from)?.ok_or_else(|| AppError::not_found("التحويل غير موجود"))?;
    if parse_status(&transfer.status) != StockTransferStatus::Sent {
        return Err(AppError::validation("لا يمكن رفض تحويل لم يُرسل بعد").into());
    }
    let reason_trimmed = reason.trim().to_string();
    if reason_trimmed.is_empty() {
        return Err(AppError::validation("سبب الرفض مطلوب").into());
    }

    let lines = TransferLineEntity::find().filter(TransferLineColumn::StockTransferId.eq(id)).order_by_asc(TransferLineColumn::Position).all(conn).await.map_err(AppError::from)?;
    let product_ids: Vec<Id> = lines.iter().map(|l| l.product_id).collect();
    let mut locked = lock_line_products(conn, &product_ids).await?;

    struct SourceLine {
        product_id: Id,
        qty: Decimal,
        value: Decimal,
    }
    let mut transit_value = Decimal::ZERO;
    let mut source_lines = Vec::with_capacity(lines.len());
    for line in &lines {
        let qty = base_qty(line.qty, line.unit_factor);
        let unit_cost = line.unit_cost.unwrap_or(Decimal::ZERO);
        let value = round2(qty * unit_cost);
        transit_value = round2(transit_value + value);
        source_lines.push(SourceLine { product_id: line.product_id, qty, value });
    }

    let now_date = DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) };
    let actor_id = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;

    let mut am: TransferActiveModel = transfer.clone().into();
    am.status = Set(status_as_str(StockTransferStatus::Rejected).to_string());
    am.rejected_at_day = Set(Some(now_date.day));
    am.rejected_at_instant = Set(now_date.instant);
    am.rejected_at_key = Set(Some(now_date.key()));
    am.rejected_by = Set(Some(actor_id));
    am.reject_reason = Set(Some(reason_trimmed.clone()));
    am.updated_at = Set(cx.clock.now);
    let updated = am.update(conn).await.map_err(AppError::from)?;

    for s in &source_lines {
        let p = locked.get_mut(&s.product_id).expect("locked above");
        stock::apply_change(
            conn,
            cx,
            p,
            s.qty,
            s.value,
            "transfer_in",
            StockRef { id: receipt_source_id(transfer.id), number: transfer.number.clone() },
            &now_date,
            Some(transfer.from_branch_id),
        )
        .await?;
    }

    post::post(
        conn,
        cx,
        PostJournal {
            date: now_date,
            description: format!("رفض تحويل مخزون {} — {reason_trimmed}", transfer.number),
            entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
            source: Some(SourceRef { kind: "stockAdjustment".to_string(), id: receipt_source_id(transfer.id), number: Some(transfer.number.clone()) }),
            lines: vec![
                {
                    let mut l = PostingLine::debit(AccountRef::Role(SystemRole::Inventory), transit_value);
                    l.branch_id = Some(transfer.from_branch_id);
                    l
                },
                {
                    let mut l = PostingLine::credit(AccountRef::Role(SystemRole::InventoryInTransit), transit_value);
                    l.branch_id = Some(transfer.from_branch_id);
                    l
                },
            ],
            allow_closed_period: false,
            attachment_ids: Vec::new(),
            template_id: None,
        },
    )
    .await?;

    activity::log(
        conn,
        cx,
        registry,
        crate::entities::platform::activity::ActivityKind::Stock,
        format!("رفض تحويل {} — {reason_trimmed}", transfer.number),
        Some(now_date),
        Some(RouteRef::list("transfers")),
    )
    .await?;
    cx.touch(crate::core::events::ChangeCategory::Ledger);
    cx.touch(crate::core::events::ChangeCategory::Catalog);

    to_dto(conn, &updated).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MariaDB 11.4 refuses a v8 UUID whose byte 8 is `0x01..=0x80`; the derived id must always
    /// land in `0xC0..=0xDF` (variant `110x`) and keep the version nibble 8 (never a v7 id).
    #[test]
    fn receipt_source_id_is_always_a_mariadb_valid_v8() {
        for b8 in [0x00u8, 0x3F, 0x80, 0x8F, 0xBF, 0xC0, 0xFF] {
            let mut bytes = [0x01u8, 0xA0, 0xF0, 0x57, 0x0B, 0xFF, 0x75, 0xD2, b8, 0xCC, 0xEE, 0x28, 0xBD, 0xD3, 0xC8, 0x21];
            bytes[8] = b8;
            let derived: uuid::Uuid = receipt_source_id(Id::from(uuid::Uuid::from_bytes(bytes))).into();
            let out = derived.into_bytes();
            assert_eq!(out[6] >> 4, 8, "version nibble");
            assert!((0xC0..=0xDF).contains(&out[8]), "byte 8 {:#x} from {b8:#x}", out[8]);
            assert_eq!(receipt_source_id(Id::from(uuid::Uuid::from_bytes(bytes))), Id::from(derived), "deterministic");
        }
    }
}
