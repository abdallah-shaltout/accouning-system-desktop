//! `reports::service::ledgers` (13 §3.7, 3.8, 3.12, 3.15, 3.16): account ledger, party ledger, day
//! book, ledger targets, dimension options.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::journal::{journal_entries, journal_lines};
use crate::entities::org::{branches, cost_centers, currencies};
use crate::entities::parties::parties;
use crate::entities::soft_delete::SoftDelete;
use crate::shared::balances;
use crate::utils::dates::BusinessClock;
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::route::RouteRef;

use super::super::dto::{AccountLedger, CurrencyOption, DateRangeInput, DayBookEntry, DayBookLine, DimensionOption, DimensionOptions, LedgerRow, LedgerTarget, LedgerTargets, NormalSide, PartyKindArg};
use super::common::{self, AccountsIndex, DateRange};

/// **`reports_get_account_ledger`** (13 §3.7).
pub async fn account_ledger<C: ConnectionTrait>(conn: &C, account_id: Id, range: &DateRangeInput) -> TxResult<AccountLedger> {
    let idx = AccountsIndex::load(conn).await?;
    let account = idx.get(account_id).cloned().ok_or_else(|| AppError::not_found("الحساب غير موجود"))?;
    let sign = if account.normal_side == "DEBIT" { Decimal::ONE } else { -Decimal::ONE };

    let date_range = DateRange::parse(range)?;

    let mut all = journal_lines::Entity::find()
        .filter(journal_lines::Column::AccountId.eq(account_id))
        .find_also_related(journal_entries::Entity)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    all.sort_by(|(la, ea), (lb, eb)| match (ea, eb) {
        (Some(ea), Some(eb)) => (ea.date().key(), ea.number.clone(), la.position).cmp(&(eb.date().key(), eb.number.clone(), lb.position)),
        _ => std::cmp::Ordering::Equal,
    });

    let mut opening = Decimal::ZERO;
    let mut rows: Vec<LedgerRow> = Vec::new();
    let mut row_links: BTreeMap<String, RouteRef> = BTreeMap::new();

    for (line, entry) in all {
        let Some(entry) = entry else { continue };
        let day = entry.date_day;
        if let Some(from) = date_range.from {
            if day < from {
                opening += sign * (line.debit - line.credit);
                continue;
            }
        }
        if let Some(to) = date_range.to {
            if day > to {
                continue;
            }
        }
        let description = line.description.clone().unwrap_or_else(|| entry.description.clone());
        row_links.insert(line.id.to_string(), RouteRef::detail("journal-entry", entry.id.to_string()));
        rows.push(LedgerRow { id: line.id, date: entry.date().key(), entry_id: entry.id, entry_number: entry.number.clone(), description, debit: line.debit, credit: line.credit, balance: Decimal::ZERO });
    }

    let opening_balance = round2(opening);
    let mut running = opening_balance;
    for r in rows.iter_mut() {
        running = round2(running + sign * (r.debit - r.credit));
        r.balance = running;
    }

    let total_debit = common::sum2(rows.iter().map(|r| r.debit));
    let total_credit = common::sum2(rows.iter().map(|r| r.credit));

    Ok(AccountLedger {
        title: format!("{} — {}", account.code, account.name),
        subtitle: Some(if account.normal_side == "DEBIT" { "حساب مدين الطبيعة".to_string() } else { "حساب دائن الطبيعة".to_string() }),
        normal_side: if account.normal_side == "DEBIT" { NormalSide::Debit } else { NormalSide::Credit },
        opening_balance,
        rows,
        total_debit,
        total_credit,
        closing_balance: running,
        row_links,
    })
}

