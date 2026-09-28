//! `record`/`log`/`log_undoable`/`entity_from_link` — a behaviour-exact port of
//! `logAudit`/`logActivity`/`entityFromLink` (`src/mocks/backend/core.ts:306-438`). This is the
//! **only** code that inserts into `audit`/`activity` (master plan rule 3, E-6).

use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use crate::core::error::AppError;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::doc_date;
use crate::entities::platform::activity::{ActiveModel as ActivityActiveModel, ActivityKind};
use crate::entities::platform::audit::{ActiveModel as AuditActiveModel, AuditAction};
use crate::entities::values::RouteRefValue;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;
use crate::utils::route::RouteRef;

use super::undo::{UndoRegistry, UndoSpec};

/// `DEFAULT_ACTION_BY_KIND` (`core.ts:307-310`): the coarser `AuditAction` used when a call site
/// doesn't say one explicitly.
pub fn default_action_by_kind(kind: ActivityKind) -> Option<AuditAction> {
    match kind {
        ActivityKind::Auth => Some(AuditAction::Login),
        ActivityKind::Settings => Some(AuditAction::Settings),
        _ => None,
    }
}

/// `ROUTE_NAME_TO_ENTITY` (`core.ts:320-332`, 11 entries) — route name -> business entity string.
fn route_name_to_entity(name: &str) -> Option<&'static str> {
    match name {
        "invoice" => Some("invoice"),
        "expense-detail" => Some("expense"),
        "adjustment" => Some("adjustment"),
        "count" => Some("count"),
        "journal-entry" => Some("journal"),
        "purchase" => Some("purchaseOrder"),
        "customer" => Some("customer"),
        "supplier" => Some("supplier"),
        "user-editor" => Some("user"),
        "voucher-detail" => Some("voucher"),
        "transfers" => Some("transfer"),
        _ => None,
    }
}

/// `ENTITY_TO_ACTIVITY_KIND` (`core.ts:383-403`, 19 entries).
fn entity_to_activity_kind(entity: &str) -> Option<ActivityKind> {
    match entity {
        "invoice" => Some(ActivityKind::Sale),
        "refund" => Some(ActivityKind::Refund),
        "purchaseOrder" => Some(ActivityKind::Purchase),
        "purchaseReturn" => Some(ActivityKind::PurchaseReturn),
        "payment" => Some(ActivityKind::Payment),
        "voucher" => Some(ActivityKind::Voucher),
        "expense" => Some(ActivityKind::Expense),
        "journal" => Some(ActivityKind::Journal),
        "product" => Some(ActivityKind::Product),
        "customer" => Some(ActivityKind::Party),
        "supplier" => Some(ActivityKind::Party),
        "user" => Some(ActivityKind::User),
        "branch" => Some(ActivityKind::Settings),
        "costCenter" => Some(ActivityKind::Settings),
        "shift" => Some(ActivityKind::Shift),
        "approval" => Some(ActivityKind::Approval),
        "adjustment" => Some(ActivityKind::Stock),
        "count" => Some(ActivityKind::Stock),
        "transfer" => Some(ActivityKind::Stock),
        _ => None,
    }
}

/// `entityFromLink` (`core.ts:335-343`, rewritten in 01.C): the route name maps to an entity and
/// `params.id` parses as an `Id`, else the fallback `(kind as entity, Id::new())`.
pub fn entity_from_link(kind: ActivityKind, link: Option<&RouteRef>) -> (String, Id) {
    if let Some(link) = link {
        if let Some(entity) = route_name_to_entity(&link.name) {
            if let Some(id_str) = link.params.as_ref().and_then(|p| p.get("id")) {
                if let Ok(id) = id_str.parse::<Id>() {
                    return (entity.to_string(), id);
                }
            }
        }
    }
    (activity_kind_as_str(kind).to_string(), Id::new())
}

