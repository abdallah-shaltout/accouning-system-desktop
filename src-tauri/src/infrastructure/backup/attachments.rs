//! Attachment blobs in a backup archive (C-16, 17-backup.md §3.1) — packs/restores the
//! `attachments` table's rows as `attachments/<id>.{blob,json,thumb}` zip payload entries, the same
//! layout `backupArchive.ts` already used for its IndexedDB store. Kept separate from
//! `dataset.rs`'s generic table dump because `attachments.blob`/`.thumbnail` are `LONGBLOB` columns,
//! which that dumper's `CAST(... AS CHAR)` approach cannot carry (see `dataset.rs`'s
//! `EXCLUDED_TABLES` note).
//!
//! `<id>.json` holds everything but the bytes (`AttachmentJsonMeta`, camelCase to match the
//! `AttachmentMeta` shape already on the wire); `<id>.blob` is the raw file bytes; `<id>.thumb` is
//! the optional WebP thumbnail's raw bytes. An archive with no `attachments/` entries at all (every
//! archive written before C-16, or the mock's own browser archives) simply restores zero
//! attachments — old backups stay fully restorable.

use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};

use crate::core::error::AppError;
use crate::core::tx::TxResult;
use crate::entities::platform::attachments::{ActiveModel, AttachmentKind, Entity as AttachmentEntity};
use crate::utils::dates::format_iso_ms;
use crate::utils::id::Id;

use super::archive::PayloadFiles;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttachmentJsonMeta {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_ref: Option<String>,
    name: String,
    mime: String,
    kind: String,
    size: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<i32>,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_by: Option<String>,
}

fn kind_to_str(kind: AttachmentKind) -> &'static str {
    match kind {
        AttachmentKind::Image => "image",
        AttachmentKind::Pdf => "pdf",
        AttachmentKind::Office => "office",
        AttachmentKind::Other => "other",
    }
}

fn kind_from_str(s: &str) -> AttachmentKind {
    match s {
        "image" => AttachmentKind::Image,
        "pdf" => AttachmentKind::Pdf,
        "office" => AttachmentKind::Office,
        _ => AttachmentKind::Other,
    }
}

/// Dumps every row of the `attachments` table as `attachments/<id>.{blob,json,thumb}` payload
/// entries, appended to `out` in id order (deterministic — matches `dataset.rs`'s own ordering
/// discipline for the checksum's sorted-name concat to still be stable run-to-run for unchanged
/// data).
pub async fn pack_attachments<C: ConnectionTrait>(conn: &C, out: &mut PayloadFiles) -> TxResult<()> {
    use sea_orm::QueryOrder;
    let rows = AttachmentEntity::find().order_by_asc(crate::entities::platform::attachments::Column::Id).all(conn).await?;

    for row in rows {
        let id = row.id.to_string();
        let meta = AttachmentJsonMeta {
            id: id.clone(),
            owner_ref: row.owner_ref.clone(),
            name: row.name.clone(),
            mime: row.mime.clone(),
            kind: kind_to_str(row.kind).to_string(),
            size: row.size,
            width: row.width,
            height: row.height,
            created_at: format_iso_ms(row.created_at),
            created_by: row.created_by.map(|i| i.to_string()),
        };
        let meta_json = serde_json::to_vec(&meta)
            .map_err(|e| AppError::internal("تعذر بناء النسخة الاحتياطية", Some(e.to_string())))?;

        out.push((format!("attachments/{id}.json"), meta_json));
        out.push((format!("attachments/{id}.blob"), row.blob.clone()));
        if let Some(thumb) = &row.thumbnail {
            out.push((format!("attachments/{id}.thumb"), thumb.clone()));
        }
    }

    Ok(())
}

/// Restores every `attachments/<id>.json` (+ matching `.blob`/`.thumb`) entry found in `files` into
/// the `attachments` table — called from `restore.rs`'s Rust-archive branch, right after
/// `dataset::wipe`+`dataset::load` (which never touch `attachments` since it's in
/// `dataset::EXCLUDED_TABLES`, so this is the one path that repopulates it). Skips any id whose
/// `.blob` entry is missing (a malformed/partial archive) rather than failing the whole restore —
/// matches the generic dumper's own "a table with zero archived rows may be absent" leniency.
pub async fn restore_attachments<C: ConnectionTrait>(conn: &C, files: &PayloadFiles) -> TxResult<()> {
    // Existing rows are gone by the time this runs (a full wipe/load restore, C-16's contract): no
    // separate `DELETE FROM attachments` needed here, but issue one defensively in case this is ever
    // called outside that flow (idempotent - safe to restore into an already-empty table twice).
    AttachmentEntity::delete_many().exec(conn).await?;

    for (name, bytes) in files {
        let Some(id_str) = name.strip_prefix("attachments/").and_then(|s| s.strip_suffix(".json")) else { continue };
        let Ok(id) = id_str.parse::<Id>() else { continue };

        let meta: AttachmentJsonMeta = match serde_json::from_slice(bytes) {
            Ok(m) => m,
            Err(_) => continue, // malformed meta entry — skip this one attachment, not the whole restore.
        };

        let Some((_, blob)) = files.iter().find(|(n, _)| n == &format!("attachments/{id_str}.blob")) else { continue };
        let thumbnail = files.iter().find(|(n, _)| n == &format!("attachments/{id_str}.thumb")).map(|(_, b)| b.clone());

        let created_at = chrono::DateTime::parse_from_rfc3339(&meta.created_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        let created_by = meta.created_by.as_deref().and_then(|s| s.parse::<Id>().ok());

        let active = ActiveModel {
            id: Set(id),
            owner_ref: Set(meta.owner_ref),
            name: Set(meta.name),
            mime: Set(meta.mime),
            kind: Set(kind_from_str(&meta.kind)),
            size: Set(meta.size),
            width: Set(meta.width),
            height: Set(meta.height),
            blob: Set(blob.clone()),
            thumbnail: Set(thumbnail),
            created_at: Set(created_at),
            created_by: Set(created_by),
        };
        active.insert(conn).await?;
    }

    Ok(())
}
