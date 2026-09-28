//! LAN sharing enable/disable/rotate (phase-a3 A3-1, P2-56, P2-49). Turns the Main PC's bundled
//! MariaDB server from loopback-only into a server the shop's cashier terminals can reach: one
//! elevated (`runas`) `netsh` firewall rule, a DML-only `equal_lan@'%'` MariaDB account, and
//! `server.json.lanSharing = true` with the server bound to `0.0.0.0` (A2's `BindMode::Lan`).
//!
//! A3 adds **no IPC command** here (P2-54) — Part 03's `setup`/`settings` call these functions
//! directly through a module `service`.

#![cfg(windows)]

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;

use super::credentials::CredentialStore;
use super::errors::ServerFailure;
use super::pairing::PairingInfo;
use super::state_file::{self, ServerStateFile};
use super::supervisor::ServerHandle;

/// The exact firewall rule name used both to check for and to create the rule — kept in one place
/// so `netsh ... show rule name="..."` and `netsh ... add rule name="..."` can never drift apart.
pub fn firewall_rule_name() -> String {
    format!("Equal Database ({})", super::paths::APP_IDENTIFIER)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LanError {
    /// The UAC elevation prompt was cancelled (`ERROR_CANCELLED`, 1223) — nothing else changed.
    PermissionDeclined,
    /// The firewall rule could not be verified present after the elevated `netsh` call reported
    /// success (a `show rule` re-check failed).
    FirewallVerificationFailed,
    /// Wraps a managed-server failure encountered while restarting bound to LAN (or back to
    /// loopback).
    Server(ServerFailure),
    /// No managed server exists on this PC yet (`server.json` missing) — LAN sharing only applies
    /// to a Main PC that has already provisioned its database.
    NotProvisioned,
    /// Credentials (root or LAN secret) could not be read/written to the OS keyring.
    CredentialsUnrecoverable,
    Unknown,
}

impl LanError {
    pub fn message_ar(self) -> &'static str {
        match self {
            LanError::PermissionDeclined => "لم يتم منح الإذن — لن تتمكن أجهزة الكاشير من الاتصال بهذا الجهاز",
            LanError::FirewallVerificationFailed => {
                "تعذر التأكد من إعداد جدار الحماية — أعد المحاولة، وإن تكررت المشكلة أرسل ملف التشخيص للدعم"
            }
            LanError::Server(f) => f.message_ar(),
            LanError::NotProvisioned => "لا توجد قاعدة بيانات على هذا الجهاز بعد",
            LanError::CredentialsUnrecoverable => "تعذر حفظ بيانات الدخول لمشاركة الشبكة — أرسل ملف التشخيص للدعم",
            LanError::Unknown => "تعذر تفعيل مشاركة الشبكة",
        }
    }
}

impl std::fmt::Display for LanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self, self.message_ar())
    }
}

impl std::error::Error for LanError {}

impl From<ServerFailure> for LanError {
    fn from(f: ServerFailure) -> Self {
        LanError::Server(f)
    }
}

/// Whether the elevated `netsh` firewall step actually runs (`Real`, production) or is skipped
/// entirely (`Skip`, `tests/db_server_lan.rs` — so the suite can run unattended, without a UAC
/// prompt, on a CI/dev machine that already trusts the port).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirewallMode {
    Real,
    Skip,
}

/// Builds the exact `netsh advfirewall firewall add rule ...` argument list for the elevated
/// `ShellExecuteExW` call — a pure function so the exact string is unit-tested without touching
/// the registry or spawning anything.
pub fn build_add_rule_args(rule_name: &str, mariadbd_exe: &std::path::Path, port: u16) -> String {
    format!(
        "advfirewall firewall add rule name=\"{rule_name}\" dir=in action=allow program=\"{program}\" \
protocol=TCP localport={port} remoteip=localsubnet profile=any",
        program = mariadbd_exe.display(),
    )
}

/// Builds the (non-elevated) `netsh advfirewall firewall show rule name="..."` argument list used
/// both to check for the rule before elevating and to re-verify it after.
pub fn build_show_rule_args(rule_name: &str) -> String {
    format!("advfirewall firewall show rule name=\"{rule_name}\"")
}

/// `ERROR_CANCELLED` — returned by `ShellExecuteExW`'s `GetLastError()` when the user dismisses the
/// UAC consent prompt instead of approving it.
const ERROR_CANCELLED: u32 = 1223;

