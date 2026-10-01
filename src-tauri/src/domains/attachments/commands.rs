//! `attachments` IPC commands (C-16) — thin layer: parse args, open a transaction, call the
//! service, map the error. Reads have no `Area` gate (the mock's `attachmentService.ts` has none
//! either — any logged-in user/terminal can view an attachment already reachable through its owning
//! document) beyond a session via `with_tx`'s default `TxOpts` (`require_user: true`). Writes
//! (`save`/`remove`) require write access to at least one area that actually attaches files today
//! (`Sales` — invoices, `Parties`, `Inventory` — products), via `cx.require_any`: an attachment's
//! `owner_ref` names an invoice, a party or a product, never a bare "attachments" permission, so the
//! gate mirrors the real document types instead of inventing a standalone one (found in the plan 21
//! Part 04 E-6 audit — the module previously claimed parity with `domains::templates::commands`,
//! which is actually gated, so that precedent didn't hold).

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_tx, TxOpts};

/// Every area that owns an attachable document type today (invoices, parties, products) — kept as
/// one list so a new attachable domain only has to add itself here.
const ATTACHMENT_WRITE_AREAS: &[(Area, Access)] = &[(Area::Sales, Access::Write), (Area::Parties, Access::Write), (Area::Inventory, Access::Write)];

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
            cx.require_any(tx, ATTACHMENT_WRITE_AREAS).await?;
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
        Box::pin(async move {
            cx.require_any(tx, ATTACHMENT_WRITE_AREAS).await?;
            service::remove_attachment(tx, cx, &id).await
        })
    })
    .await
    .map_err(Into::into)
}
