//! `AppState` (phase-a-foundation.md A-5/A-6): the one piece of process-global state every command
//! reaches through. Holds the terminal identity, device settings, the DB pool (once connected),
//! the current session (cross-cutting.md §1's "session = this process's AppState"), and the
//! change-version cursors `with_tx`'s poller compares against.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use sea_orm::DatabaseConnection;
use tauri::{AppHandle, Manager};

use crate::core::auth::AuthenticatedUser;
use crate::core::device::DeviceSettings;
use crate::core::events::{ChangeCategory, EventSink, TauriEventSink};
use crate::core::terminal::TerminalIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbStatus {
    /// No connection attempted yet, or none configured — the app runs on the mock.
    NotConfigured,
    Connecting,
    Connected,
    /// `SELECT VERSION()` didn't report MariaDB ≥ 10.11 (P2-03).
    Unsupported,
    /// A terminal's applied-migrations set doesn't match this build's list (P2-28).
    SchemaMismatch,
    Unreachable,
    /// A2-10: the Main PC's bundled MariaDB server is being provisioned/started — detail lives in
    /// `AppState.server`'s own `ServerRuntime.phase`.
    ServerStarting,
    /// A2-10: the bundled server failed to start — see `AppState.server`'s `ServerPhase::Failed`
    /// for the `ServerFailure` reason.
    ServerFailed,
    /// G-45/GB-3: the automatic pre-migration backup (P2-52) failed on a Main PC with pending
    /// migrations — migrations are never run in this case (zero data loss over availability, D-14
    /// in 17-backup.md), so the schema stays exactly as it was. Drives `core/status.rs`'s
    /// `ServerFailureScreen` message.
    MigrationBackupFailed,
}

pub struct Db {
    pub connection: DatabaseConnection,
}

pub struct AppState {
    pub terminal: TerminalIdentity,
    pub device: RwLock<DeviceSettings>,
    pub db: RwLock<Option<Db>>,
    pub db_status: RwLock<DbStatus>,
    pub session: RwLock<Option<AuthenticatedUser>>,
    pub change_seen: Mutex<BTreeMap<ChangeCategory, u64>>,
    pub events: Arc<dyn EventSink>,
    pub app_data_dir: PathBuf,
    /// A2-10: the bundled MariaDB server's supervisor handle. `ServerHandle::unmanaged()` for
    /// terminals (no managed server exists on this PC) and for every test built through
    /// `tests/support/mod.rs::TestDb::fresh()` (which talks to `bun run db:dev`, not a
    /// per-instance managed server).
    #[cfg(windows)]
    pub server: Arc<crate::infrastructure::database::supervisor::ServerHandle>,
    /// Part 02 accounting-trace ring buffer (18.F's posting-trace / "اشرح هذا الرقم" debugger):
    /// a fixed-capacity in-memory ring every posting path appends to, read by
    /// `/dev/diagnostics` → المحاسبة. Owned by `shared::ledger::trace`; empty at boot.
    pub traces: Arc<crate::shared::ledger::trace::TraceRing>,
    /// Part 03 undo/compensation registry (empty in Part 02 — no compensators are registered yet;
    /// Part 03 wires them up against real domain services). Owned by `shared::activity::undo`.
    pub undo: Arc<crate::shared::activity::undo::UndoRegistry>,
    /// Manager-PIN approval grants (G-P3) — see `core::grants`.
    pub approval_grants: Arc<crate::core::grants::ApprovalGrants>,
}

impl AppState {
    pub fn new(app_data_dir: PathBuf, terminal: TerminalIdentity, device: DeviceSettings, events: Arc<dyn EventSink>) -> Self {
        Self {
            terminal,
            device: RwLock::new(device),
            db: RwLock::new(None),
            db_status: RwLock::new(DbStatus::NotConfigured),
            session: RwLock::new(None),
            change_seen: Mutex::new(BTreeMap::new()),
            events,
            app_data_dir,
            #[cfg(windows)]
            server: crate::infrastructure::database::supervisor::ServerHandle::unmanaged(),
            traces: Arc::new(crate::shared::ledger::trace::TraceRing::new(crate::shared::ledger::trace::TRACE_RING_CAPACITY)),
            undo: Arc::new({
                let mut registry = crate::shared::activity::undo::UndoRegistry::new();
                crate::domains::register_undo(&mut registry);
                registry
            }),
            approval_grants: Arc::new(crate::core::grants::ApprovalGrants::new()),
        }
    }

