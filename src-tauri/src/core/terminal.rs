//! Terminal identity (cross-cutting.md §2). Generated once, on first launch, and never
//! regenerated — must exist and be readable before any DB connection is even configured (the
//! device-settings connection string is itself a device setting, and the terminal needs an
//! identity to log "which terminal changed the connection string" from its very first run).

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::utils::dates::iso_ms;
use crate::utils::id::Id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalIdentity {
    #[serde(rename = "terminalId")]
    pub terminal_id: Id,
    #[serde(rename = "createdAt", with = "iso_ms")]
    pub created_at: DateTime<Utc>,
}

const FILE_NAME: &str = "terminal.json";

/// Loads `<app_data_dir>/terminal.json`, creating it (with a fresh `Id`) if absent. Written
/// atomically (temp file + rename) so a crash mid-write can never corrupt or half-write it.
pub fn load_or_create(app_data_dir: &Path) -> std::io::Result<TerminalIdentity> {
    let path = app_data_dir.join(FILE_NAME);
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(identity) = serde_json::from_str::<TerminalIdentity>(&text) {
            return Ok(identity);
        }
        // Corrupt/unreadable file: fall through and recreate rather than crash-looping forever.
    }
    let identity = TerminalIdentity { terminal_id: Id::new(), created_at: Utc::now() };
    write_atomic(app_data_dir, &path, &identity)?;
    Ok(identity)
}

fn write_atomic(dir: &Path, path: &Path, identity: &TerminalIdentity) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let tmp_path = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(identity).expect("TerminalIdentity always serializes");
    std::fs::write(&tmp_path, json)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

pub fn file_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(FILE_NAME)
}

/// A2-10 (P2-55): restores `terminal.json` with a specific, already-known id rather than minting a
/// new one — used when this PC's `terminal.json` was lost but `%ProgramData%\...\database\server.json`
/// still names it as `hostTerminalId` (this PC *is* that terminal; a fresh random id would silently
/// disconnect it from its own audit trail/history). Written the same atomic way as `load_or_create`.
pub fn restore(app_data_dir: &Path, terminal_id: crate::utils::id::Id) -> std::io::Result<TerminalIdentity> {
    let path = app_data_dir.join(FILE_NAME);
    let identity = TerminalIdentity { terminal_id, created_at: Utc::now() };
    write_atomic(app_data_dir, &path, &identity)?;
    Ok(identity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_once_and_stable_across_reloads() {
        let dir = std::env::temp_dir().join(format!("equal-terminal-test-{}", Id::new()));
        let first = load_or_create(&dir).unwrap();
        let second = load_or_create(&dir).unwrap();
        assert_eq!(first.terminal_id, second.terminal_id);
        // Compared at millisecond precision (iso_ms's own resolution) rather than exact
        // equality — the round trip through JSON necessarily truncates to milliseconds.
        assert_eq!(crate::utils::dates::format_iso_ms(first.created_at), crate::utils::dates::format_iso_ms(second.created_at));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
