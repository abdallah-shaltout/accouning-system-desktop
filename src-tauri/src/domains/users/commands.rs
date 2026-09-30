//! `users` IPC commands (03-domains/03-users.md §1) — thin layer: parse args, authorize, open a
//! transaction, call the service, map the error, write session state only after `Ok` (login, self
//! -edit refresh, restore).

use sea_orm::EntityTrait;
use tauri::State;

use crate::core::auth::{Access, Area};
use crate::core::dto::ApiErrorPayload;
use crate::core::state::AppState;
use crate::core::tx::{with_read_ctx, with_tx, BoxFuture, TxOpts, TxResult};
use crate::utils::id::Id;

use super::dto::{
    User, UsersCreateUserArgs, UsersGetUserArgs, UsersLoginArgs, UsersRestoreSessionArgs, UsersUpdateUserArgs, UsersVerifyManagerPinArgs,
};
use super::service;

/// A non-UUID id (`'usr-does-not-exist'`) maps to a never-stored id (`Id::unknown_from_text`, plan 21
/// Part 04 Wave 2) so the service answers it with its own `NOT_FOUND` refusal, as the mock does,
/// instead of a generic `VALIDATION` "معرّف غير صالح".
fn parse_id(raw: &str) -> Result<Id, ApiErrorPayload> {
    Ok(raw.parse::<Id>().unwrap_or_else(|_| Id::unknown_from_text(raw)))
}

