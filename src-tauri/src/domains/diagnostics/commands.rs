//! `diagnostics` IPC commands (plan 21 Part 03 §16). Slice A (W1, top of file): the 3 production
//! commands, all read-only (`with_read_ctx` — G-32): one REPEATABLE READ snapshot per call, no
//! writes, no events, no audit rows (spec §4/§5/§6). Slice B (bottom of file): the 7 debug-build-only
//! accounting-debugger reads — every one of them starts with the same `debug_only()` gate (D-4:
//! `if !cfg!(debug_assertions) { return Err(FORBIDDEN) }`, kept as a **runtime** check rather than
//! `#[cfg(debug_assertions)]` on the function itself, so the command is registered in every build and
//! `ipc_manifest_matches_handler`'s one handler list never has to branch by build profile — see the
//! spec's D-4 and this implementer's "Needs from manager" note on `lib.rs` registration). Every
//! command here returns `Result<T, ApiErrorPayload>` per entry-file §3.2.

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::{ApiErrorPayload, AuditEntry};
use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::core::tx::{with_read_ctx, BoxFuture, TxResult};
use crate::domains::accounting::dto::JournalEntry;
use crate::domains::diagnostics::dto::{
    AccountingDocSummary, BalanceAround, DiagnosticsExplainAccountBalanceArgs, DiagnosticsExportSupportBundleArgs,
    DiagnosticsGetAuditEntriesArgs, DiagnosticsGetBalancesAroundArgs, DiagnosticsGetJournalEntryRawArgs, DiagnosticsGetPostingTraceArgs,
    DiagnosticsListRecentDocumentsArgs, DriftRow, ExplainLine, InvariantResultDto, PostingTraceDto, SupportSnapshot,
};
use crate::domains::diagnostics::service::{audit, debugger, support};

/// D-4: the one runtime gate every slice-B command opens with. Kept as a free function (not a
/// macro) so it reads identically at each of the 7 call sites and shows up once in a stack trace.
fn debug_only() -> Result<(), AppError> {
    if !cfg!(debug_assertions) {
        return Err(AppError::forbidden("أداة التشخيص المحاسبي متاحة في نسخة التطوير فقط"));
    }
    Ok(())
}

/// `diagnostics_get_audit_entries` (spec §1) — Users:Read, matching the audit-log page's area.
#[tauri::command]
pub async fn diagnostics_get_audit_entries(
    state: State<'_, AppState>,
    args: DiagnosticsGetAuditEntriesArgs,
) -> Result<Vec<AuditEntry>, ApiErrorPayload> {
    let filter = args.filter.unwrap_or_default();
    with_read_ctx(&state, move |conn, ctx| {
        Box::pin(async move {
            audit::require_audit_read(conn, ctx).await?;
            audit::get_audit_entries(conn, &filter).await
        }) as BoxFuture<'_, TxResult<Vec<AuditEntry>>>
    })
    .await
    .map_err(Into::into)
}

/// `diagnostics_get_audit_entities` (spec §1, also serves the `getAuditEntityKinds` alias — same
/// command, no second registration needed) — Users:Read.
#[tauri::command]
pub async fn diagnostics_get_audit_entities(state: State<'_, AppState>) -> Result<Vec<String>, ApiErrorPayload> {
    with_read_ctx(&state, |conn, ctx| {
        Box::pin(async move {
            audit::require_audit_read(conn, ctx).await?;
            audit::get_audit_entities(conn).await
        }) as BoxFuture<'_, TxResult<Vec<String>>>
    })
    .await
    .map_err(Into::into)
}

/// `diagnostics_export_support_bundle` (spec §1, D-1 — data part only): any signed-in user (D-2)
/// may call it; `includeDbSnapshot: true` additionally needs Settings:Write, checked inside
/// `support::support_snapshot` itself (so the exact same gate applies regardless of which other
/// checks a future slice adds around it).
#[tauri::command]
pub async fn diagnostics_export_support_bundle(
    state: State<'_, AppState>,
    args: DiagnosticsExportSupportBundleArgs,
) -> Result<SupportSnapshot, ApiErrorPayload> {
    let include_db_snapshot = args.include_db_snapshot.unwrap_or(false);
    let device = state.device.read().unwrap().clone();
    let server = support::server_diagnostics(&state);
    with_read_ctx(&state, move |conn, ctx| {
        Box::pin(async move { support::support_snapshot(&device, server, conn, ctx, include_db_snapshot).await }) as BoxFuture<'_, TxResult<SupportSnapshot>>
    })
    .await
    .map_err(Into::into)
}

// -------------------------------------------------------------------------------------------
// Slice B — debug-build-only accounting-debugger reads (spec §1/§9). Every command below opens
// with `debug_only()` (D-4) before the `with_read_ctx` snapshot even starts, so a release build
// never touches the DB for these at all; inside the snapshot, `ctx.require(Accounting, Read)` is
// the actual access gate (a debug build with no accounting-read session still gets `FORBIDDEN`,
// not the tool silently returning data).
// -------------------------------------------------------------------------------------------

