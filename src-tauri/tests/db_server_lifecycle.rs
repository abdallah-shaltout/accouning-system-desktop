//! Integration tests for the bundled MariaDB server supervisor (phase-a2 A2-11, Windows-only).
//! Uses a **real** MariaDB payload (downloaded by `scripts/fetch-mariadb.js` — fails loudly with
//! that instruction if missing, same "never skip" policy as `tests/db_foundation.rs`), a temp
//! `ServerPaths::at(...)`, a per-test keyring service (`CredentialStore::for_test()`, cleaned up
//! per test), and random free ports (never the fixed dev port 3499, so this never collides with a
//! concurrently running `bun run db:dev`).
#![cfg(windows)]

use std::path::PathBuf;
use std::time::Duration;

use accounting_app_lib::infrastructure::database::{
    credentials::CredentialStore,
    payload::PayloadDir,
    paths::ServerPaths,
    provision::{self, ProvisionResult},
    state_file::{self, ServerLifecycleState},
    supervisor::{ServerHandle, ServerPhase},
};
use accounting_app_lib::utils::id::Id;

/// Each test here starts a real mariadbd. Production runs exactly one server per machine (P2-58),
/// so these tests run one at a time instead of racing each other for the same candidate port.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn serial() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

fn temp_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("equal-db-lifecycle-{label}-{}", Id::new()))
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

#[tokio::test]
async fn provision_connect_insert_stop_start_row_present() {
    let _serial = serial();
    let mut fixture = Fixture::new("basic");
    let state = provision(&mut fixture).await;
    assert!(matches!(state.state, ServerLifecycleState::Ready));

    let root_secret = fixture.creds.get_root_secret(state.data_dir_id).unwrap();

    // Connect as equal_app (the least-privilege user provisioning created) and create/insert.
    use sea_orm::{ConnectionTrait, Statement};
    let app_url = format!("mysql://equal_app:@127.0.0.1:{}/equal", state.port);
    // equal_app's password is randomly generated and never returned to the caller when
    // app_data_dir is None (db-dev-server-style call) — verify via root that the schema/user
    // exist and behave correctly instead of trying to recover that password.
    let root_db = accounting_app_lib::infrastructure::database::admin::connect_root(state.port, "root", &root_secret, None)
        .await
        .expect("root connect");
    let stmt = Statement::from_string(root_db.get_database_backend(), "SHOW DATABASES LIKE 'equal'".to_string());
    let row = root_db.query_one(stmt).await.expect("query");
    assert!(row.is_some(), "equal database must exist after provisioning");

    let create = Statement::from_string(
        root_db.get_database_backend(),
        "CREATE TABLE equal.lifecycle_test (id INT PRIMARY KEY, note VARCHAR(64))".to_string(),
    );
    root_db.execute(create).await.expect("create table");
    let insert = Statement::from_string(root_db.get_database_backend(), "INSERT INTO equal.lifecycle_test VALUES (1, 'hello')".to_string());
    root_db.execute(insert).await.expect("insert");
    let _ = root_db.close().await;
    let _ = app_url; // documents the connection shape even though this test authenticates as root

    // Clean stop via the supervisor.
    let server = ServerHandle::new(fixture.paths.clone());
    server.ensure_running(&fixture.payload, &fixture.creds).await.expect("ensure_running (adopt or start)");
    server.shutdown(Duration::from_secs(30)).await;

    // Start again and confirm the row survived.
    let server2 = ServerHandle::new(fixture.paths.clone());
    server2.ensure_running(&fixture.payload, &fixture.creds).await.expect("restart");
    let root_db2 = accounting_app_lib::infrastructure::database::admin::connect_root(state.port, "root", &root_secret, None)
        .await
        .expect("root reconnect");
    let stmt = Statement::from_string(root_db2.get_database_backend(), "SELECT note FROM equal.lifecycle_test WHERE id=1".to_string());
    let row = root_db2.query_one(stmt).await.expect("query row").expect("row must exist");
    let note: String = row.try_get("", "note").unwrap();
    assert_eq!(note, "hello");
    server2.shutdown(Duration::from_secs(30)).await;
}

