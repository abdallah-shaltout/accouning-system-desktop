//! `ServerHandle` — the managed MariaDB server's runtime state machine (phase-a2 A2-7, P2-46,
//! P2-58). Lives in `AppState` as `Arc<ServerHandle>` (A2-10). Only one meaningful instance per
//! process: `tauri-plugin-single-instance` (P2-58) guarantees only one app process — and thus only
//! one supervisor — ever runs against a given data directory at a time.

#![cfg(windows)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::process::Child;
use tokio::sync::RwLock;

use super::config::{render_my_ini, BindMode, ServerConfig};
use super::credentials::CredentialStore;
use super::errors::{classify, log_failure, ServerFailure};
use super::payload::PayloadDir;
use super::paths::ServerPaths;
use super::process;
use super::state_file::{self, ServerLifecycleState, ServerStateFile};
use super::upgrade::{self, Transition};

#[derive(Debug, Clone, PartialEq)]
pub enum ServerPhase {
    Provisioning,
    Starting,
    Upgrading,
    Running,
    Stopped,
    Failed(ServerFailure),
}

pub struct ServerRuntime {
    pub phase: ServerPhase,
    pub version: Option<String>,
    pub port: Option<u16>,
    /// Cached so `shutdown()` can issue a clean root `SHUTDOWN` without every caller having to pass
    /// the password through explicitly (A3: LAN toggle and the tray exit path both call `shutdown`
    /// from contexts that only hold the `ServerHandle`, not the `CredentialStore`). Cleared to
    /// `None` when the phase becomes `Stopped`/`Failed` — never persisted, never logged.
    pub root_password: Option<String>,
}

/// The live handle to a managed server. `ServerHandle::unmanaged()` is used by terminals and by
/// tests that don't exercise the server at all — its `state` simply stays `Stopped` and every
/// method that would spawn a process is never called.
pub struct ServerHandle {
    pub paths: ServerPaths,
    pub state: RwLock<ServerRuntime>,
    pub intentional_stop: AtomicBool,
    child: RwLock<Option<Child>>,
    /// The pid of the server this handle currently manages — set both when it spawns one
    /// (`start_and_wait`) and when it adopts one already running (`ensure_running` step 3, e.g. the
    /// server `provision_main` leaves running, or one that outlived a previous app process).
    /// `shutdown()` stops *this* pid: keying it only off `child` made `shutdown()` a silent no-op
    /// for an adopted server, so the next `ensure_running` re-adopted it unchanged — which is how
    /// A3's LAN toggle never actually rebound the server to `0.0.0.0`.
    running_pid: RwLock<Option<u32>>,
    restart_attempts: RwLock<Vec<std::time::Instant>>,
}

impl ServerHandle {
    pub fn unmanaged() -> Arc<Self> {
        Arc::new(Self {
            paths: ServerPaths::at(PathBuf::new()),
            state: RwLock::new(ServerRuntime { phase: ServerPhase::Stopped, version: None, port: None, root_password: None }),
            intentional_stop: AtomicBool::new(false),
            child: RwLock::new(None),
            running_pid: RwLock::new(None),
            restart_attempts: RwLock::new(Vec::new()),
        })
    }

    pub fn new(paths: ServerPaths) -> Arc<Self> {
        Arc::new(Self {
            paths,
            state: RwLock::new(ServerRuntime { phase: ServerPhase::Stopped, version: None, port: None, root_password: None }),
            intentional_stop: AtomicBool::new(false),
            child: RwLock::new(None),
            running_pid: RwLock::new(None),
            restart_attempts: RwLock::new(Vec::new()),
        })
    }

    async fn set_phase(&self, phase: ServerPhase) {
        let mut guard = self.state.write().await;
        // The root password only stays valid while a server this handle knows about is actually
        // running — clear it as soon as we no longer know that for certain, rather than risk using
        // a stale secret against a server that has since been reprovisioned.
        if matches!(phase, ServerPhase::Stopped | ServerPhase::Failed(_)) {
            guard.root_password = None;
        }
        guard.phase = phase;
    }

    async fn fail(&self, failure: ServerFailure) {
        let tail = read_log_tail(&self.paths).unwrap_or_default();
        log_failure(failure, &tail);
        self.set_phase(ServerPhase::Failed(failure)).await;
    }

