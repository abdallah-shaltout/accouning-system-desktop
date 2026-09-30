//! `approval_requests, audit, activity` — the last block of `order::IMPORT_ORDER`. `audit` before
//! `activity` because `activity.audit_id` references `audit.id`.

use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait};

use crate::core::error::AppError;
use crate::core::tx::{TxError, TxResult};
use crate::entities::platform::activity::{ActiveModel as ActivityActiveModel, ActivityKind};
use crate::entities::platform::approval_requests::{ActiveModel as ApprovalRequestActiveModel, ApprovalKind, ApprovalStatus};
use crate::entities::platform::audit::{ActiveModel as AuditActiveModel, AuditAction};
use crate::entities::values::RouteRefValue;
use crate::infrastructure::import::idmap::IdMap;
use crate::infrastructure::import::model::{ActivityEntryV1, ApprovalRequestV1, AuditEntryV1};
use crate::infrastructure::import::tables::{parse_doc_date, resolve_created_at, strict_ref};
use crate::utils::money::round2;
use crate::utils::route::RouteRef;

fn remap_link(id_map: &IdMap, link: Option<&serde_json::Value>) -> Option<RouteRefValue> {
    let link = link?;
    let mut route: RouteRef = serde_json::from_value(link.clone()).ok()?;
    // Every param/query value that is a known snapshot id is remapped — not only `params.id`: the
    // mock's payment/receipt audit links are `{ name: 'payments', query: { highlight: 'pay-135' } }`
    // (G-13/PG-5), and a verbatim `pay-135` points at no row once payments are UUIDs (found by the
    // parity diff, plan 21 Part 04 P4-13). Non-id values (tabs, labels) never match a key.
    for values in [&mut route.params, &mut route.query].into_iter().flatten() {
        for value in values.values_mut() {
            if let Some(new_id) = id_map.resolve(value) {
                *value = new_id.to_string();
            }
        }
    }
    Some(RouteRefValue(route))
}

fn parse_approval_kind(kind: &str) -> ApprovalKind {
    match kind {
        "write_off" => ApprovalKind::WriteOff,
        "below_cost" => ApprovalKind::BelowCost,
        _ => ApprovalKind::Discount,
    }
}

fn parse_approval_status(status: &str) -> ApprovalStatus {
    match status {
        "approved" => ApprovalStatus::Approved,
        "rejected" => ApprovalStatus::Rejected,
        _ => ApprovalStatus::Pending,
    }
}

