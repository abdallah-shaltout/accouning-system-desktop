//! `shared::ledger::period` (C-2, P2-13): the fiscal-period guard ported from
//! `assertOpenPeriod` (`src/mocks/backend/core.ts:103-116`). Always takes shared locks on the
//! settings row and the covering fiscal-year row (even when `allow_closed_period` is set) so a
//! poster never races `close_year`'s exclusive lock; only refuses when `!allow_closed_period`.

use chrono::NaiveDate;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::TxResult;
use crate::entities::org::fiscal_years::{Column as FiscalYearColumn, Entity as FiscalYearEntity};
use crate::entities::org::settings::Entity as SettingsEntity;
use crate::utils::id::Id;

/// Ports `assertOpenPeriod` (`core.ts:103-116`). **Always** takes a shared lock on the settings
/// row, then on the first fiscal year covering `date` (`ORDER BY created_at, id`, matching the
/// mock's `db.fiscalYears.find(...)` array-order semantics) — even when `allow_closed_period` is
/// set, so posting and `close_year`'s exclusive lock (`lock_fiscal_year_exclusive`) can never
/// interleave unsafely (P2-13). Only refuses (when `!allow_closed_period`): a set lock date with
/// `date <= lock_date`, or a covering fiscal year that is closed. No covering year at all is
/// allowed — same as the mock (`core.ts:112`'s `fy?.isClosed` short-circuits on `undefined`).
pub async fn assert_open_period<C: ConnectionTrait>(
    conn: &C,
    date: &NaiveDate,
    allow_closed_period: bool,
) -> TxResult<()> {
    // Settings is a `ck_settings_singleton` table — lock its one row by id via a plain SELECT ...
    // LOCK IN SHARE MODE keyed on whatever id that row has.
    // The row's values are re-read *by* the locking read (`find_share_locked`): after waiting on a
    // concurrent writer's X lock (a lock-date change, `close_year`), a plain REPEATABLE READ select
    // would still return the pre-commit snapshot and let the poster through.
    let settings = match SettingsEntity::find().one(conn).await.map_err(AppError::from)? {
        Some(s) => lock::find_share_locked::<SettingsEntity>(conn, "settings", &s.id.to_string()).await?,
        None => None,
    };

    let fiscal_year = FiscalYearEntity::find()
        .filter(FiscalYearColumn::StartDate.lte(*date))
        .filter(FiscalYearColumn::EndDate.gte(*date))
        .order_by_asc(FiscalYearColumn::CreatedAt)
        .order_by_asc(FiscalYearColumn::Id)
        .one(conn)
        .await
        .map_err(AppError::from)?;
    let fiscal_year = match fiscal_year {
        Some(fy) => lock::find_share_locked::<FiscalYearEntity>(conn, "fiscal_years", &fy.id.to_string()).await?,
        None => None,
    };

    if allow_closed_period {
        return Ok(());
    }

    let key = date.format("%Y-%m-%d").to_string();

    if let Some(lock_date) = settings.as_ref().and_then(|s| s.accounting.as_ref()).and_then(|a| a.lock_date) {
        if *date <= lock_date {
            return Err(AppError::period_locked_by_date(&key, &lock_date.format("%Y-%m-%d").to_string()).into());
        }
    }

    if let Some(fy) = &fiscal_year {
        if fy.is_closed {
            return Err(AppError::period_locked_by_year(&key, &fy.name).into());
        }
    }

    Ok(())
}

/// `lock_fiscal_year_exclusive` — for Part 03's `close_year`/`reopen_year`: an exclusive
/// (`FOR UPDATE`) lock on the year row, taken **after** the settings shared lock in the same
/// global order as `assert_open_period` (settings (S) first, then the year (X) — P2-13's doc
/// comment) so a poster's shared lock and a closer's exclusive lock always contend on the same
/// row rather than deadlocking on lock-order mismatches.
pub async fn lock_fiscal_year_exclusive<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<()> {
    lock::for_update_by_id(conn, "fiscal_years", &id.to_string()).await?;
    Ok(())
}
