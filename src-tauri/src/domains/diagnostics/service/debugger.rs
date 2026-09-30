//! Slice B — the 7 debug-build-only accounting-debugger reads (plan 21 Part 03 §16 spec §3,
//! `accountingDebugService.ts`). Powers `/dev/diagnostics` → المحاسبة: document picker, posting
//! trace, raw journal entry, before/after balances, invariants, subledger-vs-GL drift, and "اشرح هذا
//! الرقم". Every function here is read-only and, other than [`get_posting_trace`] (which reads the
//! in-memory ring, not the DB), runs inside the caller's `with_read` snapshot. Access gating
//! (`debug build + Accounting:Read`) is the command layer's job (`commands.rs`), not this module's —
//! kept there so every one of the 7 commands applies the exact same gate the same way (D-4).

use std::collections::BTreeSet;

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::tx::TxResult;
use crate::entities::catalog::products::Entity as ProductEntity;
use crate::entities::journal::journal_drafts::Entity as DraftEntity;
use crate::entities::journal::journal_entries::{Entity as EntryEntity, Model as EntryModel};
use crate::entities::journal::journal_lines::{Column as LineColumn, Entity as LineEntity, PartyKind as LinePartyKind};
use crate::entities::org::accounts::{Column as AccountColumn, Entity as AccountEntity};
use crate::shared::balances::{customer_balance, supplier_balance, unallocated_credit_for};
use crate::shared::ledger::accounts::SystemRole;
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::super::dto::{AccountingDocSummary, BalanceAround, DriftRow, DriftRowKind, ExplainLine};

/// `listRecentDocuments` (`accountingDebugService.ts:38-55`): posted entries only (the mock's
/// `db.journalEntries`, which never held drafts), newest first by `(date, number)` string compare,
/// capped at `limit`. `hasTrace` = the process-local trace ring still has this entry's trace
/// (`state.traces.for_entry`).
pub async fn list_recent_documents<C: ConnectionTrait>(
    conn: &C,
    traces: &crate::shared::ledger::trace::TraceRing,
    limit: u32,
) -> TxResult<Vec<AccountingDocSummary>> {
    let mut entries = EntryEntity::find().all(conn).await.map_err(crate::core::tx::TxError::from)?;
    entries.sort_by(|a, b| {
        let a_date = a.date().key();
        let b_date = b.date().key();
        b_date.cmp(&a_date).then_with(|| b.number.cmp(&a.number))
    });
    let limit = limit as usize;
    Ok(entries
        .into_iter()
        .take(limit)
        .map(|e| {
            let date = e.date().key();
            let kind = journal_type_str(&e).to_string();
            AccountingDocSummary {
                has_trace: traces.for_entry(e.id).is_some(),
                id: e.id,
                number: e.number,
                date,
                description: e.description,
                kind,
                total_debit: e.total_debit,
                total_credit: e.total_credit,
                source_kind: e.source_kind,
            }
        })
        .collect())
}

fn journal_type_str(e: &EntryModel) -> &'static str {
    use crate::entities::journal::journal_entries::JournalEntryType as T;
    match e.r#type {
        T::System => "SYSTEM",
        T::Manual => "MANUAL",
        T::Opening => "OPENING",
        T::Closing => "CLOSING",
        T::VatSettlement => "VAT_SETTLEMENT",
    }
}

/// `getPostingTrace` (`:61-64`): reads only the in-memory ring, no DB access — works even if the
/// entry has since been deleted, and returns `None` once the ring has rotated past it (the mock's
/// own documented limitation, kept — spec Q-4).
pub fn get_posting_trace(traces: &crate::shared::ledger::trace::TraceRing, entry_id: Id) -> Option<crate::shared::ledger::trace::PostingTrace> {
    traces.for_entry(entry_id)
}

/// `getJournalEntryRaw` (`:66-70`): a posted entry if one exists with this id, else a draft, else
/// `None` — never `NOT_FOUND` (spec Q-4).
pub async fn get_journal_entry_raw<C: ConnectionTrait>(conn: &C, id: Id) -> TxResult<Option<crate::domains::accounting::dto::JournalEntry>> {
    if let Some(m) = EntryEntity::find_by_id(id).one(conn).await.map_err(crate::core::tx::TxError::from)? {
        return Ok(Some(crate::domains::accounting::service::rows::entry_dto(conn, &m).await?));
    }
    if let Some(m) = DraftEntity::find_by_id(id).one(conn).await.map_err(crate::core::tx::TxError::from)? {
        let base_currency = crate::core::settings::load(conn).await.map_err(crate::core::tx::TxError::App)?.currency;
        return Ok(Some(crate::domains::accounting::service::rows::draft_dto(conn, &m, &base_currency).await?));
    }
    Ok(None)
}

