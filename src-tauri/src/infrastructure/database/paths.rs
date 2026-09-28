//! Filesystem layout for the bundled, app-managed MariaDB server (phase-a2 §Layout, P2-44/P2-45).
//!
//! Production layout lives under `%ProgramData%\com.abdallah.accounting-app\database\` (shared by
//! every Windows user on the Main PC — the service-like supervisor runs per-machine, not
//! per-user). Tests and `db_dev_server` use `ServerPaths::at` with an arbitrary root instead.

use std::path::{Path, PathBuf};

/// Reused from `core::device`'s keyring service identifier — the same app identifier names both
/// the OS keyring service and the `%ProgramData%` subfolder.
pub const APP_IDENTIFIER: &str = "com.abdallah.accounting-app";

#[derive(Debug, Clone)]
pub struct ServerPaths {
    pub root: PathBuf,
}

impl ServerPaths {
    /// Test/dev constructor: an arbitrary root directory (temp dir for tests, `target/mariadb-dev`
    /// for `db_dev_server`).
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Production constructor: `%ProgramData%\com.abdallah.accounting-app\database`, resolved via
    /// `FOLDERID_ProgramData` (Win32-only; every other target returns `None` since there is no
    /// managed server on those platforms).
    #[cfg(windows)]
    pub fn machine() -> Option<Self> {
        let program_data = super::win::known_folder_program_data()?;
        Some(Self { root: program_data.join(APP_IDENTIFIER).join("database") })
    }

    #[cfg(not(windows))]
    pub fn machine() -> Option<Self> {
        None
    }

    pub fn server_json(&self) -> PathBuf {
        self.root.join("server.json")
    }

    pub fn my_ini(&self) -> PathBuf {
        self.root.join("my.ini")
    }

    pub fn readme(&self) -> PathBuf {
        self.root.join("README-اقرأني.txt")
    }

    pub fn server_bin(&self) -> PathBuf {
        self.root.join("server-bin")
    }

    pub fn server_bin_current(&self) -> PathBuf {
        self.server_bin().join("current")
    }

    pub fn server_bin_staging(&self) -> PathBuf {
        self.server_bin().join("staging")
    }

    pub fn server_bin_previous(&self) -> PathBuf {
        self.server_bin().join("previous")
    }

    pub fn mariadbd_exe(&self) -> PathBuf {
        self.server_bin_current().join("bin").join("mariadbd.exe")
    }

    pub fn mariadb_install_db_exe(&self) -> PathBuf {
        self.server_bin_current().join("bin").join("mariadb-install-db.exe")
    }

    pub fn mariadb_upgrade_exe(&self) -> PathBuf {
        self.server_bin_current().join("bin").join("mariadb-upgrade.exe")
    }

    pub fn mariadb_exe(&self) -> PathBuf {
        self.server_bin_current().join("bin").join("mariadb.exe")
    }

    pub fn data(&self) -> PathBuf {
        self.root.join("data")
    }

    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn error_log(&self) -> PathBuf {
        self.logs().join("error.log")
    }

    pub fn tmp(&self) -> PathBuf {
        self.root.join("tmp")
    }

    pub fn pid_file(&self) -> PathBuf {
        self.root.join("mariadbd.pid")
    }

    pub fn shutdown_requested(&self) -> PathBuf {
        self.root.join("shutdown.requested")
    }

    pub fn pre_upgrade_dir(&self) -> PathBuf {
        self.root.join("pre-upgrade")
    }

    /// G-46/GB-4: the managed server's own backups root — `database\backups\` — for automatic
    /// backups (pre-migration dumps, the daily/close auto backup on the Main PC, the mandatory
    /// pre-restore snapshot). Deliberately under this same managed-server tree, never under
    /// `$INSTDIR`/`$APPDATA` (P2-44: an uninstall or app-data reset must not silently delete backups
    /// that live right next to the data they protect).
    pub fn backups(&self) -> PathBuf {
        self.root.join("backups")
    }

    /// Creates every directory this layout needs (idempotent — `create_dir_all` no-ops when
    /// already present).
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.root)?;
        std::fs::create_dir_all(self.server_bin())?;
        std::fs::create_dir_all(self.data())?;
        std::fs::create_dir_all(self.logs())?;
        std::fs::create_dir_all(self.tmp())?;
        std::fs::create_dir_all(self.backups())?;
        Ok(())
    }

    /// `my.ini` (and any other config value) needs forward slashes — MariaDB's `.ini` parser on
    /// Windows treats a backslash as an escape character in some contexts, so every path written
    /// into it uses `/` (P2-48).
    pub fn to_ini_path(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ini_path_uses_forward_slashes_and_has_no_backslash() {
        let p = Path::new(r"C:\ProgramData\com.abdallah.accounting-app\database\data");
        let ini = ServerPaths::to_ini_path(p);
        assert!(!ini.contains('\\'));
        assert_eq!(ini, "C:/ProgramData/com.abdallah.accounting-app/database/data");
    }

    #[test]
    fn layout_paths_are_rooted_correctly() {
        let paths = ServerPaths::at(r"C:\test-root");
        assert_eq!(paths.server_json(), PathBuf::from(r"C:\test-root\server.json"));
        assert_eq!(paths.mariadbd_exe(), PathBuf::from(r"C:\test-root\server-bin\current\bin\mariadbd.exe"));
        assert_eq!(paths.data(), PathBuf::from(r"C:\test-root\data"));
    }

    #[test]
    fn backups_dir_is_under_the_managed_server_root() {
        let paths = ServerPaths::at(r"C:\test-root");
        assert_eq!(paths.backups(), PathBuf::from(r"C:\test-root\backups"));
    }

    #[test]
    fn ensure_dirs_creates_the_backups_dir() {
        let tmp = std::env::temp_dir().join(format!("equal-server-paths-test-{}", std::process::id()));
        let paths = ServerPaths::at(&tmp);
        paths.ensure_dirs().expect("ensure_dirs must succeed");
        assert!(paths.backups().is_dir());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
