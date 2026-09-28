//! Version transition planning and execution (phase-a2 A2-9, P2-51): decide whether the bundled
//! payload is the same version, a patch upgrade, a downgrade (refused) or a series mismatch
//! (refused) relative to what a data directory was last started with — then, for a patch upgrade,
//! actually perform the cold-copy-and-swap.

use super::errors::ServerFailure;
use super::payload::Version;
use super::state_file::ServerStateFile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transition {
    Same,
    PatchUpgrade { from: String, to: String },
    Downgrade,
    SeriesMismatch,
}

/// Pure decision function — same inputs always produce the same `Transition`, so it's unit-tested
/// without touching disk. `last_started_with` on the state file is compared against the bundled
/// version (or, for an adopted running server, its live `SELECT VERSION()` — the caller passes
/// whichever is authoritative for that call site).
pub fn plan_transition(bundled: &str, state: &ServerStateFile) -> Transition {
    let Some(bundled_v) = Version::parse(bundled) else {
        return Transition::SeriesMismatch; // an unparsable bundled version can never be trusted
    };
    let Some(current_v) = Version::parse(&state.last_started_with) else {
        return Transition::SeriesMismatch;
    };

    if bundled_v.series() != current_v.series() {
        return Transition::SeriesMismatch;
    }
    if bundled_v == current_v {
        return Transition::Same;
    }
    if bundled_v > current_v {
        Transition::PatchUpgrade { from: state.last_started_with.clone(), to: bundled.to_string() }
    } else {
        Transition::Downgrade
    }
}

impl Transition {
    pub fn as_failure(&self) -> Option<ServerFailure> {
        match self {
            Transition::Downgrade => Some(ServerFailure::VersionDowngrade),
            Transition::SeriesMismatch => Some(ServerFailure::SeriesMismatch),
            Transition::Same | Transition::PatchUpgrade { .. } => None,
        }
    }
}

#[cfg(windows)]
pub mod runner {
    //! The actual patch-upgrade execution — cold-copies `data\` before swapping `server-bin`, runs
    //! `mariadb-upgrade`, and never touches files on `Downgrade`/`SeriesMismatch` (those are pure
    //! refusals surfaced by `Transition::as_failure`).

    use std::path::Path;
    use std::process::Stdio;
    use std::time::Duration;

    use super::super::paths::ServerPaths;
    use chrono::Utc;

    /// Required headroom before a cold copy of `data\` is attempted: 2× its size + 512 MB. If
    /// free space is short, the upgrade is postponed (old binaries keep running) rather than
    /// risking a half-finished copy.
    pub fn has_enough_free_space(paths: &ServerPaths, data_size_bytes: u64) -> std::io::Result<bool> {
        let free = free_space_bytes(&paths.root)?;
        let needed = data_size_bytes.saturating_mul(2).saturating_add(512 * 1024 * 1024);
        Ok(free >= needed)
    }

    #[cfg(windows)]
    fn free_space_bytes(path: &Path) -> std::io::Result<u64> {
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        let mut free_bytes: u64 = 0;
        unsafe {
            GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut free_bytes), None, None)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
        }
        Ok(free_bytes)
    }

    pub fn dir_size_bytes(dir: &Path) -> u64 {
        let mut total = 0u64;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_dir() {
                        total += dir_size_bytes(&entry.path());
                    } else {
                        total += meta.len();
                    }
                }
            }
        }
        total
    }

    /// Copies `data\` into `pre-upgrade\<from>-to-<to>-<timestamp>\`, keeping only the newest such
    /// snapshot (older ones are removed first).
    pub fn cold_copy_data(paths: &ServerPaths, from: &str, to: &str) -> std::io::Result<()> {
        let pre_upgrade_dir = paths.pre_upgrade_dir();
        if pre_upgrade_dir.exists() {
            for entry in std::fs::read_dir(&pre_upgrade_dir)?.flatten() {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
        let ts = Utc::now().format("%Y%m%d%H%M%S");
        let dest = pre_upgrade_dir.join(format!("{from}-to-{to}-{ts}"));
        copy_dir_recursive(&paths.data(), &dest)
    }

    fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)?.flatten() {
            let ty = entry.file_type()?;
            let dest_path = dst.join(entry.file_name());
            if ty.is_dir() {
                copy_dir_recursive(&entry.path(), &dest_path)?;
            } else {
                std::fs::copy(entry.path(), dest_path)?;
            }
        }
        Ok(())
    }

    /// Swaps `server-bin`: `staging` (already populated by the caller with the new payload) →
    /// `current`, moving the old `current` to `previous` first (removed if it already exists from
    /// a prior swap).
    pub fn swap_server_bin(paths: &ServerPaths) -> std::io::Result<()> {
        let current = paths.server_bin_current();
        let staging = paths.server_bin_staging();
        let previous = paths.server_bin_previous();
        if previous.exists() {
            std::fs::remove_dir_all(&previous)?;
        }
        if current.exists() {
            std::fs::rename(&current, &previous)?;
        }
        std::fs::rename(&staging, &current)?;
        Ok(())
    }

    /// Runs `mariadb-upgrade.exe` against the now-running new server. A non-zero exit does not
    /// stop the server — it's logged and recorded as `upgradeWarning` in `server.json` by the
    /// caller.
    pub fn run_mariadb_upgrade(paths: &ServerPaths, port: u16, root_password: &str) -> std::io::Result<Result<(), String>> {
        let exe = paths.mariadb_upgrade_exe();
        let output = std::process::Command::new(&exe)
            .arg("--host=127.0.0.1")
            .arg(format!("--port={port}"))
            .arg("--user=root")
            .arg(format!("--password={root_password}"))
            .arg("--silent")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;
        if output.status.success() {
            Ok(Ok(()))
        } else {
            let combined = format!(
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            Ok(Err(combined))
        }
    }

    #[allow(dead_code)]
    pub const UPGRADE_TIMEOUT: Duration = Duration::from_secs(180);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::id::Id;
    use chrono::Utc;

    fn state_with(last_started_with: &str) -> ServerStateFile {
        ServerStateFile {
            version: 1,
            state: super::super::state_file::ServerLifecycleState::Ready,
            data_dir_id: Id::new(),
            series: "11.4".to_string(),
            initialized_with: last_started_with.to_string(),
            last_started_with: last_started_with.to_string(),
            port: 3406,
            lan_sharing: false,
            host_terminal_id: Id::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            upgrade_warning: None,
        }
    }

    #[test]
    fn same_version_is_a_no_op() {
        let state = state_with("11.4.13");
        assert_eq!(plan_transition("11.4.13", &state), Transition::Same);
    }

    #[test]
    fn patch_upgrade_within_series() {
        let state = state_with("11.4.12");
        assert_eq!(
            plan_transition("11.4.13", &state),
            Transition::PatchUpgrade { from: "11.4.12".to_string(), to: "11.4.13".to_string() }
        );
    }

    #[test]
    fn downgrade_is_refused() {
        let state = state_with("11.4.99");
        assert_eq!(plan_transition("11.4.13", &state), Transition::Downgrade);
        assert_eq!(plan_transition("11.4.13", &state).as_failure(), Some(ServerFailure::VersionDowngrade));
    }

    #[test]
    fn series_mismatch_is_refused() {
        let state = state_with("10.11.6");
        assert_eq!(plan_transition("11.4.13", &state), Transition::SeriesMismatch);
        assert_eq!(plan_transition("11.4.13", &state).as_failure(), Some(ServerFailure::SeriesMismatch));
    }
}
