//! `opening.rs` (02-setup.md §3.6, `opening.ts:59-240`): the step-8 opening-balances review/post,
//! closing 3900, and per-branch opening stock. Constants re-exported from 00-import's `idmap.rs`
//! (D-7 there).

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter};

use crate::core::error::AppError;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::journal::journal_entries::{Entity as JournalEntryEntity, JournalEntryType};
use crate::entities::journal::journal_lines::{Column as LineColumn, Entity as LineEntity, PartyKind as EntityPartyKind};
use crate::shared::activity::undo::UndoRegistry;
use crate::shared::ledger::accounts::{AccountCtx, SystemRole};
use crate::shared::ledger::post::{AccountRef, PartyRef, PostJournal, PostingLine};
use crate::utils::dates::DocDate;
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::route::RouteRef;

use crate::infrastructure::import::idmap::{ONBOARDING_CLOSE_SOURCE_ID, ONBOARDING_SOURCE_ID};

use super::super::dto::{CloseTarget, OpeningEntryInput, PostOpeningBalancesResult, PostingSide};

/// `openingBalanceEquityNet` (`opening.ts:65-74`): `round2(Σ debit − credit)` over `journal_lines`
/// of the `openingBalanceEquity` system-role account.
pub async fn get_opening_balance_equity_net<C: ConnectionTrait>(conn: &C) -> TxResult<Decimal> {
    let account = crate::shared::ledger::accounts::resolve_account(conn, SystemRole::OpeningBalanceEquity, &AccountCtx::default()).await?;
    let lines = LineEntity::find().filter(LineColumn::AccountId.eq(account.id)).all(conn).await.map_err(TxError::from)?;
    let net: Decimal = lines.iter().fold(Decimal::ZERO, |acc, l| acc + (l.debit - l.credit));
    Ok(round2(net))
}

/// `isFirstUsePosted` (`opening.ts:60-62`): any journal entry with `source_kind IN ('invoice',
/// 'purchaseOrder')`.
pub async fn is_first_use_posted<C: ConnectionTrait>(conn: &C) -> TxResult<bool> {
    use crate::entities::journal::journal_entries::Column as EntryColumn;
    let count = JournalEntryEntity::find()
        .filter(EntryColumn::SourceKind.is_in(["invoice", "purchaseOrder"]))
        .count(conn)
        .await
        .map_err(TxError::from)?;
    Ok(count > 0)
}

