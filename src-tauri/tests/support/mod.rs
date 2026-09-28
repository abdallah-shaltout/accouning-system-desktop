//! Shared DB-backed test harness (phase-a-foundation.md A-8). Needs `EQUAL_TEST_DATABASE_URL`
//! pointing at a real MariaDB ≥ 10.11 server reachable with admin privileges (to create/drop
//! throwaway databases). **Fails loudly, never skips**, per the entry file §7's own instruction —
//! a DB-backed test that silently no-ops when the variable is missing would let a real regression
//! pass "green" for the wrong reason.

use std::sync::Arc;

use accounting_app_lib::core::device::{DeviceRole, DeviceSettings};
use accounting_app_lib::core::events::CollectingEventSink;
use accounting_app_lib::core::state::{AppState, Db};
use accounting_app_lib::core::terminal::TerminalIdentity;
use accounting_app_lib::utils::id::Id;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};

/// A throwaway `equal_test_<uuid>` database, migrated, dropped on `Drop`. Also hands back an
/// `AppState` wired to a `CollectingEventSink` (so tests can assert on emitted events) with `db`
/// already populated and `db_status = Connected`.
pub struct TestDb {
    pub state: AppState,
    /// The connection URL to this exact throwaway database — used by tests that need a second,
    /// independent connection (e.g. the forced-deadlock test).
    #[allow(dead_code)]
    pub db_url: String,
    admin_url: String,
    db_name: String,
}

fn base_url() -> String {
    match std::env::var("EQUAL_TEST_DATABASE_URL") {
        Ok(url) if !url.trim().is_empty() => url,
        _ => panic!(
            "EQUAL_TEST_DATABASE_URL is not set. DB-backed tests need a real MariaDB >= 10.11 \
             instance, e.g. EQUAL_TEST_DATABASE_URL=mysql://root:password@127.0.0.1:3306 — \
             per plan 21 Part 02 §7, these tests must fail loudly, not skip, when it is absent."
        ),
    }
}

impl TestDb {
    pub async fn fresh() -> Self {
        let admin_url = base_url();
        let admin_conn = Database::connect(admin_url.clone())
            .await
            .unwrap_or_else(|e| panic!("could not connect to EQUAL_TEST_DATABASE_URL ({admin_url}): {e}"));

        let db_name = format!("equal_test_{}", Id::new().to_string().replace('-', "_"));
        admin_conn
            .execute(Statement::from_string(
                admin_conn.get_database_backend(),
                format!("CREATE DATABASE `{db_name}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci"),
            ))
            .await
            .expect("failed to create throwaway test database");

        let db_url = format!("{}/{}", admin_url.trim_end_matches('/'), db_name);
        // The admin URL may already carry a path/db-name segment (e.g.
        // `mysql://root:pw@host:3306`); rebuild cleanly by connecting to the new db by name.
        let db_url = rebuild_url_with_db(&admin_url, &db_name).unwrap_or(db_url);
        let connection: DatabaseConnection =
            Database::connect(db_url.clone()).await.unwrap_or_else(|e| panic!("could not connect to fresh test db {db_name}: {e}"));

        use migration::MigratorTrait;
        migration::Migrator::up(&connection, None)
            .await
            .unwrap_or_else(|e| panic!("failed to migrate test database {db_name}: {e}"));

        let terminal = TerminalIdentity { terminal_id: Id::new(), created_at: chrono::Utc::now() };
        let device = DeviceSettings { role: DeviceRole::Main, ..Default::default() };
        let events = Arc::new(CollectingEventSink::default());
        let state = AppState::new(std::env::temp_dir(), terminal, device, events);
        *state.db.write().unwrap() = Some(Db { connection });
        *state.db_status.write().unwrap() = accounting_app_lib::core::state::DbStatus::Connected;

        TestDb { state, db_url, admin_url, db_name }
    }
}

/// Best-effort rebuild of a `mysql://user:pass@host:port[/olddb]` URL with a different trailing
/// database name — avoids a URL-parsing crate dependency for one test-only helper.
fn rebuild_url_with_db(url: &str, db_name: &str) -> Option<String> {
    let scheme_split = url.find("://")?;
    let (scheme, rest) = url.split_at(scheme_split + 3);
    let authority = rest.split('/').next()?;
    Some(format!("{scheme}{authority}/{db_name}"))
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let admin_url = self.admin_url.clone();
        let db_name = self.db_name.clone();
        // `Drop` can't be async; spawn a detached blocking cleanup on a fresh current-thread
        // runtime rather than requiring every test to be `#[tokio::test(flavor = "multi_thread")]`.
        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("cleanup runtime");
            rt.block_on(async move {
                if let Ok(conn) = Database::connect(admin_url).await {
                    let _ = conn
                        .execute(Statement::from_string(conn.get_database_backend(), format!("DROP DATABASE IF EXISTS `{db_name}`")))
                        .await;
                }
            });
        });
        let _ = handle.join();
    }
}
