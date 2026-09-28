//! Invariants 7–9 plus the two un-numbered ones (`invariants.ts:242-364`): source-ref integrity,
//! lock date, opening balance equity, drafts isolation, allocations ≤ document total.

use std::collections::{BTreeMap, BTreeSet};

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::core::error::AppError;
use crate::entities::journal::{journal_drafts, journal_entries, journal_lines};
use crate::entities::payments::{payment_allocations, payments};
use crate::entities::purchases::purchase_orders;
use crate::entities::sales::invoices;
use crate::shared::ledger::accounts::{resolve_account, AccountCtx, SystemRole};
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::{numeric_check, InvariantResult};

const SYSTEM_SOURCE_KINDS: &[&str] = &[
    "invoice",
    "refund",
    "purchaseOrder",
    "purchaseReturn",
    "payment",
    "stockAdjustment",
    "shift",
    "expense",
    "voucher",
    "settlement",
    "fxReval",
    "opening",
];

/// 7. Every posted document has exactly one active entry (or an entry + reversal pair); every
/// `sourceRef` resolves. Three results (`invariants.ts:245-283`).
pub async fn check_source_ref_integrity<C: ConnectionTrait>(conn: &C) -> Result<Vec<InvariantResult>, AppError> {
    let entries = journal_entries::Entity::find().all(conn).await.map_err(AppError::from)?;

    let bad_source_refs: Vec<&journal_entries::Model> = entries
        .iter()
        .filter(|e| e.source_kind.as_deref().is_some_and(|k| !SYSTEM_SOURCE_KINDS.contains(&k)))
        .collect();

    let mut by_source: BTreeMap<(String, Id), Vec<&journal_entries::Model>> = BTreeMap::new();
    for e in &entries {
        if let (Some(kind), Some(id)) = (&e.source_kind, e.source_id) {
            by_source.entry((kind.clone(), id)).or_default().push(e);
        }
    }
    let multi_active = by_source.values().filter(|group| group.iter().filter(|e| !e.reversed).count() > 1).count();

    let entry_ids: BTreeSet<Id> = entries.iter().map(|e| e.id).collect();
    let reversals_resolve: Vec<&journal_entries::Model> =
        entries.iter().filter(|e| e.reversal_of_id.is_some_and(|id| !entry_ids.contains(&id))).collect();

    Ok(vec![
        InvariantResult {
            key: "source-ref-known".to_string(),
            doc: "§4.7".to_string(),
            passed: bad_source_refs.is_empty(),
            message: format!("every entry's sourceRef has a known kind ({} unrecognized)", bad_source_refs.len()),
            diff: None,
        },
        InvariantResult {
            key: "one-active-entry".to_string(),
            doc: "§4.7".to_string(),
            passed: multi_active == 0,
            message: format!("every SYSTEM document has exactly one active entry ({multi_active} with >1 active)"),
            diff: None,
        },
        InvariantResult {
            key: "reversal-resolves".to_string(),
            doc: "§4.7".to_string(),
            passed: reversals_resolve.is_empty(),
            message: format!("every reversal's original entry resolves ({} dangling)", reversals_resolve.len()),
            diff: None,
        },
    ])
}

/// 8. No entry is dated inside a locked period unless posted before the lock. Uses `LEFT(date_key,
/// 10)` semantics — the `DocDate::key()`'s first 10 chars, same as the mock's `date.slice(0, 10)`
/// on its ISO/UTC string (`invariants.ts:286-294`).
pub async fn check_lock_date<C: ConnectionTrait>(conn: &C) -> Result<InvariantResult, AppError> {
    let settings = crate::core::settings::load(conn).await?;
    let lock_date = settings.accounting.as_ref().and_then(|a| a.lock_date);

    let locked_but_posted = match lock_date {
        Some(lock_date) => {
            let lock_date_key = lock_date.format("%Y-%m-%d").to_string();
            let entries = journal_entries::Entity::find().all(conn).await.map_err(AppError::from)?;
            entries.iter().filter(|e| &e.date().key()[..10] <= lock_date_key.as_str()).count()
        }
        None => 0,
    };

    let lock_date_str = lock_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "none".to_string());
    Ok(InvariantResult {
        key: "lock-date".to_string(),
        doc: "§4.8".to_string(),
        passed: locked_but_posted == 0,
        message: format!("no entry dated inside the locked period (lockDate={lock_date_str}, {locked_but_posted} offenders)"),
        diff: None,
    })
}

