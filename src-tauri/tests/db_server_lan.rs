//! Integration tests for A3-1's LAN sharing enable/disable/rotate (phase-a3, Windows-only). Same
//! fixture pattern as `tests/db_server_lifecycle.rs`: a real MariaDB payload, a temp
//! `ServerPaths::at(...)`, a per-test keyring service, random free ports. The firewall step is
//! injected as `FirewallMode::Skip` so this suite never triggers a UAC prompt or touches the real
//! Windows Firewall — only the MariaDB-account/bind-mode/state-file side of A3-1 is exercised here.
#![cfg(windows)]

use std::path::PathBuf;

use accounting_app_lib::infrastructure::database::{
    credentials::CredentialStore,
    lan::{self, FirewallMode},
    payload::PayloadDir,
    paths::ServerPaths,
    provision::{self, ProvisionResult},
    state_file::{self, ServerLifecycleState},
    supervisor::ServerHandle,
};
use accounting_app_lib::utils::id::Id;

/// Each test here starts a real mariadbd. Production runs exactly one server per machine (P2-58),
/// so these tests run one at a time instead of racing each other for the same candidate port.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

fn temp_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("equal-db-lan-{label}-{}", Id::new()))
}

fn require_payload() -> PayloadDir {
    let payload = PayloadDir::dev();
    if payload.read_manifest().is_err() {
        panic!(
            "MariaDB payload not found at {} — run `node scripts/fetch-mariadb.js` first (A2-11).",
            payload.root.display()
        );
    }
    payload
}

struct Fixture {
    paths: ServerPaths,
    creds: CredentialStore,
    payload: PayloadDir,
    data_dir_id: Option<Id>,
}