/// `getBalancesAround` (`:76-99`): every account touched by `entryId`'s lines, with its running
/// `round2(Σ debit − credit)` balance immediately before and immediately including this entry.
/// Same-day entries are ordered by `number` (string compare) so the split is deterministic, exactly
/// as the mock's `e.date < entry.date || (e.date === entry.date && e.number < entry.number)`.
/// Unknown `entryId` returns `[]`, not `NOT_FOUND` (spec Q-4).
pub async fn get_balances_around<C: ConnectionTrait>(conn: &C, entry_id: Id) -> TxResult<Vec<BalanceAround>> {
    let Some(entry) = EntryEntity::find_by_id(entry_id).one(conn).await.map_err(crate::core::tx::TxError::from)? else {
        return Ok(Vec::new());
    };
    let entry_date = entry.date().key();
    let entry_lines = LineEntity::find().filter(LineColumn::JournalEntryId.eq(entry.id)).all(conn).await.map_err(crate::core::tx::TxError::from)?;

    // First-seen order of the entry's own lines (spec §3), not a sorted set.
    let mut account_ids: Vec<Id> = Vec::new();
    let mut seen: BTreeSet<Id> = BTreeSet::new();
    for l in &entry_lines {
        if seen.insert(l.account_id) {
            account_ids.push(l.account_id);
        }
    }
    if account_ids.is_empty() {
        return Ok(Vec::new());
    }

    let all_entries = EntryEntity::find().all(conn).await.map_err(crate::core::tx::TxError::from)?;
    let mut prior_or_same_ids: Vec<Id> = Vec::new();
    let mut before_ids: Vec<Id> = Vec::new();
    for e in &all_entries {
        let d = e.date().key();
        if d < entry_date || (d == entry_date && e.number <= entry.number) {
            prior_or_same_ids.push(e.id);
        }
        if d < entry_date || (d == entry_date && e.number < entry.number) {
            before_ids.push(e.id);
        }
    }

    let accounts = AccountEntity::find().filter(AccountColumn::Id.is_in(account_ids.clone())).all(conn).await.map_err(crate::core::tx::TxError::from)?;
    let account_by_id: std::collections::HashMap<Id, &crate::entities::org::accounts::Model> = accounts.iter().map(|a| (a.id, a)).collect();

    let mut out = Vec::with_capacity(account_ids.len());
    for account_id in account_ids {
        let before = balance_for_entries(conn, account_id, &before_ids).await?;
        let after = balance_for_entries(conn, account_id, &prior_or_same_ids).await?;
        let account = account_by_id.get(&account_id);
        out.push(BalanceAround {
            account_id,
            account_name: account.map(|a| a.name.clone()).unwrap_or_else(|| account_id.to_string()),
            account_code: account.map(|a| a.code.clone()),
            before,
            after,
        });
    }
    Ok(out)
}

async fn balance_for_entries<C: ConnectionTrait>(conn: &C, account_id: Id, entry_ids: &[Id]) -> TxResult<Decimal> {
    if entry_ids.is_empty() {
        return Ok(Decimal::ZERO);
    }
    let lines = LineEntity::find()
        .filter(LineColumn::AccountId.eq(account_id))
        .filter(LineColumn::JournalEntryId.is_in(entry_ids.to_vec()))
        .all(conn)
        .await
        .map_err(crate::core::tx::TxError::from)?;
    Ok(round2(lines.iter().fold(Decimal::ZERO, |a, l| a + (l.debit - l.credit))))
}

/// `getInvariantResults` (`:101-104`): `shared::invariants::run_all` — the one copy of the 14
/// accounting invariants (CLAUDE.md "Accounting safety"). Never re-implemented here.
pub async fn get_invariant_results<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<crate::domains::diagnostics::dto::InvariantResultDto>> {
    let results = crate::shared::invariants::run_all(conn).await.map_err(crate::core::tx::TxError::App)?;
    Ok(results.iter().map(Into::into).collect())
}

