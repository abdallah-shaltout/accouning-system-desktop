//! Device settings (cross-cutting.md §3, phase-a-foundation.md A-5): a single local JSON file,
//! `<app_data_dir>/device-settings.json`, holding this machine's own printer/connection settings —
//! never in the shared branch MariaDB row (a network `host`/Windows queue `printerName` is
//! meaningless on a different terminal).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "device-settings.json";
/// Also reused by `infrastructure::database::credentials` (A2-8) for the managed server's own
/// root/LAN secrets, keyed by a different account name so the two uses never collide.
pub const KEYRING_SERVICE: &str = "com.abdallah.accounting-app";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceRole {
    Main,
    Terminal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSettings {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    /// A3-2 (P2-57): the last IPv4 address this terminal successfully resolved `host` to. Kept so
    /// a terminal paired by the Main PC's DNS hostname can still find it after a DHCP address
    /// change, by falling back to this address when the hostname itself won't resolve/connect.
    /// Serde-optional — the on-disk file version stays 1 (an old file without this field parses to
    /// `None`, not an error).
    #[serde(rename = "lastKnownAddress", skip_serializing_if = "Option::is_none")]
    pub last_known_address: Option<String>,
}

impl ConnectionSettings {
    /// The keyring account name for this connection's password (P2-29):
    /// `db:{user}@{host}:{port}/{database}`.
    pub fn keyring_account(&self) -> String {
        format!("db:{}@{}:{}/{}", self.user, self.host, self.port, self.database)
    }

    /// P2-30: a freshly-saved connection defaults `role` to `main` when the host is loopback,
    /// else `terminal`.
    pub fn default_role(&self) -> DeviceRole {
        if matches!(self.host.as_str(), "localhost" | "127.0.0.1" | "::1") {
            DeviceRole::Main
        } else {
            DeviceRole::Terminal
        }
    }
}

/// Mirrors `ThermalPrinterSettings` in `settings/types/index.ts:85-95` (device-scoped fields only —
/// `mode`/`thermalWidthMm` stay branch-scoped, per cross-cutting.md §3's field table).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThermalPrinterConfig {
    #[serde(rename = "printerName", skip_serializing_if = "Option::is_none")]
    pub printer_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dpi: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cut: Option<bool>,
    #[serde(rename = "openDrawer", skip_serializing_if = "Option::is_none")]
    pub open_drawer: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copies: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DevicePrinterSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal: Option<ThermalPrinterConfig>,
    #[serde(rename = "a4PrinterName", skip_serializing_if = "Option::is_none")]
    pub a4_printer_name: Option<String>,
    #[serde(rename = "labelPrinterName", skip_serializing_if = "Option::is_none")]
    pub label_printer_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSettings {
    pub version: u32,
    pub role: DeviceRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<ConnectionSettings>,
    #[serde(default)]
    pub printer: DevicePrinterSettings,
    #[serde(rename = "backupFolder", skip_serializing_if = "Option::is_none")]
    pub backup_folder: Option<String>,
}

impl Default for DeviceSettings {
    fn default() -> Self {
        Self { version: 1, role: DeviceRole::Terminal, connection: None, printer: DevicePrinterSettings::default(), backup_folder: None }
    }
}

pub fn file_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(FILE_NAME)
}

/// Loads `device-settings.json`, or the default (unconfigured — `role: terminal`, no connection)
/// when the file doesn't exist yet.
pub fn load(app_data_dir: &Path) -> std::io::Result<DeviceSettings> {
    let path = file_path(app_data_dir);
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(serde_json::from_str(&text).unwrap_or_default()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(DeviceSettings::default()),
        Err(e) => Err(e),
    }
}

/// Saves `device-settings.json` atomically (temp file + rename), same mechanism as `terminal.rs`.
pub fn save(app_data_dir: &Path, settings: &DeviceSettings) -> std::io::Result<()> {
    std::fs::create_dir_all(app_data_dir)?;
    let path = file_path(app_data_dir);
    let tmp_path = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(settings).expect("DeviceSettings always serializes");
    std::fs::write(&tmp_path, json)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// A3-2 (P2-57): pairs this terminal against a Main PC's shared database. `host` is normally the
/// Main PC's DNS hostname (from `PairingInfo::host_name`) — never a bare IP, so the connection
/// keeps working across a DHCP address change (`core::db`'s terminal-connect fallback then updates
/// `lastKnownAddress` once it resolves). `code` must already be normalized (through
/// `infrastructure::database::pairing::parse_code`) before it reaches here — this function stores
/// it verbatim as the account's password, it does not itself parse pairing-code formatting.
pub fn pair_terminal(app_data_dir: &Path, host: &str, port: u16, code: &str, last_known_address: Option<String>) -> std::io::Result<()> {
    let connection = ConnectionSettings {
        host: host.to_string(),
        port,
        database: "equal".to_string(),
        user: "equal_lan".to_string(),
        last_known_address,
    };
    save_password(&connection, code).map_err(|e| std::io::Error::other(e.to_string()))?;

    let mut settings = load(app_data_dir)?;
    settings.role = DeviceRole::Terminal;
    settings.connection = Some(connection);
    save(app_data_dir, &settings)
}

/// Debug builds only: `EQUAL_DB_URL` overrides the connection entirely (phase-a-foundation.md A-5).
#[cfg(debug_assertions)]
pub fn debug_db_url_override() -> Option<String> {
    std::env::var("EQUAL_DB_URL").ok()
}

#[cfg(not(debug_assertions))]
pub fn debug_db_url_override() -> Option<String> {
    None
}

/// Reads the connection's password from the OS keyring (P2-29 — no plaintext fallback).
pub fn load_password(connection: &ConnectionSettings) -> Result<String, keyring::Error> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &connection.keyring_account())?;
    entry.get_password()
}

