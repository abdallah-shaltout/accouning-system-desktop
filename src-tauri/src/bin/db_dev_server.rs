//! `bun run db:dev` — a foreground bundled-MariaDB server for local development and testing
//! (phase-a2 A2-11, P2-53/P2-54). Provisions (if needed) and starts a server at
//! `src-tauri/target/mariadb-dev/`, port 3499, fixed root password `equal-dev`, prints the
//! `EQUAL_TEST_DATABASE_URL` a developer copies into their shell before running
//! `cargo test --manifest-path src-tauri/Cargo.toml`, and shuts down cleanly on Ctrl+C.
//!
//! This is the server that closes Phase A's never-run DB gate (its 8 DB-backed tests) and every
//! later DB-backed test — nothing else in this repo provides a real MariaDB in a headless/dev
//! environment.

/// The fixed dev/test port (P2-53): outside the production 3406–3415 candidate range.
#[cfg(windows)]
const DEV_PORT: u16 = 3499;

#[cfg(windows)]
#[tokio::main]
async fn main() {
    use accounting_app_lib::infrastructure::database::{
        credentials::CredentialStore,
        payload::PayloadDir,
        paths::ServerPaths,
        provision::{self, ProvisionResult},
        state_file,
        supervisor::ServerHandle,
    };
    use accounting_app_lib::utils::id::Id;

    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("mariadb-dev");
    let paths = ServerPaths::at(root);
    let payload = PayloadDir::dev();
    let creds = CredentialStore::fixed("equal-dev");

    if let Err(e) = payload.read_manifest() {
        eprintln!("[db_dev_server] MariaDB payload not found or invalid: {e}");
        eprintln!("[db_dev_server] run `node scripts/fetch-mariadb.js` first.");
        std::process::exit(1);
    }

    println!("[db_dev_server] provisioning/starting at {}", paths.root.display());

    // A fixed dev terminal id — this server has no notion of "which terminal owns it" the way the
    // real Main PC provisioning does, and db_dev_server never writes device-settings.json (no
    // `app_data_dir` is passed) per its own spec.
    let host_terminal_id = Id::new();

    let existing = state_file::load(&paths.server_json()).ok().flatten();
    match existing {
        Some(s) if matches!(s.state, state_file::ServerLifecycleState::Ready) => {
            println!("[db_dev_server] existing data directory found, reattaching.");
        }
        _ => match provision::provision_main_on_port(&paths, &payload, &creds, host_terminal_id, None, Some(DEV_PORT)).await {
            Ok(ProvisionResult::Provisioned(_)) => println!("[db_dev_server] provisioned a fresh data directory."),
            Ok(ProvisionResult::AlreadyProvisioned(_)) => {}
            Err(e) => {
                eprintln!("[db_dev_server] provisioning failed: {e}");
                std::process::exit(1);
            }
        },
    }

    let server = ServerHandle::new(paths.clone());
    if let Err(e) = server.ensure_running(&payload, &creds).await {
        eprintln!("[db_dev_server] failed to start: {e}");
        std::process::exit(1);
    }

    let state = state_file::load(&paths.server_json()).ok().flatten().expect("server.json must exist after a successful start");
    println!("[db_dev_server] ready.");
    println!("EQUAL_TEST_DATABASE_URL=mysql://root:equal-dev@127.0.0.1:{}", state.port);
    println!("[db_dev_server] press Ctrl+C to stop cleanly.");

    tokio::signal::ctrl_c().await.expect("failed to listen for Ctrl+C");
    println!("[db_dev_server] shutting down...");
    server.shutdown(std::time::Duration::from_secs(30)).await;
    println!("[db_dev_server] stopped.");
}

#[cfg(not(windows))]
fn main() {
    eprintln!("[db_dev_server] the bundled MariaDB server is Windows-only.");
    std::process::exit(1);
}
