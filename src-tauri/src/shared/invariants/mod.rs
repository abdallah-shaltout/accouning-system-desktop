//! `shared::invariants` (D-5) — the Rust port of `runAllInvariants()` (`src/mocks/backend/
//! invariants.ts`), the 14-check accounting invariant suite (docs/v2/02-accounting-review.md §4).
//! Same keys, same `doc` strings, same English message templates, same tolerances/rounding points
//! as the mock, so Part 04's parity harness can diff mock vs Rust results key by key. Every
//! function is pure/read-only over the connection it's given — takes `&impl ConnectionTrait` so it
//! runs inside `with_read` (one RR snapshot) **and** inside a write transaction (e.g. the D10
//! importer, before its own commit).
//!
//! Split into one file per invariant group, mirroring the mock's own function grouping:
//! - [`ledger`] — balanced entries, trial balance/balance sheet, AR/AP control, inventory GL, VAT.
//! - [`parties`] — party statement/allocation checks.
//! - [`documents`] — source-ref integrity, lock date, opening balance equity, drafts isolation,
//!   allocations-within-total.
//! - [`clearing`] — card/wallet clearing, shift variance, FX conversion.

mod clearing;
mod documents;
mod ledger;
mod parties;

use serde::Serialize;
use sea_orm::ConnectionTrait;

use crate::core::error::AppError;

/// The minor-unit diff shown when a numeric invariant fails — `{ expected, actual,
/// deltaMinorUnits }`, matching `InvariantResult['diff']` in the mock exactly (camelCase on the
/// wire via `#[serde(rename_all = "camelCase")]`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvariantDiff {
    /// JSON numbers on the wire, like the mock (rule 5: `Decimal` in Rust, never a float).
    #[serde(with = "crate::utils::money::serde_number")]
    pub expected: Decimal,
    #[serde(with = "crate::utils::money::serde_number")]
    pub actual: Decimal,
    pub delta_minor_units: i64,
}

/// One invariant's result — `key`/`doc`/`passed`/`message`/`diff`, in that exact shape
/// (`invariants.ts`'s `InvariantResult`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvariantResult {
    pub key: String,
    pub doc: String,
    pub passed: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<InvariantDiff>,
}

use rust_decimal::Decimal;

/// `closeEnough` (`invariants.ts:34-36`): `|a - b| <= tolerance` (default `0.01`).
pub(crate) fn close_enough(a: Decimal, b: Decimal, tolerance: Decimal) -> bool {
    (a - b).abs() <= tolerance
}

/// `roleAccount` (`invariants.ts`, ACC-0022): the role's account, or `None` when the chart has none
/// (the `basic` template has no card/wallet clearing account; an empty company has no chart yet).
/// `run_all` must never fail on that: nothing can have posted to a missing account, so its GL
/// balance is 0 and each check compares that 0 with its own documents. Only `NOT_FOUND` maps to
/// `None` — a database error still propagates.
pub(crate) async fn optional_account<C: ConnectionTrait>(
    conn: &C,
    role: crate::shared::ledger::accounts::SystemRole,
) -> Result<Option<crate::entities::org::accounts::Model>, AppError> {
    match crate::shared::ledger::accounts::resolve_account(conn, role, &crate::shared::ledger::accounts::AccountCtx::default()).await {
        Ok(a) => Ok(Some(a)),
        Err(AppError::NotFound { .. }) => Ok(None),
        Err(e) => Err(e),
    }
}

/// `closeEnough`'s default tolerance (`0.01`) — the one copy the check modules share.
pub(crate) fn tolerance_cents() -> Decimal {
    Decimal::new(1, 2) // 0.01
}

