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
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Statement};

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

/// The migrated template every test database is cloned from — built once per test process
/// (see `template_db`). Running all migrations per test took minutes each on MariaDB (every
/// `ALTER TABLE … ADD … STORED` rebuilds the table), which made the suite take hours.
static TEMPLATE: tokio::sync::OnceCell<String> = tokio::sync::OnceCell::const_new();

impl TestDb {
    pub async fn fresh() -> Self {
        let admin_url = base_url();
        let template = TEMPLATE.get_or_init(|| build_template(admin_url.clone())).await.clone();

        let db_name = format!("equal_test_{}", Id::new().to_string().replace('-', "_"));
        clone_database(&admin_url, &template, &db_name).await;

        let db_url = format!("{}/{}", admin_url.trim_end_matches('/'), db_name);
        // The admin URL may already carry a path/db-name segment (e.g.
        // `mysql://root:pw@host:3306`); rebuild cleanly by connecting to the new db by name.
        let db_url = rebuild_url_with_db(&admin_url, &db_name).unwrap_or(db_url);
        let connection: DatabaseConnection =
            Database::connect(db_url.clone()).await.unwrap_or_else(|e| panic!("could not connect to fresh test db {db_name}: {e}"));

        let terminal = TerminalIdentity { terminal_id: Id::new(), created_at: chrono::Utc::now() };
        let device = DeviceSettings { role: DeviceRole::Main, ..Default::default() };
        let events = Arc::new(CollectingEventSink::default());
        let state = AppState::new(std::env::temp_dir(), terminal, device, events);
        *state.db.write().unwrap() = Some(Db { connection });
        *state.db_status.write().unwrap() = accounting_app_lib::core::state::DbStatus::Connected;

        TestDb { state, db_url, admin_url, db_name }
    }
}

/// One-connection pool, so session settings (`FOREIGN_KEY_CHECKS`, `GET_LOCK`) apply to every
/// statement that follows.
async fn single_conn(url: &str) -> DatabaseConnection {
    let mut opts = ConnectOptions::new(url.to_string());
    opts.max_connections(1).min_connections(1).sqlx_logging(false);
    Database::connect(opts).await.unwrap_or_else(|e| panic!("could not connect to {url}: {e}"))
}

async fn exec(conn: &DatabaseConnection, sql: &str) {
    conn.execute_unprepared(sql).await.unwrap_or_else(|e| panic!("test db setup failed on `{sql}`: {e}"));
}

async fn strings(conn: &DatabaseConnection, sql: &str) -> Vec<String> {
    conn.query_all(Statement::from_string(conn.get_database_backend(), sql.to_string()))
        .await
        .unwrap_or_else(|e| panic!("test db setup failed on `{sql}`: {e}"))
        .iter()
        .map(|row| row.try_get_by_index::<String>(0).expect("string column"))
        .collect()
}

/// Name of the migrated template: a hash of every migration source file, so editing a migration
/// (not only adding one) makes a fresh template instead of silently reusing a stale schema.
fn template_name() -> String {
    use std::hash::{Hash, Hasher};
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migration").join("src");
    let mut files: Vec<_> = std::fs::read_dir(&dir).expect("migration/src").filter_map(|e| e.ok()).map(|e| e.path()).collect();
    files.sort();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for f in files {
        f.file_name().hash(&mut hasher);
        std::fs::read(&f).unwrap_or_default().hash(&mut hasher);
    }
    format!("equal_tpl_{:016x}", hasher.finish())
}

