//! Undo-by-compensation (rule 7, E-4): every undoable action type is registered with its
//! **existing** reversal operation (a `Compensator`); `undo()` locks the original `audit` row,
//! validates it, runs the compensator (which performs the real reversal and writes its own audit
//! row through `record`), then links both rows. Everything happens inside the caller's single
//! transaction (rule 4) — a failing compensator leaves no trace.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, EntityTrait, QuerySelect};

use crate::core::auth::Area;
use crate::core::error::{AppError, AppResult};
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::platform::audit::{self, ActiveModel as AuditActiveModel, Entity as AuditEntity};
use crate::utils::dates::RawDocDate;
use crate::utils::id::Id;

/// Recorded alongside an `audit` row when the action it represents can later be undone
/// (`AuditInput.undo`, E-2). `action_type` must be registered in the `UndoRegistry` the caller's
/// `record`/`log_undoable` call was given, or `record` fails fast with `INTERNAL`.
#[derive(Debug, Clone)]
pub struct UndoSpec {
    pub action_type: &'static str,
    pub payload: serde_json::Value,
}

/// The request driving `undo()` — `reason` is mandatory (validated non-empty after trim); `date`
/// lets the caller pick the reversal's business date, defaulting to "now" when absent (the same
/// default `PostJournal`/`ReverseRequest` callers use elsewhere).
#[derive(Debug, Clone)]
pub struct UndoRequest {
    pub reason: String,
    pub date: Option<RawDocDate>,
}

/// One registered undo handler: the existing reversal operation for a given `action_type`.
/// `compensate` runs inside the same transaction `undo()` was called in and returns the id of the
/// compensation's own audit row (written by the reversal operation itself, via `record`/
/// `log_undoable` — the compensator does not write audit rows directly).
#[async_trait]
pub trait Compensator: Send + Sync {
    /// The `action_type` string this compensator handles (matches `UndoSpec::action_type`).
    fn action_type(&self) -> &'static str;

    /// The access area `undo()` checks with `cx.require(area, Write)` before running this
    /// compensator — the same area the original action itself required.
    fn area(&self) -> Area;

    /// Runs the existing reversal operation against `original`'s recorded payload. Must pass
    /// `allow_closed_period = actor is admin` (D7) into whatever posting/period check it performs.
    /// Returns the compensation's own audit row id.
    ///
    /// G-19: receives the same `&UndoRegistry` `undo()` itself was given — a compensator's own
    /// reversal operation writes its OWN audit row via `record`/`log_undoable`
    /// (`activity::record`), and both of those calls require a `&UndoRegistry` to validate any
    /// `UndoSpec::action_type` they pass (so the compensation itself can later be undone, or so
    /// `record` can fail fast on a typo'd/unregistered action type instead of silently accepting
    /// it). Without this parameter a compensator had no legal way to call `record`/`log_undoable`
    /// at all.
    async fn compensate(&self, tx: &DatabaseTransaction, cx: &TxCtx, registry: &UndoRegistry, original: &audit::Model, req: &UndoRequest) -> AppResult<Id>;
}

/// A map of `action_type` -> `Compensator`, `Send + Sync` so it can live in `AppState` behind an
/// `Arc`. `AppState.undo` starts empty in Part 02 (no domain has registered a compensator yet);
/// Part 03 wires each domain's compensator in via `domains::register_undo`.
pub struct UndoRegistry {
    handlers: HashMap<&'static str, Arc<dyn Compensator>>,
}

impl UndoRegistry {
    pub fn new() -> Self {
        Self { handlers: HashMap::new() }
    }

    /// Registers a compensator under its own `action_type`. Panics on a duplicate — this is a
    /// startup-time programming error (two compensators claiming the same action type), never a
    /// runtime condition to recover from.
    pub fn register(&mut self, compensator: Arc<dyn Compensator>) {
        let action_type = compensator.action_type();
        if self.handlers.insert(action_type, compensator).is_some() {
            panic!("UndoRegistry: duplicate compensator registered for action_type '{action_type}'");
        }
    }

