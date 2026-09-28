//! `approvals` service logic (03-domains/04-approvals.md §3) — behaviour-exact port of
//! `src/mocks/backend/approvals.ts`. Every step below cites its mock line.

use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::AppError;
use crate::core::lock;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::doc_date;
use crate::entities::org::users::Entity as UserEntity;
use crate::entities::platform::approval_requests::{
    ActiveModel as ApprovalRequestActiveModel, Column, Entity as ApprovalRequestEntity, ApprovalStatus as EntityApprovalStatus,
};
use crate::entities::platform::activity::ActivityKind;
use crate::shared::activity;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;
use crate::utils::money::round4;
use crate::utils::route::RouteRef;

use super::dto::{to_dto, ApprovalDecisionInput, ApprovalKind, ApprovalListFilter, ApprovalRequest, ApprovalRequestInput, ApprovalStatus};

/// `KIND_LABEL` (`approvals.ts:13-17`), copied byte for byte.
fn kind_label(kind: ApprovalKind) -> &'static str {
    match kind {
        ApprovalKind::Discount => "خصم يتجاوز الحد المسموح",
        ApprovalKind::WriteOff => "إتلاف/تسوية مخزون تتجاوز حد الاعتماد",
        ApprovalKind::BelowCost => "بيع بسعر أقل من التكلفة",
    }
}

/// The `users.name` of `cx.actor.id`, or `"مستخدم"` when the row is missing (`approvalService.ts:8-10`'s
/// fallback) — the name is **never** taken from the client.
async fn actor_name<C: ConnectionTrait>(conn: &C, cx: &TxCtx) -> TxResult<String> {
    let Some(actor) = &cx.actor else {
        return Ok("مستخدم".to_string());
    };
    let row = UserEntity::find_by_id(actor.id).one(conn).await?;
    Ok(row.map(|u| u.name).unwrap_or_else(|| "مستخدم".to_string()))
}

/// **`submit`** (`approvals.ts:19-36`). Access-by-kind (D-2) is checked in the command layer before
/// this is called.
pub async fn submit<C: ConnectionTrait>(conn: &C, cx: &TxCtx, registry: &activity::UndoRegistry, input: ApprovalRequestInput) -> TxResult<ApprovalRequest> {
    let actor_id = cx
        .actor
        .as_ref()
        .map(|a| a.id)
        .ok_or_else(|| AppError::internal("لا يمكن تقديم طلب اعتماد بدون مستخدم", Some("submit() called with no actor in TxCtx".to_string())))?;
    let name = actor_name(conn, cx).await?;

    let id = Id::new();
    let now = cx.clock.now;
    let requested_at = DocDate { day: cx.clock.today(), instant: Some(now) };
    let (requested_at_day, requested_at_instant) = doc_date::write(requested_at.clone());

    // `requestNote?.trim() || undefined` (`:25`) — whitespace-only collapses to `None`.
    let request_note = input.request_note.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);

    let model = ApprovalRequestActiveModel {
        id: Set(id),
        kind: Set(input.kind.into()),
        summary: Set(input.summary.clone()),
        value: Set(round4(input.value)),
        request_note: Set(request_note),
        requested_by: Set(actor_id),
        requested_by_name: Set(name),
        requested_at_day: Set(requested_at_day),
        requested_at_instant: Set(requested_at_instant),
        status: Set(EntityApprovalStatus::Pending),
        decided_by: Set(None),
        decided_by_name: Set(None),
        decided_at_day: Set(None),
        decided_at_instant: Set(None),
        decision_comment: Set(None),
        link: Set(input.link.clone().map(Into::into)),
        created_at: Set(now),
        updated_at: Set(now),
        sync_status: Set(crate::entities::platform::approval_requests::SyncStatus::Local),
    };
    let inserted = model.insert(conn).await?;

    // `logActivity('approval', ...)` (`:33`).
    activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Approval,
        format!("طلب اعتماد جديد: {} — {}", kind_label(input.kind), input.summary),
        Some(requested_at),
        Some(RouteRef::list("approvals")),
    )
    .await?;

    // `emit('ledger:changed')` (`:34`, D-1).
    cx.touch(crate::core::events::ChangeCategory::Ledger);

    Ok(to_dto(&inserted))
}