/// `buildOpeningLines` (`opening.ts:80-119`): cash + customer + supplier + other lines; any
/// `amount == 0` skipped; the balancing 3900 line on the short side when `|diff| > 0.001`.
fn build_opening_lines(input: &OpeningEntryInput) -> Vec<PostingLine> {
    let mut lines: Vec<PostingLine> = Vec::new();

    for c in &input.cash {
        if c.amount.is_zero() {
            continue;
        }
        lines.push(PostingLine {
            account: AccountRef::Id(c.account_id),
            debit: c.amount,
            credit: Decimal::ZERO,
            description: None,
            party: None,
            branch_id: None,
            cost_center_id: None,
            currency: c.currency.clone(),
            amount_fc: c.amount_fc,
            rate: c.rate,
            trace: None,
        });
    }

    for cust in &input.customers {
        if cust.amount.is_zero() {
            continue;
        }
        let debit = if cust.side == PostingSide::Debit { cust.amount } else { Decimal::ZERO };
        let credit = if cust.side == PostingSide::Credit { cust.amount } else { Decimal::ZERO };
        lines.push(PostingLine {
            account: AccountRef::Role(SystemRole::Receivable),
            debit,
            credit,
            description: None,
            party: Some(PartyRef { kind: EntityPartyKind::Customer, id: cust.party_id }),
            branch_id: None,
            cost_center_id: None,
            currency: None,
            amount_fc: None,
            rate: None,
            trace: None,
        });
    }

    for supp in &input.suppliers {
        if supp.amount.is_zero() {
            continue;
        }
        let debit = if supp.side == PostingSide::Debit { supp.amount } else { Decimal::ZERO };
        let credit = if supp.side == PostingSide::Credit { supp.amount } else { Decimal::ZERO };
        lines.push(PostingLine {
            account: AccountRef::Role(SystemRole::Payable),
            debit,
            credit,
            description: None,
            party: Some(PartyRef { kind: EntityPartyKind::Supplier, id: supp.party_id }),
            branch_id: None,
            cost_center_id: None,
            currency: None,
            amount_fc: None,
            rate: None,
            trace: None,
        });
    }

    for o in &input.other {
        if o.amount.is_zero() {
            continue;
        }
        let debit = if o.side == PostingSide::Debit { o.amount } else { Decimal::ZERO };
        let credit = if o.side == PostingSide::Credit { o.amount } else { Decimal::ZERO };
        lines.push(PostingLine {
            account: AccountRef::Id(o.account_id),
            debit,
            credit,
            description: o.description.clone(),
            party: None,
            branch_id: None,
            cost_center_id: None,
            currency: None,
            amount_fc: None,
            rate: None,
            trace: None,
        });
    }

    let debit_total: Decimal = lines.iter().fold(Decimal::ZERO, |a, l| a + l.debit);
    let credit_total: Decimal = lines.iter().fold(Decimal::ZERO, |a, l| a + l.credit);
    let diff = round2(debit_total - credit_total);
    if diff.abs() > Decimal::new(1, 3) {
        if diff > Decimal::ZERO {
            lines.push(PostingLine {
                account: AccountRef::Role(SystemRole::OpeningBalanceEquity),
                debit: Decimal::ZERO,
                credit: diff,
                description: None,
                party: None,
                branch_id: None,
                cost_center_id: None,
                currency: None,
                amount_fc: None,
                rate: None,
                trace: None,
            });
        } else {
            lines.push(PostingLine {
                account: AccountRef::Role(SystemRole::OpeningBalanceEquity),
                debit: -diff,
                credit: Decimal::ZERO,
                description: None,
                party: None,
                branch_id: None,
                cost_center_id: None,
                currency: None,
                amount_fc: None,
                rate: None,
                trace: None,
            });
        }
    }

    lines
}

/// `closeOpeningBalanceEquity` (`:148-168`): moves 3900's balance to `capital`/`ownerCurrent`.
/// `None` when `|net| < 0.01` (nothing to close).
async fn close_opening_balance_equity<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    date: DocDate,
    target: CloseTarget,
) -> TxResult<Option<Id>> {
    let net = get_opening_balance_equity_net(conn).await?;
    if net.abs() < Decimal::new(1, 2) {
        return Ok(None);
    }

    let target_role = match target {
        CloseTarget::Capital => SystemRole::Capital,
        CloseTarget::OwnerCurrent => SystemRole::OwnerCurrent,
    };

    let lines = if net > Decimal::ZERO {
        vec![
            PostingLine::credit(AccountRef::Role(SystemRole::OpeningBalanceEquity), net),
            PostingLine::debit(AccountRef::Role(target_role), net),
        ]
    } else {
        vec![
            PostingLine::debit(AccountRef::Role(SystemRole::OpeningBalanceEquity), -net),
            PostingLine::credit(AccountRef::Role(target_role), -net),
        ]
    };

    let target_label = match target {
        CloseTarget::Capital => "رأس المال",
        CloseTarget::OwnerCurrent => "جاري المالك",
    };

    let entry = crate::shared::ledger::post::post(
        conn,
        cx,
        PostJournal {
            date,
            description: format!("إقفال حساب الأرصدة الافتتاحية (3900) إلى {target_label}"),
            entry_type: JournalEntryType::Closing,
            source: Some(crate::shared::ledger::post::SourceRef { kind: "opening".to_string(), id: ONBOARDING_CLOSE_SOURCE_ID, number: Some("OPENING-CLOSE".to_string()) }),
            lines,
            allow_closed_period: true,
            attachment_ids: Vec::new(),
            template_id: None,
        },
    )
    .await?;

    crate::shared::activity::log(
        conn,
        cx,
        registry,
        crate::entities::platform::activity::ActivityKind::Journal,
        "إقفال حساب الأرصدة الافتتاحية",
        None,
        Some(RouteRef::list("journal")),
    )
    .await?;

    Ok(Some(entry.id))
}

