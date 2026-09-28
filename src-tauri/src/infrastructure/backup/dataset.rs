//! `dataset.rs` (17-backup.md §3.2) — the `data.json` of Rust-made archives: a schema-agnostic,
//! lossless dump of every business table via `information_schema`, loaded back through a
//! topologically-sorted `wipe`/`load` inside one transaction. No `mariadb-dump`, no ORM entities —
//! this reads/writes strictly through SQL discovered at runtime (P2-52), so it never has to be
//! updated when a new table/column is added.

use std::collections::{HashMap, HashSet};

use sea_orm::{ConnectionTrait, Statement};
use serde::{Deserialize, Serialize};

use crate::core::error::AppError;
use crate::core::tx::{TxError, TxResult};

/// Tables never dumped: `seaql_migrations` (migration bookkeeping, irrelevant to a restore target
/// already on this schema version) and `change_versions` (C-11: per-branch mechanics, bumped after a
/// restore instead of restored verbatim). `document_counters` **is** dumped (numbering continues
/// correctly after a restore).
const EXCLUDED_TABLES: &[&str] = &["seaql_migrations", "change_versions"];

const MAX_ROWS_PER_INSERT: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableDump {
    pub name: String,
    pub columns: Vec<String>,
    /// Each row is one value per column, in `columns` order; SQL `NULL` becomes `None`.
    pub rows: Vec<Vec<Option<String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DataSetV1 {
    pub format: String,
    pub db_schema_version: u32,
    pub migrations: Vec<String>,
    pub tables: Vec<TableDump>,
}

pub const DATASET_FORMAT: &str = "equal-db";

/// `db_schema_version(conn)` (§3.2) = `100 + COUNT(*) FROM seaql_migrations`. Values `< 100` are the
/// mock's `SCHEMA_VERSION` space (D-3), so the two formats never collide.
pub async fn db_schema_version<C: ConnectionTrait>(conn: &C) -> TxResult<u32> {
    let stmt = Statement::from_string(conn.get_database_backend(), "SELECT COUNT(*) AS n FROM `seaql_migrations`".to_string());
    let row = conn.query_one(stmt).await.map_err(TxError::from)?;
    let n: i64 = match row {
        Some(row) => row.try_get("", "n").map_err(TxError::from)?,
        None => 0,
    };
    Ok(100 + n as u32)
}

/// `build_schema_version()` (§3.2) = `100 + migration::Migrator::migrations().len()` — the build's
/// own idea of the current schema version, independent of what's actually applied in the DB (used to
/// decide whether a restored archive needs row upgraders, §3.3).
pub fn build_schema_version() -> u32 {
    use migration::MigratorTrait;
    100 + migration::Migrator::migrations().len() as u32
}

/// The list of applied migration names, in the dataset's own `migrations` field (the Rust analogue
/// of the mock's `migrations` record).
pub async fn applied_migration_names<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<String>> {
    let stmt = Statement::from_string(
        conn.get_database_backend(),
        "SELECT `version` FROM `seaql_migrations` ORDER BY `version` ASC".to_string(),
    );
    let rows = conn.query_all(stmt).await.map_err(TxError::from)?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let name: String = row.try_get("", "version").map_err(TxError::from)?;
        out.push(name);
    }
    Ok(out)
}

/// Every base table in the current database, `information_schema.TABLES` filtered to `BASE TABLE`,
/// excluding [`EXCLUDED_TABLES`].
async fn base_tables<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<String>> {
    let stmt = Statement::from_string(
        conn.get_database_backend(),
        "SELECT TABLE_NAME AS name FROM information_schema.TABLES \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_TYPE = 'BASE TABLE' ORDER BY TABLE_NAME ASC"
            .to_string(),
    );
    let rows = conn.query_all(stmt).await.map_err(TxError::from)?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let name: String = row.try_get("", "name").map_err(TxError::from)?;
        if !EXCLUDED_TABLES.contains(&name.as_str()) {
            out.push(name);
        }
    }
    Ok(out)
}

struct ColumnInfo {
    name: String,
    data_type: String,
    is_generated: bool,
}

