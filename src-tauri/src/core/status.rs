//! `core_backend_status` — the only IPC command added in Part 02 (21.02-F F-2, C-22). Builds
//! `BackendStatus` from `AppState`: DB connection status, device role, terminal id, schema check,
//! and — on a Main PC with a managed server (Windows only) — the managed server's own live state
//! (`AppState.server`, A2's `ServerHandle`).
//!
//! No `with_tx`/domain transaction here: this is a pure read of in-process state (`AppState`'s
//! `RwLock`s and, on Windows, the server supervisor's own `tokio::sync::RwLock`), never touching
//! the database itself, so it returns `Result<BackendStatus, ApiErrorPayload>` directly rather than
//! going through `AppResult`/`with_tx` (mirrors `core/diag`'s commands, which also return a plain
//! `Result` since they don't need a DB transaction either — except they use `Result<T, String>`
//! since they predate the `AppError`/`ApiErrorPayload` catalogue; this command is new, so it uses
//! the real typed error payload from the start).

use tauri::State;

use crate::core::device::DeviceRole;
use crate::core::dto::{BackendRole, BackendServerStatus, BackendStatus, SchemaStatus, ServerFailure as DtoServerFailure, ServerState};
use crate::core::state::{AppState, DbStatus};

fn backend_role(role: DeviceRole) -> BackendRole {
    match role {
        DeviceRole::Main => BackendRole::Main,
        DeviceRole::Terminal => BackendRole::Terminal,
    }
}

/// `DbStatus` doesn't distinguish "schema behind"/"schema ahead" from "connected" the way
/// `BackendStatus.schema` does — `SchemaMismatch` is the only schema-specific variant recorded
/// today (P2-28: a terminal that finds pending migrations refuses the DB rather than trying to
/// tell "behind" from "ahead" apart), so it maps to `Behind` (the far more common real-world case:
/// this terminal hasn't been updated yet) and every other status maps to `Ok`/`Unknown` below.
fn schema_status(db_status: DbStatus) -> SchemaStatus {
    match db_status {
        DbStatus::Connected => SchemaStatus::Ok,
        DbStatus::SchemaMismatch => SchemaStatus::Behind,
        _ => SchemaStatus::Unknown,
    }
}

fn db_status_error(db_status: DbStatus) -> Option<String> {
    match db_status {
        DbStatus::NotConfigured => None,
        DbStatus::Connecting => None,
        DbStatus::Connected => None,
        DbStatus::Unsupported => Some("قاعدة البيانات غير مدعومة — يلزم MariaDB 10.11 أو أحدث".to_string()),
        DbStatus::SchemaMismatch => {
            Some("قاعدة البيانات على الجهاز الرئيسي بإصدار مختلف — حدّث البرنامج على الجهازين".to_string())
        }
        DbStatus::Unreachable => Some("تعذر الاتصال بقاعدة البيانات على الجهاز الرئيسي".to_string()),
        DbStatus::ServerStarting => None,
        DbStatus::ServerFailed => Some("تعذر تشغيل قاعدة البيانات المدمجة".to_string()),
    }
}

#[cfg(windows)]
fn server_phase_to_dto(phase: &crate::infrastructure::database::supervisor::ServerPhase) -> (ServerState, Option<DtoServerFailure>) {
    use crate::infrastructure::database::supervisor::ServerPhase;
    match phase {
        ServerPhase::Provisioning => (ServerState::Provisioning, None),
        ServerPhase::Starting => (ServerState::Starting, None),
        ServerPhase::Upgrading => (ServerState::Upgrading, None),
        ServerPhase::Running => (ServerState::Running, None),
        ServerPhase::Stopped => (ServerState::Stopped, None),
        ServerPhase::Failed(failure) => (
            ServerState::Failed,
            Some(DtoServerFailure { code: failure.code().to_string(), message: failure.message_ar().to_string() }),
        ),
    }
}

