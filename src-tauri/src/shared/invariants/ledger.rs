//! Invariants 1–5: balanced entries, trial balance / balance sheet, AR/AP control, inventory GL,
//! VAT control (`invariants.ts:57-172`).

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};
use std::collections::{BTreeMap, BTreeSet};

use crate::entities::soft_delete::SoftDelete;
use crate::core::error::AppError;
use crate::entities::expenses::expenses;
use crate::entities::journal::{journal_entries, journal_lines};
use crate::entities::org::accounts;
use crate::entities::purchases::{purchase_orders, purchase_returns};
use crate::entities::sales::{invoices, refunds};
use crate::shared::ledger::accounts::SystemRole;
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::{close_enough, numeric_check, optional_account, InvariantResult, tolerance_cents};

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

    // ACC-0022: a missing receivable/payable account means nothing was posted to it — GL and Σ
    // party balances are both 0.
    let receivable = optional_account(conn, SystemRole::Receivable).await?;
    let payable = optional_account(conn, SystemRole::Payable).await?;

    let ar_gl = match &receivable {
        Some(a) => gl_balance_debit_minus_credit(conn, a.id).await?,
        None => Decimal::ZERO,
    };
    let ap_gl = match &payable {
        Some(a) => -gl_balance_debit_minus_credit(conn, a.id).await?,
        None => Decimal::ZERO,
    };

    let mut ar_sum = Decimal::ZERO;
    if let Some(receivable) = &receivable {
        let customers = parties::Entity::find().filter(parties::Column::Kind.eq("customer")).all(conn).await.map_err(AppError::from)?;
        for c in &customers {
            ar_sum += party_ledger_sum(conn, receivable.id, journal_lines::PartyKind::Customer, c.id, true).await?;
        }
    }
    ar_sum = round2(ar_sum);

    let mut ap_sum = Decimal::ZERO;
    if let Some(payable) = &payable {
        let suppliers = parties::Entity::find().filter(parties::Column::Kind.eq("supplier")).all(conn).await.map_err(AppError::from)?;
        for s in &suppliers {
            ap_sum += party_ledger_sum(conn, payable.id, journal_lines::PartyKind::Supplier, s.id, false).await?;
        }
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

    let inv_gl = match optional_account(conn, SystemRole::Inventory).await? {
        Some(inventory) => gl_balance_debit_minus_credit(conn, inventory.id).await?,
        None => Decimal::ZERO,
    };

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

/// `vatSettlementEntryIds` (`invariants.ts`, ACC-0020): every `VAT_SETTLEMENT` entry plus any entry
/// reversing one. A settlement closes the period's output/input VAT into `vatPayable` — not document
/// VAT — so the VAT control check leaves it out of the ledger side. The payment to the authority
/// (`pay_vat_settlement_now`) posts Dr `vatPayable` / Cr cash-bank and never touches either VAT
/// account, so it needs no exclusion.
async fn vat_settlement_entry_ids<C: ConnectionTrait>(conn: &C) -> Result<BTreeSet<Id>, AppError> {
    let entries = journal_entries::Entity::find().all(conn).await.map_err(AppError::from)?;
    let mut ids: BTreeSet<Id> = entries.iter().filter(|e| e.r#type == journal_entries::JournalEntryType::VatSettlement).map(|e| e.id).collect();
    let reversals: Vec<Id> = entries.iter().filter(|e| e.reversal_of_id.is_some_and(|r| ids.contains(&r))).map(|e| e.id).collect();
    ids.extend(reversals);
    Ok(ids)
}

/// `glBalance(db, role, skipEntry)` (`invariants.ts`): Σ(debit − credit) of the account's lines,
/// leaving out the lines of the given entries.
async fn gl_balance_excluding_entries<C: ConnectionTrait>(conn: &C, account_id: Id, excluded: &BTreeSet<Id>) -> Result<Decimal, AppError> {
    let lines = journal_lines::Entity::find()
        .filter(journal_lines::Column::AccountId.eq(account_id))
        .select_only()
        .column(journal_lines::Column::JournalEntryId)
        .column(journal_lines::Column::Debit)
        .column(journal_lines::Column::Credit)
        .into_tuple::<(Id, Decimal, Decimal)>()
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(round2(lines.iter().filter(|(entry_id, _, _)| !excluded.contains(entry_id)).fold(Decimal::ZERO, |a, (_, d, c)| a + (*d - *c))))
}

/// 5. Output/input VAT GL = Σ VAT on documents for all time (no period filter — matches
/// `scripts/verify/sales.ts`) (`invariants.ts:146-166`).
pub async fn check_vat_control<C: ConnectionTrait>(conn: &C) -> Result<Vec<InvariantResult>, AppError> {
    let vat_output = optional_account(conn, SystemRole::VatOutput).await?;
    let vat_input = optional_account(conn, SystemRole::VatInput).await?;
    let settlement_ids = vat_settlement_entry_ids(conn).await?;

    let invoice_rows = invoices::Entity::find()
        .select_only()
        .column(invoices::Column::Id)
        .column(invoices::Column::TaxAmount)
        .column(invoices::Column::ExchangeRate)
        .column(invoices::Column::Currency)
        .into_tuple::<(Id, Decimal, Option<Decimal>, Option<String>)>()
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let invoice_vat_total = invoice_rows.iter().fold(Decimal::ZERO, |a, (_, tax, rate, currency)| {
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

    // ACC-0009: an FC invoice's refunds post their VAT in base at the invoice's rate, converted
    // cumulatively per invoice (`shared::currency::refund_base_split`) — so Σ refunded VAT in base is
    // the VAT share of `convert_lines_to_base([Σ refunded net, Σ refunded VAT], rate)`, exactly.
    let refund_rows = refunds::Entity::find()
        .select_only()
        .column(refunds::Column::InvoiceId)
        .column(refunds::Column::SubTotal)
        .column(refunds::Column::TaxAmount)
        .into_tuple::<(Id, Decimal, Decimal)>()
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let mut refunded_by_invoice: BTreeMap<Id, (Decimal, Decimal)> = BTreeMap::new();
    for (invoice_id, net, vat) in &refund_rows {
        let e = refunded_by_invoice.entry(*invoice_id).or_insert((Decimal::ZERO, Decimal::ZERO));
        e.0 += *net;
        e.1 += *vat;
    }
    let invoice_fx: BTreeMap<Id, Decimal> =
        invoice_rows.iter().filter_map(|(id, _, rate, currency)| match (currency, rate) { (Some(_), Some(r)) => Some((*id, *r)), _ => None }).collect();
    let refund_tax_total = refunded_by_invoice.iter().fold(Decimal::ZERO, |a, (invoice_id, (net, vat))| {
        a + match invoice_fx.get(invoice_id) {
            Some(rate) => crate::shared::currency::convert_lines_to_base(&[round2(*net), round2(*vat)], *rate)[1],
            None => *vat,
        }
    });

    let output_vat_from_docs = round2(invoice_vat_total - refund_tax_total);
    let output_vat_ledger = match &vat_output {
        Some(a) => -gl_balance_excluding_entries(conn, a.id, &settlement_ids).await?,
        None => Decimal::ZERO,
    };

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
    let input_vat_ledger = match &vat_input {
        Some(a) => gl_balance_excluding_entries(conn, a.id, &settlement_ids).await?,
        None => Decimal::ZERO,
    };

    Ok(vec![
        numeric_check("vat-output", "§4.5", "output VAT GL = Σ document VAT", output_vat_from_docs, output_vat_ledger, tolerance_cents()),
        numeric_check("vat-input", "§4.5", "input VAT GL = Σ document VAT", input_vat_from_docs, input_vat_ledger, tolerance_cents()),
    ])
}
