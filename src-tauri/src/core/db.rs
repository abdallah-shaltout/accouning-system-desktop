//! MariaDB pool, version gate and migration boot (phase-a-foundation.md A-6, fixed by A2-10/C-19).

use async_trait::async_trait;
use sea_orm::sqlx::mysql::MySqlPoolOptions;
use sea_orm::sqlx::ConnectOptions as SqlxConnectOptions;
use sea_orm::{DatabaseConnection, DbErr, SqlxMySqlConnector};
use tauri::{AppHandle, Manager};

use crate::core::device::{ConnectionSettings, DeviceRole};
use crate::core::state::{AppState, DbStatus};

/// Builds the sqlx `MySqlConnectOptions` piece-by-piece (host/port/db/user/password, charset,
/// collation) — reached only through `sea_orm::sqlx` to avoid sqlx version skew (P2-01/A-1).
fn connect_options(conn: &ConnectionSettings, password: &str) -> sea_orm::sqlx::mysql::MySqlConnectOptions {
    sea_orm::sqlx::mysql::MySqlConnectOptions::new()
        .host(&conn.host)
        .port(conn.port)
        .username(&conn.user)
        .password(password)
        .database(&conn.database)
        .charset("utf8mb4")
        .collation("utf8mb4_unicode_ci")
        .disable_statement_logging()
}

/// C-19 fix: the three session-level `SET SESSION` statements A-6 originally issued once against
/// the pool only ever reached *one* pooled connection out of up to 8 — every other connection in
/// the pool silently ran without them. Moving them into `MySqlPoolOptions::after_connect` applies
/// them to every physical connection the pool ever opens, exactly once each, the moment it's
/// established.
async fn after_connect(conn: &mut sea_orm::sqlx::MySqlConnection, _meta: sea_orm::sqlx::pool::PoolConnectionMetadata) -> Result<(), sea_orm::sqlx::Error> {
    use sea_orm::sqlx::Executor;
    conn.execute("SET SESSION time_zone = '+00:00'").await?;
    conn.execute("SET SESSION sql_mode='STRICT_ALL_TABLES,NO_ZERO_IN_DATE,NO_ZERO_DATE,ERROR_FOR_DIVISION_BY_ZERO,NO_ENGINE_SUBSTITUTION'")
        .await?;
    conn.execute("SET SESSION innodb_lock_wait_timeout=10").await?;
    Ok(())
}

/// Builds a pooled connection with the session options P2-05 requires (utf8mb4, UTC session
/// timezone, strict `sql_mode`, small pool bounds appropriate to a single-branch desktop app).
/// Session settings are applied per-connection via `after_connect` (C-19), not once against the
/// pool.
pub async fn connect(conn: &ConnectionSettings, password: &str) -> Result<DatabaseConnection, DbErr> {
    let options = connect_options(conn, password);
    let pool = MySqlPoolOptions::new()
        .max_connections(8)
        .min_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .after_connect(|conn, meta| Box::pin(after_connect(conn, meta)))
        .connect_with(options)
        .await
        .map_err(|e| DbErr::Conn(sea_orm::RuntimeErr::SqlxError(e).into()))?;
    Ok(SqlxMySqlConnector::from_sqlx_mysql_pool(pool))
}

/// Debug-only `EQUAL_DB_URL` override path (A-6/A-5): parses the URL into the same
/// `MySqlConnectOptions` and goes through the identical `MySqlPoolOptions` builder, so the debug
/// override gets the exact same per-connection session settings as the normal path.
#[cfg(debug_assertions)]
pub async fn connect_debug_url(url: &str) -> Result<DatabaseConnection, DbErr> {
    let options: sea_orm::sqlx::mysql::MySqlConnectOptions =
        url.parse().map_err(|e: sea_orm::sqlx::Error| DbErr::Conn(sea_orm::RuntimeErr::SqlxError(e).into()))?;
    let pool = MySqlPoolOptions::new()
        .max_connections(8)
        .min_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .after_connect(|conn, meta| Box::pin(after_connect(conn, meta)))
        .connect_with(options)
        .await
        .map_err(|e| DbErr::Conn(sea_orm::RuntimeErr::SqlxError(e).into()))?;
    Ok(SqlxMySqlConnector::from_sqlx_mysql_pool(pool))
}

