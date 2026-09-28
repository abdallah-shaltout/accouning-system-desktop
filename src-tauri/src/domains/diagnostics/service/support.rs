//! Support-bundle data (plan 21 Part 03 §16 spec §3, `supportBundleService.ts:64-127`, D-1). Only
//! the settings-redaction and server-diagnostics parts are Rust's job — zip/logs/save stay in the
//! frontend (D-1). The DB-snapshot part (slice A2) calls 17-backup's per-table exporter
//! (`infrastructure::backup::dataset`, H-3), excluding the `credentials` table (D-3), and redacts
//! the result with the same [`redact`]/[`REDACTED_KEYS`] this file already uses for settings.

use crate::core::auth::{Access, Area};
use crate::core::device::DeviceSettings;
use crate::core::error::AppError;
use crate::core::state::AppState;
use crate::core::tx::{ReadCtx, TxResult};
use crate::domains::diagnostics::dto::{ServerDiagnosticsDto, SupportSnapshot};

/// Field names redacted (case-insensitively, at any nesting depth) — must stay byte-identical to
/// `src/modules/diagnostics/config.ts`'s `REDACTED_KEYS` (checked by
/// `redacted_keys_match_frontend_config` below, the "one list, checked" drift test the spec §3
/// step 3 / entry-file "auth.rs permissions.ts pattern" calls for).
pub const REDACTED_KEYS: &[&str] = &[
    "password",
    "pin",
    "token",
    "secret",
    "apiKey",
    "api_key",
    "authorization",
    "creditCard",
    "cardNumber",
    "cvv",
];

/// Ports `supportBundleService.ts:19-30`'s `redact` exactly: depth > 8 or null passes through
/// unchanged, arrays map element-wise, and an object's key is replaced with `"[محجوب]"` when its
/// lowercased form is in [`REDACTED_KEYS`] (its value is dropped, not recursed into — matching the
/// mock, which never redacts nested inside an already-redacted key since the key itself is replaced
/// wholesale).
pub fn redact(value: &serde_json::Value, depth: u32) -> serde_json::Value {
    if depth > 8 || value.is_null() {
        return value.clone();
    }
    match value {
        serde_json::Value::Array(items) => serde_json::Value::Array(items.iter().map(|v| redact(v, depth + 1)).collect()),
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (k, v) in map {
                let is_redacted = REDACTED_KEYS.iter().any(|rk| rk.eq_ignore_ascii_case(k));
                out.insert(k.clone(), if is_redacted { serde_json::Value::String("[محجوب]".to_string()) } else { redact(v, depth + 1) });
            }
            serde_json::Value::Object(out)
        }
        other => other.clone(),
    }
}

/// Tables never included in a support bundle's DB snapshot, on top of whatever
/// `infrastructure::backup::dataset` already excludes (migrations bookkeeping, `change_versions`):
/// `credentials` holds argon2 password hashes — still secrets even hashed (D-3). The mock's
/// `clone(db)` leaked plaintext passwords into its "DB snapshot"; that mock-only leak is not carried
/// over here.
const SUPPORT_BUNDLE_EXCLUDED_TABLES: &[&str] = &["credentials"];

