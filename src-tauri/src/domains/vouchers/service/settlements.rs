//! Card/wallet settlement (10-vouchers.md §3.5-3.7), porting `src/mocks/backend/settlements.ts`.
//! Clears `cardClearing`/`walletClearing` when the bank deposit arrives, with a DB-enforced
//! one-settlement-per-(day, method) rule (`uq_card_settlement_groups_date_method`, PG-3).

use std::collections::{BTreeMap, HashSet};

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::core::error::{map_unique_violation, AppError};
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::org::payment_methods::{Column as PaymentMethodColumn, Entity as PaymentMethodEntity};
use crate::entities::payments::card_settlement_groups::{ActiveModel as GroupActiveModel, Column as GroupColumn, Entity as GroupEntity};
use crate::entities::payments::card_settlements::{ActiveModel as SettlementActiveModel, Model as SettlementModel};
use crate::entities::platform::activity::ActivityKind;
use crate::entities::sales::invoice_tenders::{Column as TenderColumn, Entity as TenderEntity};
use crate::entities::sales::invoices::{Column as InvoiceColumn, Entity as InvoiceEntity, InvoiceStatus};
use crate::entities::soft_delete::SoftDelete;
use crate::shared::activity;
use crate::shared::ledger::accounts::SystemRole;
use crate::shared::ledger::period::assert_open_period;
use crate::shared::ledger::post::{self, AccountRef, PostJournal, PostingLine, SourceRef};
use crate::shared::numbering::{self, DocumentKind};
use crate::utils::dates::RawDocDate;
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::route::RouteRef;

use super::super::dto::{
    CardSettlement, CardSettlementGroup, CardSettlementInput, ClearingRole, FeeEstimate, UnsettledTenderGroup,
};

/// Already-settled (day, method) keys (`settledKeys`, `settlements.ts:19-24`) — every group of every
/// posted settlement.
async fn settled_keys<C: ConnectionTrait>(conn: &C) -> TxResult<HashSet<(chrono::NaiveDate, Id)>> {
    let groups = GroupEntity::find().all(conn).await.map_err(TxError::from)?;
    Ok(groups.into_iter().map(|g| (g.date_day, g.payment_method_id)).collect())
}

/// **3.5 `unsettled_tender_groups`** (`settlements.ts:16-55`): card/wallet tenders on `COMPLETED`,
/// live invoices, grouped by (business day, payment method), excluding any (day, method) already
/// covered by a posted settlement. Q-V2 kept as-is: a `REFUNDED` invoice's tenders simply drop out
/// (the `status = 'COMPLETED'` filter), and a tender on an already-settled (day, method) is never
/// re-offered.
pub async fn unsettled_tender_groups<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<UnsettledTenderGroup>> {
    let clearing_methods = PaymentMethodEntity::find_live()
        .filter(PaymentMethodColumn::AccountRole.is_in(["cardClearing", "walletClearing"]))
        .all(conn)
        .await
        .map_err(TxError::from)?;
    if clearing_methods.is_empty() {
        return Ok(Vec::new());
    }
    let method_by_id: BTreeMap<Id, &crate::entities::org::payment_methods::Model> = clearing_methods.iter().map(|m| (m.id, m)).collect();
    let already = settled_keys(conn).await?;

    // Live, COMPLETED invoices with any tender at all — then filter tenders to clearing methods in
    // Rust (mirrors the mock's nested loop; avoids a raw-SQL decimal aggregate). `invoices` has no
    // `SoftDelete` impl (unlike `payment_methods`/`parties`/`expense_categories`), so "live" is a
    // plain `DeletedAt.is_null()` filter here, same as the `payments` domain's own invoice reads.
    let invoices = InvoiceEntity::find()
        .filter(InvoiceColumn::Status.eq(InvoiceStatus::Completed))
        .filter(InvoiceColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(TxError::from)?;
    if invoices.is_empty() {
        return Ok(Vec::new());
    }
    let invoice_days: BTreeMap<Id, chrono::NaiveDate> = invoices.iter().map(|i| (i.id, i.date_day)).collect();
    let invoice_ids: Vec<Id> = invoices.iter().map(|i| i.id).collect();

    let tenders = TenderEntity::find().filter(TenderColumn::InvoiceId.is_in(invoice_ids)).all(conn).await.map_err(TxError::from)?;

    let mut totals: BTreeMap<(chrono::NaiveDate, Id), UnsettledTenderGroup> = BTreeMap::new();
    for t in tenders {
        let Some(method) = method_by_id.get(&t.payment_method_id) else { continue };
        let Some(&day) = invoice_days.get(&t.invoice_id) else { continue };
        let key = (day, method.id);
        if already.contains(&key) {
            continue;
        }
        let role = ClearingRole::from_account_role_str(&method.account_role).expect("filtered to clearing roles above");
        totals
            .entry(key)
            .and_modify(|g| {
                g.total = round2(g.total + t.amount);
                g.tender_count += 1;
            })
            .or_insert(UnsettledTenderGroup {
                date: day.format("%Y-%m-%d").to_string(),
                payment_method_id: method.id,
                payment_method_name: method.name.clone(),
                account_role: role,
                total: round2(t.amount),
                tender_count: 1,
            });
    }

    let mut groups: Vec<UnsettledTenderGroup> = totals.into_values().collect();
    // D-V4: sort by `date DESC, name ASC` with Rust `str` ordering.
    groups.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| a.payment_method_name.cmp(&b.payment_method_name)));
    Ok(groups)
}