    /// Used only by `state::boot` on the Main PC path (A2-10), after `ServerPaths::machine()`
    /// resolves — replaces the `unmanaged()` placeholder with a real handle rooted at
    /// `%ProgramData%\...\database`.
    #[cfg(windows)]
    pub fn with_server(mut self, server: Arc<crate::infrastructure::database::supervisor::ServerHandle>) -> Self {
        self.server = server;
        self
    }
}

/// Called from `lib.rs`'s `tauri::Builder::setup`. Builds `AppState` with `db = None` and manages
/// it immediately — the window never waits for the DB (A-6): a background task connects, gates
/// and migrates afterward, and the mock keeps working in the meantime.
///
/// A2-10 adds two boot-time steps that only matter on a Main PC with a bundled server:
/// - **restore**: if `terminal.json` is missing but `%ProgramData%\...\database\server.json` is
///   `ready` and names a `hostTerminalId`, that terminal identity is restored (never regenerated,
///   cross-cutting §2) instead of minting a brand new one — this PC *is* that terminal.
/// - **reattach**: if `device-settings.json` has no connection configured yet but `server.json` is
///   `ready`, the connection is filled in from it and `role` set to `main` (P2-59) — so a device
///   settings file that was somehow lost/reset on the Main PC still finds its own database.
pub fn boot(app: AppHandle) {
    let app_data_dir = app.path().app_data_dir().expect("app_data_dir must be resolvable");

    // G-43/GU-1: resolved independently of whether `server.json` already exists — a fresh install
    // on a would-be Main PC has no `server.json` yet (first-run provisioning hasn't happened), but
    // `ServerPaths::machine()` itself still resolves from the OS `%ProgramData%` folder alone. The
    // machine *paths* (this PC's would-be managed-server root) and the machine *state file* (does a
    // server already exist there, and in what lifecycle state) are two different questions — kept
    // as two separate options below so a fresh install still gets a real, supervised `ServerHandle`
    // (rather than `unmanaged()`) even before the setup wizard's role step ever provisions anything.
    #[cfg(windows)]
    let machine_paths = crate::infrastructure::database::paths::ServerPaths::machine();
    #[cfg(not(windows))]
    let machine_paths: Option<()> = None;

    #[cfg(windows)]
    let machine_state = machine_paths.as_ref().and_then(|paths| {
        crate::infrastructure::database::state_file::load(&paths.server_json()).ok().flatten().map(|s| (paths.clone(), s))
    });
    #[cfg(not(windows))]
    let machine_state: Option<((), ())> = None;

    let terminal_path = crate::core::terminal::file_path(&app_data_dir);
    #[cfg(windows)]
    let terminal = if !terminal_path.exists() {
        match &machine_state {
            Some((_, server_state))
                if matches!(server_state.state, crate::infrastructure::database::state_file::ServerLifecycleState::Ready) =>
            {
                crate::core::terminal::restore(&app_data_dir, server_state.host_terminal_id)
                    .expect("terminal identity must be restorable")
            }
            _ => crate::core::terminal::load_or_create(&app_data_dir).expect("terminal identity must be loadable/creatable"),
        }
    } else {
        crate::core::terminal::load_or_create(&app_data_dir).expect("terminal identity must be loadable/creatable")
    };
    #[cfg(not(windows))]
    let terminal = crate::core::terminal::load_or_create(&app_data_dir).expect("terminal identity must be loadable/creatable");

    let mut device = crate::core::device::load(&app_data_dir).expect("device settings must be loadable");

    #[cfg(windows)]
    if device.connection.is_none() {
        if let Some((_paths, server_state)) = &machine_state {
            if matches!(server_state.state, crate::infrastructure::database::state_file::ServerLifecycleState::Ready) {
                device.role = crate::core::device::DeviceRole::Main;
                device.connection = Some(crate::core::device::ConnectionSettings {
                    host: "127.0.0.1".to_string(),
                    port: server_state.port,
                    database: "equal".to_string(),
                    user: "equal_app".to_string(),
                    last_known_address: None,
                });
                let _ = crate::core::device::save(&app_data_dir, &device);
            }
        }
    }

    let events: Arc<dyn EventSink> = Arc::new(TauriEventSink(app.clone()));
    let mut state = AppState::new(app_data_dir.clone(), terminal, device.clone(), events);
    // G-43/GU-1: attach a real, supervised `ServerHandle` whenever `ServerPaths::machine()` resolves
    // at all — not only once `server.json` already exists — so first-run provisioning that happens
    // mid-session (the setup wizard's role step) is supervised, restartable, LAN-toggleable and shut
    // down cleanly on exit from the very first boot, instead of running against the `unmanaged()`
    // placeholder until the next app restart. `build_server_status` already returns `None` for
    // terminals regardless of this handle's presence (it gates on `device.role == Main` first), so
    // a terminal or a non-Main PC attaching a technically-real-but-never-used handle here changes no
    // observable status.
    #[cfg(windows)]
    {
        if let Some(paths) = machine_paths {
            state = state.with_server(crate::infrastructure::database::supervisor::ServerHandle::new(paths));
        }
    }
    app.manage(state);

    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        #[cfg(windows)]
        {
            let is_main_with_server = {
                let state = handle.state::<AppState>();
                let device = state.device.read().unwrap();
                device.role == crate::core::device::DeviceRole::Main
                    && crate::infrastructure::database::paths::ServerPaths::machine()
                        .map(|p| p.server_json().exists())
                        .unwrap_or(false)
            };
            if is_main_with_server {
                boot_managed_server(&handle).await;
                // Migrations never run unless server.json is `ready` — boot_managed_server only
                // returns having reached `Running` when that was true, so this is safe.
                crate::core::db::connect_and_migrate(&handle).await;
                return;
            }
        }
        crate::core::db::connect_and_migrate(&handle).await;
    });

    // F-5 (P2-11): the cross-terminal change poller. Started unconditionally — it skips quietly
    // every tick until `AppState.db` is populated by the `connect_and_migrate` task above, then
    // starts comparing `change_versions` against `AppState.change_seen` once a connection exists.
    crate::core::poller::spawn(app);
}