    /// The main entry point: reads `server.json`, adopts an already-running server if one is
    /// verifiably ours, otherwise runs the version transition and starts a fresh one. Returns once
    /// the server answers `SELECT 1`, or a `ServerFailure` if it never does.
    pub async fn ensure_running(
        self: &Arc<Self>,
        payload: &PayloadDir,
        creds: &CredentialStore,
    ) -> Result<(), ServerFailure> {
        let Some(state) = state_file::load(&self.paths.server_json()).map_err(|_| ServerFailure::DataCorrupt)? else {
            // No server.json at all: nothing to ensure — the caller (core::state::boot) must run
            // provision_main first. Not a failure of this function; it simply has nothing to do.
            self.set_phase(ServerPhase::Stopped).await;
            return Ok(());
        };
        if !matches!(state.state, ServerLifecycleState::Ready) {
            // `initializing`: provisioning was interrupted; the caller resumes it via
            // `provision::provision_main`, which is idempotent for this exact case.
            self.set_phase(ServerPhase::Provisioning).await;
            return Err(ServerFailure::Unknown);
        }

        self.set_phase(ServerPhase::Starting).await;

        // Step 2: version gate — before adopting anything. A server still running from a previous
        // app version must never be adopted across a version change: a downgrade/series mismatch
        // is refused (data untouched), a patch upgrade stops it and upgrades below.
        let transition = upgrade::plan_transition(payload_version(payload), &state);
        if let Some(failure) = transition.as_failure() {
            self.fail(failure).await;
            return Err(failure);
        }

        // Step 3: adopt. If a pid file exists, names a live process whose image path is our
        // `server-bin\current\bin\mariadbd.exe`, and a root `SELECT @@datadir` matches our data
        // dir, reuse it without restarting (P2-58's guarantee makes this safe: nobody else could
        // have started a second, different server against this same data directory).
        if let Some(pid) = read_pid_file(&self.paths) {
            let expected_exe = self.paths.mariadbd_exe();
            if process::is_running(pid, &expected_exe) {
                let root_secret = creds.get_root_secret(state.data_dir_id).map_err(|_| ServerFailure::CredentialsUnrecoverable)?;
                if !matches!(transition, Transition::Same) {
                    // Patch upgrade: the data directory is about to be cold-copied, so the old
                    // server (not our child — `shutdown()` can't see it) must be fully stopped.
                    stop_running_server(state.port, &root_secret, pid).await;
                } else if verify_adopted_datadir(state.port, &root_secret, &self.paths.data()).await {
                    self.set_phase(ServerPhase::Running).await;
                    {
                        let mut guard = self.state.write().await;
                        guard.version = Some(state.last_started_with.clone());
                        guard.port = Some(state.port);
                        guard.root_password = Some(root_secret.clone());
                    }
                    *self.running_pid.write().await = Some(pid);
                    self.spawn_watcher(pid);
                    return Ok(());
                }
            }
        }

        // Step 4: start fresh (after the patch upgrade when there is one).

        rotate_log_if_large(&self.paths);

        if let Transition::PatchUpgrade { from, to } = &transition {
            self.set_phase(ServerPhase::Upgrading).await;
            if let Err(failure) = self.run_patch_upgrade(payload, creds, &state, from, to).await {
                self.fail(failure).await;
                return Err(failure);
            }
        } else {
            self.copy_payload_if_needed(payload).map_err(|_| ServerFailure::PayloadMissing)?;
        }

        let root_secret = creds.get_root_secret(state.data_dir_id).map_err(|_| ServerFailure::CredentialsUnrecoverable)?;
        self.start_and_wait(state.port, &root_secret, bind_mode_for(&state)).await
    }

    /// Copies the payload into `server-bin\current` when it's missing entirely (first-ever start
    /// after provisioning already did this — this is a defensive no-op in the common case).
    fn copy_payload_if_needed(&self, payload: &PayloadDir) -> std::io::Result<()> {
        if self.paths.mariadbd_exe().exists() {
            return Ok(());
        }
        copy_dir_recursive(&payload.root, &self.paths.server_bin_current())
    }