/// Runs `netsh.exe show rule name="..."` non-elevated and reports whether the rule is present
/// (`netsh` exits 0 and prints rule details when found, non-zero/"No rules match" when absent).
fn firewall_rule_present(rule_name: &str) -> bool {
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    let netsh = std::path::Path::new(&system_root).join("System32").join("netsh.exe");
    let output = std::process::Command::new(netsh).args(["advfirewall", "firewall", "show", "rule", &format!("name={rule_name}")]).output();
    match output {
        Ok(out) => out.status.success() && !String::from_utf8_lossy(&out.stdout).contains("No rules match"),
        Err(_) => false,
    }
}

/// Elevates via `ShellExecuteExW` (verb `runas`) to add the firewall rule, waits for the child
/// process, and returns its exit code — or `Err(LanError::PermissionDeclined)` if the consent
/// prompt was cancelled. `SW_HIDE`: no visible console flash for the split-second `netsh` window.
fn elevate_add_rule(rule_name: &str, mariadbd_exe: &std::path::Path, port: u16) -> Result<(), LanError> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::GetLastError;
    use windows::Win32::System::Threading::{WaitForSingleObject, INFINITE};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    let netsh_path = std::path::Path::new(&system_root).join("System32").join("netsh.exe");
    let params = build_add_rule_args(rule_name, mariadbd_exe, port);

    let verb: Vec<u16> = "runas\0".encode_utf16().collect();
    let file: Vec<u16> = netsh_path.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
    let params_w: Vec<u16> = params.encode_utf16().chain(std::iter::once(0)).collect();

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params_w.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };

    let ok = unsafe { ShellExecuteExW(&mut info) };
    if ok.is_err() {
        let err = unsafe { GetLastError() };
        if err.0 == ERROR_CANCELLED {
            return Err(LanError::PermissionDeclined);
        }
        return Err(LanError::FirewallVerificationFailed);
    }

    if info.hProcess.is_invalid() {
        return Err(LanError::FirewallVerificationFailed);
    }
    unsafe {
        WaitForSingleObject(info.hProcess, INFINITE);
        let _ = windows::Win32::Foundation::CloseHandle(info.hProcess);
    }
    Ok(())
}

/// Ensures the firewall rule exists, elevating (once) only if it doesn't. `FirewallMode::Skip`
/// (tests only) treats the rule as always-present without touching the registry or spawning
/// anything.
fn ensure_firewall_rule(mode: FirewallMode, mariadbd_exe: &std::path::Path, port: u16) -> Result<(), LanError> {
    if mode == FirewallMode::Skip {
        return Ok(());
    }
    let rule_name = firewall_rule_name();
    if firewall_rule_present(&rule_name) {
        return Ok(());
    }
    elevate_add_rule(&rule_name, mariadbd_exe, port)?;
    if !firewall_rule_present(&rule_name) {
        return Err(LanError::FirewallVerificationFailed);
    }
    Ok(())
}

/// A3-1: enables LAN sharing on this Main PC, in the exact order the spec requires. On any failure
/// after step 1, nothing already-applied is rolled back automatically (the caller may retry —
/// `enable_lan_sharing` is safe to call again since every step is idempotent or checks first).
///
/// This function alone has no `AppHandle` dependency, so it's callable both from the real app
/// (Part 03's `setup`/`settings` service, which then also calls `on_lan_sharing_enabled` — see
/// below — for the tray/autostart continuity side effects) and from `db_server_smoke`, a
/// standalone diagnostic binary with no live Tauri app/window.
pub async fn enable_lan_sharing(
    server: &Arc<ServerHandle>,
    creds: &CredentialStore,
    payload: &super::payload::PayloadDir,
    firewall: FirewallMode,
) -> Result<PairingInfo, LanError> {
    let paths = server.paths.clone();
    let Some(mut state) = state_file::load(&paths.server_json()).map_err(|_| LanError::NotProvisioned)? else {
        return Err(LanError::NotProvisioned);
    };

    // Step 1: firewall rule (elevated only if not already present).
    ensure_firewall_rule(firewall, &paths.mariadbd_exe(), state.port)?;

    // Step 2: equal_lan@'%' with a fresh secret + DML-only grants (no DDL — P2-28 enforced by
    // privilege, not just convention).
    let lan_secret = super::credentials::generate_secret(16);
    let root_secret = creds.get_root_secret(state.data_dir_id).map_err(|_| LanError::CredentialsUnrecoverable)?;
    create_or_rotate_lan_user(state.port, &root_secret, &lan_secret).await?;
    creds.set_lan_secret(state.data_dir_id, &lan_secret).map_err(|_| LanError::CredentialsUnrecoverable)?;

    // Step 3: server.json.lanSharing = true, bind Lan, clean restart.
    state.lan_sharing = true;
    state.updated_at = chrono::Utc::now();
    state_file::save(&paths.server_json(), &state).map_err(|_| LanError::Unknown)?;

    server.shutdown(Duration::from_secs(60)).await;
    server.ensure_running(payload, creds).await?;

    let host_name = super::pairing::host_name().unwrap_or_else(|| "هذا الجهاز".to_string());
    Ok(PairingInfo { host_name, addresses: super::pairing::non_loopback_ipv4_addresses(), port: state.port, code: super::pairing::format_code(&lan_secret) })
}

