//! `products` inventory IPC commands (06b-inventory.md §1) — stock adjustments, movements,
//! batches/expiry, debit-note drafts, stock counts. Thin layer: parse args, authorize (area
//! `inventory` for every command per §1), open a transaction, call the service, map the error.
//! Reads use `with_read_ctx`; writes use `with_tx`. `createStockAdjustment`/`completeAdjustment`
//! build `ApprovalCheck` from `state.approval_grants` (G-P3, manager-owned — see "Needs from
//! manager" in this wave's final report) BEFORE `with_tx`, per S-4's doc comment: the grant lookup
//! reads `AppState` directly and can't happen inside the `Fn` closure `with_tx` may retry.

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_read_ctx, with_tx, BoxFuture, TxOpts, TxResult};
use crate::utils::id::Id;

use super::super::dto::inventory::*;
use super::super::service::adjustments::{self, ApprovalCheck};
use super::super::service::batches;
use super::super::service::counts;
use super::super::service::movements;

type CmdResult<T> = Result<T, ApiErrorPayload>;

fn parse_id(raw: &str) -> Result<Id, ApiErrorPayload> {
    raw.parse::<Id>().map_err(|_| crate::core::error::AppError::validation("معرّف غير صالح").into())
}

fn parse_ids(raw: &[String]) -> Result<Vec<Id>, ApiErrorPayload> {
    raw.iter().map(|s| parse_id(s)).collect()
}

// --- Stock adjustments --------------------------------------------------------------------------

#[tauri::command]
pub async fn products_get_stock_adjustments(state: State<'_, AppState>, args: ProductsGetStockAdjustmentsArgs) -> CmdResult<Vec<StockAdjustment>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            adjustments::get_stock_adjustments(tx, args.filter).await
        }) as BoxFuture<'_, TxResult<Vec<StockAdjustment>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_get_stock_adjustment(state: State<'_, AppState>, args: ProductsGetStockAdjustmentArgs) -> CmdResult<StockAdjustmentDetail> {
    let id = parse_id(&args.id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            adjustments::get_stock_adjustment(tx, id).await
        }) as BoxFuture<'_, TxResult<StockAdjustmentDetail>>
    })
    .await
    .map_err(Into::into)
}

/// S-4's `ApprovalCheck` is computed here, before `with_tx`, from `state.approval_grants`
/// (G-P3 — see this wave's "Needs from manager" note): `AppState` isn't reachable from inside the
/// `Fn` closure `with_tx` may retry on deadlock, so the grant snapshot must be read up front.
fn approval_check(state: &AppState, approved_by: Option<Id>) -> ApprovalCheck {
    match approved_by {
        Some(id) => ApprovalCheck { granted: state.approval_grants.is_granted(id) },
        None => ApprovalCheck::none(),
    }
}

#[tauri::command]
pub async fn products_create_stock_adjustment(state: State<'_, AppState>, args: ProductsCreateStockAdjustmentArgs) -> CmdResult<StockAdjustment> {
    let undo = state.undo.clone();
    let approval = approval_check(&state, args.input.approved_by);
    let as_draft = args.as_draft.unwrap_or(false);
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        let approval = ApprovalCheck { granted: approval.granted };
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            adjustments::record_stock_adjustment(tx, cx, &undo, input, as_draft, approval).await
        }) as BoxFuture<'_, TxResult<StockAdjustment>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_complete_adjustment(state: State<'_, AppState>, args: ProductsCompleteAdjustmentArgs) -> CmdResult<StockAdjustment> {
    let id = parse_id(&args.id)?;
    let undo = state.undo.clone();
    let approved_by = args.approved_by;
    let approval = approval_check(&state, approved_by);
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        let approval = ApprovalCheck { granted: approval.granted };
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            adjustments::complete_adjustment(tx, cx, &undo, id, approved_by, approval).await
        }) as BoxFuture<'_, TxResult<StockAdjustment>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_delete_draft_adjustment(state: State<'_, AppState>, args: ProductsDeleteDraftAdjustmentArgs) -> CmdResult<()> {
    let id = parse_id(&args.id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            adjustments::delete_draft_adjustment(tx, cx, &undo, id).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(Into::into)
}

// --- Stock movements -----------------------------------------------------------------------------

#[tauri::command]
pub async fn products_get_stock_movements(state: State<'_, AppState>, args: ProductsGetStockMovementsArgs) -> CmdResult<Vec<StockMovementRow>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            movements::get_stock_movements(tx, args.filter).await
        }) as BoxFuture<'_, TxResult<Vec<StockMovementRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_get_stock_movements_paged(
    state: State<'_, AppState>,
    args: ProductsGetStockMovementsPagedArgs,
) -> CmdResult<crate::core::dto::PagedResult<StockMovementRow>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            movements::get_stock_movements_paged(tx, args.query).await
        }) as BoxFuture<'_, TxResult<crate::core::dto::PagedResult<StockMovementRow>>>
    })
    .await
    .map_err(Into::into)
}