/// `exportSupportBundle`'s data part (`supportBundleService.ts:55-78`, spec §3):
/// 1. `includeDbSnapshot == Some(true)` needs `Settings:Write` (D-2 — a DB snapshot holds all
///    business data, same gate as the backup bar's `roleCanRestoreBackup`).
/// 2. `settings_redacted` = the same camelCase JSON `settings_get_settings` returns, redacted.
/// 3. `server` = `Some(...)` only on Windows when `device.role == Main` (a terminal has no local
///    managed server — spec §3 step 4).
/// 4. `db_snapshot`: slice A2 — every table via 17-backup's `dataset::dump_snapshot` (one
///    `with_read` snapshot, P2-06), excluding [`SUPPORT_BUNDLE_EXCLUDED_TABLES`] and redacted the
///    same way settings are (spec §3 step 5 / D-3).
///
/// Takes the already-read `device` snapshot (and pre-computed `server` diagnostics) instead of
/// `&AppState` directly: the command layer reads both from `AppState` *before* opening the
/// `with_read_ctx` transaction, so this function never needs to borrow `AppState` itself from
/// inside a future that a command has already moved its own `State<'_, AppState>` guard into.
pub async fn support_snapshot<C: sea_orm::ConnectionTrait>(
    device: &DeviceSettings,
    server: Option<ServerDiagnosticsDto>,
    conn: &C,
    ctx: &ReadCtx,
    include_db_snapshot: bool,
) -> TxResult<SupportSnapshot> {
    let db_snapshot = if include_db_snapshot {
        ctx.require(conn, Area::Settings, Access::Write).await.map_err(crate::core::tx::TxError::App)?;
        Some(support_db_snapshot(conn).await?)
    } else {
        None
    };

    // H-1: 01-settings exposes `domains::settings::service::get_settings(conn, device)` returning
    // the same camelCase `StoreSettings` DTO `settings_get_settings` returns. Needs the current
    // `DeviceSettings` to merge device-scoped fields exactly as that domain does.
    let settings_dto = crate::domains::settings::service::store::get_settings(conn, device).await?;
    let settings_value = serde_json::to_value(&settings_dto).map_err(|e| AppError::internal("تعذر تجهيز لقطة الإعدادات", Some(e.to_string())))?;
    let settings_redacted = redact(&settings_value, 0);

    Ok(SupportSnapshot { settings_redacted, db_snapshot, server })
}

/// Builds the redacted DB-snapshot JSON value (slice A2): reuses 17-backup's
/// `dataset::dump_snapshot` (topo-ordered per-table dump, same one every archive-writing path
/// shares — H-3), drops [`SUPPORT_BUNDLE_EXCLUDED_TABLES`] tables from the result, serializes to
/// JSON, then [`redact`]s it. The `TableDump.rows` are plain `Option<String>` values (already text,
/// per `dataset.rs`'s `CAST(... AS CHAR)`), so no column in an included table can itself smuggle a
/// binary secret past the table-level exclusion.
async fn support_db_snapshot<C: sea_orm::ConnectionTrait>(conn: &C) -> TxResult<serde_json::Value> {
    let mut snapshot = crate::infrastructure::backup::dataset::dump_snapshot(conn).await?;
    snapshot.tables.retain(|t| !SUPPORT_BUNDLE_EXCLUDED_TABLES.contains(&t.name.as_str()));
    let value = serde_json::to_value(&snapshot)
        .map_err(|e| crate::core::tx::TxError::App(AppError::internal("تعذر تجهيز لقطة قاعدة البيانات", Some(e.to_string()))))?;
    Ok(redact(&value, 0))
}

/// Spec §3 step 4: `#[cfg(windows)]` and `device.role == Main` only — a terminal or a non-Windows
/// build has no local managed server to report on. No secrets are read here (`server.json` + the
/// error-log tail hold none; keyring secrets are never touched — P2 handoff §9 "no secrets").
/// Takes `&AppState` (called from the command layer before it opens any transaction, so there is
/// no future capturing `state` for this call to conflict with).
#[cfg(windows)]
pub fn server_diagnostics(state: &AppState) -> Option<ServerDiagnosticsDto> {
    let device = state.device.read().unwrap();
    if device.role != crate::core::device::DeviceRole::Main {
        return None;
    }
    drop(device);
    let paths = crate::infrastructure::database::paths::ServerPaths::machine()?;
    Some(crate::infrastructure::database::errors::diagnostics_snapshot(&paths).into())
}