/// `postOpeningBalances` (`setupService.ts:187-203`): posts the opening entry, then closes 3900 —
/// both inside the caller's transaction. Locks settings exclusively first (D-11).
pub async fn post_opening_balances<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    input: OpeningEntryInput,
    close_target: CloseTarget,
) -> TxResult<PostOpeningBalancesResult> {
    // D-11: lock settings exclusively first (no lock upgrade with `ledger::post`, which itself
    // only ever shares-locks the settings row — see `shared::ledger::period`).
    let _locked = crate::core::settings::load_shared_locked(conn).await.map_err(TxError::App)?;

    let date_naive = chrono::NaiveDate::parse_from_str(&input.date, "%Y-%m-%d").map_err(|_| AppError::validation("تاريخ غير صالح"))?;
    let doc_date = DocDate::from(date_naive);

    let lines = build_opening_lines(&input);

    let entry = crate::shared::ledger::post::post(
        conn,
        cx,
        PostJournal {
            date: doc_date,
            description: "القيد الافتتاحي".to_string(),
            entry_type: JournalEntryType::Opening,
            source: Some(crate::shared::ledger::post::SourceRef { kind: "opening".to_string(), id: ONBOARDING_SOURCE_ID, number: Some("OPENING".to_string()) }),
            lines,
            allow_closed_period: true,
            attachment_ids: Vec::new(),
            template_id: None,
        },
    )
    .await?;

    crate::shared::activity::log(
        conn,
        cx,
        registry,
        crate::entities::platform::activity::ActivityKind::Journal,
        "ترحيل القيد الافتتاحي",
        None,
        Some(RouteRef::list("journal")),
    )
    .await?;

    let closing_id = close_opening_balance_equity(conn, cx, registry, doc_date, close_target).await?;

    // `onboarding.openingEntryId`/`closingEntryId` (None clears).
    let locked = crate::core::settings::load_shared_locked(conn).await.map_err(TxError::App)?;
    let mut onboarding = locked.onboarding.clone().unwrap_or(crate::entities::values::OnboardingState {
        business_type: None,
        go_live_date: None,
        completed_step: None,
        skipped: Vec::new(),
        done: Vec::new(),
        finished_at: None,
        opening_entry_id: None,
        closing_entry_id: None,
        coa_template: None,
    });
    onboarding.opening_entry_id = Some(entry.id);
    onboarding.closing_entry_id = closing_id;
    let mut settings_model: crate::entities::org::settings::ActiveModel = locked.into();
    use sea_orm::{ActiveModelTrait, Set};
    settings_model.onboarding = Set(Some(onboarding));
    settings_model.updated_at = Set(cx.clock.now);
    settings_model.update(conn).await.map_err(TxError::from)?;

    Ok(PostOpeningBalancesResult { opening_entry_id: entry.id, closing_entry_id: closing_id })
}

/// `recloseOpeningBalanceEquity` — this alone (idempotent no-op at zero).
pub async fn reclose_opening_balance_equity<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    date: chrono::NaiveDate,
    target: CloseTarget,
) -> TxResult<()> {
    let _locked = crate::core::settings::load_shared_locked(conn).await.map_err(TxError::App)?;
    close_opening_balance_equity(conn, cx, registry, DocDate::from(date), target).await?;
    Ok(())
}