pub async fn insert_approval_requests<C: ConnectionTrait>(
    conn: &C,
    rows: &[ApprovalRequestV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
    rounded: &mut i64,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(requested_by) = id_map.resolve(&row.requested_by) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في approval_requests")));
        };
        let (requested_day, requested_instant) =
            parse_doc_date(&row.requested_at, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في approval_requests"))?;
        let decided = row.decided_at.as_deref().and_then(|s| parse_doc_date(s, tz));

        let value = round2(row.value);
        if value != row.value {
            *rounded += 1;
        }

        let model = ApprovalRequestActiveModel {
            id: Set(id),
            kind: Set(parse_approval_kind(&row.kind)),
            summary: Set(row.summary.clone()),
            value: Set(value),
            request_note: Set(row.request_note.clone()),
            requested_by: Set(requested_by),
            requested_by_name: Set(row.requested_by_name.clone()),
            requested_at_day: Set(requested_day),
            requested_at_instant: Set(requested_instant),
            status: Set(parse_approval_status(&row.status)),
            decided_by: Set(strict_ref(id_map, row.decided_by.as_deref(), "approval_requests")?),
            decided_by_name: Set(row.decided_by_name.clone()),
            decided_at_day: Set(decided.map(|(d, _)| d)),
            decided_at_instant: Set(decided.and_then(|(_, i)| i)),
            decision_comment: Set(row.decision_comment.clone()),
            link: Set(remap_link(id_map, row.link.as_ref())),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
            updated_at: Set(resolve_created_at(row.updated_at.as_deref(), import_base, i)),
            sync_status: Set(crate::entities::platform::approval_requests::SyncStatus::Local),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

/// Step 6's own rule: `audit` rows import with `action_type = NULL`, `payload = NULL`,
/// `is_undoable = false`, `undo_of/undone_by = NULL`, `terminal_id = NULL` (legacy rows carry no
/// terminal). `audit` is optional in old snapshots (`db.ts:123`) — an absent array is simply zero
/// rows, handled by the caller passing an empty slice.
pub async fn insert_audit<C: ConnectionTrait>(
    conn: &C,
    rows: &[AuditEntryV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(user_id) = id_map.resolve(&row.user_id) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في audit")));
        };
        let (at_day, at_instant) = parse_doc_date(&row.at, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في audit"))?;

        let mut before = row.before.clone();
        let mut after = row.after.clone();
        if let Some(v) = &mut before {
            id_map.remap_json(v);
        }
        if let Some(v) = &mut after {
            id_map.remap_json(v);
        }

        let action = match row.action.as_str() {
            "update" => AuditAction::Update,
            "post" => AuditAction::Post,
            "void" => AuditAction::Void,
            "reverse" => AuditAction::Reverse,
            "delete" => AuditAction::Delete,
            "login" => AuditAction::Login,
            "settings" => AuditAction::Settings,
            _ => AuditAction::Create,
        };

        // Polymorphic (B-1): resolve if known, else mint consistently (an audit row about a table
        // this importer doesn't otherwise carry, e.g. a settings/auth action).
        let entity_id = id_map.resolve(&row.entity_id).unwrap_or_else(|| id_map.resolve_or_mint(&row.entity_id));

        let model = AuditActiveModel {
            id: Set(id),
            entity: Set(row.entity.clone()),
            entity_id: Set(entity_id),
            entity_label: Set(row.entity_label.clone()),
            action: Set(action),
            before: Set(before),
            after: Set(after),
            user_id: Set(user_id),
            branch_id: Set(strict_ref(id_map, row.branch_id.as_deref(), "audit")?),
            at_day: Set(at_day),
            at_instant: Set(at_instant),
            reason: Set(row.reason.clone()),
            message: Set(row.message.clone()),
            link: Set(remap_link(id_map, row.link.as_ref())),
            action_type: Set(None),
            payload: Set(None),
            is_undoable: Set(false),
            undo_of: Set(None),
            undone_by: Set(None),
            terminal_id: Set(None),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}

pub async fn insert_activity<C: ConnectionTrait>(
    conn: &C,
    rows: &[ActivityEntryV1],
    id_map: &IdMap,
    tz: Option<chrono_tz::Tz>,
    import_base: chrono::DateTime<chrono::Utc>,
) -> TxResult<()> {
    for (i, row) in rows.iter().enumerate() {
        let id = id_map.assign(&row.id);
        let Some(user_id) = id_map.resolve(&row.user_id) else {
            return Err(TxError::App(AppError::validation("تعذر الاستيراد: مرجع غير موجود في activity")));
        };
        let (day, instant) = parse_doc_date(&row.date, tz).ok_or_else(|| AppError::validation("تاريخ غير صالح في activity"))?;

        let kind = match row.kind.as_str() {
            "refund" => ActivityKind::Refund,
            "purchase" => ActivityKind::Purchase,
            "purchase_return" => ActivityKind::PurchaseReturn,
            "payment" => ActivityKind::Payment,
            "stock" => ActivityKind::Stock,
            "journal" => ActivityKind::Journal,
            "product" => ActivityKind::Product,
            "party" => ActivityKind::Party,
            "user" => ActivityKind::User,
            "settings" => ActivityKind::Settings,
            "auth" => ActivityKind::Auth,
            "shift" => ActivityKind::Shift,
            "expense" => ActivityKind::Expense,
            "voucher" => ActivityKind::Voucher,
            "approval" => ActivityKind::Approval,
            _ => ActivityKind::Sale,
        };

        let model = ActivityActiveModel {
            id: Set(id),
            date_day: Set(day),
            date_instant: Set(instant),
            user_id: Set(user_id),
            kind: Set(kind),
            message: Set(row.message.clone()),
            link: Set(remap_link(id_map, row.link.as_ref())),
            audit_id: Set(strict_ref(id_map, row.audit_id.as_deref(), "activity")?),
            created_at: Set(resolve_created_at(row.created_at.as_deref(), import_base, i)),
        };
        model.insert(conn).await.map_err(TxError::from)?;
    }
    Ok(())
}
