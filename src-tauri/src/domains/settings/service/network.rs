//! `network.rs` — LAN sharing status/enable/disable/rotate + reconnect (01-settings.md §3
//! "Network", Part 02 handoff §9, P2-54/56/57). Windows bodies; every other target refuses with
//! `FORBIDDEN` + `ServerFailure::UnsupportedPlatform`-shaped text (kept local since that variant
//! lives in `infrastructure::database::errors`, Windows-only — this file provides its own constant
//! message on non-Windows so it compiles on every target).

use crate::core::auth::{Access, Area, AuthenticatedUser};
use crate::core::error::{AppError, AppResult};
use crate::core::state::AppState;

use super::super::dto::{LanRole, LanSharingStatus, PairingInfo};

#[cfg(not(windows))]
const UNSUPPORTED_PLATFORM_MESSAGE: &str = "هذه الميزة متاحة على ويندوز فقط";

/// Common gate for the 3 writes (§3 "Network"): the actor must have `Settings:Write` and this
/// device must be the Main PC. Takes a bare `actor` + `conn` (not a full `TxCtx`/`ReadCtx`) so it
/// can run from a plain `with_read` access check before any write happens (§1's disposition: "the
/// rest reads AppState, server.json, keyring" — no transaction needed for the gate itself).
pub async fn require_main_write<C: sea_orm::ConnectionTrait>(conn: &C, actor: Option<&AuthenticatedUser>, state: &AppState) -> AppResult<()> {
    let role = state.device.read().unwrap().role;
    require_main_write_parts(conn, actor, role).await
}

/// Same gate as [`require_main_write`], taking the device role directly instead of `&AppState` —
/// for a caller (a command's `with_read` closure that also needs to move `state` into its own
/// `async move` future) that must read `state.device.read().unwrap().role` *before* the closure so
/// it never has to capture `&AppState` alongside an owned `state` move in the same future.
pub async fn require_main_write_parts<C: sea_orm::ConnectionTrait>(conn: &C, actor: Option<&AuthenticatedUser>, role: crate::core::device::DeviceRole) -> AppResult<()> {
    crate::core::settings::require(conn, actor, Area::Settings, Access::Write).await?;
    if role != crate::core::device::DeviceRole::Main {
        return Err(AppError::forbidden("هذا الإعداد متاح على الجهاز الرئيسي فقط"));
    }
    Ok(())
}

#[cfg(windows)]
pub async fn get_status(state: &AppState) -> AppResult<LanSharingStatus> {
    use crate::infrastructure::database::state_file;

    let device = state.device.read().unwrap().clone();
    if device.role != crate::core::device::DeviceRole::Main {
        return Ok(LanSharingStatus {
            role: LanRole::Terminal,
            provisioned: false,
            lan_sharing: false,
            connected_terminals: 0,
            pairing: None,
            main_host: device.connection.as_ref().map(|c| c.host.clone()),
        });
    }

    let server_state = state_file::load(&state.server.paths.server_json()).ok().flatten();
    let Some(server_state) = server_state else {
        return Ok(LanSharingStatus { role: LanRole::Main, provisioned: false, lan_sharing: false, connected_terminals: 0, pairing: None, main_host: None });
    };

    if !server_state.lan_sharing {
        return Ok(LanSharingStatus {
            role: LanRole::Main,
            provisioned: true,
            lan_sharing: false,
            connected_terminals: 0,
            pairing: None,
            main_host: None,
        });
    }

    let creds = crate::infrastructure::database::credentials::CredentialStore::production();
    let secret = creds.get_lan_secret(server_state.data_dir_id).ok();
    let pairing = secret.map(|s| PairingInfo {
        host_name: crate::infrastructure::database::pairing::host_name().unwrap_or_else(|| "هذا الجهاز".to_string()),
        addresses: crate::infrastructure::database::pairing::non_loopback_ipv4_addresses(),
        port: server_state.port,
        code: crate::infrastructure::database::pairing::format_code(&s),
    });
    let connected_terminals = match creds.get_root_secret(server_state.data_dir_id) {
        Ok(root_secret) => crate::infrastructure::database::lan::connected_terminal_count(server_state.port, &root_secret).await as u32,
        Err(_) => 0,
    };

    Ok(LanSharingStatus { role: LanRole::Main, provisioned: true, lan_sharing: true, connected_terminals, pairing, main_host: None })
}

#[cfg(not(windows))]
pub async fn get_status(_state: &AppState) -> AppResult<LanSharingStatus> {
    Err(AppError::forbidden(UNSUPPORTED_PLATFORM_MESSAGE))
}

