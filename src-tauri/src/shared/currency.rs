//! `shared::currency` (C-4): base currency, exchange rates, FC → base conversion. Ports
//! `src/mocks/backend/currency.ts`'s `baseCurrency`/`isBaseCurrency`/`latestRate`/`requireRate`/
//! `toBase`/`convertLinesToBase` 1:1. Rate convention throughout (matching the mock): `rate` = base
//! currency per 1 unit of the foreign currency.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::entities::org::currencies::Entity as CurrencyEntity;
use crate::entities::org::exchange_rates::{Column as RateColumn, Entity as RateEntity};
use crate::entities::org::settings::Entity as SettingsEntity;
use crate::utils::money::round2;

/// `baseCurrency()` (`currency.ts:12-14`): `settings.currency`. Unlike the mock (which falls back
/// to a country profile's default when `db.settings.currency` is falsy), the settings row's
/// `currency` column is `NOT NULL` in this schema (B-2), so there is no fallback branch to port —
/// a missing settings row is the same `AppError::internal` `require`/`load` already raise
/// elsewhere, not something this function re-invents.
pub async fn base_currency<C: ConnectionTrait>(conn: &C) -> Result<String, AppError> {
    let settings = SettingsEntity::find().one(conn).await.map_err(AppError::from)?.ok_or_else(|| {
        AppError::internal("لم يتم العثور على صف الإعدادات — يجب تشغيل معالج الإعداد الأولي أولاً", None)
    })?;
    Ok(settings.currency)
}

/// `isBaseCurrency` (`currency.ts:16-18`): no code, or a code equal to the base currency.
pub async fn is_base_currency<C: ConnectionTrait>(conn: &C, code: Option<&str>) -> Result<bool, AppError> {
    match code {
        None => Ok(true),
        Some(code) => Ok(code == base_currency(conn).await?),
    }
}

/// `latestRate` (`currency.ts:77-83`): a fixed rate wins outright, else the latest `exchange_rates`
/// row with `date <= cutoff` (ties broken by the highest `date`, i.e. the most recent).
pub async fn latest_rate<C: ConnectionTrait>(conn: &C, currency: &str, as_of_day: NaiveDate) -> Result<Option<Decimal>, AppError> {
    let currency_row = CurrencyEntity::find_by_id(currency.to_string()).one(conn).await.map_err(AppError::from)?;
    if let Some(c) = &currency_row {
        if c.fixed == Some(true) {
            if let Some(rate) = c.fixed_rate {
                return Ok(Some(rate));
            }
        }
    }

    let candidate = RateEntity::find()
        .filter(RateColumn::Currency.eq(currency))
        .filter(RateColumn::Date.lte(as_of_day))
        .order_by_desc(RateColumn::Date)
        .one(conn)
        .await
        .map_err(AppError::from)?;

    Ok(candidate.map(|r| r.rate))
}

/// `requireRate` (`currency.ts:85-89`): `latest_rate`, or a `VALIDATION` error naming the currency.
pub async fn require_rate<C: ConnectionTrait>(conn: &C, currency: &str, as_of_day: NaiveDate) -> Result<Decimal, AppError> {
    latest_rate(conn, currency, as_of_day)
        .await?
        .ok_or_else(|| AppError::validation(format!("لا يوجد سعر صرف لعملة {currency}")))
}

/// `toBase` (`currency.ts:112-114`): `round2(fc * rate)`.
pub fn to_base(fc: Decimal, rate: Decimal) -> Decimal {
    round2(fc * rate)
}

/// `convertLinesToBase` (`currency.ts:97-110`): converts a set of already-rounded FC line amounts
/// to base currency, putting any cent gap between `Σ perLine` and `round2(Σfc × rate)` on the
/// line with the LARGEST `|fc|` (first index on ties — `for i in 1..` never replaces index 0 on an
/// exact tie since the comparison is strictly `>`). Returns the per-line base amounts, same
/// order/length as `fc_amounts`.
pub fn convert_lines_to_base(fc_amounts: &[Decimal], rate: Decimal) -> Vec<Decimal> {
    if fc_amounts.is_empty() {
        return Vec::new();
    }

    let mut per_line: Vec<Decimal> = fc_amounts.iter().map(|fc| round2(*fc * rate)).collect();
    let fc_total = round2(fc_amounts.iter().fold(Decimal::ZERO, |a, b| a + *b));
    let target = round2(fc_total * rate);
    let sum_per_line = round2(per_line.iter().fold(Decimal::ZERO, |a, b| a + *b));
    let diff = round2(target - sum_per_line);

    if !diff.is_zero() {
        let mut largest_idx = 0usize;
        for (i, fc) in fc_amounts.iter().enumerate().skip(1) {
            if fc.abs() > fc_amounts[largest_idx].abs() {
                largest_idx = i;
            }
        }
        per_line[largest_idx] = round2(per_line[largest_idx] + diff);
    }

    per_line
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn to_base_rounds_half_away_from_zero() {
        assert_eq!(to_base(dec!(10.005), dec!(1)), dec!(10.01));
    }

    #[test]
    fn convert_lines_to_base_puts_gap_on_largest_line() {
        // rate = 1/3: each line's own round2(fc/3) rounds independently, but round2(total/3)
        // disagrees with their sum by exactly one cent — that cent must land on the largest |fc|.
        let rate = dec!(1) / dec!(3);
        let fc = [dec!(0.01), dec!(0.04)];
        let out = convert_lines_to_base(&fc, rate);
        let sum: Decimal = out.iter().fold(Decimal::ZERO, |a, b| a + *b);
        let target = round2(round2(fc[0] + fc[1]) * rate);
        assert_eq!(sum, target, "per-line amounts must always sum to round2(fcTotal * rate)");
        // The largest-|fc| line (index 1, fc = 0.04) is the one that absorbed the gap: it differs
        // from its own naive round2(fc * rate) by the gap, while index 0 matches its naive value.
        let naive: Vec<Decimal> = fc.iter().map(|f| round2(*f * rate)).collect();
        assert_eq!(out[0], naive[0], "the smaller line must be untouched");
        assert_ne!(out[1], naive[1], "the largest line must absorb the rounding gap");
    }

    #[test]
    fn convert_lines_to_base_tie_goes_to_first_index() {
        let fc = [dec!(5), dec!(-5)];
        let out = convert_lines_to_base(&fc, dec!(1));
        assert_eq!(out, vec![dec!(5), dec!(-5)]);
    }

    #[test]
    fn convert_lines_to_base_empty() {
        let out = convert_lines_to_base(&[], dec!(1.5));
        assert!(out.is_empty());
    }
}