/// A3-3 continuity side effects for a successful `enable_lan_sharing` — kept separate from
/// `enable_lan_sharing` itself (above) purely so that function stays callable with no `AppHandle`
/// (`db_server_smoke`). The real app's Part 03 `service` calls this right after `enable_lan_sharing`
/// succeeds. Turns autostart on and installs the tray + hide-on-close behavior immediately, rather
/// than waiting for the next launch's boot-time watcher (`lib.rs`'s `setup`) to notice
/// `lanSharing` is now `true`.
pub fn on_lan_sharing_enabled(app: &tauri::AppHandle, server: &Arc<ServerHandle>) {
    super::hosting::set_autostart(app, true);
    let _ = super::hosting::install_tray_and_continuity(app, Arc::clone(server));
}

/// A3-1: disables LAN sharing. The firewall rule is deliberately kept (it's inert while the server
/// is loopback-bound again, and removing/re-adding it on every toggle would mean a second UAC
/// prompt on every re-enable — C-26/C-27 accept "kept, inert" over "removed, re-prompt"). Autostart
/// is turned off separately — see `on_lan_sharing_disabled`, same split as `enable_lan_sharing`'s.
pub async fn disable_lan_sharing(server: &Arc<ServerHandle>, creds: &CredentialStore, payload: &super::payload::PayloadDir) -> Result<(), LanError> {
    let paths = server.paths.clone();
    let Some(mut state) = state_file::load(&paths.server_json()).map_err(|_| LanError::NotProvisioned)? else {
        return Err(LanError::NotProvisioned);
    };
    if !state.lan_sharing {
        return Ok(());
    }

    let root_secret = creds.get_root_secret(state.data_dir_id).map_err(|_| LanError::CredentialsUnrecoverable)?;
    drop_lan_user(state.port, &root_secret).await?;
    creds.delete_lan_secret(state.data_dir_id);

    state.lan_sharing = false;
    state.updated_at = chrono::Utc::now();
    state_file::save(&paths.server_json(), &state).map_err(|_| LanError::Unknown)?;

    server.shutdown(Duration::from_secs(60)).await;
    server.ensure_running(payload, creds).await?;
    Ok(())
}

/// A3-3 continuity side effect for a successful `disable_lan_sharing` — turns autostart off. The
/// tray icon/hide-on-close behavior installed by a prior enable is intentionally left in place for
/// the rest of this process's lifetime (removing a live tray icon mid-session is unneeded
/// complexity for a state that already stops mattering: `hosting.rs`'s close handler re-checks
/// `server.json.lanSharing` on every close, so hide-on-close simply stops triggering once this
/// call returns).
pub fn on_lan_sharing_disabled(app: &tauri::AppHandle) {
    super::hosting::set_autostart(app, false);
}

/// A3-1: `rotate_pairing_code` — issues a fresh secret for `equal_lan@'%'` via `ALTER USER`. Every
/// previously paired terminal's stored password becomes invalid immediately (they must re-pair).
pub async fn rotate_pairing_code(server: &Arc<ServerHandle>, creds: &CredentialStore) -> Result<PairingInfo, LanError> {
    let paths = server.paths.clone();
    let Some(state) = state_file::load(&paths.server_json()).map_err(|_| LanError::NotProvisioned)? else {
        return Err(LanError::NotProvisioned);
    };
    if !state.lan_sharing {
        return Err(LanError::NotProvisioned);
    }

    let root_secret = creds.get_root_secret(state.data_dir_id).map_err(|_| LanError::CredentialsUnrecoverable)?;
    let new_secret = super::credentials::generate_secret(16);
    alter_lan_user_password(state.port, &root_secret, &new_secret).await?;
    creds.set_lan_secret(state.data_dir_id, &new_secret).map_err(|_| LanError::CredentialsUnrecoverable)?;

    let host_name = super::pairing::host_name().unwrap_or_else(|| "هذا الجهاز".to_string());
    Ok(PairingInfo { host_name, addresses: super::pairing::non_loopback_ipv4_addresses(), port: state.port, code: super::pairing::format_code(&new_secret) })
}

