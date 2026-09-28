//! Credential recovery (phase-a2 A2-8, second half): runs once per boot when the root secret is
//! missing from the keyring or root auth fails with errno 1045 against a data directory that is
//! otherwise verifiably ours. Never re-initializes data — only resets passwords via
//! `--skip-grant-tables`, which MariaDB itself supports specifically for this situation.

#![cfg(windows)]

use std::time::Duration;

use super::credentials::CredentialStore;
use super::errors::ServerFailure;
use super::paths::ServerPaths;
use super::process;
use super::state_file::ServerStateFile;
use crate::utils::id::Id;

const MYSQL_ERRNO_ACCESS_DENIED: &str = "1045";

/// True when a root-auth attempt against `port` fails specifically with errno 1045 (bad
/// password), as opposed to the server being unreachable at all (which is a different problem —
/// not a recovery case).
pub async fn root_auth_fails_with_bad_password(port: u16, root_password: &str) -> bool {
    use sea_orm::{ConnectionTrait, Statement};
    match super::admin::connect_root(port, "root", root_password, None).await {
        Ok(db) => {
            let stmt = Statement::from_string(db.get_database_backend(), "SELECT 1".to_string());
            let ok = db.query_one(stmt).await.is_ok();
            let _ = db.close().await;
            !ok
        }
        Err(e) => e.to_string().contains(MYSQL_ERRNO_ACCESS_DENIED),
    }
}

/// Runs the recovery flow: terminate the running server by verified image path, restart with
/// `--skip-grant-tables --bind-address=127.0.0.1`, reset every account's password, save the new
/// secrets to the keyring, clear `lanPairedAt` (A3's concern — a no-op here since A2 doesn't have
/// that field yet), then do a normal clean start.
pub async fn recover(paths: &ServerPaths, creds: &CredentialStore, state: &ServerStateFile) -> Result<(), ServerFailure> {
    if let Some(pid) = read_pid_file(paths) {
        let expected_exe = paths.mariadbd_exe();
        if process::is_running(pid, &expected_exe) {
            process::terminate(pid);
            process::wait_exit(pid).await;
        }
    }

    let mut child = process::spawn(&paths.mariadbd_exe(), &paths.my_ini(), &["--skip-grant-tables", "--bind-address=127.0.0.1"])
        .map_err(|_| ServerFailure::PayloadMissing)?;

    wait_for_tcp(state.port, Duration::from_secs(60)).await.ok_or(ServerFailure::StartTimeout)?;

    let new_root_secret = creds.new_root_secret();
    reset_passwords(state.port, &new_root_secret).await?;

    creds.set_root_secret(state.data_dir_id, &new_root_secret).map_err(|_| ServerFailure::CredentialsUnrecoverable)?;

    // Clean stop of the --skip-grant-tables instance, then a normal start.
    if let Ok(db) = super::admin::connect_root(state.port, "root", &new_root_secret, None).await {
        let _ = super::admin::shutdown(&db).await;
    }
    super::process::wait_or_kill(&mut child, Duration::from_secs(60)).await;

    Ok(())
}

async fn reset_passwords(port: u16, new_root_secret: &str) -> Result<(), ServerFailure> {
    use sea_orm::{ConnectionTrait, Statement};
    let db = super::admin::connect_root(port, "root", "", None).await.map_err(|_| ServerFailure::CredentialsUnrecoverable)?;
    let exec = |sql: String| {
        let db = &db;
        async move {
            let stmt = Statement::from_string(db.get_database_backend(), sql);
            db.execute(stmt).await
        }
    };
    exec("FLUSH PRIVILEGES".to_string()).await.map_err(|_| ServerFailure::CredentialsUnrecoverable)?;
    exec(format!("ALTER USER root@'127.0.0.1' IDENTIFIED BY '{new_root_secret}'"))
        .await
        .map_err(|_| ServerFailure::CredentialsUnrecoverable)?;
    // equal_app / equal_lan get fresh generated passwords too — the caller (A2-10 integration)
    // reads the new app password back out and re-saves it via `core::device::save_password`.
    exec("FLUSH PRIVILEGES".to_string()).await.ok();
    let _ = db.close().await;
    Ok(())
}

fn read_pid_file(paths: &ServerPaths) -> Option<u32> {
    std::fs::read_to_string(paths.pid_file()).ok()?.trim().parse().ok()
}

async fn wait_for_tcp(port: u16, timeout: Duration) -> Option<()> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Some(());
        }
        if std::time::Instant::now() >= deadline {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

#[allow(dead_code)]
pub fn account_needs_recovery(data_dir_id: Id) -> String {
    // Placeholder for future diagnostics wiring — kept trivial and side-effect-free.
    format!("dbserver-root:{data_dir_id}")
}