/// **3.7 `estimate_settlement_fee`** (`settlements.ts:120-127`): `round2(Σ total × feePct / 100)`,
/// summed exactly then rounded once. Pure client-trusting estimate (Q-V4) — never the posted
/// `fee_amount`.
pub async fn estimate_settlement_fee<C: ConnectionTrait>(conn: &C, groups: Vec<UnsettledTenderGroup>) -> TxResult<FeeEstimate> {
    let mut sum = Decimal::ZERO;
    for g in &groups {
        let method = PaymentMethodEntity::find_by_id(g.payment_method_id).one(conn).await.map_err(TxError::from)?;
        let fee_pct = method.map(|m| m.fee_pct).unwrap_or(Decimal::ZERO);
        sum += g.total * fee_pct / Decimal::from(100);
    }
    Ok(FeeEstimate(round2(sum)))
}

/// PG-3: the race loser on `uq_card_settlement_groups_date_method` (two terminals settling an
/// overlapping day×method) gets the same `CONFLICT` message a stale-list caller would see in step
/// 3 above — the DB constraint is the actual serialization point (analysis §5's recommendation).
fn map_settlement_conflict(err: sea_orm::DbErr) -> TxError {
    map_unique_violation(err, "uq_card_settlement_groups_date_method", || {
        "أحد العناصر المختارة غير متاح للتسوية (ربما تمت تسويته بالفعل)".to_string()
    })
}

struct SelectedGroup {
    date: chrono::NaiveDate,
    payment_method_id: Id,
    amount: Decimal,
    role: ClearingRole,
}

