//! Continuity while LAN sharing is on (phase-a3 A3-3, P2-58): a tray icon, hide-on-close instead of
//! exit-on-close, autostart toggled only by `lan::enable_lan_sharing`/`disable_lan_sharing`, and an
//! exit path that warns the owner when a cashier terminal is currently connected. With
//! `lanSharing` off, none of this changes anything — `lib.rs`'s existing close/exit behavior is
//! untouched (A2's `ExitRequested` handler, unmodified).

#![cfg(windows)]

use std::sync::Arc;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

use super::credentials::CredentialStore;
use super::paths::ServerPaths;
use super::state_file;
use super::supervisor::ServerHandle;

const MENU_OPEN: &str = "equal-lan-open";
const MENU_EXIT: &str = "equal-lan-exit";

/// Process-wide guard so the tray + hide-on-close behavior is installed at most once, however many
/// call sites race to install it (`lib.rs`'s boot-time watcher for "already sharing at launch", and
/// `lan::enable_lan_sharing` for "just turned on this run").
static TRAY_INSTALLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Installs the tray icon + hide-on-close behavior. Call this once a managed server is `Running`
/// with `server.json.lanSharing == true` — from `lib.rs`'s boot-time watcher (LAN sharing was
/// already on at launch) or right after `lan::enable_lan_sharing` succeeds (turned on this run).
/// Idempotent: a second call for the same process is a silent no-op.
pub fn install_tray_and_continuity(app: &AppHandle, server: Arc<ServerHandle>) -> tauri::Result<()> {
    if TRAY_INSTALLED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Ok(());
    }
    // Tooltip = the main window's own title (tauri.conf.json's `app.windows[0].title`) — never a
    // hard-coded app name (CLAUDE.md "brand" rule).
    let tooltip = app
        .get_webview_window("main")
        .and_then(|w| w.title().ok())
        .unwrap_or_default();

    let open_item = MenuItem::with_id(app, MENU_OPEN, "فتح البرنامج", true, None::<&str>)?;
    let exit_item = MenuItem::with_id(app, MENU_EXIT, "إيقاف خادم البيانات والخروج", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &exit_item])?;

    let app_for_menu = app.clone();
    let server_for_menu = Arc::clone(&server);
    let mut builder = TrayIconBuilder::with_id("equal-lan-tray")
        .tooltip(tooltip)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            MENU_OPEN => show_and_focus_main(app),
            MENU_EXIT => {
                let app = app_for_menu.clone();
                let server = Arc::clone(&server_for_menu);
                tauri::async_runtime::spawn(async move { handle_exit_request(&app, server).await });
            }
            _ => {}
        });
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app)?;

    // Hide-on-close: while LAN sharing is on, closing the main window hides it instead of exiting
    // the app (the server must keep serving terminals). `lib.rs`'s existing `ExitRequested` handler
    // (A2, `.run(|app, event| ...)`) is unaffected — this only intercepts the *window's* own
    // `CloseRequested`, which fires before an `ExitRequested` would even be considered.
    if let Some(window) = app.get_webview_window("main") {
        let server_for_close = Arc::clone(&server);
        let window_for_close = window.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Only intercept while this handle is actually serving LAN terminals right now —
                // re-read `server.json` each time rather than trusting a boot-time snapshot, since
                // LAN sharing can be toggled off/on while the app is running.
                if lan_sharing_is_on(&server_for_close.paths) {
                    api.prevent_close();
                    let _ = window_for_close.hide();
                }
            }
        });
    }

    Ok(())
}

fn lan_sharing_is_on(paths: &ServerPaths) -> bool {
    state_file::load(&paths.server_json()).ok().flatten().map(|s| s.lan_sharing).unwrap_or(false)
}

fn show_and_focus_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// «إيقاف خادم البيانات والخروج»: counts connected LAN terminals and, if any, asks for
/// confirmation through the native dialog before actually stopping the server and exiting — never
/// a silent kick of a cashier mid-sale.
async fn handle_exit_request(app: &AppHandle, server: Arc<ServerHandle>) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

    let count = connected_count(&server).await;
    if count > 0 {
        let (tx, rx) = tokio::sync::oneshot::channel();
        app.dialog()
            .message(format!(
                "يوجد {count} جهاز كاشير متصل الآن وسيتوقف عن العمل. هل تريد الخروج؟"
            ))
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom("خروج".to_string(), "إلغاء".to_string()))
            .show(move |confirmed| {
                let _ = tx.send(confirmed);
            });
        let confirmed = rx.await.unwrap_or(false);
        if !confirmed {
            return;
        }
    }

    server.shutdown(std::time::Duration::from_secs(60)).await;
    app.exit(0);
}

async fn connected_count(server: &Arc<ServerHandle>) -> u64 {
    let Some(state) = state_file::load(&server.paths.server_json()).ok().flatten() else {
        return 0;
    };
    let creds = CredentialStore::production();
    let Ok(root_secret) = creds.get_root_secret(state.data_dir_id) else {
        return 0;
    };
    super::lan::connected_terminal_count(state.port, &root_secret).await
}

/// A3-3: autostart is toggled **only** from `lan::on_lan_sharing_enabled`/`on_lan_sharing_disabled`
/// (called right after `lan::enable_lan_sharing`/`disable_lan_sharing` succeed) — never anywhere
/// else. Uses the Rust `ManagerExt::autolaunch()` handle only (no JS permission is added, matching
/// the phase brief).
pub fn set_autostart(app: &AppHandle, enabled: bool) {
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    let result = if enabled { autolaunch.enable() } else { autolaunch.disable() };
    if let Err(e) = result {
        log::error!(target: "infrastructure::database", "failed to toggle autostart (enabled={enabled}): {e}");
    }
}

/// Registered in `lib.rs`'s `tauri::Builder` alongside the other plugins — `MacosLauncher` is
/// irrelevant on this Windows-first app but the plugin API requires picking one; `--background` is
/// the arg passed when the OS starts the app at logon (A3-3: `setup` checks for it to decide
/// whether to show the main window).
pub fn autostart_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    // `macos_launcher` exists only on macOS builds; the builder's default is already `LaunchAgent`.
    tauri_plugin_autostart::Builder::new()
        .args(["--background"])
        .build()
}

/// `setup()` calls this once the window exists: shows it immediately unless the process was
/// launched with `--background` (autostart at logon) — no flash of the window before it hides
/// itself again on a LAN-sharing PC.
pub fn show_main_window_unless_background(app: &AppHandle) {
    let background = std::env::args().any(|a| a == "--background");
    if background {
        return;
    }
    show_and_focus_main(app);
}