/// A2-10: brings the Main PC's bundled server up before the unchanged `connect_and_migrate` path
/// runs. Resumes an interrupted `provision_main` when `server.json` is `initializing`, otherwise
/// calls `ensure_running`. On failure, records `DbStatus::ServerFailed` and returns — the mock
/// keeps the app usable either way.
#[cfg(windows)]
async fn boot_managed_server(handle: &AppHandle) {
    use crate::infrastructure::database::{credentials::CredentialStore, payload::PayloadDir, paths::ServerPaths};

    let Some(paths) = ServerPaths::machine() else { return };
    let state = handle.state::<AppState>();
    *state.db_status.write().unwrap() = DbStatus::ServerStarting;

    let payload = PayloadDir::from_app(handle).unwrap_or_else(PayloadDir::dev);
    let creds = CredentialStore::production();

    let server_json = crate::infrastructure::database::state_file::load(&paths.server_json()).ok().flatten();
    let result = match server_json {
        Some(s) if matches!(s.state, crate::infrastructure::database::state_file::ServerLifecycleState::Ready) => {
            state.server.ensure_running(&payload, &creds).await
        }
        _ => {
            let app_data_dir = state.app_data_dir.clone();
            crate::infrastructure::database::provision::provision_main(
                &paths,
                &payload,
                &creds,
                state.terminal.terminal_id,
                Some(&app_data_dir),
            )
            .await
            .map(|_| ())
        }
    };

    if let Err(_failure) = result {
        *state.db_status.write().unwrap() = DbStatus::ServerFailed;
    }
}