    async fn run_patch_upgrade(
        self: &Arc<Self>,
        payload: &PayloadDir,
        creds: &CredentialStore,
        state: &ServerStateFile,
        from: &str,
        to: &str,
    ) -> Result<(), ServerFailure> {
        use super::upgrade::runner;

        // Clean shutdown of whatever might still be running against the old binaries.
        let root_secret = creds.get_root_secret(state.data_dir_id).map_err(|_| ServerFailure::CredentialsUnrecoverable)?;
        self.shutdown(Duration::from_secs(60)).await;

        let data_size = runner::dir_size_bytes(&self.paths.data());
        let has_space = runner::has_enough_free_space(&self.paths, data_size).unwrap_or(false);
        if !has_space {
            log::error!(target: "infrastructure::database", "upgrade postponed: low disk space (need 2x{data_size} + 512MB)");
            // Don't swap — start the old `current` as-is; retried on the next start.
            return self.start_and_wait(state.port, &root_secret, bind_mode_for(state)).await;
        }

        runner::cold_copy_data(&self.paths, from, to).map_err(|_| ServerFailure::DiskFull)?;

        std::fs::create_dir_all(self.paths.server_bin_staging()).map_err(|_| ServerFailure::PayloadMissing)?;
        copy_dir_recursive(&payload.root, &self.paths.server_bin_staging()).map_err(|_| ServerFailure::PayloadMissing)?;
        runner::swap_server_bin(&self.paths).map_err(|_| ServerFailure::PayloadMissing)?;

        self.start_and_wait(state.port, &root_secret, bind_mode_for(state)).await?;

        match runner::run_mariadb_upgrade(&self.paths, state.port, &root_secret) {
            Ok(Ok(())) => {}
            Ok(Err(output)) => {
                log::error!(target: "infrastructure::database", "mariadb-upgrade exited non-zero: {output}");
                if let Some(mut s) = state_file::load(&self.paths.server_json()).ok().flatten() {
                    s.upgrade_warning = Some(output);
                    let _ = state_file::save(&self.paths.server_json(), &s);
                }
                // The server keeps running even though the upgrade tool failed.
            }
            Err(e) => log::error!(target: "infrastructure::database", "failed to run mariadb-upgrade: {e}"),
        }

        if let Some(mut s) = state_file::load(&self.paths.server_json()).ok().flatten() {
            s.last_started_with = to.to_string();
            s.updated_at = chrono::Utc::now();
            let _ = state_file::save(&self.paths.server_json(), &s);
        }

        Ok(())
    }