/// **`reports_get_party_ledger`** (13 §3.8): delegates to `shared::balances::customer_statement`/
/// `supplier_statement` — never re-derived here.
pub async fn party_ledger<C: ConnectionTrait>(conn: &C, kind: PartyKindArg, party_id: Id, range: &DateRangeInput, clock: &BusinessClock) -> TxResult<AccountLedger> {
    let kind_str = match kind {
        PartyKindArg::Customer => "customer",
        PartyKindArg::Supplier => "supplier",
    };
    let party = parties::Entity::find_by_id(party_id)
        .filter(parties::Column::DeletedAt.is_null())
        .one(conn)
        .await
        .map_err(AppError::from)?
        .filter(|p| p.kind == kind_str)
        .ok_or_else(|| AppError::not_found(if matches!(kind, PartyKindArg::Customer) { "العميل غير موجود" } else { "المورد غير موجود" }))?;

    let all = match kind {
        PartyKindArg::Customer => balances::customer_statement(conn, party_id).await.map_err(TxErrFromApp)?,
        PartyKindArg::Supplier => balances::supplier_statement(conn, party_id).await.map_err(TxErrFromApp)?,
    };

    let date_range = DateRange::parse(range)?;
    // The mock compares each row's **local business day** (`localDateKey(r.date)` /
    // `inDateRange(r.date, …)`), not its raw key: an instant key (`2026-06-30T09:00:01.000Z`) is
    // byte-greater than a `to` of `2026-06-30` and would wrongly fall out of the range.
    let before: Vec<&balances::PartyStatementRow> = all.iter().filter(|r| date_range.from.is_some_and(|f| common::day_of_key(&r.date_key, clock) < f)).collect();
    let in_range: Vec<&balances::PartyStatementRow> = all.iter().filter(|r| common::in_range(common::day_of_key(&r.date_key, clock), &date_range)).collect();

    let opening = before.last().map(|r| r.balance).unwrap_or(Decimal::ZERO);

    let mut row_links: BTreeMap<String, RouteRef> = BTreeMap::new();
    for r in &in_range {
        let route = match r.kind {
            "invoice" | "refund" => RouteRef::detail("invoice", r.ref_id.to_string()),
            "payment" => RouteRef::list("payments").with_query("highlight", r.ref_id.to_string()),
            _ => RouteRef::detail("purchase", r.ref_id.to_string()),
        };
        row_links.insert(r.id.to_string(), route);
    }

    let rows: Vec<LedgerRow> = in_range
        .iter()
        .map(|r| LedgerRow { id: r.id, date: r.date_key.clone(), entry_id: r.ref_id, entry_number: r.number.clone(), description: r.description.clone(), debit: r.debit, credit: r.credit, balance: r.balance })
        .collect();

    let total_debit = common::sum2(rows.iter().map(|r| r.debit));
    let total_credit = common::sum2(rows.iter().map(|r| r.credit));
    let closing_balance = in_range.last().map(|r| r.balance).unwrap_or(opening);

    Ok(AccountLedger {
        title: party.name,
        subtitle: Some(match kind {
            PartyKindArg::Customer => "كشف حساب عميل — الرصيد الموجب مستحق لنا".to_string(),
            PartyKindArg::Supplier => "كشف حساب مورد — الرصيد الموجب مستحق للمورد".to_string(),
        }),
        normal_side: match kind {
            PartyKindArg::Customer => NormalSide::Debit,
            PartyKindArg::Supplier => NormalSide::Credit,
        },
        opening_balance: opening,
        rows,
        total_debit,
        total_credit,
        closing_balance,
        row_links,
    })
}

/// Bridges `AppError` (from `shared::balances`, a plain `Result<_, AppError>`) into `TxResult`'s
/// `TxError` at the one call site above.
struct TxErrFromApp(AppError);
impl From<AppError> for TxErrFromApp {
    fn from(e: AppError) -> Self {
        TxErrFromApp(e)
    }
}
impl From<TxErrFromApp> for crate::core::tx::TxError {
    fn from(e: TxErrFromApp) -> Self {
        crate::core::tx::TxError::App(e.0)
    }
}

