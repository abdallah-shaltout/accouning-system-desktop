//! `domains::accounting::service` — one file per area (12-accounting.md §3, 12b-period-close.md §3).

pub mod accounts;
pub mod journal;
pub mod journal_reads;
pub mod period;
pub mod rows;
pub mod templates;
pub mod vat;

use crate::core::auth::Role;
use crate::core::tx::TxCtx;

/// `is_admin(cx)` — replaces `canPostToClosedPeriod()` (`accountingService.ts:35-37`); every write
/// path that needs `allow_closed_period` calls this instead of re-deriving the role check.
pub fn is_admin(cx: &TxCtx) -> bool {
    cx.actor.as_ref().map(|a| a.role == Role::Admin).unwrap_or(false)
}
