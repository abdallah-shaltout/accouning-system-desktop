//! `session.rs` (02-setup.md §3.2, D-1): the bootstrap session — while `onboarding.finished_at` is
//! NULL and nobody is logged in, the first active `admin` user becomes this process's session, so
//! the public wizard route reaches the same session-scoped commands the mock reached with no auth
//! at all. It never outlives onboarding (`finish_onboarding` clears it) and satisfies 03-users H-1
//! (`users_create_user` stays authorised during the wizard).

use std::sync::Mutex;

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::auth::AuthenticatedUser;
use crate::core::state::AppState;
use crate::core::tx::{TxError, TxResult};
use crate::domains::users::service::to_authenticated;
use crate::entities::org::settings::Entity as SettingsEntity;
use crate::entities::org::users::{Column as UserColumn, Entity as UserEntity};
use crate::utils::id::Id;

/// Process-local: remembers which user id `ensure_bootstrap_session` itself logged in, so
/// `clear_bootstrap_session` only ever clears a session it set — never a real login that happened
/// to race it.
static BOOTSTRAP_USER: Mutex<Option<Id>> = Mutex::new(None);

/// Called after a transaction that may have just seeded the shell commits (`get_onboarding_progress`'s
/// command body) — takes its own connection (a fresh read) since `state.session` write must happen
/// outside any transaction. No-op when a session already exists, or onboarding has finished, or
/// there is no settings row / no admin user yet.
pub async fn ensure_bootstrap_session<C: ConnectionTrait>(state: &AppState, conn: &C) -> TxResult<()> {
    if state.session.read().unwrap().is_some() {
        return Ok(());
    }

    let settings = SettingsEntity::find().one(conn).await.map_err(TxError::from)?;
    let finished = settings.as_ref().and_then(|s| s.onboarding.as_ref()).and_then(|o| o.finished_at);
    if finished.is_some() {
        return Ok(());
    }
    let Some(settings) = settings else { return Ok(()) };

    let admin = UserEntity::find()
        .filter(UserColumn::Role.eq("admin"))
        .filter(UserColumn::Active.eq(true))
        .order_by_asc(UserColumn::CreatedAt)
        .order_by_asc(UserColumn::Id)
        .one(conn)
        .await
        .map_err(TxError::from)?;
    let Some(admin) = admin else { return Ok(()) };

    let authenticated: AuthenticatedUser = to_authenticated(&admin, settings.default_branch_id);
    *state.session.write().unwrap() = Some(authenticated);
    *BOOTSTRAP_USER.lock().unwrap() = Some(admin.id);
    Ok(())
}

/// `finish_onboarding` clears the bootstrap session — only if the current session is still the same
/// user this module itself logged in (never clobbers a real login that happened to race it).
pub fn clear_bootstrap_session(state: &AppState) {
    let mut bootstrap = BOOTSTRAP_USER.lock().unwrap();
    if let Some(bootstrap_id) = *bootstrap {
        let mut session = state.session.write().unwrap();
        if let Some(current) = session.as_ref() {
            if current.id == bootstrap_id {
                *session = None;
            }
        }
        *bootstrap = None;
    }
}
