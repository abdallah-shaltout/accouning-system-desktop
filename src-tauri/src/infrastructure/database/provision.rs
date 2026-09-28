//! First-time server provisioning (phase-a2 A2-8, P2-49/P2-50): creates the data directory, runs
//! `mariadb-install-db`, locks down root, creates the `equal` schema and `equal_app` user, then
//! flips `server.json` to `ready`. **Idempotent and resumable** — if the process is killed
//! mid-provisioning, `server.json` is left at `state: initializing` and the next call safely
//! starts over (deleting `data\` is only ever safe *because* no migration and no business write
//! can happen before `ready`).

#![cfg(windows)]

use std::net::TcpListener;
use std::process::Stdio;
use std::time::Duration;

use chrono::Utc;

use super::config::{render_my_ini, BindMode, ServerConfig};
use super::credentials::CredentialStore;
use super::errors::ServerFailure;
use super::payload::PayloadDir;
use super::paths::ServerPaths;
use super::process;
use super::state_file::{self, ServerLifecycleState, ServerStateFile};
use crate::utils::id::Id;

pub enum ProvisionResult {
    AlreadyProvisioned(ServerStateFile),
    Provisioned(ServerStateFile),
}

const CANDIDATE_PORTS: [u16; 10] = [3406, 3407, 3408, 3409, 3410, 3411, 3412, 3413, 3414, 3415];

/// Finds the first candidate port where binding both `127.0.0.1:p` and `0.0.0.0:p` succeeds (the
/// listeners are dropped immediately after the check — this only proves the port is free right
/// now, same as any "pick a free port" check has to accept).
fn pick_free_port() -> Option<u16> {
    CANDIDATE_PORTS.into_iter().find(|&p| port_is_free(p))
}

fn port_is_free(p: u16) -> bool {
    let loopback = TcpListener::bind(("127.0.0.1", p));
    let any = TcpListener::bind(("0.0.0.0", p));
    loopback.is_ok() && any.is_ok()
}

const README_TEXT: &str = "هذا المجلد يحتوي بيانات نشاطك التجاري في ايكوال المحاسبي.\n\
لا تحذف هذا المجلد ولا أي ملف بداخله.\n\
النسخ الاحتياطي يُدار من داخل البرنامج (الإعدادات ← النسخ الاحتياطي) ولا يعتمد على نسخ هذا المجلد يدوياً.\n\
عند إلغاء تثبيت البرنامج، تبقى بياناتك هنا حتى تُثبّته مرة أخرى على نفس الجهاز.\n";

/// The full provisioning pipeline. Safe to call repeatedly: a `ready` state file returns
/// immediately (`AlreadyProvisioned`), an `initializing` one resumes from scratch.
/// `app_data_dir` is `None` for `db_dev_server` (A2-11 — provisions a database with no device
/// settings at all, per its own spec) and `Some(dir)` for the real app (writes
/// `device-settings.json` role/connection, C-23).
pub async fn provision_main(
    paths: &ServerPaths,
    payload: &PayloadDir,
    creds: &CredentialStore,
    host_terminal_id: Id,
    app_data_dir: Option<&std::path::Path>,
) -> Result<ProvisionResult, ServerFailure> {
    provision_main_on_port(paths, payload, creds, host_terminal_id, app_data_dir, None).await
}

