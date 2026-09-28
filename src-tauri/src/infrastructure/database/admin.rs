//! A short-lived, single-connection root session against `127.0.0.1:<port>` — used for readiness
//! probing, provisioning grants, and `SHUTDOWN` (phase-a2 A2-7). Deliberately not pooled: this is
//! an occasional administrative connection, not the app's own data pool (`core::db`).

use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};

/// Opens one connection to `mysql://<user>:<password>@127.0.0.1:<port>/<database>`. `database`
/// is `None` for the pre-`CREATE DATABASE equal` steps of provisioning (connects to no default
/// schema).
pub async fn connect_root(port: u16, user: &str, password: &str, database: Option<&str>) -> Result<DatabaseConnection, DbErr> {
    let db_part = database.unwrap_or("");
    let url = format!("mysql://{user}:{password}@127.0.0.1:{port}/{db_part}", password = escape(password));
    let mut opts = ConnectOptions::new(url);
    opts.max_connections(1).min_connections(0).connect_timeout(std::time::Duration::from_secs(5)).sqlx_logging(false);
    Database::connect(opts).await
}

fn escape(password: &str) -> String {
    let mut out = String::with_capacity(password.len());
    for b in password.bytes() {
        match b {
            b':' | b'@' | b'/' | b'%' | b'?' | b'#' => out.push_str(&format!("%{b:02X}")),
            _ => out.push(b as char),
        }
    }
    out
}

/// `SELECT 1` — the readiness probe the supervisor polls every 250 ms while starting.
pub async fn probe_ready(port: u16, user: &str, password: &str) -> bool {
    use sea_orm::{ConnectionTrait, Statement};
    let Ok(db) = connect_root(port, user, password, None).await else {
        return false;
    };
    let stmt = Statement::from_string(db.get_database_backend(), "SELECT 1".to_string());
    let ok = db.query_one(stmt).await.is_ok();
    let _ = db.close().await;
    ok
}

/// Issues `SHUTDOWN` over an existing admin connection (MariaDB's native clean-stop statement —
/// equivalent to `mariadb-admin shutdown` but without spawning another process).
pub async fn shutdown(db: &DatabaseConnection) -> Result<(), DbErr> {
    use sea_orm::{ConnectionTrait, Statement};
    let stmt = Statement::from_string(db.get_database_backend(), "SHUTDOWN".to_string());
    // The server closes the connection as part of shutting down, so an error here is expected and
    // not itself a failure — only a successful send matters, not necessarily a clean response.
    let _ = db.execute(stmt).await;
    Ok(())
}