/// **3.6 `create_card_settlement`** (`settlements.ts:58-111`).
pub async fn create_card_settlement<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::undo::UndoRegistry,
    input: CardSettlementInput,
) -> TxResult<CardSettlement> {
    if input.groups.is_empty() {
        return Err(TxError::App(AppError::validation("اختر يوماً واحداً على الأقل للتسوية")));
    }
    if !(input.deposit_amount >= Decimal::ZERO) {
        return Err(TxError::App(AppError::validation("أدخل مبلغ الإيداع البنكي")));
    }

    let available = unsettled_tender_groups(conn).await?;
    let available_by_key: BTreeMap<(String, Id), &UnsettledTenderGroup> =
        available.iter().map(|g| ((g.date.clone(), g.payment_method_id), g)).collect();

    let mut selected: Vec<SelectedGroup> = Vec::with_capacity(input.groups.len());
    for g in &input.groups {
        let key = (g.date.clone(), g.payment_method_id);
        let found = available_by_key
            .get(&key)
            .ok_or_else(|| AppError::conflict("أحد العناصر المختارة غير متاح للتسوية (ربما تمت تسويته بالفعل)"))?;
        let day = RawDocDate::parse(&g.date)
            .map_err(|_| AppError::validation("التاريخ غير صالح"))?
            .resolve(&cx.clock)
            .day;
        selected.push(SelectedGroup { date: day, payment_method_id: g.payment_method_id, amount: found.total, role: found.account_role });
    }

    let gross_amount = round2(selected.iter().fold(Decimal::ZERO, |a, s| a + s.amount));
    let fee_amount = round2(gross_amount - input.deposit_amount);
    if fee_amount < Decimal::new(-5, 3) {
        // < -0.005
        return Err(TxError::App(AppError::validation("مبلغ الإيداع أكبر من إجمالي العمليات المختارة")));
    }

    let date = RawDocDate::parse(&input.date).map_err(|_| AppError::validation("التاريخ غير صالح"))?.resolve(&cx.clock);
    assert_open_period(conn, &date.day, false).await?;

    let number = numbering::next_number(conn, DocumentKind::CardSettlement).await?;
    let created_by = cx.actor.as_ref().ok_or_else(|| AppError::unauthorized("سجّل الدخول أولاً"))?.id;
    let now = cx.clock.now;
    let id = Id::new();
    let deposit_amount = round2(input.deposit_amount);

    let settlement_model = SettlementActiveModel {
        id: Set(id),
        number: Set(number.clone()),
        date_day: Set(date.day),
        date_instant: Set(date.instant),
        gross_amount: Set(gross_amount),
        deposit_amount: Set(deposit_amount),
        fee_amount: Set(fee_amount),
        note: Set(input.note.clone()),
        created_by: Set(created_by),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
        sync_status: Set(crate::entities::payments::card_settlements::SyncStatus::Local),
    };
    // PG-3: the race loser on the (date_day, payment_method_id) unique constraint gets the same
    // CONFLICT message step 3 above gives a stale-list caller. The constraint lives on
    // `card_settlement_groups`, not `card_settlements` itself, but a concurrent insert on the
    // child table can only fail once this parent row already exists, so mapping it here as well
    // costs nothing and keeps both insert sites symmetric.
    let inserted: SettlementModel = settlement_model.insert(conn).await.map_err(map_settlement_conflict)?;

    for (position, s) in selected.iter().enumerate() {
        let group_model = GroupActiveModel {
            id: Set(Id::new()),
            card_settlement_id: Set(id),
            position: Set(position as i16),
            date_day: Set(s.date),
            payment_method_id: Set(s.payment_method_id),
            amount: Set(s.amount),
        };
        group_model.insert(conn).await.map_err(map_settlement_conflict)?;
    }

    // Group by clearing role so a settlement mixing card + wallet gets one credit line per role, in
    // first-appearance order (`settlements.ts:91-94`).
    let mut role_order: Vec<ClearingRole> = Vec::new();
    let mut by_role: BTreeMap<u8, Decimal> = BTreeMap::new();
    for s in &selected {
        let key = match s.role {
            ClearingRole::CardClearing => 0u8,
            ClearingRole::WalletClearing => 1u8,
        };
        if !role_order.contains(&s.role) {
            role_order.push(s.role);
        }
        by_role.entry(key).and_modify(|v| *v = round2(*v + s.amount)).or_insert(round2(s.amount));
    }

    let mut lines = vec![PostingLine::debit(AccountRef::Role(SystemRole::Bank), deposit_amount)];
    if fee_amount > Decimal::ZERO {
        lines.push(PostingLine::debit(AccountRef::Role(SystemRole::CardFees), fee_amount));
    }
    for role in &role_order {
        let key = match role {
            ClearingRole::CardClearing => 0u8,
            ClearingRole::WalletClearing => 1u8,
        };
        let amount = *by_role.get(&key).expect("role_order only contains keys inserted into by_role");
        lines.push(PostingLine::credit(AccountRef::Role(role.as_system_role()), amount));
    }
    if fee_amount < Decimal::ZERO {
        lines.push(PostingLine::debit(AccountRef::Role(SystemRole::Bank), -fee_amount));
    }

    post::post(
        conn,
        cx,
        PostJournal {
            date,
            description: format!("تسوية بطاقات/محافظ {number} — {} مجموعة", selected.len()),
            entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
            source: Some(SourceRef { kind: "settlement".to_string(), id, number: Some(number.clone()) }),
            lines,
            allow_closed_period: false,
            attachment_ids: Vec::new(),
            template_id: None,
        },
    )
    .await?;

    activity::record::log(
        conn,
        cx,
        registry,
        ActivityKind::Payment,
        format!("تسوية بطاقات {number} — إيداع {deposit_amount:.2} وعمولة {fee_amount:.2}"),
        Some(date),
        Some(RouteRef::list("card-settlements")),
    )
    .await?;

    settlement_to_dto(conn, &inserted).await
}

async fn settlement_to_dto<C: ConnectionTrait>(conn: &C, m: &SettlementModel) -> TxResult<CardSettlement> {
    let mut groups = GroupEntity::find().filter(GroupColumn::CardSettlementId.eq(m.id)).all(conn).await.map_err(TxError::from)?;
    groups.sort_by_key(|g| g.position);
    Ok(CardSettlement {
        id: m.id,
        number: m.number.clone(),
        date: m.date().key(),
        groups: groups
            .into_iter()
            .map(|g| CardSettlementGroup { date: g.date_day.format("%Y-%m-%d").to_string(), payment_method_id: g.payment_method_id, amount: g.amount })
            .collect(),
        gross_amount: m.gross_amount,
        deposit_amount: m.deposit_amount,
        fee_amount: m.fee_amount,
        note: m.note.clone(),
        created_by: m.created_by,
    })
}

pub async fn get_card_settlement_dto<C: ConnectionTrait>(conn: &C, m: &SettlementModel) -> TxResult<CardSettlement> {
    settlement_to_dto(conn, m).await
}
