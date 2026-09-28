//! `domains::invoices::service::credit` — credit-limit check (08 §3.1, sole Rust caller), a
//! behaviour-exact port of `parties/helpers/creditLimit.ts:23-43`.

use rust_decimal::Decimal;
use sea_orm::ConnectionTrait;

use crate::core::auth::{Access, Area, Role};
use crate::core::error::AppError;
use crate::core::settings::parse_role_access_overrides;
use crate::core::tx::TxResult;
use crate::utils::dates::DocDate;

/// `assertWithinCreditLimit` (`creditLimit.ts:23-32`): returns (does not error) if
/// `new_receivable <= 0`, if `limit <= 0`, if `current + new <= limit + 0.005`, or if
/// `can_override`; else `FORBIDDEN` with the exact worked message.
pub fn assert_within_credit_limit(
    customer_name: &str,
    limit: Option<Decimal>,
    current_balance: Decimal,
    new_receivable: Decimal,
    can_override: bool,
) -> TxResult<()> {
    if new_receivable <= Decimal::ZERO {
        return Ok(());
    }
    let limit = limit.unwrap_or(Decimal::ZERO);
    if limit <= Decimal::ZERO {
        return Ok(());
    }
    let projected = current_balance + new_receivable;
    if projected <= limit + Decimal::new(5, 3) {
        return Ok(());
    }
    if can_override {
        return Ok(());
    }
    Err(AppError::forbidden(format!(
        "تجاوز الحد الائتماني للعميل \"{customer_name}\": الرصيد الحالي {} + هذه الفاتورة {} = {}، والحد المسموح {}",
        js_number_string_2dp(current_balance),
        js_number_string_2dp(new_receivable),
        js_number_string_2dp(projected),
        js_number_string_2dp(limit)
    ))
    .into())
}

/// `{x:.2}` = `toFixed(2)` of an already-`round2`ed decimal — the message formats every figure with
/// exactly 2 decimals (unlike `js_number_string`, which drops trailing zeros); this local helper is
/// `format!("{:.2}", round2(x))` so the message matches `toFixed(2)` byte-for-byte.
fn js_number_string_2dp(d: Decimal) -> String {
    format!("{:.2}", crate::utils::money::round2(d))
}

/// `canOverride` (`invoices/08-invoices.md` §3.1): `role_can(role, Accounting, Write, overrides) ||
/// role_can(role, Parties, Write, overrides)` (`permissions.ts:101-103`).
pub async fn role_can_override_credit_limit<C: ConnectionTrait>(conn: &C, role: Role) -> TxResult<bool> {
    let settings = crate::core::settings::load(conn).await?;
    let overrides = parse_role_access_overrides(settings.role_access_overrides.as_ref());
    Ok(crate::core::auth::role_can(role, Area::Accounting, Access::Write, &overrides) || crate::core::auth::role_can(role, Area::Parties, Access::Write, &overrides))
}

/// `computeDueDate(instant, days)` (`creditLimit.ts:38-43`): `None` if `days <= 0`; else the
/// instant in the business timezone + `days` calendar days at the same wall time, back to UTC
/// (`setDate` semantics — a plain calendar-day add, not a 24h-multiple add, so it's DST-safe the
/// same way the mock's `Date.setDate` is).
pub fn compute_due_date(now: chrono::DateTime<chrono::Utc>, tz: Option<chrono_tz::Tz>, days: Option<i32>) -> Option<DocDate> {
    let days = days?;
    if days <= 0 {
        return None;
    }
    use chrono::TimeZone;
    let local_naive = match tz {
        Some(tz) => now.with_timezone(&tz).naive_local(),
        None => now.with_timezone(&chrono::Local).naive_local(),
    };
    let shifted_naive = local_naive + chrono::Duration::days(days as i64);
    let shifted_utc: Option<chrono::DateTime<chrono::Utc>> = match tz {
        Some(tz) => tz.from_local_datetime(&shifted_naive).single().map(|d| d.with_timezone(&chrono::Utc)),
        None => chrono::Local.from_local_datetime(&shifted_naive).single().map(|d| d.with_timezone(&chrono::Utc)),
    };
    let shifted_utc = shifted_utc.unwrap_or(now + chrono::Duration::days(days as i64));
    Some(DocDate { day: crate::utils::dates::local_date_key(shifted_utc, tz), instant: Some(shifted_utc) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn credit_limit_passes_when_within() {
        assert!(assert_within_credit_limit("أحمد", Some(dec!(1000)), dec!(500), dec!(400), false).is_ok());
    }

    #[test]
    fn credit_limit_blocks_when_exceeded_and_not_overridable() {
        let err = assert_within_credit_limit("أحمد", Some(dec!(1000)), dec!(900), dec!(200), false);
        assert!(err.is_err());
    }

    #[test]
    fn credit_limit_override_bypasses_the_block() {
        assert!(assert_within_credit_limit("أحمد", Some(dec!(1000)), dec!(900), dec!(200), true).is_ok());
    }

    #[test]
    fn credit_limit_zero_or_negative_new_receivable_always_passes() {
        assert!(assert_within_credit_limit("أحمد", Some(dec!(100)), dec!(900), Decimal::ZERO, false).is_ok());
        assert!(assert_within_credit_limit("أحمد", Some(dec!(100)), dec!(900), dec!(-5), false).is_ok());
    }

    #[test]
    fn compute_due_date_none_when_days_not_positive() {
        let now = chrono::Utc::now();
        assert!(compute_due_date(now, None, None).is_none());
        assert!(compute_due_date(now, None, Some(0)).is_none());
        assert!(compute_due_date(now, None, Some(-1)).is_none());
    }
}