/// `information_schema.COLUMNS` for `table`, ordered by `ORDINAL_POSITION`, excluding generated
/// columns (`EXTRA LIKE '%GENERATED%'` — `_key`, `_live`, `open_key`, `default_key`).
async fn table_columns<C: ConnectionTrait>(conn: &C, table: &str) -> TxResult<Vec<ColumnInfo>> {
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "SELECT COLUMN_NAME AS name, DATA_TYPE AS data_type, EXTRA AS extra FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? ORDER BY ORDINAL_POSITION ASC",
        [table.into()],
    );
    let rows = conn.query_all(stmt).await.map_err(TxError::from)?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let name: String = row.try_get("", "name").map_err(TxError::from)?;
        let data_type: String = row.try_get("", "data_type").map_err(TxError::from)?;
        let extra: Option<String> = row.try_get("", "extra").ok().flatten();
        let is_generated = extra.map(|e| e.to_uppercase().contains("GENERATED")).unwrap_or(false);
        out.push(ColumnInfo { name, data_type, is_generated });
    }
    Ok(out)
}

/// One FK edge: `(child_table, child_column) -> (parent_table)`, nullability of the child column.
struct FkEdge {
    child_table: String,
    child_column: String,
    parent_table: String,
    nullable: bool,
}

/// Every FK relationship in the schema, from `information_schema.KEY_COLUMN_USAGE` joined against
/// `COLUMNS` for nullability (a self-reference or a cycle edge is broken by deferring a **nullable**
/// FK column — §3.2's `topo_order`).
async fn fk_edges<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<FkEdge>> {
    let stmt = Statement::from_string(
        conn.get_database_backend(),
        "SELECT k.TABLE_NAME AS child_table, k.COLUMN_NAME AS child_column, k.REFERENCED_TABLE_NAME AS parent_table, \
                c.IS_NULLABLE AS nullable \
         FROM information_schema.KEY_COLUMN_USAGE k \
         JOIN information_schema.COLUMNS c \
           ON c.TABLE_SCHEMA = k.TABLE_SCHEMA AND c.TABLE_NAME = k.TABLE_NAME AND c.COLUMN_NAME = k.COLUMN_NAME \
         WHERE k.TABLE_SCHEMA = DATABASE() AND k.REFERENCED_TABLE_NAME IS NOT NULL"
            .to_string(),
    );
    let rows = conn.query_all(stmt).await.map_err(TxError::from)?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let child_table: String = row.try_get("", "child_table").map_err(TxError::from)?;
        let child_column: String = row.try_get("", "child_column").map_err(TxError::from)?;
        let parent_table: String = row.try_get("", "parent_table").map_err(TxError::from)?;
        let nullable: String = row.try_get("", "nullable").map_err(TxError::from)?;
        if EXCLUDED_TABLES.contains(&child_table.as_str()) || EXCLUDED_TABLES.contains(&parent_table.as_str()) {
            continue;
        }
        out.push(FkEdge { child_table, child_column, parent_table, nullable: nullable.eq_ignore_ascii_case("YES") });
    }
    Ok(out)
}

/// One deferred column: dumped/loaded as `NULL` first, then patched with a per-row `UPDATE` once
/// every table has its own rows in place.
#[derive(Debug, Clone)]
pub struct DeferredColumn {
    pub table: String,
    pub column: String,
}

/// A table's primary-key column names, in ordinal order (used both for `ORDER BY` in the dump and to
/// build the `WHERE <pk> = ?` clause for a deferred `UPDATE`).
async fn primary_key_columns<C: ConnectionTrait>(conn: &C, table: &str) -> TxResult<Vec<String>> {
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "SELECT COLUMN_NAME AS name FROM information_schema.KEY_COLUMN_USAGE \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? AND CONSTRAINT_NAME = 'PRIMARY' ORDER BY ORDINAL_POSITION ASC",
        [table.into()],
    );
    let rows = conn.query_all(stmt).await.map_err(TxError::from)?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(row.try_get::<String>("", "name").map_err(TxError::from)?);
    }
    Ok(out)
}