    /// Writes `my.ini`, spawns `mariadbd`, and polls `SELECT 1` every 250 ms for up to 120 s
    /// (InnoDB crash recovery can legitimately take that long). If the child process exits before
    /// readiness, classifies `error.log` and fails instead of waiting out the full timeout.
    /// `bind` (A3): `BindMode::Lan` while `server.json.lanSharing` is true, `Loopback` otherwise —
    /// every start path re-derives this from `server.json` via `bind_mode_for` so the bind mode can
    /// never drift from what was actually persisted.
    async fn start_and_wait(self: &Arc<Self>, port: u16, root_password: &str, bind: BindMode) -> Result<(), ServerFailure> {
        let cfg = ServerConfig { paths: &self.paths, port, bind };
        render_and_write(&cfg).map_err(|_| ServerFailure::AccessDenied)?;

        let mut child = process::spawn(&self.paths.mariadbd_exe(), &self.paths.my_ini(), &[]).map_err(|_| ServerFailure::PayloadMissing)?;
        let pid = child.id().ok_or(ServerFailure::Unknown)?;

        let deadline = std::time::Instant::now() + Duration::from_secs(120);
        loop {
            if let Ok(Some(_status)) = child.try_wait() {
                // Child already exited — classify from the log instead of waiting out the timeout.
                let tail = read_log_tail(&self.paths).unwrap_or_default();
                let failure = classify(&tail);
                self.fail(failure).await;
                return Err(failure);
            }
            if super::admin::probe_ready(port, "root", root_password).await {
                *self.child.write().await = Some(child);
                *self.running_pid.write().await = Some(pid);
                self.set_phase(ServerPhase::Running).await;
                {
                    let mut guard = self.state.write().await;
                    guard.port = Some(port);
                    guard.root_password = Some(root_password.to_string());
                }
                self.spawn_watcher(pid);
                return Ok(());
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.start_kill();
                self.fail(ServerFailure::StartTimeout).await;
                return Err(ServerFailure::StartTimeout);
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    /// Spawns the crash-watcher: waits for the process to exit, and if it wasn't an intentional
    /// stop (no `intentional_stop` flag, no fresh `shutdown.requested` marker), treats it as a
    /// crash and restarts with backoff (2 s / 5 s / 15 s), giving up after 3 attempts within 10
    /// minutes.
    fn spawn_watcher(self: &Arc<Self>, pid: u32) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            process::wait_exit(pid).await;
            if this.intentional_stop.swap(false, Ordering::SeqCst) {
                this.set_phase(ServerPhase::Stopped).await;
                return;
            }
            if let Some(marker_age) = shutdown_marker_age(&this.paths) {
                if marker_age < Duration::from_secs(5 * 60) {
                    let _ = std::fs::remove_file(this.paths.shutdown_requested());
                    this.set_phase(ServerPhase::Stopped).await;
                    return;
                }
            }

            // Unexpected exit: a crash. Restart with backoff.
            let mut attempts = this.restart_attempts.write().await;
            let now = std::time::Instant::now();
            attempts.retain(|t| now.duration_since(*t) < Duration::from_secs(10 * 60));
            if attempts.len() >= 3 {
                drop(attempts);
                this.fail(ServerFailure::CrashLoop).await;
                return;
            }
            let backoff = [Duration::from_secs(2), Duration::from_secs(5), Duration::from_secs(15)][attempts.len().min(2)];
            attempts.push(now);
            drop(attempts);

            tokio::time::sleep(backoff).await;
            log::error!(target: "infrastructure::database", "mariadbd exited unexpectedly, restarting after {backoff:?}");
            // The next `ensure_running` call (triggered by the caller's own retry loop, or the
            // periodic health task in A2-10) will re-adopt-or-restart cleanly from `server.json`.
            this.set_phase(ServerPhase::Stopped).await;
        });
    }

    /// Clean stop: sets `intentional_stop`, issues a root `SHUTDOWN`, waits for the process to
    /// exit, and on timeout falls back to `terminate` (InnoDB will run crash recovery on the next
    /// start, which is safe — just slower).
    /// Stops the managed server whether this handle spawned it or adopted it (`running_pid`).
    pub async fn shutdown(self: &Arc<Self>, timeout: Duration) {
        let child_pid = {
            let guard = self.child.read().await;
            guard.as_ref().and_then(|c| c.id())
        };
        let managed_pid = child_pid.or(*self.running_pid.read().await);
        let Some(pid) = managed_pid.filter(|pid| process::is_running(*pid, &self.paths.mariadbd_exe())) else {
            // Nothing of ours is running: leave `intentional_stop` unset, since no watcher will
            // consume it and a later genuine crash must not be mistaken for a requested stop.
            *self.child.write().await = None;
            *self.running_pid.write().await = None;
            self.set_phase(ServerPhase::Stopped).await;
            return;
        };
        self.intentional_stop.store(true, Ordering::SeqCst);

        let port = self.state.read().await.port;
        if let (Some(port), Some(root_password)) = (port, self.cached_root_password().await) {
            if let Ok(db) = super::admin::connect_root(port, "root", &root_password, None).await {
                let _ = super::admin::shutdown(&db).await;
            }
        }

        let waited = tokio::time::timeout(timeout, process::wait_exit(pid)).await;
        if waited.is_err() {
            log::error!(target: "infrastructure::database", "clean shutdown timed out after {timeout:?}; terminating");
            // Re-check the image path: never terminate a pid that has since been reused.
            if process::is_running(pid, &self.paths.mariadbd_exe()) {
                process::terminate(pid);
            }
        }
        *self.child.write().await = None;
        *self.running_pid.write().await = None;
        self.set_phase(ServerPhase::Stopped).await;
    }

    /// The root password cached in `ServerRuntime` the last time this handle successfully started
    /// or adopted the server (A3 fix: previously always `None`, which meant `shutdown()` could
    /// never issue a clean root `SHUTDOWN` and always fell through to the timeout+terminate path).
    async fn cached_root_password(&self) -> Option<String> {
        self.state.read().await.root_password.clone()
    }
}

