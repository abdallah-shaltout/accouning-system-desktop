//! Invariant 6: `Σ document outstanding − unallocated credit ± opening balance = sub-ledger
//! balance`, checked four ways (customer/supplier × statement/allocation) — `invariants.ts:175-240`.

use std::collections::{BTreeMap, BTreeSet};

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::error::AppError;
use crate::entities::journal::journal_entries;
use crate::entities::journal::journal_lines::{self, PartyKind};
use crate::entities::parties::parties;
use crate::entities::payments::{payment_allocations, payments};
use crate::shared::ledger::accounts::SystemRole;
use crate::utils::id::Id;
use crate::entities::purchases::{purchase_orders, purchase_returns};
use crate::entities::sales::{invoices, refunds};
use crate::shared::balances::{customer_balance, customer_statement, supplier_balance, supplier_statement, unallocated_credit_for};
use crate::utils::money::round2;

use super::{close_enough, optional_account, InvariantResult, tolerance_cents};

/// Entry sources whose party lines the allocation check already reads through their documents
/// (outstanding, refunds/returns settled to the party, payments' unallocated credit) —
/// `DOCUMENT_SOURCE_KINDS` in `invariants.ts`.
const DOCUMENT_SOURCE_KINDS: &[&str] = &["invoice", "refund", "payment", "purchaseOrder", "purchaseReturn"];

/// `nonDocumentNet` (`invariants.ts`, ACC-0021), for every party of `kind` at once: party lines on
/// the control account with no invoice/refund/purchase/return/payment behind them — an opening
/// balance (and its reversal), a credit expense on a supplier, a manual AR/AP line (B1 write-off or
/// reclassification), an FX revaluation — count in full, less what a payment allocated to an
/// opening balance (`target_kind = opening`, which also left the payment's unallocated credit).
/// Customer lines count debit − credit, supplier lines credit − debit.
async fn non_document_nets<C: ConnectionTrait>(conn: &C, account_id: Id, kind: PartyKind) -> Result<BTreeMap<Id, Decimal>, AppError> {
    let document_entries: BTreeSet<Id> = journal_entries::Entity::find()
        .all(conn)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .filter(|e| e.source_kind.as_deref().is_some_and(|k| DOCUMENT_SOURCE_KINDS.contains(&k)))
        .map(|e| e.id)
        .collect();
    let lines = journal_lines::Entity::find()
        .filter(journal_lines::Column::AccountId.eq(account_id))
        .filter(journal_lines::Column::PartyKind.eq(kind.clone()))
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let mut nets: BTreeMap<Id, Decimal> = BTreeMap::new();
    for l in lines.iter().filter(|l| !document_entries.contains(&l.journal_entry_id)) {
        let Some(party_id) = l.party_id else { continue };
        let signed = if kind == PartyKind::Customer { l.debit - l.credit } else { l.credit - l.debit };
        *nets.entry(party_id).or_insert(Decimal::ZERO) += signed;
    }

    let target_type = if kind == PartyKind::Customer { payments::PaymentTargetType::Customer } else { payments::PaymentTargetType::Supplier };
    let party_payments = payments::Entity::find().filter(payments::Column::TargetType.eq(target_type)).all(conn).await.map_err(AppError::from)?;
    for p in &party_payments {
        let opening_allocated = payment_allocations::Entity::find()
            .filter(payment_allocations::Column::PaymentId.eq(p.id))
            .filter(payment_allocations::Column::TargetKind.eq(payment_allocations::PaymentAllocationTargetKind::Opening))
            .all(conn)
            .await
            .map_err(AppError::from)?
            .iter()
            .fold(Decimal::ZERO, |a, al| a + al.amount);
        if opening_allocated != Decimal::ZERO {
            *nets.entry(p.target_id).or_insert(Decimal::ZERO) -= opening_allocated;
        }
    }
    Ok(nets.into_iter().map(|(id, net)| (id, round2(net))).collect())
}