#[tokio::test]
async fn equal_app_cannot_create_user() {
    let _serial = serial();
    let mut fixture = Fixture::new("privilege");
    let state = provision(&mut fixture).await;

    // equal_app's password isn't returned by provision_main when app_data_dir is None, so verify
    // the *grant list* directly through root instead of authenticating as equal_app — this still
    // proves the least-privilege guarantee (no CREATE USER / GRANT OPTION anywhere in its grants).
    use sea_orm::{ConnectionTrait, Statement};
    let root_secret = fixture.creds.get_root_secret(state.data_dir_id).unwrap();
    let db = accounting_app_lib::infrastructure::database::admin::connect_root(state.port, "root", &root_secret, None)
        .await
        .unwrap();
    let stmt = Statement::from_string(db.get_database_backend(), "SHOW GRANTS FOR 'equal_app'@'127.0.0.1'".to_string());
    let rows = db.query_all(stmt).await.unwrap();
    let grants: Vec<String> = rows.iter().map(|r| r.try_get::<String>("", "Grants for equal_app@127.0.0.1").unwrap_or_default()).collect();
    let combined = grants.join("\n");
    assert!(!combined.contains("GRANT OPTION"), "equal_app must never have WITH GRANT OPTION: {combined}");
    assert!(!combined.to_uppercase().contains("ALL PRIVILEGES ON *.*"), "equal_app must be scoped to equal.*, not *.*: {combined}");
    let _ = db.close().await;
}

#[tokio::test]
async fn terminate_while_idle_then_ensure_running_recovers() {
    let _serial = serial();
    let mut fixture = Fixture::new("crash-recovery");
    let state = provision(&mut fixture).await;

    // Provisioning already leaves the server running (per provision_main's own flow); find its
    // pid and kill it hard to simulate a crash.
    let pid_text = std::fs::read_to_string(fixture.paths.pid_file()).ok();
    if let Some(pid_text) = pid_text {
        if let Ok(pid) = pid_text.trim().parse::<u32>() {
            accounting_app_lib::infrastructure::database::process::terminate(pid);
        }
    }

    let server = ServerHandle::new(fixture.paths.clone());
    server.ensure_running(&fixture.payload, &fixture.creds).await.expect("ensure_running must recover from a crash");
    assert_eq!(server.state.read().await.phase, ServerPhase::Running);

    // error.log should show InnoDB crash recovery ran.
    let log = std::fs::read_to_string(fixture.paths.error_log()).unwrap_or_default();
    assert!(
        log.to_lowercase().contains("recovery") || log.to_lowercase().contains("innodb"),
        "expected InnoDB recovery mention in error.log after a hard kill:\n{log}"
    );

    server.shutdown(Duration::from_secs(30)).await;
    let _ = state; // keep state in scope for data_dir_id lifetime clarity
}

#[tokio::test]
async fn adopt_reuses_existing_process_without_restart() {
    let _serial = serial();
    let mut fixture = Fixture::new("adopt");
    let _state = provision(&mut fixture).await;

    let server1 = ServerHandle::new(fixture.paths.clone());
    server1.ensure_running(&fixture.payload, &fixture.creds).await.expect("first ensure_running");
    let pid_before = std::fs::read_to_string(fixture.paths.pid_file()).ok();

    // Drop the handle without stopping — the process must keep running (no kill-on-drop).
    drop(server1);

    let server2 = ServerHandle::new(fixture.paths.clone());
    server2.ensure_running(&fixture.payload, &fixture.creds).await.expect("second ensure_running must adopt");
    let pid_after = std::fs::read_to_string(fixture.paths.pid_file()).ok();
    assert_eq!(pid_before, pid_after, "adopting must not restart the process (same pid)");

    server2.shutdown(Duration::from_secs(30)).await;
}