/// Kahn's topological sort over the FK graph (§3.2 `topo_order`): tables with no incoming edge come
/// first. A self-reference or a cycle is broken by deferring one **nullable** FK column of the edge
/// (dumped/loaded as `NULL`, then backfilled by a per-row `UPDATE` after every table is loaded); a
/// cycle with no nullable edge anywhere in it is an `INTERNAL` error (never reached in practice — the
/// unit test pins that none exists in this schema).
pub struct TopoOrder {
    pub tables: Vec<String>,
    pub deferred: Vec<DeferredColumn>,
}

pub async fn topo_order<C: ConnectionTrait>(conn: &C) -> TxResult<TopoOrder> {
    let tables = base_tables(conn).await?;
    let table_set: HashSet<&str> = tables.iter().map(|s| s.as_str()).collect();
    let mut edges = fk_edges(conn).await?;
    // Self-references (child == parent) are always deferred, never part of the graph's ordering.
    let mut deferred = Vec::new();
    edges.retain(|e| {
        if e.child_table == e.parent_table {
            deferred.push(DeferredColumn { table: e.child_table.clone(), column: e.child_column.clone() });
            false
        } else {
            table_set.contains(e.parent_table.as_str()) && table_set.contains(e.child_table.as_str())
        }
    });

    // Build adjacency (parent -> children) and in-degree (per child, number of distinct parents not
    // yet resolved) for Kahn's algorithm. Multiple FK columns from the same child to the same parent
    // count once for ordering purposes (the edge just needs the parent placed before the child).
    let mut children_of: HashMap<&str, HashSet<&str>> = HashMap::new();
    let mut parents_of: HashMap<&str, HashSet<&str>> = HashMap::new();
    for e in &edges {
        children_of.entry(e.parent_table.as_str()).or_default().insert(e.child_table.as_str());
        parents_of.entry(e.child_table.as_str()).or_default().insert(e.parent_table.as_str());
    }

    let mut remaining_parents: HashMap<&str, HashSet<&str>> = HashMap::new();
    for t in &tables {
        remaining_parents.insert(t.as_str(), parents_of.get(t.as_str()).cloned().unwrap_or_default());
    }

    let mut ordered: Vec<String> = Vec::with_capacity(tables.len());
    let mut ready: Vec<&str> = tables.iter().map(|s| s.as_str()).filter(|t| remaining_parents[t].is_empty()).collect();
    ready.sort(); // deterministic order among tables with no dependency.

    while let Some(t) = ready.pop() {
        ordered.push(t.to_string());
        if let Some(children) = children_of.get(t) {
            let mut newly_ready = Vec::new();
            for &child in children {
                if let Some(set) = remaining_parents.get_mut(child) {
                    set.remove(t);
                    if set.is_empty() {
                        newly_ready.push(child);
                    }
                }
            }
            newly_ready.sort();
            ready.extend(newly_ready);
            ready.sort();
        }
    }

    if ordered.len() != tables.len() {
        // A cycle remains among tables whose mutual FKs are all NOT NULL — break it by deferring the
        // first still-unresolved edge with a nullable column; if none is nullable, that's the
        // documented INTERNAL case (the test suite pins that this schema has none).
        let unresolved: HashSet<&str> = tables.iter().map(|s| s.as_str()).filter(|t| !ordered.contains(&t.to_string())).collect();
        let mut broke_one = false;
        for e in &edges {
            if unresolved.contains(e.child_table.as_str()) && e.nullable {
                deferred.push(DeferredColumn { table: e.child_table.clone(), column: e.child_column.clone() });
                if let Some(set) = remaining_parents.get_mut(e.child_table.as_str()) {
                    set.remove(e.parent_table.as_str());
                }
                broke_one = true;
            }
        }
        if !broke_one {
            return Err(TxError::App(AppError::internal(
                "تعذرت قراءة بنية قاعدة البيانات — دورة مراجع بلا عمود قابل للفراغ",
                Some(format!("unresolved cycle among: {unresolved:?}")),
            )));
        }
        // Re-run the sort now that at least one edge was deferred; recursion depth is bounded by the
        // (small, fixed) number of tables, so a straightforward loop-until-fixpoint is used instead
        // of a second Kahn pass copy — reuse this function's own logic via a bounded retry.
        return Box::pin(topo_order_with_extra_deferred(conn, deferred)).await;
    }

    Ok(TopoOrder { tables: ordered, deferred })
}

