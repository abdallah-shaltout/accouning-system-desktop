//! Row-locking helpers for use inside a `with_tx` closure (phase-a-foundation.md A-7).
//!
//! **Global lock order** (must be followed by every domain to avoid deadlocks): document row(s) →
//! party rows (sorted) → product rows (sorted, then their batches) → settings row (S) →
//! fiscal-year row (S/X) → `document_counters` rows → `change_versions` (only at commit, inside
//! `with_tx` itself — never taken by domain code directly).

use sea_orm::{ConnectionTrait, DbErr, Statement};

/// `SELECT 1 FROM <table> WHERE id = ? FOR UPDATE` — exclusive row lock by primary key. `table`
/// must be a trusted, hard-coded identifier (never user input) since it's interpolated directly.
pub async fn for_update_by_id(conn: &impl ConnectionTrait, table: &str, id: &str) -> Result<(), DbErr> {
    let sql = format!("SELECT id FROM `{table}` WHERE id = ? FOR UPDATE");
    let stmt = Statement::from_sql_and_values(conn.get_database_backend(), &sql, [id.into()]);
    conn.query_all(stmt).await?;
    Ok(())
}

/// `SELECT 1 FROM <table> WHERE id IN (...) ORDER BY id FOR UPDATE` — locks several rows in a
/// fixed, sorted order so two transactions taking the same set of locks can never deadlock against
/// each other by acquiring them in different orders.
pub async fn for_update_many_sorted(conn: &impl ConnectionTrait, table: &str, ids: &[String]) -> Result<(), DbErr> {
    if ids.is_empty() {
        return Ok(());
    }
    let mut sorted = ids.to_vec();
    sorted.sort();
    let placeholders = sorted.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("SELECT id FROM `{table}` WHERE id IN ({placeholders}) ORDER BY id FOR UPDATE");
    let values: Vec<sea_orm::Value> = sorted.iter().map(|s| s.as_str().into()).collect();
    let stmt = Statement::from_sql_and_values(conn.get_database_backend(), &sql, values);
    conn.query_all(stmt).await?;
    Ok(())
}

/// `SELECT ... LOCK IN SHARE MODE` — a shared read lock, the syntax MariaDB accepts (rather than
/// the standard-SQL `FOR SHARE`, which older MariaDB/InnoDB releases don't parse). Used for
/// `assert_open_period`'s settings/fiscal-year reads (P2-13): every poster takes a shared lock so
/// concurrent posters don't serialize against each other, while `close_year` takes an exclusive
/// lock via `for_update_by_id`.
pub async fn share_lock_by_id(conn: &impl ConnectionTrait, table: &str, id: &str) -> Result<(), DbErr> {
    let sql = format!("SELECT id FROM `{table}` WHERE id = ? LOCK IN SHARE MODE");
    let stmt = Statement::from_sql_and_values(conn.get_database_backend(), &sql, [id.into()]);
    conn.query_all(stmt).await?;
    Ok(())
}