#[cfg(windows)]
fn map_lan_error(e: crate::infrastructure::database::lan::LanError) -> AppError {
    use crate::infrastructure::database::lan::LanError;
    match e {
        LanError::PermissionDeclined => AppError::forbidden(e.message_ar()),
        LanError::NotProvisioned => AppError::validation(e.message_ar()),
        other => AppError::internal(other.message_ar(), None),
    }
}

#[cfg(windows)]
pub async fn enable(app: &tauri::AppHandle, state: &AppState) -> AppResult<PairingInfo> {
    use crate::infrastructure::database::{credentials::CredentialStore, lan, payload::PayloadDir};

    let payload = PayloadDir::from_app(app).unwrap_or_else(PayloadDir::dev);
    let creds = CredentialStore::production();

    let info = lan::enable_lan_sharing(&state.server, &creds, &payload, lan::FirewallMode::Real).await.map_err(map_lan_error)?;
    lan::on_lan_sharing_enabled(app, &state.server);
    Ok(info.into())
}

#[cfg(not(windows))]
pub async fn enable(_app: &tauri::AppHandle, _state: &AppState) -> AppResult<PairingInfo> {
    Err(AppError::forbidden(UNSUPPORTED_PLATFORM_MESSAGE))
}

#[cfg(windows)]
pub async fn disable(app: &tauri::AppHandle, state: &AppState, confirm_disconnect: bool) -> AppResult<LanSharingStatus> {
    use crate::infrastructure::database::{credentials::CredentialStore, lan, payload::PayloadDir, state_file};

    let server_state = state_file::load(&state.server.paths.server_json())
        .ok()
        .flatten()
        .ok_or_else(|| AppError::validation(crate::infrastructure::database::lan::LanError::NotProvisioned.message_ar()))?;

    if server_state.lan_sharing {
        let creds = CredentialStore::production();
        if let Ok(root_secret) = creds.get_root_secret(server_state.data_dir_id) {
            let count = lan::connected_terminal_count(server_state.port, &root_secret).await;
            if count > 0 && !confirm_disconnect {
                return Err(AppError::conflict(format!("أجهزة الكاشير متصلة الآن ({count}) — أكّد الإيقاف لقطع اتصالها")));
            }
        }
    }

    let payload = PayloadDir::from_app(app).unwrap_or_else(PayloadDir::dev);
    let creds = CredentialStore::production();
    lan::disable_lan_sharing(&state.server, &creds, &payload).await.map_err(map_lan_error)?;
    lan::on_lan_sharing_disabled(app);

    get_status(state).await
}

#[cfg(not(windows))]
pub async fn disable(_app: &tauri::AppHandle, _state: &AppState, _confirm_disconnect: bool) -> AppResult<LanSharingStatus> {
    Err(AppError::forbidden(UNSUPPORTED_PLATFORM_MESSAGE))
}

#[cfg(windows)]
pub async fn rotate_pairing_code(state: &AppState) -> AppResult<PairingInfo> {
    use crate::infrastructure::database::credentials::CredentialStore;
    let creds = CredentialStore::production();
    let info = crate::infrastructure::database::lan::rotate_pairing_code(&state.server, &creds).await.map_err(map_lan_error)?;
    Ok(info.into())
}

#[cfg(not(windows))]
pub async fn rotate_pairing_code(_state: &AppState) -> AppResult<PairingInfo> {
    Err(AppError::forbidden(UNSUPPORTED_PLATFORM_MESSAGE))
}

/// `reconnectBackend` (D-9): no session required — the failure screen shows before anyone can log
/// in. Main with `server.json` ready → `ensure_running`; then (any role) `connect_and_migrate`.
/// Never touches data; returns `()`.
pub async fn reconnect(app: &tauri::AppHandle, state: &AppState) -> AppResult<()> {
    #[cfg(windows)]
    {
        let is_main = state.device.read().unwrap().role == crate::core::device::DeviceRole::Main;
        if is_main {
            if let Some(paths) = crate::infrastructure::database::paths::ServerPaths::machine() {
                if let Ok(Some(server_state)) = crate::infrastructure::database::state_file::load(&paths.server_json()) {
                    if matches!(server_state.state, crate::infrastructure::database::state_file::ServerLifecycleState::Ready) {
                        let payload = crate::infrastructure::database::payload::PayloadDir::from_app(app)
                            .unwrap_or_else(crate::infrastructure::database::payload::PayloadDir::dev);
                        let creds = crate::infrastructure::database::credentials::CredentialStore::production();
                        let _ = state.server.ensure_running(&payload, &creds).await;
                    }
                }
            }
        }
    }
    crate::core::db::connect_and_migrate(app).await;
    Ok(())
}