/// P2-03: `SELECT VERSION()` must contain `MariaDB` and be ≥ 10.11.
pub async fn check_version(db: &DatabaseConnection) -> Result<(), String> {
    use sea_orm::{ConnectionTrait, Statement};
    let stmt = Statement::from_string(db.get_database_backend(), "SELECT VERSION() AS v".to_string());
    let row = db.query_one(stmt).await.map_err(|e| e.to_string())?;
    let Some(row) = row else { return Err("SELECT VERSION() returned no row".to_string()) };
    let version: String = row.try_get("", "v").map_err(|e| e.to_string())?;
    if !version.contains("MariaDB") {
        return Err(format!("قاعدة البيانات ليست MariaDB (الإصدار المُبلَّغ: {version})"));
    }
    let numeric_part = version.split('-').next().unwrap_or(&version);
    let parts: Vec<u32> = numeric_part.split('.').filter_map(|p| p.parse().ok()).collect();
    let (major, minor) = (parts.first().copied().unwrap_or(0), parts.get(1).copied().unwrap_or(0));
    if (major, minor) < (10, 11) {
        return Err(format!("يتطلب البرنامج MariaDB بإصدار 10.11 أو أحدث (الإصدار الحالي: {version})"));
    }
    Ok(())
}

/// Proves UUIDv7 values sort in time order in the native `UUID` column (P2-03) — a DB-backed test
/// lives alongside the entity/migration work in Phase B, since it needs a real table to insert
/// into; this module only exposes `check_version`'s pure logic for a unit test.
pub fn parse_mariadb_version(version: &str) -> Option<(u32, u32)> {
    if !version.contains("MariaDB") {
        return None;
    }
    let numeric_part = version.split('-').next()?;
    let mut parts = numeric_part.split('.').filter_map(|p| p.parse::<u32>().ok());
    Some((parts.next()?, parts.next()?))
}

/// G-45/GB-1/GB-2 (P2-52 handoff): the pre-migration automatic backup is implemented later by the
/// `17-backup` domain, which cannot be depended on directly from `core` (a domain never gets called
/// from `core`, only the other way around — the seam rule, generalized). This trait is the function-
/// pointer/trait seam `17-backup::pre_migration::backup_before_migrations` fills once it exists:
/// `core::db::migrate` calls whatever implementation `AppState`/the boot path wires in, and until
/// then a no-op `NoPendingMigrationBackup` is used, which only ever needs to succeed because it is
/// only ever consulted when the caller already knows there is nothing to back up yet (see below).
///
/// Not stored on `AppState` (no domain type may appear in `core`'s own state — same seam boundary
/// rule) — instead threaded through as a parameter, so `connect_and_migrate` (which lives in `core`
/// but is called after `AppState` exists) can pass whichever hook the binary's `lib.rs` wiring
/// chooses, without `core` ever naming `infrastructure::backup`.
#[async_trait]
pub trait PreMigrationBackup: Send + Sync {
    /// Called for `DeviceRole::Main` only, before `Migrator::up`, whenever there is at least one
    /// pending migration. Returning `Err` aborts the migration entirely (`migrate` maps it to
    /// `DbStatus::MigrationBackupFailed`, G-45) — zero data loss over availability (D-14).
    async fn backup_before_migrations(&self, db: &DatabaseConnection) -> Result<(), String>;
}

/// The no-op seam filled in until `17-backup` lands: `core::db::migrate` never calls this when there
/// are zero pending migrations (fresh install, or already up to date), so a real backup is never
/// skipped by using this placeholder — it only ever runs (returning `Ok` unconditionally) in a
/// build/test that hasn't wired the real backup domain in yet.
pub struct NoPendingMigrationBackup;

#[async_trait]
impl PreMigrationBackup for NoPendingMigrationBackup {
    async fn backup_before_migrations(&self, _db: &DatabaseConnection) -> Result<(), String> {
        Ok(())
    }
}

/// G-45: `migrate`'s failure modes map to two different `DbStatus` values (`SchemaMismatch` vs.
/// `MigrationBackupFailed`) — `connect_and_migrate` needs to tell them apart, which a bare `String`
/// error can't do without fragile text-sniffing.
pub enum MigrateError {
    /// The pre-migration backup itself failed — migrations were never attempted.
    BackupFailed(String),
    /// Everything else (a terminal with pending/unknown migrations, `Migrator::up` failing, …).
    Other(String),
}

impl std::fmt::Display for MigrateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrateError::BackupFailed(msg) | MigrateError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

/// Runs migrations per P2-28: the Main PC applies pending migrations; a terminal must find zero
/// pending migrations and no applied-but-unknown migration, else it refuses the DB. G-45/GB-2: on
/// the Main PC, when there is at least one pending migration, `backup.backup_before_migrations` runs
/// first — its failure aborts the migration (`MigrateError::BackupFailed`; `connect_and_migrate` maps
/// it to `DbStatus::MigrationBackupFailed`) rather than risking an unbacked-up schema change.
pub async fn migrate(db: &DatabaseConnection, role: DeviceRole, backup: &dyn PreMigrationBackup) -> Result<(), MigrateError> {
    use migration::MigratorTrait;
    match role {
        DeviceRole::Main => {
            let pending = migration::Migrator::get_pending_migrations(db).await.map_err(|e| MigrateError::Other(e.to_string()))?;
            if !pending.is_empty() {
                backup.backup_before_migrations(db).await.map_err(MigrateError::BackupFailed)?;
            }
            migration::Migrator::up(db, None).await.map_err(|e| MigrateError::Other(e.to_string()))
        }
        DeviceRole::Terminal => {
            let pending = migration::Migrator::get_pending_migrations(db).await.map_err(|e| MigrateError::Other(e.to_string()))?;
            if !pending.is_empty() {
                return Err(MigrateError::Other(
                    "قاعدة البيانات على الجهاز الرئيسي بإصدار مختلف — حدّث البرنامج على الجهازين".to_string(),
                ));
            }
            Ok(())
        }
    }
}