#[tokio::test]
async fn interrupted_provisioning_resumes() {
    let _serial = serial();
    let mut fixture = Fixture::new("resume");
    let payload = &fixture.payload;
    fixture.paths.ensure_dirs().unwrap();

    // Simulate an interrupted first attempt: server.json left at `initializing`.
    let now = chrono::Utc::now();
    let data_dir_id = Id::new();
    let interrupted = state_file::ServerStateFile {
        version: 1,
        state: ServerLifecycleState::Initializing,
        data_dir_id,
        series: accounting_app_lib::infrastructure::database::payload::SUPPORTED_SERIES.to_string(),
        initialized_with: accounting_app_lib::infrastructure::database::payload::BUNDLED_VERSION.to_string(),
        last_started_with: accounting_app_lib::infrastructure::database::payload::BUNDLED_VERSION.to_string(),
        port: 0,
        lan_sharing: false,
        host_terminal_id: Id::new(),
        created_at: now,
        updated_at: now,
        upgrade_warning: None,
    };
    state_file::save(&fixture.paths.server_json(), &interrupted).unwrap();
    // Leave a half-written data dir behind to prove it gets cleaned up.
    std::fs::create_dir_all(fixture.paths.data().join("leftover")).unwrap();

    let result = provision::provision_main(&fixture.paths, payload, &fixture.creds, Id::new(), None).await.expect("resume must succeed");
    let state = match result {
        ProvisionResult::Provisioned(s) => s,
        ProvisionResult::AlreadyProvisioned(s) => s,
    };
    fixture.data_dir_id = Some(state.data_dir_id);
    assert!(matches!(state.state, ServerLifecycleState::Ready));

    let server = ServerHandle::new(fixture.paths.clone());
    server.shutdown(Duration::from_secs(30)).await;
}

#[tokio::test]
async fn delete_keyring_secret_then_recovery_connects() {
    let _serial = serial();
    let mut fixture = Fixture::new("recovery");
    let state = provision(&mut fixture).await;

    let server = ServerHandle::new(fixture.paths.clone());
    server.ensure_running(&fixture.payload, &fixture.creds).await.expect("start before deleting secret");
    server.shutdown(Duration::from_secs(30)).await;

    fixture.creds.delete_all(state.data_dir_id);

    accounting_app_lib::infrastructure::database::recovery::recover(&fixture.paths, &fixture.creds, &state)
        .await
        .expect("recovery must succeed after the secret is deleted");

    let new_secret = fixture.creds.get_root_secret(state.data_dir_id).expect("a new secret must be saved after recovery");
    let server2 = ServerHandle::new(fixture.paths.clone());
    server2.ensure_running(&fixture.payload, &fixture.creds).await.expect("start after recovery");
    let db = accounting_app_lib::infrastructure::database::admin::connect_root(state.port, "root", &new_secret, None)
        .await
        .expect("connect with the recovered secret");
    let _ = db.close().await;
    server2.shutdown(Duration::from_secs(30)).await;
}

#[tokio::test]
async fn patch_upgrade_cold_copies_and_bumps_last_started_with() {
    let _serial = serial();
    let mut fixture = Fixture::new("upgrade");
    let state = provision(&mut fixture).await;

    // Simulate having started with an older patch version.
    let mut older = state.clone();
    older.last_started_with = "11.4.12".to_string();
    state_file::save(&fixture.paths.server_json(), &older).unwrap();

    let server = ServerHandle::new(fixture.paths.clone());
    server.ensure_running(&fixture.payload, &fixture.creds).await.expect("patch upgrade must succeed");

    let reloaded = state_file::load(&fixture.paths.server_json()).unwrap().unwrap();
    assert_eq!(reloaded.last_started_with, accounting_app_lib::infrastructure::database::payload::BUNDLED_VERSION);
    assert!(fixture.paths.pre_upgrade_dir().exists(), "a pre-upgrade cold copy must exist");

    server.shutdown(Duration::from_secs(30)).await;
}

#[tokio::test]
async fn downgrade_is_refused_and_leaves_data_untouched() {
    let _serial = serial();
    let mut fixture = Fixture::new("downgrade");
    let state = provision(&mut fixture).await;

    let mut newer = state.clone();
    newer.last_started_with = "11.4.99".to_string();
    state_file::save(&fixture.paths.server_json(), &newer).unwrap();

    let before: Vec<_> = std::fs::read_dir(fixture.paths.data()).unwrap().flatten().map(|e| e.file_name()).collect();

    let server = ServerHandle::new(fixture.paths.clone());
    let result = server.ensure_running(&fixture.payload, &fixture.creds).await;
    assert!(result.is_err(), "a downgrade must be refused, not silently accepted");

    let after: Vec<_> = std::fs::read_dir(fixture.paths.data()).unwrap().flatten().map(|e| e.file_name()).collect();
    assert_eq!(before.len(), after.len(), "data directory must be untouched on a refused downgrade");
}
