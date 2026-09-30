//! `attachments` IPC commands (C-16) — thin layer: parse args, open a transaction, call the
//! service, map the error. No `Area` gate (the mock's `attachmentService.ts` has none either — any
//! logged-in user/terminal can read or write an attachment, same as picking a file in
//! `AttachmentField`) — every command still requires a session via `with_tx`'s default `TxOpts`
//! (`require_user: true`), reads included, same reasoning as `domains::templates::commands`.

use tauri::State;

use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_tx, TxOpts};

use super::dto::{
    AttachmentMeta, AttachmentRecord, AttachmentsFetchAttachmentArgs, AttachmentsFetchAttachmentsArgs, AttachmentsFetchAttachmentsByIdsArgs,
    AttachmentsRemoveAttachmentArgs, AttachmentsSaveAttachmentArgs,
};
use super::service;

#[tauri::command]
pub async fn attachments_fetch_attachments(state: State<'_, AppState>, args: AttachmentsFetchAttachmentsArgs) -> Result<Vec<AttachmentMeta>, ApiErrorPayload> {
    with_tx(&state, TxOpts::default(), move |tx, _cx| {
        let owner_ref = args.owner_ref.clone();
        Box::pin(async move { service::fetch_attachments(tx, &owner_ref).await })
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn attachments_fetch_attachment(state: State<'_, AppState>, args: AttachmentsFetchAttachmentArgs) -> Result<Option<AttachmentRecord>, ApiErrorPayload> {
    with_tx(&state, TxOpts::default(), move |tx, _cx| {
        let id = args.id.clone();
        Box::pin(async move { service::fetch_attachment(tx, &id).await })
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn attachments_fetch_attachments_by_ids(
    state: State<'_, AppState>,
    args: AttachmentsFetchAttachmentsByIdsArgs,
) -> Result<Vec<AttachmentRecord>, ApiErrorPayload> {
    with_tx(&state, TxOpts::default(), move |tx, _cx| {
        let ids = args.ids.clone();
        Box::pin(async move { service::fetch_attachments_by_ids(tx, &ids).await })
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn attachments_save_attachment(state: State<'_, AppState>, args: AttachmentsSaveAttachmentArgs) -> Result<AttachmentRecord, ApiErrorPayload> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let args = args.clone();
        Box::pin(async move {
            service::save_attachment(
                tx,
                cx,
                args.id,
                args.owner_ref,
                args.name,
                args.mime,
                args.kind,
                args.width,
                args.height,
                args.blob_base64,
                args.thumbnail_base64,
            )
            .await
        })
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn attachments_remove_attachment(state: State<'_, AppState>, args: AttachmentsRemoveAttachmentArgs) -> Result<(), ApiErrorPayload> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let id = args.id.clone();
        Box::pin(async move { service::remove_attachment(tx, cx, &id).await })
    })
    .await
    .map_err(Into::into)
}
