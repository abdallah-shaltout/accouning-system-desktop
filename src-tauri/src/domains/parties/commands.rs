//! `parties` IPC commands (05-parties.md §1) — 15 commands. Every read `with_read_ctx` +
//! `Parties:Read`; every write `with_tx` + `Parties:Write`.

use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::core::tx::{with_read_ctx, with_tx, BoxFuture, TxOpts, TxResult};
use crate::utils::id::Id;

use super::dto::{
    AgingBucket, Customer, DuplicateWarning, OptionalMoney, PartiesCheckDuplicatesArgs, PartiesGetCustomerArgs, PartiesGetCustomerStatementArgs, PartiesGetCustomersArgs,
    PartiesGetLinkedNetBalanceArgs, PartiesGetPartyAgingArgs, PartiesGetPartyGroupsArgs, PartiesGetPartyHistoryArgs, PartiesGetSupplierArgs, PartiesGetSupplierStatementArgs,
    PartiesGetSuppliersArgs, PartiesLinkPartyRecordsArgs, PartiesSaveCustomerArgs, PartiesSaveSupplierArgs, PartiesUnlinkPartyRecordArgs, PartyGroup, PartyHistoryEntry, PartyStatementRow,
    Supplier,
};
use super::service;

fn parse_id(raw: &str) -> Result<Id, ApiErrorPayload> {
    raw.parse::<Id>().map_err(|_| AppError::validation("معرّف غير صالح").into())
}

fn parse_optional_id(raw: &Option<String>) -> Result<Option<Id>, ApiErrorPayload> {
    match raw {
        Some(s) if !s.is_empty() => parse_id(s).map(Some),
        _ => Ok(None),
    }
}

#[tauri::command]
pub async fn parties_check_duplicates(state: State<'_, AppState>, args: PartiesCheckDuplicatesArgs) -> Result<Vec<DuplicateWarning>, ApiErrorPayload> {
    let exclude_id = parse_optional_id(&args.exclude_id)?;
    with_read_ctx(&state, move |tx, ctx| {
        let input = args.input.clone();
        Box::pin(async move {
            ctx.require(tx, Area::Parties, Access::Read).await?;
            service::read::check_duplicates(tx, input, exclude_id).await
        }) as BoxFuture<'_, TxResult<Vec<DuplicateWarning>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn parties_get_party_groups(state: State<'_, AppState>, args: PartiesGetPartyGroupsArgs) -> Result<Vec<PartyGroup>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Parties, Access::Read).await?;
            service::read::get_party_groups(tx, args.kind).await
        }) as BoxFuture<'_, TxResult<Vec<PartyGroup>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn parties_get_customers(state: State<'_, AppState>, args: PartiesGetCustomersArgs) -> Result<Vec<Customer>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let filter = args.filter.clone().unwrap_or_default();
        Box::pin(async move {
            ctx.require(tx, Area::Parties, Access::Read).await?;
            service::read::get_customers(tx, filter).await
        }) as BoxFuture<'_, TxResult<Vec<Customer>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn parties_get_customer(state: State<'_, AppState>, args: PartiesGetCustomerArgs) -> Result<Customer, ApiErrorPayload> {
    let id = parse_id(&args.id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Parties, Access::Read).await?;
            service::read::get_customer(tx, id).await
        }) as BoxFuture<'_, TxResult<Customer>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn parties_save_customer(state: State<'_, AppState>, args: PartiesSaveCustomerArgs) -> Result<Customer, ApiErrorPayload> {
    let id = parse_optional_id(&args.id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Parties, Access::Write).await?;
            service::write::save_customer(tx, cx, &undo, input, id).await
        }) as BoxFuture<'_, TxResult<Customer>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn parties_get_customer_statement(state: State<'_, AppState>, args: PartiesGetCustomerStatementArgs) -> Result<Vec<PartyStatementRow>, ApiErrorPayload> {
    let id = parse_id(&args.id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Parties, Access::Read).await?;
            service::read::get_customer_statement(tx, id).await
        }) as BoxFuture<'_, TxResult<Vec<PartyStatementRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn parties_get_suppliers(state: State<'_, AppState>, args: PartiesGetSuppliersArgs) -> Result<Vec<Supplier>, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, ctx| {
        let filter = args.filter.clone().unwrap_or_default();
        Box::pin(async move {
            ctx.require(tx, Area::Parties, Access::Read).await?;
            service::read::get_suppliers(tx, filter).await
        }) as BoxFuture<'_, TxResult<Vec<Supplier>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn parties_get_supplier(state: State<'_, AppState>, args: PartiesGetSupplierArgs) -> Result<Supplier, ApiErrorPayload> {
    let id = parse_id(&args.id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Parties, Access::Read).await?;
            service::read::get_supplier(tx, id).await
        }) as BoxFuture<'_, TxResult<Supplier>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn parties_save_supplier(state: State<'_, AppState>, args: PartiesSaveSupplierArgs) -> Result<Supplier, ApiErrorPayload> {
    let id = parse_optional_id(&args.id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Parties, Access::Write).await?;
            service::write::save_supplier(tx, cx, &undo, input, id).await
        }) as BoxFuture<'_, TxResult<Supplier>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn parties_get_supplier_statement(state: State<'_, AppState>, args: PartiesGetSupplierStatementArgs) -> Result<Vec<PartyStatementRow>, ApiErrorPayload> {
    let id = parse_id(&args.id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Parties, Access::Read).await?;
            service::read::get_supplier_statement(tx, id).await
        }) as BoxFuture<'_, TxResult<Vec<PartyStatementRow>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn parties_link_party_records(state: State<'_, AppState>, args: PartiesLinkPartyRecordsArgs) -> Result<(), ApiErrorPayload> {
    let customer_id = parse_id(&args.customer_id)?;
    let supplier_id = parse_id(&args.supplier_id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Parties, Access::Write).await?;
            service::link::link(tx, cx, &undo, customer_id, supplier_id).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn parties_unlink_party_record(state: State<'_, AppState>, args: PartiesUnlinkPartyRecordArgs) -> Result<(), ApiErrorPayload> {
    let party_id = parse_id(&args.party_id)?;
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Parties, Access::Write).await?;
            service::link::unlink(tx, cx, &undo, party_id, args.kind).await
        }) as BoxFuture<'_, TxResult<()>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn parties_get_linked_net_balance(state: State<'_, AppState>, args: PartiesGetLinkedNetBalanceArgs) -> Result<OptionalMoney, ApiErrorPayload> {
    let customer_id = parse_optional_id(&args.customer_id)?;
    let supplier_id = parse_optional_id(&args.supplier_id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Parties, Access::Read).await?;
            let result = service::link::get_linked_net_balance(tx, customer_id, supplier_id).await?;
            Ok(OptionalMoney(result))
        }) as BoxFuture<'_, TxResult<OptionalMoney>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn parties_get_party_history(state: State<'_, AppState>, args: PartiesGetPartyHistoryArgs) -> Result<Vec<PartyHistoryEntry>, ApiErrorPayload> {
    let party_id = parse_id(&args.party_id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Parties, Access::Read).await?;
            service::read::get_party_history(tx, party_id).await
        }) as BoxFuture<'_, TxResult<Vec<PartyHistoryEntry>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn parties_get_party_aging(state: State<'_, AppState>, args: PartiesGetPartyAgingArgs) -> Result<Vec<AgingBucket>, ApiErrorPayload> {
    let party_id = parse_id(&args.party_id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Parties, Access::Read).await?;
            service::aging::get_party_aging(tx, &ctx.clock, args.kind, party_id).await
        }) as BoxFuture<'_, TxResult<Vec<AgingBucket>>>
    })
    .await
    .map_err(Into::into)
}

