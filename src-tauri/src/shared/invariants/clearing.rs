//! Invariant 10 (card/wallet clearing), 11 (shift variance), and the un-numbered FX-conversion
//! check (`invariants.ts:312-333`, `366-417` region of the file — card/wallet, shift, FX).

use std::collections::BTreeSet;

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::error::AppError;
use crate::entities::journal::journal_entries;
use crate::entities::org::payment_methods;
use crate::entities::payments::card_settlement_groups;
use crate::entities::soft_delete::SoftDelete;
use crate::entities::sales::{invoice_tenders, shifts};
use crate::shared::invariants::ledger::gl_balance_debit_minus_credit;
use crate::shared::ledger::accounts::SystemRole;
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::{close_enough, numeric_check, optional_account, InvariantResult, tolerance_cents};

/// 10. Card and wallet clearing balances match the unsettled tenders (`invariants.ts` §4.10).
pub async fn check_clearing_accounts<C: ConnectionTrait>(conn: &C) -> Result<Vec<InvariantResult>, AppError> {
    // All methods, deleted included — the mock reads `db.paymentMethods` unfiltered, and old
    // tenders still reference a method deleted since (`invariants.ts:306-307`).
    let methods = payment_methods::Entity::find_including_deleted().all(conn).await.map_err(AppError::from)?;
    let card_method_ids: BTreeSet<Id> = methods.iter().filter(|m| m.account_role == "cardClearing").map(|m| m.id).collect();
    let wallet_method_ids: BTreeSet<Id> = methods.iter().filter(|m| m.account_role == "walletClearing").map(|m| m.id).collect();

    let tenders = invoice_tenders::Entity::find().all(conn).await.map_err(AppError::from)?;
    let tender_sum = |ids: &BTreeSet<Id>| round2(tenders.iter().filter(|t| ids.contains(&t.payment_method_id)).fold(Decimal::ZERO, |a, t| a + t.amount));

    let settlement_groups = card_settlement_groups::Entity::find().all(conn).await.map_err(AppError::from)?;
    let settled_sum =
        |ids: &BTreeSet<Id>| round2(settlement_groups.iter().filter(|g| ids.contains(&g.payment_method_id)).fold(Decimal::ZERO, |a, g| a + g.amount));

    // ACC-0022: a chart with no clearing account (the `basic` template) has nothing posted to it.
    let card_clearing = optional_account(conn, SystemRole::CardClearing).await?;
    let wallet_clearing = optional_account(conn, SystemRole::WalletClearing).await?;

    let card_tenders = round2(tender_sum(&card_method_ids) - settled_sum(&card_method_ids));
    let card_ledger = match &card_clearing {
        Some(a) => gl_balance_debit_minus_credit(conn, a.id).await?,
        None => Decimal::ZERO,
    };
    let wallet_tenders = round2(tender_sum(&wallet_method_ids) - settled_sum(&wallet_method_ids));
    let wallet_ledger = match &wallet_clearing {
        Some(a) => gl_balance_debit_minus_credit(conn, a.id).await?,
        None => Decimal::ZERO,
    };

    Ok(vec![
        numeric_check("card-clearing", "§4.10", "card clearing = unsettled card tenders", card_tenders, card_ledger, tolerance_cents()),
        numeric_check("wallet-clearing", "§4.10", "wallet clearing = unsettled wallet tenders", wallet_tenders, wallet_ledger, tolerance_cents()),
    ])
}

/// 11. Every closed shift: expected cash − counted cash = the posted over/short amount
/// (`invariants.ts` §4.11).
pub async fn check_shift_variance<C: ConnectionTrait>(conn: &C) -> Result<InvariantResult, AppError> {
    let closed = shifts::Entity::find().filter(shifts::Column::Status.eq(shifts::ShiftStatus::Closed)).all(conn).await.map_err(AppError::from)?;
    if closed.is_empty() {
        return Ok(InvariantResult {
            key: "shift-variance".to_string(),
            doc: "§4.11".to_string(),
            passed: true,
            message: "no closed shifts yet — vacuously holds".to_string(),
            diff: None,
        });
    }
    let mut offenders = Vec::new();
    for s in &closed {
        let expected = s.expected_cash.unwrap_or(Decimal::ZERO);
        let counted = s.counted_cash.unwrap_or(Decimal::ZERO);
        let stored_variance = s.variance.unwrap_or(Decimal::ZERO);
        let derived_variance = round2(counted - expected);
        if !close_enough(stored_variance, derived_variance, tolerance_cents()) {
            offenders.push(format!("{} (stored {} ≠ derived {})", s.number, stored_variance, derived_variance));
        }
    }
    Ok(InvariantResult {
        key: "shift-variance".to_string(),
        doc: "§4.11".to_string(),
        passed: offenders.is_empty(),
        message: if offenders.is_empty() {
            format!("all {} closed shift(s) match", closed.len())
        } else {
            format!("{} mismatched: {}", offenders.len(), offenders.join(", "))
        },
        diff: None,
    })
}

/// FX base = fc × rate within 0.01 — every journal line carrying `amountFc` + `rate` converts to
/// its own `debit`/`credit` (base currency) within tolerance (`invariants.ts` — un-numbered, "FX
/// base = fc × rate").
pub async fn check_fx_conversion<C: ConnectionTrait>(conn: &C) -> Result<InvariantResult, AppError> {
    use crate::entities::journal::journal_lines;

    let entries = journal_entries::Entity::find().find_with_related(journal_lines::Entity).all(conn).await.map_err(AppError::from)?;
    let mut offenders = Vec::new();
    for (entry, lines) in &entries {
        for l in lines {
            let (Some(amount_fc), Some(rate)) = (l.amount_fc, l.rate) else { continue };
            let base = if l.debit > Decimal::ZERO { l.debit } else { l.credit };
            let expected = round2(amount_fc * rate);
            if !close_enough(base, expected, tolerance_cents()) {
                offenders.push(format!("{}/{} (fc {} × {} = {} ≠ {})", entry.number, l.id, amount_fc, rate, expected, base));
            }
        }
    }
    Ok(InvariantResult {
        key: "fx-conversion".to_string(),
        doc: "§4 (FX base = fc × rate within 0.01)".to_string(),
        passed: offenders.is_empty(),
        message: if offenders.is_empty() {
            "every FX line converts within 0.01".to_string()
        } else {
            format!("{} mismatched: {}", offenders.len(), offenders.join(", "))
        },
        diff: None,
    })
}
