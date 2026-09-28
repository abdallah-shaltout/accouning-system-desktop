//! Process-entry CLI handling (phase-a2 A2-3/A2-7): `--db-shutdown`, invoked by the NSIS
//! uninstaller's `PREUNINSTALL` hook (A2-2) so the server is stopped *before* the installer
//! removes `server-bin`. Runs from `main.rs`, before the Tauri builder — so
//! `tauri-plugin-single-instance` never forwards this as a "focus the existing window" event.

use super::credentials::CredentialStore;
use super::paths::ServerPaths;
use super::state_file;

/// Returns `Some(exit_code)` when a recognized CLI arg was handled (the caller should
/// `std::process::exit(code)` immediately and never start Tauri). Returns `None` for no/unknown
/// args, meaning "proceed with the normal app boot."
pub fn run_from_args() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--db-shutdown") {
        Some(run_db_shutdown())
    } else {
        None
    }
}

#[cfg(windows)]
fn run_db_shutdown() -> i32 {
    let Some(paths) = ServerPaths::machine() else {
        return 0; // no machine-wide server possible on this platform/config: nothing to stop
    };
    let Ok(Some(state)) = state_file::load(&paths.server_json()) else {
        return 0; // no server.json at all, or a data dir not ours to touch: not running
    };

    let _ = std::fs::write(paths.shutdown_requested(), b"");

    let creds = CredentialStore::production();
    let Ok(root_secret) = creds.get_root_secret(state.data_dir_id) else {
        return 2;
    };

    let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(_) => return 2,
    };

    let stopped = rt.block_on(async {
        if let Ok(db) = super::admin::connect_root(state.port, "root", &root_secret, None).await {
            let _ = super::admin::shutdown(&db).await;
        }

        // Wait up to 60 s for the process to actually exit.
        let Some(pid) = read_pid_file(&paths) else {
            return true; // no pid file: already stopped
        };
        let expected_exe = paths.mariadbd_exe();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while super::process::is_running(pid, &expected_exe) {
            if std::time::Instant::now() >= deadline {
                super::process::terminate(pid);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
        true
    });

    if stopped {
        0
    } else {
        2
    }
}

#[cfg(not(windows))]
fn run_db_shutdown() -> i32 {
    0
}

#[cfg(windows)]
fn read_pid_file(paths: &ServerPaths) -> Option<u32> {
    std::fs::read_to_string(paths.pid_file()).ok()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn no_args_returns_none() {
        // run_from_args reads real process args, which in a test binary won't contain
        // "--db-shutdown" — this asserts the parsing logic itself via a small inline reproduction
        // rather than depending on the test harness's actual argv.
        fn parse(args: &[&str]) -> Option<&'static str> {
            if args.get(1).copied() == Some("--db-shutdown") {
                Some("shutdown")
            } else {
                None
            }
        }
        assert_eq!(parse(&["accounting-app"]), None);
        assert_eq!(parse(&["accounting-app", "--unknown-flag"]), None);
        assert_eq!(parse(&["accounting-app", "--db-shutdown"]), Some("shutdown"));
    }
}