/// §1's 15 commands, in table order.
pub fn ipc_signatures() -> Vec<crate::core::ipc::IpcSig> {
    use super::dto::*;
    vec![
        crate::ipc_sig!(parties_check_duplicates, PartiesCheckDuplicatesArgs, Vec<DuplicateWarning>),
        crate::ipc_sig!(parties_get_party_groups, PartiesGetPartyGroupsArgs, Vec<PartyGroup>),
        crate::ipc_sig!(parties_get_customers, PartiesGetCustomersArgs, Vec<Customer>),
        crate::ipc_sig!(parties_get_customer, PartiesGetCustomerArgs, Customer),
        crate::ipc_sig!(parties_save_customer, PartiesSaveCustomerArgs, Customer),
        crate::ipc_sig!(parties_get_customer_statement, PartiesGetCustomerStatementArgs, Vec<PartyStatementRow>),
        crate::ipc_sig!(parties_get_suppliers, PartiesGetSuppliersArgs, Vec<Supplier>),
        crate::ipc_sig!(parties_get_supplier, PartiesGetSupplierArgs, Supplier),
        crate::ipc_sig!(parties_save_supplier, PartiesSaveSupplierArgs, Supplier),
        crate::ipc_sig!(parties_get_supplier_statement, PartiesGetSupplierStatementArgs, Vec<PartyStatementRow>),
        crate::ipc_sig!(parties_link_party_records, PartiesLinkPartyRecordsArgs, ()),
        crate::ipc_sig!(parties_unlink_party_record, PartiesUnlinkPartyRecordArgs, ()),
        crate::ipc_sig!(parties_get_linked_net_balance, PartiesGetLinkedNetBalanceArgs, OptionalMoney),
        crate::ipc_sig!(parties_get_party_history, PartiesGetPartyHistoryArgs, Vec<PartyHistoryEntry>),
        crate::ipc_sig!(parties_get_party_aging, PartiesGetPartyAgingArgs, Vec<AgingBucket>),
    ]
}