/// `postOpeningStockForBranch` (`:203-240`): per-branch opening stock. Unknown branch -> `NOT_FOUND`
/// (D-8). Filters out non-positive-qty lines; empty -> no-op (no error, no entry).
pub async fn post_opening_stock<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    branch_id: Id,
    date: chrono::NaiveDate,
    lines: Vec<super::super::dto::OpeningStockLine>,
) -> TxResult<()> {
    use crate::entities::org::branches::Entity as BranchEntity;

    let branch = BranchEntity::find_by_id(branch_id).one(conn).await.map_err(TxError::from)?.ok_or_else(|| AppError::not_found("الفرع غير موجود"))?;

    let valid: Vec<&super::super::dto::OpeningStockLine> = lines.iter().filter(|l| l.qty > Decimal::ZERO).collect();
    if valid.is_empty() {
        return Ok(());
    }

    let ids: Vec<Id> = valid.iter().map(|l| l.product_id).collect();
    let mut locked_products = crate::shared::stock::lock_products(conn, &ids).await?;

    let adj_ref_id = Id::new();
    let doc_date = DocDate::from(date);
    let mut total = Decimal::ZERO;

    for line in &valid {
        let p = locked_products.get_mut(&line.product_id).ok_or_else(|| AppError::not_found("المنتج غير موجود"))?;
        let unit_cost = if line.unit_cost.is_zero() { p.cost_price() } else { line.unit_cost };
        let value = round2(line.qty * unit_cost);
        total += value;

        crate::shared::stock::apply_change(
            conn,
            cx,
            p,
            line.qty,
            value,
            "stock_in",
            crate::shared::stock::StockRef { id: adj_ref_id, number: "OPENING-STOCK".to_string() },
            &doc_date,
            Some(branch_id),
        )
        .await?;

        if p.track_batches() {
            if let Some(batch_no) = &line.batch_no {
                let expiry = line.expiry_date.as_deref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
                crate::shared::stock::batches::receive_batch(
                    conn,
                    cx,
                    line.product_id,
                    line.qty,
                    unit_cost,
                    batch_no.clone(),
                    expiry,
                    &doc_date,
                    &crate::shared::stock::StockRef { id: adj_ref_id, number: "OPENING-STOCK".to_string() },
                )
                .await?;
            }
        }
    }

    total = round2(total);
    if total <= Decimal::ZERO {
        // Q-5: stock movements posted above with no journal entry — kept quirk, matches the mock.
        return Ok(());
    }

    let inventory_account = crate::shared::ledger::accounts::resolve_account(
        conn,
        SystemRole::Inventory,
        &AccountCtx { branch_id: Some(branch_id), currency: None },
    )
    .await?;
    let equity_account = crate::shared::ledger::accounts::resolve_account(conn, SystemRole::OpeningBalanceEquity, &AccountCtx::default()).await?;

    crate::shared::ledger::post::post(
        conn,
        cx,
        PostJournal {
            date: doc_date,
            description: format!("رصيد افتتاحي للمخزون — {}", branch.name),
            entry_type: JournalEntryType::Opening,
            source: Some(crate::shared::ledger::post::SourceRef { kind: "opening".to_string(), id: adj_ref_id, number: Some("OPENING-STOCK".to_string()) }),
            lines: vec![
                PostingLine::debit(AccountRef::Id(inventory_account.id), total),
                PostingLine::credit(AccountRef::Id(equity_account.id), total),
            ],
            allow_closed_period: true,
            attachment_ids: Vec::new(),
            template_id: None,
        },
    )
    .await?;

    crate::shared::activity::log(
        conn,
        cx,
        registry,
        crate::entities::platform::activity::ActivityKind::Stock,
        format!("رصيد افتتاحي للمخزون — {} صنف", valid.len()),
        Some(doc_date),
        Some(RouteRef::list("movements")),
    )
    .await?;

    Ok(())
}