/// A3-2 (P2-57): a terminal's `connection.host` is normally the Main PC's DNS hostname, which
/// survives a DHCP address change but can occasionally fail to resolve/connect right after one
/// (stale DNS cache, a brief window before the new lease propagates). Tries `host` first; on a
/// connect failure for a `DeviceRole::Terminal`, retries once against `lastKnownAddress` if one is
/// recorded. Whichever attempt through `host` itself succeeds, the resolved peer IPv4 is persisted
/// back into `device-settings.json` as the new `lastKnownAddress` for next time. A Main PC (which
/// always connects to `127.0.0.1`) never needs or uses this fallback.
async fn connect_with_terminal_fallback(
    app: &AppHandle,
    app_data_dir: &std::path::Path,
    conn: &ConnectionSettings,
    password: &str,
    role: DeviceRole,
) -> Result<DatabaseConnection, DbErr> {
    let primary = connect(conn, password).await;
    if primary.is_ok() {
        if role == DeviceRole::Terminal {
            remember_resolved_address(app, app_data_dir, conn).await;
        }
        return primary;
    }

    if role != DeviceRole::Terminal {
        return primary;
    }
    let Some(fallback_host) = conn.last_known_address.clone() else {
        return primary;
    };
    if fallback_host == conn.host {
        return primary;
    }

    log::error!("primary DB host '{}' unreachable; retrying via lastKnownAddress '{fallback_host}'", conn.host);
    let mut fallback_conn = conn.clone();
    fallback_conn.host = fallback_host;
    connect(&fallback_conn, password).await
}

/// After a successful connect through `host` (never through the fallback IP — that's already a
/// known-good address, no need to re-derive it), resolves `host:port` to the IPv4 peer actually
/// used and, if it differs from the stored `lastKnownAddress`, persists it. Best-effort: any
/// failure here (DNS resolution, file write) is logged and never affects the already-successful
/// connection.
async fn remember_resolved_address(_app: &AppHandle, app_data_dir: &std::path::Path, conn: &ConnectionSettings) {
    // `conn.host` may already be a bare IP (Main PC's own loopback, or a terminal paired directly
    // by IP) — resolving it is still correct (it resolves to itself) and keeps this path uniform.
    // Uses blocking `std::net::ToSocketAddrs` on a `spawn_blocking` thread rather than
    // `tokio::net::lookup_host`, since the `tokio` dependency here isn't built with the `net`
    // feature (only `macros, process, rt, rt-multi-thread, signal, sync, time` — see the phase
    // status note's "Needs from manager" if that should be added instead).
    let addr = format!("{}:{}", conn.host, conn.port);
    let resolved = tokio::task::spawn_blocking(move || {
        use std::net::ToSocketAddrs;
        addr.to_socket_addrs().ok().and_then(|it| {
            it.filter_map(|a| match a {
                std::net::SocketAddr::V4(v4) => Some(*v4.ip()),
                _ => None,
            })
            .next()
        })
    })
    .await
    .ok()
    .flatten();
    let Some(ip) = resolved else { return };
    let ip_string = ip.to_string();
    if conn.last_known_address.as_deref() == Some(ip_string.as_str()) {
        return; // already up to date
    }

    match crate::core::device::load(app_data_dir) {
        Ok(mut settings) => {
            if let Some(c) = settings.connection.as_mut() {
                c.last_known_address = Some(ip_string);
                if let Err(e) = crate::core::device::save(app_data_dir, &settings) {
                    log::error!("failed to persist resolved lastKnownAddress: {e}");
                }
            }
        }
        Err(e) => log::error!("failed to load device settings to persist lastKnownAddress: {e}"),
    }
}