/// Builds the `server` sub-status (C-22) from `AppState.server` — present only on a Main PC with a
/// managed server. `ServerHandle::unmanaged()` (used by terminals and by every test build) always
/// reports `Stopped` with no version/port, so this only returns `Some` in practice when the device
/// role is `Main` and a managed server was actually configured at boot (`state::boot`'s
/// `with_server`).
#[cfg(windows)]
async fn build_server_status(state: &AppState, role: DeviceRole) -> Option<BackendServerStatus> {
    if role != DeviceRole::Main {
        return None;
    }
    let runtime = state.server.state.read().await;
    let (phase, failure) = server_phase_to_dto(&runtime.phase);
    // `ServerHandle::unmanaged()` never leaves `Provisioning`/`Starting`/etc. by construction — but
    // an unmanaged handle is indistinguishable from a genuinely-`Stopped` managed one purely from
    // its `ServerRuntime`, so this also checks the on-disk state file (same source
    // `diagnostics_snapshot` reads) for `lan_sharing`, falling back to `false` when there is none
    // (no managed server was ever provisioned on this PC).
    let state_file = crate::infrastructure::database::state_file::load(&state.server.paths.server_json()).ok().flatten();
    let Some(state_file) = state_file else { return None };
    Some(BackendServerStatus {
        state: phase,
        version: runtime.version.clone().unwrap_or_else(|| state_file.last_started_with.clone()),
        port: runtime.port.map(i32::from).unwrap_or(state_file.port as i32),
        lan_sharing: state_file.lan_sharing,
        failure,
    })
}

#[cfg(not(windows))]
async fn build_server_status(_state: &AppState, _role: DeviceRole) -> Option<BackendServerStatus> {
    None
}

/// Result of `core_backend_status` — no args, read-only, never fails: even a fully disconnected
/// state (`DbStatus::NotConfigured`) is a normal, representable `BackendStatus` (`connected:
/// false`), not an error. Kept as a `Result` (rather than a bare `BackendStatus`) only so a future
/// failure mode has somewhere to go without changing the command's signature — mirrors the
/// `ApiErrorPayload`-shaped rejection every other typed command uses (F-3's `backendCall`).
#[tauri::command]
pub async fn core_backend_status(state: State<'_, AppState>) -> Result<BackendStatus, crate::core::dto::ApiErrorPayload> {
    let db_status = *state.db_status.read().unwrap();
    let role = state.device.read().unwrap().role;
    let terminal_id = state.terminal.terminal_id;

    let server = build_server_status(&state, role).await;

    Ok(BackendStatus {
        connected: db_status == DbStatus::Connected,
        role: backend_role(role),
        terminal_id: terminal_id.to_string(),
        server_version: server.as_ref().map(|s| s.version.clone()),
        schema: schema_status(db_status),
        error: db_status_error(db_status),
        server,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_status_maps_known_db_statuses() {
        assert!(matches!(schema_status(DbStatus::Connected), SchemaStatus::Ok));
        assert!(matches!(schema_status(DbStatus::SchemaMismatch), SchemaStatus::Behind));
        assert!(matches!(schema_status(DbStatus::NotConfigured), SchemaStatus::Unknown));
        assert!(matches!(schema_status(DbStatus::Unreachable), SchemaStatus::Unknown));
    }

    #[test]
    fn db_status_error_is_none_only_for_healthy_or_transitional_states() {
        assert!(db_status_error(DbStatus::Connected).is_none());
        assert!(db_status_error(DbStatus::NotConfigured).is_none());
        assert!(db_status_error(DbStatus::Connecting).is_none());
        assert!(db_status_error(DbStatus::ServerStarting).is_none());
        assert!(db_status_error(DbStatus::Unsupported).is_some());
        assert!(db_status_error(DbStatus::SchemaMismatch).is_some());
        assert!(db_status_error(DbStatus::Unreachable).is_some());
        assert!(db_status_error(DbStatus::ServerFailed).is_some());
    }

    #[test]
    fn backend_role_maps_device_role() {
        assert!(matches!(backend_role(DeviceRole::Main), BackendRole::Main));
        assert!(matches!(backend_role(DeviceRole::Terminal), BackendRole::Terminal));
    }
}
