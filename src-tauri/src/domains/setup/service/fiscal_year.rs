//! `fiscal_year.rs` (02-setup.md §3.5, `setup.ts:67-93`, `setupService.ts:140-145`): sets the
//! fiscal year to start on (startMonth, startDay) of the go-live year and run 12 months —
//! wholesale-replaces `fiscal_years` (always exactly one row while onboarding).

use chrono::Datelike;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryOrder, Set};

use crate::core::error::AppError;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::domains::accounting::dto::FiscalYear as FiscalYearDto;
use crate::entities::journal::journal_entries::Entity as JournalEntryEntity;
use crate::entities::org::fiscal_years::{ActiveModel as FiscalYearActiveModel, Column as FyColumn, Entity as FiscalYearEntity};
use crate::utils::id::Id;

/// `js_date(y, m, d)` — mirrors JS `new Date(year, month0, day)`'s overflow normalization exactly:
/// month/day out of range roll forward/back through the calendar (Feb 30 -> Mar 2). `month` is
/// 1-based here (the caller passes `startMonth` directly); internally converted to 0-based before
/// the JS-style day-count arithmetic.
fn js_date(year: i32, month_1based: i32, day: i32) -> chrono::NaiveDate {
    // JS `new Date(y, m0, d)`: first resolve (y, m0) by carrying `m0` overflow into the year, then
    // add `(d - 1)` days to the first of that resolved month — this reproduces month-end/day
    // overflow (`new Date(2026, 1, 30)` -> March 2, 2026) exactly the way V8 does.
    let m0 = month_1based - 1; // 0-based, may be negative or >= 12 in principle (never here).
    let extra_years = m0.div_euclid(12);
    let month0 = m0.rem_euclid(12);
    let resolved_year = year + extra_years;
    let first_of_month = chrono::NaiveDate::from_ymd_opt(resolved_year, (month0 + 1) as u32, 1)
        .expect("month0 is always 0..=11 after rem_euclid");
    first_of_month + chrono::Duration::days((day - 1) as i64)
}

/// `applyFiscalYear` (`setup.ts:67-93`, `setupService.ts:140-145`).
pub async fn apply<C: ConnectionTrait>(conn: &C, cx: &TxCtx, start_month: i32, start_day: i32, go_live: chrono::NaiveDate) -> TxResult<FiscalYearDto> {
    let journal_count = JournalEntryEntity::find().count(conn).await.map_err(TxError::from)?;
    if journal_count > 0 {
        return Err(TxError::App(AppError::forbidden("لا يمكن تغيير السنة المالية بعد بدء الترحيل")));
    }

    let mut start = js_date(go_live.year(), start_month, start_day);
    if start > go_live {
        start = js_date(go_live.year() - 1, start_month, start_day);
    }
    let end = js_date(start.year() + 1, start.month() as i32, start.day() as i32) - chrono::Duration::days(1);
    let name = start.year().to_string();

    let existing = FiscalYearEntity::find().order_by_asc(FyColumn::CreatedAt).order_by_asc(FyColumn::Id).all(conn).await.map_err(TxError::from)?;

    let (id, created_at) = match existing.first() {
        Some(first) => (first.id, first.created_at),
        None => (Id::new(), cx.clock.now),
    };

    // Delete every row except the kept id (id kept per the mock's `db.fiscalYears[0]?.id`).
    for row in existing.iter().skip(1) {
        FiscalYearEntity::delete_by_id(row.id).exec(conn).await.map_err(TxError::from)?;
    }

    let model = FiscalYearActiveModel {
        id: Set(id),
        name: Set(name),
        start_date: Set(start),
        end_date: Set(end),
        is_closed: Set(false),
        closing_entry_id: Set(None),
        closed_at: Set(None),
        closed_by: Set(None),
        created_at: Set(created_at),
        updated_at: Set(cx.clock.now),
    };

    let saved = if existing.is_empty() {
        model.insert(conn).await.map_err(TxError::from)?
    } else {
        // Upsert-by-id: an existing row with this id is updated in place.
        model.update(conn).await.map_err(TxError::from)?
    };

    // `onboarding.goLiveDate = go_live`.
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
    onboarding.go_live_date = Some(go_live);
    let mut settings_model: crate::entities::org::settings::ActiveModel = locked.into();
    settings_model.onboarding = Set(Some(onboarding));
    settings_model.updated_at = Set(cx.clock.now);
    settings_model.update(conn).await.map_err(TxError::from)?;

    Ok(FiscalYearDto::from_model(saved))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_date_normalizes_feb_30_to_mar_2() {
        let d = js_date(2026, 2, 30);
        assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2026, 3, 2).unwrap());
    }

    #[test]
    fn js_date_plain_date_passthrough() {
        let d = js_date(2026, 1, 1);
        assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
    }

    #[test]
    fn js_date_july_first() {
        let d = js_date(2025, 7, 1);
        assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2025, 7, 1).unwrap());
    }
}