/// **`reports_get_day_book`** (13 §3.12).
pub async fn day_book<C: ConnectionTrait>(conn: &C, range: &DateRangeInput) -> TxResult<Vec<DayBookEntry>> {
    let date_range = DateRange::parse(range)?;
    let idx = AccountsIndex::load(conn).await?;

    let mut rows = journal_lines::Entity::find().find_also_related(journal_entries::Entity).all(conn).await.map_err(AppError::from)?;
    // Group lines by entry, keeping only entries whose day is in range.
    let mut by_entry: std::collections::HashMap<Id, (journal_entries::Model, Vec<journal_lines::Model>)> = std::collections::HashMap::new();
    for (line, entry) in rows.drain(..) {
        let Some(entry) = entry else { continue };
        if !common::in_range(entry.date_day, &date_range) {
            continue;
        }
        by_entry.entry(entry.id).or_insert_with(|| (entry.clone(), Vec::new())).1.push(line);
    }

    let mut entries: Vec<(journal_entries::Model, Vec<journal_lines::Model>)> = by_entry.into_values().collect();
    entries.sort_by(|(a, _), (b, _)| (a.date().key(), a.number.clone()).cmp(&(b.date().key(), b.number.clone())));

    Ok(entries
        .into_iter()
        .map(|(entry, mut lines)| {
            lines.sort_by_key(|l| l.position);
            let day_lines = lines
                .into_iter()
                .map(|l| {
                    let acc = idx.get(l.account_id);
                    DayBookLine { account_code: acc.map(|a| a.code.clone()).unwrap_or_default(), account_name: acc.map(|a| a.name.clone()).unwrap_or_default(), debit: l.debit, credit: l.credit }
                })
                .collect();
            DayBookEntry { id: entry.id, number: entry.number.clone(), date: entry.date().key(), description: entry.description.clone(), lines: day_lines }
        })
        .collect())
}

/// **`reports_get_ledger_targets`** (13 §3.15).
pub async fn ledger_targets<C: ConnectionTrait>(conn: &C) -> TxResult<LedgerTargets> {
    let idx = AccountsIndex::load(conn).await?;
    let mut accounts: Vec<&common::AccountRow> = idx.iter_ordered().collect();
    accounts.sort_by(|a, b| a.code.cmp(&b.code));
    let accounts = accounts.into_iter().map(|a| LedgerTarget { id: a.id.to_string(), label: format!("{} — {}", a.code, a.name) }).collect();

    let customers = parties::Entity::find_live()
        .filter(parties::Column::Kind.eq("customer"))
        .order_by_asc(parties::Column::CreatedAt)
        .order_by_asc(parties::Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .map(|p| LedgerTarget { id: p.id.to_string(), label: p.name })
        .collect();

    let suppliers = parties::Entity::find_live()
        .filter(parties::Column::Kind.eq("supplier"))
        .order_by_asc(parties::Column::CreatedAt)
        .order_by_asc(parties::Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .map(|p| LedgerTarget { id: p.id.to_string(), label: p.name })
        .collect();

    Ok(LedgerTargets { accounts, customers, suppliers })
}

/// **`reports_get_dimension_options`** (13 §3.16).
pub async fn dimension_options<C: ConnectionTrait>(conn: &C) -> TxResult<DimensionOptions> {
    let branches = branches::Entity::find()
        .filter(branches::Column::DeletedAt.is_null())
        .filter(branches::Column::Active.eq(true))
        .order_by_asc(branches::Column::CreatedAt)
        .order_by_asc(branches::Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .map(|b| DimensionOption { id: b.id.to_string(), label: b.name })
        .collect();

    let cost_centers = cost_centers::Entity::find_live()
        .filter(cost_centers::Column::Active.eq(true))
        .order_by_asc(cost_centers::Column::CreatedAt)
        .order_by_asc(cost_centers::Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .map(|c| DimensionOption { id: c.id.to_string(), label: c.name })
        .collect();

    let currencies = currencies::Entity::find()
        .filter(currencies::Column::Active.eq(true))
        .order_by_asc(currencies::Column::CreatedAt)
        .order_by_asc(currencies::Column::Code)
        .all(conn)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .map(|c| CurrencyOption { code: c.code.clone(), label: format!("{} ({})", c.name_ar, c.code) })
        .collect();

    Ok(DimensionOptions { branches, cost_centers, currencies })
}
