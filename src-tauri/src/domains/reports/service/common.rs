//! `reports::service::common` (13 §3.0) — shared helpers used by both 13 and 13b's service files:
//! date-range parsing, `movements`/`movements_by_cost_center` (the ledger scan every statement
//! report is built on), `AccountsIndex`, `lines`/`sum2`/`compute_pnl`.

use std::collections::HashMap;

use chrono::NaiveDate;
use indexmap::IndexMap;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::journal::{journal_entries, journal_lines};
use crate::entities::org::accounts;
use crate::utils::dates::{BusinessClock, RawDocDate};
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::super::dto::{DateRangeInput, DimensionFilter, ProfitAndLoss, StatementLine};

// --- Date range --------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default)]
pub struct DateRange {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

impl DateRange {
    /// `None`/`""` -> no bound (`utils.ts:111-112`); otherwise must parse as `YYYY-MM-DD`, else
    /// `VALIDATION "تاريخ غير صالح"` (R-2 — the mock never validates; an IPC boundary must).
    pub fn parse(input: &DateRangeInput) -> Result<Self, AppError> {
        let from = parse_bound(input.from.as_deref())?;
        let to = parse_bound(input.to.as_deref())?;
        Ok(DateRange { from, to })
    }

    /// A range with only an upper bound (`{ to: asOf }`) — cumulative-since-inception scans
    /// (balance sheet, cash flow opening/closing).
    pub fn to_only(to: NaiveDate) -> Self {
        DateRange { from: None, to: Some(to) }
    }
}

fn parse_bound(s: Option<&str>) -> Result<Option<NaiveDate>, AppError> {
    match s {
        None => Ok(None),
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d").map(Some).map_err(|_| AppError::validation("تاريخ غير صالح")),
    }
}

/// `dayBefore` (`rs:81-84`): a calendar step, no timezone.
pub fn day_before(d: NaiveDate) -> NaiveDate {
    d.pred_opt().expect("NaiveDate::pred_opt only fails at the proleptic Gregorian calendar's minimum date")
}

/// `inDateRange` (`utils.ts:104-109`) on a resolved business day.
pub fn in_range(day: NaiveDate, range: &DateRange) -> bool {
    if let Some(from) = range.from {
        if day < from {
            return false;
        }
    }
    if let Some(to) = range.to {
        if day > to {
            return false;
        }
    }
    true
}

/// The business day of a key string that came back from `shared::balances` (`RawDocDate::parse` +
/// `.resolve(clock).day`).
pub fn day_of_key(key: &str, clock: &BusinessClock) -> NaiveDate {
    match RawDocDate::parse(key) {
        Ok(raw) => raw.resolve(clock).day,
        // A malformed key should never happen (every key comes from `DocDate::key()`); fall back to
        // the clock's "today" rather than panicking on a read-only report.
        Err(_) => clock.today(),
    }
}

/// `daysBetween` (`Math.floor((a - b) / 86_400_000)` on two `YYYY-MM-DD` strings, both UTC
/// midnight) = a whole-day `NaiveDate` difference.
pub fn days_between(a: NaiveDate, b: NaiveDate) -> i64 {
    (a - b).num_days()
}

// --- Movements (the ledger scan) ---------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default)]
pub struct DcTotal {
    pub d: Decimal,
    pub c: Decimal,
}

pub type Movements = HashMap<Id, DcTotal>;

/// `movements` (`rs:64-79`): `Σ(debit, credit)` per account for entries within `range`, optionally
/// narrowed by `dim` (branch/cost-center/currency, additive — a `None`/empty value adds no
/// condition). One SQL statement; `SUM` of `DECIMAL(19,2)` columns is exact.
pub async fn movements<C: ConnectionTrait>(conn: &C, range: &DateRange, dim: Option<&DimensionFilter>) -> TxResult<Movements> {
    let mut query = journal_lines::Entity::find().find_also_related(journal_entries::Entity);
    // Date bounds apply to the entry's business day — filtered in Rust below (the entry set is
    // read fully first since we still need each line's account/branch/cost-center/currency).
    if let Some(dim) = dim {
        if let Some(branch_id) = dim.branch_id.as_deref().filter(|s| !s.is_empty()) {
            // The mock's `line.branchId === dim.branchId` matches nothing for an id that is not a
            // branch (a stale filter), so an unparseable id yields no rows — never "no filter".
            match branch_id.parse::<Id>() {
                Ok(id) => query = query.filter(journal_lines::Column::BranchId.eq(id)),
                Err(_) => return Ok(HashMap::new()),
            }
        }
        if let Some(cc_id) = dim.cost_center_id.as_deref().filter(|s| !s.is_empty()) {
            match cc_id.parse::<Id>() {
                Ok(id) => query = query.filter(journal_lines::Column::CostCenterId.eq(id)),
                Err(_) => return Ok(HashMap::new()),
            }
        }
        if let Some(currency) = dim.currency.as_deref().filter(|s| !s.is_empty()) {
            query = query.filter(journal_lines::Column::Currency.eq(currency));
        }
    }

    let rows = query.all(conn).await.map_err(AppError::from)?;
    let mut map: Movements = HashMap::new();
    for (line, entry) in rows {
        let Some(entry) = entry else { continue };
        if !in_range(entry.date_day, range) {
            continue;
        }
        let t = map.entry(line.account_id).or_default();
        t.d += line.debit;
        t.c += line.credit;
    }
    Ok(map)
}

const UNASSIGNED_CC: &str = "__unassigned__";

/// `movementsByCostCenter` (`rs:165-180`): same scan grouped by
/// `(COALESCE(cost_center_id, "__unassigned__"), account_id)`, insertion-ordered (G-33).
pub async fn movements_by_cost_center<C: ConnectionTrait>(conn: &C, range: &DateRange) -> TxResult<IndexMap<String, Movements>> {
    let rows = journal_lines::Entity::find().find_also_related(journal_entries::Entity).all(conn).await.map_err(AppError::from)?;
    let mut by_cc: IndexMap<String, Movements> = IndexMap::new();
    for (line, entry) in rows {
        let Some(entry) = entry else { continue };
        if !in_range(entry.date_day, range) {
            continue;
        }
        let cc_key = line.cost_center_id.map(|id| id.to_string()).unwrap_or_else(|| UNASSIGNED_CC.to_string());
        let acc_map = by_cc.entry(cc_key).or_default();
        let t = acc_map.entry(line.account_id).or_default();
        t.d += line.debit;
        t.c += line.credit;
    }
    Ok(by_cc)
}

pub fn unassigned_cc_key() -> &'static str {
    UNASSIGNED_CC
}

