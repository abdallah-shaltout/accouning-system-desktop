//! Bundled, app-managed MariaDB for the Main PC (D11, 21.02-A2/A3). Owned by phase A2 — see
//! `plans/pending/21-rust-backend/02-core-and-shared/phase-a2-bundled-database.md`.
//!
//! Every Win32-only internal (spawning processes, the session-end hook, `%ProgramData%`
//! resolution, disk-space checks) is `#[cfg(windows)]`, so `cargo build` on any other target still
//! succeeds — those platforms simply have no managed server, and every entry point below returns
//! `ServerFailure::UnsupportedPlatform` instead of failing to compile.

pub mod config;
pub mod credentials;
pub mod errors;
pub mod payload;
pub mod paths;
pub mod state_file;
pub mod upgrade;

#[cfg(windows)]
pub mod admin;
#[cfg(windows)]
pub mod cli;
#[cfg(windows)]
pub mod process;
#[cfg(windows)]
pub mod provision;
#[cfg(windows)]
pub mod recovery;
#[cfg(windows)]
pub mod session_end;
#[cfg(windows)]
pub mod supervisor;
#[cfg(windows)]
mod win;

// 21.02-A3 — Main-PC LAN hosting: firewall, pairing, continuity (D11, D8, P2-56–P2-58).
#[cfg(windows)]
pub mod hosting;
#[cfg(windows)]
pub mod lan;
#[cfg(windows)]
pub mod pairing;

#[cfg(not(windows))]
pub mod cli {
    //! Non-Windows stub: there is no managed server to shut down, so `--db-shutdown` (which only
    //! the Windows installer's uninstall hook ever passes) is a no-op success on every other
    //! target.
    pub fn run_from_args() -> Option<i32> {
        None
    }
}
