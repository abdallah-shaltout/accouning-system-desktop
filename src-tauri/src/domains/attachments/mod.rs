//! `attachments` domain (C-16, 21.02-B follow-up) — attachment blob storage, moved from the
//! browser's per-device IndexedDB (`src/mocks/attachments.ts`) into MariaDB so every terminal
//! sharing the Main-PC DB (D8) sees a file attached on another terminal, and so backups include it.
//! No undo compensator (attachments are files a user picked, not a ledger effect — same reasoning
//! as `templates`' "no undo compensator, design metadata").

pub mod commands;
pub mod dto;
pub mod service;

use crate::core::ipc::IpcSig;
use crate::ipc_sig;

pub fn ipc_signatures() -> Vec<IpcSig> {
    vec![
        ipc_sig!(attachments_fetch_attachments, dto::AttachmentsFetchAttachmentsArgs, Vec<dto::AttachmentMeta>),
        ipc_sig!(attachments_fetch_attachment, dto::AttachmentsFetchAttachmentArgs, Option<dto::AttachmentRecord>),
        ipc_sig!(attachments_fetch_attachments_by_ids, dto::AttachmentsFetchAttachmentsByIdsArgs, Vec<dto::AttachmentRecord>),
        ipc_sig!(attachments_save_attachment, dto::AttachmentsSaveAttachmentArgs, dto::AttachmentRecord),
        ipc_sig!(attachments_remove_attachment, dto::AttachmentsRemoveAttachmentArgs, ()),
    ]
}

pub fn export_bindings(cfg: &ts_rs::Config) {
    use ts_rs::TS;
    dto::AttachmentKind::export_all(cfg).expect("export AttachmentKind");
    dto::AttachmentMeta::export_all(cfg).expect("export AttachmentMeta");
    dto::AttachmentRecord::export_all(cfg).expect("export AttachmentRecord");
    dto::AttachmentsFetchAttachmentsArgs::export_all(cfg).expect("export AttachmentsFetchAttachmentsArgs");
    dto::AttachmentsFetchAttachmentArgs::export_all(cfg).expect("export AttachmentsFetchAttachmentArgs");
    dto::AttachmentsFetchAttachmentsByIdsArgs::export_all(cfg).expect("export AttachmentsFetchAttachmentsByIdsArgs");
    dto::AttachmentsSaveAttachmentArgs::export_all(cfg).expect("export AttachmentsSaveAttachmentArgs");
    dto::AttachmentsRemoveAttachmentArgs::export_all(cfg).expect("export AttachmentsRemoveAttachmentArgs");
}