fn activity_kind_as_str(kind: ActivityKind) -> &'static str {
    match kind {
        ActivityKind::Sale => "sale",
        ActivityKind::Refund => "refund",
        ActivityKind::Purchase => "purchase",
        ActivityKind::PurchaseReturn => "purchase_return",
        ActivityKind::Payment => "payment",
        ActivityKind::Stock => "stock",
        ActivityKind::Journal => "journal",
        ActivityKind::Product => "product",
        ActivityKind::Party => "party",
        ActivityKind::User => "user",
        ActivityKind::Settings => "settings",
        ActivityKind::Auth => "auth",
        ActivityKind::Shift => "shift",
        ActivityKind::Expense => "expense",
        ActivityKind::Voucher => "voucher",
        ActivityKind::Approval => "approval",
    }
}

/// Everything `logAudit`/`record` needs (`LogAuditInput`, `core.ts:373-386`). `user_id`/`at` default
/// to the transaction's actor/clock when `None`, mirroring the mock's implicit "current user/now".
pub struct AuditInput {
    pub entity: String,
    pub entity_id: Id,
    pub entity_label: Option<String>,
    pub action: AuditAction,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
    pub user_id: Option<Id>,
    pub branch_id: Option<Id>,
    pub at: Option<DocDate>,
    pub reason: Option<String>,
    pub message: String,
    pub link: Option<RouteRef>,
    /// Also appended to the `activity` feed under this kind. Defaults to `ENTITY_TO_ACTIVITY_KIND`
    /// (falling back to `settings`, `core.ts:437`) when `None`.
    pub activity_kind: Option<ActivityKind>,
    /// When set, this action can later be undone through the registry (E-4).
    pub undo: Option<UndoSpec>,
}

/// `logAudit` (`core.ts:409-438`): inserts **one** `audit` row and **one** `activity` row, in that
/// order, both inside the caller's transaction. If `undo` names an `action_type` that isn't in the
/// registry, this is a programming bug — fails fast with `AppError::Internal` (E-2).
pub async fn record<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &UndoRegistry, input: AuditInput) -> TxResult<Id> {
    if let Some(undo) = &input.undo {
        if !registry.contains(undo.action_type) {
            return Err(AppError::internal(
                "نوع عملية غير مسجل للتراجع",
                Some(format!("undo.action_type '{}' is not registered in the UndoRegistry", undo.action_type)),
            )
            .into());
        }
    }

    let user_id = input.user_id.or(cx.actor.as_ref().map(|a| a.id)).ok_or_else(|| {
        AppError::internal("لا يمكن تسجيل العملية بدون مستخدم", Some("record() called with no user_id and no actor in TxCtx".to_string()))
    })?;
    let at = input.at.unwrap_or_else(|| DocDate { day: cx.clock.today(), instant: Some(cx.clock.now) });
    let (at_day, at_instant) = doc_date::write(at);

    let audit_id = Id::new();
    let is_undoable = input.undo.is_some();
    let (action_type, payload) = match &input.undo {
        Some(spec) => (Some(spec.action_type.to_string()), Some(spec.payload.clone())),
        None => (None, None),
    };

    let audit = AuditActiveModel {
        id: Set(audit_id),
        entity: Set(input.entity.clone()),
        entity_id: Set(input.entity_id),
        entity_label: Set(input.entity_label),
        action: Set(input.action),
        before: Set(input.before),
        after: Set(input.after),
        user_id: Set(user_id),
        branch_id: Set(input.branch_id),
        at_day: Set(at_day),
        at_instant: Set(at_instant),
        reason: Set(input.reason),
        message: Set(input.message.clone()),
        link: Set(input.link.clone().map(RouteRefValue::from)),
        action_type: Set(action_type),
        payload: Set(payload),
        is_undoable: Set(is_undoable),
        undo_of: Set(None),
        undone_by: Set(None),
        terminal_id: Set(Some(cx.terminal_id)),
        created_at: Set(cx.clock.now),
    };
    audit.insert(conn).await?;

    let kind = input.activity_kind.unwrap_or_else(|| entity_to_activity_kind(&input.entity).unwrap_or(ActivityKind::Settings));

    let activity = ActivityActiveModel {
        id: Set(Id::new()),
        date_day: Set(at_day),
        date_instant: Set(at_instant),
        user_id: Set(user_id),
        kind: Set(kind),
        message: Set(input.message),
        link: Set(input.link.map(RouteRefValue::from)),
        audit_id: Set(Some(audit_id)),
        created_at: Set(cx.clock.now),
    };
    activity.insert(conn).await?;

    Ok(audit_id)
}