/// Builds (or reuses) the migrated template database. A MariaDB named lock keeps two test
/// processes from building the same template at once; a `_tpl_ready` marker table is written
/// last, so a half-built template (crash mid-migration) is rebuilt rather than cloned.
async fn build_template(admin_url: String) -> String {
    let name = template_name();
    let admin = single_conn(&admin_url).await;
    exec(&admin, "DO GET_LOCK('equal_test_template', 600)").await;

    let ready = strings(
        &admin,
        &format!("SELECT CAST(table_name AS CHAR) FROM information_schema.tables WHERE table_schema = '{name}' AND table_name = '_tpl_ready'"),
    )
    .await;
    if ready.is_empty() {
        for stale in strings(&admin, "SELECT CAST(schema_name AS CHAR) FROM information_schema.schemata WHERE schema_name LIKE 'equal\\_tpl\\_%'").await {
            exec(&admin, &format!("DROP DATABASE IF EXISTS `{stale}`")).await;
        }
        exec(&admin, &format!("CREATE DATABASE `{name}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci")).await;
        let tpl_url = rebuild_url_with_db(&admin_url, &name).expect("template url");
        let tpl = single_conn(&tpl_url).await;
        use migration::MigratorTrait;
        migration::Migrator::up(&tpl, None).await.unwrap_or_else(|e| panic!("failed to migrate template database {name}: {e}"));
        exec(&tpl, "CREATE TABLE `_tpl_ready` (`x` INT)").await;
        let _ = tpl.close().await;
    }

    exec(&admin, "DO RELEASE_LOCK('equal_test_template')").await;
    let _ = admin.close().await;
    name
}

/// Creates `db_name` as an exact copy of the template: every table's own `SHOW CREATE TABLE`
/// (columns, generated columns, indexes, foreign keys) plus its rows (seed rows such as
/// `document_counters` and `seaql_migrations`), generated columns excluded from the copy.
async fn clone_database(admin_url: &str, template: &str, db_name: &str) {
    let admin = single_conn(admin_url).await;
    exec(&admin, &format!("CREATE DATABASE `{db_name}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci")).await;
    let _ = admin.close().await;

    let url = rebuild_url_with_db(admin_url, db_name).expect("test db url");
    let conn = single_conn(&url).await;
    exec(&conn, "SET FOREIGN_KEY_CHECKS = 0").await;
    let tables = strings(
        &conn,
        &format!(
            "SELECT CAST(table_name AS CHAR) FROM information_schema.tables \
             WHERE table_schema = '{template}' AND table_type = 'BASE TABLE' AND table_name <> '_tpl_ready'"
        ),
    )
    .await;
    for table in &tables {
        let rows = conn
            .query_all(Statement::from_string(conn.get_database_backend(), format!("SHOW CREATE TABLE `{template}`.`{table}`")))
            .await
            .unwrap_or_else(|e| panic!("SHOW CREATE TABLE {table} failed: {e}"));
        let ddl: String = rows[0].try_get_by_index(1).expect("Create Table column");
        exec(&conn, &ddl).await;
    }
    for table in &tables {
        let cols = strings(
            &conn,
            &format!(
                "SELECT CAST(CONCAT('`', column_name, '`') AS CHAR) FROM information_schema.columns \
                 WHERE table_schema = '{template}' AND table_name = '{table}' AND is_generated = 'NEVER' \
                 ORDER BY ordinal_position"
            ),
        )
        .await
        .join(", ");
        exec(&conn, &format!("INSERT INTO `{table}` ({cols}) SELECT {cols} FROM `{template}`.`{table}`")).await;
    }
    exec(&conn, "SET FOREIGN_KEY_CHECKS = 1").await;
    let _ = conn.close().await;
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
        // Release this test's own pool first: a test that panicked mid-transaction leaves a
        // connection holding metadata locks, and DROP DATABASE would wait on it forever.
        drop(self.state.db.write().ok().and_then(|mut db| db.take()));
        let admin_url = self.admin_url.clone();
        let db_name = self.db_name.clone();
        // `Drop` can't be async; spawn a detached blocking cleanup on a fresh current-thread
        // runtime rather than requiring every test to be `#[tokio::test(flavor = "multi_thread")]`.
        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("cleanup runtime");
            rt.block_on(async move {
                let conn = single_conn(&admin_url).await;
                // Never block the suite: if a lock is still held, give up after 10 s and leave
                // the throwaway database behind (a later cleanup drops `equal_test_%`).
                let _ = conn.execute_unprepared("SET SESSION lock_wait_timeout = 10").await;
                let _ = conn.execute_unprepared(&format!("DROP DATABASE IF EXISTS `{db_name}`")).await;
                let _ = conn.close().await;
            });
        });
        let _ = handle.join();
    }
}