/// D-3: `users_get_users` is reachable from both a Users-area screen and `InvoiceListPage.vue`
/// (Sales-area cashier) — `require_any`, first grant wins.
#[tauri::command]
pub async fn users_get_users(state: State<'_, AppState>) -> Result<Vec<User>, ApiErrorPayload> {
    with_read_ctx(&state, |tx, ctx| {
        Box::pin(async move {
            ctx.require_any(tx, &[(Area::Users, Access::Read), (Area::Sales, Access::Read)]).await?;
            service::get_users(tx).await
        }) as BoxFuture<'_, TxResult<Vec<User>>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn users_get_user(state: State<'_, AppState>, args: UsersGetUserArgs) -> Result<User, ApiErrorPayload> {
    let id = parse_id(&args.id)?;
    with_read_ctx(&state, move |tx, ctx| {
        Box::pin(async move {
            ctx.require(tx, Area::Users, Access::Read).await?;
            service::get_user(tx, id).await
        }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn users_create_user(state: State<'_, AppState>, args: UsersCreateUserArgs) -> Result<User, ApiErrorPayload> {
    let undo = state.undo.clone();
    with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Users, Access::Write).await?;
            service::create_user(tx, cx, &undo, input).await
        }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .map_err(Into::into)
}

#[tauri::command]
pub async fn users_update_user(state: State<'_, AppState>, args: UsersUpdateUserArgs) -> Result<User, ApiErrorPayload> {
    let id = parse_id(&args.id)?;
    let undo = state.undo.clone();
    let result = with_tx(&state, TxOpts::default(), move |tx, cx| {
        let input = args.input.clone();
        let undo = undo.clone();
        Box::pin(async move {
            cx.require(tx, Area::Users, Access::Write).await?;
            service::update_user(tx, cx, &undo, id, input).await
        }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .map_err(ApiErrorPayload::from)?;

    // D-4: only after the transaction has committed, if the edited user is the current session,
    // replace `AppState.session` with a freshly derived `AuthenticatedUser` (mirrors
    // `useAuthStore.patchCurrent`) so later commands in this process see fresh
    // max_discount/branches immediately.
    let should_refresh = state.session.read().unwrap().as_ref().map(|s| s.id) == Some(id);
    if should_refresh {
        if let Ok(new_session) = build_authenticated_for(&state, id).await {
            *state.session.write().unwrap() = Some(new_session);
        }
    }

    Ok(result)
}

/// Re-derives the `AuthenticatedUser` for `id` from the DB, for the D-4 self-edit session refresh —
/// a short read-only round trip outside the command's own transaction (already committed).
async fn build_authenticated_for(state: &AppState, id: Id) -> Result<crate::core::auth::AuthenticatedUser, ApiErrorPayload> {
    with_read_ctx(state, move |tx, _ctx| {
        Box::pin(async move {
            let row = crate::entities::org::users::Entity::find_by_id(id)
                .one(tx)
                .await
                .map_err(crate::core::error::AppError::from)?
                .ok_or_else(|| crate::core::error::AppError::not_found("المستخدم غير موجود"))?;
            let settings = crate::core::settings::load(tx).await?;
            Ok(super::service::to_authenticated(&row, settings.default_branch_id))
        }) as BoxFuture<'_, TxResult<crate::core::auth::AuthenticatedUser>>
    })
    .await
    .map_err(Into::into)
}

/// `login` runs with no session (`TxOpts { require_user: false }`) — a fresh terminal must be able
/// to authenticate before it has one.
#[tauri::command]
pub async fn users_login(state: State<'_, AppState>, args: UsersLoginArgs) -> Result<User, ApiErrorPayload> {
    let undo = state.undo.clone();
    let result = with_tx(&state, TxOpts { require_user: false }, move |tx, cx| {
        let username = args.username.clone();
        let password = args.password.clone();
        let undo = undo.clone();
        Box::pin(async move { service::login(tx, cx, &undo, &username, &password).await }) as BoxFuture<'_, TxResult<(User, crate::core::auth::AuthenticatedUser)>>
    })
    .await
    .map_err(ApiErrorPayload::from)?;

    // A new login silently replaces an existing session (mock `authService.ts:75`) — written only
    // after the transaction committed successfully.
    let (user, authenticated) = result;
    *state.session.write().unwrap() = Some(authenticated);
    Ok(user)
}

#[tauri::command]
pub async fn users_logout(state: State<'_, AppState>) -> Result<(), ApiErrorPayload> {
    *state.session.write().unwrap() = None;
    Ok(())
}

/// D-1: session-bound — re-attaches only *this process's* existing session; `args.user_id` alone
/// never authenticates. Uses `with_read_ctx`'s bare read snapshot for the row lookup, but reads/
/// writes `AppState.session` directly (a read-only transaction can't carry that side effect).
#[tauri::command]
pub async fn users_restore_session(state: State<'_, AppState>, args: UsersRestoreSessionArgs) -> Result<Option<User>, ApiErrorPayload> {
    let user_id = parse_id(&args.user_id)?;
    let current = state.session.read().unwrap().clone();

    // No session gate here (mirrors the mock: `restoreSession` is callable with no active session
    // and simply returns `None` in that case rather than `UNAUTHORIZED`) — `with_read` (not
    // `with_read_ctx`) is used since this must succeed even when `current` is `None`.
    let result = crate::core::tx::with_read(&state, move |tx| {
        Box::pin(async move { service::restore_session(tx, current.as_ref(), user_id).await }) as BoxFuture<'_, TxResult<Option<(User, crate::core::auth::AuthenticatedUser)>>>
    })
    .await
    .map_err(ApiErrorPayload::from)?;

    match result {
        Some((user, authenticated)) => {
            *state.session.write().unwrap() = Some(authenticated);
            Ok(Some(user))
        }
        None => {
            // Missing/inactive user for a matching session id also clears the session (§3).
            let still_matches = state.session.read().unwrap().as_ref().map(|s| s.id) == Some(user_id);
            if still_matches {
                *state.session.write().unwrap() = None;
            }
            Ok(None)
        }
    }
}

/// Session-only gate (any role — cashier/storekeeper call this for the inventory approval PIN).
#[tauri::command]
pub async fn users_verify_manager_pin(state: State<'_, AppState>, args: UsersVerifyManagerPinArgs) -> Result<User, ApiErrorPayload> {
    with_read_ctx(&state, move |tx, _ctx| {
        let username = args.username.clone();
        let password = args.password.clone();
        Box::pin(async move { service::verify_manager_pin(tx, &username, &password).await }) as BoxFuture<'_, TxResult<User>>
    })
    .await
    .map(|user| {
        // G-P3: a verified manager PIN authorises approvals on this terminal for a short window.
        state.approval_grants.grant(user.id);
        user
    })
    .map_err(Into::into)
}
