//! Invariant 6: `Σ document outstanding − unallocated credit ± opening balance = sub-ledger
//! balance`, checked four ways (customer/supplier × statement/allocation) — `invariants.ts:175-240`.

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::error::AppError;
use crate::entities::parties::parties;
use crate::entities::purchases::{purchase_orders, purchase_returns};
use crate::entities::sales::{invoices, refunds};
use crate::shared::balances::{customer_balance, customer_statement, supplier_balance, supplier_statement, unallocated_credit_for};
use crate::utils::money::round2;

use super::{close_enough, InvariantResult, tolerance_cents};

pub async fn check_party_allocation<C: ConnectionTrait>(conn: &C) -> Result<Vec<InvariantResult>, AppError> {
    let mut results = Vec::new();

    let customers = parties::Entity::find().filter(parties::Column::Kind.eq("customer")).all(conn).await.map_err(AppError::from)?;
    let suppliers = parties::Entity::find().filter(parties::Column::Kind.eq("supplier")).all(conn).await.map_err(AppError::from)?;

    // customer-statement: running balance of the statement's last row == customerBalance().
    let mut cust_statement_mismatch: Vec<&str> = Vec::new();
    for c in &customers {
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
    for s in &suppliers {
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

    let mut cust_alloc_mismatch: Vec<&str> = Vec::new();
    for c in &customers {
        let outstanding = round2(all_invoices.iter().filter(|i| i.customer_id == Some(c.id) && i.status != invoices::InvoiceStatus::Draft).fold(
            Decimal::ZERO,
            |a, i| {
                let refunded_to_ar: Decimal =
                    all_refunds.iter().filter(|r| r.invoice_id == i.id).fold(Decimal::ZERO, |b, r| b + r.settled_to_receivable);
                a + (i.grand_total - refunded_to_ar - i.paid_amount)
            },
        ));
        let credit = unallocated_credit_for(conn, crate::entities::journal::journal_lines::PartyKind::Customer, c.id).await?;
        let balance = customer_balance(conn, c.id).await?;
        if !close_enough(round2(outstanding - credit), balance, tolerance_cents()) {
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
        let balance = supplier_balance(conn, s.id).await?;
        if !close_enough(round2(outstanding - credit), balance, tolerance_cents()) {
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
