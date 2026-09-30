//! Invariants 7–9 plus the two un-numbered ones (`invariants.ts:242-364`): source-ref integrity,
//! lock date, opening balance equity, drafts isolation, allocations ≤ document total.

use std::collections::{BTreeMap, BTreeSet};

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::entities::journal::{journal_drafts, journal_entries, journal_lines};
use crate::entities::payments::{payment_allocations, payments};
use crate::entities::purchases::purchase_orders;
use crate::entities::sales::invoices;
use crate::shared::ledger::accounts::SystemRole;
use crate::utils::id::Id;
use crate::utils::money::round2;

use super::{numeric_check, optional_account, InvariantResult};

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

/// `isAllocationFxEntry` (`invariants.ts`, ACC-0015): a payment's "allocate later" realized-FX entry
/// (09 §3.3, `allocation_fx_lines` in `domains/payments/service/allocate.rs`) moves no money — only
/// the party's control account and `fxGain`/`fxLoss`. It doesn't count toward `one-active-entry`;
/// the payment's own entry (which always hits a cash/bank account) must still be unique.
async fn is_allocation_fx_entry<C: ConnectionTrait>(conn: &C, e: &journal_entries::Model) -> Result<bool, AppError> {
    use crate::entities::org::accounts;
    if e.source_kind.as_deref() != Some("payment") {
        return Ok(false);
    }
    let lines = journal_lines::Entity::find().filter(journal_lines::Column::JournalEntryId.eq(e.id)).all(conn).await.map_err(AppError::from)?;
    let mut has_fx = false;
    for l in &lines {
        let role = accounts::Entity::find_by_id(l.account_id).one(conn).await.map_err(AppError::from)?.and_then(|a| a.system_role);
        match role.as_deref() {
            Some("fxGain") | Some("fxLoss") => has_fx = true,
            Some("receivable") | Some("payable") => {}
            _ => return Ok(false),
        }
    }
    Ok(has_fx)
}

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
    // ACC-0015: a payment's "allocate later" realized-FX entry is a designed second entry, not a
    // second posting — only groups that still look multi-active need their lines inspected.
    let mut multi_active = 0usize;
    for group in by_source.values() {
        let active: Vec<&&journal_entries::Model> = group.iter().filter(|e| !e.reversed).collect();
        if active.len() <= 1 {
            continue;
        }
        let mut counted = 0usize;
        for e in active {
            if !is_allocation_fx_entry(conn, e).await? {
                counted += 1;
            }
        }
        if counted > 1 {
            multi_active += 1;
        }
    }

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

/// The singleton settings row, or `None` before setup has written one — `run_all` never fails on
/// that (ACC-0022); the mock always has a `settings` object.
async fn settings_row<C: ConnectionTrait>(conn: &C) -> Result<Option<crate::entities::org::settings::Model>, AppError> {
    crate::entities::org::settings::Entity::find().one(conn).await.map_err(AppError::from)
}

/// `lockHistory` (`invariants.ts`, ACC-0019): when each lock date took effect. `save_lock_date`
/// (`domains/accounting/service/period.rs`; mock `saveLockDate`) records every change in the
/// activity feed as `تحديد تاريخ القفل {date}` / `إزالة تاريخ القفل` (kind `settings`) — the only
/// record of when a lock was set. Oldest first (`created_at`, then id — insertion order).
async fn lock_history<C: ConnectionTrait>(conn: &C) -> Result<Vec<(chrono::DateTime<chrono::Utc>, Option<String>)>, AppError> {
    use crate::entities::platform::activity;
    const LOCK_SET_PREFIX: &str = "تحديد تاريخ القفل ";
    const LOCK_CLEARED: &str = "إزالة تاريخ القفل";
    let rows = activity::Entity::find()
        .filter(activity::Column::Kind.eq(activity::ActivityKind::Settings))
        .order_by_asc(activity::Column::CreatedAt)
        .order_by_asc(activity::Column::Id)
        .all(conn)
        .await
        .map_err(AppError::from)?;
    let strict_ymd = |s: &str| {
        let b = s.as_bytes();
        b.len() == 10 && b[4] == b'-' && b[7] == b'-' && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    };
    Ok(rows
        .into_iter()
        .filter_map(|a| {
            if a.message == LOCK_CLEARED {
                Some((a.created_at, None))
            } else {
                a.message.strip_prefix(LOCK_SET_PREFIX).filter(|d| strict_ymd(d)).map(|d| (a.created_at, Some(d.to_string())))
            }
        })
        .collect())
}

/// `lockOverrideAllowed` (`invariants.ts`, ACC-0019): postings the period guard lets through on
/// purpose (docs/v2/02 B2): opening and year-close entries and the FX-revaluation auto-reversal
/// always pass `allow_closed_period`; a manual entry (post, reversal, draft post) or a VAT
/// settlement passes it when the user who posted it is an admin (`is_admin(cx)`).
fn lock_override_allowed(e: &journal_entries::Model, admins: &BTreeSet<Id>) -> bool {
    use journal_entries::JournalEntryType as T;
    match e.r#type {
        T::Opening | T::Closing => true,
        _ if e.source_kind.as_deref() == Some("fxReval") => true,
        T::Manual | T::VatSettlement => admins.contains(&e.posted_by.unwrap_or(e.created_by)),
        _ => false,
    }
}

