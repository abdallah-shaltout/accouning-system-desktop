//! `domains::accounting::undo_period` (12b-owned, phase-e E-5): the fourth accounting compensator,
//! `accounting.closeYear`.

use async_trait::async_trait;
use sea_orm::DatabaseTransaction;

use crate::core::auth::Area;
use crate::core::error::{AppError, AppResult};
use crate::core::tx::TxCtx;
use crate::entities::platform::audit;
use crate::shared::activity::undo::{Compensator, UndoRegistry, UndoRequest};
use crate::utils::id::Id;

pub struct CloseYearCompensator;

#[async_trait]
impl Compensator for CloseYearCompensator {
    fn action_type(&self) -> &'static str {
        "accounting.closeYear"
    }

    fn area(&self) -> Area {
        Area::Accounting
    }

    /// Parses `payload.fiscalYearId` and calls `reopen_year`. D7 holds by construction: the year
    /// is closed, and `reopen_year` itself refuses a non-admin with `إعادة فتح السنة المالية للمدير
    /// فقط` — the same check `undo()`'s `cx.require(area, Write)` cannot express (reopen needs
    /// admin specifically, not just Accounting/Write). `req.date` is ignored (reopen has no date
    /// parameter, D-A2); `req.reason` is stored by `shared::activity::undo::undo` on the link, not
    /// by `reopen_year` itself.
    async fn compensate(&self, tx: &DatabaseTransaction, cx: &TxCtx, registry: &UndoRegistry, original: &audit::Model, _req: &UndoRequest) -> AppResult<Id> {
        let payload = original
            .payload
            .as_ref()
            .ok_or_else(|| AppError::internal("سجل قابل للتراجع بدون بيانات", Some("accounting.closeYear audit row has no payload".to_string())))?;
        let fiscal_year_id_str = payload.get("fiscalYearId").and_then(|v| v.as_str()).ok_or_else(|| {
            AppError::internal("سجل قابل للتراجع بدون معرف السنة المالية", Some("accounting.closeYear payload missing fiscalYearId".to_string()))
        })?;
        let fiscal_year_id: Id = fiscal_year_id_str.parse().map_err(|_| {
            AppError::internal("معرف سنة مالية غير صالح في بيانات التراجع", Some(format!("invalid fiscalYearId in payload: {fiscal_year_id_str}")))
        })?;

        let (_fy, audit_id) = super::service::period::reopen_year(tx, cx, registry, fiscal_year_id).await.map_err(|e| e.into_app_error())?;
        Ok(audit_id)
    }
}

pub fn register(r: &mut UndoRegistry) {
    r.register(std::sync::Arc::new(CloseYearCompensator));
}
