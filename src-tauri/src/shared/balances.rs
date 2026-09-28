//! `shared::balances` (D-4, P2-26) — read-only party balances/statements, a behaviour-exact port
//! of `src/mocks/backend/balances.ts` and the allocation helpers `src/mocks/backend/payments.ts:81-106`.
//!
//! Every function here reads a party's lines off the receivable/payable **control account**
//! (docs/v2/02-accounting-review.md C2): "a party's balance is Σ(debit − credit) of the journal
//! lines tagged with that party on the receivable/payable control account — not by summing open
//! documents." This is what makes `ar-control`/`ap-control` (D-5) hold structurally.

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::entities::journal::journal_entries;
use crate::entities::journal::journal_lines::{self, PartyKind};
use crate::entities::payments::{payment_allocations, payments};
use crate::entities::purchases::purchase_orders;
use crate::entities::sales::invoices;
use crate::shared::ledger::accounts::{resolve_account, AccountCtx, SystemRole};
use crate::utils::id::Id;
use crate::utils::money::round2;

/// One control-account ledger line tagged with a party, joined to its entry's `source_kind`/
/// `source_id`/`source_number` (needed by the statement builders below).
struct PartyLine {
    id: Id,
    date_key: String,
    debit: Decimal,
    credit: Decimal,
    amount_fc: Option<Decimal>,
    description: Option<String>,
    source_kind: Option<String>,
    source_id: Option<Id>,
    source_number: Option<String>,
    entry_number: String,
    entry_description: String,
}

async fn party_ledger_lines<C: ConnectionTrait>(conn: &C, kind: PartyKind, party_id: Id, account_id: Id) -> Result<Vec<PartyLine>, AppError> {
    let rows = journal_lines::Entity::find()
        .filter(journal_lines::Column::AccountId.eq(account_id))
        .filter(journal_lines::Column::PartyKind.eq(kind))
        .filter(journal_lines::Column::PartyId.eq(party_id))
        .find_also_related(journal_entries::Entity)
        .all(conn)
        .await
        .map_err(AppError::from)?;

    let mut out = Vec::with_capacity(rows.len());
    for (line, entry) in rows {
        let Some(entry) = entry else { continue };
        out.push(PartyLine {
            id: line.id,
            date_key: entry.date().key(),
            debit: line.debit,
            credit: line.credit,
            amount_fc: line.amount_fc,
            description: line.description.clone(),
            source_kind: entry.source_kind.clone(),
            source_id: entry.source_id,
            source_number: entry.source_number.clone(),
            entry_number: entry.number.clone(),
            entry_description: entry.description.clone(),
        });
    }
    Ok(out)
}

async fn receivable_account_id<C: ConnectionTrait>(conn: &C) -> Result<Id, AppError> {
    Ok(resolve_account(conn, SystemRole::Receivable, &AccountCtx::default()).await?.id)
}

async fn payable_account_id<C: ConnectionTrait>(conn: &C) -> Result<Id, AppError> {
    Ok(resolve_account(conn, SystemRole::Payable, &AccountCtx::default()).await?.id)
}

/// `customerBalance` (`balances.ts:21-23`): Σ(debit − credit) of the customer's receivable lines.
pub async fn customer_balance<C: ConnectionTrait>(conn: &C, customer_id: Id) -> Result<Decimal, AppError> {
    let account_id = receivable_account_id(conn).await?;
    let lines = party_ledger_lines(conn, PartyKind::Customer, customer_id, account_id).await?;
    Ok(round2(lines.iter().fold(Decimal::ZERO, |a, l| a + (l.debit - l.credit))))
}

/// `supplierBalance` (`balances.ts:25-27`): Σ(credit − debit) of the supplier's payable lines.
pub async fn supplier_balance<C: ConnectionTrait>(conn: &C, supplier_id: Id) -> Result<Decimal, AppError> {
    let account_id = payable_account_id(conn).await?;
    let lines = party_ledger_lines(conn, PartyKind::Supplier, supplier_id, account_id).await?;
    Ok(round2(lines.iter().fold(Decimal::ZERO, |a, l| a + (l.credit - l.debit))))
}

/// `customerBalanceFc` (`balances.ts:35-37`): the customer's balance in its OWN foreign currency —
/// only lines actually tagged with `amount_fc` count.
pub async fn customer_balance_fc<C: ConnectionTrait>(conn: &C, customer_id: Id) -> Result<Decimal, AppError> {
    let account_id = receivable_account_id(conn).await?;
    let lines = party_ledger_lines(conn, PartyKind::Customer, customer_id, account_id).await?;
    Ok(round2(lines.iter().filter(|l| l.amount_fc.is_some()).fold(Decimal::ZERO, |a, l| {
        let fc = l.amount_fc.unwrap_or(Decimal::ZERO);
        a + if l.debit > Decimal::ZERO { fc } else { -fc }
    })))
}