/// 8. No entry is dated inside a locked period unless its creation time is before the lock
/// (ACC-0019, `invariants.ts` `checkLockDate`): an offender is an entry posted while a lock covering
/// its date was already in force — the lock in force at posting time (`posted_at`, else
/// `created_at`) comes from the lock history, not today's lock date, so setting or moving a lock
/// over existing history is never an offence. A lock change recorded at the very same instant as
/// the posting could have come before or after it, so the entry counts only if every possible lock
/// state at that instant covers it. With no recorded history nothing is flagged. The entry's day is
/// `DocDate::key()`'s first 10 chars, same as the mock's `date.slice(0, 10)`.
pub async fn check_lock_date<C: ConnectionTrait>(conn: &C) -> Result<InvariantResult, AppError> {
    let settings = settings_row(conn).await?;
    let lock_date = settings.as_ref().and_then(|s| s.accounting.as_ref()).and_then(|a| a.lock_date);
    let history = lock_history(conn).await?;

    let mut locked_but_posted = 0usize;
    if !history.is_empty() {
        let admins: BTreeSet<Id> = crate::entities::org::users::Entity::find()
            .all(conn)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .filter(|u| u.role == "admin")
            .map(|u| u.id)
            .collect();
        let entries = journal_entries::Entity::find().all(conn).await.map_err(AppError::from)?;
        for e in &entries {
            if lock_override_allowed(e, &admins) {
                continue;
            }
            let at = e.posted_at_instant.unwrap_or(e.created_at);
            let before: Option<String> = history.iter().filter(|(t, _)| *t < at).last().and_then(|(_, d)| d.clone());
            let mut candidates = vec![before];
            candidates.extend(history.iter().filter(|(t, _)| *t == at).map(|(_, d)| d.clone()));
            let key = e.date().key();
            let day = &key[..10];
            if candidates.iter().all(|c| matches!(c, Some(lock) if day <= lock.as_str())) {
                locked_but_posted += 1;
            }
        }
    }

    let lock_date_str = lock_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "none".to_string());
    Ok(InvariantResult {
        key: "lock-date".to_string(),
        doc: "§4.8".to_string(),
        passed: locked_but_posted == 0,
        message: format!("no entry posted into a locked period after the lock (lockDate={lock_date_str}, {locked_but_posted} offenders)"),
        diff: None,
    })
}

/// 9. `openingBalanceEquity` (3900) nets to zero once onboarding is complete (ACC-0023,
/// `invariants.ts` `checkOpeningBalanceEquity`). While the setup wizard is still running
/// (`settings.onboarding` present without `finished_at`) opening entries legitimately sit in 3900
/// until the wizard re-closes it, so the check isn't enforced yet. A company with no onboarding
/// record never ran the wizard (an import) and is treated as complete.
pub async fn check_opening_balance_equity<C: ConnectionTrait>(conn: &C) -> Result<InvariantResult, AppError> {
    let settings = settings_row(conn).await?;
    if settings.as_ref().and_then(|s| s.onboarding.as_ref()).is_some_and(|o| o.finished_at.is_none()) {
        return Ok(InvariantResult {
            key: "opening-balance-equity".to_string(),
            doc: "§4.9".to_string(),
            passed: true,
            message: "onboarding in progress — openingBalanceEquity (3900) not enforced yet".to_string(),
            diff: None,
        });
    }
    // The mock's `db.accounts.find(...)` returns `undefined` (no throw) when the role has no account
    // — `obeNet` simply stays `0` in that case.
    let obe_net = match optional_account(conn, SystemRole::OpeningBalanceEquity).await? {
        Some(account) => {
            let lines = journal_lines::Entity::find()
                .filter(journal_lines::Column::AccountId.eq(account.id))
                .all(conn)
                .await
                .map_err(AppError::from)?;
            lines.iter().fold(Decimal::ZERO, |a, l| a + (l.debit - l.credit))
        }
        None => Decimal::ZERO,
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
/// grand total, in the document's currency (ACC-0018, `invariants.ts` `checkAllocationsWithinTotal`).
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
            // ACC-0018: compared in the document's currency — `grand_total` is in the document's own
            // currency, `amount` is the base amount posted to AR/AP at the document's rate and
            // `amount_fc` is what the allocation settles of the document (`apply_allocation` adds
            // `amount_fc` else `amount` to `paid_amount`).
            let settled = a.amount_fc.unwrap_or(a.amount);
            if let Some(total) = total {
                if settled > total + Decimal::new(1, 2) {
                    offenders.push(format!(
                        "{}->{} ({} > {})",
                        p.id,
                        a.target_id,
                        crate::utils::money::js_number_string(settled),
                        crate::utils::money::js_number_string(total)
                    ));
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