/// **`list`** (`:67-69`): `[WHERE status = ?]` then insertion order from SQL, then a stable Rust
/// sort by `requested_at` key descending — ties keep insertion order (`Vec::sort_by` is stable).
pub async fn list<C: ConnectionTrait>(conn: &C, filter: Option<ApprovalListFilter>) -> TxResult<Vec<ApprovalRequest>> {
    let mut query = ApprovalRequestEntity::find();
    if let Some(status) = filter.and_then(|f| f.status) {
        let entity_status: EntityApprovalStatus = status.into();
        query = query.filter(Column::Status.eq(entity_status));
    }
    let rows = query.order_by_asc(Column::CreatedAt).order_by_asc(Column::Id).all(conn).await?;

    let mut dtos: Vec<ApprovalRequest> = rows.iter().map(to_dto).collect();
    dtos.sort_by(|a, b| b.requested_at.cmp(&a.requested_at));
    Ok(dtos)
}

/// **`pending_count`** (`:71-73`).
pub async fn pending_count<C: ConnectionTrait>(conn: &C) -> TxResult<u32> {
    let count = ApprovalRequestEntity::find().filter(Column::Status.eq(EntityApprovalStatus::Pending)).count(conn).await?;
    Ok(count as u32)
}

/// **`decide`** (`:44-65`) — shared by `approve_request`/`reject_request`. `status` is the
/// requested outcome (`approved`/`rejected`); `ApprovalStatus::Pending` is never a valid argument
/// here (the command layer only calls this with a terminal status).
pub async fn decide<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &activity::UndoRegistry,
    id: Id,
    status: ApprovalStatus,
    input: ApprovalDecisionInput,
) -> TxResult<ApprovalRequest> {
    let actor_id = cx
        .actor
        .as_ref()
        .map(|a| a.id)
        .ok_or_else(|| AppError::internal("لا يمكن اتخاذ قرار اعتماد بدون مستخدم", Some("decide() called with no actor in TxCtx".to_string())))?;

    // 1. Lock + load (`:38-41`).
    lock::for_update_by_id(conn, "approval_requests", &id.to_string()).await?;
    let existing = ApprovalRequestEntity::find_by_id(id).one(conn).await?.ok_or_else(|| AppError::not_found("طلب الاعتماد غير موجود"))?;

    // 2. Already decided (`:46`) — read after the lock.
    if existing.status != EntityApprovalStatus::Pending {
        return Err(AppError::validation("تم اتخاذ قرار بشأن هذا الطلب مسبقاً").into());
    }

    // 3. Reject requires a non-empty comment (`:47`).
    let trimmed_comment = input.comment.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    if status == ApprovalStatus::Rejected && trimmed_comment.is_none() {
        return Err(AppError::validation("أدخل سبب الرفض").into());
    }

    let name = actor_name(conn, cx).await?;
    let now = cx.clock.now;
    let decided_at = DocDate { day: cx.clock.today(), instant: Some(now) };
    let (decided_at_day, decided_at_instant) = doc_date::write(decided_at.clone());

    // 4. Update.
    let mut active_model: ApprovalRequestActiveModel = existing.clone().into();
    active_model.status = Set(status.into());
    active_model.decided_by = Set(Some(actor_id));
    active_model.decided_by_name = Set(Some(name));
    active_model.decided_at_day = Set(Some(decided_at_day));
    active_model.decided_at_instant = Set(decided_at_instant);
    active_model.decision_comment = Set(trimmed_comment);
    active_model.updated_at = Set(now);
    let updated = active_model.update(conn).await?;

    // 5. `logActivity` (`:56-62`).
    let approved = status == ApprovalStatus::Approved;
    let kind: ApprovalKind = existing.kind.clone().into();
    activity::log(
        conn,
        cx,
        registry,
        ActivityKind::Approval,
        format!("{} طلب: {} — {}", if approved { "اعتماد" } else { "رفض" }, kind_label(kind), existing.summary),
        Some(decided_at),
        Some(RouteRef::list("approvals")),
    )
    .await?;

    // 6. `emit('ledger:changed')` (`:63`).
    cx.touch(crate::core::events::ChangeCategory::Ledger);

    Ok(to_dto(&updated))
}
