//! `shared::balances` (D-4, P2-26) — read-only party balances/statements, a behaviour-exact port
//! of `src/mocks/backend/balances.ts` and the allocation helpers `src/mocks/backend/payments.ts:81-106`.
//!
//! Every function here reads a party's lines off the receivable/payable **control account**
//! (docs/v2/02-accounting-review.md C2): "a party's balance is Σ(debit − credit) of the journal
//! lines tagged with that party on the receivable/payable control account — not by summing open
//! documents." This is what makes `ar-control`/`ap-control` (D-5) hold structurally.

use std::collections::HashMap;

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::entities::journal::journal_entries;
use crate::entities::journal::journal_lines::{self, PartyKind};
use crate::entities::payments::{payment_allocations, payments};
use crate::entities::purchases::purchase_orders;
use crate::entities::sales::invoices;
use crate::shared::ledger::accounts::{resolve_account, AccountCtx, SystemRole};
use crate::utils::dates::DocDate;
use crate::utils::id::Id;
use crate::utils::money::round2;

/// One control-account ledger line tagged with a party, joined to its entry's `source_kind`/
/// `source_id`/`source_number` (needed by the statement builders below).
///
/// G-5: also carries the entry's `created_at`/`id` and the line's own `position` — the mock's
/// array insertion order for same-day ties (`journalEntries` is pushed in posting order, and a
/// journal's `lines[]` keeps the caller's order). `party_ledger_lines` sorts by
/// `(date_key, entry_created_at, entry_id, position)` before returning, so every caller (balance
/// sums, statements) sees lines in the exact order the mock's array would produce.
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
    entry_created_at: chrono::DateTime<chrono::Utc>,
    entry_id: Id,
    position: i16,
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
            entry_created_at: entry.created_at,
            entry_id: entry.id,
            position: line.position,
        });
    }
    // G-5: stable order — date_key, then the mock's array-insertion order for same-day ties
    // (entry creation order, then line position within the entry).
    out.sort_by(|a, b| (&a.date_key, a.entry_created_at, a.entry_id, a.position).cmp(&(&b.date_key, b.entry_created_at, b.entry_id, b.position)));
    Ok(out)
}