/// 9. `openingBalanceEquity` (3900) nets to zero once onboarding is complete (`invariants.ts:297-309`).
pub async fn check_opening_balance_equity<C: ConnectionTrait>(conn: &C) -> Result<InvariantResult, AppError> {
    let account = resolve_account(conn, SystemRole::OpeningBalanceEquity, &AccountCtx::default()).await;
    let obe_net = match account {
        Ok(account) => {
            let lines = journal_lines::Entity::find()
                .filter(journal_lines::Column::AccountId.eq(account.id))
                .all(conn)
                .await
                .map_err(AppError::from)?;
            lines.iter().fold(Decimal::ZERO, |a, l| a + (l.debit - l.credit))
        }
        // The mock's `db.accounts.find(...)` returns `undefined` (no throw) when the role has no
        // account — `obeNet` simply stays `0` in that case, unlike `resolve_account`'s `NOT_FOUND`
        // (used everywhere else in this port, since posting always needs the account to exist).
        Err(_) => Decimal::ZERO,
    };
    Ok(numeric_check("opening-balance-equity", "§4.9", "openingBalanceEquity (3900) is zero", Decimal::ZERO, round2(obe_net), Decimal::new(1, 2)))
}

/// Drafts never affect balances: no `journal_drafts.id` leaked into `journal_entries`
/// (`invariants.ts:335-344`) — by construction the two are separate tables (P2-14), so this checks
/// nothing has copied a draft's id into the posted ledger.
pub async fn check_drafts_isolated<C: ConnectionTrait>(conn: &C) -> Result<InvariantResult, AppError> {
    let draft_ids: BTreeSet<Id> = journal_drafts::Entity::find().all(conn).await.map_err(AppError::from)?.into_iter().map(|d| d.id).collect();
    let leaked = if draft_ids.is_empty() {
        0
    } else {
        journal_entries::Entity::find().filter(journal_entries::Column::Id.is_in(draft_ids)).all(conn).await.map_err(AppError::from)?.len()
    };
    Ok(InvariantResult {
        key: "drafts-isolated".to_string(),
        doc: "§4 (drafts never affect balances)".to_string(),
        passed: leaked == 0,
        message: if leaked == 0 {
            "no draft ids leaked into posted entries".to_string()
        } else {
            format!("{leaked} draft ids found in posted entries")
        },
        diff: None,
    })
}

/// Allocations ≤ document total: every payment allocation never exceeds the target document's
/// grand total (`invariants.ts:347-364`).
pub async fn check_allocations_within_total<C: ConnectionTrait>(conn: &C) -> Result<InvariantResult, AppError> {
    let payments_rows = payments::Entity::find().all(conn).await.map_err(AppError::from)?;
    let mut offenders = Vec::new();
    for p in &payments_rows {
        let allocations = payment_allocations::Entity::find()
            .filter(payment_allocations::Column::PaymentId.eq(p.id))
            .all(conn)
            .await
            .map_err(AppError::from)?;
        for a in &allocations {
            let total = match a.target_kind {
                payment_allocations::PaymentAllocationTargetKind::Invoice => {
                    invoices::Entity::find_by_id(a.target_id).one(conn).await.map_err(AppError::from)?.map(|i| i.grand_total)
                }
                payment_allocations::PaymentAllocationTargetKind::PurchaseOrder => {
                    purchase_orders::Entity::find_by_id(a.target_id).one(conn).await.map_err(AppError::from)?.map(|o| o.grand_total)
                }
                payment_allocations::PaymentAllocationTargetKind::Opening => None,
            };
            if let Some(total) = total {
                if a.amount > total + Decimal::new(1, 2) {
                    offenders.push(format!("{}->{} ({} > {})", p.id, a.target_id, a.amount, total));
                }
            }
        }
    }
    Ok(InvariantResult {
        key: "allocations-within-total".to_string(),
        doc: "§4 (allocations ≤ document total)".to_string(),
        passed: offenders.is_empty(),
        message: if offenders.is_empty() {
            "every allocation ≤ its document total".to_string()
        } else {
            format!("{} over-allocated: {}", offenders.len(), offenders.join(", "))
        },
        diff: None,
    })
}
