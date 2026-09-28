//! Windows session-end handling (phase-a2 A2-7): blocks a Windows shutdown/sign-out long enough
//! to cleanly stop the managed MariaDB server, so InnoDB never has to run crash recovery just
//! because the OS was shutting down, not because the app crashed.
//!
//! A **hidden top-level window** (not message-only — message-only windows never receive
//! `WM_QUERYENDSESSION`/`WM_ENDSESSION` broadcasts) is created on a dedicated thread with its own
//! message loop. Started only while a managed server is actually running.

#![cfg(windows)]

use std::sync::Arc;

use super::supervisor::ServerHandle;

/// Starts the hidden window + message loop on a new OS thread. The thread runs for the lifetime
/// of the process; there is nothing to join because the process exits (or the message loop is
/// torn down with the app) when the app itself exits.
pub fn start(server: Arc<ServerHandle>) {
    std::thread::spawn(move || unsafe { run_message_loop(server) });
}

unsafe fn run_message_loop(server: Arc<ServerHandle>) {
    use std::sync::OnceLock;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Shutdown::{ShutdownBlockReasonCreate, ShutdownBlockReasonDestroy};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW, TranslateMessage, CW_USEDEFAULT, MSG,
        WINDOW_EX_STYLE, WNDCLASSW, WS_OVERLAPPEDWINDOW,
    };

    // Store the handle where the static window procedure can reach it — Win32 window procs are
    // plain `extern "system" fn`s with no closure capture, so process-global storage is the only
    // option (one such window ever exists per process, matching `ServerHandle`'s own P2-58
    // singleton guarantee).
    static SERVER: OnceLock<Arc<ServerHandle>> = OnceLock::new();
    let _ = SERVER.set(server);

    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        const WM_QUERYENDSESSION: u32 = 0x0011;
        const WM_ENDSESSION: u32 = 0x0016;

        match msg {
            WM_QUERYENDSESSION => {
                let reason_text: Vec<u16> = "ايكوال يحفظ البيانات…\0".encode_utf16().collect();
                unsafe {
                    let _ = ShutdownBlockReasonCreate(hwnd, windows::core::PCWSTR(reason_text.as_ptr()));
                }
                LRESULT(1) // TRUE: allow shutdown to proceed to WM_ENDSESSION
            }
            WM_ENDSESSION => {
                if wparam.0 != 0 {
                    if let Some(server) = SERVER.get() {
                        // WM_ENDSESSION handling must block until the server is stopped — spin up
                        // a throwaway current-thread runtime rather than requiring this whole
                        // module to run inside Tokio (it's a dedicated OS thread by design).
                        let server = Arc::clone(server);
                        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
                        if let Ok(rt) = rt {
                            rt.block_on(server.shutdown(std::time::Duration::from_secs(20)));
                        }
                    }
                    unsafe {
                        let _ = ShutdownBlockReasonDestroy(hwnd);
                    }
                }
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }

    let class_name: Vec<u16> = "EqualSessionEndWindow\0".encode_utf16().collect();
    let hinstance = GetModuleHandleW(PCWSTR::null()).unwrap_or_default();

    let wc = WNDCLASSW {
        lpfnWndProc: Some(wndproc),
        hInstance: hinstance.into(),
        lpszClassName: PCWSTR(class_name.as_ptr()),
        ..Default::default()
    };
    let atom = RegisterClassW(&wc);
    if atom == 0 {
        log::error!(target: "infrastructure::database", "session_end: RegisterClassW failed");
        return;
    }

    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        PCWSTR(class_name.as_ptr()),
        PCWSTR(class_name.as_ptr()),
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        0,
        0,
        None,
        None,
        Some(hinstance.into()),
        None,
    );
    let Ok(_hwnd) = hwnd else {
        log::error!(target: "infrastructure::database", "session_end: CreateWindowExW failed");
        return;
    };
    // Deliberately never shown (`ShowWindow` is never called) — a hidden top-level window still
    // receives WM_QUERYENDSESSION/WM_ENDSESSION broadcasts, unlike a message-only window.

    let mut msg = MSG::default();
    loop {
        let result = GetMessageW(&mut msg, None, 0, 0);
        if result.0 <= 0 {
            break;
        }
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}