/// Batch variant of the party-lookups below (G-2): one query for every party's ledger lines on
/// the given control account, grouped by `party_id` — turns an N-party list screen's balance
/// column from `2N+` queries into 3 total (this call + the two batch balance readers below use
/// it once each). Same ordering guarantee as `party_ledger_lines`.
async fn party_ledger_lines_batch<C: ConnectionTrait>(
    conn: &C,
    kind: PartyKind,
    party_ids: &[Id],
    account_id: Id,
) -> Result<HashMap<Id, Vec<PartyLine>>, AppError> {
    let mut out: HashMap<Id, Vec<PartyLine>> = HashMap::new();
    if party_ids.is_empty() {
        return Ok(out);
    }

    let rows = journal_lines::Entity::find()
        .filter(journal_lines::Column::AccountId.eq(account_id))
        .filter(journal_lines::Column::PartyKind.eq(kind))
        .filter(journal_lines::Column::PartyId.is_in(party_ids.iter().copied()))
        .find_also_related(journal_entries::Entity)
        .all(conn)
        .await
        .map_err(AppError::from)?;

    for (line, entry) in rows {
        let Some(entry) = entry else { continue };
        let Some(party_id) = line.party_id else { continue };
        out.entry(party_id).or_default().push(PartyLine {
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
            entry_created_at: entry.created_at,
            entry_id: entry.id,
            position: line.position,
        });
    }

    for lines in out.values_mut() {
        lines.sort_by(|a, b| (&a.date_key, a.entry_created_at, a.entry_id, a.position).cmp(&(&b.date_key, b.entry_created_at, b.entry_id, b.position)));
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

/// G-2: batch `customer_balance` — one query for every id in `customer_ids`, same
/// `round2(Σ(debit − credit))` per party as the single-party function. A party with no matching
/// ledger lines is simply absent from the map (callers `.get(id).copied().unwrap_or(Decimal::ZERO)`).
pub async fn customer_balances<C: ConnectionTrait>(conn: &C, customer_ids: &[Id]) -> Result<HashMap<Id, Decimal>, AppError> {
    // No parties → no account lookup (like `unallocated_credits`): the mock resolves the control
    // account per row (`accountFor` inside `customerBalance`), so an empty list never needs one and a
    // chart without that role still lists `[]` (Part 04 Wave 2, L1 `import/import-edge` books).
    if customer_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let account_id = receivable_account_id(conn).await?;
    let by_party = party_ledger_lines_batch(conn, PartyKind::Customer, customer_ids, account_id).await?;
    Ok(by_party
        .into_iter()
        .map(|(id, lines)| (id, round2(lines.iter().fold(Decimal::ZERO, |a, l| a + (l.debit - l.credit)))))
        .collect())
}

/// G-2: batch `supplier_balance` — see `customer_balances`.
pub async fn supplier_balances<C: ConnectionTrait>(conn: &C, supplier_ids: &[Id]) -> Result<HashMap<Id, Decimal>, AppError> {
    // No parties → no account lookup (like `unallocated_credits`): the mock resolves the control
    // account per row (`accountFor` inside `supplierBalance`), so an empty list never needs one and a
    // chart without that role still lists `[]` (Part 04 Wave 2, L1 `import/import-edge` books).
    if supplier_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let account_id = payable_account_id(conn).await?;
    let by_party = party_ledger_lines_batch(conn, PartyKind::Supplier, supplier_ids, account_id).await?;
    Ok(by_party
        .into_iter()
        .map(|(id, lines)| (id, round2(lines.iter().fold(Decimal::ZERO, |a, l| a + (l.credit - l.debit)))))
        .collect())
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

/// `refundCreditedBase` (`src/mocks/backend/sales.ts`, ACC-0010) summed per customer: the base-
/// currency customer credit every `customer_credit` refund left on the party — exactly what
/// `create_refund` credited to the receivable for it (an FC invoice's credit at the invoice's own rate,
/// telescoped over the invoice's refunds in creation order). A customer with none is absent.
async fn refund_credits<C: ConnectionTrait>(conn: &C, customer_ids: &[Id]) -> Result<HashMap<Id, Decimal>, AppError> {
    use crate::entities::sales::refunds;
    let mut out: HashMap<Id, Decimal> = HashMap::new();
    if customer_ids.is_empty() {
        return Ok(out);
    }
    let credited = refunds::Entity::find().filter(refunds::Column::CreditedToAccount.gt(Decimal::ZERO)).all(conn).await.map_err(AppError::from)?;
    if credited.is_empty() {
        return Ok(out);
    }
    let invoice_ids: Vec<Id> = credited.iter().map(|r| r.invoice_id).collect();
    let invs: HashMap<Id, invoices::Model> = invoices::Entity::find()
        .filter(invoices::Column::Id.is_in(invoice_ids))
        .filter(invoices::Column::CustomerId.is_in(customer_ids.iter().copied()))
        .all(conn)
        .await
        .map_err(AppError::from)?
        .into_iter()
        .map(|i| (i.id, i))
        .collect();
    for r in &credited {
        let Some(inv) = invs.get(&r.invoice_id) else { continue };
        let Some(customer_id) = inv.customer_id else { continue };
        let credit = r.credited_to_account.unwrap_or(Decimal::ZERO);
        let base = match (&inv.currency, inv.exchange_rate) {
            (Some(_), Some(rate)) => {
                let all = refunds::Entity::find()
                    .filter(refunds::Column::InvoiceId.eq(inv.id))
                    .order_by_asc(refunds::Column::CreatedAt)
                    .order_by_asc(refunds::Column::Number)
                    .all(conn)
                    .await
                    .map_err(AppError::from)?;
                let before = round2(all.iter().take_while(|x| x.id != r.id).fold(Decimal::ZERO, |a, x| a + x.grand_total));
                let total_base = round2(crate::shared::currency::to_base(round2(before + r.grand_total), rate) - crate::shared::currency::to_base(before, rate));
                crate::shared::currency::refund_cash_back_base(r.settled_to_receivable, r.cash_back, total_base, rate)
            }
            _ => credit,
        };
        *out.entry(customer_id).or_insert(Decimal::ZERO) += base;
    }
    Ok(out)
}

/// `unallocatedCreditFor` (`payments.ts`): Σ unallocated credit across every RECEIVED/PAID payment
/// for a party, plus (customers, ACC-0010) every refund kept as customer credit (`refund_credits`) —
/// docs/v2/02-accounting-review.md D4: "keep as customer credit (an unallocated credit on the party)".
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
    if matches!(target_type, PartyKind::Customer) {
        total += refund_credits(conn, &[target_id]).await?.get(&target_id).copied().unwrap_or(Decimal::ZERO);
    }
    Ok(round2(total))
}

/// G-2: batch `unallocated_credit_for` — one query for every RECEIVED/PAID payment across all of
/// `target_ids` (still one `payment_allocations` query per payment inside `unallocated_amount`,
/// since that math is per-payment and cheap; the win here is the single `payments` query instead
/// of one per party). A party with no matching payments is absent from the map.
pub async fn unallocated_credits<C: ConnectionTrait>(conn: &C, target_type: PartyKind, target_ids: &[Id]) -> Result<HashMap<Id, Decimal>, AppError> {
    use crate::entities::payments::payments::{PaymentTargetType, PaymentType};
    let mut out: HashMap<Id, Decimal> = HashMap::new();
    if target_ids.is_empty() {
        return Ok(out);
    }

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
        .filter(payments::Column::TargetId.is_in(target_ids.iter().copied()))
        .filter(payments::Column::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(AppError::from)?;

    for p in &rows {
        let amount = unallocated_amount(conn, p).await?;
        *out.entry(p.target_id).or_insert(Decimal::ZERO) += amount;
    }
    if matches!(target_type, PartyKind::Customer) {
        for (id, credit) in refund_credits(conn, target_ids).await? {
            let v = out.entry(id).or_insert(Decimal::ZERO);
            *v = round2(*v + credit);
        }
    }
    Ok(out)
}

/// `OpenDocument.kind` (`payments.ts:42,63`) — which document table the row came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenDocumentKind {
    Invoice,
    PurchaseOrder,
}

/// An open document a payment can be allocated against (`OpenDocument`, `payments.ts:29-70` /
/// `modules/payments/types/index.ts:100-116`) — full field set (G-1/G-24: the mock's shape, not
/// just `id`/`outstanding`), so payments/parties DTO assembly and the aging report can build
/// their own `OpenDocument`/aging-bucket DTOs straight off this without a second query. `total`
/// and `outstanding` are always BASE currency (FC documents convert via their own `rate`);
/// `fc_outstanding` carries the raw FC figure when the document has a currency, matching the
/// mock's `i.currency ? fcOutstanding : undefined`.
pub struct OpenDocument {
    pub id: Id,
    pub kind: OpenDocumentKind,
    pub number: String,
    pub date: DocDate,
    /// Invoices only (`payments.ts:46`) — a purchase order's `OpenDocument` never sets this.
    pub due_date: Option<chrono::NaiveDate>,
    /// The same due date as the DTO key (`DocDate::key` — the stored ISO instant when the invoice
    /// has one, else `YYYY-MM-DD`), exactly the `dueDate` string the mock's `OpenDocument` carries
    /// and `Invoice.dueDate` already renders.
    pub due_date_key: Option<String>,
    pub total: Decimal,
    pub outstanding: Decimal,
    pub currency: Option<String>,
    pub fc_outstanding: Option<Decimal>,
    pub rate: Option<Decimal>,
}

/// `getOpenDocumentsFor` (customer side): completed invoices with `invoiceOutstanding > 0`. Order
/// matches the mock's `sort((a, b) => a.date.localeCompare(b.date))` — the full date-string sort,
/// not `date_day` alone (a same-day ISO instant would tie-break by string, which `(date_day,
/// date_instant, created_at, id)` also gets right since `date_instant` sorts NULLs first exactly
/// like `undefined` never being less than a set string never applies here — same-day invoices
/// without an instant fall back to `created_at`/`id`, matching insertion order like every other
/// stable-sort port in this module).
pub async fn open_invoices_for<C: ConnectionTrait>(conn: &C, customer_id: Id) -> Result<Vec<OpenDocument>, AppError> {
    let rows = invoices::Entity::find()
        .filter(invoices::Column::CustomerId.eq(customer_id))
        .filter(invoices::Column::Status.eq(invoices::InvoiceStatus::Completed))
        .filter(invoices::Column::DeletedAt.is_null())
        .order_by_asc(invoices::Column::DateDay)
        .order_by_asc(invoices::Column::DateInstant)
        .order_by_asc(invoices::Column::CreatedAt)
        .order_by_asc(invoices::Column::Id)
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
            let converted = match (i.currency.as_ref(), i.exchange_rate) {
                (Some(_), Some(rate)) => Some(rate),
                _ => None,
            };
            let outstanding = converted.map(|rate| round2(fc_outstanding * rate)).unwrap_or(fc_outstanding);
            let total = converted
                .map(|rate| round2((i.grand_total - i.refunded_amount) * rate))
                .unwrap_or_else(|| round2(i.grand_total - i.refunded_amount));
            Some(OpenDocument {
                id: i.id,
                kind: OpenDocumentKind::Invoice,
                number: i.number.clone(),
                date: i.date(),
                due_date: i.due_date_day,
                due_date_key: i.due_date().map(|d| d.key()),
                total,
                outstanding,
                currency: i.currency.clone(),
                fc_outstanding: i.currency.is_some().then_some(fc_outstanding),
                rate: i.exchange_rate,
            })
        })
        .collect())
}

