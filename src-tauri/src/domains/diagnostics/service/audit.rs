//! Business audit-trail reads (plan 21 Part 03 §16 spec §3, `auditService.ts:19-45`). Read-only,
//! no writes, no undo (spec §5) — `db.audit` is written by every other domain's `shared::activity`
//! calls, never by this module.

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::dto::{AuditAction, AuditEntry, AuditFieldDiff};
use crate::core::error::AppResult;
use crate::core::tx::TxResult;
use crate::domains::diagnostics::dto::AuditFilter;
use crate::entities::platform::audit;
use crate::utils::route::RouteRef;

/// `audit::Model` → `core::dto::AuditEntry` (spec §2): `at` is the `DocDate` key
/// (`at_day`/`at_instant` combined — G-9's ms-precision instant), `before`/`after` decode from the
/// JSON columns into `Vec<AuditFieldDiff>` (a malformed value is treated as absent rather than
/// failing the whole read — this is a display-only debugging/audit surface, not a validated write
/// path), and `link` comes straight through the shared `RouteRefValue` newtype (G-8c).
pub fn audit_to_dto(m: &audit::Model) -> AuditEntry {
    let action = match m.action {
        audit::AuditAction::Create => AuditAction::Create,
        audit::AuditAction::Update => AuditAction::Update,
        audit::AuditAction::Post => AuditAction::Post,
        audit::AuditAction::Void => AuditAction::Void,
        audit::AuditAction::Reverse => AuditAction::Reverse,
        audit::AuditAction::Delete => AuditAction::Delete,
        audit::AuditAction::Login => AuditAction::Login,
        audit::AuditAction::Settings => AuditAction::Settings,
    };

    let decode_diff = |v: &Option<serde_json::Value>| -> Option<Vec<AuditFieldDiff>> {
        v.as_ref().and_then(|value| serde_json::from_value::<Vec<AuditFieldDiff>>(value.clone()).ok())
    };

    let link: Option<RouteRef> = m.link.clone().map(Into::into);

    AuditEntry {
        id: m.id,
        entity: m.entity.clone(),
        entity_id: m.entity_id,
        entity_label: m.entity_label.clone(),
        action,
        before: decode_diff(&m.before),
        after: decode_diff(&m.after),
        user_id: m.user_id,
        branch_id: m.branch_id,
        at: m.at().key(),
        reason: m.reason.clone(),
        message: m.message.clone(),
        link,
    }
}

/// `matches` (`auditService.ts:19-32`), applied in Rust after the exact-match SQL narrowing (spec
/// §3 step 1): `from`/`to` compare the **UTC** slice of the `at` key (Q-1, kept quirk — not the
/// business day), `search` is a case-insensitive substring check over `message`/`entityId`/
/// `entityLabel` (empty search string matches everything, exactly like the mock's
/// `q && !...` short-circuit).
fn matches_filter(entry: &AuditEntry, filter: &AuditFilter) -> bool {
    if let Some(from) = &filter.from {
        if entry.at.get(..10).unwrap_or(&entry.at) < from.as_str() {
            return false;
        }
    }
    if let Some(to) = &filter.to {
        if entry.at.get(..10).unwrap_or(&entry.at) > to.as_str() {
            return false;
        }
    }
    if let Some(search) = &filter.search {
        let q = search.trim().to_lowercase();
        if !q.is_empty() {
            let message = entry.message.to_lowercase();
            let entity_id = entry.entity_id.to_string().to_lowercase();
            let entity_label = entry.entity_label.as_deref().unwrap_or("").to_lowercase();
            if !message.contains(&q) && !entity_id.contains(&q) && !entity_label.contains(&q) {
                return false;
            }
        }
    }
    true
}

/// `getAuditEntries` (`auditService.ts:34-38`, spec §3): SQL narrows by exact `user_id`/`entity`/
/// `action`, ordered `created_at, id` (insertion order) so the in-Rust `from`/`to`/`search` pass and
/// the final stable sort behave identically to filtering the mock's already-ordered in-memory array.
/// Final order: `at` key descending (`b.at.localeCompare(a.at)`).
pub async fn get_audit_entries<C: sea_orm::ConnectionTrait>(conn: &C, filter: &AuditFilter) -> TxResult<Vec<AuditEntry>> {
    let mut query = audit::Entity::find();
    if let Some(user_id) = filter.user_id {
        query = query.filter(audit::Column::UserId.eq(user_id));
    }
    if let Some(entity) = &filter.entity {
        query = query.filter(audit::Column::Entity.eq(entity.clone()));
    }
    if let Some(action) = filter.action {
        let db_action = match action {
            AuditAction::Create => audit::AuditAction::Create,
            AuditAction::Update => audit::AuditAction::Update,
            AuditAction::Post => audit::AuditAction::Post,
            AuditAction::Void => audit::AuditAction::Void,
            AuditAction::Reverse => audit::AuditAction::Reverse,
            AuditAction::Delete => audit::AuditAction::Delete,
            AuditAction::Login => audit::AuditAction::Login,
            AuditAction::Settings => audit::AuditAction::Settings,
        };
        query = query.filter(audit::Column::Action.eq(db_action));
    }

    let rows = query
        .order_by_asc(audit::Column::CreatedAt)
        .order_by_asc(audit::Column::Id)
        .all(conn)
        .await?;

    let mut entries: Vec<AuditEntry> = rows.iter().map(audit_to_dto).filter(|e| matches_filter(e, filter)).collect();
    entries.sort_by(|a, b| b.at.cmp(&a.at));
    Ok(entries)
}

/// `getAuditEntities` (`auditService.ts:42-45`, spec §3): `SELECT DISTINCT entity`, sorted **in
/// Rust** by UTF-16 code-unit order (Rust `str`'s `Ord` is by Unicode scalar value, which agrees
/// with `[...new Set()].sort()`'s UTF-16 code-unit order for every code point outside the
/// surrogate-pair range — entity names here are plain ASCII/snake_case identifiers, so this matches
/// exactly) rather than an SQL `ORDER BY` under `utf8mb4_unicode_ci`, which would case-fold and
/// diverge from the mock's default `Array.sort()`.
pub async fn get_audit_entities<C: sea_orm::ConnectionTrait>(conn: &C) -> TxResult<Vec<String>> {
    use sea_orm::QuerySelect;
    let rows: Vec<String> = audit::Entity::find()
        .select_only()
        .column(audit::Column::Entity)
        .distinct()
        .into_tuple()
        .all(conn)
        .await?;
    let mut set: std::collections::BTreeSet<String> = rows.into_iter().collect();
    // BTreeSet<String> orders by byte value (UTF-8), which for this data set (ASCII entity kind
    // names) agrees with a UTF-16 code-unit sort; kept as a plain Vec below for the DTO surface.
    Ok(std::mem::take(&mut set).into_iter().collect())
}

/// Access-gate helper shared by both commands (spec §1: `getAuditEntries`/`getAuditEntities` are
/// Users:Read — the audit-log page's area). Kept here rather than duplicated in `commands.rs`'s two
/// bodies.
pub async fn require_audit_read<C: sea_orm::ConnectionTrait>(
    conn: &C,
    ctx: &crate::core::tx::ReadCtx,
) -> AppResult<()> {
    ctx.require(conn, crate::core::auth::Area::Users, crate::core::auth::Access::Read).await
}