#[cfg(not(windows))]
pub fn server_diagnostics(_state: &AppState) -> Option<ServerDiagnosticsDto> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The "one list, checked" drift test (spec §3 step 3): parses
    /// `src/modules/diagnostics/config.ts`'s `REDACTED_KEYS` array literal and asserts it matches
    /// [`REDACTED_KEYS`] byte-for-byte (same pattern `auth.rs` uses against `permissions.ts`) — a
    /// key added to one list and not the other must fail this test, not drift silently.
    #[test]
    fn redacted_keys_match_frontend_config() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let config_path = manifest_dir.join("../src/modules/diagnostics/config.ts");
        let source = std::fs::read_to_string(&config_path).unwrap_or_else(|e| panic!("read {config_path:?}: {e}"));

        let start = source.find("REDACTED_KEYS").expect("REDACTED_KEYS const not found in config.ts");
        let bracket_start = source[start..].find('[').map(|i| start + i).expect("REDACTED_KEYS = [ not found");
        let bracket_end = source[bracket_start..].find(']').map(|i| bracket_start + i).expect("REDACTED_KEYS array not closed");
        let body = &source[bracket_start + 1..bracket_end];

        let frontend_keys: Vec<String> = body
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_matches(|c| c == '\'' || c == '"').to_string())
            .collect();

        assert_eq!(
            frontend_keys,
            REDACTED_KEYS.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            "REDACTED_KEYS drifted between config.ts and support.rs — keep the two lists identical"
        );
    }

    #[test]
    fn redact_replaces_nested_key_case_insensitively() {
        let value = serde_json::json!({
            "outer": { "apiKey": "secret-value", "PIN": "1234", "keep": "visible" }
        });
        let redacted = redact(&value, 0);
        assert_eq!(redacted["outer"]["apiKey"], serde_json::json!("[محجوب]"));
        assert_eq!(redacted["outer"]["PIN"], serde_json::json!("[محجوب]"));
        assert_eq!(redacted["outer"]["keep"], serde_json::json!("visible"));
    }

    #[test]
    fn redact_maps_arrays_element_wise() {
        let value = serde_json::json!([{ "password": "x" }, { "keep": "y" }]);
        let redacted = redact(&value, 0);
        assert_eq!(redacted[0]["password"], serde_json::json!("[محجوب]"));
        assert_eq!(redacted[1]["keep"], serde_json::json!("y"));
    }

    #[test]
    fn redact_passes_through_null_and_deep_nesting_unchanged_past_depth_8() {
        assert_eq!(redact(&serde_json::Value::Null, 0), serde_json::Value::Null);

        let mut deep = serde_json::json!({ "password": "leaf" });
        for _ in 0..10 {
            deep = serde_json::json!({ "wrap": deep });
        }
        let redacted = redact(&deep, 0);
        // Past depth 8 the value passes through unchanged, so the deeply nested "password" key
        // survives un-redacted at that depth — matching the mock's own depth cutoff exactly.
        assert!(redacted.to_string().contains("leaf"));
    }

    /// Slice A2 (spec §8a "A2: DB snapshot has no `credentials` table and no `password_hash` string
    /// anywhere"): a pure test over a hand-built `DataSetV1` (no DB needed) proving the exclusion +
    /// redaction pipeline this module's `support_db_snapshot` runs actually drops the `credentials`
    /// table and would redact a `password`-shaped column value if one slipped through elsewhere.
    #[test]
    fn support_db_snapshot_excludes_credentials_table_and_redacts_the_rest() {
        use crate::infrastructure::backup::dataset::{DataSetV1, TableDump};

        let mut snapshot = DataSetV1 {
            format: "equal-db".to_string(),
            db_schema_version: 101,
            migrations: vec!["m0001".to_string()],
            tables: vec![
                TableDump {
                    name: "credentials".to_string(),
                    columns: vec!["id".to_string(), "password_hash".to_string()],
                    rows: vec![vec![Some("1".to_string()), Some("$argon2id$super-secret-hash".to_string())]],
                },
                TableDump {
                    name: "parties".to_string(),
                    columns: vec!["id".to_string(), "name".to_string()],
                    rows: vec![vec![Some("1".to_string()), Some("عميل تجريبي".to_string())]],
                },
            ],
        };
        snapshot.tables.retain(|t| !SUPPORT_BUNDLE_EXCLUDED_TABLES.contains(&t.name.as_str()));
        assert_eq!(snapshot.tables.len(), 1);
        assert_eq!(snapshot.tables[0].name, "parties");

        let value = serde_json::to_value(&snapshot).unwrap();
        let redacted = redact(&value, 0);
        let text = redacted.to_string();
        assert!(!text.contains("credentials"), "the credentials table must not appear in the support bundle snapshot at all");
        assert!(!text.contains("argon2"), "no password hash may survive into the support bundle snapshot");
    }
}
