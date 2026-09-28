//! `invoices` IPC commands (08 §1, 08b §1) — thin layer: parse args, authorize, open a transaction,
//! call the service, map the error.

use tauri::State;

use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_read_ctx, with_tx, BoxFuture, TxOpts, TxResult};

use super::dto::*;
use super::service::{access, held, quotations, reads, refund, sale, shifts};

type CmdResult<T> = Result<T, ApiErrorPayload>;

fn parse_id(raw: &str) -> Result<crate::utils::id::Id, ApiErrorPayload> {
    raw.parse::<crate::utils::id::Id>().map_err(|_| crate::core::error::AppError::validation("معرّف غير صالح").into())
}

// --- 08: reads ---------------------------------------------------------------------------------

#[tauri::command]
pub async fn invoices_get_invoices(state: State<'_, AppState>, args: InvoicesGetInvoicesArgs) -> CmdResult<Vec<InvoiceRow>> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let filter = args.filter.clone();
        Box::pin(async move {
            cx.require_any(tx, access::SALES_POS_PARTIES_READ).await?;
            let read_ctx = crate::core::tx::ReadCtx { actor: cx.actor.clone(), terminal_id: cx.terminal_id, clock: cx.clock };
            reads::get_invoices(tx, &read_ctx, filter).await
        }) as BoxFuture<'_, TxResult<Vec<InvoiceRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_get_invoices_paged(state: State<'_, AppState>, args: InvoicesGetInvoicesPagedArgs) -> CmdResult<crate::core::dto::PagedResult<InvoiceRow>> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let query = args.query.clone();
        Box::pin(async move {
            cx.require_any(tx, access::SALES_READ).await?;
            let read_ctx = crate::core::tx::ReadCtx { actor: cx.actor.clone(), terminal_id: cx.terminal_id, clock: cx.clock };
            reads::get_invoices_paged(tx, &read_ctx, query).await
        }) as BoxFuture<'_, TxResult<crate::core::dto::PagedResult<InvoiceRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_get_invoice(state: State<'_, AppState>, args: InvoicesGetInvoiceArgs) -> CmdResult<InvoiceDetail> {
    let id = parse_id(&args.id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require_any(tx, access::SALES_POS_READ).await?;
            reads::get_invoice(tx, id).await
        }) as BoxFuture<'_, TxResult<InvoiceDetail>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_preview_sale(state: State<'_, AppState>, args: InvoicesPreviewSaleArgs) -> CmdResult<Vec<JournalPreviewLine>> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        Box::pin(async move {
            cx.require_any(tx, access::SALES_POS_READ).await?;
            sale::preview_sale(tx, cx, input).await
        }) as BoxFuture<'_, TxResult<Vec<JournalPreviewLine>>>
    })
    .await
    .map_err(Into::into)
}

// --- 08: writes ----------------------------------------------------------------------------------

#[tauri::command]
pub async fn invoices_create_sale(state: State<'_, AppState>, args: InvoicesCreateSaleArgs) -> CmdResult<Invoice> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require_any(tx, access::SALES_POS_WRITE).await?;
            sale::create_sale(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<Invoice>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_create_refund(state: State<'_, AppState>, args: InvoicesCreateRefundArgs) -> CmdResult<Refund> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require_any(tx, access::SALES_POS_WRITE).await?;
            refund::create_refund(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<Refund>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_get_refund(state: State<'_, AppState>, args: InvoicesGetRefundArgs) -> CmdResult<Refund> {
    let id = parse_id(&args.id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require_any(tx, access::SALES_POS_READ).await?;
            reads::get_refund(tx, id).await
        }) as BoxFuture<'_, TxResult<Refund>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_get_invoice_print_data(state: State<'_, AppState>, args: InvoicesGetInvoicePrintDataArgs) -> CmdResult<PrintData> {
    let device = state.device.read().unwrap().clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let id = args.id.clone();
        let device = device.clone();
        Box::pin(async move {
            cx.require_any(tx, access::SALES_POS_SETTINGS_READ).await?;
            let read_ctx = crate::core::tx::ReadCtx { actor: cx.actor.clone(), terminal_id: cx.terminal_id, clock: cx.clock };
            reads::get_invoice_print_data(tx, &read_ctx, &device, &id).await
        }) as BoxFuture<'_, TxResult<PrintData>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_get_quotations(state: State<'_, AppState>, args: InvoicesGetQuotationsArgs) -> CmdResult<Vec<QuotationRow>> {
    with_read_ctx(&state, move |tx, ctx| {
        let filter = args.filter.clone();
        Box::pin(async move {
            ctx.require_any(tx, access::SALES_READ).await?;
            reads::get_quotations(tx, filter).await
        }) as BoxFuture<'_, TxResult<Vec<QuotationRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_get_quotation(state: State<'_, AppState>, args: InvoicesGetQuotationArgs) -> CmdResult<QuotationRow> {
    let id = parse_id(&args.id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require_any(tx, access::SALES_READ).await?;
            reads::get_quotation(tx, id).await
        }) as BoxFuture<'_, TxResult<QuotationRow>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_save_quotation(state: State<'_, AppState>, args: InvoicesSaveQuotationArgs) -> CmdResult<Quotation> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        Box::pin(async move {
            cx.require_any(tx, access::SALES_WRITE).await?;
            quotations::save_quotation(tx, cx, input).await
        }) as BoxFuture<'_, TxResult<Quotation>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_set_quotation_status(state: State<'_, AppState>, args: InvoicesSetQuotationStatusArgs) -> CmdResult<Quotation> {
    let id = parse_id(&args.id)?;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let status = args.status;
        Box::pin(async move {
            cx.require_any(tx, access::SALES_WRITE).await?;
            quotations::set_quotation_status(tx, id, status).await
        }) as BoxFuture<'_, TxResult<Quotation>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_convert_quotation_to_invoice(state: State<'_, AppState>, args: InvoicesConvertQuotationToInvoiceArgs) -> CmdResult<Invoice> {
    let id = parse_id(&args.id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let payment = args.payment.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require_any(tx, access::SALES_WRITE).await?;
            quotations::convert_quotation_to_invoice(tx, cx, &undo, id, payment).await
        }) as BoxFuture<'_, TxResult<Invoice>>
    })
    .await
    .map_err(Into::into)
}

