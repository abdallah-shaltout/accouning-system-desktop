//! Shared helpers for the `dashboard` domain (14-analytics.md §3.0, §3.5; 14b-insights.md §3.0).

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::error::AppError;
use crate::entities::journal::journal_lines::{Column as LineColumn, Entity as LineEntity};
use crate::shared::ledger::accounts::{resolve_account, AccountCtx, SystemRole};
use crate::utils::money::round2;

/// `accountBalance` (`dashboardService.ts:14-19` / `insightRules.ts:44-49`): resolves `role` to its
/// account (default ctx — branch/currency-less, same as the mock's single-branch/base-currency
/// world) then sums `debit − credit` over every journal line posted to it.
pub async fn account_balance<C: ConnectionTrait>(conn: &C, role: SystemRole) -> Result<Decimal, AppError> {
    let account = resolve_account(conn, role, &AccountCtx::default()).await?;
    let lines = LineEntity::find().filter(LineColumn::AccountId.eq(account.id)).all(conn).await.map_err(AppError::from)?;
    Ok(round2(lines.iter().fold(Decimal::ZERO, |a, l| a + (l.debit - l.credit))))
}

/// `accountBalanceAsOf` (`dashboardService.ts:236-244`): like `account_balance`, but only counts
/// lines whose entry's business day is `<= as_of` (inclusive).
pub async fn account_balance_as_of<C: ConnectionTrait>(conn: &C, role: SystemRole, as_of: chrono::NaiveDate) -> Result<Decimal, AppError> {
    use crate::entities::journal::journal_entries::{Column as EntryColumn, Entity as EntryEntity};

    let account = resolve_account(conn, role, &AccountCtx::default()).await?;
    let entries = EntryEntity::find().filter(EntryColumn::DateDay.lte(as_of)).all(conn).await.map_err(AppError::from)?;
    let entry_ids: Vec<_> = entries.iter().map(|e| e.id).collect();
    if entry_ids.is_empty() {
        return Ok(Decimal::ZERO);
    }
    let lines = LineEntity::find()
        .filter(LineColumn::AccountId.eq(account.id))
        .filter(LineColumn::JournalEntryId.is_in(entry_ids))
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(round2(lines.iter().fold(Decimal::ZERO, |a, l| a + (l.debit - l.credit))))
}

/// `daysBetween` (`insightRules.ts:40-42`): whole days between two `YYYY-MM-DD` keys, `a − b`
/// (floor division of the millisecond difference, matching the mock's `Math.floor`).
pub fn days_between(a: chrono::NaiveDate, b: chrono::NaiveDate) -> i64 {
    (a - b).num_days()
}

/// `dayOfKey` — the `YYYY-MM-DD` prefix of a raw mock date-key string. A day key already has this
/// shape; an ISO instant's first 10 characters are its calendar day in UTC (the timezone the mock's
/// `Date` constructor from a `Z`-suffixed string always ends up in for a plain `.slice(0, 10)`
/// read). Only used to compare a raw stored key (from `DocDate::key()`) against a `NaiveDate` cutoff
/// as `YYYY-MM-DD` byte strings, exactly like the mock's own string comparisons (13 §3.0 convention).
pub fn day_of_key(key: &str) -> &str {
    if key.len() >= 10 {
        &key[..10]
    } else {
        key
    }
}

/// `truncDays` (14b §3.0): `t.trunc() as i64` — a fractional-day threshold (e.g. `deadStockDays`
/// read from `InsightThresholds` as a `Decimal`) truncates toward zero, matching JS `setDate(getDate()
/// - t)` on a non-integer `t` (V8 truncates the argument to `Date.prototype.setDate`).
pub fn trunc_days(t: Decimal) -> i64 {
    t.trunc().to_i64().unwrap_or(0)
}