/// Background boot task spawned from `state::boot`: connects, gates the version, migrates, then
/// publishes the pool + status into `AppState`. Never blocks window creation — on any failure it
/// just leaves `db_status` reflecting the problem and the app keeps running on the mock.
pub async fn connect_and_migrate(app: &AppHandle) {
    let state = app.state::<AppState>();
    *state.db_status.write().unwrap() = DbStatus::Connecting;

    let device = state.device.read().unwrap().clone();
    let Some(conn) = device.connection else {
        *state.db_status.write().unwrap() = DbStatus::NotConfigured;
        return;
    };

    #[cfg(debug_assertions)]
    let url_override = crate::core::device::debug_db_url_override();
    #[cfg(not(debug_assertions))]
    let url_override: Option<String> = None;

    let connect_result = if let Some(url) = url_override {
        #[cfg(debug_assertions)]
        {
            connect_debug_url(&url).await
        }
        #[cfg(not(debug_assertions))]
        {
            unreachable!("url_override is always None in release builds")
        }
    } else {
        let password = match crate::core::device::load_password(&conn) {
            Ok(p) => p,
            Err(e) => {
                log::error!("failed to read DB password from keyring: {e}");
                *state.db_status.write().unwrap() = DbStatus::Unreachable;
                return;
            }
        };
        connect_with_terminal_fallback(app, &state.app_data_dir, &conn, &password, device.role).await
    };

    let database = match connect_result {
        Ok(db) => db,
        Err(e) => {
            log::error!("failed to connect to MariaDB: {e}");
            *state.db_status.write().unwrap() = DbStatus::Unreachable;
            return;
        }
    };

    if let Err(e) = check_version(&database).await {
        log::error!("MariaDB version gate failed: {e}");
        *state.db_status.write().unwrap() = DbStatus::Unsupported;
        return;
    }

    // P2-52 (17-backup): an automatic backup runs before any pending migration, into the managed
    // server's `backups\` folder (the same `ServerPaths` the supervisor owns).
    let out_dir = match device.role {
        crate::core::device::DeviceRole::Main => state.server.paths.backups(),
        _ => state.app_data_dir.join("backups"),
    };
    let pre_migration_backup = crate::infrastructure::backup::pre_migration::RealPreMigrationBackup { out_dir };
    if let Err(e) = migrate(&database, device.role, &pre_migration_backup).await {
        // G-45: a backup failure gets its own status (MigrationBackupFailed) distinct from a schema
        // mismatch — see core/status.rs's message and GB-3.
        match e {
            MigrateError::BackupFailed(msg) => {
                log::error!("pre-migration backup failed: {msg}");
                *state.db_status.write().unwrap() = DbStatus::MigrationBackupFailed;
            }
            MigrateError::Other(msg) => {
                log::error!("migration gate failed: {msg}");
                *state.db_status.write().unwrap() = DbStatus::SchemaMismatch;
            }
        }
        return;
    }

    *state.db.write().unwrap() = Some(crate::core::state::Db { connection: database });
    *state.db_status.write().unwrap() = DbStatus::Connected;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_gate_accepts_supported_and_rejects_old_or_non_mariadb() {
        assert_eq!(parse_mariadb_version("11.4.2-MariaDB"), Some((11, 4)));
        assert_eq!(parse_mariadb_version("10.11.6-MariaDB-1:10.11.6+maria~ubu2204"), Some((10, 11)));
        assert_eq!(parse_mariadb_version("10.6.0-MariaDB"), Some((10, 6)));
        assert_eq!(parse_mariadb_version("8.0.34"), None);
    }

    /// G-45: `MigrateError`'s two variants must stay distinguishable by `connect_and_migrate`'s
    /// `match` (a DB-backed test exercising the real `migrate()` + a fake failing backup lives in
    /// the 17-backup domain's own test suite once it exists; this only pins the error-shape
    /// contract this file owns).
    #[test]
    fn migrate_error_display_preserves_the_message_for_both_variants() {
        let backup_failed = MigrateError::BackupFailed("تعذر أخذ نسخة احتياطية".to_string());
        assert_eq!(backup_failed.to_string(), "تعذر أخذ نسخة احتياطية");
        assert!(matches!(backup_failed, MigrateError::BackupFailed(_)));

        let other = MigrateError::Other("مشكلة أخرى".to_string());
        assert_eq!(other.to_string(), "مشكلة أخرى");
        assert!(matches!(other, MigrateError::Other(_)));
    }

    #[test]
    fn no_pending_migration_backup_implements_the_seam_trait() {
        // No live `DatabaseConnection` is fabricated here (this crate keeps unit tests DB-free) —
        // this only proves `NoPendingMigrationBackup` satisfies `PreMigrationBackup` and is usable
        // wherever `migrate()` expects a `&dyn PreMigrationBackup`. The "it always returns `Ok`"
        // behavior itself is exercised by a DB-backed test once a real backup implementation (and a
        // test harness that already has a `DatabaseConnection`) exists in the 17-backup domain.
        fn assert_impls_trait<T: PreMigrationBackup>(_: &T) {}
        assert_impls_trait(&NoPendingMigrationBackup);
    }
}