    pub fn get(&self, action_type: &str) -> Option<Arc<dyn Compensator>> {
        self.handlers.get(action_type).cloned()
    }

    pub fn contains(&self, action_type: &str) -> bool {
        self.handlers.contains_key(action_type)
    }
}

impl Default for UndoRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs the undo of the audit row `audit_id`, inside the caller's transaction (rule 4). Steps
/// (E-4): lock the row `FOR UPDATE`; validate reason/undoable/not-already-undone; check the
/// compensator's area for `Write`; run `compensate`; link `original.undone_by` / `comp.undo_of`.
pub async fn undo(
    conn: &DatabaseTransaction,
    cx: &TxCtx,
    registry: &UndoRegistry,
    audit_id: Id,
    req: UndoRequest,
) -> TxResult<Id> {
    let reason = req.reason.trim();
    if reason.is_empty() {
        return Err(AppError::validation("سبب التراجع مطلوب").into());
    }

    let original = AuditEntity::find_by_id(audit_id)
        .lock_exclusive()
        .one(conn)
        .await?
        .ok_or_else(|| AppError::not_found("السجل غير موجود"))?;

    if !original.is_undoable {
        return Err(AppError::validation("هذه العملية لا يمكن التراجع عنها").into());
    }
    if original.undone_by.is_some() {
        return Err(AppError::conflict("تم التراجع عن هذه العملية بالفعل").into());
    }

    let action_type = original.action_type.as_deref().ok_or_else(|| {
        AppError::internal("سجل قابل للتراجع بدون action_type", Some(format!("audit row {audit_id} has is_undoable=true but action_type=NULL")))
    })?;
    let compensator = registry.get(action_type).ok_or_else(|| {
        AppError::internal("نوع عملية غير مسجل للتراجع", Some(format!("undo(): action_type '{action_type}' is not registered in the UndoRegistry")))
    })?;

    cx.require(conn, compensator.area(), crate::core::auth::Access::Write).await?;

    let req = UndoRequest { reason: reason.to_string(), date: req.date };
    let comp_id = compensator.compensate(conn, cx, registry, &original, &req).await?;

    let mut original_active: AuditActiveModel = original.clone().into();
    original_active.undone_by = sea_orm::Set(Some(comp_id));
    original_active.update(conn).await?;

    let comp_model = AuditEntity::find_by_id(comp_id)
        .one(conn)
        .await?
        .ok_or_else(|| AppError::internal("سجل التراجع غير موجود بعد تنفيذه", Some(format!("compensator wrote audit id {comp_id} but it can't be read back"))))?;
    let mut comp_active: AuditActiveModel = comp_model.into();
    comp_active.undo_of = sea_orm::Set(Some(audit_id));
    comp_active.update(conn).await?;

    Ok(comp_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Noop;
    #[async_trait]
    impl Compensator for Noop {
        fn action_type(&self) -> &'static str {
            "test.noop"
        }
        fn area(&self) -> Area {
            Area::Accounting
        }
        async fn compensate(
            &self,
            _tx: &DatabaseTransaction,
            _cx: &TxCtx,
            _registry: &UndoRegistry,
            _original: &audit::Model,
            _req: &UndoRequest,
        ) -> Result<Id, AppError> {
            Ok(Id::new())
        }
    }

    #[test]
    fn register_and_get_round_trip() {
        let mut registry = UndoRegistry::new();
        assert!(!registry.contains("test.noop"));
        registry.register(Arc::new(Noop));
        assert!(registry.contains("test.noop"));
        assert!(registry.get("test.noop").is_some());
        assert!(registry.get("test.other").is_none());
    }

    #[test]
    #[should_panic(expected = "duplicate compensator")]
    fn register_panics_on_duplicate() {
        let mut registry = UndoRegistry::new();
        registry.register(Arc::new(Noop));
        registry.register(Arc::new(Noop));
    }
}
