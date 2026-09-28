//! Invariants 1–5: balanced entries, trial balance / balance sheet, AR/AP control, inventory GL,
//! VAT control (`invariants.ts:57-172`).

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};
use std::collections::BTreeMap;

use crate::entities::soft_delete::SoftDelete;
use crate::core::error::AppError;
use crate::entities::expenses::expenses;
use crate::entities::journal::{journal_entries, journal_lines};
use crate::entities::org::accounts;
use crate::entities::purchases::{purchase_orders, purchase_returns};
use crate::entities::sales::{invoices, refunds};
use crate::shared::ledger::accounts::{resolve_account, AccountCtx, SystemRole};
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::{close_enough, numeric_check, InvariantResult, tolerance_cents};

fn tolerance_balanced() -> Decimal {
    Decimal::new(1, 3)
}

/// 1. Every entry: Σdebit = Σcredit; no line is both debit and credit; ≥ 2 lines. Three separate
/// results (`invariants.ts:59-83`), so a failure in one clause never hides another.
pub async fn check_balanced_entries<C: ConnectionTrait>(conn: &C) -> Result<Vec<InvariantResult>, AppError> {
    let entries = journal_entries::Entity::find().all(conn).await.map_err(AppError::from)?;
    let unbalanced: Vec<&journal_entries::Model> =
        entries.iter().filter(|e| !close_enough(e.total_debit, e.total_credit, tolerance_balanced())).collect();

    let lines = journal_lines::Entity::find().all(conn).await.map_err(AppError::from)?;
    let both_sides: Vec<&journal_lines::Model> = lines.iter().filter(|l| l.debit > Decimal::ZERO && l.credit > Decimal::ZERO).collect();

    let mut line_counts: BTreeMap<Id, usize> = BTreeMap::new();
    for l in &lines {
        *line_counts.entry(l.journal_entry_id).or_insert(0) += 1;
    }
    let too_short: Vec<&journal_entries::Model> = entries.iter().filter(|e| *line_counts.get(&e.id).unwrap_or(&0) < 2).collect();

    Ok(vec![
        InvariantResult {
            key: "balanced-entries".to_string(),
            doc: "§4.1".to_string(),
            passed: unbalanced.is_empty(),
            message: format!(
                "every journal entry is balanced ({} unbalanced: {})",
                unbalanced.len(),
                unbalanced.iter().map(|e| e.number.as_str()).collect::<Vec<_>>().join(", ")
            ),
            diff: None,
        },
        InvariantResult {
            key: "no-both-sided-lines".to_string(),
            doc: "§4.1".to_string(),
            passed: both_sides.is_empty(),
            message: format!("no journal line is both debit and credit ({} offenders)", both_sides.len()),
            diff: None,
        },
        InvariantResult {
            key: "min-two-lines".to_string(),
            doc: "§4.1".to_string(),
            passed: too_short.is_empty(),
            message: format!("every journal entry has >= 2 lines ({} offenders)", too_short.len()),
            diff: None,
        },
    ])
}

/// 2. Trial balance is balanced; balance sheet `A = L + E` (incl. the current result).
/// `invariants.ts:89-124`: per-account net (debit − credit) is rounded to 2dp BEFORE summing.
pub async fn check_trial_balance<C: ConnectionTrait>(conn: &C) -> Result<Vec<InvariantResult>, AppError> {
    let lines = journal_lines::Entity::find().all(conn).await.map_err(AppError::from)?;
    let mut movement: BTreeMap<Id, (Decimal, Decimal)> = BTreeMap::new();
    for l in &lines {
        let e = movement.entry(l.account_id).or_insert((Decimal::ZERO, Decimal::ZERO));
        e.0 += l.debit;
        e.1 += l.credit;
    }

    // Every account, deleted included — the mock's trial balance walks `db.accounts` unfiltered
    // (`invariants.ts:109`), and a deleted account can still carry posted history.
    let account_rows = accounts::Entity::find_including_deleted().all(conn).await.map_err(AppError::from)?;
    let mut trial_debit = Decimal::ZERO;
    let mut trial_credit = Decimal::ZERO;
    let mut assets = Decimal::ZERO;
    let mut liabilities = Decimal::ZERO;
    let mut equity = Decimal::ZERO;

    for a in &account_rows {
        if a.is_group {
            continue;
        }
        let Some((d, c)) = movement.get(&a.id) else { continue };
        let debit_net = round2(*d - *c);
        if debit_net > Decimal::ZERO {
            trial_debit += debit_net;
        } else {
            trial_credit += -debit_net;
        }
        match a.kind.as_str() {
            "ASSET" => assets += debit_net,
            "LIABILITY" => liabilities += -debit_net,
            "EQUITY" => equity += -debit_net,
            "REVENUE" => equity += -debit_net,
            "EXPENSE" => equity -= debit_net,
            _ => {}
        }
    }
    trial_debit = round2(trial_debit);
    trial_credit = round2(trial_credit);
    assets = round2(assets);
    liabilities = round2(liabilities);
    equity = round2(equity);

    Ok(vec![
        numeric_check("trial-balance", "§4.2", "trial balance is balanced", trial_credit, trial_debit, tolerance_cents()),
        numeric_check(
            "balance-sheet",
            "§4.2",
            "balance sheet balanced: A = L + E incl. current result",
            round2(liabilities + equity),
            assets,
            tolerance_cents(),
        ),
    ])
}