// --- Accounts index ------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRow {
    pub id: Id,
    pub code: String,
    pub name: String,
    pub kind: String,
    pub subtype: String,
    pub normal_side: String,
    pub is_group: bool,
    pub parent_id: Option<Id>,
    pub system_role: Option<String>,
}

impl From<accounts::Model> for AccountRow {
    fn from(m: accounts::Model) -> Self {
        AccountRow { id: m.id, code: m.code, name: m.name, kind: m.kind, subtype: m.subtype, normal_side: m.normal_side, is_group: m.is_group, parent_id: m.parent_id, system_role: m.system_role }
    }
}

/// Every **live** account (`find_live`), `(created_at, id)` order — the mock's `db.accounts` array
/// order.
pub struct AccountsIndex {
    pub by_id: HashMap<Id, AccountRow>,
    /// `(created_at, id)` order, mirrored as a plain `Vec` for callers that iterate in array order.
    pub ordered: Vec<Id>,
}

impl AccountsIndex {
    pub async fn load<C: ConnectionTrait>(conn: &C) -> TxResult<Self> {
        use crate::entities::soft_delete::SoftDelete;
        let rows = accounts::Entity::find_live()
            .order_by_asc(accounts::Column::CreatedAt)
            .order_by_asc(accounts::Column::Id)
            .all(conn)
            .await
            .map_err(AppError::from)?;
        let mut by_id = HashMap::with_capacity(rows.len());
        let mut ordered = Vec::with_capacity(rows.len());
        for r in rows {
            let row = AccountRow::from(r);
            ordered.push(row.id);
            by_id.insert(row.id, row);
        }
        Ok(AccountsIndex { by_id, ordered })
    }

    pub fn get(&self, id: Id) -> Option<&AccountRow> {
        self.by_id.get(&id)
    }

