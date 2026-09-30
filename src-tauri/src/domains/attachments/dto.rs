//! `attachments` domain DTOs (C-16) — mirrors `src/modules/core/services/attachmentService.ts`'s
//! re-exported `AttachmentKind`/`AttachmentMeta` shapes (from `src/mocks/attachments.ts`). Blob
//! bytes cross IPC as standard base64 strings (the `pdf_base64`/`archive_base64` precedent), not
//! raw `Vec<u8>` — `tauri::command` args/returns are JSON, so a byte array would otherwise become a
//! very large JSON number array. `#[ts(export_to = "core/types/gen/")]`: `core` is the module that
//! owns `attachmentService.ts` (AGENT_MEMORY.md's Service API table).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::utils::id::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub enum AttachmentKind {
    Image,
    Pdf,
    Office,
    Other,
}

impl From<AttachmentKind> for crate::entities::platform::attachments::AttachmentKind {
    fn from(k: AttachmentKind) -> Self {
        use crate::entities::platform::attachments::AttachmentKind as E;
        match k {
            AttachmentKind::Image => E::Image,
            AttachmentKind::Pdf => E::Pdf,
            AttachmentKind::Office => E::Office,
            AttachmentKind::Other => E::Other,
        }
    }
}

impl From<crate::entities::platform::attachments::AttachmentKind> for AttachmentKind {
    fn from(k: crate::entities::platform::attachments::AttachmentKind) -> Self {
        use crate::entities::platform::attachments::AttachmentKind as E;
        match k {
            E::Image => AttachmentKind::Image,
            E::Pdf => AttachmentKind::Pdf,
            E::Office => AttachmentKind::Office,
            E::Other => AttachmentKind::Other,
        }
    }
}

/// `AttachmentMeta` (`src/mocks/attachments.ts`) — every field but the blob/thumbnail bytes.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct AttachmentMeta {
    #[ts(type = "string")]
    pub id: Id,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_ref: Option<String>,
    pub name: String,
    pub mime: String,
    pub kind: AttachmentKind,
    pub size: i32,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    pub created_at: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
}

/// `AttachmentRecord` (`src/mocks/attachments.ts`) — `AttachmentMeta` plus the blob/thumbnail bytes,
/// standard base64 (T-note above).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct AttachmentRecord {
    #[ts(type = "string")]
    pub id: Id,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_ref: Option<String>,
    pub name: String,
    pub mime: String,
    pub kind: AttachmentKind,
    pub size: i32,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    pub created_at: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    /// Standard base64 of the stored (possibly resized/re-encoded) file's bytes.
    pub blob_base64: String,
    /// Standard base64 of the small WebP preview, images only.
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_base64: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct AttachmentsFetchAttachmentsArgs {
    pub owner_ref: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct AttachmentsFetchAttachmentArgs {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct AttachmentsFetchAttachmentsByIdsArgs {
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct AttachmentsSaveAttachmentArgs {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_ref: Option<String>,
    pub name: String,
    pub mime: String,
    pub kind: AttachmentKind,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    pub blob_base64: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_base64: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core/types/gen/")]
pub struct AttachmentsRemoveAttachmentArgs {
    pub id: String,
}