async fn party_ledger_sum<C: ConnectionTrait>(
    conn: &C,
    account_id: Id,
    party_kind: journal_lines::PartyKind,
    party_id: Id,
    debit_minus_credit: bool,
) -> Result<Decimal, AppError> {
    let lines = journal_lines::Entity::find()
        .filter(journal_lines::Column::AccountId.eq(account_id))
        .filter(journal_lines::Column::PartyKind.eq(party_kind))
        .filter(journal_lines::Column::PartyId.eq(party_id))
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(round2(lines.iter().fold(Decimal::ZERO, |a, l| a + if debit_minus_credit { l.debit - l.credit } else { l.credit - l.debit })))
}

/// 3. `GL(receivable) = Σ customer sub-ledgers`; `GL(payable) = Σ supplier sub-ledgers`
/// (`invariants.ts:126-136`).
pub async fn check_ar_ap_control<C: ConnectionTrait>(conn: &C) -> Result<Vec<InvariantResult>, AppError> {
    use crate::entities::parties::parties;

    let receivable = resolve_account(conn, SystemRole::Receivable, &AccountCtx::default()).await?;
    let payable = resolve_account(conn, SystemRole::Payable, &AccountCtx::default()).await?;

    let ar_gl = gl_balance_debit_minus_credit(conn, receivable.id).await?;
    let ap_gl = -gl_balance_debit_minus_credit(conn, payable.id).await?;

    let customers = parties::Entity::find().filter(parties::Column::Kind.eq("customer")).all(conn).await.map_err(AppError::from)?;
    let mut ar_sum = Decimal::ZERO;
    for c in &customers {
        ar_sum += party_ledger_sum(conn, receivable.id, journal_lines::PartyKind::Customer, c.id, true).await?;
    }
    ar_sum = round2(ar_sum);

    let suppliers = parties::Entity::find().filter(parties::Column::Kind.eq("supplier")).all(conn).await.map_err(AppError::from)?;
    let mut ap_sum = Decimal::ZERO;
    for s in &suppliers {
        ap_sum += party_ledger_sum(conn, payable.id, journal_lines::PartyKind::Supplier, s.id, false).await?;
    }
    ap_sum = round2(ap_sum);

    Ok(vec![
        numeric_check("ar-control", "§4.3", "AR GL = Σ customer balances", ar_sum, ar_gl, tolerance_cents()),
        numeric_check("ap-control", "§4.3", "AP GL = Σ supplier balances", ap_sum, ap_gl, tolerance_cents()),
    ])
}

/// `glBalance` (`invariants.ts:48-55`): Σ(debit − credit) of every line on the role's account.
pub(crate) async fn gl_balance_debit_minus_credit<C: ConnectionTrait>(conn: &C, account_id: Id) -> Result<Decimal, AppError> {
    let lines = journal_lines::Entity::find()
        .filter(journal_lines::Column::AccountId.eq(account_id))
        .select_only()
        .column(journal_lines::Column::Debit)
        .column(journal_lines::Column::Credit)
        .into_tuple::<(Decimal, Decimal)>()
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(round2(lines.iter().fold(Decimal::ZERO, |a, (d, c)| a + (*d - *c))))
}