/// A3-1/P2-56: the bind mode a start must use is always re-derived from `server.json.lanSharing`,
/// never cached anywhere else — so a start can never silently disagree with what was persisted.
fn bind_mode_for(state: &ServerStateFile) -> BindMode {
    if state.lan_sharing {
        BindMode::Lan
    } else {
        BindMode::Loopback
    }
}

/// Stops a server this handle did not spawn (found through the pid file): a clean root
/// `SHUTDOWN`, then terminate if it hasn't exited within 60 s.
async fn stop_running_server(port: u16, root_secret: &str, pid: u32) {
    if let Ok(db) = super::admin::connect_root(port, "root", root_secret, None).await {
        let _ = super::admin::shutdown(&db).await;
    }
    if tokio::time::timeout(Duration::from_secs(60), process::wait_exit(pid)).await.is_err() {
        log::error!(target: "infrastructure::database", "old server did not stop within 60s before upgrade; terminating");
        process::terminate(pid);
    }
}

fn render_and_write(cfg: &ServerConfig) -> std::io::Result<()> {
    cfg.paths.ensure_dirs()?;
    let text = render_my_ini(cfg);
    let path = cfg.paths.my_ini();
    let tmp_path = path.with_extension("ini.tmp");
    std::fs::write(&tmp_path, text)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

fn payload_version(payload: &PayloadDir) -> &'static str {
    // The manifest's version is already validated (series+version) at read time; A2-4's constant
    // is authoritative for what this *build* supports, so use it directly here rather than
    // re-reading the manifest on every transition check.
    let _ = payload;
    super::payload::BUNDLED_VERSION
}

fn read_pid_file(paths: &ServerPaths) -> Option<u32> {
    std::fs::read_to_string(paths.pid_file()).ok()?.trim().parse().ok()
}

fn shutdown_marker_age(paths: &ServerPaths) -> Option<Duration> {
    let meta = std::fs::metadata(paths.shutdown_requested()).ok()?;
    let modified = meta.modified().ok()?;
    modified.elapsed().ok()
}

fn read_log_tail(paths: &ServerPaths) -> Option<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(paths.error_log()).ok()?;
    let len = file.metadata().ok()?.len();
    let start = len.saturating_sub(64 * 1024);
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = String::new();
    file.read_to_string(&mut buf).ok()?;
    Some(buf)
}

/// Rotates `error.log` at start when it exceeds 10 MB, keeping `.1`/`.2`/`.3`.
fn rotate_log_if_large(paths: &ServerPaths) {
    let log_path = paths.error_log();
    let Ok(meta) = std::fs::metadata(&log_path) else { return };
    if meta.len() <= 10 * 1024 * 1024 {
        return;
    }
    let _ = std::fs::remove_file(log_path.with_extension("log.3"));
    for n in (1..=2).rev() {
        let from = log_path.with_extension(format!("log.{n}"));
        let to = log_path.with_extension(format!("log.{}", n + 1));
        let _ = std::fs::rename(from, to);
    }
    let _ = std::fs::rename(&log_path, log_path.with_extension("log.1"));
}

async fn verify_adopted_datadir(port: u16, root_password: &str, expected_datadir: &std::path::Path) -> bool {
    use sea_orm::{ConnectionTrait, Statement};
    let Ok(db) = super::admin::connect_root(port, "root", root_password, None).await else {
        return false;
    };
    let stmt = Statement::from_string(db.get_database_backend(), "SELECT @@datadir AS d".to_string());
    let Ok(Some(row)) = db.query_one(stmt).await else {
        return false;
    };
    let Ok(datadir): Result<String, _> = row.try_get("", "d") else {
        return false;
    };
    // `@@datadir` comes back with backslashes and a trailing separator on Windows; normalise
    // both sides the same way before comparing.
    let actual = datadir.replace('\\', "/");
    let actual = actual.trim_end_matches('/');
    let expected = expected_datadir.to_string_lossy().replace('\\', "/");
    let expected = expected.trim_end_matches('/');
    actual.eq_ignore_ascii_case(expected)
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}