/// Re-runs [`topo_order`]'s Kahn pass with a caller-supplied set of already-deferred edges excluded
/// from the graph up front — used only by the cycle-breaking fallback above.
async fn topo_order_with_extra_deferred<C: ConnectionTrait>(conn: &C, mut pre_deferred: Vec<DeferredColumn>) -> TxResult<TopoOrder> {
    let tables = base_tables(conn).await?;
    let table_set: HashSet<&str> = tables.iter().map(|s| s.as_str()).collect();
    let deferred_pairs: HashSet<(String, String)> = pre_deferred.iter().map(|d| (d.table.clone(), d.column.clone())).collect();
    let mut edges = fk_edges(conn).await?;
    edges.retain(|e| {
        if e.child_table == e.parent_table {
            return false; // already captured in pre_deferred by the caller.
        }
        if deferred_pairs.contains(&(e.child_table.clone(), e.child_column.clone())) {
            return false;
        }
        table_set.contains(e.parent_table.as_str()) && table_set.contains(e.child_table.as_str())
    });

    let mut children_of: HashMap<&str, HashSet<&str>> = HashMap::new();
    let mut parents_of: HashMap<&str, HashSet<&str>> = HashMap::new();
    for e in &edges {
        children_of.entry(e.parent_table.as_str()).or_default().insert(e.child_table.as_str());
        parents_of.entry(e.child_table.as_str()).or_default().insert(e.parent_table.as_str());
    }
    let mut remaining_parents: HashMap<&str, HashSet<&str>> = HashMap::new();
    for t in &tables {
        remaining_parents.insert(t.as_str(), parents_of.get(t.as_str()).cloned().unwrap_or_default());
    }
    let mut ordered: Vec<String> = Vec::with_capacity(tables.len());
    let mut ready: Vec<&str> = tables.iter().map(|s| s.as_str()).filter(|t| remaining_parents[t].is_empty()).collect();
    ready.sort();
    while let Some(t) = ready.pop() {
        ordered.push(t.to_string());
        if let Some(children) = children_of.get(t) {
            let mut newly_ready = Vec::new();
            for &child in children {
                if let Some(set) = remaining_parents.get_mut(child) {
                    set.remove(t);
                    if set.is_empty() {
                        newly_ready.push(child);
                    }
                }
            }
            newly_ready.sort();
            ready.extend(newly_ready);
            ready.sort();
        }
    }

    if ordered.len() != tables.len() {
        let unresolved: HashSet<&str> = tables.iter().map(|s| s.as_str()).filter(|t| !ordered.contains(&t.to_string())).collect();
        let mut broke_one = false;
        for e in &edges {
            if unresolved.contains(e.child_table.as_str()) && e.nullable {
                pre_deferred.push(DeferredColumn { table: e.child_table.clone(), column: e.child_column.clone() });
                broke_one = true;
            }
        }
        if !broke_one {
            return Err(TxError::App(AppError::internal(
                "تعذرت قراءة بنية قاعدة البيانات — دورة مراجع بلا عمود قابل للفراغ",
                Some(format!("unresolved cycle among: {unresolved:?}")),
            )));
        }
        return Box::pin(topo_order_with_extra_deferred(conn, pre_deferred)).await;
    }

    Ok(TopoOrder { tables: ordered, deferred: pre_deferred })
}

