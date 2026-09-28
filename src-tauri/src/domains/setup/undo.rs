//! `domains::setup::undo` (02-setup.md §5, phase-e E-5): registers **one** compensator —
//! `setup.postPartyOpening` — reversing a party opening balance via `reverse_party_opening`.

use async_trait::async_trait;
use sea_orm::DatabaseTransaction;

use crate::core::auth::{Area, Role};
use crate::core::error::{AppError, AppResult};
use crate::core::tx::TxCtx;
use crate::entities::platform::audit;
use crate::shared::activity::undo::{Compensator, UndoRegistry, UndoRequest};
use crate::utils::id::Id;

pub struct PartyOpeningCompensator;

#[async_trait]
impl Compensator for PartyOpeningCompensator {
    fn action_type(&self) -> &'static str {
        "setup.postPartyOpening"
    }

    fn area(&self) -> Area {
        Area::Parties
    }

    /// D7: `allow_closed_period = actor is admin`. Reads `payload.entryId`, calls
    /// `reverse_party_opening`, and returns its audit row id (the registry links `undo_of`/`undone_by`).
    async fn compensate(
        &self,
        tx: &DatabaseTransaction,
        cx: &TxCtx,
        registry: &UndoRegistry,
        original: &audit::Model,
        req: &UndoRequest,
    ) -> AppResult<Id> {
        let payload = original.payload.as_ref().ok_or_else(|| {
            AppError::internal("سجل قابل للتراجع بدون بيانات", Some("setup.postPartyOpening audit row has no payload".to_string()))
        })?;
        let entry_id_str = payload.get("entryId").and_then(|v| v.as_str()).ok_or_else(|| {
            AppError::internal("سجل قابل للتراجع بدون معرف القيد", Some("setup.postPartyOpening payload missing entryId".to_string()))
        })?;
        let entry_id: Id = entry_id_str
            .parse()
            .map_err(|_| AppError::internal("معرف قيد غير صالح في بيانات التراجع", Some(format!("invalid entryId in payload: {entry_id_str}"))))?;

        let allow_closed_period = cx.actor.as_ref().map(|a| a.role == Role::Admin).unwrap_or(false);

        let audit_id = super::service::party_opening::reverse_party_opening(
            tx,
            cx,
            registry,
            entry_id,
            allow_closed_period,
            Some(req.reason.clone()),
        )
        .await
        .map_err(|e| e.into_app_error())?;

        Ok(audit_id)
    }
}

pub fn register_undo(r: &mut UndoRegistry) {
    r.register(std::sync::Arc::new(PartyOpeningCompensator));
}
