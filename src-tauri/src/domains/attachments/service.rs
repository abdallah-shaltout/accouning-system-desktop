//! `attachments` service logic (C-16) — behaviour-exact port of
//! `src/mocks/attachments.ts` (IndexedDB) via its seam wrapper
//! `src/modules/core/services/attachmentService.ts`: `fetchAttachments` (by `ownerRef`, newest
//! first), `fetchAttachment` (by id), `saveAttachment` (upsert), `removeAttachment` (hard delete),
//! plus `fetchAttachmentsByIds` (a batch lookup no mock caller needed yet, but every other domain's
//! `attachment_ids: Vec<String>` field will want once it stops being an opaque id list — same
//! pattern as `fetchAttachments`, just keyed by an explicit id list instead of `ownerRef`).
//!
//! Not branch-scoped, not soft-deleted (the mock's IndexedDB store was neither) — every logged-in
//! user/terminal shares the one `attachments` table (D8), which is the whole point of C-16.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::AppError;
use crate::core::tx::{TxCtx, TxResult};
use crate::entities::platform::attachments::{ActiveModel, Column, Entity as AttachmentEntity};
use crate::utils::dates::format_iso_ms;
use crate::utils::id::Id;

use super::dto::{AttachmentKind, AttachmentMeta, AttachmentRecord};

/// Mirrors `helpers/attachments.ts`'s `MAX_ATTACHMENT_SIZE` (10MB) — enforced again here since a
/// real backend must never trust a frontend-side check alone (CLAUDE.md: strict server-side
/// validation over trusting the frontend).
pub const MAX_ATTACHMENT_SIZE: usize = 10 * 1024 * 1024;

fn decode_base64(field: &str, s: &str) -> TxResult<Vec<u8>> {
    BASE64
        .decode(s)
        .map_err(|_| AppError::validation(format!("تعذرت قراءة الملف — {field} غير صالح")).into())
}

fn to_meta(model: &crate::entities::platform::attachments::Model) -> AttachmentMeta {
    AttachmentMeta {
        id: model.id,
        owner_ref: model.owner_ref.clone(),
        name: model.name.clone(),
        mime: model.mime.clone(),
        kind: model.kind.into(),
        size: model.size as i32,
        width: model.width,
        height: model.height,
        created_at: format_iso_ms(model.created_at),
        created_by: model.created_by.map(|id| id.to_string()),
    }
}

fn to_record(model: crate::entities::platform::attachments::Model) -> AttachmentRecord {
    AttachmentRecord {
        id: model.id,
        owner_ref: model.owner_ref.clone(),
        name: model.name.clone(),
        mime: model.mime.clone(),
        kind: model.kind.into(),
        size: model.size as i32,
        width: model.width,
        height: model.height,
        created_at: format_iso_ms(model.created_at),
        created_by: model.created_by.map(|id| id.to_string()),
        blob_base64: BASE64.encode(&model.blob),
        thumbnail_base64: model.thumbnail.as_ref().map(|b| BASE64.encode(b)),
    }
}

/// `listAttachments(ownerRef)` (`attachments.ts:88-101`, T: `fetchAttachments`) — every attachment
/// for the given owner, newest first, meta only (never the blob — matches the mock's own
/// `getAll()` + `map(({ blob, thumbnail, ...meta }) => meta)`).
pub async fn fetch_attachments<C: ConnectionTrait>(conn: &C, owner_ref: &str) -> TxResult<Vec<AttachmentMeta>> {
    let rows = AttachmentEntity::find()
        .filter(Column::OwnerRef.eq(owner_ref))
        .order_by_desc(Column::CreatedAt)
        .all(conn)
        .await?;
    Ok(rows.iter().map(to_meta).collect())
}

/// `getAttachment(id)` (`attachments.ts:73-81`, T: `fetchAttachment`) — the full record (with blob),
/// or `None` for an unparsable/missing id (the mock's IndexedDB `get` resolves `undefined`, never
/// throws, for a missing key).
pub async fn fetch_attachment<C: ConnectionTrait>(conn: &C, id: &str) -> TxResult<Option<AttachmentRecord>> {
    let Ok(parsed) = id.parse::<Id>() else { return Ok(None) };
    let row = AttachmentEntity::find_by_id(parsed).one(conn).await?;
    Ok(row.map(to_record))
}