/// Writes the connection's password to the OS keyring.
pub fn save_password(connection: &ConnectionSettings, password: &str) -> Result<(), keyring::Error> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &connection.keyring_account())?;
    entry.set_password(password)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_role_from_host() {
        let localhost = ConnectionSettings {
            host: "localhost".into(),
            port: 3306,
            database: "equal".into(),
            user: "root".into(),
            last_known_address: None,
        };
        assert!(matches!(localhost.default_role(), DeviceRole::Main));

        let lan = ConnectionSettings {
            host: "192.168.1.10".into(),
            port: 3306,
            database: "equal".into(),
            user: "root".into(),
            last_known_address: None,
        };
        assert!(matches!(lan.default_role(), DeviceRole::Terminal));
    }

    #[test]
    fn load_missing_file_gives_default() {
        let dir = std::env::temp_dir().join(format!("equal-device-test-missing-{}", crate::utils::id::Id::new()));
        let settings = load(&dir).unwrap();
        assert!(matches!(settings.role, DeviceRole::Terminal));
        assert!(settings.connection.is_none());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = std::env::temp_dir().join(format!("equal-device-test-roundtrip-{}", crate::utils::id::Id::new()));
        let mut settings = DeviceSettings::default();
        settings.role = DeviceRole::Main;
        settings.printer.a4_printer_name = Some("HP LaserJet".to_string());
        save(&dir, &settings).unwrap();
        let loaded = load(&dir).unwrap();
        assert!(matches!(loaded.role, DeviceRole::Main));
        assert_eq!(loaded.printer.a4_printer_name.as_deref(), Some("HP LaserJet"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A3-2: `lastKnownAddress` is serde-optional — an old on-disk file (no such key at all) must
    /// still parse, with the field defaulting to `None`, and the file version must stay `1`.
    #[test]
    fn last_known_address_is_optional_and_round_trips() {
        let dir = std::env::temp_dir().join(format!("equal-device-test-lka-{}", crate::utils::id::Id::new()));

        // An old-shaped file with no `lastKnownAddress` key at all must still load.
        let old_shape = r#"{
            "version": 1,
            "role": "terminal",
            "connection": { "host": "MAINPC", "port": 3406, "database": "equal", "user": "equal_lan" }
        }"#;
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(file_path(&dir), old_shape).unwrap();
        let loaded = load(&dir).unwrap();
        assert_eq!(loaded.version, 1);
        let conn = loaded.connection.as_ref().expect("connection must parse");
        assert_eq!(conn.last_known_address, None);

        // Setting it and round-tripping through save/load must preserve it, version unchanged.
        let mut settings = loaded;
        settings.connection.as_mut().unwrap().last_known_address = Some("192.168.1.50".to_string());
        save(&dir, &settings).unwrap();
        let reloaded = load(&dir).unwrap();
        assert_eq!(reloaded.version, 1);
        assert_eq!(reloaded.connection.unwrap().last_known_address.as_deref(), Some("192.168.1.50"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pair_terminal_writes_role_connection_and_password() {
        let dir = std::env::temp_dir().join(format!("equal-device-test-pair-{}", crate::utils::id::Id::new()));
        // Use a disposable keyring service by pairing then immediately reading back through the
        // same public API — `save_password`/`load_password` always target `KEYRING_SERVICE`, so
        // this test exercises the real round trip rather than mocking the keyring away.
        let host = "TESTHOST-PAIRING";
        let code = "ABCD1234EFGH5678";
        pair_terminal(&dir, host, 3406, code, Some("192.168.1.77".to_string())).expect("pair_terminal must succeed");

        let settings = load(&dir).unwrap();
        assert!(matches!(settings.role, DeviceRole::Terminal));
        let conn = settings.connection.expect("connection must be set");
        assert_eq!(conn.host, host);
        assert_eq!(conn.port, 3406);
        assert_eq!(conn.database, "equal");
        assert_eq!(conn.user, "equal_lan");
        assert_eq!(conn.last_known_address.as_deref(), Some("192.168.1.77"));

        let password = load_password(&conn).expect("password must be readable back from the keyring");
        assert_eq!(password, code);

        // Clean up the keyring entry this test created.
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, &conn.keyring_account()) {
            let _ = entry.delete_credential();
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