/// `getDriftReport` (`:124-201`) — display-only subledger-vs-GL drift per customer/supplier/product.
/// Pass/fail is never decided here (spec §3): a row is emitted whenever `|diff| > 0.01`, mirroring
/// `checkPartyAllocation`'s own tolerance, purely so the debugger can show *which* document first
/// diverged.
pub async fn get_drift_report<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<DriftRow>> {
    use crate::entities::parties::parties;
    use crate::entities::purchases::{purchase_orders, purchase_returns};
    use crate::entities::sales::{invoices, refunds};

    let mut rows = Vec::new();

    let customers = parties::Entity::find().filter(parties::Column::Kind.eq("customer")).all(conn).await.map_err(crate::core::tx::TxError::from)?;
    let suppliers = parties::Entity::find().filter(parties::Column::Kind.eq("supplier")).all(conn).await.map_err(crate::core::tx::TxError::from)?;
    let all_invoices = invoices::Entity::find().filter(invoices::Column::DeletedAt.is_null()).all(conn).await.map_err(crate::core::tx::TxError::from)?;
    let all_refunds = refunds::Entity::find().all(conn).await.map_err(crate::core::tx::TxError::from)?;
    let all_pos = purchase_orders::Entity::find()
        .filter(purchase_orders::Column::Status.eq(purchase_orders::PurchaseStatus::Received))
        .all(conn)
        .await
        .map_err(crate::core::tx::TxError::from)?;
    let all_returns = purchase_returns::Entity::find().all(conn).await.map_err(crate::core::tx::TxError::from)?;

    let all_entries = EntryEntity::find().all(conn).await.map_err(crate::core::tx::TxError::from)?;
    let mut sorted_entries: Vec<&EntryModel> = all_entries.iter().collect();
    sorted_entries.sort_by(|a, b| a.date().key().cmp(&b.date().key()).then_with(|| a.number.cmp(&b.number)));
    let mut lines_by_entry: std::collections::HashMap<Id, Vec<crate::entities::journal::journal_lines::Model>> = std::collections::HashMap::new();
    let all_lines = LineEntity::find().all(conn).await.map_err(crate::core::tx::TxError::from)?;
    for l in all_lines {
        lines_by_entry.entry(l.journal_entry_id).or_default().push(l);
    }

    let account_ctx = crate::shared::ledger::accounts::AccountCtx::default();
    let receivable_id = crate::shared::ledger::accounts::resolve_account(conn, SystemRole::Receivable, &account_ctx)
        .await
        .map_err(crate::core::tx::TxError::App)?
        .id;
    let payable_id = crate::shared::ledger::accounts::resolve_account(conn, SystemRole::Payable, &account_ctx)
        .await
        .map_err(crate::core::tx::TxError::App)?
        .id;
    let accounts = AccountEntity::find().all(conn).await.map_err(crate::core::tx::TxError::from)?;
    let inventory_ids: BTreeSet<Id> =
        accounts.iter().filter(|a| a.system_role.as_deref() == Some(SystemRole::Inventory.as_str())).map(|a| a.id).collect();

    /// `firstDivergingEntry` (`:138-147`): walks chronologically, tracking a running GL total for
    /// this party's control-account lines; the first entry after which running stops matching the
    /// already-final subledger total is reported.
    fn first_diverging_entry(
        sorted_entries: &[&EntryModel],
        lines_by_entry: &std::collections::HashMap<Id, Vec<crate::entities::journal::journal_lines::Model>>,
        account_id: Id,
        party_kind: LinePartyKind,
        party_id: Id,
        subledger_total: Decimal,
        sign: Decimal,
    ) -> (Option<Id>, Option<String>) {
        let mut running = Decimal::ZERO;
        for e in sorted_entries {
            let Some(lines) = lines_by_entry.get(&e.id) else { continue };
            let delta = sign
                * lines
                    .iter()
                    .filter(|l| l.account_id == account_id && l.party_kind == Some(party_kind) && l.party_id == Some(party_id))
                    .fold(Decimal::ZERO, |a, l| a + (l.debit - l.credit));
            if delta == Decimal::ZERO {
                continue;
            }
            running = round2(running + delta);
            if (running - subledger_total).abs() > crate::shared::invariants::tolerance_cents() {
                return (Some(e.id), Some(e.number.clone()));
            }
        }
        (None, None)
    }

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
        let credit = unallocated_credit_for(conn, LinePartyKind::Customer, c.id).await.map_err(crate::core::tx::TxError::App)?;
        let subledger = round2(outstanding - credit);
        let gl = customer_balance(conn, c.id).await.map_err(crate::core::tx::TxError::App)?;
        let diff = round2(subledger - gl);
        if diff.abs() > crate::shared::invariants::tolerance_cents() {
            let (first_id, first_number) =
                first_diverging_entry(&sorted_entries, &lines_by_entry, receivable_id, LinePartyKind::Customer, c.id, gl, Decimal::ONE);
            rows.push(DriftRow {
                kind: DriftRowKind::Customer,
                id: c.id,
                label: c.name.clone(),
                subledger,
                gl,
                diff,
                first_diverging_doc_id: first_id,
                first_diverging_doc_number: first_number,
            });
        }
    }

    for s in &suppliers {
        let outstanding = round2(all_pos.iter().filter(|p| p.supplier_id == s.id).fold(Decimal::ZERO, |a, p| {
            let reduced_by_returns: Decimal = all_returns.iter().filter(|r| r.purchase_order_id == p.id).fold(Decimal::ZERO, |b, r| {
                b + if r.refund_method == purchase_returns::RefundMethod::Credit { r.grand_total } else { r.settled_to_payable }
            });
            a + (p.grand_total - reduced_by_returns - p.paid_amount)
        }));
        let credit = unallocated_credit_for(conn, LinePartyKind::Supplier, s.id).await.map_err(crate::core::tx::TxError::App)?;
        let subledger = round2(outstanding - credit);
        let gl = supplier_balance(conn, s.id).await.map_err(crate::core::tx::TxError::App)?;
        let diff = round2(subledger - gl);
        if diff.abs() > crate::shared::invariants::tolerance_cents() {
            let (first_id, first_number) =
                first_diverging_entry(&sorted_entries, &lines_by_entry, payable_id, LinePartyKind::Supplier, s.id, -gl, -Decimal::ONE);
            rows.push(DriftRow {
                kind: DriftRowKind::Supplier,
                id: s.id,
                label: s.name.clone(),
                subledger,
                gl,
                diff,
                first_diverging_doc_id: first_id,
                first_diverging_doc_number: first_number,
            });
        }
    }

    // Products (spec Q-3, kept quirk): no per-product journal dimension exists, so a per-product row
    // just repeats the aggregate inventory-GL-vs-stock-value diff for every stocked product — only
    // emitted at all when that aggregate is itself off by more than a cent.
    let inv_gl = round2(all_lines_touching(&lines_by_entry, &inventory_ids));
    let products = ProductEntity::find().filter(crate::entities::catalog::products::Column::DeletedAt.is_null()).all(conn).await.map_err(crate::core::tx::TxError::from)?;
    let inv_sum = round2(products.iter().filter(|p| p.r#type == "product").fold(Decimal::ZERO, |a, p| a + p.stock_value));
    if (inv_gl - inv_sum).abs() > crate::shared::invariants::tolerance_cents() {
        for p in &products {
            if p.r#type != "product" {
                continue;
            }
            rows.push(DriftRow {
                kind: DriftRowKind::Product,
                id: p.id,
                label: p.name.clone(),
                subledger: p.stock_value,
                gl: inv_gl,
                diff: round2(inv_sum - inv_gl),
                first_diverging_doc_id: None,
                first_diverging_doc_number: None,
            });
        }
    }

    Ok(rows)
}