/// Dumps one table's live rows: `SELECT CAST(<col> AS CHAR CHARACTER SET utf8mb4) … FROM <t> ORDER
/// BY <primary key>` (§3.2) — exact text for every SQL type, `NULL` -> `None`. A `BLOB`/`BINARY`/
/// `VARBINARY`/`BIT` column anywhere makes the table `INTERNAL` (none exist in this schema; the test
/// suite pins it).
async fn dump_table<C: ConnectionTrait>(conn: &C, table: &str) -> TxResult<TableDump> {
    let columns = table_columns(conn, table).await?;
    let dumpable: Vec<&ColumnInfo> = columns.iter().filter(|c| !c.is_generated).collect();

    for c in &dumpable {
        let dt = c.data_type.to_uppercase();
        if dt.contains("BLOB") || dt.contains("BINARY") || dt == "BIT" {
            return Err(TxError::App(AppError::internal(
                "تعذر تصدير النسخة الاحتياطية — عمود ثنائي غير مدعوم",
                Some(format!("{table}.{} is {dt}", c.name)),
            )));
        }
    }

    let pk = primary_key_columns(conn, table).await?;
    let select_list = dumpable
        .iter()
        .map(|c| format!("CAST(`{}` AS CHAR CHARACTER SET utf8mb4) AS `{}`", c.name, c.name))
        .collect::<Vec<_>>()
        .join(", ");
    let order_by = if pk.is_empty() { String::new() } else { format!(" ORDER BY {}", pk.iter().map(|c| format!("`{c}`")).collect::<Vec<_>>().join(", ")) };
    let sql = if dumpable.is_empty() {
        format!("SELECT 1 FROM `{table}` LIMIT 0")
    } else {
        format!("SELECT {select_list} FROM `{table}`{order_by}")
    };

    let stmt = Statement::from_string(conn.get_database_backend(), sql);
    let query_rows = conn.query_all(stmt).await.map_err(TxError::from)?;
    let column_names: Vec<String> = dumpable.iter().map(|c| c.name.clone()).collect();

    let mut rows = Vec::with_capacity(query_rows.len());
    for row in query_rows {
        let mut values = Vec::with_capacity(column_names.len());
        for name in &column_names {
            let v: Option<String> = row.try_get("", name).map_err(TxError::from)?;
            values.push(v);
        }
        rows.push(values);
    }

    Ok(TableDump { name: table.to_string(), columns: column_names, rows })
}

/// Dumps every table in `order.tables`, in that order (§3.2). Called inside one `with_read` snapshot
/// (P2-06) so the whole archive reflects one consistent point in time.
pub async fn dump_all<C: ConnectionTrait>(conn: &C, order: &TopoOrder) -> TxResult<Vec<TableDump>> {
    let mut out = Vec::with_capacity(order.tables.len());
    for table in &order.tables {
        out.push(dump_table(conn, table).await?);
    }
    Ok(out)
}

/// Builds the whole `DataSetV1` for the current point-in-time snapshot: topo-order, every table's
/// rows, the schema version and applied migration names — the one call every archive-writing path
/// (`auto::build_and_write_auto_backup`, `restore`'s pre-restore backup, `pre_migration`) shares
/// instead of re-assembling `DataSetV1` by hand.
pub async fn dump_snapshot<C: ConnectionTrait>(conn: &C) -> TxResult<DataSetV1> {
    let order = topo_order(conn).await?;
    let tables = dump_all(conn, &order).await?;
    let schema_version = db_schema_version(conn).await?;
    let migrations = applied_migration_names(conn).await?;
    Ok(DataSetV1 { format: DATASET_FORMAT.to_string(), db_schema_version: schema_version, migrations, tables })
}

/// `wipe(conn, order)` (§3.2): clear every deferred FK column first (`UPDATE <t> SET <col> = NULL`),
/// then `DELETE FROM` every table in **reverse** topological order (children before parents).
pub async fn wipe<C: ConnectionTrait>(conn: &C, order: &TopoOrder) -> TxResult<()> {
    for d in &order.deferred {
        let stmt = Statement::from_string(conn.get_database_backend(), format!("UPDATE `{}` SET `{}` = NULL", d.table, d.column));
        conn.execute(stmt).await.map_err(TxError::from)?;
    }
    for table in order.tables.iter().rev() {
        let stmt = Statement::from_string(conn.get_database_backend(), format!("DELETE FROM `{table}`"));
        conn.execute(stmt).await.map_err(TxError::from)?;
    }
    Ok(())
}

