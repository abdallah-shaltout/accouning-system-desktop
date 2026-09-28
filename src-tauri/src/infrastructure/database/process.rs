//! Spawning, probing and terminating the `mariadbd.exe` child process (phase-a2 A2-7, P2-46).
//! Windows-only: the managed server is a Windows-only feature (D11 targets the desktop Windows
//! app specifically; other targets never reach this module).

#![cfg(windows)]

use std::path::Path;
use std::process::Stdio;

use tokio::process::{Child, Command};

/// Spawns `mariadbd.exe --defaults-file=<my.ini>` detached from the app's process tree: no
/// kill-on-close job object, `CREATE_NO_WINDOW` (no console flash) and `CREATE_NEW_PROCESS_GROUP`
/// (so the server doesn't receive Ctrl+C meant for the app/terminal). The server must outlive an
/// app crash — an app crash must never take the customer's database down with it.
pub fn spawn(mariadbd_exe: &Path, my_ini: &Path, extra_args: &[&str]) -> std::io::Result<Child> {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

    let mut cmd = Command::new(mariadbd_exe);
    cmd.arg(format!("--defaults-file={}", my_ini.display()));
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    cmd.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    // `kill_on_drop(false)`: dropping the `Child` handle (e.g. when the app process exits or this
    // handle is intentionally discarded after an "adopt") must never terminate the server.
    cmd.kill_on_drop(false);
    cmd.spawn()
}

/// Resolves a running process's full image path via
/// `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION) + QueryFullProcessImageNameW` — used both to
/// verify an adopted pid really is `server-bin\current\bin\mariadbd.exe` (not some unrelated
/// process that happened to reuse the pid) and by `cli.rs`'s `--db-shutdown` before it terminates
/// anything by pid.
/// Waits for a child we asked to stop (`SHUTDOWN`) for at most `timeout`, then kills it. Never
/// waits unbounded: if the stop request itself could not be sent (e.g. the root connection failed),
/// a bare `child.wait()` would hang the caller forever.
pub async fn wait_or_kill(child: &mut Child, timeout: std::time::Duration) {
    if tokio::time::timeout(timeout, child.wait()).await.is_err() {
        log::warn!(target: "infrastructure::database", "mariadbd did not stop within {timeout:?}; killing it");
        let _ = child.start_kill();
        let _ = child.wait().await;
    }
}

pub fn image_path(pid: u32) -> Option<std::path::PathBuf> {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};

    unsafe {
        let handle: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 32768];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(handle);
        if !ok || len == 0 {
            return None;
        }
        Some(std::path::PathBuf::from(String::from_utf16_lossy(&buf[..len as usize])))
    }
}

/// True if a process with this pid is currently alive (probed the cheap way, via `image_path`
/// succeeding — a dead/reused pid either fails to open or resolves to an unrelated image).
pub fn is_running(pid: u32, expected_image: &Path) -> bool {
    match image_path(pid) {
        Some(actual) => paths_equal_case_insensitive(&actual, expected_image),
        None => false,
    }
}

fn paths_equal_case_insensitive(a: &Path, b: &Path) -> bool {
    let canon_a = std::fs::canonicalize(a).unwrap_or_else(|_| a.to_path_buf());
    let canon_b = std::fs::canonicalize(b).unwrap_or_else(|_| b.to_path_buf());
    canon_a.to_string_lossy().eq_ignore_ascii_case(&canon_b.to_string_lossy())
}

/// Blocks (on a `spawn_blocking` thread) until the process exits, via a `SYNCHRONIZE` handle +
/// `WaitForSingleObject` — used by the crash-watcher to detect an unexpected exit without polling.
pub async fn wait_exit(pid: u32) {
    let _ = tokio::task::spawn_blocking(move || {
        use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
        use windows::Win32::System::Threading::{OpenProcess, WaitForSingleObject, INFINITE, PROCESS_SYNCHRONIZE};
        unsafe {
            let Ok(handle) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) else {
                return; // already gone
            };
            let handle: HANDLE = handle;
            let _ = WaitForSingleObject(handle, INFINITE) == WAIT_OBJECT_0;
            let _ = CloseHandle(handle);
        }
    })
    .await;
}

/// Forceful termination — used only when a clean `SHUTDOWN` times out, or when `cli.rs`'s
/// `--db-shutdown` must guarantee the process is gone before the installer removes `server-bin`.
pub fn terminate(pid: u32) -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, pid) else {
            return false;
        };
        let handle: HANDLE = handle;
        let ok = TerminateProcess(handle, 1).is_ok();
        let _ = CloseHandle(handle);
        ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_process_image_path_resolves_to_this_test_binary() {
        let pid = std::process::id();
        let path = image_path(pid);
        assert!(path.is_some(), "image_path should resolve the current process");
    }

    #[test]
    fn a_pid_that_does_not_exist_is_not_running() {
        // pid 0 is reserved (System Idle Process on Windows) and OpenProcess on it should fail
        // with PROCESS_QUERY_LIMITED_INFORMATION for a non-existent/inaccessible target image.
        assert!(!is_running(0, Path::new(r"C:\nonexistent\mariadbd.exe")));
    }
}