/// `diagnostics_list_recent_documents` (spec §1) — debug + Accounting:Read.
#[tauri::command]
pub async fn diagnostics_list_recent_documents(
    state: State<'_, AppState>,
    args: DiagnosticsListRecentDocumentsArgs,
) -> Result<Vec<AccountingDocSummary>, ApiErrorPayload> {
    debug_only()?;
    let limit = args.limit.unwrap_or(100);
    let traces = state.traces.clone();
    with_read_ctx(&state, move |conn, ctx| {
        Box::pin(async move {
            ctx.require(conn, Area::Accounting, Access::Read).await?;
            debugger::list_recent_documents(conn, &traces, limit).await
        }) as BoxFuture<'_, TxResult<Vec<AccountingDocSummary>>>
    })
    .await
    .map_err(Into::into)
}

/// `diagnostics_get_posting_trace` (spec §1) — debug + Accounting:Read. Reads only the in-memory
/// ring (`AppState.traces`), never the DB, but still opens a `with_read_ctx` snapshot so the access
/// check runs the same way as every other command here.
#[tauri::command]
pub async fn diagnostics_get_posting_trace(
    state: State<'_, AppState>,
    args: DiagnosticsGetPostingTraceArgs,
) -> Result<Option<PostingTraceDto>, ApiErrorPayload> {
    debug_only()?;
    let traces = state.traces.clone();
    let entry_id = args.entry_id;
    with_read_ctx(&state, move |conn, ctx| {
        Box::pin(async move {
            ctx.require(conn, Area::Accounting, Access::Read).await?;
            Ok(debugger::get_posting_trace(&traces, entry_id).as_ref().map(Into::into))
        }) as BoxFuture<'_, TxResult<Option<PostingTraceDto>>>
    })
    .await
    .map_err(Into::into)
}

/// `diagnostics_get_journal_entry_raw` (spec §1) — debug + Accounting:Read.
#[tauri::command]
pub async fn diagnostics_get_journal_entry_raw(
    state: State<'_, AppState>,
    args: DiagnosticsGetJournalEntryRawArgs,
) -> Result<Option<JournalEntry>, ApiErrorPayload> {
    debug_only()?;
    let id = args.id;
    with_read_ctx(&state, move |conn, ctx| {
        Box::pin(async move {
            ctx.require(conn, Area::Accounting, Access::Read).await?;
            debugger::get_journal_entry_raw(conn, id).await
        }) as BoxFuture<'_, TxResult<Option<JournalEntry>>>
    })
    .await
    .map_err(Into::into)
}

/// `diagnostics_get_balances_around` (spec §1) — debug + Accounting:Read.
#[tauri::command]
pub async fn diagnostics_get_balances_around(
    state: State<'_, AppState>,
    args: DiagnosticsGetBalancesAroundArgs,
) -> Result<Vec<BalanceAround>, ApiErrorPayload> {
    debug_only()?;
    let entry_id = args.entry_id;
    with_read_ctx(&state, move |conn, ctx| {
        Box::pin(async move {
            ctx.require(conn, Area::Accounting, Access::Read).await?;
            debugger::get_balances_around(conn, entry_id).await
        }) as BoxFuture<'_, TxResult<Vec<BalanceAround>>>
    })
    .await
    .map_err(Into::into)
}

/// `diagnostics_get_invariant_results` (spec §1) — debug + Accounting:Read; the RR snapshot doubles
/// as the "one consistent point in time" `shared::invariants::run_all` needs (spec §4).
#[tauri::command]
pub async fn diagnostics_get_invariant_results(state: State<'_, AppState>) -> Result<Vec<InvariantResultDto>, ApiErrorPayload> {
    debug_only()?;
    with_read_ctx(&state, |conn, ctx| {
        Box::pin(async move {
            ctx.require(conn, Area::Accounting, Access::Read).await?;
            debugger::get_invariant_results(conn).await
        }) as BoxFuture<'_, TxResult<Vec<InvariantResultDto>>>
    })
    .await
    .map_err(Into::into)
}

/// `diagnostics_get_drift_report` (spec §1) — debug + Accounting:Read.
#[tauri::command]
pub async fn diagnostics_get_drift_report(state: State<'_, AppState>) -> Result<Vec<DriftRow>, ApiErrorPayload> {
    debug_only()?;
    with_read_ctx(&state, |conn, ctx| {
        Box::pin(async move {
            ctx.require(conn, Area::Accounting, Access::Read).await?;
            debugger::get_drift_report(conn).await
        }) as BoxFuture<'_, TxResult<Vec<DriftRow>>>
    })
    .await
    .map_err(Into::into)
}

/// `diagnostics_explain_account_balance` (spec §1) — debug + Accounting:Read.
#[tauri::command]
pub async fn diagnostics_explain_account_balance(
    state: State<'_, AppState>,
    args: DiagnosticsExplainAccountBalanceArgs,
) -> Result<Vec<ExplainLine>, ApiErrorPayload> {
    debug_only()?;
    let account_id = args.account_id;
    let party_id = args.party_id;
    with_read_ctx(&state, move |conn, ctx| {
        Box::pin(async move {
            ctx.require(conn, Area::Accounting, Access::Read).await?;
            debugger::explain_account_balance(conn, account_id, party_id).await
        }) as BoxFuture<'_, TxResult<Vec<ExplainLine>>>
    })
    .await
    .map_err(Into::into)
}