// --- 08b: shifts -----------------------------------------------------------------------------

#[tauri::command]
pub async fn invoices_get_current_shift(state: State<'_, AppState>) -> CmdResult<Option<ShiftRow>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require_any(tx, access::POS_DASHBOARD_READ).await?;
            shifts::get_current_shift(tx, ctx.terminal_id).await
        }) as BoxFuture<'_, TxResult<Option<ShiftRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_get_shifts(state: State<'_, AppState>, args: InvoicesGetShiftsArgs) -> CmdResult<Vec<ShiftRow>> {
    with_read_ctx(&state, move |tx, ctx| {
        let filter = args.filter.clone().unwrap_or_default();
        Box::pin(async move {
            ctx.require_any(tx, access::POS_READ).await?;
            shifts::get_shifts(tx, filter).await
        }) as BoxFuture<'_, TxResult<Vec<ShiftRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_get_shift(state: State<'_, AppState>, args: InvoicesGetShiftArgs) -> CmdResult<ShiftRow> {
    let id = parse_id(&args.id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require_any(tx, access::POS_READ).await?;
            shifts::get_shift(tx, id).await
        }) as BoxFuture<'_, TxResult<ShiftRow>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_open_pos_shift(state: State<'_, AppState>, args: InvoicesOpenPosShiftArgs) -> CmdResult<Shift> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require_any(tx, access::POS_WRITE).await?;
            shifts::open_pos_shift(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<Shift>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_get_x_report(state: State<'_, AppState>, args: InvoicesGetXReportArgs) -> CmdResult<ShiftRow> {
    let shift_id = parse_id(&args.shift_id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require_any(tx, access::POS_READ).await?;
            shifts::get_x_report(tx, shift_id).await
        }) as BoxFuture<'_, TxResult<ShiftRow>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_close_pos_shift(state: State<'_, AppState>, args: InvoicesClosePosShiftArgs) -> CmdResult<Shift> {
    let shift_id = parse_id(&args.shift_id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require_any(tx, access::POS_WRITE).await?;
            shifts::close_pos_shift(tx, cx, &undo, shift_id, input).await
        }) as BoxFuture<'_, TxResult<Shift>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_force_close_pos_shift(state: State<'_, AppState>, args: InvoicesForceClosePosShiftArgs) -> CmdResult<Shift> {
    let shift_id = parse_id(&args.shift_id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let counted_cash = args.counted_cash;
        let undo = undo.clone();
        Box::pin(async move {
            cx.require_any(tx, access::POS_WRITE).await?;
            shifts::force_close_pos_shift(tx, cx, &undo, shift_id, counted_cash).await
        }) as BoxFuture<'_, TxResult<Shift>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_record_cash_in_out(state: State<'_, AppState>, args: InvoicesRecordCashInOutArgs) -> CmdResult<()> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let kind = args.kind;
        let amount = args.amount;
        let note = args.note.clone();
        Box::pin(async move {
            cx.require_any(tx, access::POS_WRITE).await?;
            shifts::record_cash_in_out(tx, cx, kind, amount, note).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(Into::into)
}

// --- 08b: held sales -------------------------------------------------------------------------

#[tauri::command]
pub async fn invoices_get_held_sales(state: State<'_, AppState>) -> CmdResult<Vec<HeldSale>> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require_any(tx, access::POS_READ).await?;
            held::get_held_sales(tx, ctx.terminal_id).await
        }) as BoxFuture<'_, TxResult<Vec<HeldSale>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_hold_sale(state: State<'_, AppState>, args: InvoicesHoldSaleArgs) -> CmdResult<HeldSale> {
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        Box::pin(async move {
            cx.require_any(tx, access::POS_WRITE).await?;
            held::hold_sale(tx, cx, input).await
        }) as BoxFuture<'_, TxResult<HeldSale>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_resume_held_sale(state: State<'_, AppState>, args: InvoicesResumeHeldSaleArgs) -> CmdResult<HeldSale> {
    let id = parse_id(&args.id)?;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require_any(tx, access::POS_WRITE).await?;
            held::resume_held_sale(tx, id).await
        }) as BoxFuture<'_, TxResult<HeldSale>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn invoices_discard_held_sale(state: State<'_, AppState>, args: InvoicesDiscardHeldSaleArgs) -> CmdResult<()> {
    let id = parse_id(&args.id)?;
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        Box::pin(async move {
            cx.require_any(tx, access::POS_WRITE).await?;
            held::discard_held_sale(tx, id).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(Into::into)
}