// --- Batches / expiry ----------------------------------------------------------------------------

#[tauri::command]
pub async fn products_get_batches(state: State<'_, AppState>, args: ProductsGetBatchesArgs) -> CmdResult<Vec<ProductBatch>> {
    let product_id = parse_id(&args.product_id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            batches::get_batches(tx, product_id).await
        }) as BoxFuture<'_, TxResult<Vec<ProductBatch>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_get_expiry_report(state: State<'_, AppState>) -> CmdResult<Vec<ExpiryRow>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            batches::get_expiry_report(tx).await
        }) as BoxFuture<'_, TxResult<Vec<ExpiryRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_write_off_expired_batches(state: State<'_, AppState>, args: ProductsWriteOffExpiredBatchesArgs) -> CmdResult<StockAdjustment> {
    let batch_ids = parse_ids(&args.batch_ids)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let batch_ids = batch_ids.clone();
        let note = args.note.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            batches::write_off_expired_batches(tx, cx, &undo, batch_ids, note).await
        }) as BoxFuture<'_, TxResult<StockAdjustment>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_return_batches_to_supplier(state: State<'_, AppState>, args: ProductsReturnBatchesToSupplierArgs) -> CmdResult<DebitNoteDraft> {
    let supplier_id = parse_id(&args.supplier_id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let lines: Vec<DebitNoteDraftLineInput> = args.lines.clone();
        let note = args.note.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            batches::return_batches_to_supplier(tx, cx, &undo, supplier_id, lines, note).await
        }) as BoxFuture<'_, TxResult<DebitNoteDraft>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_get_debit_note_drafts(state: State<'_, AppState>) -> CmdResult<Vec<DebitNoteDraft>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            batches::get_debit_note_drafts(tx).await
        }) as BoxFuture<'_, TxResult<Vec<DebitNoteDraft>>>
    })
    .await
    .map_err(Into::into)
}

// --- Stock counts --------------------------------------------------------------------------------

#[tauri::command]
pub async fn products_get_stock_counts(state: State<'_, AppState>) -> CmdResult<Vec<StockCount>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            counts::get_stock_counts(tx).await
        }) as BoxFuture<'_, TxResult<Vec<StockCount>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_get_stock_count(state: State<'_, AppState>, args: ProductsGetStockCountArgs) -> CmdResult<StockCount> {
    let id = parse_id(&args.id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Inventory, Access::Read).await?;
            counts::get_stock_count(tx, id).await
        }) as BoxFuture<'_, TxResult<StockCount>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_create_stock_count(state: State<'_, AppState>, args: ProductsCreateStockCountArgs) -> CmdResult<StockCount> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            counts::create_stock_count(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<StockCount>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_update_stock_count_line(state: State<'_, AppState>, args: ProductsUpdateStockCountLineArgs) -> CmdResult<StockCount> {
    let count_id = parse_id(&args.count_id)?;
    let product_id = parse_id(&args.product_id)?;
    let qty = args.qty;
    let delta = args.delta.unwrap_or(false);
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            counts::update_stock_count_line(tx, count_id, product_id, qty, delta).await
        }) as BoxFuture<'_, TxResult<StockCount>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_submit_count_for_review(state: State<'_, AppState>, args: ProductsCountIdArgs) -> CmdResult<StockCount> {
    let count_id = parse_id(&args.count_id)?;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            counts::submit_count_for_review(tx, count_id).await
        }) as BoxFuture<'_, TxResult<StockCount>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_resume_counting(state: State<'_, AppState>, args: ProductsCountIdArgs) -> CmdResult<StockCount> {
    let count_id = parse_id(&args.count_id)?;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            counts::resume_counting(tx, count_id).await
        }) as BoxFuture<'_, TxResult<StockCount>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn products_complete_stock_count(state: State<'_, AppState>, args: ProductsCountIdArgs) -> CmdResult<StockAdjustment> {
    let count_id = parse_id(&args.count_id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Inventory, Access::Write).await?;
            counts::complete_stock_count(tx, cx, &undo, count_id).await
        }) as BoxFuture<'_, TxResult<StockAdjustment>>
    })
    .await
    .map_err(Into::into)
}
