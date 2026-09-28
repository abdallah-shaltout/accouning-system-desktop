//! `server.json` v1 (phase-a2 §Layout): the one durable record of "does a managed MariaDB server
//! exist here, and what state is it in." Read on every boot before anything else touches the data
//! directory — corrupt/unknown JSON must be a loud error, never silently treated as "no server"
//! (that would make `provision_main` re-initialize a data directory that actually holds business
//! data).

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::utils::dates::iso_ms;
use crate::utils::id::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerLifecycleState {
    Initializing,
    Ready,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStateFile {
    pub version: u32,
    pub state: ServerLifecycleState,
    #[serde(rename = "dataDirId")]
    pub data_dir_id: Id,
    pub series: String,
    /// The bundled version this data directory was first initialized with — never changes once
    /// set (used by `upgrade::plan_transition` to detect a downgrade attempt).
    #[serde(rename = "initializedWith")]
    pub initialized_with: String,
    /// The bundled version the server was last successfully started with — updated after every
    /// successful patch upgrade.
    #[serde(rename = "lastStartedWith")]
    pub last_started_with: String,
    pub port: u16,
    /// Written by A3; A2 only reads it and treats a missing/absent value as `false`.
    #[serde(default, rename = "lanSharing")]
    pub lan_sharing: bool,
    #[serde(rename = "hostTerminalId")]
    pub host_terminal_id: Id,
    #[serde(rename = "createdAt", with = "iso_ms")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt", with = "iso_ms")]
    pub updated_at: DateTime<Utc>,
    /// Set by `upgrade::run_patch_upgrade` when `mariadb-upgrade` exits non-zero — the server
    /// keeps running, but the UI/diagnostics should surface this.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "upgradeWarning")]
    pub upgrade_warning: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum StateFileError {
    #[error("could not read server.json: {0}")]
    Io(#[from] std::io::Error),
    #[error("server.json is corrupt or has an unrecognized shape: {0}")]
    Parse(#[from] serde_json::Error),
}

/// Loads `server.json`. Returns `Ok(None)` only when the file is genuinely absent (no server has
/// ever been provisioned here) — any other failure (unreadable, corrupt JSON) is `Err`, never
/// coerced into `None`.
pub fn load(path: &Path) -> Result<Option<ServerStateFile>, StateFileError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(serde_json::from_str(&text)?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Atomic write (temp file + rename), same mechanism as `core::terminal`/`core::device`.
pub fn save(path: &Path, state: &ServerStateFile) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp_path = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(state).expect("ServerStateFile always serializes");
    std::fs::write(&tmp_path, json)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ServerStateFile {
        let now = Utc::now();
        ServerStateFile {
            version: 1,
            state: ServerLifecycleState::Ready,
            data_dir_id: Id::new(),
            series: "11.4".to_string(),
            initialized_with: "11.4.13".to_string(),
            last_started_with: "11.4.13".to_string(),
            port: 3406,
            lan_sharing: false,
            host_terminal_id: Id::new(),
            created_at: now,
            updated_at: now,
            upgrade_warning: None,
        }
    }

    #[test]
    fn missing_file_is_none_not_an_error() {
        let dir = std::env::temp_dir().join(format!("equal-serverjson-missing-{}", Id::new()));
        let path = dir.join("server.json");
        assert!(load(&path).unwrap().is_none());
    }

    #[test]
    fn round_trips_atomically() {
        let dir = std::env::temp_dir().join(format!("equal-serverjson-roundtrip-{}", Id::new()));
        let path = dir.join("server.json");
        let state = sample();
        save(&path, &state).unwrap();
        let loaded = load(&path).unwrap().unwrap();
        assert_eq!(loaded.data_dir_id, state.data_dir_id);
        assert_eq!(loaded.port, state.port);
        assert!(matches!(loaded.state, ServerLifecycleState::Ready));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_json_is_an_error_not_a_silent_default() {
        let dir = std::env::temp_dir().join(format!("equal-serverjson-corrupt-{}", Id::new()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("server.json");
        std::fs::write(&path, "{ this is not valid json").unwrap();
        assert!(matches!(load(&path), Err(StateFileError::Parse(_))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_shape_is_an_error() {
        let dir = std::env::temp_dir().join(format!("equal-serverjson-unknown-{}", Id::new()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("server.json");
        std::fs::write(&path, r#"{"totallyDifferent": true}"#).unwrap();
        assert!(load(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