/// `deltaMinorUnits` (`invariants.ts:44`): `Math.round((actual - expected) * 100)` — JS
/// `Math.round` semantics, i.e. round **half up** (toward +∞), NOT half-away-from-zero. Ported
/// literally: `floor(x + 0.5)`.
pub(crate) fn delta_minor_units(expected: Decimal, actual: Decimal) -> i64 {
    use rust_decimal::prelude::ToPrimitive;
    let diff = (actual - expected) * Decimal::from(100);
    // Exact decimal arithmetic (rule 5: no floats); `floor(x + 0.5)` is JS `Math.round`.
    (diff + Decimal::new(5, 1)).floor().to_i64().unwrap_or(0)
}

/// `numericCheck` (`invariants.ts:38-46`): builds a passed/failed result from an expected/actual
/// pair, with the mock's exact message shape `"{label} ({actual} vs {expected})"` and a `diff`
/// only when it failed.
pub(crate) fn numeric_check(key: &str, doc: &str, label: &str, expected: Decimal, actual: Decimal, tolerance: Decimal) -> InvariantResult {
    let passed = close_enough(expected, actual, tolerance);
    InvariantResult {
        key: key.to_string(),
        doc: doc.to_string(),
        passed,
        message: format!("{label} ({} vs {})", crate::utils::money::js_number_string(actual), crate::utils::money::js_number_string(expected)),
        diff: if passed {
            None
        } else {
            Some(InvariantDiff { expected, actual, delta_minor_units: delta_minor_units(expected, actual) })
        },
    }
}

/// Runs every invariant and flattens the results, in the same order `runAllInvariants` returns
/// them (`invariants.ts:365-382`).
pub async fn run_all<C: ConnectionTrait>(conn: &C) -> Result<Vec<InvariantResult>, AppError> {
    let mut out = Vec::new();
    out.extend(ledger::check_balanced_entries(conn).await?);
    out.extend(ledger::check_trial_balance(conn).await?);
    out.extend(ledger::check_ar_ap_control(conn).await?);
    out.push(ledger::check_inventory_gl(conn).await?);
    out.extend(ledger::check_vat_control(conn).await?);
    out.extend(parties::check_party_allocation(conn).await?);
    out.extend(documents::check_source_ref_integrity(conn).await?);
    out.push(documents::check_lock_date(conn).await?);
    out.push(documents::check_opening_balance_equity(conn).await?);
    out.extend(clearing::check_clearing_accounts(conn).await?);
    out.push(clearing::check_shift_variance(conn).await?);
    out.push(documents::check_drafts_isolated(conn).await?);
    out.push(documents::check_allocations_within_total(conn).await?);
    out.push(clearing::check_fx_conversion(conn).await?);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn close_enough_matches_default_tolerance() {
        assert!(close_enough(dec!(1.00), dec!(1.009), tolerance_cents()));
        assert!(!close_enough(dec!(1.00), dec!(1.02), tolerance_cents()));
    }

    #[test]
    fn delta_minor_units_rounds_half_up_like_js_math_round() {
        // JS Math.round(0.5) === 1 (half up, not half-away-from-zero — same result for positives,
        // but Math.round(-0.5) === -0 in JS, i.e. rounds toward +Infinity, not away from zero).
        assert_eq!(delta_minor_units(dec!(1.00), dec!(1.005)), 1);
        assert_eq!(delta_minor_units(dec!(1.00), dec!(0.995)), 0);
    }

    #[test]
    fn numeric_check_passes_within_tolerance() {
        let r = numeric_check("k", "§4", "label", dec!(100), dec!(100.005), tolerance_cents());
        assert!(r.passed);
        assert!(r.diff.is_none());
    }

    #[test]
    fn numeric_check_fails_outside_tolerance_with_diff() {
        let r = numeric_check("k", "§4", "label", dec!(100), dec!(101), tolerance_cents());
        assert!(!r.passed);
        assert_eq!(r.message, "label (101 vs 100)");
        let diff = r.diff.unwrap();
        assert_eq!(diff.expected, dec!(100));
        assert_eq!(diff.actual, dec!(101));
        assert_eq!(diff.delta_minor_units, 100);
    }
}