/// 4. `GL(inventory) = Σ product.stockValue` (`invariants.ts:139-143`).
pub async fn check_inventory_gl<C: ConnectionTrait>(conn: &C) -> Result<InvariantResult, AppError> {
    use crate::entities::catalog::products;

    let inventory = resolve_account(conn, SystemRole::Inventory, &AccountCtx::default()).await?;
    let inv_gl = gl_balance_debit_minus_credit(conn, inventory.id).await?;

    let rows = products::Entity::find()
        .select_only()
        .column(products::Column::Type)
        .column(products::Column::StockValue)
        .into_tuple::<(String, Decimal)>()
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let inv_sum = round2(rows.iter().fold(Decimal::ZERO, |a, (t, v)| a + if t == "product" { *v } else { Decimal::ZERO }));

    Ok(numeric_check("inventory-gl", "§4.4", "inventory GL = Σ product.stockValue", inv_sum, inv_gl, tolerance_cents()))
}

/// 5. Output/input VAT GL = Σ VAT on documents for all time (no period filter — matches
/// `scripts/verify/sales.ts`) (`invariants.ts:146-166`).
pub async fn check_vat_control<C: ConnectionTrait>(conn: &C) -> Result<Vec<InvariantResult>, AppError> {
    let vat_output = resolve_account(conn, SystemRole::VatOutput, &AccountCtx::default()).await?;
    let vat_input = resolve_account(conn, SystemRole::VatInput, &AccountCtx::default()).await?;

    let invoice_rows = invoices::Entity::find()
        .select_only()
        .column(invoices::Column::TaxAmount)
        .column(invoices::Column::ExchangeRate)
        .column(invoices::Column::Currency)
        .into_tuple::<(Decimal, Option<Decimal>, Option<String>)>()
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let invoice_vat_total = invoice_rows.iter().fold(Decimal::ZERO, |a, (tax, rate, currency)| {
        let base = if currency.is_some() {
            if let Some(rate) = rate {
                round2(*tax * *rate)
            } else {
                *tax
            }
        } else {
            *tax
        };
        a + base
    });

    let refund_tax_total = refunds::Entity::find()
        .select_only()
        .column(refunds::Column::TaxAmount)
        .into_tuple::<Decimal>()
        .all(conn)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .fold(Decimal::ZERO, |a, t| a + t);

    let output_vat_from_docs = round2(invoice_vat_total - refund_tax_total);
    let output_vat_ledger = -gl_balance_debit_minus_credit(conn, vat_output.id).await?;

    let recoverable_pos = purchase_orders::Entity::find()
        .filter(purchase_orders::Column::Status.eq(purchase_orders::PurchaseStatus::Received))
        .filter(
            Condition::any()
                .add(purchase_orders::Column::VatNotRecoverable.eq(false))
                .add(purchase_orders::Column::VatNotRecoverable.is_null()),
        )
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let recoverable_po_ids: std::collections::BTreeSet<Id> = recoverable_pos.iter().map(|p| p.id).collect();
    let po_tax_total = recoverable_pos.iter().fold(Decimal::ZERO, |a, p| a + p.tax_amount);

    let returns = purchase_returns::Entity::find().all(conn).await.map_err(AppError::from)?;
    let return_tax_total =
        returns.iter().filter(|r| recoverable_po_ids.contains(&r.purchase_order_id)).fold(Decimal::ZERO, |a, r| a + r.tax_amount);

    let expense_tax_total = expenses::Entity::find()
        .select_only()
        .column(expenses::Column::TaxAmount)
        .into_tuple::<Decimal>()
        .all(conn)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .fold(Decimal::ZERO, |a, t| a + t);

    let input_vat_from_docs = round2(po_tax_total - return_tax_total + expense_tax_total);
    let input_vat_ledger = gl_balance_debit_minus_credit(conn, vat_input.id).await?;

    Ok(vec![
        numeric_check("vat-output", "§4.5", "output VAT GL = Σ document VAT", output_vat_from_docs, output_vat_ledger, tolerance_cents()),
        numeric_check("vat-input", "§4.5", "input VAT GL = Σ document VAT", input_vat_from_docs, input_vat_ledger, tolerance_cents()),
    ])
}