/// `logActivity` (`core.ts:352-364`): a thin adapter over `record` — resolves `(entity, entity_id)`
/// from the link (or the fallback), and the action from `DEFAULT_ACTION_BY_KIND` (defaulting to
/// `create`, `core.ts:359`).
pub async fn log<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    kind: ActivityKind,
    message: impl Into<String>,
    date: Option<DocDate>,
    link: Option<RouteRef>,
) -> TxResult<Id> {
    let (entity, entity_id) = entity_from_link(kind.clone(), link.as_ref());
    record(
        conn,
        cx,
        registry,
        AuditInput {
            entity,
            entity_id,
            entity_label: None,
            action: default_action_by_kind(kind.clone()).unwrap_or(AuditAction::Create),
            before: None,
            after: None,
            user_id: None,
            branch_id: None,
            at: date,
            reason: None,
            message: message.into(),
            link,
            activity_kind: Some(kind),
            undo: None,
        },
    )
    .await
}

/// `log` plus an `UndoSpec` — the undoable variant.
#[allow(clippy::too_many_arguments)]
pub async fn log_undoable<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    kind: ActivityKind,
    message: impl Into<String>,
    date: Option<DocDate>,
    link: Option<RouteRef>,
    undo: UndoSpec,
) -> TxResult<Id> {
    let (entity, entity_id) = entity_from_link(kind.clone(), link.as_ref());
    record(
        conn,
        cx,
        registry,
        AuditInput {
            entity,
            entity_id,
            entity_label: None,
            action: default_action_by_kind(kind.clone()).unwrap_or(AuditAction::Create),
            before: None,
            after: None,
            user_id: None,
            branch_id: None,
            at: date,
            reason: None,
            message: message.into(),
            link,
            activity_kind: Some(kind),
            undo: Some(undo),
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn route(name: &str, id: Option<&str>) -> RouteRef {
        let params = id.map(|id| {
            let mut m = BTreeMap::new();
            m.insert("id".to_string(), id.to_string());
            m
        });
        RouteRef { name: name.to_string(), params, query: None }
    }

    #[test]
    fn entity_from_link_resolves_known_route_and_id() {
        let id = Id::new();
        let link = route("invoice", Some(&id.to_string()));
        let (entity, entity_id) = entity_from_link(ActivityKind::Sale, Some(&link));
        assert_eq!(entity, "invoice");
        assert_eq!(entity_id, id);
    }

    #[test]
    fn entity_from_link_falls_back_on_list_route() {
        let link = route("approvals", None);
        let (entity, _id) = entity_from_link(ActivityKind::Approval, Some(&link));
        assert_eq!(entity, "approval");
    }

    #[test]
    fn entity_from_link_falls_back_when_no_link() {
        let (entity, _id) = entity_from_link(ActivityKind::Journal, None);
        assert_eq!(entity, "journal");
    }

    #[test]
    fn entity_from_link_falls_back_on_unmapped_route() {
        let link = route("some-unmapped-route", Some("abc"));
        let (entity, _id) = entity_from_link(ActivityKind::Product, Some(&link));
        assert_eq!(entity, "product");
    }

    #[test]
    fn entity_from_link_falls_back_when_id_is_not_a_valid_uuid() {
        let link = route("invoice", Some("not-a-uuid"));
        let (entity, _id) = entity_from_link(ActivityKind::Sale, Some(&link));
        // route matched but id didn't parse -> fallback to (kind, new id), not (entity, garbage).
        assert_eq!(entity, "sale");
    }

    #[test]
    fn default_action_by_kind_matches_table() {
        assert_eq!(default_action_by_kind(ActivityKind::Auth), Some(AuditAction::Login));
        assert_eq!(default_action_by_kind(ActivityKind::Settings), Some(AuditAction::Settings));
        assert_eq!(default_action_by_kind(ActivityKind::Sale), None);
    }
}
