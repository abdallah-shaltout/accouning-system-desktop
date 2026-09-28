//! The bundled MariaDB payload (phase-a2 A2-4): where to find it (installed resource dir, or the
//! dev-tree folder `scripts/fetch-mariadb.js` downloads), and its `EQUAL-PAYLOAD.json` manifest.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The only MariaDB major.minor series this build supports (P2-51 series-mismatch gate). Bumping
/// this is a deliberate, tested decision — never a drive-by edit.
pub const SUPPORTED_SERIES: &str = "11.4";
/// The exact patch version this build ships (must match `scripts/fetch-mariadb.js`'s `VERSION` —
/// enforced by a drift-guard unit test in this module).
pub const BUNDLED_VERSION: &str = "11.4.13";

#[derive(Debug, Clone, Deserialize)]
pub struct PayloadManifest {
    pub series: String,
    pub version: String,
    #[allow(dead_code)]
    pub sha256: String,
    #[allow(dead_code)]
    pub files: u64,
    #[allow(dead_code)]
    pub bytes: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum PayloadError {
    #[error("payload-missing")]
    Missing,
    #[error("payload manifest is corrupt: {0}")]
    Corrupt(String),
    #[error("payload series/version mismatch: manifest has {series} {version}, this build expects {SUPPORTED_SERIES} {BUNDLED_VERSION}", series = .0, version = .1)]
    Mismatch(String, String),
}

pub struct PayloadDir {
    pub root: PathBuf,
}

impl PayloadDir {
    /// Installed layout: `resource_dir()/mariadb` — the folder `tauri.windows.conf.json`'s
    /// `bundle.resources` maps `mariadb-<version>-winx64/` onto (A2-2).
    #[cfg(windows)]
    pub fn from_app(app: &tauri::AppHandle) -> Option<Self> {
        use tauri::Manager;
        let dir = app.path().resource_dir().ok()?.join("mariadb");
        Some(Self { root: dir })
    }

    #[cfg(not(windows))]
    pub fn from_app(_app: &tauri::AppHandle) -> Option<Self> {
        None
    }

    /// Dev-tree layout: the versioned folder `scripts/fetch-mariadb.js` downloads directly next to
    /// `src-tauri/Cargo.toml`, used by `tauri dev` and `db_dev_server`.
    pub fn dev() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("mariadb-{BUNDLED_VERSION}-winx64"));
        Self { root }
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join("EQUAL-PAYLOAD.json")
    }

    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    /// Reads and validates `EQUAL-PAYLOAD.json`: it must exist, parse, and name this exact
    /// series/version (a mismatched or absent manifest is always `PayloadError`, never a silent
    /// "assume it's fine").
    pub fn read_manifest(&self) -> Result<PayloadManifest, PayloadError> {
        let path = self.manifest_path();
        let text = std::fs::read_to_string(&path).map_err(|_| PayloadError::Missing)?;
        let manifest: PayloadManifest = serde_json::from_str(&text).map_err(|e| PayloadError::Corrupt(e.to_string()))?;
        if manifest.series != SUPPORTED_SERIES || manifest.version != BUNDLED_VERSION {
            return Err(PayloadError::Mismatch(manifest.series, manifest.version));
        }
        Ok(manifest)
    }
}

/// `major.minor.patch` version comparison for the upgrade/downgrade gate (A2-9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn parse(s: &str) -> Option<Self> {
        // Tolerate a MariaDB-style suffix (e.g. "11.4.2-MariaDB") by taking the part before '-'.
        let numeric = s.split('-').next().unwrap_or(s);
        let mut parts = numeric.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        Some(Self { major, minor, patch })
    }

    pub fn series(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }
}

/// Parses `scripts/fetch-mariadb.js` for its `SERIES`/`VERSION` constants and confirms they match
/// `SUPPORTED_SERIES`/`BUNDLED_VERSION` — same drift-guard technique phase-a-foundation.md's
/// `ROLE_ACCESS` test uses against `permissions.ts`, so the JS build script and the Rust
/// supervisor can never silently diverge.
pub fn read_fetch_script_constants(script_path: &Path) -> Option<(String, String)> {
    let text = std::fs::read_to_string(script_path).ok()?;
    let series = extract_const(&text, "SERIES")?;
    let version = extract_const(&text, "VERSION")?;
    Some((series, version))
}

fn extract_const(text: &str, name: &str) -> Option<String> {
    let needle = format!("const {name} = \"");
    let start = text.find(&needle)? + needle.len();
    let rest = &text[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parses_and_compares() {
        let a = Version::parse("11.4.12").unwrap();
        let b = Version::parse("11.4.13").unwrap();
        assert!(a < b);
        assert_eq!(a.series(), "11.4");
        assert!(Version::parse("garbage").is_none());
    }

    #[test]
    fn drift_guard_fetch_script_matches_supported_constants() {
        let script_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("scripts").join("fetch-mariadb.js");
        let (series, version) = read_fetch_script_constants(&script_path)
            .unwrap_or_else(|| panic!("could not read SERIES/VERSION from {}", script_path.display()));
        assert_eq!(series, SUPPORTED_SERIES, "scripts/fetch-mariadb.js SERIES drifted from SUPPORTED_SERIES");
        assert_eq!(version, BUNDLED_VERSION, "scripts/fetch-mariadb.js VERSION drifted from BUNDLED_VERSION");
    }
}
