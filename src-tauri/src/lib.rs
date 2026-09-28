// 21.02-A: module layout after the moves. Always refer to the first as `crate::core::…` and
// never write `use core::…` in this file (P2-37) — that would collide with the builtin `core`
// crate. `domains` doesn't exist yet (Part 03 creates it).
pub mod core;
pub mod domains;
pub mod entities;
pub mod infrastructure;
pub mod shared;
pub mod utils;

/// The installer ships a pinned WebView2 runtime next to the exe (`webviewInstallMode: fixedRuntime`,
/// see scripts/fetch-webview2.js), so the UI renders the same on every PC. When that folder isn't
/// there — `tauri dev` (target/debug) or a damaged install — fall back to the system WebView2
/// instead of failing to open the window.
fn context() -> tauri::Context<tauri::Wry> {
    #[allow(unused_mut)]
    let mut context = tauri::generate_context!();
    #[cfg(windows)]
    {
        use tauri::utils::config::WebviewInstallMode;
        let install_mode = &mut context.config_mut().bundle.windows.webview_install_mode;
        if let WebviewInstallMode::FixedRuntime { path } = install_mode {
            let present = std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|dir| dir.join(&*path).join("msedgewebview2.exe")))
                .is_some_and(|exe| exe.exists());
            if !present {
                *install_mode = WebviewInstallMode::default();
            }
        }
    }
    context
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::Manager as _;

    // Guards against a second `RunEvent::ExitRequested` re-entering the shutdown-and-exit branch
    // below while the first one's async shutdown is still in flight.
    let exit_in_progress = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    tauri::Builder::default()
        // Guarantees only one app process — and thus only one MariaDB supervisor — ever runs
        // against the machine's data directory at a time (P2-58). Must be the *first* plugin
        // registered so a second launch is caught before anything else (window creation, DB boot)
        // starts.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        // A3-3: registered unconditionally (cheap no-op plugin when never enabled) — the Rust
        // `ManagerExt::autolaunch()` handle is toggled only by
        // `infrastructure::database::lan::enable_lan_sharing`/`disable_lan_sharing`, never anywhere
        // else. No JS permission is added for this (`--background` is a plain process arg, not an
        // IPC-visible capability).
        .plugin({
            #[cfg(windows)]
            {
                infrastructure::database::hosting::autostart_plugin()
            }
            #[cfg(not(windows))]
            {
                tauri_plugin_autostart::Builder::new().build()
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();
            core::state::boot(handle.clone());

            // A3-3: show the main window immediately unless this launch is the OS starting the app
            // at logon in the background (`--background`, passed only by the autostart entry this
            // phase registers) — no flash of the window before it hides itself again on a
            // LAN-sharing PC.
            #[cfg(windows)]
            infrastructure::database::hosting::show_main_window_unless_background(&handle);
            #[cfg(not(windows))]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                }
            }

            // A3-3: if LAN sharing is already on at launch (the common case — the owner enabled it
            // on a previous run), install the tray icon + hide-on-close behavior once the managed
            // server reaches `Running`. If LAN sharing gets turned on *during* this run instead,
            // `infrastructure::database::lan::enable_lan_sharing` installs it itself right after a
            // successful enable (`install_tray_and_continuity` is idempotent — see its doc comment
            // — so this boot-time watcher and that call can never install it twice).
            #[cfg(windows)]
            {
                let handle_for_tray = handle.clone();
                tauri::async_runtime::spawn(async move {
                    use tauri::Manager as _;
                    for _ in 0..240 {
                        // up to ~60s: provisioning + first start can legitimately take a while
                        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                        let state = handle_for_tray.state::<core::state::AppState>();
                        let is_running = state
                            .server
                            .state
                            .try_read()
                            .map(|s| s.phase == infrastructure::database::supervisor::ServerPhase::Running)
                            .unwrap_or(false);
                        if !is_running {
                            continue;
                        }
                        // A2-7: block Windows shutdown/sign-out until the managed server has
                        // stopped cleanly — started once, only after the server is really running.
                        infrastructure::database::session_end::start(std::sync::Arc::clone(&state.server));
                        let lan_sharing = infrastructure::database::state_file::load(&state.server.paths.server_json())
                            .ok()
                            .flatten()
                            .map(|s| s.lan_sharing)
                            .unwrap_or(false);
                        if lan_sharing {
                            let server = std::sync::Arc::clone(&state.server);
                            let _ = infrastructure::database::hosting::install_tray_and_continuity(&handle_for_tray, server);
                        }
                        break;
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            infrastructure::pdf::render::render_pdf,
            infrastructure::pdf::render::render_preview,
            infrastructure::print::commands::list_printers,
            infrastructure::print::commands::print_thermal_receipt,
            infrastructure::print::commands::print_test_receipt,
            core::diag::diag_append,
            core::diag::diag_read,
            core::diag::diag_clear,
            core::diag::diag_open_folder,
            core::diag::diag_rotate,
            core::status::core_backend_status
        ])
        .build(context())
        .expect("error while building tauri application")
        .run(move |app_handle, event| {
            let tauri::RunEvent::ExitRequested { api, .. } = event else { return };

            #[cfg(windows)]
            let managed_and_running = {
                use infrastructure::database::supervisor::ServerPhase;
                let state = app_handle.state::<core::state::AppState>();
                state.server.state.try_read().map(|s| s.phase == ServerPhase::Running).unwrap_or(false)
            };
            #[cfg(not(windows))]
            let managed_and_running = false;

            if !managed_and_running {
                return;
            }
            if exit_in_progress.swap(true, std::sync::atomic::Ordering::SeqCst) {
                return; // a shutdown is already in flight from a previous exit request
            }

            api.prevent_exit();
            let app_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                #[cfg(windows)]
                {
                    let state = app_handle.state::<core::state::AppState>();
                    let server = std::sync::Arc::clone(&state.server);
                    server.shutdown(std::time::Duration::from_secs(60)).await;
                }
                app_handle.exit(0);
            });
        });
}