fn all_lines_touching(
    lines_by_entry: &std::collections::HashMap<Id, Vec<crate::entities::journal::journal_lines::Model>>,
    account_ids: &BTreeSet<Id>,
) -> Decimal {
    lines_by_entry
        .values()
        .flatten()
        .filter(|l| account_ids.contains(&l.account_id))
        .fold(Decimal::ZERO, |a, l| a + (l.debit - l.credit))
}

/// `explainAccountBalance` (`:215-237`) — "اشرح هذا الرقم": every posted line on `account_id`
/// (optionally narrowed to `party_id`), newest first by `docDate`.
pub async fn explain_account_balance<C: ConnectionTrait>(conn: &C, account_id: Id, party_id: Option<Id>) -> TxResult<Vec<ExplainLine>> {
    let mut query = LineEntity::find().filter(LineColumn::AccountId.eq(account_id));
    if let Some(pid) = party_id {
        query = query.filter(LineColumn::PartyId.eq(pid));
    }
    let lines = query.find_also_related(EntryEntity).all(conn).await.map_err(crate::core::tx::TxError::from)?;

    let mut out: Vec<ExplainLine> = lines
        .into_iter()
        .filter_map(|(l, e)| {
            let e = e?;
            Some(ExplainLine {
                doc_id: e.id,
                doc_number: e.number.clone(),
                doc_date: e.date().key(),
                source_kind: e.source_kind.clone(),
                description: l.description.clone().or_else(|| Some(e.description.clone())),
                debit: l.debit,
                credit: l.credit,
            })
        })
        .collect();
    out.sort_by(|a, b| b.doc_date.cmp(&a.doc_date));
    Ok(out)
}