pub async fn check_party_allocation<C: ConnectionTrait>(conn: &C) -> Result<Vec<InvariantResult>, AppError> {
    let mut results = Vec::new();

    let customers = parties::Entity::find().filter(parties::Column::Kind.eq("customer")).all(conn).await.map_err(AppError::from)?;
    let suppliers = parties::Entity::find().filter(parties::Column::Kind.eq("supplier")).all(conn).await.map_err(AppError::from)?;

    // ACC-0022: with no receivable/payable account nothing was posted to it — every party's
    // balance and statement is 0 (the mock's `partyBalance` / `partyStatementEnd`).
    let receivable = optional_account(conn, SystemRole::Receivable).await?;
    let payable = optional_account(conn, SystemRole::Payable).await?;

    // customer-statement: running balance of the statement's last row == customerBalance().
    let mut cust_statement_mismatch: Vec<&str> = Vec::new();
    for c in customers.iter().filter(|_| receivable.is_some()) {
        let st = customer_statement(conn, c.id).await?;
        let last = st.last().map(|r| r.balance).unwrap_or(Decimal::ZERO);
        if !close_enough(last, customer_balance(conn, c.id).await?, tolerance_cents()) {
            cust_statement_mismatch.push(c.name.as_str());
        }
    }
    results.push(InvariantResult {
        key: "customer-statement".to_string(),
        doc: "§4.6".to_string(),
        passed: cust_statement_mismatch.is_empty(),
        message: format!(
            "customer statement running balance = customerBalance() ({} mismatched: {})",
            cust_statement_mismatch.len(),
            cust_statement_mismatch.join(", ")
        ),
        diff: None,
    });

    // supplier-statement.
    let mut sup_statement_mismatch: Vec<&str> = Vec::new();
    for s in suppliers.iter().filter(|_| payable.is_some()) {
        let st = supplier_statement(conn, s.id).await?;
        let last = st.last().map(|r| r.balance).unwrap_or(Decimal::ZERO);
        if !close_enough(last, supplier_balance(conn, s.id).await?, tolerance_cents()) {
            sup_statement_mismatch.push(s.name.as_str());
        }
    }
    results.push(InvariantResult {
        key: "supplier-statement".to_string(),
        doc: "§4.6".to_string(),
        passed: sup_statement_mismatch.is_empty(),
        message: format!(
            "supplier statement running balance = supplierBalance() ({} mismatched: {})",
            sup_statement_mismatch.len(),
            sup_statement_mismatch.join(", ")
        ),
        diff: None,
    });

    // customer-allocation: Σoutstanding (invoices, non-DRAFT) − Σrefund.settledToReceivable −
    // unallocated credit == customerBalance().
    let all_invoices = invoices::Entity::find().filter(invoices::Column::DeletedAt.is_null()).all(conn).await.map_err(AppError::from)?;
    let all_refunds = refunds::Entity::find().all(conn).await.map_err(AppError::from)?;
    let customer_other = match &receivable {
        Some(a) => non_document_nets(conn, a.id, PartyKind::Customer).await?,
        None => BTreeMap::new(),
    };
    let supplier_other = match &payable {
        Some(a) => non_document_nets(conn, a.id, PartyKind::Supplier).await?,
        None => BTreeMap::new(),
    };

    let mut cust_alloc_mismatch: Vec<&str> = Vec::new();
    for c in &customers {
        let outstanding = round2(all_invoices.iter().filter(|i| i.customer_id == Some(c.id) && i.status != invoices::InvoiceStatus::Draft).fold(
            Decimal::ZERO,
            |a, i| {
                let refunded_to_ar: Decimal =
                    all_refunds.iter().filter(|r| r.invoice_id == i.id).fold(Decimal::ZERO, |b, r| b + r.settled_to_receivable);
                // ACC-0009: an FC invoice's outstanding is in its own currency — compared to the
                // (base) ledger at the invoice's rate, like `open_invoices_for`.
                let fc = i.grand_total - refunded_to_ar - i.paid_amount;
                a + match (&i.currency, i.exchange_rate) {
                    (Some(_), Some(rate)) => round2(fc * rate),
                    _ => fc,
                }
            },
        ));
        let credit = unallocated_credit_for(conn, crate::entities::journal::journal_lines::PartyKind::Customer, c.id).await?;
        let balance = if receivable.is_some() { customer_balance(conn, c.id).await? } else { Decimal::ZERO };
        let other = customer_other.get(&c.id).copied().unwrap_or(Decimal::ZERO);
        if !close_enough(round2(outstanding + other - credit), balance, tolerance_cents()) {
            cust_alloc_mismatch.push(c.name.as_str());
        }
    }
    results.push(InvariantResult {
        key: "customer-allocation".to_string(),
        doc: "§4.6".to_string(),
        passed: cust_alloc_mismatch.is_empty(),
        message: format!(
            "customer: Σoutstanding − unallocated credit = customerBalance() ({} mismatched: {})",
            cust_alloc_mismatch.len(),
            cust_alloc_mismatch.join(", ")
        ),
        diff: None,
    });

    // supplier-allocation: Σoutstanding (RECEIVED POs) − Σreturns (credit -> grandTotal, else
    // settledToPayable) − unallocated credit == supplierBalance().
    let all_pos = purchase_orders::Entity::find()
        .filter(purchase_orders::Column::Status.eq(purchase_orders::PurchaseStatus::Received))
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let all_returns = purchase_returns::Entity::find().all(conn).await.map_err(AppError::from)?;

    let mut sup_alloc_mismatch: Vec<&str> = Vec::new();
    for s in &suppliers {
        let outstanding = round2(all_pos.iter().filter(|p| p.supplier_id == s.id).fold(Decimal::ZERO, |a, p| {
            let reduced_by_returns: Decimal = all_returns.iter().filter(|r| r.purchase_order_id == p.id).fold(Decimal::ZERO, |b, r| {
                b + if r.refund_method == purchase_returns::RefundMethod::Credit { r.grand_total } else { r.settled_to_payable }
            });
            a + (p.grand_total - reduced_by_returns - p.paid_amount)
        }));
        let credit = unallocated_credit_for(conn, crate::entities::journal::journal_lines::PartyKind::Supplier, s.id).await?;
        let balance = if payable.is_some() { supplier_balance(conn, s.id).await? } else { Decimal::ZERO };
        let other = supplier_other.get(&s.id).copied().unwrap_or(Decimal::ZERO);
        if !close_enough(round2(outstanding + other - credit), balance, tolerance_cents()) {
            sup_alloc_mismatch.push(s.name.as_str());
        }
    }
    results.push(InvariantResult {
        key: "supplier-allocation".to_string(),
        doc: "§4.6".to_string(),
        passed: sup_alloc_mismatch.is_empty(),
        message: format!(
            "supplier: Σoutstanding − unallocated credit = supplierBalance() ({} mismatched: {})",
            sup_alloc_mismatch.len(),
            sup_alloc_mismatch.join(", ")
        ),
        diff: None,
    });

    Ok(results)
}