/// `getOpenDocumentsFor` (supplier side): received purchase orders with `purchaseOutstanding > 0`.
/// Same ordering rule as `open_invoices_for`.
pub async fn open_purchase_orders_for<C: ConnectionTrait>(conn: &C, supplier_id: Id) -> Result<Vec<OpenDocument>, AppError> {
    let rows = purchase_orders::Entity::find()
        .filter(purchase_orders::Column::SupplierId.eq(supplier_id))
        .filter(purchase_orders::Column::Status.eq(purchase_orders::PurchaseStatus::Received))
        .filter(purchase_orders::Column::DeletedAt.is_null())
        .order_by_asc(purchase_orders::Column::DateDay)
        .order_by_asc(purchase_orders::Column::DateInstant)
        .order_by_asc(purchase_orders::Column::CreatedAt)
        .order_by_asc(purchase_orders::Column::Id)
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
            let converted = match (p.currency.as_ref(), p.exchange_rate) {
                (Some(_), Some(rate)) => Some(rate),
                _ => None,
            };
            let outstanding = converted.map(|rate| round2(fc_outstanding * rate)).unwrap_or(fc_outstanding);
            let total = converted
                .map(|rate| round2((p.grand_total - p.returned_amount) * rate))
                .unwrap_or_else(|| round2(p.grand_total - p.returned_amount));
            Some(OpenDocument {
                id: p.id,
                kind: OpenDocumentKind::PurchaseOrder,
                number: p.number.clone(),
                date: p.date(),
                due_date: None,
                due_date_key: None,
                total,
                outstanding,
                currency: p.currency.clone(),
                fc_outstanding: p.currency.is_some().then_some(fc_outstanding),
                rate: p.exchange_rate,
            })
        })
        .collect())
}
