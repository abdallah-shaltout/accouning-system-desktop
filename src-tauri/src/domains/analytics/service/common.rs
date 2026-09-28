//! Shared helpers for the 3 `analytics_*` reads (14-analytics.md §3.0).

use rust_decimal::Decimal;

use crate::core::error::AppError;

/// A-1: `days` defaults 30, bounded `1..=366`.
pub fn validate_days(days: Option<i32>) -> Result<i32, AppError> {
    let days = days.unwrap_or(30);
    if !(1..=366).contains(&days) {
        return Err(AppError::validation("عدد الأيام يجب أن يكون بين 1 و 366"));
    }
    Ok(days)
}

/// A-1: `limit` defaults per caller, bounded `1..=100`.
pub fn validate_limit(limit: Option<i32>, default: i32) -> Result<i32, AppError> {
    let limit = limit.unwrap_or(default);
    if !(1..=100).contains(&limit) {
        return Err(AppError::validation("الحد الأقصى للنتائج يجب أن يكون بين 1 و 100"));
    }
    Ok(limit)
}

/// `js_num` — plain `${}` interpolation of a number the mock never runs through `formatNumber`
/// (a raw JS `Number` template literal): shortest round-trip text, matching `utils::money::
/// js_number_string` (already the crate-wide "plain JS number to string" helper).
pub fn js_num(d: Decimal) -> String {
    crate::utils::money::js_number_string(d)
}