/// `load(conn, dataset, order)` (§3.2): batched multi-row `INSERT INTO <t> (<archive columns>)
/// VALUES (?, …)` (≤ 200 rows/statement) with string parameters, in `order.tables` order, then the
/// deferred `UPDATE`s (needs every table's rows to already be in place so both endpoints of a
/// deferred FK exist). An archive column missing from the current table is `VALIDATION` (only
/// reachable if a row upgrader is missing for a schema change — §3.3).
pub async fn load<C: ConnectionTrait>(conn: &C, dataset: &DataSetV1, order: &TopoOrder) -> TxResult<()> {
    let dumps: HashMap<&str, &TableDump> = dataset.tables.iter().map(|t| (t.name.as_str(), t)).collect();

    for table in &order.tables {
        let Some(dump) = dumps.get(table.as_str()) else { continue }; // a table with zero archived rows may be absent.
        if dump.rows.is_empty() {
            continue;
        }

        let current_columns: HashSet<String> = table_columns(conn, table).await?.into_iter().filter(|c| !c.is_generated).map(|c| c.name).collect();
        for col in &dump.columns {
            if !current_columns.contains(col) {
                return Err(TxError::App(AppError::validation("هذه النسخة غير متوافقة مع إصدار قاعدة البيانات الحالي")));
            }
        }

        let quoted_columns = dump.columns.iter().map(|c| format!("`{c}`")).collect::<Vec<_>>().join(", ");
        for chunk in dump.rows.chunks(MAX_ROWS_PER_INSERT) {
            let mut placeholders = Vec::with_capacity(chunk.len());
            let mut values: Vec<sea_orm::Value> = Vec::with_capacity(chunk.len() * dump.columns.len());
            for row in chunk {
                placeholders.push(format!("({})", vec!["?"; row.len()].join(", ")));
                for v in row {
                    values.push(v.clone().into());
                }
            }
            let sql = format!("INSERT INTO `{table}` ({quoted_columns}) VALUES {}", placeholders.join(", "));
            let stmt = Statement::from_sql_and_values(conn.get_database_backend(), &sql, values);
            conn.execute(stmt).await.map_err(TxError::from)?;
        }
    }

    // Deferred FK backfill: one UPDATE per row that has a resolvable (non-null) value for that
    // column in the archive — requires the column list + PK columns of the owning table.
    for d in &order.deferred {
        let Some(dump) = dumps.get(d.table.as_str()) else { continue };
        let Some(col_idx) = dump.columns.iter().position(|c| c == &d.column) else { continue };
        let pk_columns = primary_key_columns(conn, &d.table).await?;
        let pk_indices: Vec<Option<usize>> = pk_columns.iter().map(|pk| dump.columns.iter().position(|c| c == pk)).collect();
        if pk_indices.iter().any(|i| i.is_none()) {
            continue; // PK not in the archive's column list for this table — nothing safe to key by.
        }
        let pk_indices: Vec<usize> = pk_indices.into_iter().flatten().collect();

        for row in &dump.rows {
            let Some(value) = &row[col_idx] else { continue }; // NULL in the archive: nothing to backfill.
            let where_clause = pk_columns.iter().map(|c| format!("`{c}` = ?")).collect::<Vec<_>>().join(" AND ");
            let sql = format!("UPDATE `{}` SET `{}` = ? WHERE {where_clause}", d.table, d.column);
            let mut values: Vec<sea_orm::Value> = vec![sea_orm::Value::from(value.clone())];
            for &idx in &pk_indices {
                values.push(row[idx].clone().into());
            }
            let stmt = Statement::from_sql_and_values(conn.get_database_backend(), &sql, values);
            conn.execute(stmt).await.map_err(TxError::from)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_schema_version_is_100_plus_migration_count() {
        use migration::MigratorTrait;
        let expected = 100 + migration::Migrator::migrations().len() as u32;
        assert_eq!(build_schema_version(), expected);
    }

    #[test]
    fn dataset_format_constant_is_equal_db() {
        assert_eq!(DATASET_FORMAT, "equal-db");
    }
}