    /// Ordered iterator over the live accounts (array order).
    pub fn iter_ordered(&self) -> impl Iterator<Item = &AccountRow> {
        self.ordered.iter().filter_map(move |id| self.by_id.get(id))
    }

    /// `groupNameOf` (`rs:52-56`): the nearest `is_group` ancestor's name, walking `parent_id`
    /// through the **live** index only (Q-4: a soft-deleted parent ends the walk, same as the
    /// mock's hard delete) — `""` when none.
    pub fn group_name_of(&self, account: &AccountRow) -> String {
        let mut parent = account.parent_id.and_then(|id| self.by_id.get(&id));
        while let Some(p) = parent {
            if p.is_group {
                return p.name.clone();
            }
            parent = p.parent_id.and_then(|id| self.by_id.get(&id));
        }
        String::new()
    }
}

/// `sum2` (`utils.ts:70-72`): `round2(Σ)` — the mock's `sum` always rounds once at the end.
pub fn sum2(values: impl IntoIterator<Item = Decimal>) -> Decimal {
    round2(values.into_iter().fold(Decimal::ZERO, |a, x| a + x))
}

/// `lines` (`rs:116-125`): live, non-group accounts of `kind` with any movement, matching an
/// optional extra filter, sorted by code; `amount = round2(sign × (d − c))` **per account**; drop
/// `|amount| <= 0.001`.
pub fn lines(idx: &AccountsIndex, kind: &str, mv: &Movements, sign: i64, filter: impl Fn(&AccountRow) -> bool) -> Vec<StatementLine> {
    let sign_d = Decimal::from(sign);
    let mut candidates: Vec<&AccountRow> = idx.iter_ordered().filter(|a| !a.is_group && a.kind == kind && mv.contains_key(&a.id) && filter(a)).collect();
    candidates.sort_by(|a, b| a.code.cmp(&b.code));
    candidates
        .into_iter()
        .map(|a| {
            let t = mv.get(&a.id).copied().unwrap_or_default();
            let amount = round2(sign_d * (t.d - t.c));
            StatementLine { account_id: a.id, code: a.code.clone(), name: a.name.clone(), amount }
        })
        .filter(|l| l.amount.abs() > Decimal::new(1, 3)) // 0.001
        .collect()
}

fn abs_gt_0_001(d: Decimal) -> bool {
    d.abs() > Decimal::new(1, 3)
}

/// `computePnl` (`rs:127-138`): revenue/cogs/expenses lines, each rounded per account then summed
/// (`sum2` on already-rounded values, A§7's "round per row, then sum" chain), differences rounded
/// again.
pub fn compute_pnl(idx: &AccountsIndex, mv: &Movements) -> ProfitAndLoss {
    let revenue = lines(idx, "REVENUE", mv, -1, |_| true);
    let cogs = lines(idx, "EXPENSE", mv, 1, |a| a.subtype == "costOfSales");
    let expenses = lines(idx, "EXPENSE", mv, 1, |a| a.subtype != "costOfSales");
    let net_revenue = sum2_from_lines(&revenue);
    let total_cogs = sum2_from_lines(&cogs);
    let total_expenses = sum2_from_lines(&expenses);
    let gross_profit = round2(net_revenue - total_cogs);
    let net_income = round2(gross_profit - total_expenses);
    ProfitAndLoss { revenue, net_revenue, cogs, total_cogs, gross_profit, expenses, total_expenses, net_income }
}

fn sum2_from_lines(lines: &[StatementLine]) -> Decimal {
    // Per §7: `sum()` on already-`round2`'d per-line amounts, summed raw — a no-op re-round since
    // every summand is already 2dp, but ported literally (no extra `round2` here matches the mock's
    // own `sum` which is itself `round2(Σ)` — applying it to already-rounded values is idempotent).
    sum2(lines.iter().map(|l| l.amount))
}

/// `jsNum` = `js_number_string` for numbers interpolated into Arabic explanation text.
pub fn js_num(d: Decimal) -> String {
    crate::utils::money::js_number_string(d)
}

/// Small helper kept alongside `lines`'s own inline filter — exposed for callers (`cash_flow`) that
/// need the same `|x| > 0.001` cut on a plain `Decimal` outside a `StatementLine`.
pub fn keep_significant(d: Decimal) -> bool {
    abs_gt_0_001(d)
}
