//! `attachments` entity (C-16; migration m0017). Attachment blobs, moved from the browser's
//! per-device IndexedDB (`src/mocks/attachments.ts`) into MariaDB so every terminal sharing the
//! Main-PC DB (D8) sees a file attached on another terminal, and so backups can include it.
//! Source: `AttachmentRecord`/`AttachmentMeta` in `src/mocks/attachments.ts`.
//!
//! Not a soft-delete table (the mock's IndexedDB store had no soft delete either — `removeAttachment`
//! is a hard delete). Not branch-scoped — `owner_ref` alone identifies what a file is attached to.
//! `blob`/`thumbnail` are `LONGBLOB`: **excluded** from `infrastructure::backup::dataset`'s generic
//! table dump (a `BLOB` column anywhere makes that dumper `INTERNAL`) — the backup archive builder
//! packs/restores this table's rows as `attachments/<id>.{blob,json,thumb}` zip entries instead, the
//! same layout `backupArchive.ts` already uses for its IndexedDB store.

use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "kind")]
pub enum AttachmentKind {
    #[sea_orm(string_value = "image")]
    Image,
    #[sea_orm(string_value = "pdf")]
    Pdf,
    #[sea_orm(string_value = "office")]
    Office,
    #[sea_orm(string_value = "other")]
    Other,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "attachments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    #[sea_orm(nullable)]
    pub owner_ref: Option<String>,
    pub name: String,
    pub mime: String,
    pub kind: AttachmentKind,
    /// Bytes of the stored (possibly resized/re-encoded) file — mirrors `AttachmentMeta.size`.
    pub size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    #[sea_orm(column_type = "custom(\"LONGBLOB\")")]
    pub blob: Vec<u8>,
    /// Small WebP preview for images only.
    #[sea_orm(column_type = "custom(\"LONGBLOB\")", nullable)]
    pub thumbnail: Option<Vec<u8>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub created_by: Option<Id>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