/// Batch lookup by id list (new — no mock precedent, see module doc). Skips any id that fails to
/// parse or has no row, same "missing = absent, never an error" behaviour as `fetch_attachment`.
/// Returned in the same order as `ids` (deduplicated ids only counted once — a caller repeating an
/// id in `attachment_ids` gets the row once).
pub async fn fetch_attachments_by_ids<C: ConnectionTrait>(conn: &C, ids: &[String]) -> TxResult<Vec<AttachmentRecord>> {
    let parsed: Vec<Id> = ids.iter().filter_map(|s| s.parse::<Id>().ok()).collect();
    if parsed.is_empty() {
        return Ok(Vec::new());
    }
    let rows = AttachmentEntity::find().filter(Column::Id.is_in(parsed.clone())).all(conn).await?;
    let mut by_id: std::collections::HashMap<Id, crate::entities::platform::attachments::Model> = rows.into_iter().map(|r| (r.id, r)).collect();
    Ok(parsed.into_iter().filter_map(|id| by_id.remove(&id)).map(to_record).collect())
}

/// `putAttachment(record)` (`attachments.ts:52-62`, T: `saveAttachment`) — upsert by id: a fresh id
/// (absent, or present but not yet stored) inserts a new row; an existing id overwrites every field
/// in place, keeping `createdAt`/`createdBy` from **args** the caller sends (mirrors IndexedDB
/// `put()`'s full-record replace — the mock never partially patches). Size is re-validated
/// server-side (10MB, VALIDATION, Arabic message) regardless of what the frontend already checked.
pub async fn save_attachment<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    id: Option<String>,
    owner_ref: Option<String>,
    name: String,
    mime: String,
    kind: AttachmentKind,
    width: Option<i32>,
    height: Option<i32>,
    blob_base64: String,
    thumbnail_base64: Option<String>,
) -> TxResult<AttachmentRecord> {
    let blob = decode_base64("الملف", &blob_base64)?;
    if blob.len() > MAX_ATTACHMENT_SIZE {
        return Err(AppError::validation(format!(
            "الملف أكبر من الحد المسموح ({} ميجابايت)",
            MAX_ATTACHMENT_SIZE / 1024 / 1024
        ))
        .into());
    }
    let thumbnail = match thumbnail_base64 {
        Some(s) => Some(decode_base64("الصورة المصغّرة", &s)?),
        None => None,
    };

    // Absent id → a new one. A present id must be a UUID: re-keying it silently (the old behaviour)
    // returned success while the caller kept the id it sent in its document's `attachmentIds`, so the
    // file could never be fetched again (Part 04 Wave 2, L1). The frontend's
    // `attachmentService.newAttachmentId()` always sends a UUID on Rust.
    let resolved_id = match id.as_deref() {
        None => Id::new(),
        Some(raw) => raw.parse::<Id>().map_err(|_| AppError::validation("معرّف المرفق غير صالح"))?,
    };

    let created_by = cx.actor.as_ref().map(|a| a.id);
    let now = cx.clock.now;

    let active = ActiveModel {
        id: Set(resolved_id),
        owner_ref: Set(owner_ref),
        name: Set(name),
        mime: Set(mime),
        kind: Set(kind.into()),
        size: Set(blob.len() as i64),
        width: Set(width),
        height: Set(height),
        blob: Set(blob),
        thumbnail: Set(thumbnail),
        created_at: Set(now),
        created_by: Set(created_by),
    };

    // Upsert: MariaDB `INSERT ... ON DUPLICATE KEY UPDATE` semantics via SeaORM's `save` would
    // require an `ActiveModel` fetched from the DB to distinguish insert/update; a plain existence
    // check + insert-or-update is simpler and avoids a partial-column upsert clause listing every
    // field twice.
    let exists = AttachmentEntity::find_by_id(resolved_id).one(conn).await?.is_some();
    let saved = if exists {
        active.update(conn).await?
    } else {
        active.insert(conn).await?
    };

    cx.touch(crate::core::events::ChangeCategory::Catalog);
    Ok(to_record(saved))
}

/// `deleteAttachment(id)` (`attachments.ts:83-93`, T: `removeAttachment`) — hard delete; a missing
/// id is a no-op (the mock's IndexedDB `delete()` never errors on a missing key).
pub async fn remove_attachment<C: ConnectionTrait>(conn: &C, cx: &TxCtx, id: &str) -> TxResult<()> {
    let Ok(parsed) = id.parse::<Id>() else { return Ok(()) };
    AttachmentEntity::delete_by_id(parsed).exec(conn).await?;
    cx.touch(crate::core::events::ChangeCategory::Catalog);
    Ok(())
}
