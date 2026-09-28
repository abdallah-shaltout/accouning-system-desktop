//! `device.rs` (02-setup.md §3.1, Windows bodies; other targets: `FORBIDDEN` unsupported): the
//! first-run role step and terminal pairing — `get_device_setup_state`/`provision_main`/
//! `pair_terminal`, the Part 02 A3-2/P2-54 handoff.

use crate::core::error::AppResult;

use super::super::dto::{DeviceRoleWire, DeviceSetupState};

/// Non-Windows stub message (no managed server, no LAN pairing target exists off Windows).
#[cfg(not(windows))]
fn unsupported() -> crate::core::error::AppError {
    crate::core::error::AppError::forbidden("هذه الميزة غير متاحة على هذا النظام")
}

#[cfg(windows)]
pub async fn get_device_setup_state(app: &tauri::AppHandle, state: &crate::core::state::AppState) -> AppResult<DeviceSetupState> {
    use std::time::Duration;

    // Waits (<= 60s, 250ms steps) while db_status is Connecting/ServerStarting — boot connects in
    // the background.
    for _ in 0..240 {
        let status = *state.db_status.read().unwrap();
        if !matches!(status, crate::core::state::DbStatus::Connecting | crate::core::state::DbStatus::ServerStarting) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    let device = state.device.read().unwrap().clone();
    let configured = device.connection.is_some() || crate::core::device::debug_db_url_override().is_some();
    let role = match device.role {
        crate::core::device::DeviceRole::Main => DeviceRoleWire::Main,
        crate::core::device::DeviceRole::Terminal => DeviceRoleWire::Terminal,
    };
    let can_host_database = crate::infrastructure::database::payload::PayloadDir::from_app(app).and_then(|p| p.read_manifest().ok()).is_some();

    let has_users = if configured {
        let connected = matches!(*state.db_status.read().unwrap(), crate::core::state::DbStatus::Connected);
        if connected {
            let connection = state.db.read().unwrap().as_ref().map(|db| db.connection.clone());
            match connection {
                Some(connection) => {
                    use sea_orm::{EntityTrait, PaginatorTrait};
                    crate::entities::org::users::Entity::find().count(&connection).await.map(|c| c > 0).unwrap_or(false)
                }
                None => false,
            }
        } else {
            false
        }
    } else {
        false
    };

    Ok(DeviceSetupState { configured, role, can_host_database, has_users })
}

#[cfg(not(windows))]
pub async fn get_device_setup_state(_app: &tauri::AppHandle, _state: &crate::core::state::AppState) -> AppResult<DeviceSetupState> {
    Err(unsupported())
}

#[cfg(windows)]
pub async fn provision_main(app: &tauri::AppHandle, state: &crate::core::state::AppState) -> AppResult<DeviceSetupState> {
    use crate::core::error::AppError;
    use crate::infrastructure::database::{credentials::CredentialStore, errors::ServerFailure, payload::PayloadDir, provision};

    let device = state.device.read().unwrap().clone();
    let configured = device.connection.is_some() || crate::core::device::debug_db_url_override().is_some();
    if configured {
        return Err(AppError::conflict("هذا الجهاز مُعدّ بالفعل"));
    }

    let payload = PayloadDir::from_app(app).ok_or_else(|| AppError::internal(ServerFailure::PayloadMissing.message_ar(), None))?;
    if payload.read_manifest().is_err() {
        return Err(AppError::internal(ServerFailure::PayloadMissing.message_ar(), None));
    }

    let creds = CredentialStore::production();

    let result = provision::provision_main(&state.server.paths, &payload, &creds, state.terminal.terminal_id, Some(&state.app_data_dir)).await;
    let _state_file = match result {
        Ok(r) => r,
        Err(f) => {
            log::error!(target: "infrastructure::database", "provision_main failed: {} ({})", f.code(), f.message_ar());
            return Err(AppError::internal(f.message_ar(), None));
        }
    };

    *state.device.write().unwrap() = crate::core::device::load(&state.app_data_dir).map_err(|e| AppError::internal("تعذر قراءة إعدادات الجهاز", Some(e.to_string())))?;

    if let Err(f) = state.server.ensure_running(&payload, &creds).await {
        log::error!(target: "infrastructure::database", "ensure_running after provision_main failed: {} ({})", f.code(), f.message_ar());
        return Err(AppError::internal(f.message_ar(), None));
    }

    crate::core::db::connect_and_migrate(app).await;

    let status = *state.db_status.read().unwrap();
    if !matches!(status, crate::core::state::DbStatus::Connected) {
        // `core/status.rs`'s wording for "the embedded database could not be brought up" (this
        // path only runs right after a fresh provision + `ensure_running`, so any other status here
        // means the embedded server itself failed to come up cleanly).
        return Err(AppError::internal("تعذر تشغيل قاعدة البيانات المدمجة", None));
    }

    get_device_setup_state(app, state).await
}

#[cfg(not(windows))]
pub async fn provision_main(_app: &tauri::AppHandle, _state: &crate::core::state::AppState) -> AppResult<DeviceSetupState> {
    Err(unsupported())
}

#[cfg(windows)]
pub async fn pair_terminal(
    app: &tauri::AppHandle,
    state: &crate::core::state::AppState,
    input: super::super::dto::PairTerminalInput,
) -> AppResult<DeviceSetupState> {
    use crate::core::error::AppError;
    use crate::infrastructure::database::pairing;

    let device = state.device.read().unwrap().clone();
    let configured = device.connection.is_some() || crate::core::device::debug_db_url_override().is_some();
    if configured {
        return Err(AppError::conflict("هذا الجهاز مُعدّ بالفعل"));
    }

    let host = input.host.trim();
    if host.is_empty() {
        return Err(AppError::validation("أدخل اسم الجهاز الرئيسي أو عنوانه"));
    }
    if input.port == 0 {
        return Err(AppError::validation("رقم المنفذ غير صحيح"));
    }
    let secret = pairing::parse_code(&input.code).ok_or_else(|| AppError::validation("رمز الاقتران غير صحيح — أدخله كما يظهر على الجهاز الرئيسي"))?;

    // Probe: connect, check version, migrate (Terminal role) — close the probe pool after.
    let probe_settings = crate::core::device::ConnectionSettings {
        host: host.to_string(),
        port: input.port,
        database: "equal".to_string(),
        user: "equal_lan".to_string(),
        last_known_address: None,
    };
    let probe_db = crate::core::db::connect(&probe_settings, &secret)
        .await
        .map_err(|_| AppError::validation("تعذر الاتصال بالجهاز الرئيسي — تأكد من الاسم والرمز وأن الجهاز الرئيسي يعمل"))?;

    if crate::core::db::check_version(&probe_db).await.is_err() {
        return Err(AppError::validation("قاعدة البيانات غير مدعومة — يلزم MariaDB 10.11 أو أحدث"));
    }

    if crate::core::db::migrate(&probe_db, crate::core::device::DeviceRole::Terminal, &crate::core::db::NoPendingMigrationBackup)
        .await
        .is_err()
    {
        return Err(AppError::validation("قاعدة البيانات على الجهاز الرئيسي بإصدار مختلف — حدّث البرنامج على الجهازين"));
    }
    drop(probe_db);

    // First IPv4 from a DNS lookup (None when `host` is already an IP).
    let last_known_address = tokio::net::lookup_host((host, input.port))
        .await
        .ok()
        .and_then(|mut it| it.find_map(|addr| match addr.ip() {
            std::net::IpAddr::V4(v4) => Some(v4.to_string()),
            _ => None,
        }));

    crate::core::device::pair_terminal(&state.app_data_dir, host, input.port, &secret, last_known_address)
        .map_err(|e| AppError::internal("تعذر حفظ إعدادات الاقتران", Some(e.to_string())))?;

    *state.device.write().unwrap() = crate::core::device::load(&state.app_data_dir).map_err(|e| AppError::internal("تعذر قراءة إعدادات الجهاز", Some(e.to_string())))?;

    crate::core::db::connect_and_migrate(app).await;

    get_device_setup_state(app, state).await
}

#[cfg(not(windows))]
pub async fn pair_terminal(
    _app: &tauri::AppHandle,
    _state: &crate::core::state::AppState,
    _input: super::super::dto::PairTerminalInput,
) -> AppResult<DeviceSetupState> {
    Err(unsupported())
}

