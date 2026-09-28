//! `revaluation.rs` — `get_revaluation_preview`/`get_default_revaluation_rates`/`post_revaluation`
//! (01-settings.md §3 "Revaluation", D-10: ported here because these three functions are
//! `settings.*` even though `revaluation.ts` is its own mock file).

use std::collections::BTreeMap;

use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::{AppError, AppResult};
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::journal::journal_lines::{Column as LineColumn, Entity as LineEntity, PartyKind};
use crate::entities::org::accounts::{Column as AccountColumn, Entity as AccountEntity};
use crate::entities::org::currencies::{Column as CurrencyColumn, Entity as CurrencyEntity};
use crate::entities::org::exchange_rates::{Column as RateColumn, Entity as RateEntity};
use crate::entities::parties::parties::Entity as PartyEntity;
use crate::entities::platform::activity::ActivityKind;
use crate::shared::activity::undo::UndoRegistry;
use crate::shared::ledger::accounts::SystemRole;
use crate::shared::ledger::{self, AccountRef, PartyRef, PostJournal, PostingLine};
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::route::RouteRef;

use super::super::dto::{FcBalanceKind, FcBalanceRow, OrderedNumberMap, RevaluationResult};

/// `openFcBalances` (`revaluation.ts:26-69`).
pub async fn open_fc_balances<C: ConnectionTrait>(conn: &C, rates: &BTreeMap<String, Decimal>) -> AppResult<Vec<FcBalanceRow>> {
    let mut rows: Vec<FcBalanceRow> = Vec::new();

    for (party_kind, role, dto_kind) in [
        (PartyKind::Customer, SystemRole::Receivable, FcBalanceKind::Customer),
        (PartyKind::Supplier, SystemRole::Payable, FcBalanceKind::Supplier),
    ] {
        // First live account with this system role, in (created_at, id) order.
        let account = AccountEntity::find()
            .filter(AccountColumn::SystemRole.eq(role.as_str()))
            .filter(AccountColumn::DeletedAt.is_null())
            .order_by_asc(AccountColumn::CreatedAt)
            .order_by_asc(AccountColumn::Id)
            .one(conn)
            .await
            .map_err(AppError::from)?;
        let Some(account) = account else { continue };

        let lines = LineEntity::find()
            .filter(LineColumn::AccountId.eq(account.id))
            .filter(LineColumn::PartyKind.eq(party_kind))
            .filter(LineColumn::AmountFc.is_not_null())
            .filter(LineColumn::Currency.is_not_null())
            .find_also_related(crate::entities::journal::journal_entries::Entity)
            .all(conn)
            .await
            .map_err(AppError::from)?;

        // Group by party in first-appearance order; entry (created_at, id) then line position
        // order comes from the underlying query — SeaORM doesn't let us `ORDER BY` two tables'
        // columns easily via `find_also_related`, so we re-sort explicitly below.
        let mut ordered: Vec<(crate::entities::journal::journal_lines::Model, crate::entities::journal::journal_entries::Model)> = lines
            .into_iter()
            .filter_map(|(l, e)| e.map(|e| (l, e)))
            .collect();
        ordered.sort_by(|(la, ea), (lb, eb)| (ea.created_at, ea.id, la.position).cmp(&(eb.created_at, eb.id, lb.position)));

        let mut by_party: Vec<(Id, Vec<crate::entities::journal::journal_lines::Model>)> = Vec::new();
        for (line, _entry) in ordered {
            let Some(party_id) = line.party_id else { continue };
            if let Some(existing) = by_party.iter_mut().find(|(id, _)| *id == party_id) {
                existing.1.push(line);
            } else {
                by_party.push((party_id, vec![line]));
            }
        }

        for (party_id, party_lines) in by_party {
            let currency = party_lines[0].currency.clone().unwrap_or_default();
            let Some(rate) = rates.get(&currency).copied() else { continue };
            if rate.is_zero() {
                continue;
            }

            let fc_balance = round2(party_lines.iter().fold(Decimal::ZERO, |acc, l| {
                let amount_fc = l.amount_fc.unwrap_or(Decimal::ZERO);
                let signed = match party_kind {
                    PartyKind::Customer => {
                        if l.debit > Decimal::ZERO {
                            amount_fc
                        } else {
                            -amount_fc
                        }
                    }
                    PartyKind::Supplier => {
                        if l.credit > Decimal::ZERO {
                            amount_fc
                        } else {
                            -amount_fc
                        }
                    }
                };
                acc + signed
            }));

            let sign = if matches!(party_kind, PartyKind::Supplier) { Decimal::NEGATIVE_ONE } else { Decimal::ONE };
            let base_balance = round2(party_lines.iter().fold(Decimal::ZERO, |acc, l| acc + (l.debit - l.credit)) * sign);

            if fc_balance.abs() < Decimal::new(5, 3) {
                continue;
            }

            let revalued_base = round2(fc_balance * rate);
            let name = PartyEntity::find_by_id(party_id).one(conn).await.map_err(AppError::from)?.map(|p| p.name).unwrap_or_else(|| party_id.to_string());

            rows.push(FcBalanceRow {
                kind: dto_kind,
                id: party_id,
                name,
                currency,
                fc_balance,
                base_balance,
                revalued_base,
                gain_loss: round2(revalued_base - base_balance),
            });
        }
    }

    // FC cash/bank/other non-group accounts with a currency, in array order (created_at, id).
    let fc_accounts = AccountEntity::find()
        .filter(AccountColumn::Currency.is_not_null())
        .filter(AccountColumn::IsGroup.eq(false))
        .filter(AccountColumn::DeletedAt.is_null())
        .order_by_asc(AccountColumn::CreatedAt)
        .order_by_asc(AccountColumn::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;

    for account in fc_accounts {
        let Some(currency) = &account.currency else { continue };
        let Some(rate) = rates.get(currency).copied() else { continue };
        if rate.is_zero() {
            continue;
        }
        let lines = LineEntity::find()
            .filter(LineColumn::AccountId.eq(account.id))
            .filter(LineColumn::AmountFc.is_not_null())
            .all(conn)
            .await
            .map_err(AppError::from)?;
        if lines.is_empty() {
            continue;
        }

        let net = lines.iter().fold(Decimal::ZERO, |acc, l| acc + (l.debit - l.credit));
        // Q-7: exact zero-compare in Rust (Decimal has no float noise, unlike the mock's JS sum).
        let fc_balance = if net.is_zero() {
            Decimal::ZERO
        } else {
            round2(lines.iter().fold(Decimal::ZERO, |acc, l| {
                let amount_fc = l.amount_fc.unwrap_or(Decimal::ZERO);
                acc + amount_fc * if l.debit > Decimal::ZERO { Decimal::ONE } else { Decimal::NEGATIVE_ONE }
            }))
        };
        let base_balance = round2(net);
        if fc_balance.abs() < Decimal::new(5, 3) {
            continue;
        }
        let revalued_base = round2(fc_balance * rate);
        rows.push(FcBalanceRow {
            kind: FcBalanceKind::Account,
            id: account.id,
            name: account.name.clone(),
            currency: currency.clone(),
            fc_balance,
            base_balance,
            revalued_base,
            gain_loss: round2(revalued_base - base_balance),
        });
    }

    Ok(rows)
}

/// `getDefaultRevaluationRates` (`revaluation.ts:72-80`): active currencies in order; latest rate
/// with no date cutoff (Q-8: matches the mock's `'9999-99-99'` — every rate on record is a
/// candidate, not just past ones), else `fixed && fixedRate`.
pub async fn default_revaluation_rates<C: ConnectionTrait>(conn: &C) -> AppResult<OrderedNumberMap> {
    let currencies = CurrencyEntity::find()
        .filter(CurrencyColumn::Active.eq(true))
        .order_by_asc(CurrencyColumn::CreatedAt)
        .order_by_asc(CurrencyColumn::Code)
        .all(conn)
        .await
        .map_err(AppError::from)?;

    let mut out = Vec::new();
    for c in currencies {
        let latest = RateEntity::find()
            .filter(RateColumn::Currency.eq(c.code.clone()))
            .order_by_desc(RateColumn::Date)
            .one(conn)
            .await
            .map_err(AppError::from)?;
        if let Some(r) = latest {
            out.push((c.code.clone(), r.rate));
        } else if c.fixed == Some(true) {
            if let Some(fixed_rate) = c.fixed_rate {
                out.push((c.code.clone(), fixed_rate));
            }
        }
    }
    Ok(OrderedNumberMap(out))
}

fn first_of_next_month(date: NaiveDate) -> NaiveDate {
    let (year, month) = if date.month() == 12 { (date.year() + 1, 1) } else { (date.year(), date.month() + 1) };
    NaiveDate::from_ymd_opt(year, month, 1).expect("valid calendar date")
}

/// `postRevaluation` (`revaluation.ts:96-139`).
pub async fn post_revaluation<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    date: NaiveDate,
    rates: &BTreeMap<String, Decimal>,
) -> TxResult<RevaluationResult> {
    let all_rows = open_fc_balances(conn, rates).await?;
    let rows: Vec<FcBalanceRow> = all_rows.into_iter().filter(|r| r.gain_loss.abs() >= Decimal::new(1, 2)).collect();
    if rows.is_empty() {
        return Err(TxError::App(AppError::validation("لا توجد أرصدة عملات أجنبية بحاجة لإعادة تقييم بهذه الأسعار")));
    }

    let mut lines: Vec<PostingLine> = Vec::new();
    let mut total_gain = Decimal::ZERO;

    for row in &rows {
        total_gain = round2(total_gain + row.gain_loss);
        let desc = format!("إعادة تقييم {} — {}", row.currency, row.name);
        match row.kind {
            FcBalanceKind::Account => {
                let mut line = if row.gain_loss > Decimal::ZERO {
                    PostingLine::debit(AccountRef::Id(row.id), row.gain_loss)
                } else {
                    PostingLine::credit(AccountRef::Id(row.id), -row.gain_loss)
                };
                line.description = Some(desc);
                lines.push(line);
            }
            FcBalanceKind::Customer => {
                let mut line = if row.gain_loss > Decimal::ZERO {
                    PostingLine::debit(AccountRef::Role(SystemRole::Receivable), row.gain_loss)
                } else {
                    PostingLine::credit(AccountRef::Role(SystemRole::Receivable), -row.gain_loss)
                };
                line.party = Some(PartyRef { kind: PartyKind::Customer, id: row.id });
                line.description = Some(desc);
                lines.push(line);
            }
            FcBalanceKind::Supplier => {
                let mut line = if row.gain_loss > Decimal::ZERO {
                    PostingLine::credit(AccountRef::Role(SystemRole::Payable), row.gain_loss)
                } else {
                    PostingLine::debit(AccountRef::Role(SystemRole::Payable), -row.gain_loss)
                };
                line.party = Some(PartyRef { kind: PartyKind::Supplier, id: row.id });
                line.description = Some(desc);
                lines.push(line);
            }
        }
    }

    if total_gain > Decimal::ZERO {
        let mut line = PostingLine::credit(AccountRef::Role(SystemRole::FxGain), total_gain);
        line.description = Some("صافي إعادة تقييم العملات".to_string());
        lines.push(line);
    } else if total_gain < Decimal::ZERO {
        let mut line = PostingLine::debit(AccountRef::Role(SystemRole::FxLoss), -total_gain);
        line.description = Some("صافي إعادة تقييم العملات".to_string());
        lines.push(line);
    }

    let date_key = date.format("%Y-%m-%d").to_string();
    let req = PostJournal {
        date: date.into(),
        description: format!("إعادة تقييم العملات بتاريخ {date_key}"),
        entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
        source: Some(crate::shared::ledger::post::SourceRef { kind: "fxReval".to_string(), id: Id::new(), number: Some(format!("FXR-{date_key}")) }),
        lines,
        allow_closed_period: false,
        attachment_ids: Vec::new(),
        template_id: None,
    };
    let entry = ledger::post(conn, cx, req).await?;

    // Mirror: a NEW post with the exact opposite debit/credit, dated the first of next month,
    // allow_closed_period: true (Q-10: not linked by reversal_of_id — a fresh post, not `reverse`).
    let reversal_date = first_of_next_month(date);
    let mirror_lines: Vec<PostingLine> = {
        let posted_lines = crate::entities::journal::journal_lines::Entity::find()
            .filter(crate::entities::journal::journal_lines::Column::JournalEntryId.eq(entry.id))
            .order_by_asc(crate::entities::journal::journal_lines::Column::Position)
            .all(conn)
            .await
            .map_err(TxError::from)?;
        posted_lines
            .into_iter()
            .map(|l| PostingLine {
                account: AccountRef::Id(l.account_id),
                debit: l.credit,
                credit: l.debit,
                description: l.description,
                party: match (l.party_kind, l.party_id) {
                    (Some(kind), Some(id)) => Some(PartyRef { kind, id }),
                    _ => None,
                },
                branch_id: None,
                cost_center_id: None,
                currency: None,
                amount_fc: None,
                rate: None,
                trace: None,
            })
            .collect()
    };
    let mirror_req = PostJournal {
        date: reversal_date.into(),
        description: format!("عكس إعادة تقييم العملات بتاريخ {date_key}"),
        entry_type: crate::entities::journal::journal_entries::JournalEntryType::System,
        source: Some(crate::shared::ledger::post::SourceRef {
            kind: "fxReval".to_string(),
            id: Id::new(),
            number: Some(format!("FXR-{date_key}")),
        }),
        lines: mirror_lines,
        allow_closed_period: true,
        attachment_ids: Vec::new(),
        template_id: None,
    };
    let reversal = ledger::post(conn, cx, mirror_req).await?;

    crate::shared::activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Journal,
        format!("إعادة تقييم العملات بتاريخ {date_key} ({} رصيد)", rows.len()),
        Some(date.into()),
        Some(RouteRef::detail("journal-entry", entry.id.to_string())),
    )
    .await?;

    Ok(RevaluationResult { entry_id: entry.id, reversal_entry_id: reversal.id, rows })
}