impl Fixture {
    fn new(label: &str) -> Self {
        Self { paths: ServerPaths::at(temp_root(label)), creds: CredentialStore::for_test(), payload: require_payload(), data_dir_id: None }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // A failed assertion skips the test's own `shutdown`: stop this fixture's server here so it
        // can't hold the port/files and break the next (serialised) test.
        if let Some(pid) = std::fs::read_to_string(self.paths.pid_file()).ok().and_then(|s| s.trim().parse::<u32>().ok()) {
            use accounting_app_lib::infrastructure::database::process;
            if process::is_running(pid, &self.paths.mariadbd_exe()) {
                process::terminate(pid);
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
        if let Some(id) = self.data_dir_id {
            self.creds.delete_all(id);
        }
        let _ = std::fs::remove_dir_all(&self.paths.root);
    }
}

async fn provision(fixture: &mut Fixture) -> state_file::ServerStateFile {
    let result = provision::provision_main(&fixture.paths, &fixture.payload, &fixture.creds, Id::new(), None)
        .await
        .expect("provisioning must succeed");
    let state = match result {
        ProvisionResult::Provisioned(s) => s,
        ProvisionResult::AlreadyProvisioned(s) => s,
    };
    fixture.data_dir_id = Some(state.data_dir_id);
    state
}

/// Connects with a bare mysql:// URL as a given user/password over TCP to a specific host — used
/// here to prove a non-loopback connection is refused before LAN sharing and accepted after.
async fn try_connect_tcp(host: &str, port: u16, user: &str, password: &str) -> Result<sea_orm::DatabaseConnection, sea_orm::DbErr> {
    use sea_orm::ConnectOptions;
    let mut opts = ConnectOptions::new(format!("mysql://{user}:{password}@{host}:{port}/equal"));
    opts.max_connections(1).connect_timeout(std::time::Duration::from_secs(3)).sqlx_logging(false);
    sea_orm::Database::connect(opts).await
}

/// The machine's own first non-loopback IPv4 — the address a terminal would actually connect
/// through. Every CI/dev box that can run this Windows-only integration suite at all has at least
/// one such interface (even a disconnected Ethernet/Wi-Fi adapter usually still has a link-local or
/// a DHCP-less private address; if truly none is found the test explains why it skipped instead of
/// failing for an environment reason unrelated to the property under test).
fn a_local_non_loopback_ipv4() -> Option<String> {
    accounting_app_lib::infrastructure::database::pairing::non_loopback_ipv4_addresses().into_iter().next()
}

#[tokio::test]
async fn refused_before_connected_after_enable_insert_and_ddl_refused() {
    let _serial = serial();
    let Some(local_ip) = a_local_non_loopback_ipv4() else {
        eprintln!("no non-loopback IPv4 interface found on this machine — skipping");
        return;
    };

    let mut fixture = Fixture::new("enable");
    let state = provision(&mut fixture).await;
    let server = ServerHandle::new(fixture.paths.clone());
    server.ensure_running(&fixture.payload, &fixture.creds).await.expect("initial start must succeed");

    // Before enabling: a connection to this machine's own non-loopback IPv4 must be refused (the
    // server is loopback-bound — this fails at the TCP/bind level regardless of credentials,
    // since `equal_lan` doesn't even exist yet at this point).
    let refused = try_connect_tcp(&local_ip, state.port, "equal_lan", "irrelevant-not-created-yet").await;
    assert!(refused.is_err(), "a connection to the non-loopback address must be refused before LAN sharing is enabled");

    // Enable LAN sharing (firewall skipped for this test).
    let info = lan::enable_lan_sharing(&server, &fixture.creds, &fixture.payload, FirewallMode::Skip)
        .await
        .expect("enable_lan_sharing must succeed");
    assert_eq!(info.port, state.port, "the port must never change while enabling LAN sharing");

    let after_state = state_file::load(&fixture.paths.server_json()).unwrap().unwrap();
    assert!(after_state.lan_sharing, "server.json.lanSharing must be true after enabling");
    assert!(matches!(after_state.state, ServerLifecycleState::Ready));

    // After enabling: equal_lan can connect through the non-loopback address, using the code from
    // PairingInfo (dashed groups — the same shape a cashier would type on a terminal).
    let secret = accounting_app_lib::infrastructure::database::pairing::parse_code(&info.code).expect("PairingInfo's own code must parse");
    let db = try_connect_tcp(&local_ip, state.port, "equal_lan", &secret)
        .await
        .expect("equal_lan must be able to connect through the non-loopback address after enabling");

    use sea_orm::{ConnectionTrait, Statement};
    // INSERT must be allowed (DML grant) against a table the test creates as equal_app-equivalent
    // via a root connection first (equal_lan itself has no CREATE).
    let root_secret = fixture.creds.get_root_secret(after_state.data_dir_id).unwrap();
    let root_db = accounting_app_lib::infrastructure::database::admin::connect_root(state.port, "root", &root_secret, Some("equal"))
        .await
        .unwrap();
    root_db
        .execute(Statement::from_string(root_db.get_database_backend(), "CREATE TABLE IF NOT EXISTS lan_probe (id INT)".to_string()))
        .await
        .unwrap();

    let insert = db
        .execute(Statement::from_string(db.get_database_backend(), "INSERT INTO lan_probe (id) VALUES (1)".to_string()))
        .await;
    assert!(insert.is_ok(), "equal_lan must be able to INSERT (DML grant)");

    // DDL must be refused (errno 1142 — command denied).
    let ddl = db
        .execute(Statement::from_string(db.get_database_backend(), "CREATE TABLE lan_probe_ddl (id INT)".to_string()))
        .await;
    let err = ddl.expect_err("equal_lan must not be able to CREATE TABLE");
    assert!(err.to_string().contains("1142"), "expected errno 1142, got: {err}");

    let _ = db.close().await;
    let _ = root_db.close().await;

    server.shutdown(std::time::Duration::from_secs(30)).await;
}

#[tokio::test]
async fn rotation_invalidates_the_old_code() {
    let _serial = serial();
    let Some(local_ip) = a_local_non_loopback_ipv4() else {
        eprintln!("no non-loopback IPv4 interface found on this machine — skipping");
        return;
    };

    let mut fixture = Fixture::new("rotate");
    let state = provision(&mut fixture).await;
    let server = ServerHandle::new(fixture.paths.clone());
    server.ensure_running(&fixture.payload, &fixture.creds).await.expect("initial start must succeed");

    let info = lan::enable_lan_sharing(&server, &fixture.creds, &fixture.payload, FirewallMode::Skip)
        .await
        .expect("enable_lan_sharing must succeed");
    let old_secret = accounting_app_lib::infrastructure::database::pairing::parse_code(&info.code).unwrap();

    // The old code works right after enabling.
    try_connect_tcp(&local_ip, state.port, "equal_lan", &old_secret)
        .await
        .expect("the freshly issued code must work immediately after enabling");

    let rotated = lan::rotate_pairing_code(&server, &fixture.creds).await.expect("rotate_pairing_code must succeed");
    let new_secret = accounting_app_lib::infrastructure::database::pairing::parse_code(&rotated.code).unwrap();
    assert_ne!(old_secret, new_secret, "rotation must issue a different secret");

    // The old code must now be refused.
    let old_still_works = try_connect_tcp(&local_ip, state.port, "equal_lan", &old_secret).await;
    assert!(old_still_works.is_err(), "the old pairing code must be invalidated by rotation");

    // The new code must work.
    try_connect_tcp(&local_ip, state.port, "equal_lan", &new_secret).await.expect("the newly rotated code must work");

    server.shutdown(std::time::Duration::from_secs(30)).await;
}

#[tokio::test]
async fn disabling_drops_the_user_and_refuses_lan_connections_again() {
    let _serial = serial();
    let Some(local_ip) = a_local_non_loopback_ipv4() else {
        eprintln!("no non-loopback IPv4 interface found on this machine — skipping");
        return;
    };

    let mut fixture = Fixture::new("disable");
    let state = provision(&mut fixture).await;
    let server = ServerHandle::new(fixture.paths.clone());
    server.ensure_running(&fixture.payload, &fixture.creds).await.expect("initial start must succeed");

    let info = lan::enable_lan_sharing(&server, &fixture.creds, &fixture.payload, FirewallMode::Skip)
        .await
        .expect("enable_lan_sharing must succeed");
    let secret = accounting_app_lib::infrastructure::database::pairing::parse_code(&info.code).unwrap();
    try_connect_tcp(&local_ip, state.port, "equal_lan", &secret).await.expect("must connect while LAN sharing is on");

    lan::disable_lan_sharing(&server, &fixture.creds, &fixture.payload).await.expect("disable_lan_sharing must succeed");

    let after_state = state_file::load(&fixture.paths.server_json()).unwrap().unwrap();
    assert!(!after_state.lan_sharing, "server.json.lanSharing must be false after disabling");

    let refused_again = try_connect_tcp(&local_ip, state.port, "equal_lan", &secret).await;
    assert!(refused_again.is_err(), "a LAN connection must be refused again after disabling (loopback-bound + user dropped)");

    server.shutdown(std::time::Duration::from_secs(30)).await;
}
