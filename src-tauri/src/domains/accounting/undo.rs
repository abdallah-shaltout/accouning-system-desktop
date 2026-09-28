//! `domains::accounting::undo` (12-owned, phase-e E-5): three of the four accounting compensators.
//! The fourth (`accounting.closeYear`) is `undo_period.rs` (12b-owned).

use async_trait::async_trait;
use sea_orm::DatabaseTransaction;

use crate::core::auth::Area;
use crate::core::error::{AppError, AppResult};
use crate::core::tx::TxCtx;
use crate::entities::platform::audit;
use crate::shared::activity::undo::{Compensator, UndoRegistry, UndoRequest};
use crate::utils::id::Id;

/// Shared compensation logic for the three action types that are all "reverse this journal entry":
/// `accounting.createJournalEntry` (post path only), `accounting.postJournalDraft`,
/// `accounting.postRecurringTemplate`. D7 holds via `reverse_journal_entry`'s own
/// `allow_closed_period = is_admin(cx)`.
pub struct ReverseEntryCompensator {
    pub action_type: &'static str,
}

#[async_trait]
impl Compensator for ReverseEntryCompensator {
    fn action_type(&self) -> &'static str {
        self.action_type
    }

    fn area(&self) -> Area {
        Area::Accounting
    }

    /// Parses `payload.journalEntryId`, calls `reverse_journal_entry` with `req.date` (or "now"
    /// when absent) and `req.reason`, and returns its audit row id. An entry already reversed by
    /// hand gets `هذا القيد معكوس بالفعل` from `reverse_journal_entry` itself.
    async fn compensate(&self, tx: &DatabaseTransaction, cx: &TxCtx, registry: &UndoRegistry, original: &audit::Model, req: &UndoRequest) -> AppResult<Id> {
        let payload = original.payload.as_ref().ok_or_else(|| {
            AppError::internal("سجل قابل للتراجع بدون بيانات", Some(format!("{} audit row has no payload", self.action_type)))
        })?;
        let entry_id_str = payload.get("journalEntryId").and_then(|v| v.as_str()).ok_or_else(|| {
            AppError::internal("سجل قابل للتراجع بدون معرف القيد", Some(format!("{} payload missing journalEntryId", self.action_type)))
        })?;
        let entry_id: Id = entry_id_str
            .parse()
            .map_err(|_| AppError::internal("معرف قيد غير صالح في بيانات التراجع", Some(format!("invalid journalEntryId in payload: {entry_id_str}"))))?;

        let date = match &req.date {
            Some(raw) => raw.resolve(&cx.clock),
            None => crate::utils::dates::DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) },
        };

        let (_entry, audit_id) = super::service::journal::reverse_journal_entry(tx, cx, registry, entry_id, date, &req.reason)
            .await
            .map_err(|e| e.into_app_error())?;

        Ok(audit_id)
    }
}

/// Registers the three `ReverseEntryCompensator`s, then 12b's `CloseYearCompensator` (called from
/// here so `domains::mod.rs::register_undo` only needs the one `accounting::register_undo` line).
pub fn register_undo(r: &mut UndoRegistry) {
    r.register(std::sync::Arc::new(ReverseEntryCompensator { action_type: "accounting.createJournalEntry" }));
    r.register(std::sync::Arc::new(ReverseEntryCompensator { action_type: "accounting.postJournalDraft" }));
    r.register(std::sync::Arc::new(ReverseEntryCompensator { action_type: "accounting.postRecurringTemplate" }));
    super::undo_period::register(r);
}