// --- shared fixture helpers ------------------------------------------------------------------------
//
// The migrations seed no reference rows, so a fixture that inserts `settings` (FKs to `currencies`
// and `branches`) or any row carrying a currency code has to insert those parents first — the real
// app gets them from the setup wizard (`domains/setup`), which a low-level test doesn't run.

/// The last `n` hex digits of an id — the UUIDv7 *random* tail. The leading digits are the
/// millisecond timestamp, so two ids minted in the same millisecond share them; a code/number built
/// from the prefix collides on the `uq_*_code`/`uq_*_number` uniques.
#[allow(dead_code)]
pub fn unique_tail(id: Id, n: usize) -> String {
    let hex = id.to_string().replace('-', "");
    hex[hex.len() - n.min(hex.len())..].to_string()
}

/// A short code that is unique per call (`<prefix><8 random hex digits>`).
#[allow(dead_code)]
pub fn unique_code(prefix: &str) -> String {
    format!("{prefix}{}", unique_tail(Id::new(), 8))
}

/// Inserts the `currencies` row for `code` if it is missing (idempotent; decimals 2).
#[allow(dead_code)]
pub async fn seed_currency<C: ConnectionTrait>(conn: &C, code: &str) {
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT IGNORE INTO currencies (code, name_ar, symbol, decimals, active) VALUES (?, ?, ?, 2, TRUE)",
        [code.into(), code.into(), code.into()],
    );
    conn.execute(stmt).await.unwrap_or_else(|e| panic!("seeding currency {code} failed: {e}"));
}

/// Inserts a minimal branch (unique code) and returns its id.
#[allow(dead_code)]
pub async fn seed_branch<C: ConnectionTrait>(conn: &C) -> Id {
    let id = Id::new();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO branches (id, name, code) VALUES (?, 'الفرع الرئيسي', ?)",
        [id.to_string().into(), format!("BR{}", unique_tail(id, 8)).into()],
    );
    conn.execute(stmt).await.unwrap_or_else(|e| panic!("seeding a branch failed: {e}"));
    id
}

/// The parents every `settings` row needs: its base `currency` row and a default branch. Returns
/// the branch id to put in `settings.default_branch_id`.
#[allow(dead_code)]
pub async fn seed_settings_parents<C: ConnectionTrait>(conn: &C, currency: &str) -> Id {
    seed_currency(conn, currency).await;
    seed_branch(conn).await
}

/// A minimal valid singleton `settings` row (base `currency`, a fresh default branch, A4 printer),
/// with its parents. Returns the default branch id. For low-level tests (shared::stock, ledger,
/// invariants) whose code path calls `core::settings::load` but that don't run the setup wizard.
#[allow(dead_code)]
pub async fn seed_minimal_settings<C: ConnectionTrait>(conn: &C, currency: &str) -> Id {
    let branch_id = seed_settings_parents(conn, currency).await;
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO settings (id, store_name, currency, invoice_number_prefix, printer, default_branch_id) \
         VALUES (?, 'متجر تجريبي', ?, 'INV-', '{\"mode\":\"a4\",\"thermalWidthMm\":80}', ?)",
        [Id::new().to_string().into(), currency.into(), branch_id.to_string().into()],
    );
    conn.execute(stmt).await.unwrap_or_else(|e| panic!("seeding settings failed: {e}"));
    branch_id
}

/// Inserts the `users` row for a session user (FK target of `audit.user_id`, `activity.user_id`,
/// `journal_entries.created_by`, …). `role` is the DB enum value (`admin`, `accountant`, …).
#[allow(dead_code)]
pub async fn seed_user<C: ConnectionTrait>(conn: &C, id: Id, role: &str) {
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "INSERT INTO users (id, username, name, role) VALUES (?, ?, 'مستخدم تجريبي', ?)",
        [id.to_string().into(), format!("user_{}", unique_tail(id, 12)).into(), role.into()],
    );
    conn.execute(stmt).await.unwrap_or_else(|e| panic!("seeding user failed: {e}"));
}