/// `provision_main` with an optional fixed port (`db_dev_server` asks for 3499 so it never collides
/// with a real Main PC's 3406–3415 range on the same developer machine, A2-11). `None` picks the
/// first free candidate port; a fixed port that is busy fails with `PortUnavailable`.
pub async fn provision_main_on_port(
    paths: &ServerPaths,
    payload: &PayloadDir,
    creds: &CredentialStore,
    host_terminal_id: Id,
    app_data_dir: Option<&std::path::Path>,
    fixed_port: Option<u16>,
) -> Result<ProvisionResult, ServerFailure> {
    if let Some(existing) = state_file::load(&paths.server_json()).map_err(|_| ServerFailure::DataCorrupt)? {
        if matches!(existing.state, ServerLifecycleState::Ready) {
            return Ok(ProvisionResult::AlreadyProvisioned(existing));
        }
        // `initializing`: a previous attempt was interrupted. No migration and no business write
        // ever happens before `ready`, so it is safe — and only safe for this exact reason — to
        // wipe `data\` and start over.
        let _ = std::fs::remove_dir_all(paths.data());
    }

    payload.read_manifest().map_err(|_| ServerFailure::PayloadMissing)?;

    paths.ensure_dirs().map_err(|_| ServerFailure::AccessDenied)?;

    let data_dir_id = Id::new();
    let port = match fixed_port {
        Some(p) if port_is_free(p) => p,
        Some(_) => return Err(ServerFailure::PortUnavailable),
        None => pick_free_port().ok_or(ServerFailure::PortUnavailable)?,
    };
    let now = Utc::now();
    let initializing_state = ServerStateFile {
        version: 1,
        state: ServerLifecycleState::Initializing,
        data_dir_id,
        series: super::payload::SUPPORTED_SERIES.to_string(),
        initialized_with: super::payload::BUNDLED_VERSION.to_string(),
        last_started_with: super::payload::BUNDLED_VERSION.to_string(),
        port,
        lan_sharing: false,
        host_terminal_id,
        created_at: now,
        updated_at: now,
        upgrade_warning: None,
    };
    state_file::save(&paths.server_json(), &initializing_state).map_err(|_| ServerFailure::AccessDenied)?;
    std::fs::write(paths.readme(), README_TEXT.as_bytes()).map_err(|_| ServerFailure::AccessDenied)?;

    // Copy payload into server-bin/staging → verify → rename to current.
    let staging = paths.server_bin_staging();
    std::fs::create_dir_all(&staging).map_err(|_| ServerFailure::AccessDenied)?;
    copy_dir_recursive(&payload.root, &staging).map_err(|_| ServerFailure::PayloadMissing)?;
    let current = paths.server_bin_current();
    if current.exists() {
        std::fs::remove_dir_all(&current).map_err(|_| ServerFailure::AccessDenied)?;
    }
    std::fs::rename(&staging, &current).map_err(|_| ServerFailure::PayloadMissing)?;

    let root_secret = creds.new_root_secret();
    creds.set_root_secret(data_dir_id, &root_secret).map_err(|_| ServerFailure::CredentialsUnrecoverable)?;

    run_install_db(paths, &root_secret, port)?;
    // mariadb-install-db writes its own my.ini into the data dir; we always run from our own
    // generated one instead, so remove it to avoid two configs disagreeing.
    let _ = std::fs::remove_file(paths.data().join("my.ini"));

    let cfg = ServerConfig { paths, port, bind: BindMode::Loopback };
    render_and_write(&cfg).map_err(|_| ServerFailure::AccessDenied)?;

    let mut child = process::spawn(&paths.mariadbd_exe(), &paths.my_ini(), &["--skip-grant-tables"])
        .map_err(|_| ServerFailure::PayloadMissing)?;
    wait_for_ready_or_fail(&mut child, paths, port, None).await?;

    lock_down_and_create_app_user(port, &root_secret, app_data_dir).await?;

    // Restart cleanly without --skip-grant-tables now that accounts exist.
    if let Ok(db) = super::admin::connect_root(port, "root", &root_secret, None).await {
        let _ = super::admin::shutdown(&db).await;
    }
    super::process::wait_or_kill(&mut child, Duration::from_secs(60)).await;
    let mut child = process::spawn(&paths.mariadbd_exe(), &paths.my_ini(), &[]).map_err(|_| ServerFailure::PayloadMissing)?;
    wait_for_ready_or_fail(&mut child, paths, port, Some(&root_secret)).await?;
    std::mem::forget(child); // hand off to the supervisor's own watcher on the next ensure_running

    let mut ready_state = initializing_state;
    ready_state.state = ServerLifecycleState::Ready;
    ready_state.updated_at = Utc::now();
    state_file::save(&paths.server_json(), &ready_state).map_err(|_| ServerFailure::AccessDenied)?;

    Ok(ProvisionResult::Provisioned(ready_state))
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

fn run_install_db(paths: &ServerPaths, root_password: &str, port: u16) -> Result<(), ServerFailure> {
    let exe = paths.mariadb_install_db_exe();
    let bin_dir = exe.parent().map(|p| p.to_path_buf());
    let mut cmd = std::process::Command::new(&exe);
    cmd.arg(format!("--datadir={}", paths.data().display()))
        .arg(format!("--password={root_password}"))
        .arg(format!("--port={port}"))
        .arg("--allow-remote-root-access")
        .arg("--silent")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = bin_dir {
        cmd.current_dir(dir);
    }
    let output = cmd.output().map_err(|_| ServerFailure::PayloadMissing)?;
    if !output.status.success() {
        log::error!(
            target: "infrastructure::database",
            "mariadb-install-db failed: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let combined = format!("{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        return Err(super::errors::classify(&combined));
    }
    Ok(())
}

async fn wait_for_ready_or_fail(
    child: &mut tokio::process::Child,
    paths: &ServerPaths,
    port: u16,
    root_password: Option<&str>,
) -> Result<(), ServerFailure> {
    let deadline = std::time::Instant::now() + Duration::from_secs(120);
    loop {
        if let Ok(Some(_status)) = child.try_wait() {
            let tail = read_log_tail(paths).unwrap_or_default();
            return Err(super::errors::classify(&tail));
        }
        let ready = match root_password {
            Some(pw) => super::admin::probe_ready(port, "root", pw).await,
            // With --skip-grant-tables there's no password yet; a bare TCP connect to the port is
            // the only readiness signal available at this step.
            None => std::net::TcpStream::connect(("127.0.0.1", port)).is_ok(),
        };
        if ready {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.start_kill();
            return Err(ServerFailure::StartTimeout);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

/// Locks down root to `127.0.0.1`-only, drops anonymous accounts and the `test` database, then
/// creates the `equal` schema and the `equal_app` least-privilege user (P2-49/P2-50 step 7).
async fn lock_down_and_create_app_user(
    port: u16,
    root_secret: &str,
    app_data_dir: Option<&std::path::Path>,
) -> Result<(), ServerFailure> {
    use sea_orm::{ConnectionTrait, Statement};
    // Under --skip-grant-tables every statement is allowed regardless of user/password.
    let db = super::admin::connect_root(port, "root", "", None).await.map_err(|_| ServerFailure::Unknown)?;
    let exec = |sql: String| {
        let db = &db;
        async move {
            let stmt = Statement::from_string(db.get_database_backend(), sql);
            db.execute(stmt).await
        }
    };

    exec("FLUSH PRIVILEGES".to_string()).await.map_err(|_| ServerFailure::Unknown)?;
    exec(format!("CREATE USER IF NOT EXISTS root@'127.0.0.1' IDENTIFIED BY '{root_secret}'"))
        .await
        .map_err(|_| ServerFailure::Unknown)?;
    exec("GRANT ALL PRIVILEGES ON *.* TO root@'127.0.0.1' WITH GRANT OPTION".to_string())
        .await
        .map_err(|_| ServerFailure::Unknown)?;
    exec("DROP USER IF EXISTS root@'%'".to_string()).await.ok();
    exec("DELETE FROM mysql.user WHERE User=''".to_string()).await.ok();
    exec("DROP DATABASE IF EXISTS test".to_string()).await.ok();
    exec("CREATE DATABASE IF NOT EXISTS equal CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci".to_string())
        .await
        .map_err(|_| ServerFailure::Unknown)?;

    let app_password = super::credentials::generate_secret(32);
    exec(format!("CREATE USER IF NOT EXISTS equal_app@'127.0.0.1' IDENTIFIED BY '{app_password}'"))
        .await
        .map_err(|_| ServerFailure::Unknown)?;
    exec(
        "GRANT SELECT, INSERT, UPDATE, DELETE, CREATE, ALTER, DROP, INDEX, REFERENCES, CREATE TEMPORARY TABLES, \
         LOCK TABLES, CREATE VIEW, SHOW VIEW ON equal.* TO equal_app@'127.0.0.1'"
            .to_string(),
    )
    .await
    .map_err(|_| ServerFailure::Unknown)?;
    exec("FLUSH PRIVILEGES".to_string()).await.ok();

    // Save the app password (P2-29's keyring account convention) and write device-settings.json
    // (C-23) so `core::db` connects with the unchanged Phase A path on the next boot. Skipped
    // entirely when `app_data_dir` is `None` (db_dev_server — no device settings at all, A2-11).
    if let Some(dir) = app_data_dir {
        let connection = crate::core::device::ConnectionSettings {
            host: "127.0.0.1".to_string(),
            port,
            database: "equal".to_string(),
            user: "equal_app".to_string(),
            last_known_address: None,
        };
        crate::core::device::save_password(&connection, &app_password).map_err(|_| ServerFailure::CredentialsUnrecoverable)?;
        let mut settings = crate::core::device::load(dir).unwrap_or_default();
        settings.role = crate::core::device::DeviceRole::Main;
        settings.connection = Some(connection);
        let _ = crate::core::device::save(dir, &settings);
    }

    let _ = db.close().await;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A2's test list requires proving port selection skips a port that's already bound. Holding
    /// the default port (3406) on both `127.0.0.1` and `0.0.0.0` — the same pair `pick_free_port`
    /// itself checks — must push the pick to the next candidate.
    #[test]
    fn port_selection_skips_a_held_default_port() {
        let held_loopback = TcpListener::bind(("127.0.0.1", CANDIDATE_PORTS[0]));
        let held_any = TcpListener::bind(("0.0.0.0", CANDIDATE_PORTS[0]));
        let (Ok(_held_loopback), Ok(_held_any)) = (held_loopback, held_any) else {
            // Some other process already holds 3406 on this machine/CI runner — the property this
            // test exists to prove (skip a held port) can't be isolated here, so don't fail the
            // build over host port contention outside this test's control.
            eprintln!("port_selection_skips_a_held_default_port: could not hold port {} to test with, skipping", CANDIDATE_PORTS[0]);
            return;
        };

        let picked = pick_free_port().expect("a free port must be found among the candidates");
        assert_ne!(picked, CANDIDATE_PORTS[0], "must not pick a port that is already bound");
        assert!(CANDIDATE_PORTS.contains(&picked), "picked port must be one of the declared candidates");
    }
}