/// `supplierBalanceFc` (`balances.ts:39-41`).
pub async fn supplier_balance_fc<C: ConnectionTrait>(conn: &C, supplier_id: Id) -> Result<Decimal, AppError> {
    let account_id = payable_account_id(conn).await?;
    let lines = party_ledger_lines(conn, PartyKind::Supplier, supplier_id, account_id).await?;
    Ok(round2(lines.iter().filter(|l| l.amount_fc.is_some()).fold(Decimal::ZERO, |a, l| {
        let fc = l.amount_fc.unwrap_or(Decimal::ZERO);
        a + if l.credit > Decimal::ZERO { fc } else { -fc }
    })))
}

/// One row of a party statement (`PartyStatementRow`): the ledger line plus a running balance.
#[derive(Debug, Clone)]
pub struct PartyStatementRow {
    pub id: Id,
    pub date_key: String,
    pub kind: &'static str,
    pub ref_id: Id,
    pub number: String,
    pub description: String,
    pub debit: Decimal,
    pub credit: Decimal,
    pub balance: Decimal,
}

fn with_running_balance(mut rows: Vec<(PartyLine, &'static str, Id, String, String)>, sign: Decimal) -> Vec<PartyStatementRow> {
    rows.sort_by(|a, b| a.0.date_key.cmp(&b.0.date_key));
    let mut balance = Decimal::ZERO;
    rows.into_iter()
        .map(|(l, kind, ref_id, number, description)| {
            balance = round2(balance + sign * (l.debit - l.credit));
            PartyStatementRow { id: l.id, date_key: l.date_key.clone(), kind, ref_id, number, description, debit: l.debit, credit: l.credit, balance }
        })
        .collect()
}

/// `customerStatement` (`balances.ts:50-70`): one row per receivable-control-account line tagged
/// with this customer. `kind` follows `sourceRef.kind`: `refund` -> `refund`, `payment` ->
/// `payment`, `opening` -> `opening`, else `invoice`. Running balance = Σ(debit − credit), so the
/// final row equals `customer_balance` by construction.
pub async fn customer_statement<C: ConnectionTrait>(conn: &C, customer_id: Id) -> Result<Vec<PartyStatementRow>, AppError> {
    let account_id = receivable_account_id(conn).await?;
    let lines = party_ledger_lines(conn, PartyKind::Customer, customer_id, account_id).await?;
    let rows = lines
        .into_iter()
        .map(|l| {
            let kind = match l.source_kind.as_deref() {
                Some("refund") => "refund",
                Some("payment") => "payment",
                Some("opening") => "opening",
                _ => "invoice",
            };
            let ref_id = l.source_id.unwrap_or(l.id);
            let number = l.source_number.clone().unwrap_or_else(|| l.entry_number.clone());
            let description = l.description.clone().unwrap_or_else(|| l.entry_description.clone());
            (l, kind, ref_id, number, description)
        })
        .collect();
    Ok(with_running_balance(rows, Decimal::ONE))
}

/// `supplierStatement` (`balances.ts:76-95`): `kind` follows `sourceRef.kind`: `purchaseReturn`,
/// `payment`, `opening`, else `purchaseOrder`. Running balance = Σ(credit − debit).
pub async fn supplier_statement<C: ConnectionTrait>(conn: &C, supplier_id: Id) -> Result<Vec<PartyStatementRow>, AppError> {
    let account_id = payable_account_id(conn).await?;
    let lines = party_ledger_lines(conn, PartyKind::Supplier, supplier_id, account_id).await?;
    let rows = lines
        .into_iter()
        .map(|l| {
            let kind = match l.source_kind.as_deref() {
                Some("purchaseReturn") => "purchaseReturn",
                Some("payment") => "payment",
                Some("opening") => "opening",
                _ => "purchaseOrder",
            };
            let ref_id = l.source_id.unwrap_or(l.id);
            let number = l.source_number.clone().unwrap_or_else(|| l.entry_number.clone());
            let description = l.description.clone().unwrap_or_else(|| l.entry_description.clone());
            (l, kind, ref_id, number, description)
        })
        .collect();
    Ok(with_running_balance(rows, -Decimal::ONE))
}

/// `allocatedTotal` (`payments.ts:82-84`): Σ of a payment's allocations (in the control-account
/// amount).
pub async fn allocated_total<C: ConnectionTrait>(conn: &C, payment_id: Id) -> Result<Decimal, AppError> {
    let allocations = payment_allocations::Entity::find()
        .filter(payment_allocations::Column::PaymentId.eq(payment_id))
        .all(conn)
        .await
        .map_err(AppError::from)?;
    Ok(allocations.iter().fold(Decimal::ZERO, |a, x| a + x.amount))
}

/// `unallocatedAmount` (`payments.ts:94-97`): money still sitting on a payment, in the base
/// currency actually tendered. An FC allocation's cash-consumed is `amountFc × payment.rate`
/// (never its AR/AP-side `amount`) — see the mock's own comment on why subtracting
/// `allocated_total` here would strand a realized FX gain/loss as phantom unallocated credit.
pub async fn unallocated_amount<C: ConnectionTrait>(conn: &C, payment: &payments::Model) -> Result<Decimal, AppError> {
    let allocations = payment_allocations::Entity::find()
        .filter(payment_allocations::Column::PaymentId.eq(payment.id))
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let cash_consumed = allocations.iter().fold(Decimal::ZERO, |a, alloc| {
        a + match (alloc.amount_fc, payment.rate) {
            (Some(fc), Some(rate)) => round2(fc * rate),
            _ => alloc.amount,
        }
    });
    Ok((round2(payment.amount - cash_consumed)).max(Decimal::ZERO))
}

/// `unallocatedCreditFor` (`payments.ts:99-106`): Σ unallocated credit across every
/// RECEIVED/PAID payment for a party.
pub async fn unallocated_credit_for<C: ConnectionTrait>(conn: &C, target_type: PartyKind, target_id: Id) -> Result<Decimal, AppError> {
    use crate::entities::payments::payments::{PaymentTargetType, PaymentType};
    let payment_type = match target_type {
        PartyKind::Customer => PaymentType::Received,
        PartyKind::Supplier => PaymentType::Paid,
    };
    let target_type_col = match target_type {
        PartyKind::Customer => PaymentTargetType::Customer,
        PartyKind::Supplier => PaymentTargetType::Supplier,
    };
    let rows = payments::Entity::find()
        .filter(payments::Column::Type.eq(payment_type))
        .filter(payments::Column::TargetType.eq(target_type_col))
        .filter(payments::Column::TargetId.eq(target_id))
        .filter(payments::Column::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?;

    let mut total = Decimal::ZERO;
    for p in &rows {
        total += unallocated_amount(conn, p).await?;
    }
    Ok(total)
}

/// An open document a payment can be allocated against (`OpenDocument`, `payments.ts:29-70`),
/// pared to what a balance/allocation reader needs (full DTO assembly is Part 03's job).
pub struct OpenDocument {
    pub id: Id,
    pub outstanding: Decimal,
}

/// `getOpenDocumentsFor` (customer side): completed invoices with `invoiceOutstanding > 0`, sorted
/// by date. Outstanding is always base-currency (FC invoices convert via their own rate).
pub async fn open_invoices_for<C: ConnectionTrait>(conn: &C, customer_id: Id) -> Result<Vec<OpenDocument>, AppError> {
    let rows = invoices::Entity::find()
        .filter(invoices::Column::CustomerId.eq(customer_id))
        .filter(invoices::Column::Status.eq(invoices::InvoiceStatus::Completed))
        .filter(invoices::Column::DeletedAt.is_null())
        .order_by_asc(invoices::Column::DateDay)
        .all(conn)
        .await
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .filter_map(|i| {
            let fc_outstanding = round2(i.grand_total - i.refunded_amount - i.paid_amount);
            if fc_outstanding <= Decimal::ZERO {
                return None;
            }
            let outstanding = match (i.currency.as_ref(), i.exchange_rate) {
                (Some(_), Some(rate)) => round2(fc_outstanding * rate),
                _ => fc_outstanding,
            };
            Some(OpenDocument { id: i.id, outstanding })
        })
        .collect())
}

/// `getOpenDocumentsFor` (supplier side): received purchase orders with `purchaseOutstanding > 0`.
pub async fn open_purchase_orders_for<C: ConnectionTrait>(conn: &C, supplier_id: Id) -> Result<Vec<OpenDocument>, AppError> {
    let rows = purchase_orders::Entity::find()
        .filter(purchase_orders::Column::SupplierId.eq(supplier_id))
        .filter(purchase_orders::Column::Status.eq(purchase_orders::PurchaseStatus::Received))
        .filter(purchase_orders::Column::DeletedAt.is_null())
        .order_by_asc(purchase_orders::Column::DateDay)
        .all(conn)
        .await
        .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .filter_map(|p| {
            let fc_outstanding = round2(p.grand_total - p.returned_amount - p.paid_amount);
            if fc_outstanding <= Decimal::ZERO {
                return None;
            }
            let outstanding = match (p.currency.as_ref(), p.exchange_rate) {
                (Some(_), Some(rate)) => round2(fc_outstanding * rate),
                _ => fc_outstanding,
            };
            Some(OpenDocument { id: p.id, outstanding })
        })
        .collect())
}