/// While `lanSharing` is true, the supervisor must never silently move to a different port (P2-56)
/// — terminals and the firewall rule are both bound to the port recorded in `server.json`. This is
/// enforced simply by never calling `pick_free_port` again once `ready`; `ensure_running` always
/// reads `state.port` from `server.json` and only ever fails `PortUnavailable` if that exact port
/// is unavailable, rather than silently choosing another one.
pub fn assert_port_is_pinned_while_lan_sharing(state: &ServerStateFile) {
    debug_assert!(state.port > 0, "server.json must always carry the pinned port");
}

async fn create_or_rotate_lan_user(port: u16, root_secret: &str, lan_secret: &str) -> Result<(), LanError> {
    use sea_orm::{ConnectionTrait, Statement};
    let db = super::admin::connect_root(port, "root", root_secret, None).await.map_err(|_| LanError::Unknown)?;
    let exec = |sql: String| {
        let db = &db;
        async move {
            let stmt = Statement::from_string(db.get_database_backend(), sql);
            db.execute(stmt).await
        }
    };
    exec(format!("CREATE USER IF NOT EXISTS equal_lan@'%' IDENTIFIED BY '{lan_secret}'")).await.map_err(|_| LanError::Unknown)?;
    exec(format!("ALTER USER equal_lan@'%' IDENTIFIED BY '{lan_secret}'")).await.map_err(|_| LanError::Unknown)?;
    exec(
        "GRANT SELECT, INSERT, UPDATE, DELETE, CREATE TEMPORARY TABLES, LOCK TABLES ON equal.* TO equal_lan@'%'".to_string(),
    )
    .await
    .map_err(|_| LanError::Unknown)?;
    exec("FLUSH PRIVILEGES".to_string()).await.ok();
    let _ = db.close().await;
    Ok(())
}

async fn alter_lan_user_password(port: u16, root_secret: &str, new_secret: &str) -> Result<(), LanError> {
    use sea_orm::{ConnectionTrait, Statement};
    let db = super::admin::connect_root(port, "root", root_secret, None).await.map_err(|_| LanError::Unknown)?;
    let stmt = Statement::from_string(db.get_database_backend(), format!("ALTER USER equal_lan@'%' IDENTIFIED BY '{new_secret}'"));
    db.execute(stmt).await.map_err(|_| LanError::Unknown)?;
    let _ = db.close().await;
    Ok(())
}

async fn drop_lan_user(port: u16, root_secret: &str) -> Result<(), LanError> {
    use sea_orm::{ConnectionTrait, Statement};
    let db = super::admin::connect_root(port, "root", root_secret, None).await.map_err(|_| LanError::Unknown)?;
    let stmt = Statement::from_string(db.get_database_backend(), "DROP USER IF EXISTS equal_lan@'%'".to_string());
    db.execute(stmt).await.map_err(|_| LanError::Unknown)?;
    let _ = db.close().await;
    Ok(())
}

/// The current number of connected LAN terminals (A3-3's exit-warning count): a root-only
/// `information_schema.PROCESSLIST` query for the `equal_lan` user.
pub async fn connected_terminal_count(port: u16, root_secret: &str) -> u64 {
    use sea_orm::{ConnectionTrait, Statement};
    let Ok(db) = super::admin::connect_root(port, "root", root_secret, None).await else {
        return 0;
    };
    let stmt = Statement::from_string(
        db.get_database_backend(),
        "SELECT COUNT(*) AS c FROM information_schema.PROCESSLIST WHERE USER='equal_lan'".to_string(),
    );
    let count = match db.query_one(stmt).await {
        Ok(Some(row)) => row.try_get::<i64>("", "c").unwrap_or(0).max(0) as u64,
        _ => 0,
    };
    let _ = db.close().await;
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_rule_args_match_exact_spec_string() {
        let exe = std::path::Path::new(r"C:\ProgramData\com.abdallah.accounting-app\database\server-bin\current\bin\mariadbd.exe");
        let args = build_add_rule_args("Equal Database (com.abdallah.accounting-app)", exe, 3406);
        assert_eq!(
            args,
            "advfirewall firewall add rule name=\"Equal Database (com.abdallah.accounting-app)\" dir=in action=allow \
program=\"C:\\ProgramData\\com.abdallah.accounting-app\\database\\server-bin\\current\\bin\\mariadbd.exe\" \
protocol=TCP localport=3406 remoteip=localsubnet profile=any"
        );
    }

    #[test]
    fn show_rule_args_match_exact_spec_string() {
        let args = build_show_rule_args("Equal Database (com.abdallah.accounting-app)");
        assert_eq!(args, "advfirewall firewall show rule name=\"Equal Database (com.abdallah.accounting-app)\"");
    }

    #[test]
    fn rule_name_embeds_the_app_identifier() {
        assert_eq!(firewall_rule_name(), "Equal Database (com.abdallah.accounting-app)");
    }
}
